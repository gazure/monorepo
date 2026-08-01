//! Ball flight, in feet and seconds.
//!
//! The ball is the only object in the game with a real physical state: a position
//! whose `z` is height above the grass, and a velocity. Every batted-ball outcome
//! is read off this trajectory rather than decided in advance, which is the whole
//! point — before, the result was chosen at the moment of contact and the flight
//! was decoration.

use bevy::prelude::*;

use super::field;

/// Feet per second squared.
pub const GRAVITY: f32 = 32.174;

/// Linear drag, per second. A baseball loses a great deal to the air: with no
/// drag at all, 105 mph off the bat at 28° would carry 579 feet. Solved
/// numerically so that same contact lands at 405 — a comfortable home run —
/// which puts the rest of the launch-angle range in realistic territory too.
pub const DRAG: f32 = 0.166;

/// How much speed a ball keeps when it bounces.
const RESTITUTION: f32 = 0.36;

/// How much a bouncing or rolling ball is slowed by the turf each bounce.
const GROUND_FRICTION: f32 = 0.72;

/// Below this speed a rolling ball is treated as dead.
const REST_SPEED: f32 = 6.0;

/// Miles per hour to feet per second.
pub const MPH_TO_FPS: f32 = 1.466_67;

/// Physical truth for the live ball. The two views each render from this rather
/// than keeping their own copy, so they cannot drift apart.
#[derive(Debug, Default, Resource)]
pub struct LiveBall {
    /// Feet. `z` is height above the ground.
    pub pos: Vec3,
    /// Feet per second.
    pub vel: Vec3,
    /// Whether the ball is under physics right now.
    pub live: bool,
    /// Seconds since the ball was put in play.
    pub elapsed: f32,
}

impl LiveBall {
    pub fn height(&self) -> f32 {
        self.pos.z
    }

    /// Launches the ball from the plate.
    pub fn hit(&mut self, exit_velocity: f32, launch: f32, spray: f32) {
        let horizontal = exit_velocity * launch.cos();
        self.pos = Vec3::new(0.0, 1.5, 3.0);
        self.vel = Vec3::new(
            horizontal * spray.sin(),
            horizontal * spray.cos(),
            exit_velocity * launch.sin(),
        );
        self.live = true;
        self.elapsed = 0.0;
    }

    pub fn clear(&mut self) {
        self.live = false;
        self.vel = Vec3::ZERO;
        self.elapsed = 0.0;
    }
}

/// Advances a ball one step, bouncing it off the ground. Shared by the live
/// simulation and the lookahead used to position fielders, so the prediction can
/// never disagree with what the player watches.
pub fn step(pos: Vec3, vel: Vec3, dt: f32) -> (Vec3, Vec3) {
    let accel = Vec3::new(-DRAG * vel.x, -DRAG * vel.y, -GRAVITY - DRAG * vel.z);
    let mut vel = vel + accel * dt;
    let mut pos = pos + vel * dt;

    if pos.z <= 0.0 {
        pos.z = 0.0;
        if vel.z < 0.0 {
            vel.z = -vel.z * RESTITUTION;
            vel.x *= GROUND_FRICTION;
            vel.y *= GROUND_FRICTION;
            if vel.z < REST_SPEED * 0.5 {
                vel.z = 0.0;
            }
        }
        if vel.truncate().length() < REST_SPEED {
            vel = Vec3::ZERO;
        }
    }

    (pos, vel)
}

/// What a batted ball does, worked out by running the same physics forward.
///
/// `path` is the whole trajectory, which is what lets the fielders run a real
/// pursuit against the ball instead of teleporting to a precomputed answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Flight {
    /// Where the ball first touches the ground.
    pub landing: Vec2,
    /// Seconds until it does.
    pub time_to_land: f32,
    /// Highest the ball gets.
    pub apex: f32,
    /// Cleared the wall in fair territory.
    pub home_run: bool,
    /// Where the ball finally stops rolling, and when.
    pub resting: Vec2,
    pub time_to_rest: f32,
    /// `(time, position)` samples from contact until the ball is dead.
    pub path: Vec<(f32, Vec3)>,
}

const SIM_DT: f32 = 1.0 / 240.0;
const SIM_MAX_TIME: f32 = 15.0;
/// Trajectory samples are kept every this many integration steps.
const PATH_STRIDE: usize = 4;

impl Flight {
    /// The earliest moment a fielder starting at `from` could be on the ball with
    /// it low enough to reach, and where that happens.
    ///
    /// This is a genuine pursuit: it walks the trajectory and asks when the
    /// fielder's running time stops exceeding the ball's flight time. A ball in
    /// the gap is one where that never happens while the ball is still airborne,
    /// so the fielder ends up collecting it off the turf instead.
    pub fn intercept(&self, from: Vec2, speed: f32) -> Intercept {
        for &(time, pos) in &self.path {
            if pos.z > field::CATCH_REACH {
                continue;
            }
            // A fielder cannot be standing beyond the wall, so a ball that has
            // already left the park is not his to catch.
            let ground = pos.truncate();
            if field::is_fair(ground) && ground.length() > field::fence_distance(field::spray_angle(ground)) {
                continue;
            }
            if field::FIELDER_REACTION + from.distance(ground) / speed <= time {
                return Intercept {
                    time,
                    point: ground,
                    height: pos.z,
                };
            }
        }

        // The ball outran him the whole way. It is lying still by the time he
        // arrives, so he picks it up wherever it stopped — the trajectory samples
        // end at that point, which is why the loop above cannot find this case.
        let travel = field::FIELDER_REACTION + from.distance(self.resting) / speed;
        Intercept {
            time: self.time_to_rest.max(travel),
            point: self.resting,
            height: 0.0,
        }
    }
}

/// Where and when a fielder gets to the ball.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Intercept {
    pub time: f32,
    pub point: Vec2,
    /// Height of the ball at that moment. Above the grass means a catch.
    pub height: f32,
}

impl Intercept {
    /// Whether the ball was still in the air, i.e. this is a catch rather than a
    /// ball being picked up.
    pub fn in_air(self) -> bool {
        self.height > 0.4
    }
}

/// Runs a batted ball to its conclusion.
pub fn simulate(exit_velocity: f32, launch: f32, spray: f32) -> Flight {
    let mut probe = LiveBall::default();
    probe.hit(exit_velocity, launch, spray);

    let mut pos = probe.pos;
    let mut vel = probe.vel;

    let mut apex = pos.z;
    let mut landing = None;
    let mut time_to_land = SIM_MAX_TIME;
    let mut home_run = false;
    let mut t = 0.0;
    let mut path = vec![(0.0, pos)];
    let mut steps = 0usize;

    while t < SIM_MAX_TIME {
        let (next_pos, next_vel) = step(pos, vel, SIM_DT);
        t += SIM_DT;
        steps += 1;

        apex = apex.max(next_pos.z);

        if !home_run && field::clears_fence(next_pos.truncate(), next_pos.z) {
            home_run = true;
        }

        if landing.is_none() && next_pos.z <= 0.0 {
            landing = Some(next_pos.truncate());
            time_to_land = t;
        }

        pos = next_pos;
        vel = next_vel;

        if steps.is_multiple_of(PATH_STRIDE) {
            path.push((t, pos));
        }

        if landing.is_some() && vel == Vec3::ZERO {
            break;
        }
    }

    // Always finish on the true resting place so pursuit has somewhere to end.
    path.push((t, pos));

    let landing = landing.unwrap_or_else(|| pos.truncate());

    Flight {
        landing,
        time_to_land,
        apex,
        home_run,
        resting: pos.truncate(),
        time_to_rest: t,
        path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Launch angle in degrees, for readability in the tests below.
    fn deg(d: f32) -> f32 {
        d.to_radians()
    }

    #[test]
    fn a_crushed_ball_carries_a_realistic_distance() {
        // 105 mph off the bat at 28 degrees. With no drag at all this would sail
        // 579 feet, so this is the check that keeps the air honest.
        let distance = simulate(105.0 * MPH_TO_FPS, deg(28.0), 0.0).landing.length();
        assert!(
            (390.0..=420.0).contains(&distance),
            "expected roughly 400 feet, got {distance}"
        );
    }

    #[test]
    fn a_ball_with_height_to_spare_over_the_wall_is_a_home_run() {
        let flight = simulate(108.0 * MPH_TO_FPS, deg(30.0), 0.0);
        assert!(flight.home_run, "432 feet to center should be well gone");
    }

    #[test]
    fn a_ball_that_only_just_reaches_the_wall_is_not_a_home_run() {
        // This one lands a couple of feet the other side of the wall, but by the
        // time it gets there it is barely a foot off the ground, so it hits the
        // wall instead of clearing it. Distance alone is not enough — which is
        // what makes a ball off the wall a different outcome from a home run.
        let flight = simulate(102.0 * MPH_TO_FPS, deg(28.0), 0.0);
        assert!(
            flight.landing.length() > field::fence_distance(0.0),
            "this ball should land past the wall, got {} feet",
            flight.landing.length()
        );
        assert!(!flight.home_run, "but it crosses the wall too low to clear it");
    }

    #[test]
    fn a_routine_fly_ball_stays_in_the_park() {
        let flight = simulate(88.0 * MPH_TO_FPS, deg(35.0), 0.0);
        let distance = flight.landing.length();
        assert!(
            (250.0..=360.0).contains(&distance),
            "expected an outfield fly, got {distance} feet"
        );
        assert!(!flight.home_run);
    }

    #[test]
    fn a_towering_pop_up_goes_high_and_nowhere() {
        let flight = simulate(70.0 * MPH_TO_FPS, deg(72.0), 0.0);
        assert!(flight.apex > 90.0, "a pop up should climb, got apex {}", flight.apex);
        assert!(
            flight.landing.length() < 160.0,
            "a pop up should not travel far, got {} feet",
            flight.landing.length()
        );
    }

    #[test]
    fn a_fielder_standing_where_a_fly_ball_lands_catches_it_in_the_air() {
        let flight = simulate(88.0 * MPH_TO_FPS, deg(35.0), 0.0);
        let intercept = flight.intercept(flight.landing, field::FIELDER_SPEED);
        assert!(intercept.in_air(), "he should catch it before it lands");
        assert!(intercept.time < flight.time_to_land);
    }

    #[test]
    fn a_fielder_too_far_away_cannot_catch_it_on_the_fly() {
        // A fly ball to right field, with the fielder standing in left. He has no
        // chance in the air and has to go and pick it up.
        let flight = simulate(90.0 * MPH_TO_FPS, deg(32.0), deg(30.0));
        let wrong_side = Vec2::new(-200.0, 240.0);

        let intercept = flight.intercept(wrong_side, field::FIELDER_SPEED);
        assert!(!intercept.in_air(), "he is on the other side of the outfield");
        assert!(
            intercept.time > flight.time_to_land,
            "he can only arrive after it has come down"
        );
    }

    #[test]
    fn not_every_ball_in_the_park_can_be_caught() {
        // With the real defensive alignment there must exist batted balls that
        // fall in, or the game would be unplayable: every fair ball an out.
        let mut safe = 0;
        for launch in [10.0, 14.0, 18.0, 22.0] {
            for spray in [-40.0, -16.0, 0.0, 16.0, 40.0] {
                let flight = simulate(102.0 * MPH_TO_FPS, deg(launch), deg(spray));
                if flight.home_run || !field::is_fair(flight.landing) {
                    continue;
                }
                let caught = field::FIELDER_HOMES
                    .iter()
                    .any(|&(_, spot)| flight.intercept(spot, field::FIELDER_SPEED).in_air());
                if !caught {
                    safe += 1;
                }
            }
        }
        assert!(safe > 0, "no hard-hit fair ball anywhere found a hole in the defence");
    }

    #[test]
    fn a_grounder_that_stops_rolling_still_gets_picked_up() {
        // Regression: the trajectory samples stop when the ball comes to rest, so
        // a fielder who arrives afterwards found no intercept at all and the play
        // could never be resolved.
        let flight = simulate(95.0 * MPH_TO_FPS, deg(3.0), 0.0);
        let shortstop = field::FIELDER_HOMES[4].1;

        let intercept = flight.intercept(shortstop, field::FIELDER_SPEED);
        assert!(!intercept.in_air(), "a grounder is fielded off the turf");
        assert!(
            intercept.time >= flight.time_to_rest,
            "he cannot arrive before the ball has finished rolling"
        );
        assert!(intercept.point.distance(flight.resting) < 1.0);
    }

    #[test]
    fn a_pursuing_fielder_never_arrives_before_he_could_have_run_there() {
        // The intercept contract: running time must fit inside the arrival time,
        // otherwise the fielder is teleporting.
        for (ev, launch, spray) in [(92.0, 30.0, 15.0), (78.0, 8.0, -35.0), (101.0, 45.0, 5.0)] {
            let flight = simulate(ev * MPH_TO_FPS, deg(launch), deg(spray));
            for (position, spot) in field::FIELDER_HOMES {
                let intercept = flight.intercept(spot, field::FIELDER_SPEED);
                let run = spot.distance(intercept.point) / field::FIELDER_SPEED;
                assert!(
                    run <= intercept.time + 1e-3,
                    "{position} needed {run}s to cover the ground but arrived at {}s",
                    intercept.time
                );
            }
        }
    }

    #[test]
    fn the_recorded_path_agrees_with_the_summary() {
        let flight = simulate(100.0 * MPH_TO_FPS, deg(25.0), 0.0);
        assert!(flight.path.len() > 10, "the path should be sampled, not empty");

        let (last_time, last_pos) = *flight.path.last().expect("the path is never empty");
        assert!(
            last_pos.truncate().distance(flight.resting) < 1e-3,
            "the path should end where the ball came to rest"
        );
        assert!((last_time - flight.time_to_rest).abs() < 1e-3);
        // Times increase monotonically, or `at` and `intercept` are meaningless.
        for pair in flight.path.windows(2) {
            assert!(pair[1].0 >= pair[0].0);
        }
    }

    #[test]
    fn pulling_the_ball_sends_it_to_left_field() {
        let pulled = simulate(95.0 * MPH_TO_FPS, deg(25.0), deg(-30.0));
        assert!(pulled.landing.x < 0.0, "a negative spray angle is left field");
        assert!(field::is_fair(pulled.landing), "30 degrees is inside the foul line");

        let sliced = simulate(95.0 * MPH_TO_FPS, deg(25.0), deg(30.0));
        assert!(sliced.landing.x > 0.0, "a positive spray angle is right field");
        // Symmetric: same distance either way.
        assert!((pulled.landing.length() - sliced.landing.length()).abs() < 1.0);
    }

    #[test]
    fn a_ball_hit_outside_the_foul_lines_lands_foul() {
        let flight = simulate(95.0 * MPH_TO_FPS, deg(25.0), deg(60.0));
        assert!(!field::is_fair(flight.landing));
        assert!(!flight.home_run, "a foul ball is never a home run");
    }

    #[test]
    fn harder_contact_travels_further() {
        let mut previous = 0.0;
        for mph in [70.0, 80.0, 90.0, 100.0, 110.0] {
            let distance = simulate(mph * MPH_TO_FPS, deg(28.0), 0.0).landing.length();
            assert!(
                distance > previous,
                "{mph} mph went {distance} feet, not further than the previous {previous}"
            );
            previous = distance;
        }
    }

    #[test]
    fn a_ball_always_comes_to_rest() {
        // Nothing should still be moving when the simulation gives up, or the
        // fielding code would wait forever for a ball that never stops.
        for launch in [1.0, 10.0, 30.0, 50.0, 70.0] {
            let flight = simulate(100.0 * MPH_TO_FPS, deg(launch), 0.0);
            assert!(flight.time_to_rest < SIM_MAX_TIME, "launch {launch} never came to rest");
            assert!(flight.time_to_land <= flight.time_to_rest);
        }
    }

    #[test]
    fn a_rolling_ball_loses_speed_and_stops() {
        let mut pos = Vec3::new(0.0, 40.0, 0.0);
        let mut vel = Vec3::new(0.0, 60.0, 0.0);
        for _ in 0..2000 {
            let (p, v) = step(pos, vel, SIM_DT);
            pos = p;
            vel = v;
        }
        assert_eq!(vel, Vec3::ZERO, "a grounder should stop rolling");
        assert!(pos.z.abs() < 1e-6, "it should be sitting on the ground");
    }
}

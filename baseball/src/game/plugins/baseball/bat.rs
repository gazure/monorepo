//! Swinging the bat, and what contact produces.
//!
//! The important thing here is what this module *does not* do: it never decides
//! whether the batter got a single or flied out. It converts a swing into an exit
//! velocity, a launch angle and a spray angle, and then the ball and the fielders
//! settle the argument. Previously a lookup on swing timing chose the outcome
//! outright, and any well-timed swing was a guaranteed home run.
//!
//! A swing has two independent parts. *Timing* decides how flush the contact is
//! and which way the ball is pulled. *Style* is the shape of the swing — how high
//! the bat travels and how steeply it climbs — and that is what turns the same
//! contact into a grounder or a fly ball. Choosing a style is a real gamble: only
//! an uppercut can drive a ball out of the park, and an uppercut cannot reach a
//! pitch at the top of the zone at all.

use super::{
    ball::MPH_TO_FPS,
    pitch::{self, LivePitch},
};

/// Swing this far from the moment the ball crosses the plate and you miss it
/// entirely. About a third of a fastball's flight, so timing is genuinely tight.
pub const TIMING_WINDOW: f32 = 0.155;

/// Miss the ball's height by this much and the bat goes clean over or under it.
pub const HEIGHT_WINDOW: f32 = 1.05;

/// Exit velocity range in miles per hour, from a scraped foul tip to the best
/// contact the batter is capable of.
const EV_WEAK: f32 = 52.0;
const EV_BEST: f32 = 108.0;

/// Degrees of launch angle gained per foot of "swinging under the ball". Most of
/// the launch angle comes from the swing's own attack angle; this is the part the
/// pitch location contributes.
const LAUNCH_PER_FOOT: f32 = 18.0;
const LAUNCH_MIN: f32 = -14.0;
const LAUNCH_MAX: f32 = 58.0;

/// Spray angle at the very edge of the timing window, in degrees. Deliberately
/// wider than the 45° foul line so the worst-timed contact hooks foul — that is
/// where foul balls come from, rather than a special case in the code.
const SPRAY_LIMIT: f32 = 52.0;

/// Degrees of pull per foot the pitch is inside, on top of whatever the timing
/// contributes. Without this, spray depends on timing alone, which means the only
/// way to pull a ball is to mis-hit it — so every home run goes to dead centre and
/// pulling is punished instead of rewarded. An inside pitch gets hooked to the
/// batter's pull side; an outside one goes the other way.
const PULL_PER_FOOT: f32 = 20.0;

/// Nothing can be sprayed further round than this, so a wild swing cannot send
/// the ball behind the batter.
const SPRAY_CLAMP: f32 = 75.0;

/// The shape of the swing. The batter picks one before the ball arrives.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwingStyle {
    /// Flat and slightly high in the zone. Keeps the ball down; cannot lift it.
    Level,
    /// The default. A modest upward path through the middle of the zone.
    #[default]
    Normal,
    /// Low and steep. The only way to drive a ball out, and helpless against
    /// anything at the top of the zone.
    Lift,
}

impl SwingStyle {
    /// Height the bat travels through, in feet.
    pub fn plane(self) -> f32 {
        pitch::ZONE_MID
            + match self {
                SwingStyle::Level => 0.25,
                SwingStyle::Normal => 0.0,
                SwingStyle::Lift => -0.30,
            }
    }

    /// Upward tilt of the swing path, in degrees.
    pub fn attack(self) -> f32 {
        match self {
            SwingStyle::Level => 3.0,
            SwingStyle::Normal => 13.0,
            SwingStyle::Lift => 27.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SwingStyle::Level => "LEVEL",
            SwingStyle::Normal => "NORMAL",
            SwingStyle::Lift => "LIFT",
        }
    }
}

/// A swing the batter has committed to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Swing {
    /// Seconds between the bat arriving and the ball crossing the plate.
    /// Negative is early, positive is late.
    pub timing: f32,
    pub style: SwingStyle,
}

/// What the bat did to the ball.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Contact {
    /// Swung through it.
    Whiff,
    /// Put it in play. Whether that is a hit, an out or a foul ball is for the
    /// trajectory and the defence to decide.
    Struck {
        /// Feet per second.
        exit_velocity: f32,
        /// Radians above the horizontal.
        launch: f32,
        /// Radians from dead centre; negative pulls to left field.
        spray: f32,
        /// `0.0` to `1.0`, for picking the sound and the dust.
        quality: f32,
    },
}

/// Works out what happens when this swing meets this pitch.
pub fn resolve(pitch: &LivePitch, swing: Swing) -> Contact {
    let crossing = pitch.crossing();

    if swing.timing.abs() >= TIMING_WINDOW {
        return Contact::Whiff;
    }

    let height_error = crossing.y - swing.style.plane();
    if height_error.abs() >= HEIGHT_WINDOW {
        return Contact::Whiff;
    }

    // Both errors matter and they compound: being on time is no use if the bat is
    // a foot under the ball. Both fall off quadratically, so there is a usable
    // sweet spot rather than a knife edge — a linear penalty made it impossible to
    // pull a ball with any authority.
    let timing_quality = 1.0 - (swing.timing / TIMING_WINDOW).powi(2);
    let height_quality = 1.0 - (height_error / HEIGHT_WINDOW).powi(2);
    let quality = (timing_quality * height_quality).clamp(0.0, 1.0);

    let exit_velocity = (EV_WEAK + (EV_BEST - EV_WEAK) * quality) * MPH_TO_FPS;

    // The swing's own attack angle does most of the work; catching the ball above
    // or below the plane tilts it further up or beats it into the ground.
    let launch = (swing.style.attack() + height_error * LAUNCH_PER_FOOT).clamp(LAUNCH_MIN, LAUNCH_MAX);

    // Early swings pull the ball, late swings push it the other way. A
    // right-handed batter pulls to left field, which is negative on the field's
    // x-axis, so an early (negative) swing gives a negative spray angle. Pitch
    // location adds to that: inside pitches (also negative x) get pulled.
    let spray =
        ((swing.timing / TIMING_WINDOW) * SPRAY_LIMIT + crossing.x * PULL_PER_FOOT).clamp(-SPRAY_CLAMP, SPRAY_CLAMP);

    Contact::Struck {
        exit_velocity,
        launch: launch.to_radians(),
        spray: spray.to_radians(),
        quality,
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec2;

    use super::{
        super::{ball, field, pitch::PitchPlan},
        *,
    };

    fn pitch_at(x: f32, y: f32) -> LivePitch {
        LivePitch::thrown(PitchPlan {
            kind: pitch::PitchKind::Fastball,
            target: Vec2::new(x, y),
        })
    }

    struct Hit {
        exit_velocity: f32,
        launch_degrees: f32,
        spray_degrees: f32,
        quality: f32,
    }

    fn swing_at(pitch: &LivePitch, timing: f32, style: SwingStyle) -> Hit {
        match resolve(pitch, Swing { timing, style }) {
            Contact::Struck {
                exit_velocity,
                launch,
                spray,
                quality,
            } => Hit {
                exit_velocity,
                launch_degrees: launch.to_degrees(),
                spray_degrees: spray.to_degrees(),
                quality,
            },
            Contact::Whiff => panic!("expected contact, got a whiff"),
        }
    }

    fn flight_of(hit: &Hit) -> ball::Flight {
        ball::simulate(
            hit.exit_velocity,
            hit.launch_degrees.to_radians(),
            hit.spray_degrees.to_radians(),
        )
    }

    #[test]
    fn swinging_far_too_early_or_late_misses_completely() {
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        for timing in [-0.5, -TIMING_WINDOW, TIMING_WINDOW, 0.4] {
            assert_eq!(
                resolve(
                    &pitch,
                    Swing {
                        timing,
                        style: SwingStyle::Normal
                    }
                ),
                Contact::Whiff,
                "timing {timing} should have missed"
            );
        }
    }

    #[test]
    fn an_uppercut_cannot_reach_the_top_of_the_zone() {
        // The cost of choosing to swing for the fences.
        let high = pitch_at(0.0, pitch::ZONE_TOP);
        assert_eq!(
            resolve(
                &high,
                Swing {
                    timing: 0.0,
                    style: SwingStyle::Lift
                }
            ),
            Contact::Whiff,
            "a lift swing is too low to catch a pitch at the letters"
        );
        // A normal swing handles it.
        assert!(matches!(
            resolve(
                &high,
                Swing {
                    timing: 0.0,
                    style: SwingStyle::Normal
                }
            ),
            Contact::Struck { .. }
        ));
    }

    #[test]
    fn perfect_timing_down_the_middle_is_the_hardest_contact() {
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        let hit = swing_at(&pitch, 0.0, SwingStyle::Normal);

        assert!((hit.quality - 1.0).abs() < 1e-5, "this is the best possible swing");
        assert!((hit.exit_velocity / MPH_TO_FPS - EV_BEST).abs() < 0.1);
        assert!(hit.spray_degrees.abs() < 1e-5, "dead-centre timing goes up the middle");
    }

    #[test]
    fn perfect_timing_is_no_longer_an_automatic_home_run() {
        // The regression that motivated all of this: the old code returned
        // `PitchOutcome::HomeRun` for any swing whose timing quality passed 0.9,
        // regardless of what the ball then did. Flawless timing with the wrong
        // swing shape now produces a ground ball.
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        let hit = swing_at(&pitch, 0.0, SwingStyle::Level);

        assert!((hit.quality - 1.0).abs() < 0.1, "he squared it up");
        assert!(hit.launch_degrees < 0.0, "a level swing beats it into the ground");
        assert!(!flight_of(&hit).home_run, "a ground ball is not a home run");
    }

    #[test]
    fn only_an_uppercut_can_drive_the_ball_out_of_the_park() {
        // A hittable pitch, squared up perfectly, with each swing shape in turn.
        let pitch = pitch_at(0.0, pitch::ZONE_MID - 0.4);

        let lift = flight_of(&swing_at(&pitch, 0.0, SwingStyle::Lift));
        let normal = flight_of(&swing_at(&pitch, 0.0, SwingStyle::Normal));
        let level = flight_of(&swing_at(&pitch, 0.0, SwingStyle::Level));

        assert!(lift.home_run, "the lift swing should carry out");
        assert!(!normal.home_run, "a normal swing is a line drive");
        assert!(!level.home_run);
        assert!(
            lift.landing.length() > normal.landing.length(),
            "lift {} should out-travel normal {}",
            lift.landing.length(),
            normal.landing.length()
        );
        assert!(normal.landing.length() > level.landing.length());
    }

    #[test]
    fn an_inside_pitch_is_pulled_and_an_outside_one_goes_the_other_way() {
        // Location, not just timing, decides where the ball goes. Without this the
        // only way to pull would be to mis-time the swing.
        let inside = swing_at(&pitch_at(-0.7, pitch::ZONE_MID), 0.0, SwingStyle::Normal);
        let middle = swing_at(&pitch_at(0.0, pitch::ZONE_MID), 0.0, SwingStyle::Normal);
        let outside = swing_at(&pitch_at(0.7, pitch::ZONE_MID), 0.0, SwingStyle::Normal);

        assert!(inside.spray_degrees < -5.0, "an inside pitch should be pulled");
        assert!(
            middle.spray_degrees.abs() < 1e-4,
            "a pitch down the middle goes straight"
        );
        assert!(outside.spray_degrees > 5.0, "an outside pitch goes the other way");

        assert!(flight_of(&inside).landing.x < 0.0);
        assert!(flight_of(&outside).landing.x > 0.0);
    }

    #[test]
    fn a_pulled_ball_can_be_hit_out_without_being_mis_timed() {
        // The payoff for the location term: a batter who turns on an inside pitch
        // gets a home run down the line at full exit velocity, rather than having
        // to spray the ball by hitting it badly.
        let inside = swing_at(&pitch_at(-0.7, pitch::ZONE_MID - 0.3), 0.0, SwingStyle::Lift);
        assert!(inside.quality > 0.9, "this is flush contact, not a mis-hit");

        let flight = flight_of(&inside);
        assert!(flight.home_run, "a pulled fly ball should leave the park");
        assert!(flight.landing.x < 0.0, "and it should land in left field");
    }

    #[test]
    fn home_runs_are_not_the_only_thing_a_good_swing_produces() {
        // Guard against over-tuning: sweeping the plausible swing space, home runs
        // should be a small minority of contact, and only from the lift swing.
        let mut contacts = 0;
        let mut home_runs = 0;
        let mut lift_only = true;

        for style in [SwingStyle::Level, SwingStyle::Normal, SwingStyle::Lift] {
            for &x in &[-0.6_f32, 0.0, 0.6] {
                for &height in &[1.9_f32, 2.5, 3.1] {
                    for &timing in &[-0.10_f32, -0.05, 0.0, 0.05, 0.10] {
                        let pitch = pitch_at(x, height);
                        let Contact::Struck {
                            exit_velocity,
                            launch,
                            spray,
                            ..
                        } = resolve(&pitch, Swing { timing, style })
                        else {
                            continue;
                        };
                        contacts += 1;
                        if ball::simulate(exit_velocity, launch, spray).home_run {
                            home_runs += 1;
                            lift_only &= style == SwingStyle::Lift;
                        }
                    }
                }
            }
        }

        assert!(contacts > 50, "the sweep should make contact plenty of times");
        assert!(home_runs > 0, "a home run has to be possible");
        let rate = home_runs as f32 / contacts as f32;
        assert!(rate < 0.20, "home runs should be rare, got {:.0}%", rate * 100.0);
        assert!(lift_only, "only the lift swing should be capable of a home run");
    }

    #[test]
    fn swinging_early_pulls_the_ball_and_late_pushes_it() {
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        let early = swing_at(&pitch, -0.09, SwingStyle::Normal);
        let late = swing_at(&pitch, 0.09, SwingStyle::Normal);

        assert!(early.spray_degrees < 0.0, "an early swing pulls to left field");
        assert!(late.spray_degrees > 0.0, "a late swing goes to right field");
        assert!((early.spray_degrees + late.spray_degrees).abs() < 1e-4, "symmetric");

        assert!(flight_of(&early).landing.x < 0.0);
        assert!(flight_of(&late).landing.x > 0.0);
    }

    #[test]
    fn the_worst_timed_contact_hooks_into_foul_territory() {
        // Foul balls are meant to emerge from the geometry, not a special case.
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        let barely = TIMING_WINDOW * 0.99;

        for timing in [-barely, barely] {
            let hit = swing_at(&pitch, timing, SwingStyle::Normal);
            assert!(
                hit.spray_degrees.abs() > field::FOUL_LINE_ANGLE.to_degrees(),
                "timing {timing} sprayed {} degrees, still inside the foul line",
                hit.spray_degrees
            );
            assert!(!field::is_fair(flight_of(&hit).landing), "so the ball should land foul");
        }
    }

    #[test]
    fn a_high_pitch_launches_higher_than_a_low_one_for_the_same_swing() {
        let low = swing_at(&pitch_at(0.0, pitch::ZONE_BOTTOM + 0.05), 0.0, SwingStyle::Normal);
        let high = swing_at(&pitch_at(0.0, pitch::ZONE_TOP - 0.05), 0.0, SwingStyle::Normal);

        assert!(high.launch_degrees > low.launch_degrees);
        assert!(low.launch_degrees < 5.0, "a pitch at the knees stays down");
    }

    #[test]
    fn better_timing_is_always_hit_harder() {
        let pitch = pitch_at(0.0, pitch::ZONE_MID);
        let mut previous = 0.0;
        for timing in [0.14, 0.10, 0.06, 0.03, 0.0] {
            let hit = swing_at(&pitch, timing, SwingStyle::Normal);
            assert!(
                hit.exit_velocity > previous,
                "timing {timing} should beat the previous exit velocity"
            );
            previous = hit.exit_velocity;
        }
    }

    #[test]
    fn every_swing_style_has_a_pitch_it_handles_best() {
        // No style should be strictly dominant, or the choice is not a choice.
        let heights = [pitch::ZONE_BOTTOM, pitch::ZONE_MID, pitch::ZONE_TOP - 0.05];
        for style in [SwingStyle::Level, SwingStyle::Normal, SwingStyle::Lift] {
            let best = heights
                .iter()
                .filter_map(|&h| match resolve(&pitch_at(0.0, h), Swing { timing: 0.0, style }) {
                    Contact::Struck { quality, .. } => Some(quality),
                    Contact::Whiff => None,
                })
                .fold(0.0_f32, f32::max);
            assert!(
                best > 0.7,
                "{} has no pitch height it handles well (best quality {best})",
                style.label()
            );
        }
    }

    #[test]
    fn the_quality_penalty_for_missing_the_height_is_gentle_near_the_plane() {
        // A quadratic falloff is what makes a usable sweet spot; a linear one made
        // it impossible to hit a hard fly ball at all.
        let pitch = pitch_at(0.0, pitch::ZONE_MID + 0.2);
        let hit = swing_at(&pitch, 0.0, SwingStyle::Normal);
        assert!(
            hit.quality > 0.9,
            "two inches off the plane should barely matter, got {}",
            hit.quality
        );
    }
}

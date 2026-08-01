//! Pitching: what was thrown, where it ended up, and what the umpire made of it.
//!
//! Everything is expressed in **plate coordinates**: `x` is feet either side of
//! the middle of the plate (positive toward first base, which is the viewer's
//! right in the at-bat view) and `y` is feet above the ground. A pitch is aimed at
//! a plate coordinate and arrives there; the break is what makes it *look* like it
//! was going somewhere else on the way.
//!
//! This is the module that makes balls, fouls and walks reachable at all. The old
//! code hardcoded every un-swung pitch to a strike, which left `Balls`,
//! `CountResult::Walk` and `HitByPitch` unreachable from the game.

use bevy::prelude::*;

/// Half the width of the strike zone: the plate is 17 inches across, plus a
/// little for the width of the ball.
pub const ZONE_HALF_WIDTH: f32 = 0.83;
pub const ZONE_BOTTOM: f32 = 1.6;
pub const ZONE_TOP: f32 = 3.4;
/// Belt high, and the height the bat swings through by default.
pub const ZONE_MID: f32 = ZONE_BOTTOM + (ZONE_TOP - ZONE_BOTTOM) / 2.0;

/// How far outside the zone a pitch can be aimed.
pub const AIM_LIMIT_X: f32 = 1.7;
pub const AIM_LIMIT_LOW: f32 = 0.7;
pub const AIM_LIMIT_HIGH: f32 = 4.4;

/// Roughly where the ball leaves the pitcher's hand, in plate coordinates. A
/// right-handed pitcher releases from a three-quarter slot, up and to the side.
pub const RELEASE: Vec2 = Vec2::new(0.9, 6.0);

/// Distance the ball actually travels: the rubber is 60'6" away but the pitcher
/// strides most of the way through it before letting go.
pub const RELEASE_DISTANCE: f32 = 55.0;

/// Hit by pitch: inside this far past the inside corner and the batter wears it.
const HBP_X: f32 = -1.75;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PitchKind {
    Fastball,
    Slider,
    Curveball,
    Changeup,
}

pub const PITCH_KINDS: [PitchKind; 4] = [
    PitchKind::Fastball,
    PitchKind::Slider,
    PitchKind::Curveball,
    PitchKind::Changeup,
];

impl PitchKind {
    /// Miles per hour.
    pub fn speed(self) -> f32 {
        match self {
            PitchKind::Fastball => 94.0,
            PitchKind::Slider => 86.0,
            PitchKind::Curveball => 79.0,
            PitchKind::Changeup => 81.0,
        }
    }

    /// How far the ball deviates from where it appeared to be heading, in feet.
    ///
    /// Signs are for a right-handed pitcher facing a right-handed batter, who
    /// stands on the third-base side (negative `x`): the slider runs away from
    /// him, the curveball drops and bores in.
    pub fn break_vector(self) -> Vec2 {
        match self {
            PitchKind::Fastball => Vec2::new(0.0, -0.10),
            PitchKind::Slider => Vec2::new(0.90, -0.50),
            PitchKind::Curveball => Vec2::new(-0.20, -1.50),
            PitchKind::Changeup => Vec2::new(0.25, -0.75),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            PitchKind::Fastball => "FASTBALL",
            PitchKind::Slider => "SLIDER",
            PitchKind::Curveball => "CURVEBALL",
            PitchKind::Changeup => "CHANGEUP",
        }
    }

    /// Seconds from release until the ball reaches the plate.
    pub fn flight_time(self) -> f32 {
        RELEASE_DISTANCE / (self.speed() * super::ball::MPH_TO_FPS)
    }
}

/// A pitch chosen but not yet thrown.
#[derive(Debug, Clone, Copy, Resource)]
pub struct PitchPlan {
    pub kind: PitchKind,
    /// Plate coordinate the pitcher is aiming at.
    pub target: Vec2,
}

impl Default for PitchPlan {
    fn default() -> Self {
        Self {
            kind: PitchKind::Fastball,
            target: Vec2::new(0.0, ZONE_MID),
        }
    }
}

impl PitchPlan {
    /// Nudges the aim, keeping it within reach of the plate.
    pub fn aim(&mut self, delta: Vec2) {
        self.target.x = (self.target.x + delta.x).clamp(-AIM_LIMIT_X, AIM_LIMIT_X);
        self.target.y = (self.target.y + delta.y).clamp(AIM_LIMIT_LOW, AIM_LIMIT_HIGH);
    }

    pub fn cycle_kind(&mut self, forward: bool) {
        let index = PITCH_KINDS.iter().position(|&k| k == self.kind).unwrap_or(0);
        let count = PITCH_KINDS.len();
        let next = if forward {
            (index + 1) % count
        } else {
            (index + count - 1) % count
        };
        self.kind = PITCH_KINDS[next];
    }
}

/// A pitch in flight.
#[derive(Debug, Clone, Copy, Resource)]
pub struct LivePitch {
    pub kind: PitchKind,
    /// Where it will actually cross the plate.
    pub target: Vec2,
    pub elapsed: f32,
    pub flight: f32,
    /// Set once the batter has committed, so a second press cannot swing twice.
    pub swung: bool,
}

impl Default for LivePitch {
    fn default() -> Self {
        Self {
            kind: PitchKind::Fastball,
            target: Vec2::new(0.0, ZONE_MID),
            elapsed: 0.0,
            flight: PitchKind::Fastball.flight_time(),
            swung: false,
        }
    }
}

impl LivePitch {
    pub fn thrown(plan: PitchPlan) -> Self {
        Self {
            kind: plan.kind,
            target: plan.target,
            elapsed: 0.0,
            flight: plan.kind.flight_time(),
            swung: false,
        }
    }

    /// How far along the flight the ball is, from `0` at release to `1` at the
    /// plate. Keeps climbing past `1` so the ball can carry on into the mitt.
    pub fn progress(&self) -> f32 {
        self.elapsed / self.flight
    }

    /// Plate coordinate the ball occupies at a given point in its flight.
    ///
    /// The ball tracks toward an *apparent* target and the break pulls it away
    /// late, which is why it arrives exactly on `target` while never having looked
    /// like it was going there. That late deviation is the whole difficulty of a
    /// breaking ball.
    pub fn spot_at(&self, progress: f32) -> Vec2 {
        let bend = self.kind.break_vector();
        let apparent = self.target - bend;
        RELEASE.lerp(apparent, progress) + bend * progress * progress
    }

    /// Where the ball crosses the plate, which is what the umpire judges.
    pub fn crossing(&self) -> Vec2 {
        self.target
    }

    /// Whether the ball has reached the plate yet.
    pub fn reached_plate(&self) -> bool {
        self.elapsed >= self.flight
    }

    /// Far enough past the plate that the catcher has it and the pitch is over.
    pub fn past_catcher(&self) -> bool {
        self.elapsed >= self.flight * 1.35
    }
}

/// Whether a pitch crossing at this spot is a strike.
pub fn in_zone(spot: Vec2) -> bool {
    spot.x.abs() <= ZONE_HALF_WIDTH && (ZONE_BOTTOM..=ZONE_TOP).contains(&spot.y)
}

/// Whether the pitch got away far enough inside to hit the batter.
pub fn hits_batter(spot: Vec2) -> bool {
    spot.x <= HBP_X && spot.y < 5.0
}

/// How far outside the zone a spot is, in feet. Zero inside. Drives how likely a
/// batter is to lay off, and how tempting a pitch looks.
pub fn distance_outside(spot: Vec2) -> f32 {
    let dx = (spot.x.abs() - ZONE_HALF_WIDTH).max(0.0);
    let dy = if spot.y > ZONE_TOP {
        spot.y - ZONE_TOP
    } else if spot.y < ZONE_BOTTOM {
        ZONE_BOTTOM - spot.y
    } else {
        0.0
    };
    Vec2::new(dx, dy).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pitch_always_arrives_exactly_where_it_was_aimed() {
        // The break bends the path but must not move the destination, otherwise
        // the pitcher cannot aim and the umpire is judging the wrong spot.
        for kind in PITCH_KINDS {
            for target in [
                Vec2::new(0.0, ZONE_MID),
                Vec2::new(-0.7, 1.8),
                Vec2::new(0.8, 3.3),
                Vec2::new(1.5, 0.9),
            ] {
                let pitch = LivePitch::thrown(PitchPlan { kind, target });
                let arrival = pitch.spot_at(1.0);
                assert!(
                    arrival.distance(target) < 1e-4,
                    "{kind:?} aimed at {target:?} arrived at {arrival:?}"
                );
            }
        }
    }

    #[test]
    fn a_pitch_starts_from_the_release_point() {
        let pitch = LivePitch::thrown(PitchPlan::default());
        assert!(pitch.spot_at(0.0).distance(RELEASE) < 1e-4);
    }

    #[test]
    fn a_breaking_ball_deviates_from_the_straight_line_but_a_fastball_barely_does() {
        let target = Vec2::new(0.0, ZONE_MID);

        let deviation = |kind: PitchKind| {
            let pitch = LivePitch::thrown(PitchPlan { kind, target });
            // Compare the real path against a straight line from release to target.
            (0..=10)
                .map(|i| {
                    let t = i as f32 / 10.0;
                    let straight = RELEASE.lerp(target, t);
                    pitch.spot_at(t).distance(straight)
                })
                .fold(0.0_f32, f32::max)
        };

        let curve = deviation(PitchKind::Curveball);
        let fastball = deviation(PitchKind::Fastball);
        assert!(
            curve > 0.3,
            "a curveball should visibly leave the straight line, got {curve}"
        );
        assert!(fastball < 0.1, "a fastball should look straight, got {fastball}");
        assert!(curve > fastball * 4.0);
    }

    #[test]
    fn a_curveball_takes_longer_to_arrive_than_a_fastball() {
        assert!(PitchKind::Curveball.flight_time() > PitchKind::Fastball.flight_time());
        // Sanity: a big-league fastball reaches the plate in under half a second.
        let fastball = PitchKind::Fastball.flight_time();
        assert!(
            (0.35..=0.45).contains(&fastball),
            "fastball flight time of {fastball}s is not realistic"
        );
    }

    #[test]
    fn the_strike_zone_has_the_edges_you_would_expect() {
        assert!(in_zone(Vec2::new(0.0, ZONE_MID)), "down the middle");
        assert!(in_zone(Vec2::new(ZONE_HALF_WIDTH - 0.01, ZONE_BOTTOM + 0.01)), "corner");
        assert!(!in_zone(Vec2::new(ZONE_HALF_WIDTH + 0.05, ZONE_MID)), "just outside");
        assert!(!in_zone(Vec2::new(0.0, ZONE_TOP + 0.05)), "high");
        assert!(!in_zone(Vec2::new(0.0, ZONE_BOTTOM - 0.05)), "low");
    }

    #[test]
    fn distance_outside_is_zero_in_the_zone_and_grows_beyond_it() {
        assert!(distance_outside(Vec2::new(0.0, ZONE_MID)).abs() < 1e-6);

        let close = distance_outside(Vec2::new(ZONE_HALF_WIDTH + 0.2, ZONE_MID));
        let far = distance_outside(Vec2::new(ZONE_HALF_WIDTH + 1.0, ZONE_MID));
        assert!(far > close && close > 0.0);

        // Symmetric, and works vertically too.
        let high = distance_outside(Vec2::new(0.0, ZONE_TOP + 0.5));
        let low = distance_outside(Vec2::new(0.0, ZONE_BOTTOM - 0.5));
        assert!((high - low).abs() < 1e-5);
    }

    #[test]
    fn a_pitch_way_inside_hits_the_batter_but_a_strike_never_does() {
        assert!(hits_batter(Vec2::new(-2.0, 2.5)));
        assert!(!hits_batter(Vec2::new(0.0, ZONE_MID)));
        assert!(!hits_batter(Vec2::new(1.5, 2.5)), "outside cannot hit a righty");
    }

    #[test]
    fn aiming_is_clamped_to_somewhere_near_the_plate() {
        let mut plan = PitchPlan::default();
        for _ in 0..100 {
            plan.aim(Vec2::new(1.0, 1.0));
        }
        assert!(plan.target.x <= AIM_LIMIT_X && plan.target.y <= AIM_LIMIT_HIGH);

        for _ in 0..200 {
            plan.aim(Vec2::new(-1.0, -1.0));
        }
        assert!(plan.target.x >= -AIM_LIMIT_X && plan.target.y >= AIM_LIMIT_LOW);
    }

    #[test]
    fn cycling_pitch_kinds_wraps_around_in_both_directions() {
        let mut plan = PitchPlan::default();
        let start = plan.kind;
        for _ in 0..PITCH_KINDS.len() {
            plan.cycle_kind(true);
        }
        assert_eq!(plan.kind, start, "a full cycle forward returns to the start");

        plan.cycle_kind(false);
        assert_eq!(plan.kind, PITCH_KINDS[PITCH_KINDS.len() - 1], "backwards wraps");
    }

    #[test]
    fn a_pitch_reaches_the_plate_before_it_reaches_the_catcher() {
        let mut pitch = LivePitch::thrown(PitchPlan::default());
        pitch.elapsed = pitch.flight;
        assert!(pitch.reached_plate());
        assert!(!pitch.past_catcher(), "it has only just crossed");

        pitch.elapsed = pitch.flight * 1.4;
        assert!(pitch.past_catcher());
    }
}

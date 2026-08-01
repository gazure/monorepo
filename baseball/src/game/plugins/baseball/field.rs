//! The one and only description of where things are on the field.
//!
//! Everything here is in **feet**, with home plate at the origin, `+Y` pointing
//! at center field and `+X` at the right-field line. The field camera renders one
//! world unit per foot, so these numbers *are* the world coordinates — there is
//! no second coordinate system to keep in sync.
//!
//! This module exists because the previous layout drew the bases as children of a
//! 45°-rotated square while positioning the nine fielders in unrotated world
//! space. The two disagreed, and nothing in the code could tell you which was
//! right. Deriving every position from real dimensions removes the question.

use baseball_game_rules::{Base, PlayerPosition};
use bevy::prelude::*;

/// Distance between consecutive bases.
pub const BASE_PATH: f32 = 90.0;

/// Bases sit on a square rotated 45°, so each leg splits into equal components.
const BASE_LEG: f32 = BASE_PATH * std::f32::consts::FRAC_1_SQRT_2;

pub const HOME: Vec2 = Vec2::new(0.0, 0.0);
pub const FIRST: Vec2 = Vec2::new(BASE_LEG, BASE_LEG);
pub const SECOND: Vec2 = Vec2::new(0.0, BASE_LEG * 2.0);
pub const THIRD: Vec2 = Vec2::new(-BASE_LEG, BASE_LEG);

/// Front edge of the pitcher's rubber.
pub const MOUND: Vec2 = Vec2::new(0.0, 60.5);

/// Where the catcher sets up, behind the plate.
pub const CATCHER: Vec2 = Vec2::new(0.0, -8.0);

/// Radius of the infield dirt, measured from the mound.
pub const INFIELD_DIRT_RADIUS: f32 = 95.0;

/// The foul lines leave home plate at 45° either side of center, which is exactly
/// the diagonal through first and third base.
pub const FOUL_LINE_ANGLE: f32 = std::f32::consts::FRAC_PI_4;

/// Fence distance straight up the middle and down the lines. A touch shorter than
/// a real park: the hardest contact the batter can produce carries about 410 feet,
/// and a ball has to clear the wall *with height to spare* rather than merely
/// reach it, so a deeper fence would put home runs out of reach entirely.
const FENCE_CENTER: f32 = 385.0;
const FENCE_CORNER: f32 = 320.0;

/// Height of the outfield wall, which a fly ball has to clear.
pub const FENCE_HEIGHT: f32 = 8.0;

/// Highest a fielder can reach to pull a ball down.
pub const CATCH_REACH: f32 = 9.0;

/// Seconds a batter needs to reach first base. Roughly average; used to decide
/// infield hits by racing the throw against it.
pub const RUN_TO_FIRST: f32 = 4.3;

/// Seconds to cover one base once already running.
pub const RUN_PER_BASE: f32 = 3.6;

/// How fast a fielder closes on the ball, in feet per second. Deliberately below
/// a sprinter's top speed: it stands in for the whole business of reading the
/// ball, accelerating and running a route rather than a straight line.
pub const FIELDER_SPEED: f32 = 21.0;

/// Dead time between the crack of the bat and a fielder actually moving. Without
/// it the defence reacts instantly and converts almost everything: this single
/// constant is the difference between a playable game and every ball being an out.
pub const FIELDER_REACTION: f32 = 0.42;

/// How fast a thrown ball travels, in feet per second.
pub const THROW_SPEED: f32 = 115.0;

/// Time a fielder spends collecting the ball and getting rid of it.
pub const FIELDING_DELAY: f32 = 0.55;

/// Spray angle of a batted ball: `0` is dead center, negative pulls to left field
/// and positive slices to right. Fair territory is the ±45° wedge.
pub fn spray_angle(point: Vec2) -> f32 {
    point.x.atan2(point.y)
}

/// Turns a spray angle and a distance back into a field position.
pub fn point_at(spray: f32, distance: f32) -> Vec2 {
    Vec2::new(distance * spray.sin(), distance * spray.cos())
}

/// Fair territory is bounded by the two foul lines through first and third, so a
/// point is fair when it is no further sideways than it is deep.
pub fn is_fair(point: Vec2) -> bool {
    point.y > 0.0 && point.x.abs() <= point.y
}

/// Distance to the wall along a given spray angle. Deepest to center and
/// shortest down the lines, bulging out through the power alleys the way a real
/// outfield does rather than tapering in a straight line.
pub fn fence_distance(spray: f32) -> f32 {
    let t = (spray.abs() / FOUL_LINE_ANGLE).clamp(0.0, 1.0);
    FENCE_CENTER - (FENCE_CENTER - FENCE_CORNER) * t.powf(1.5)
}

/// Whether a ball at this spot has left the park, given how high it still is.
pub fn clears_fence(point: Vec2, height: f32) -> bool {
    is_fair(point) && point.length() >= fence_distance(spray_angle(point)) && height >= FENCE_HEIGHT
}

pub fn base_position(base: Base) -> Vec2 {
    match base {
        Base::First => FIRST,
        Base::Second => SECOND,
        Base::Third => THIRD,
        Base::Home => HOME,
    }
}

/// Where each defender starts every pitch. Ordered so the pitcher and catcher
/// come first, which is the order the at-bat scene wants them in.
pub const FIELDER_HOMES: [(PlayerPosition, Vec2); 9] = [
    (PlayerPosition::Pitcher, MOUND),
    (PlayerPosition::Catcher, CATCHER),
    (PlayerPosition::FirstBase, Vec2::new(52.0, 78.0)),
    (PlayerPosition::SecondBase, Vec2::new(34.0, 132.0)),
    (PlayerPosition::Shortstop, Vec2::new(-34.0, 132.0)),
    (PlayerPosition::ThirdBase, Vec2::new(-52.0, 78.0)),
    (PlayerPosition::LeftField, Vec2::new(-152.0, 232.0)),
    (PlayerPosition::CenterField, Vec2::new(0.0, 288.0)),
    (PlayerPosition::RightField, Vec2::new(152.0, 232.0)),
];

/// The region the field camera must always keep on screen. Wide enough for both
/// foul poles and deep enough for the center-field wall, with room to spare.
pub const VIEW_WIDTH: f32 = 660.0;
pub const VIEW_HEIGHT: f32 = 500.0;

/// Vertical center of that region: the midpoint between the plate and the wall,
/// nudged back so the infield — where most of the action is — sits low and
/// central rather than crammed against the bottom edge.
pub const VIEW_CENTER_Y: f32 = 185.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bases_are_ninety_feet_apart() {
        // If the diagonal maths is wrong every position derived from it is too.
        assert!((HOME.distance(FIRST) - BASE_PATH).abs() < 0.01);
        assert!((FIRST.distance(SECOND) - BASE_PATH).abs() < 0.01);
        assert!((SECOND.distance(THIRD) - BASE_PATH).abs() < 0.01);
        assert!((THIRD.distance(HOME) - BASE_PATH).abs() < 0.01);
    }

    #[test]
    fn the_foul_lines_run_through_first_and_third_base() {
        // Both bags sit exactly on the boundary, so they are fair by a hair and
        // a shade outside is not.
        assert!(is_fair(FIRST));
        assert!(is_fair(THIRD));
        assert!(!is_fair(Vec2::new(FIRST.x + 1.0, FIRST.y)));
        assert!(!is_fair(Vec2::new(THIRD.x - 1.0, THIRD.y)));
    }

    #[test]
    fn balls_hit_behind_the_plate_are_never_fair() {
        assert!(!is_fair(Vec2::new(0.0, -10.0)));
        assert!(!is_fair(Vec2::new(0.0, 0.0)));
    }

    #[test]
    fn spray_angle_and_point_at_are_inverses() {
        for spray in [-0.7_f32, -0.3, 0.0, 0.25, 0.75] {
            let point = point_at(spray, 250.0);
            assert!(
                (spray_angle(point) - spray).abs() < 1e-4,
                "round trip failed at {spray}"
            );
            assert!((point.length() - 250.0).abs() < 1e-3);
        }
    }

    #[test]
    fn the_fence_is_deepest_to_center_and_shortest_down_the_lines() {
        let center = fence_distance(0.0);
        let alley = fence_distance(FOUL_LINE_ANGLE / 2.0);
        let corner = fence_distance(FOUL_LINE_ANGLE);

        assert!((center - FENCE_CENTER).abs() < 0.01);
        assert!((corner - FENCE_CORNER).abs() < 0.01);
        assert!(
            alley < center && alley > corner,
            "power alley {alley} should sit between {corner} and {center}"
        );
        // Symmetric about center field.
        assert!((fence_distance(0.4) - fence_distance(-0.4)).abs() < 1e-5);
    }

    #[test]
    fn a_ball_only_leaves_the_park_if_it_is_fair_deep_and_high() {
        let deep_center = point_at(0.0, FENCE_CENTER + 5.0);
        assert!(clears_fence(deep_center, 20.0), "deep, fair and high should be gone");
        assert!(!clears_fence(deep_center, 2.0), "a ball on the ground hits the wall");

        let shallow = point_at(0.0, 300.0);
        assert!(!clears_fence(shallow, 40.0), "300 feet to center is not a home run");

        let foul = point_at(FOUL_LINE_ANGLE + 0.1, FENCE_CENTER);
        assert!(!clears_fence(foul, 40.0), "foul territory is never a home run");
    }

    #[test]
    fn every_fielder_starts_in_fair_territory_except_the_catcher() {
        for (position, spot) in FIELDER_HOMES {
            if matches!(position, PlayerPosition::Catcher) {
                assert!(spot.y < 0.0, "the catcher sets up behind the plate");
                continue;
            }
            assert!(is_fair(spot), "{position} starts outside fair territory at {spot:?}");
        }
    }

    #[test]
    fn no_fielder_starts_beyond_the_wall() {
        for (position, spot) in FIELDER_HOMES {
            if matches!(position, PlayerPosition::Catcher) {
                continue;
            }
            let fence = fence_distance(spray_angle(spot));
            assert!(
                spot.length() < fence,
                "{position} starts {} feet out, past the {fence} foot wall",
                spot.length()
            );
        }
    }

    #[test]
    fn the_camera_view_covers_the_whole_park() {
        // Both foul poles and the deepest point of the wall have to be inside the
        // region the camera guarantees, or the layout is clipped.
        let half_width = VIEW_WIDTH / 2.0;
        let top = VIEW_CENTER_Y + VIEW_HEIGHT / 2.0;
        let bottom = VIEW_CENTER_Y - VIEW_HEIGHT / 2.0;

        for spray in [-FOUL_LINE_ANGLE, 0.0, FOUL_LINE_ANGLE] {
            let wall = point_at(spray, fence_distance(spray));
            assert!(
                wall.x.abs() <= half_width,
                "wall at {wall:?} is off the side of the view"
            );
            assert!(wall.y <= top, "wall at {wall:?} is above the view");
        }
        assert!(bottom < CATCHER.y, "the catcher should be inside the view");
    }
}

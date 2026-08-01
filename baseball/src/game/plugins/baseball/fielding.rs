//! Turning a batted ball into a result the rules engine understands.
//!
//! Nothing here looks at how well the ball was struck. It looks at where the ball
//! went, who could get to it, and whether the throw beat the runner. Everything
//! falls out of those three races, which is why the same swing can be an out or a
//! double depending on where the defence happens to be standing.

use baseball_game_rules::{PitchOutcome, PlayResult, PlayerPosition};
use bevy::prelude::*;

use super::{
    ball::{Flight, Intercept},
    field,
};

/// The bits of game state that change what a batted ball is worth.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Situation {
    pub runner_on_first: bool,
    pub runner_on_third: bool,
    /// Outs already recorded this half inning.
    pub outs: u8,
}

/// A batted ball, resolved.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Resolution {
    pub outcome: PitchOutcome,
    /// Who made the play, for the commentary line. `None` for a home run.
    pub fielder: Option<PlayerPosition>,
    /// Where and when the defence got to the ball, so the fielders can be
    /// animated running to exactly the spot the outcome was computed from.
    pub intercept: Option<Intercept>,
}

/// A caught fly this deep lets a runner tag up and score.
const SACRIFICE_DEPTH: f32 = 220.0;

/// A line drive never climbs above this.
const LINE_DRIVE_APEX: f32 = 26.0;

/// Above this apex an infield fly is a popup rather than a fly ball.
const POPUP_APEX: f32 = 70.0;

/// Balls fielded inside this radius count as infield plays.
const INFIELD_RADIUS: f32 = 165.0;

/// How much faster than the runner the throw has to be to turn two.
const DOUBLE_PLAY_MARGIN: f32 = 0.6;

fn is_outfielder(position: PlayerPosition) -> bool {
    matches!(
        position,
        PlayerPosition::LeftField | PlayerPosition::CenterField | PlayerPosition::RightField
    )
}

/// The fielder who gets to the ball first. Catches in the air win over pickups,
/// because a fielder who can catch it will.
fn best_fielder(flight: &Flight) -> (PlayerPosition, Intercept) {
    let mut best: Option<(PlayerPosition, Intercept)> = None;

    for (position, home) in field::FIELDER_HOMES {
        let intercept = flight.intercept(home, field::FIELDER_SPEED);
        let better = match best {
            None => true,
            Some((_, current)) => match (intercept.in_air(), current.in_air()) {
                (true, false) => true,
                (false, true) => false,
                _ => intercept.time < current.time,
            },
        };
        if better {
            best = Some((position, intercept));
        }
    }

    best.expect("there are always nine fielders")
}

/// How many bases the batter takes on a ball the defence failed to catch.
///
/// The runner is racing the ball to each bag in turn: he keeps going for as long
/// as the throw to the next base would arrive after he does.
fn bases_taken(point: Vec2, ready_at: f32) -> u8 {
    let bags = [
        (field::FIRST, field::RUN_TO_FIRST),
        (field::SECOND, field::RUN_TO_FIRST + field::RUN_PER_BASE),
        (field::THIRD, field::RUN_TO_FIRST + field::RUN_PER_BASE * 2.0),
    ];

    let mut bases = 0;
    for (bag, runner_arrives) in bags {
        let throw_arrives = ready_at + point.distance(bag) / field::THROW_SPEED;
        if throw_arrives > runner_arrives {
            bases += 1;
        } else {
            break;
        }
    }
    bases
}

/// Classifies a caught ball by the shape of its flight.
fn catch_kind(flight: &Flight, by: PlayerPosition, situation: Situation) -> PlayResult {
    let deep_enough = flight.landing.length() >= SACRIFICE_DEPTH;
    if situation.runner_on_third && situation.outs < 2 && deep_enough {
        return PlayResult::SacrificeFly;
    }
    if flight.apex < LINE_DRIVE_APEX {
        return PlayResult::Lineout;
    }
    if flight.apex > POPUP_APEX && !is_outfielder(by) {
        return PlayResult::Popout;
    }
    PlayResult::Flyout
}

/// Works out what a batted ball was worth.
pub fn resolve(flight: &Flight, situation: Situation) -> Resolution {
    if flight.home_run {
        return Resolution {
            outcome: PitchOutcome::HomeRun,
            fielder: None,
            intercept: None,
        };
    }

    let (position, intercept) = best_fielder(flight);

    // A fly ball caught on the fly is an out wherever it was caught — a foul pop
    // to the catcher retires the batter just the same as one in fair territory.
    if intercept.in_air() {
        return Resolution {
            outcome: PitchOutcome::InPlay(catch_kind(flight, position, situation)),
            fielder: Some(position),
            intercept: Some(intercept),
        };
    }

    // Nobody caught it. A ball that comes down outside the lines is simply foul.
    if !field::is_fair(flight.landing) {
        return Resolution {
            outcome: PitchOutcome::Foul,
            fielder: Some(position),
            intercept: Some(intercept),
        };
    }

    let ready_at = intercept.time + field::FIELDING_DELAY;
    let throw_to_first = ready_at + intercept.point.distance(field::FIRST) / field::THROW_SPEED;
    let in_infield = intercept.point.length() < INFIELD_RADIUS;

    // The throw beat the batter. An outfielder never will, so this branch is
    // effectively the infield without needing to say so.
    if throw_to_first <= field::RUN_TO_FIRST {
        let turns_two = situation.runner_on_first
            && situation.outs < 2
            && in_infield
            && throw_to_first <= field::RUN_TO_FIRST - DOUBLE_PLAY_MARGIN;

        let result = if turns_two {
            PlayResult::DoublePlay
        } else {
            PlayResult::Groundout
        };
        return Resolution {
            outcome: PitchOutcome::InPlay(result),
            fielder: Some(position),
            intercept: Some(intercept),
        };
    }

    // The batter beat the throw to first. If there was a runner forced at second,
    // the defence may still have taken the sure out there instead.
    if situation.runner_on_first && situation.outs < 2 && in_infield {
        let throw_to_second = ready_at + intercept.point.distance(field::SECOND) / field::THROW_SPEED;
        if throw_to_second <= field::RUN_PER_BASE {
            return Resolution {
                outcome: PitchOutcome::InPlay(PlayResult::FieldersChoice),
                fielder: Some(position),
                intercept: Some(intercept),
            };
        }
    }

    let result = match bases_taken(intercept.point, ready_at) {
        0 | 1 => PlayResult::Single,
        2 => PlayResult::Double,
        _ => PlayResult::Triple,
    };

    Resolution {
        outcome: PitchOutcome::InPlay(result),
        fielder: Some(position),
        intercept: Some(intercept),
    }
}

#[cfg(test)]
mod tests {
    use super::{super::ball, *};

    fn deg(d: f32) -> f32 {
        d.to_radians()
    }

    fn hit(mph: f32, launch: f32, spray: f32) -> Flight {
        ball::simulate(mph * ball::MPH_TO_FPS, deg(launch), deg(spray))
    }

    fn play(mph: f32, launch: f32, spray: f32) -> Resolution {
        resolve(&hit(mph, launch, spray), Situation::default())
    }

    fn result_of(resolution: Resolution) -> PlayResult {
        match resolution.outcome {
            PitchOutcome::InPlay(result) => result,
            other => panic!("expected a ball in play, got {other:?}"),
        }
    }

    #[test]
    fn a_ball_over_the_wall_is_a_home_run_and_nobody_fields_it() {
        let resolution = play(108.0, 30.0, 0.0);
        assert_eq!(resolution.outcome, PitchOutcome::HomeRun);
        assert!(resolution.fielder.is_none());
    }

    #[test]
    fn a_routine_fly_ball_to_the_outfield_is_caught() {
        let resolution = play(88.0, 35.0, 0.0);
        assert!(
            matches!(
                result_of(resolution),
                PlayResult::Flyout | PlayResult::Lineout | PlayResult::Popout
            ),
            "expected an out, got {:?}",
            result_of(resolution)
        );
        assert!(resolution.intercept.expect("someone caught it").in_air());
    }

    #[test]
    fn a_towering_infield_pop_up_is_a_popout() {
        let resolution = play(66.0, 74.0, 0.0);
        assert_eq!(result_of(resolution), PlayResult::Popout);
    }

    #[test]
    fn a_soft_grounder_to_the_infield_is_an_out_at_first() {
        let resolution = play(72.0, 2.0, deg(6.0).to_degrees());
        assert_eq!(result_of(resolution), PlayResult::Groundout);
        let intercept = resolution.intercept.expect("fielded on the ground");
        assert!(!intercept.in_air());
    }

    #[test]
    fn a_ball_that_lands_outside_the_lines_is_foul() {
        let resolution = play(95.0, 20.0, 60.0);
        assert_eq!(resolution.outcome, PitchOutcome::Foul);
    }

    #[test]
    fn a_foul_pop_up_that_is_caught_is_still_an_out() {
        // Caught in the air retires the batter wherever it happens.
        let flight = hit(58.0, 76.0, 55.0);
        assert!(!field::is_fair(flight.landing), "this one comes down foul");
        let resolution = resolve(&flight, Situation::default());
        if resolution.intercept.is_some_and(Intercept::in_air) {
            assert!(
                matches!(resolution.outcome, PitchOutcome::InPlay(_)),
                "a caught foul fly is an out, not a foul ball"
            );
        }
    }

    #[test]
    fn a_ball_the_defence_cannot_reach_becomes_a_hit() {
        // Hard and low into the left-centre gap: nobody catches it, so the batter
        // has to be credited with something.
        let resolution = play(104.0, 14.0, -20.0);
        assert!(
            matches!(
                result_of(resolution),
                PlayResult::Single | PlayResult::Double | PlayResult::Triple
            ),
            "expected a hit, got {:?}",
            result_of(resolution)
        );
    }

    #[test]
    fn a_ball_off_the_wall_is_worth_more_than_a_soft_single() {
        // Two uncaught balls, one much deeper. The deeper one has to pay better,
        // because the throw comes from further away.
        let deep = play(102.0, 28.0, 0.0);
        let shallow = play(74.0, 16.0, 0.0);

        let deep_bases = match result_of(deep) {
            PlayResult::Single => 1,
            PlayResult::Double => 2,
            PlayResult::Triple => 3,
            other => panic!("expected a hit off the wall, got {other:?}"),
        };
        // The shallow one may well be caught or fielded for an out; all that
        // matters is that it is never worth more than the ball off the wall.
        let shallow_bases = match result_of(shallow) {
            PlayResult::Single => 1,
            PlayResult::Double => 2,
            PlayResult::Triple => 3,
            _ => 0,
        };
        assert!(
            deep_bases >= 2,
            "a ball to the wall should be at least a double, got {deep_bases}"
        );
        assert!(deep_bases > shallow_bases);
    }

    #[test]
    fn a_runner_on_first_turns_a_sharp_grounder_into_two_outs() {
        let flight = hit(70.0, 1.0, 8.0);
        let empty = resolve(&flight, Situation::default());
        let forced = resolve(
            &flight,
            Situation {
                runner_on_first: true,
                outs: 0,
                ..Situation::default()
            },
        );

        assert_eq!(result_of(empty), PlayResult::Groundout, "nobody to force");
        assert_eq!(
            result_of(forced),
            PlayResult::DoublePlay,
            "with a runner on first the same ball is two"
        );
    }

    #[test]
    fn there_is_no_double_play_with_two_already_out() {
        let flight = hit(70.0, 1.0, 8.0);
        let resolution = resolve(
            &flight,
            Situation {
                runner_on_first: true,
                outs: 2,
                ..Situation::default()
            },
        );
        assert_eq!(
            result_of(resolution),
            PlayResult::Groundout,
            "the third out ends it; there is no second out to get"
        );
    }

    #[test]
    fn a_deep_fly_with_a_runner_on_third_is_a_sacrifice() {
        let flight = hit(90.0, 34.0, 0.0);
        let sac = resolve(
            &flight,
            Situation {
                runner_on_third: true,
                outs: 1,
                ..Situation::default()
            },
        );
        assert_eq!(result_of(sac), PlayResult::SacrificeFly);

        // With two out there is nothing to sacrifice: the inning is over.
        let two_out = resolve(
            &flight,
            Situation {
                runner_on_third: true,
                outs: 2,
                ..Situation::default()
            },
        );
        assert_ne!(result_of(two_out), PlayResult::SacrificeFly);
    }

    #[test]
    fn a_shallow_fly_is_not_deep_enough_to_sacrifice() {
        let flight = hit(68.0, 40.0, 0.0);
        assert!(flight.landing.length() < SACRIFICE_DEPTH);
        let resolution = resolve(
            &flight,
            Situation {
                runner_on_third: true,
                outs: 0,
                ..Situation::default()
            },
        );
        assert_ne!(result_of(resolution), PlayResult::SacrificeFly);
    }

    #[test]
    fn the_fielder_credited_is_always_the_one_who_reached_the_ball() {
        // The outcome and the animation have to agree, or fielders run to the
        // wrong place while the scoreboard says something else.
        for (mph, launch, spray) in [(88.0, 35.0, 0.0), (95.0, 5.0, -20.0), (80.0, 25.0, 30.0)] {
            let flight = hit(mph, launch, spray);
            let resolution = resolve(&flight, Situation::default());
            let (position, intercept) = (
                resolution.fielder.expect("somebody fielded it"),
                resolution.intercept.expect("with an intercept"),
            );
            let home = field::FIELDER_HOMES
                .iter()
                .find(|&&(p, _)| p == position)
                .map(|&(_, spot)| spot)
                .expect("credited fielder must be one of the nine");
            let run = home.distance(intercept.point) / field::FIELDER_SPEED;
            assert!(
                run <= intercept.time + 1e-3,
                "{position} could not have reached {:?} by {}s",
                intercept.point,
                intercept.time
            );
        }
    }

    /// Sweeps the whole space of pitches and swings a batter can actually produce
    /// and checks the resulting mix of outcomes looks like baseball.
    ///
    /// This is the guard on a long chain of tuning — drag, exit velocity, fielder
    /// speed, reaction time, defensive alignment. The first version of this model
    /// put batting average on balls in play at .082, because the fielders reacted
    /// instantly and ran at sprinter's pace, so almost everything hit in the air
    /// was caught. Any of those constants drifting will show up here.
    #[test]
    fn the_mix_of_outcomes_resembles_real_baseball() {
        use super::super::{
            bat,
            pitch::{LivePitch, PitchKind, PitchPlan},
        };

        let mut fair = 0;
        let mut hits = 0;
        let mut extra_base = 0;
        let mut caught_in_air = 0;

        for style in [bat::SwingStyle::Level, bat::SwingStyle::Normal, bat::SwingStyle::Lift] {
            for xi in -3..=3 {
                for hi in 0..=6 {
                    for ti in -6..=6 {
                        let target = Vec2::new(xi as f32 * 0.28, 1.7 + hi as f32 * 0.28);
                        let pitch = LivePitch::thrown(PitchPlan {
                            kind: PitchKind::Fastball,
                            target,
                        });
                        let bat::Contact::Struck {
                            exit_velocity,
                            launch,
                            spray,
                            ..
                        } = bat::resolve(
                            &pitch,
                            bat::Swing {
                                timing: ti as f32 * 0.024,
                                style,
                            },
                        )
                        else {
                            continue;
                        };

                        let flight = ball::simulate(exit_velocity, launch, spray);
                        let resolution = resolve(&flight, Situation::default());
                        if resolution.outcome == PitchOutcome::Foul {
                            continue;
                        }
                        fair += 1;

                        if resolution.intercept.is_some_and(Intercept::in_air) {
                            caught_in_air += 1;
                        }
                        match resolution.outcome {
                            PitchOutcome::HomeRun | PitchOutcome::InPlay(PlayResult::Double | PlayResult::Triple) => {
                                hits += 1;
                                extra_base += 1;
                            }
                            PitchOutcome::InPlay(PlayResult::Single) => hits += 1,
                            _ => {}
                        }
                    }
                }
            }
        }

        assert!(fair > 500, "the sweep should put plenty of balls in play, got {fair}");

        let babip = hits as f32 / fair as f32;
        assert!(
            (0.24..=0.34).contains(&babip),
            "batting average on balls in play is {babip:.3}, which is not baseball"
        );

        let extra_base_share = extra_base as f32 / hits as f32;
        assert!(
            (0.20..=0.50).contains(&extra_base_share),
            "extra-base hits are {:.0}% of all hits, which is not baseball",
            extra_base_share * 100.0
        );

        let air_share = caught_in_air as f32 / fair as f32;
        assert!(
            (0.20..=0.45).contains(&air_share),
            "{:.0}% of balls in play are caught in the air, which is not baseball",
            air_share * 100.0
        );
    }

    #[test]
    fn every_batted_ball_resolves_to_something() {
        // No combination of contact should leave the game with nothing to apply,
        // which would strand the play state machine forever.
        for launch in [-10.0, 0.0, 12.0, 25.0, 40.0, 55.0] {
            for spray in [-60.0, -30.0, 0.0, 30.0, 60.0] {
                for mph in [55.0, 75.0, 95.0, 108.0] {
                    let resolution = resolve(&hit(mph, launch, spray), Situation::default());
                    match resolution.outcome {
                        PitchOutcome::HomeRun | PitchOutcome::Foul | PitchOutcome::InPlay(_) => {}
                        other => panic!("{mph}/{launch}/{spray} produced {other:?}"),
                    }
                }
            }
        }
    }
}

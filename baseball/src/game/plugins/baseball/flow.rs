//! The pitch loop: choosing a pitch, throwing it, swinging at it, and settling
//! what happened.
//!
//! One trip through [`Phase`] is one pitch. The rules engine is only ever touched
//! in [`apply_outcome`], which is the single place a [`PitchOutcome`] is handed
//! over — everything before that is presentation and physics.

use baseball_game_rules::{PitchOutcome, PlayResult};
use bevy::prelude::*;

use super::{
    Banner, BatterIntent, Diamond, Fielder, Phase, PhaseTimer, RandomSource,
    ball::{self, LiveBall},
    bat, effects, fielding, pitch,
};

/// How long the AI takes to choose a pitch and wind up.
const AI_WINDUP: f32 = 0.85;

/// How long a result stays on screen before the next batter.
const RESOLVE_DWELL: f32 = 1.5;
const BIG_RESULT_DWELL: f32 = 2.6;

/// How long the between-innings card is shown.
pub const INNING_DWELL: f32 = 2.2;

/// Bookkeeping for the pitch currently being played out.
#[derive(Debug, Default, Resource)]
pub struct PitchLoop {
    /// What to hand the rules engine once the play finishes.
    pub pending: Option<PitchOutcome>,
    /// Where to go after the result has been shown.
    pub after_resolve: Option<Phase>,
    /// When a ball in play is finished with, in seconds since contact.
    pub done_at: f32,
    /// The AI batter's decision, made at release and acted on at the plate.
    pub ai_swing: Option<bat::Swing>,
    /// Seconds the AI still needs before it throws.
    pub ai_windup: f32,
}

// ------------------------------------------------------------------ AI

/// The AI's pitch selection. Mixes speeds and works off the plate often enough
/// that the human has to be selective, which is what makes a walk reachable.
fn ai_pitch_plan(rng: &mut RandomSource) -> pitch::PitchPlan {
    let kind = rng.pick(&pitch::PITCH_KINDS);
    let target = if rng.chance(0.60) {
        Vec2::new(
            rng.range(-pitch::ZONE_HALF_WIDTH * 0.92, pitch::ZONE_HALF_WIDTH * 0.92),
            rng.range(pitch::ZONE_BOTTOM + 0.15, pitch::ZONE_TOP - 0.15),
        )
    } else {
        // Just off the plate: tempting, but a ball if the batter lays off.
        Vec2::new(rng.range(-1.55, 1.55), rng.range(1.05, 4.05))
    };
    pitch::PitchPlan { kind, target }
}

/// Whether the AI batter offers at this pitch, and how well it times it.
fn ai_batter_decision(rng: &mut RandomSource, live: &pitch::LivePitch) -> Option<bat::Swing> {
    let outside = pitch::distance_outside(live.crossing());

    // Happy to swing at strikes, increasingly unwilling the further off the plate
    // the pitch finishes.
    let willingness = if outside <= 0.0 {
        0.84
    } else {
        (0.70 - outside * 0.58).max(0.05)
    };
    if !rng.chance(willingness) {
        return None;
    }

    // Harder pitches to read produce sloppier timing.
    let spread = match live.kind {
        pitch::PitchKind::Fastball => 0.072,
        pitch::PitchKind::Changeup => 0.092,
        pitch::PitchKind::Slider => 0.088,
        pitch::PitchKind::Curveball => 0.104,
    };

    let style = if rng.chance(0.28) {
        bat::SwingStyle::Lift
    } else if rng.chance(0.25) {
        bat::SwingStyle::Level
    } else {
        bat::SwingStyle::Normal
    };

    Some(bat::Swing {
        timing: rng.range(-spread, spread),
        style,
    })
}

// ------------------------------------------------------------------ windup

pub fn begin_windup(
    mut loop_state: ResMut<PitchLoop>,
    mut live_ball: ResMut<LiveBall>,
    mut plan: ResMut<pitch::PitchPlan>,
    mut rng: ResMut<RandomSource>,
    diamond: Res<Diamond>,
    mut fielders: Query<&mut Fielder>,
) {
    live_ball.clear();
    loop_state.pending = None;
    loop_state.after_resolve = None;
    loop_state.ai_swing = None;

    // Everyone back to their spot for the next pitch.
    for mut fielder in fielders.iter_mut() {
        fielder.target = None;
    }

    if diamond.human_is_batting() {
        // The AI is pitching: choose now, throw after a beat.
        *plan = ai_pitch_plan(&mut rng);
        loop_state.ai_windup = AI_WINDUP;
    } else {
        loop_state.ai_windup = 0.0;
    }
}

pub fn windup_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    diamond: Res<Diamond>,
    mut loop_state: ResMut<PitchLoop>,
    mut plan: ResMut<pitch::PitchPlan>,
    mut intent: ResMut<BatterIntent>,
    mut next: ResMut<NextState<Phase>>,
) {
    if diamond.human_is_batting() {
        // Batting: pick how to swing while waiting for the pitch.
        read_swing_style(&keys, &mut intent);

        loop_state.ai_windup -= time.delta_secs();
        if loop_state.ai_windup <= 0.0 {
            next.set(Phase::Pitch);
        }
        return;
    }

    // Pitching: choose a pitch and a spot, then throw it.
    let aim_step = 1.9 * time.delta_secs();
    let mut nudge = Vec2::ZERO;
    if keys.pressed(KeyCode::ArrowLeft) {
        nudge.x -= aim_step;
    }
    if keys.pressed(KeyCode::ArrowRight) {
        nudge.x += aim_step;
    }
    if keys.pressed(KeyCode::ArrowUp) {
        nudge.y += aim_step;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        nudge.y -= aim_step;
    }
    if nudge != Vec2::ZERO {
        plan.aim(nudge);
    }

    for (key, kind) in [
        (KeyCode::Digit1, pitch::PitchKind::Fastball),
        (KeyCode::Digit2, pitch::PitchKind::Slider),
        (KeyCode::Digit3, pitch::PitchKind::Curveball),
        (KeyCode::Digit4, pitch::PitchKind::Changeup),
    ] {
        if keys.just_pressed(key) {
            plan.kind = kind;
        }
    }
    if keys.just_pressed(KeyCode::KeyE) {
        plan.cycle_kind(true);
    }
    if keys.just_pressed(KeyCode::KeyQ) {
        plan.cycle_kind(false);
    }

    if keys.just_pressed(KeyCode::Space) {
        next.set(Phase::Pitch);
    }
}

fn read_swing_style(keys: &ButtonInput<KeyCode>, intent: &mut BatterIntent) {
    if keys.pressed(KeyCode::ArrowUp) {
        intent.style = bat::SwingStyle::Lift;
    } else if keys.pressed(KeyCode::ArrowDown) {
        intent.style = bat::SwingStyle::Level;
    } else {
        intent.style = bat::SwingStyle::Normal;
    }
}

// ------------------------------------------------------------------ the pitch

pub fn release_pitch(
    plan: Res<pitch::PitchPlan>,
    mut live: ResMut<pitch::LivePitch>,
    mut loop_state: ResMut<PitchLoop>,
    mut rng: ResMut<RandomSource>,
    diamond: Res<Diamond>,
) {
    *live = pitch::LivePitch::thrown(*plan);

    // The AI batter commits now and acts on it when the ball arrives, so its
    // decision cannot depend on anything it should not have seen.
    loop_state.ai_swing = if diamond.human_is_batting() {
        None
    } else {
        ai_batter_decision(&mut rng, &live)
    };
}

pub fn advance_pitch(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    diamond: Res<Diamond>,
    mut live: ResMut<pitch::LivePitch>,
    mut intent: ResMut<BatterIntent>,
    mut loop_state: ResMut<PitchLoop>,
    mut live_ball: ResMut<LiveBall>,
    mut splashes: MessageWriter<effects::Splash>,
    mut fielders: Query<&mut Fielder>,
    mut next: ResMut<NextState<Phase>>,
) {
    live.elapsed += time.delta_secs();

    let human_batting = diamond.human_is_batting();
    if human_batting {
        read_swing_style(&keys, &mut intent);
    }

    // A swing, from whichever side is batting.
    let swing = if human_batting {
        (!live.swung && keys.just_pressed(KeyCode::Space)).then(|| bat::Swing {
            timing: live.elapsed - live.flight,
            style: intent.style,
        })
    } else {
        (!live.swung && live.reached_plate())
            .then_some(loop_state.ai_swing)
            .flatten()
    };

    if let Some(swing) = swing {
        live.swung = true;
        match bat::resolve(&live, swing) {
            bat::Contact::Whiff => {
                loop_state.pending = Some(PitchOutcome::Strike);
                splashes.write(effects::Splash::Whiff);
                next.set(Phase::Resolve);
            }
            bat::Contact::Struck {
                exit_velocity,
                launch,
                spray,
                quality,
            } => {
                let flight = ball::simulate(exit_velocity, launch, spray);
                let resolution = fielding::resolve(&flight, diamond.situation());

                // Send the credited fielder to the exact spot the outcome was
                // computed from, and let two neighbours shade that way, so the
                // defence looks like it is reacting to the ball it is reacting to.
                if let Some(intercept) = resolution.intercept {
                    for mut fielder in fielders.iter_mut() {
                        fielder.target = if Some(fielder.position) == resolution.fielder {
                            Some(intercept.point)
                        } else {
                            let toward = fielder.home.lerp(intercept.point, 0.30);
                            (fielder.home.distance(intercept.point) < 150.0).then_some(toward)
                        };
                    }
                }

                loop_state.done_at = match resolution.intercept {
                    Some(intercept) => intercept.time + 0.55,
                    None => flight.time_to_land + 0.9,
                };
                loop_state.pending = Some(resolution.outcome);
                live_ball.hit(exit_velocity, launch, spray);
                splashes.write(effects::Splash::Contact { quality });
                next.set(Phase::BallInPlay);
            }
        }
        return;
    }

    // Nobody offered. The umpire calls it once the ball is past.
    if !live.swung && live.past_catcher() {
        let crossing = live.crossing();
        loop_state.pending = Some(if pitch::hits_batter(crossing) {
            PitchOutcome::HitByPitch
        } else if pitch::in_zone(crossing) {
            PitchOutcome::Strike
        } else {
            PitchOutcome::Ball
        });
        next.set(Phase::Resolve);
    }
}

// ------------------------------------------------------------------ ball in play

pub fn begin_ball_in_play(mut live_ball: ResMut<LiveBall>) {
    live_ball.elapsed = 0.0;
}

pub fn advance_ball_in_play(
    time: Res<Time>,
    mut live_ball: ResMut<LiveBall>,
    loop_state: Res<PitchLoop>,
    mut next: ResMut<NextState<Phase>>,
) {
    let dt = time.delta_secs();
    live_ball.elapsed += dt;

    // Integrated with the same function the prediction used, so what the player
    // watches is what the outcome was read from.
    let (pos, vel) = ball::step(live_ball.pos, live_ball.vel, dt);
    live_ball.pos = pos;
    live_ball.vel = vel;

    if live_ball.elapsed >= loop_state.done_at {
        next.set(Phase::Resolve);
    }
}

// ------------------------------------------------------------------ resolving

/// Hands the outcome to the rules engine. The one place the game state changes.
pub fn apply_outcome(
    mut diamond: ResMut<Diamond>,
    mut loop_state: ResMut<PitchLoop>,
    mut banner: ResMut<Banner>,
    mut timer: ResMut<PhaseTimer>,
    mut live_ball: ResMut<LiveBall>,
    mut splashes: MessageWriter<effects::Splash>,
) {
    live_ball.live = false;

    let Some(outcome) = loop_state.pending.take() else {
        // Nothing to apply: go straight on rather than stalling the loop.
        loop_state.after_resolve = Some(Phase::Windup);
        timer.set(0.1);
        return;
    };

    let before = diamond
        .game()
        .map(|game| (game.current_half_inning().half(), game.current_inning().as_number()));

    // `GameOutcome::advance` consumes the value it advances, so hand it a clone.
    diamond.outcome = diamond.outcome.clone().advance(outcome);

    let after = diamond
        .game()
        .map(|game| (game.current_half_inning().half(), game.current_inning().as_number()));

    // Headline for the middle of the screen.
    let (headline, good) = describe(outcome);
    banner.headline = headline;
    banner.good_for_batter = good;
    banner.detail = count_line(&diamond);

    if matches!(outcome, PitchOutcome::HomeRun) {
        splashes.write(effects::Splash::HomeRun);
    }

    let big = matches!(
        outcome,
        PitchOutcome::HomeRun | PitchOutcome::InPlay(PlayResult::Triple | PlayResult::DoublePlay)
    );
    timer.set(if big { BIG_RESULT_DWELL } else { RESOLVE_DWELL });

    loop_state.after_resolve = Some(if diamond.outcome.is_complete() {
        Phase::GameOver
    } else if before != after {
        Phase::InningBreak
    } else {
        Phase::Windup
    });
}

fn describe(outcome: PitchOutcome) -> (String, bool) {
    match outcome {
        PitchOutcome::HomeRun => ("HOME RUN!".to_string(), true),
        PitchOutcome::Ball => ("BALL".to_string(), true),
        PitchOutcome::Strike => ("STRIKE".to_string(), false),
        PitchOutcome::Foul => ("FOUL BALL".to_string(), false),
        PitchOutcome::HitByPitch => ("HIT BY PITCH".to_string(), true),
        PitchOutcome::InPlay(play) => (play.label().to_string(), play.is_hit()),
    }
}

/// The line under the headline: the count, or what the plate appearance became.
fn count_line(diamond: &Diamond) -> String {
    let Some(game) = diamond.game() else {
        return String::new();
    };
    let half = game.current_half_inning();
    let count = half.current_plate_appearance().count();
    format!(
        "{}-{}   {} OUT",
        count.balls().as_number(),
        count.strikes().as_number(),
        half.outs().as_number()
    )
}

pub fn advance_resolve(
    time: Res<Time>,
    mut timer: ResMut<PhaseTimer>,
    loop_state: Res<PitchLoop>,
    mut next: ResMut<NextState<Phase>>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        next.set(loop_state.after_resolve.unwrap_or(Phase::Windup));
    }
}

pub fn advance_inning_break(time: Res<Time>, mut timer: ResMut<PhaseTimer>, mut next: ResMut<NextState<Phase>>) {
    if timer.0.tick(time.delta()).just_finished() {
        next.set(Phase::Windup);
    }
}

// ------------------------------------------------------------------ pause

pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<super::Paused>,
    mut next: ResMut<NextState<Phase>>,
) {
    if keys.just_pressed(KeyCode::Escape) {
        paused.0 = !paused.0;
    }
    if paused.0 && keys.just_pressed(KeyCode::KeyT) {
        paused.0 = false;
        next.set(Phase::Title);
    }
}

/// Where a fielder should be standing right now, used by the tests below to check
/// the defence is being sent to the same place the outcome came from.
#[cfg(test)]
fn credited_target(resolution: &fielding::Resolution) -> Option<Vec2> {
    resolution.intercept.map(|intercept| intercept.point)
}

#[cfg(test)]
mod tests {
    use super::{super::field, *};

    fn rng() -> RandomSource {
        RandomSource::default()
    }

    #[test]
    fn the_ai_pitcher_throws_strikes_and_balls_in_a_sane_mix() {
        let mut rng = rng();
        let mut strikes = 0;
        let total = 400;
        for _ in 0..total {
            let plan = ai_pitch_plan(&mut rng);
            if pitch::in_zone(plan.target) {
                strikes += 1;
            }
        }
        let rate = f64::from(strikes) / f64::from(total);
        assert!(
            (0.45..=0.75).contains(&rate),
            "the AI threw {:.0}% strikes, which is not pitching",
            rate * 100.0
        );
    }

    #[test]
    fn the_ai_pitcher_uses_its_whole_repertoire() {
        let mut rng = rng();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            seen.insert(ai_pitch_plan(&mut rng).kind);
        }
        assert_eq!(seen.len(), pitch::PITCH_KINDS.len(), "some pitch is never thrown");
    }

    #[test]
    fn the_ai_pitcher_never_aims_somewhere_unreachable() {
        let mut rng = rng();
        for _ in 0..300 {
            let target = ai_pitch_plan(&mut rng).target;
            assert!(target.x.abs() <= pitch::AIM_LIMIT_X, "aimed at {target:?}");
            assert!(
                (pitch::AIM_LIMIT_LOW..=pitch::AIM_LIMIT_HIGH).contains(&target.y),
                "aimed at {target:?}"
            );
        }
    }

    #[test]
    fn the_ai_batter_swings_at_strikes_far_more_than_at_balls() {
        let mut rng = rng();
        let offer_rate = |target: Vec2, rng: &mut RandomSource| {
            let live = pitch::LivePitch::thrown(pitch::PitchPlan {
                kind: pitch::PitchKind::Fastball,
                target,
            });
            let swings = (0..400).filter(|_| ai_batter_decision(rng, &live).is_some()).count();
            swings as f32 / 400.0
        };

        let strike = offer_rate(Vec2::new(0.0, pitch::ZONE_MID), &mut rng);
        let close = offer_rate(Vec2::new(pitch::ZONE_HALF_WIDTH + 0.3, pitch::ZONE_MID), &mut rng);
        let miles_off = offer_rate(Vec2::new(0.0, pitch::AIM_LIMIT_HIGH), &mut rng);

        assert!(strike > 0.7, "should attack strikes, offered at {strike:.2}");
        assert!(close < strike, "should be more careful just off the plate");
        assert!(
            miles_off < 0.25,
            "should mostly lay off a pitch nowhere near the zone, offered at {miles_off:.2}"
        );
    }

    #[test]
    fn the_ai_batter_times_a_curveball_worse_than_a_fastball() {
        let mut rng = rng();
        let spread_of = |kind: pitch::PitchKind, rng: &mut RandomSource| {
            let live = pitch::LivePitch::thrown(pitch::PitchPlan {
                kind,
                target: Vec2::new(0.0, pitch::ZONE_MID),
            });
            let mut worst = 0.0_f32;
            for _ in 0..500 {
                if let Some(swing) = ai_batter_decision(rng, &live) {
                    worst = worst.max(swing.timing.abs());
                }
            }
            worst
        };

        let fastball = spread_of(pitch::PitchKind::Fastball, &mut rng);
        let curveball = spread_of(pitch::PitchKind::Curveball, &mut rng);
        assert!(
            curveball > fastball,
            "a curveball should be harder to time: {curveball} vs {fastball}"
        );
    }

    #[test]
    fn the_ai_batter_sometimes_swings_for_the_fences() {
        let mut rng = rng();
        let live = pitch::LivePitch::thrown(pitch::PitchPlan {
            kind: pitch::PitchKind::Fastball,
            target: Vec2::new(0.0, pitch::ZONE_MID),
        });
        let mut styles = std::collections::HashSet::new();
        for _ in 0..300 {
            if let Some(swing) = ai_batter_decision(&mut rng, &live) {
                styles.insert(swing.style);
            }
        }
        assert_eq!(styles.len(), 3, "the AI should vary its swing, saw {styles:?}");
    }

    #[test]
    fn every_outcome_has_something_to_print() {
        let outcomes = [
            PitchOutcome::Ball,
            PitchOutcome::Strike,
            PitchOutcome::Foul,
            PitchOutcome::HitByPitch,
            PitchOutcome::HomeRun,
            PitchOutcome::InPlay(PlayResult::Single),
            PitchOutcome::InPlay(PlayResult::DoublePlay),
        ];
        for outcome in outcomes {
            let (headline, _) = describe(outcome);
            assert!(!headline.is_empty(), "{outcome:?} has no headline");
        }
    }

    #[test]
    fn hits_read_as_good_for_the_batter_and_outs_do_not() {
        assert!(describe(PitchOutcome::HomeRun).1);
        assert!(describe(PitchOutcome::InPlay(PlayResult::Double)).1);
        assert!(describe(PitchOutcome::Ball).1);
        assert!(!describe(PitchOutcome::Strike).1);
        assert!(!describe(PitchOutcome::InPlay(PlayResult::Groundout)).1);
    }

    #[test]
    fn the_fielder_sent_to_the_ball_is_the_one_the_outcome_credits() {
        // Guards the link between the simulation and the animation: the defender
        // who runs to the ball has to be the defender the result was computed for.
        for (mph, launch, spray) in [(88.0_f32, 35.0_f32, 0.0_f32), (95.0, 4.0, -18.0), (78.0, 22.0, 33.0)] {
            let flight = ball::simulate(mph * ball::MPH_TO_FPS, f32::to_radians(launch), f32::to_radians(spray));
            let resolution = fielding::resolve(&flight, fielding::Situation::default());
            if resolution.outcome == PitchOutcome::HomeRun {
                continue;
            }
            let target = credited_target(&resolution).expect("a fielded ball has an intercept");
            let position = resolution.fielder.expect("and a fielder");
            let home = field::FIELDER_HOMES
                .iter()
                .find(|&&(p, _)| p == position)
                .map(|&(_, spot)| spot)
                .expect("credited fielder is one of the nine");
            assert!(
                home.distance(target) / field::FIELDER_SPEED <= resolution.intercept.unwrap().time + 1e-3,
                "{position} is sent somewhere they could not reach in time"
            );
        }
    }
}

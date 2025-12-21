use baseball_game_rules::*;
use bevy::prelude::*;

use crate::game::plugins::baseball::{components::*, constants::*, resources::*, state::BallState};

pub fn reset_pitch_timer(mut timer: ResMut<PitchTimer>) {
    *timer = PitchTimer::default();
}

pub fn update_pitch_timer(time: Res<Time>, mut timer: ResMut<PitchTimer>, mut state: ResMut<NextState<BallState>>) {
    if timer.timer.tick(time.delta()).just_finished() {
        state.set(BallState::Pitch);
    }
}

pub fn reset_ball_position(mut ball: Query<(&mut Transform, &mut BallVelocity), With<Ball>>) {
    if let Ok((mut tform, mut velocity)) = ball.single_mut() {
        *tform = BALL_START;
        velocity.set(Vec3::ZERO);
    }
}

pub fn start_pitch(mut ball: Query<&mut BallVelocity, With<Ball>>) {
    if let Ok(mut velocity) = ball.single_mut() {
        // Pitch travels in negative Y direction (toward home plate)
        velocity.set(Vec3::new(0.0, -PITCH_SPEED, 0.0));
    }
}

pub fn move_ball(time: Res<Time>, mut ball: Query<(&mut Transform, &BallVelocity), With<Ball>>) {
    if let Ok((mut tform, velocity)) = ball.single_mut() {
        tform.translation += velocity.v * time.delta_secs();
    }
}

pub fn swing(
    input: Res<ButtonInput<KeyCode>>,
    mut ball: Query<(&Transform, &mut BallVelocity), With<Ball>>,
    mut state: ResMut<NextState<BallState>>,
    mut swing_timing: ResMut<LastSwingTiming>,
) {
    if input.just_pressed(KeyCode::Space)
        && let Ok((ball_pos, mut velo)) = ball.single_mut()
    {
        let ball_y = ball_pos.translation.y;
        let distance_from_zone = ball_y - STRIKE_ZONE_Y;

        // Check if the ball is within the swing window
        if distance_from_zone.abs() > SWING_WINDOW {
            // Swung too early or ball already past - miss/strike
            // Ball continues, no state change yet (will be handled by pitch resolution)
            swing_timing.timing = None;
            return;
        }

        // Timing factor: -1.0 (early) to 1.0 (late)
        // Early = ball hasn't reached zone yet (positive distance) = pull to left field
        // Late = ball past zone (negative distance) = slice to right field
        let timing = (distance_from_zone / SWING_WINDOW).clamp(-1.0, 1.0);
        swing_timing.timing = Some(timing);

        // Calculate hit direction based on timing
        // Base direction is toward center field (0, 1) with X offset based on timing
        // Early (timing > 0) → pull to left field (negative X)
        // Late (timing < 0) → slice to right field (positive X)
        let x_direction = -timing * 0.8; // Negative because early = left
        let y_direction = 1.0; // Always toward outfield
        let hit_direction = Vec3::new(x_direction, y_direction, 0.0).normalize();

        // Speed based on how well-timed the swing was (closer to zone = harder hit)
        let timing_quality = 1.0 - timing.abs();
        let base_speed = 200.0;
        let hit_speed = base_speed * (0.5 + 0.5 * timing_quality);

        velo.set(hit_direction * hit_speed);
        state.set(BallState::InPlay);
    }
}

pub fn check_pitch_result(
    ball: Query<&Transform, With<Ball>>,
    mut game_data: ResMut<GameData>,
    mut state: ResMut<NextState<BallState>>,
    mut result_query: Query<(&mut Text, &mut Visibility), With<ResultText>>,
    mut result_timer: ResMut<ResultDisplayTimer>,
) {
    if let Ok(ball_pos) = ball.single() {
        // Ball passed the catcher - pitch is over
        if ball_pos.translation.y < CATCHER_Y {
            // Ball wasn't hit, count as a strike (simplified - no balls for now)
            let outcome = PitchOutcome::Strike;
            game_data.game_result = game_data.game_result.clone().advance(outcome);
            state.set(BallState::PrePitch);

            // Show result
            if let Ok((mut text, mut vis)) = result_query.single_mut() {
                **text = outcome.result_text().to_string();
                *vis = Visibility::Visible;
                *result_timer = ResultDisplayTimer::default();
            }
        }
    }
}

pub fn reset_play_timer(mut timer: ResMut<PlayResolveTimer>) {
    *timer = PlayResolveTimer::default();
}

pub fn update_play_timer(
    time: Res<Time>,
    mut timer: ResMut<PlayResolveTimer>,
    swing_timing: Res<LastSwingTiming>,
    mut game_data: ResMut<GameData>,
    mut state: ResMut<NextState<BallState>>,
    mut result_query: Query<(&mut Text, &mut Visibility), With<ResultText>>,
    mut result_timer: ResMut<ResultDisplayTimer>,
) {
    if timer.timer.tick(time.delta()).just_finished() {
        let outcome = determine_hit_outcome(&game_data, swing_timing.timing);

        // Show result text
        if let Ok((mut text, mut vis)) = result_query.single_mut() {
            **text = outcome.result_text().to_string();
            *vis = Visibility::Visible;
            *result_timer = ResultDisplayTimer::default();
        }

        game_data.game_result = game_data.game_result.clone().advance(outcome);
        state.set(BallState::PrePitch);
    }
}

/// Determine hit outcome based on timing quality
fn determine_hit_outcome(game_data: &GameData, timing: Option<f32>) -> PitchOutcome {
    let timing_quality = timing.map(|t| 1.0 - t.abs()).unwrap_or(0.0);

    // Get current baserunners and batter from game state
    let (baserunners, batter) = if let Some(game) = game_data.game_result.game_ref() {
        (
            game.current_half_inning().baserunners(),
            game.current_half_inning().current_batter(),
        )
    } else {
        (BaserunnerState::empty(), BattingPosition::First)
    };

    // Determine hit type based on timing quality
    if timing_quality > 0.9 {
        // Perfect timing - home run!
        PitchOutcome::HomeRun
    } else if timing_quality > 0.7 {
        // Great timing - triple
        PitchOutcome::InPlay(PlayOutcome::triple(baserunners, batter))
    } else if timing_quality > 0.5 {
        // Good timing - double
        PitchOutcome::InPlay(PlayOutcome::double(baserunners, batter))
    } else if timing_quality > 0.3 {
        // Decent timing - single
        PitchOutcome::InPlay(PlayOutcome::single(baserunners, batter))
    } else {
        // Poor timing - groundout
        PitchOutcome::InPlay(PlayOutcome::groundout())
    }
}

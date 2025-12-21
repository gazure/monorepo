mod components;
mod constants;
mod resources;
pub mod state;
mod systems;

use bevy::prelude::*;
pub use resources::*;
pub use systems::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseballPlugin;

impl Plugin for BaseballPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, setup_field, setup_ui, reset_pitch_timer))
            .add_systems(
                OnEnter(state::BallState::PrePitch),
                (reset_pitch_timer, reset_ball_position),
            )
            .add_systems(
                Update,
                (
                    update_pitch_timer.run_if(in_state(state::BallState::PrePitch)),
                    (move_ball, swing, check_pitch_result).run_if(in_state(state::BallState::Pitch)),
                    (move_ball, update_play_timer).run_if(in_state(state::BallState::InPlay)),
                    (
                        update_score_ui,
                        update_inning_ui,
                        update_count_ui,
                        update_baserunners_ui,
                        update_result_display,
                    ),
                ),
            )
            .add_systems(OnEnter(state::BallState::Pitch), start_pitch)
            .add_systems(OnEnter(state::BallState::InPlay), reset_play_timer)
            .init_resource::<GameData>()
            .init_state::<state::BallState>()
            .init_resource::<PitchTimer>()
            .init_resource::<LastSwingTiming>()
            .init_resource::<PlayResolveTimer>()
            .init_resource::<ResultDisplayTimer>();
    }
}

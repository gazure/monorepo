mod components;
mod resources;
mod systems;

use bevy::{
    ecs::schedule::IntoScheduleConfigs,
    prelude::{App, OnEnter, OnExit, Plugin, PostUpdate, Update, in_state},
};

use crate::GameState;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), systems::setup)
            .add_systems(
                Update,
                (
                    systems::handle_keyboard_input,
                    systems::handle_camera_controls,
                    systems::manage_chunks,
                    systems::handle_mouse_input,
                    systems::update_grid,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(PostUpdate, systems::render_cells.run_if(in_state(GameState::Playing)))
            .add_systems(OnExit(GameState::Playing), systems::cleanup_game);
    }
}

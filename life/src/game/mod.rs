mod components;
mod resources;
mod systems;

use bevy::prelude::{App, Plugin, PostUpdate, Startup, Update};

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::setup)
            .add_systems(
                Update,
                (
                    systems::handle_keyboard_input,
                    systems::handle_camera_controls,
                    systems::handle_mouse_input,
                    systems::update_grid,
                ),
            )
            .add_systems(PostUpdate, systems::render_cells);
    }
}

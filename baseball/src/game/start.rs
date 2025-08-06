use bevy::{prelude::*, window::WindowMode};
use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};

use crate::game::plugins::baseball::BaseballPlugin;

/// Runs the game.
pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin {
            enable_multipass_for_primary_context: true,
        })
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(BaseballPlugin)
        .run();
}

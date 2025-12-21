use bevy::{log::LogPlugin, prelude::*, window::WindowMode};

// TODO Wait for inspector to run with bevy 0.17
// use bevy_inspector_egui::{bevy_egui::EguiPlugin, quick::WorldInspectorPlugin};
use crate::game::plugins::baseball::BaseballPlugin;

pub fn run() {
    tracingx::init_dev();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        ..default()
                    }),
                    ..default()
                })
                .disable::<LogPlugin>(),
        )
        // .add_plugins(EguiPlugin::default())
        // .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(BaseballPlugin)
        .run();
}

#![expect(clippy::needless_pass_by_value)]
#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_possible_wrap)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::cast_precision_loss)]

mod game;

use bevy::{
    prelude::{App, DefaultPlugins, PluginGroup, ToString, Window, WindowPlugin, default},
    window::{MonitorSelection, WindowMode},
};
pub use game::GamePlugin;

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Conway's Game of Life".to_string(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}

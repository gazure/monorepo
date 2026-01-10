#![expect(clippy::needless_pass_by_value)]
#![expect(clippy::cast_possible_truncation)]
#![expect(clippy::cast_possible_wrap)]
#![expect(clippy::cast_sign_loss)]
#![expect(clippy::cast_precision_loss)]

mod game;
mod menu;

use bevy::{
    app::Startup,
    prelude::{
        App, AppExtStates, Camera2d, Commands, Component, DefaultPlugins, Plugin, PluginGroup, States, ToString,
        Window, WindowPlugin, default,
    },
    window::{MonitorSelection, WindowMode},
};
use game::GamePlugin;
use menu::MenuPlugin;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Menu,
    Playing,
}

#[derive(Component)]
pub struct GameCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((Camera2d, GameCamera));
}

/// Main plugin that orchestrates the Game of Life application.
/// Initializes the game state and registers child plugins.
pub struct LifePlugin;

impl Plugin for LifePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Startup, spawn_camera)
            .add_plugins(MenuPlugin)
            .add_plugins(GamePlugin);
    }
}

pub fn run() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Game of Life".to_string(),
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(LifePlugin)
        .run();
}

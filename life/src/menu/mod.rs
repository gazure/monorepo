mod systems;

use bevy::{
    ecs::schedule::IntoScheduleConfigs,
    prelude::{App, OnEnter, OnExit, Plugin, Startup, Update, in_state},
};

use crate::GameState;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            // Menu setup runs at Startup (OnEnter doesn't trigger for default state)
            // AND OnEnter so it rebuilds when returning from game
            .add_systems(Startup, systems::setup_menu)
            .add_systems(OnEnter(GameState::Menu), systems::setup_menu)
            .add_systems(Update, systems::menu_action.run_if(in_state(GameState::Menu)))
            .add_systems(OnExit(GameState::Menu), systems::cleanup_menu);
    }
}

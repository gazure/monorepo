use baseball_game_rules::{Game, GameOutcome};
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameData {
    pub game_result: GameOutcome,
}

impl Default for GameData {
    fn default() -> Self {
        Self {
            game_result: GameOutcome::InProgress(Game::new()),
        }
    }
}

#[derive(Resource, Debug)]
pub struct PitchTimer {
    pub timer: Timer,
}

impl Default for PitchTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Once),
        }
    }
}

/// Stores the timing quality of the last swing for determining hit type
#[derive(Resource, Debug, Default)]
pub struct LastSwingTiming {
    /// Timing value from -1.0 (late) to 1.0 (early), None if no swing
    pub timing: Option<f32>,
}

/// Timer for delaying play resolution after a hit
#[derive(Resource, Debug)]
pub struct PlayResolveTimer {
    pub timer: Timer,
}

impl Default for PlayResolveTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(3.0, TimerMode::Once),
        }
    }
}

/// Timer for how long to display the result text
#[derive(Resource, Debug)]
pub struct ResultDisplayTimer {
    pub timer: Timer,
}

impl Default for ResultDisplayTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(2.0, TimerMode::Once),
        }
    }
}

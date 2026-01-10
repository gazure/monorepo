use std::collections::HashMap;

use bevy::prelude::{Entity, Resource, Timer, TimerMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPattern {
    #[default]
    ActivationCount,
    Binary,
    NeighborCount,
    PastelRainbow,
    Neon,
    Monochrome,
    Ocean,
    Fire,
    GenerationBased,
}

#[derive(Resource)]
pub struct SimulationState {
    pub paused: bool,
    pub step_mode: bool,
    pub generation: u64,
    pub update_timer: Timer,
    pub color_pattern: ColorPattern,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            paused: true,
            step_mode: false,
            generation: 0,
            update_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            color_pattern: ColorPattern::default(),
        }
    }
}

#[derive(Resource, Default)]
pub struct ChunkManager {
    pub active_chunks: HashMap<(i32, i32), Entity>,
    pub current_center_chunk: Option<(i32, i32)>,
}

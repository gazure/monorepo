use bevy::prelude::*;

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BallState {
    #[default]
    PrePitch,
    Pitch,
    InPlay,
}

use bevy::prelude::Component;

#[derive(Component)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
}

use bevy::prelude::*;

pub const FIELD_BROWN: Color = Color::srgb(0.6, 0.4, 0.2);
pub const MOUND_BROWN: Color = Color::srgb(0.7, 0.5, 0.3);
pub const BALL_START: Transform = Transform::from_xyz(0.0, -60.0, 10.0);

/// Strike zone Y position (where home plate is, ball travels from pitcher toward this)
pub const STRIKE_ZONE_Y: f32 = -390.0;
/// How far from the strike zone the ball can be and still be hittable
pub const SWING_WINDOW: f32 = 50.0;
/// Pitch speed (units per second toward home plate)
pub const PITCH_SPEED: f32 = 300.0;
/// Y position past which the ball is considered past the batter (catcher position)
pub const CATCHER_Y: f32 = -450.0;

/// Base positions on the field (first, second, third)
pub const BASE_POSITIONS: [(f32, f32); 3] = [
    (185.0, -185.0), // First base
    (185.0, 185.0),  // Second base
    (-185.0, 185.0), // Third base
];

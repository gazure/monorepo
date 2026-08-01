//! Palette and type sizes.
//!
//! Colours are authored in linear space and a few of the highlights deliberately
//! sit above `1.0` so the bloom pass has something to catch.

use bevy::prelude::*;

/// Grass, cut in two tones so the outfield reads as mown stripes.
pub const GRASS: Color = Color::srgb(0.16, 0.42, 0.18);
pub const GRASS_LIGHT: Color = Color::srgb(0.19, 0.48, 0.21);
pub const WARNING_TRACK: Color = Color::srgb(0.45, 0.29, 0.16);
pub const INFIELD_DIRT: Color = Color::srgb(0.52, 0.33, 0.19);
pub const MOUND_DIRT: Color = Color::srgb(0.58, 0.38, 0.22);
pub const CHALK: Color = Color::srgb(0.94, 0.94, 0.90);
pub const WALL: Color = Color::srgb(0.10, 0.20, 0.12);
pub const WALL_CAP: Color = Color::srgb(0.85, 0.83, 0.72);

/// Night sky behind the at-bat view, and the stands above it.
pub const SKY: Color = Color::srgb(0.04, 0.05, 0.10);
pub const STANDS: Color = Color::srgb(0.09, 0.10, 0.16);
pub const CROWD_DARK: Color = Color::srgb(0.13, 0.14, 0.20);

/// Uniforms. Home whites and away greys, with a saturated cap so the two teams
/// stay legible as tiny dots on the field view.
pub const HOME_UNIFORM: Color = Color::srgb(0.90, 0.90, 0.94);
pub const HOME_TRIM: Color = Color::srgb(0.16, 0.34, 0.72);
pub const AWAY_UNIFORM: Color = Color::srgb(0.42, 0.44, 0.50);
pub const AWAY_TRIM: Color = Color::srgb(0.72, 0.20, 0.22);

pub const BALL: Color = Color::srgb(1.6, 1.6, 1.5);
pub const BALL_SHADOW: Color = Color::srgba(0.0, 0.0, 0.0, 0.35);

/// The strike zone box and the pitch target reticle.
pub const ZONE: Color = Color::srgba(0.85, 0.90, 1.0, 0.22);
pub const ZONE_EDGE: Color = Color::srgb(0.75, 0.82, 1.0);
pub const TARGET: Color = Color::srgb(1.4, 0.9, 0.2);

/// Score bug furniture.
pub const BUG_PANEL: Color = Color::srgb(0.06, 0.07, 0.11);
pub const BUG_EDGE: Color = Color::srgb(0.18, 0.20, 0.28);
pub const BUG_ACCENT: Color = Color::srgb(0.95, 0.65, 0.15);
pub const TEXT: Color = Color::srgb(0.95, 0.96, 0.98);
pub const TEXT_DIM: Color = Color::srgb(0.55, 0.58, 0.66);

/// Count lamps: balls green, strikes amber, outs red, unlit slots near-black.
pub const LAMP_OFF: Color = Color::srgb(0.14, 0.15, 0.19);
pub const LAMP_BALL: Color = Color::srgb(0.30, 1.30, 0.45);
pub const LAMP_STRIKE: Color = Color::srgb(1.40, 0.85, 0.20);
pub const LAMP_OUT: Color = Color::srgb(1.40, 0.28, 0.24);

/// Runner pips on the little base diamond.
pub const BASE_EMPTY: Color = Color::srgb(0.16, 0.17, 0.22);
pub const BASE_OCCUPIED: Color = Color::srgb(1.30, 0.90, 0.25);

pub const BANNER_GOOD: Color = Color::srgb(1.40, 1.10, 0.30);
pub const BANNER_BAD: Color = Color::srgb(0.85, 0.88, 0.95);

/// Scales a colour's brightness, keeping its alpha. Used for glows and dimming.
pub fn scale(color: Color, factor: f32) -> Color {
    let linear = color.to_linear();
    Color::linear_rgba(
        linear.red * factor,
        linear.green * factor,
        linear.blue * factor,
        linear.alpha,
    )
}

pub fn with_alpha(color: Color, alpha: f32) -> Color {
    color.with_alpha(alpha)
}

//! Palette, sizing and layout constants.
//!
//! Colours are authored in linear RGB with values deliberately pushed past 1.0
//! for the pieces themselves: the camera renders to an HDR target with bloom, so
//! anything brighter than white blooms into the surrounding darkness.

use bevy::prelude::*;

use super::piece::PieceKind;

/// Edge length of one board cell, in world pixels.
pub const CELL: f32 = 26.0;
/// Gap between the drawn block and its cell boundary.
pub const CELL_INSET: f32 = 2.0;

// --- surfaces -------------------------------------------------------------

pub const BACKDROP: Color = Color::srgb(0.031, 0.043, 0.078);
pub const PLAYFIELD: Color = Color::srgb(0.055, 0.078, 0.137);
pub const SOCKET: Color = Color::srgb(0.086, 0.114, 0.192);
pub const PANEL: Color = Color::srgb(0.070, 0.094, 0.161);
pub const PANEL_EDGE: Color = Color::srgb(0.145, 0.184, 0.290);

// --- type -----------------------------------------------------------------

pub const TEXT: Color = Color::srgb(0.898, 0.933, 1.0);
pub const TEXT_DIM: Color = Color::srgb(0.435, 0.494, 0.635);
pub const ACCENT: Color = Color::srgb(0.133, 0.878, 1.0);
pub const ACCENT_WARM: Color = Color::srgb(1.0, 0.686, 0.192);

/// Glow around the board that currently has focus.
pub const FOCUS_GLOW: Color = Color::linear_rgb(0.10, 0.95, 1.30);
/// Glow around the board that does not.
pub const IDLE_GLOW: Color = Color::linear_rgb(0.06, 0.08, 0.16);

/// Body colour of a locked or falling block.
pub fn piece_color(kind: PieceKind) -> Color {
    match kind {
        PieceKind::I => Color::linear_rgb(0.05, 0.72, 1.05),
        PieceKind::O => Color::linear_rgb(1.05, 0.72, 0.06),
        PieceKind::T => Color::linear_rgb(0.62, 0.20, 1.05),
        PieceKind::S => Color::linear_rgb(0.10, 0.92, 0.36),
        PieceKind::Z => Color::linear_rgb(1.05, 0.12, 0.30),
        PieceKind::J => Color::linear_rgb(0.14, 0.30, 1.05),
        PieceKind::L => Color::linear_rgb(1.05, 0.40, 0.04),
    }
}

/// Brighter inner face, drawn on top of the body to give each block a bevel.
pub fn piece_shine(kind: PieceKind) -> Color {
    scale(piece_color(kind), 1.9)
}

/// Multiply a colour's linear channels, leaving alpha alone.
pub fn scale(color: Color, factor: f32) -> Color {
    let c = color.to_linear();
    Color::linear_rgba(c.red * factor, c.green * factor, c.blue * factor, c.alpha)
}

/// Replace a colour's alpha channel.
pub fn with_alpha(color: Color, alpha: f32) -> Color {
    let c = color.to_linear();
    Color::linear_rgba(c.red, c.green, c.blue, alpha)
}

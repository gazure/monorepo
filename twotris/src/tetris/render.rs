//! Arena construction and per-frame board drawing.
//!
//! Every cell of the visible field owns two persistent sprites: a body and a
//! brighter inner face that gives the block a bevel. [`draw_boards`] repaints
//! them from board state every frame rather than reacting to redraw messages —
//! which is what the old ghost-piece desync came down to.

use bevy::prelude::*;

use super::{
    Active, Arena, BoardSlot, Focus, Fonts, Hold, Paused, RandomSource, Scoreboard,
    board::{Board, COLS, HIDDEN_ROWS, ROWS, SPAWN_X, SPAWN_Y, VISIBLE_ROWS},
    game::RestartRequest,
    piece::{Bag, Piece, PieceKind},
    theme::{self, CELL, CELL_INSET},
    ui,
};

pub const BOARD_W: f32 = COLS as f32 * CELL;
pub const BOARD_H: f32 = VISIBLE_ROWS as f32 * CELL;
pub const PANEL_W: f32 = 112.0;
const PANEL_GAP: f32 = 14.0;
const UNIT_W: f32 = BOARD_W + PANEL_GAP + PANEL_W;
const UNIT_GAP: f32 = 60.0;
pub const TOTAL_W: f32 = UNIT_W * 2.0 + UNIT_GAP;

pub const BOARD_CENTER_Y: f32 = -40.0;
pub const HUD_H: f32 = 78.0;
pub const HUD_CENTER_Y: f32 = BOARD_CENTER_Y + BOARD_H / 2.0 + 24.0 + HUD_H / 2.0;

// Local z-order within a board.
const Z_GLOW: f32 = -3.0;
const Z_FIELD: f32 = -2.0;
const Z_SOCKET: f32 = -1.0;
const Z_BODY: f32 = 0.0;
const Z_SHINE: f32 = 0.1;
pub const Z_EFFECT: f32 = 6.0;

/// World-space x of a board's centre.
pub fn board_x(slot: usize) -> f32 {
    -TOTAL_W / 2.0 + slot as f32 * (UNIT_W + UNIT_GAP) + BOARD_W / 2.0
}

/// World-space x of a board's side panel.
pub fn panel_x(slot: usize) -> f32 {
    -TOTAL_W / 2.0 + slot as f32 * (UNIT_W + UNIT_GAP) + BOARD_W + PANEL_GAP + PANEL_W / 2.0
}

/// Board-local offset of a cell. Rows in the spawn buffer land above the
/// playfield and are simply never drawn.
pub fn cell_offset(x: usize, y: usize) -> Vec2 {
    Vec2::new(
        (x as f32 - (COLS as f32 - 1.0) / 2.0) * CELL,
        ((VISIBLE_ROWS as f32 - 1.0) / 2.0 - (y as f32 - HIDDEN_ROWS as f32)) * CELL,
    )
}

/// One drawable cell of the visible playfield. Body and shine are separate
/// sibling entities so drawing is a single flat query with no hierarchy walk.
#[derive(Debug, Component)]
pub struct Cell {
    pub x: usize,
    pub y: usize,
    pub shine: bool,
}

/// The halo behind a playfield, which pulses for the focused board.
#[derive(Debug, Component)]
pub struct BoardGlow;

pub fn spawn_arena(
    commands: Commands,
    existing: Query<Entity, With<Arena>>,
    fonts: Res<Fonts>,
    scoreboard: ResMut<Scoreboard>,
    random: ResMut<RandomSource>,
    paused: ResMut<Paused>,
) {
    build_arena(commands, &existing, &fonts, scoreboard, random, paused);
}

/// Rebuilds the arena mid-run when the player restarts from the pause overlay,
/// which stays inside `Playing` and so never re-fires `OnEnter`.
pub fn restart_arena(
    mut request: ResMut<RestartRequest>,
    commands: Commands,
    existing: Query<Entity, With<Arena>>,
    fonts: Res<Fonts>,
    scoreboard: ResMut<Scoreboard>,
    random: ResMut<RandomSource>,
    paused: ResMut<Paused>,
) {
    if !request.0 {
        return;
    }
    request.0 = false;
    build_arena(commands, &existing, &fonts, scoreboard, random, paused);
}

fn build_arena(
    mut commands: Commands,
    existing: &Query<Entity, With<Arena>>,
    fonts: &Fonts,
    mut scoreboard: ResMut<Scoreboard>,
    mut random: ResMut<RandomSource>,
    mut paused: ResMut<Paused>,
) {
    for entity in existing {
        commands.entity(entity).despawn();
    }
    scoreboard.start_run();
    paused.0 = false;

    let root = commands
        .spawn((Arena, Transform::default(), Visibility::default()))
        .id();

    for slot in 0..2 {
        let board = spawn_board(&mut commands, slot, random.as_mut());
        let panel = ui::spawn_panel(&mut commands, slot, fonts);
        commands.entity(root).add_children(&[board, panel]);
    }

    let hud = ui::spawn_hud(&mut commands, fonts);
    commands.entity(root).add_child(hud);
}

fn spawn_board(commands: &mut Commands, slot: usize, random: &mut RandomSource) -> Entity {
    let mut bag = Bag::default();
    let first = Piece::spawn(bag.pop(random), SPAWN_X, SPAWN_Y);

    let mut board = commands.spawn((
        Board::default(),
        BoardSlot(slot),
        bag,
        Hold::default(),
        Active::new(first),
        Transform::from_xyz(board_x(slot), BOARD_CENTER_Y, 0.0),
        Visibility::default(),
    ));
    if slot == 0 {
        board.insert(Focus);
    }

    board.with_children(|parent| {
        // Halo, then the field itself, then the empty-cell sockets.
        parent.spawn((
            BoardGlow,
            Sprite::from_color(theme::IDLE_GLOW, Vec2::new(BOARD_W + 20.0, BOARD_H + 20.0)),
            Transform::from_xyz(0.0, 0.0, Z_GLOW),
        ));
        parent.spawn((
            Sprite::from_color(theme::PLAYFIELD, Vec2::new(BOARD_W + 6.0, BOARD_H + 6.0)),
            Transform::from_xyz(0.0, 0.0, Z_FIELD),
        ));

        for y in HIDDEN_ROWS..ROWS {
            for x in 0..COLS {
                let offset = cell_offset(x, y);
                parent.spawn((
                    Sprite::from_color(theme::SOCKET, Vec2::splat(CELL - CELL_INSET * 2.0 - 2.0)),
                    Transform::from_xyz(offset.x, offset.y, Z_SOCKET),
                ));
                parent.spawn((
                    Cell { x, y, shine: false },
                    Sprite::from_color(Color::NONE, Vec2::splat(CELL - CELL_INSET)),
                    Transform::from_xyz(offset.x, offset.y, Z_BODY),
                    Visibility::Hidden,
                ));
                parent.spawn((
                    Cell { x, y, shine: true },
                    Sprite::from_color(Color::NONE, Vec2::splat(CELL * 0.46)),
                    Transform::from_xyz(offset.x, offset.y + CELL * 0.13, Z_SHINE),
                    Visibility::Hidden,
                ));
            }
        }
    });

    board.id()
}

/// What a single cell position should look like this frame.
enum Paint {
    Empty,
    Ghost(PieceKind),
    /// A solid block, plus how far its lock timer has run (0.0 for settled cells).
    Block(PieceKind, f32),
}

pub fn draw_boards(
    boards: Query<(Entity, &Board, Option<&Active>), With<BoardSlot>>,
    mut cells: Query<(&Cell, &ChildOf, &mut Sprite, &mut Visibility)>,
) {
    for (entity, board, active) in &boards {
        // Cells of the falling piece and of its landing shadow, recomputed every
        // frame so the ghost can never drift out of sync with the piece.
        let (piece_cells, ghost_cells, piece_kind, lock_progress) = match active {
            Some(active) => (
                active.piece.cells(),
                board.ghost(&active.piece).cells(),
                Some(active.piece.kind),
                active.lock.fraction(),
            ),
            None => ([(-1, -1); 4], [(-1, -1); 4], None, 0.0),
        };

        for (cell, child_of, mut sprite, mut visibility) in &mut cells {
            if child_of.parent() != entity {
                continue;
            }
            let coord = (cell.x as i32, cell.y as i32);

            let paint = if let Some(kind) = board.cell(cell.x, cell.y) {
                Paint::Block(kind, 0.0)
            } else if let Some(kind) = piece_kind.filter(|_| piece_cells.contains(&coord)) {
                Paint::Block(kind, lock_progress)
            } else if let Some(kind) = piece_kind.filter(|_| ghost_cells.contains(&coord)) {
                Paint::Ghost(kind)
            } else {
                Paint::Empty
            };

            match paint {
                Paint::Empty => *visibility = Visibility::Hidden,
                // The ghost is a dim wash with no highlight, so it reads as a
                // shadow of the piece rather than another block.
                Paint::Ghost(kind) => {
                    *visibility = if cell.shine {
                        Visibility::Hidden
                    } else {
                        Visibility::Visible
                    };
                    sprite.color = theme::with_alpha(theme::scale(theme::piece_color(kind), 0.6), 0.32);
                }
                Paint::Block(kind, lock) => {
                    *visibility = Visibility::Visible;
                    sprite.color = if cell.shine {
                        theme::piece_shine(kind)
                    } else {
                        // A grounded piece brightens as its lock timer runs out.
                        theme::scale(theme::piece_color(kind), 1.0 + lock * 1.4)
                    };
                }
            }
        }
    }
}

pub fn animate_board_frames(
    time: Res<Time>,
    boards: Query<(&Board, Option<&Focus>), With<BoardSlot>>,
    mut glows: Query<(&ChildOf, &mut Sprite), With<BoardGlow>>,
) {
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 2.6).sin();

    for (child_of, mut glow) in &mut glows {
        let Ok((board, focus)) = boards.get(child_of.parent()) else {
            continue;
        };

        // The focused board breathes; either board tints toward red as it fills.
        let base = if focus.is_some() {
            theme::scale(theme::FOCUS_GLOW, 0.45 + 0.55 * pulse)
        } else {
            theme::IDLE_GLOW
        }
        .to_linear();
        let danger = ((board.fill_ratio() - 0.55) / 0.45).clamp(0.0, 1.0);
        glow.color = Color::linear_rgb(
            base.red + danger * (1.7 - base.red),
            base.green * (1.0 - danger * 0.85),
            base.blue * (1.0 - danger * 0.85),
        );
    }
}

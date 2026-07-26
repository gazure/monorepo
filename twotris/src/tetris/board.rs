//! The playfield: locked cells only.
//!
//! The falling piece is deliberately *not* written into the board. It lives on
//! the board entity as an [`Active`] component and is composited at draw time.
//! Keeping the two apart means collision queries never have to reason about the
//! piece colliding with itself.

use std::fmt::{Display, Formatter, Result as FmtResult};

use bevy::prelude::*;

use super::piece::{Piece, PieceKind};

pub const COLS: usize = 10;
/// Rows the player can see.
pub const VISIBLE_ROWS: usize = 20;
/// Rows above the visible field where pieces spawn.
pub const HIDDEN_ROWS: usize = 2;
pub const ROWS: usize = VISIBLE_ROWS + HIDDEN_ROWS;

pub const SPAWN_X: i32 = 3;
/// Straddles the buffer boundary so a new piece is partly visible at once.
pub const SPAWN_Y: i32 = 1;

/// Offsets tried, in order, when a rotation would collide. This is a
/// simplification of SRS: enough to wall-kick an I-piece off either wall and to
/// floor-kick out of a well, without the full per-shape offset tables.
const KICKS: [(i32, i32); 8] = [(0, 0), (-1, 0), (1, 0), (-2, 0), (2, 0), (0, -1), (-1, -1), (1, -1)];

pub type Row = [Option<PieceKind>; COLS];

#[derive(Debug, Component)]
pub struct Board {
    cells: [Row; ROWS],
}

impl Default for Board {
    fn default() -> Self {
        Self {
            cells: [[None; COLS]; ROWS],
        }
    }
}

impl Board {
    /// Places a cell directly. Test-only: real play only ever fills the board
    /// through [`Board::lock`].
    #[cfg(test)]
    pub fn set_cell(&mut self, x: usize, y: usize, kind: Option<PieceKind>) {
        self.cells[y][x] = kind;
    }

    pub fn cell(&self, x: usize, y: usize) -> Option<PieceKind> {
        self.cells.get(y).and_then(|row| row.get(x)).copied().flatten()
    }

    /// Anything off the sides or below the floor counts as blocked; space above
    /// the buffer is free so a piece can be nudged upward by a floor kick.
    fn blocked(&self, x: i32, y: i32) -> bool {
        if x < 0 || x >= COLS as i32 || y >= ROWS as i32 {
            return true;
        }
        if y < 0 {
            return false;
        }
        self.cells[y as usize][x as usize].is_some()
    }

    pub fn collides(&self, piece: &Piece) -> bool {
        piece.cells().iter().any(|&(x, y)| self.blocked(x, y))
    }

    pub fn lock(&mut self, piece: &Piece) {
        for &(x, y) in &piece.cells() {
            if x >= 0 && y >= 0 && x < COLS as i32 && y < ROWS as i32 {
                self.cells[y as usize][x as usize] = Some(piece.kind);
            }
        }
    }

    pub fn try_move(&self, piece: &Piece, dx: i32, dy: i32) -> Option<Piece> {
        let moved = piece.moved(dx, dy);
        (!self.collides(&moved)).then_some(moved)
    }

    /// Rotate with wall kicks. `dir` is +1 clockwise, -1 counter-clockwise.
    pub fn try_rotate(&self, piece: &Piece, dir: i32) -> Option<Piece> {
        let rotated = piece.rotated(dir);
        KICKS
            .iter()
            .map(|&(dx, dy)| rotated.moved(dx, dy))
            .find(|candidate| !self.collides(candidate))
    }

    /// How many rows the piece can fall before it would collide.
    pub fn drop_distance(&self, piece: &Piece) -> i32 {
        let mut distance = 0;
        while !self.collides(&piece.moved(0, distance + 1)) {
            distance += 1;
        }
        distance
    }

    pub fn ghost(&self, piece: &Piece) -> Piece {
        piece.moved(0, self.drop_distance(piece))
    }

    pub fn resting(&self, piece: &Piece) -> bool {
        self.collides(&piece.moved(0, 1))
    }

    /// Rows that are ready to clear. Separate from [`Board::clear_full_rows`] so
    /// callers can grab the row colours for confetti before they are wiped.
    pub fn full_rows(&self) -> Vec<usize> {
        (0..ROWS)
            .filter(|&y| self.cells[y].iter().all(Option::is_some))
            .collect()
    }

    pub fn row(&self, y: usize) -> Row {
        self.cells.get(y).copied().unwrap_or([None; COLS])
    }

    /// Removes every full row and compacts what remains downward. Returns the
    /// indices that were cleared, so the renderer can flash them.
    pub fn clear_full_rows(&mut self) -> Vec<usize> {
        let cleared = self.full_rows();
        if cleared.is_empty() {
            return cleared;
        }
        let kept: Vec<Row> = (0..ROWS)
            .filter(|y| !cleared.contains(y))
            .map(|y| self.cells[y])
            .collect();
        let mut compacted = [[None; COLS]; ROWS];
        let offset = ROWS - kept.len();
        for (i, row) in kept.into_iter().enumerate() {
            compacted[offset + i] = row;
        }
        self.cells = compacted;
        cleared
    }

    /// Highest occupied row, as a fraction of the visible field. Drives the
    /// danger tint on the playfield frame.
    pub fn fill_ratio(&self) -> f32 {
        let top = (0..ROWS).find(|&y| self.cells[y].iter().any(Option::is_some));
        match top {
            Some(y) => {
                let filled = ROWS.saturating_sub(y) as f32;
                (filled / VISIBLE_ROWS as f32).clamp(0.0, 1.0)
            }
            None => 0.0,
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        for row in &self.cells {
            for cell in row {
                write!(f, "{}", if cell.is_some() { "X" } else { "." })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn fill_row(board: &mut Board, y: usize) {
        for x in 0..COLS {
            board.cells[y][x] = Some(PieceKind::O);
        }
    }

    #[test]
    fn a_new_board_is_empty_and_nothing_collides() {
        let board = Board::default();
        assert!(!board.collides(&Piece::spawn(PieceKind::T, SPAWN_X, SPAWN_Y)));
        assert!(board.fill_ratio() < f32::EPSILON);
    }

    #[test]
    fn walls_and_floor_block_movement() {
        let board = Board::default();
        let piece = Piece::spawn(PieceKind::O, 0, 0);
        assert!(board.try_move(&piece, -1, 0).is_none(), "moved through the left wall");
        let piece = Piece::spawn(PieceKind::O, COLS as i32 - 2, 0);
        assert!(board.try_move(&piece, 1, 0).is_none(), "moved through the right wall");
    }

    #[test]
    fn a_piece_drops_to_the_floor() {
        let board = Board::default();
        let piece = Piece::spawn(PieceKind::O, SPAWN_X, SPAWN_Y);
        let ghost = board.ghost(&piece);
        assert!(board.resting(&ghost), "ghost is not resting on anything");
        assert!(board.collides(&ghost.moved(0, 1)), "ghost could fall further");
    }

    #[test]
    fn clearing_compacts_the_rows_above() {
        let mut board = Board::default();
        fill_row(&mut board, ROWS - 1);
        board.cells[ROWS - 2][0] = Some(PieceKind::T);

        assert_eq!(board.clear_full_rows(), vec![ROWS - 1]);
        // The lone cell that was sitting on the cleared row falls into it.
        assert_eq!(board.cell(0, ROWS - 1), Some(PieceKind::T));
        assert_eq!(board.cell(0, ROWS - 2), None);
    }

    #[test]
    fn clearing_four_rows_at_once_reports_all_of_them() {
        let mut board = Board::default();
        for y in ROWS - 4..ROWS {
            fill_row(&mut board, y);
        }
        assert_eq!(board.clear_full_rows().len(), 4);
        assert!(
            board.fill_ratio() < f32::EPSILON,
            "board should be empty after the clear"
        );
    }

    #[test]
    fn an_i_piece_kicks_off_the_right_wall() {
        let board = Board::default();
        // Vertical I flush against the right wall; rotating it flat needs a kick.
        let piece = Piece {
            kind: PieceKind::I,
            rotation: 1,
            x: COLS as i32 - 3,
            y: 4,
        };
        assert!(!board.collides(&piece), "test setup is already colliding");
        let rotated = board.try_rotate(&piece, 1).expect("rotation should kick off the wall");
        assert!(!board.collides(&rotated));
    }

    #[test]
    fn rotation_fails_when_boxed_in_on_every_side() {
        let mut board = Board::default();
        for y in 0..ROWS {
            for x in 0..COLS {
                board.cells[y][x] = Some(PieceKind::O);
            }
        }
        // Carve out exactly the two cells a vertical I would need, and nothing else.
        board.cells[5][4] = None;
        board.cells[6][4] = None;
        let piece = Piece {
            kind: PieceKind::T,
            rotation: 0,
            x: 4,
            y: 5,
        };
        assert!(board.try_rotate(&piece, 1).is_none(), "rotated inside solid rock");
    }
}

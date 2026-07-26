//! Tetromino shapes, rotation states and the 7-bag randomiser.
//!
//! Every shape is stored as four `(x, y)` cell offsets per rotation, in a const
//! table. `y` grows downward to match the board's row indexing.

use std::collections::VecDeque;

use bevy::prelude::*;

use super::RandomSource;

/// Cell offsets of one rotation state, relative to the piece origin.
type Cells = [(i32, i32); 4];
/// The four clockwise rotation states of a shape.
type Rotations = [Cells; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind {
    I,
    O,
    T,
    S,
    Z,
    J,
    L,
}

pub const KINDS: [PieceKind; 7] = [
    PieceKind::I,
    PieceKind::O,
    PieceKind::T,
    PieceKind::S,
    PieceKind::Z,
    PieceKind::J,
    PieceKind::L,
];

const I_ROTATIONS: Rotations = [
    [(0, 1), (1, 1), (2, 1), (3, 1)],
    [(2, 0), (2, 1), (2, 2), (2, 3)],
    [(0, 2), (1, 2), (2, 2), (3, 2)],
    [(1, 0), (1, 1), (1, 2), (1, 3)],
];

const O_ROTATIONS: Rotations = [
    [(0, 0), (1, 0), (0, 1), (1, 1)],
    [(0, 0), (1, 0), (0, 1), (1, 1)],
    [(0, 0), (1, 0), (0, 1), (1, 1)],
    [(0, 0), (1, 0), (0, 1), (1, 1)],
];

const T_ROTATIONS: Rotations = [
    [(1, 0), (0, 1), (1, 1), (2, 1)],
    [(1, 0), (1, 1), (2, 1), (1, 2)],
    [(0, 1), (1, 1), (2, 1), (1, 2)],
    [(1, 0), (0, 1), (1, 1), (1, 2)],
];

const S_ROTATIONS: Rotations = [
    [(1, 0), (2, 0), (0, 1), (1, 1)],
    [(1, 0), (1, 1), (2, 1), (2, 2)],
    [(1, 1), (2, 1), (0, 2), (1, 2)],
    [(0, 0), (0, 1), (1, 1), (1, 2)],
];

const Z_ROTATIONS: Rotations = [
    [(0, 0), (1, 0), (1, 1), (2, 1)],
    [(2, 0), (1, 1), (2, 1), (1, 2)],
    [(0, 1), (1, 1), (1, 2), (2, 2)],
    [(1, 0), (0, 1), (1, 1), (0, 2)],
];

const J_ROTATIONS: Rotations = [
    [(0, 0), (0, 1), (1, 1), (2, 1)],
    [(1, 0), (2, 0), (1, 1), (1, 2)],
    [(0, 1), (1, 1), (2, 1), (2, 2)],
    [(1, 0), (1, 1), (0, 2), (1, 2)],
];

const L_ROTATIONS: Rotations = [
    [(2, 0), (0, 1), (1, 1), (2, 1)],
    [(1, 0), (1, 1), (1, 2), (2, 2)],
    [(0, 1), (1, 1), (2, 1), (0, 2)],
    [(0, 0), (1, 0), (1, 1), (1, 2)],
];

impl PieceKind {
    pub const fn rotations(self) -> &'static Rotations {
        match self {
            PieceKind::I => &I_ROTATIONS,
            PieceKind::O => &O_ROTATIONS,
            PieceKind::T => &T_ROTATIONS,
            PieceKind::S => &S_ROTATIONS,
            PieceKind::Z => &Z_ROTATIONS,
            PieceKind::J => &J_ROTATIONS,
            PieceKind::L => &L_ROTATIONS,
        }
    }

    /// Cells of the spawn rotation, normalised so the shape's bounding box
    /// starts at the origin. Used to centre preview thumbnails.
    pub fn preview_cells(self) -> ([(i32, i32); 4], i32, i32) {
        let cells = self.rotations()[0];
        let min_x = cells.iter().map(|c| c.0).min().unwrap_or(0);
        let min_y = cells.iter().map(|c| c.1).min().unwrap_or(0);
        let max_x = cells.iter().map(|c| c.0).max().unwrap_or(0);
        let max_y = cells.iter().map(|c| c.1).max().unwrap_or(0);
        let mut out = cells;
        for cell in &mut out {
            cell.0 -= min_x;
            cell.1 -= min_y;
        }
        (out, max_x - min_x + 1, max_y - min_y + 1)
    }
}

/// A tetromino positioned on a board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub rotation: usize,
    /// Column of the piece's bounding-box origin.
    pub x: i32,
    /// Row of the piece's bounding-box origin.
    pub y: i32,
}

impl Piece {
    pub fn spawn(kind: PieceKind, x: i32, y: i32) -> Self {
        Self {
            kind,
            rotation: 0,
            x,
            y,
        }
    }

    /// Absolute board coordinates occupied by this piece.
    pub fn cells(&self) -> Cells {
        let mut out = self.kind.rotations()[self.rotation];
        for cell in &mut out {
            cell.0 += self.x;
            cell.1 += self.y;
        }
        out
    }

    pub fn moved(&self, dx: i32, dy: i32) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }

    /// `dir` is +1 for clockwise, -1 for counter-clockwise.
    pub fn rotated(&self, dir: i32) -> Self {
        let rotation = (self.rotation as i32 + dir).rem_euclid(4) as usize;
        Self { rotation, ..*self }
    }
}

/// 7-bag randomiser: every shape appears once per bag, so droughts are bounded.
#[derive(Debug, Component, Default)]
pub struct Bag {
    queue: VecDeque<PieceKind>,
}

impl Bag {
    /// Keeps at least two bags queued so `peek` never comes up short.
    fn top_up(&mut self, rng: &mut RandomSource) {
        while self.queue.len() <= KINDS.len() {
            let mut bag = KINDS;
            // Fisher-Yates, driven by the seeded run RNG.
            for i in (1..bag.len()).rev() {
                let j = rng.next(0, i as u32 + 1) as usize;
                bag.swap(i, j);
            }
            self.queue.extend(bag);
        }
    }

    pub fn pop(&mut self, rng: &mut RandomSource) -> PieceKind {
        self.top_up(rng);
        self.queue.pop_front().unwrap_or(PieceKind::I)
    }

    /// The next `count` shapes, without consuming them.
    pub fn peek(&mut self, rng: &mut RandomSource, count: usize) -> Vec<PieceKind> {
        self.top_up(rng);
        self.queue.iter().take(count).copied().collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn every_rotation_has_four_cells_in_a_four_wide_box() {
        for kind in KINDS {
            for cells in kind.rotations() {
                assert_eq!(cells.len(), 4);
                for &(x, y) in cells {
                    assert!((0..4).contains(&x), "{kind:?} x={x} out of box");
                    assert!((0..4).contains(&y), "{kind:?} y={y} out of box");
                }
            }
        }
    }

    #[test]
    fn rotating_four_times_returns_to_spawn() {
        for kind in KINDS {
            let piece = Piece::spawn(kind, 3, 0);
            let spun = piece.rotated(1).rotated(1).rotated(1).rotated(1);
            assert_eq!(piece, spun, "{kind:?} did not round-trip");
        }
    }

    #[test]
    fn counter_clockwise_undoes_clockwise() {
        for kind in KINDS {
            let piece = Piece::spawn(kind, 3, 0);
            assert_eq!(piece, piece.rotated(1).rotated(-1), "{kind:?}");
        }
    }

    #[test]
    fn a_bag_deals_each_shape_once_before_repeating() {
        let mut rng = RandomSource::default();
        let mut bag = Bag::default();
        let dealt: Vec<_> = (0..KINDS.len()).map(|_| bag.pop(&mut rng)).collect();
        for kind in KINDS {
            assert_eq!(
                dealt.iter().filter(|k| **k == kind).count(),
                1,
                "{kind:?} appeared the wrong number of times in {dealt:?}"
            );
        }
    }

    #[test]
    fn peek_agrees_with_the_pops_that_follow() {
        let mut rng = RandomSource::default();
        let mut bag = Bag::default();
        let peeked = bag.peek(&mut rng, 3);
        let popped: Vec<_> = (0..3).map(|_| bag.pop(&mut rng)).collect();
        assert_eq!(peeked, popped);
    }
}

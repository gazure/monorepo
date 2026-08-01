//! Gameplay: input, gravity, locking, scoring.

use std::time::Duration;

use bevy::prelude::*;
use tracingx::debug;

use super::{
    Active, Focus, GameState, Hold, LinesCleared, Paused, PieceLocked, RandomSource, Scoreboard,
    board::{Board, SPAWN_X, SPAWN_Y},
    effects::ScreenShake,
    piece::{Bag, Piece},
};

/// Gravity interval at level 1, in seconds.
pub const BASE_GRAVITY: f32 = 0.80;
/// Seconds a grounded piece waits before locking.
pub const LOCK_DELAY: f32 = 0.50;
/// How many times moving or rotating may refresh the lock timer. Without a cap,
/// spinning a piece in place stalls the game forever.
pub const MAX_LOCK_RESETS: u8 = 15;
/// Levels stop getting faster here.
const MAX_LEVEL: u32 = 20;

/// Delay before auto-repeat kicks in, in seconds.
const DAS_DELAY: f32 = 0.14;
/// Interval between auto-repeated shifts.
const ARR_INTERVAL: f32 = 0.035;
/// Soft drop is this many times faster than the current gravity.
const SOFT_DROP_FACTOR: f32 = 14.0;
/// The unfocused board falls at this multiple of the focused board's interval.
const UNFOCUSED_GRAVITY_SCALE: f32 = 2.0;

/// Everything gravity needs from a board, in one place.
type BoardsQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Board,
        &'static mut Active,
        &'static mut Hold,
        &'static mut Bag,
        Option<&'static Focus>,
    ),
>;

/// Auto-repeat state for horizontal movement (delayed auto shift).
#[derive(Debug, Default, Resource)]
pub struct Das {
    dir: i32,
    timer: f32,
    repeating: bool,
}

impl Das {
    fn reset(&mut self) {
        self.dir = 0;
        self.timer = 0.0;
        self.repeating = false;
    }
}

/// Set when the player asks for a fresh run without leaving `Playing`, which
/// would not re-fire `OnEnter` on its own.
#[derive(Debug, Default, Resource)]
pub struct RestartRequest(pub bool);

fn gravity_interval(level: u32) -> f32 {
    (BASE_GRAVITY - (level.min(MAX_LEVEL) - 1) as f32 * 0.037).max(0.05)
}

/// Applies a clear to the scoreboard and reports what to shout about it.
fn score_clear(scoreboard: &mut Scoreboard, rows: usize) -> (u32, Option<&'static str>) {
    let base: u32 = match rows {
        1 => 100,
        2 => 300,
        3 => 500,
        4 => 800,
        _ => return (0, None),
    };

    let tetris = rows == 4;
    let chained = tetris && scoreboard.back_to_back;
    let mut points = base * scoreboard.level;
    if chained {
        points = points * 3 / 2;
    }

    scoreboard.combo += 1;
    if scoreboard.combo > 0 {
        points += 50 * scoreboard.combo as u32 * scoreboard.level;
    }

    scoreboard.back_to_back = tetris;
    scoreboard.lines += rows as u32;
    scoreboard.level = (1 + scoreboard.lines / 10).min(MAX_LEVEL);
    scoreboard.score += points;

    let label = match (rows, chained) {
        (1, _) => "SINGLE",
        (2, _) => "DOUBLE",
        (3, _) => "TRIPLE",
        (4, true) => "B2B TETRIS",
        (4, false) => "TETRIS",
        _ => return (points, None),
    };
    (points, Some(label))
}

pub fn swap_focus(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut das: ResMut<Das>,
    focused: Query<Entity, (With<Board>, With<Focus>)>,
    unfocused: Query<Entity, (With<Board>, Without<Focus>)>,
) {
    if !input.just_pressed(KeyCode::KeyF) && !input.just_pressed(KeyCode::Tab) {
        return;
    }
    debug!("swapping focus");
    for entity in &focused {
        commands.entity(entity).remove::<Focus>();
    }
    for entity in &unfocused {
        commands.entity(entity).insert(Focus);
    }
    // Otherwise the new board inherits a half-charged auto-repeat.
    das.reset();
}

pub fn handle_input(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut das: ResMut<Das>,
    mut random: ResMut<RandomSource>,
    mut scoreboard: ResMut<Scoreboard>,
    mut shake: ResMut<ScreenShake>,
    mut boards: Query<(&Board, &mut Active, &mut Hold, &mut Bag), With<Focus>>,
) {
    let Ok((board, mut active, mut hold, mut bag)) = boards.single_mut() else {
        return;
    };
    let grounded = board.resting(&active.piece);

    // --- horizontal, with delayed auto shift ---
    let dir = i32::from(input.pressed(KeyCode::ArrowRight)) - i32::from(input.pressed(KeyCode::ArrowLeft));
    let mut shift = 0;
    if dir == 0 {
        das.reset();
    } else if das.dir == dir {
        das.timer += time.delta_secs();
        let threshold = if das.repeating { ARR_INTERVAL } else { DAS_DELAY };
        if das.timer >= threshold {
            das.timer -= threshold;
            das.repeating = true;
            shift = dir;
        }
    } else {
        das.dir = dir;
        das.timer = 0.0;
        das.repeating = false;
        shift = dir;
    }
    if shift != 0
        && let Some(moved) = board.try_move(&active.piece, shift, 0)
    {
        active.piece = moved;
        if grounded {
            active.nudge();
        }
    }

    // --- rotation ---
    let spin = i32::from(input.just_pressed(KeyCode::ArrowUp) || input.just_pressed(KeyCode::KeyX))
        - i32::from(input.just_pressed(KeyCode::KeyZ));
    if spin != 0
        && let Some(rotated) = board.try_rotate(&active.piece, spin)
    {
        active.piece = rotated;
        if grounded {
            active.nudge();
        }
    }

    // --- hold ---
    if input.just_pressed(KeyCode::KeyC) && !hold.used {
        let incoming = match hold.kind {
            Some(kind) => kind,
            None => bag.pop(random.as_mut()),
        };
        let swapped = Piece::spawn(incoming, SPAWN_X, SPAWN_Y);
        // Refuse the swap rather than topping out on it.
        if !board.collides(&swapped) {
            hold.kind = Some(active.piece.kind);
            hold.used = true;
            *active = Active::new(swapped);
        }
    }

    // --- hard drop ---
    if input.just_pressed(KeyCode::Space) {
        let distance = board.drop_distance(&active.piece);
        active.piece = active.piece.moved(0, distance);
        scoreboard.score += 2 * distance as u32;
        active.force_lock = true;
        shake.add(0.12 + distance as f32 * 0.004);
    }
}

pub fn apply_gravity(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut random: ResMut<RandomSource>,
    mut scoreboard: ResMut<Scoreboard>,
    mut shake: ResMut<ScreenShake>,
    mut next_state: ResMut<NextState<GameState>>,
    mut cleared: MessageWriter<LinesCleared>,
    mut locked: MessageWriter<PieceLocked>,
    mut boards: BoardsQuery,
) {
    let soft_drop_held = input.pressed(KeyCode::ArrowDown);
    let level_interval = gravity_interval(scoreboard.level);

    for (entity, mut board, mut active, mut hold, mut bag, focus) in &mut boards {
        let focused = focus.is_some();
        let soft = focused && soft_drop_held;
        let interval = if soft {
            (level_interval / SOFT_DROP_FACTOR).max(0.012)
        } else if focused {
            level_interval
        } else {
            level_interval * UNFOCUSED_GRAVITY_SCALE
        };
        active.fall.set_duration(Duration::from_secs_f32(interval));
        active.fall.tick(time.delta());

        let resting = board.resting(&active.piece);
        if resting {
            active.lock.tick(time.delta());
        } else {
            active.lock.reset();
            if active.fall.is_finished()
                && let Some(moved) = board.try_move(&active.piece, 0, 1)
            {
                active.piece = moved;
                if soft {
                    scoreboard.score += 1;
                }
            }
        }

        let should_lock = active.force_lock || (resting && active.lock.is_finished());
        if !should_lock {
            continue;
        }

        // --- lock ---
        let piece = active.piece;
        board.lock(&piece);
        locked.write(PieceLocked { board: entity, piece });

        let rows = board.full_rows();
        if rows.is_empty() {
            scoreboard.combo = -1;
        } else {
            // Grab the row colours before the clear wipes them.
            let swatches = rows.iter().map(|&y| board.row(y)).collect();
            board.clear_full_rows();

            let count = rows.len();
            let (points, label) = score_clear(scoreboard.as_mut(), count);
            shake.add(0.14 + count as f32 * 0.10);
            cleared.write(LinesCleared {
                board: entity,
                rows,
                swatches,
                points,
                label,
            });
        }

        hold.used = false;
        let next = Piece::spawn(bag.pop(random.as_mut()), SPAWN_X, SPAWN_Y);
        if board.collides(&next) {
            debug!("board {entity} topped out");
            next_state.set(GameState::GameOver);
        } else {
            *active = Active::new(next);
        }
    }
}

pub fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    mut paused: ResMut<Paused>,
    mut restart: ResMut<RestartRequest>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if input.just_pressed(KeyCode::Escape) {
        paused.0 = !paused.0;
    }
    if !paused.0 {
        return;
    }
    if input.just_pressed(KeyCode::KeyR) {
        paused.0 = false;
        restart.0 = true;
    }
    if input.just_pressed(KeyCode::KeyT) {
        paused.0 = false;
        next_state.set(GameState::Title);
    }
}

/// Stops the boards on a top-out and banks the run's score.
pub fn clear_active(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut paused: ResMut<Paused>,
    boards: Query<Entity, With<Active>>,
) {
    paused.0 = false;
    scoreboard.best = scoreboard.best.max(scoreboard.score);
    for entity in &boards {
        commands.entity(entity).remove::<Active>();
    }
}

#[cfg(test)]
mod test {
    use bevy::ecs::{message::Messages, schedule::SingleThreadedExecutor};

    use super::*;
    use crate::tetris::{
        board::{COLS, ROWS},
        piece::PieceKind,
    };

    /// A bare world running just the gameplay systems: no rendering, no assets,
    /// and a `Time` nothing else overwrites, so gravity can be stepped exactly.
    fn harness() -> (World, Schedule) {
        let mut world = World::new();
        world.insert_resource(Time::<()>::default());
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(RandomSource::default());
        world.insert_resource(Das::default());
        world.insert_resource(ScreenShake::default());
        world.insert_resource(NextState::<GameState>::default());
        world.insert_resource(Messages::<LinesCleared>::default());
        world.insert_resource(Messages::<PieceLocked>::default());

        let mut scoreboard = Scoreboard::default();
        scoreboard.start_run();
        world.insert_resource(scoreboard);

        let mut schedule = Schedule::default();
        // No task pool in this harness, so keep it off the threaded executor.
        schedule.set_executor(SingleThreadedExecutor::default());
        schedule.add_systems((handle_input, apply_gravity).chain());
        (world, schedule)
    }

    fn add_board(world: &mut World, board: Board, piece: Piece, focused: bool) -> Entity {
        let mut entity = world.spawn((board, Bag::default(), Hold::default(), Active::new(piece)));
        if focused {
            entity.insert(Focus);
        }
        entity.id()
    }

    /// Advances the clock and runs one frame, clearing `just_pressed` afterwards
    /// the way Bevy's own input system does.
    fn step(world: &mut World, schedule: &mut Schedule, seconds: f32) {
        world
            .resource_mut::<Time<()>>()
            .advance_by(Duration::from_secs_f32(seconds));
        schedule.run(world);
        world.resource_mut::<ButtonInput<KeyCode>>().clear();
    }

    fn piece_of(world: &mut World, board: Entity) -> Piece {
        world
            .get::<Active>(board)
            .expect("board still has an active piece")
            .piece
    }

    fn fill_row_except(board: &mut Board, y: usize, gaps: &[usize]) {
        for x in 0..COLS {
            if !gaps.contains(&x) {
                board.set_cell(x, y, Some(PieceKind::L));
            }
        }
    }

    #[test]
    fn hard_drop_locks_at_once_and_pays_two_points_a_row() {
        let (mut world, mut schedule) = harness();
        let piece = Piece::spawn(PieceKind::O, SPAWN_X, SPAWN_Y);
        let expected = Board::default().drop_distance(&piece);
        let board = add_board(&mut world, Board::default(), piece, true);

        world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
        step(&mut world, &mut schedule, 1.0 / 60.0);

        let landed = world.get::<Board>(board).expect("board exists");
        assert_eq!(landed.cell(SPAWN_X as usize, ROWS - 1), Some(PieceKind::O));
        assert_eq!(world.resource::<Scoreboard>().score, 2 * expected as u32);
    }

    #[test]
    fn a_grounded_piece_waits_out_the_lock_delay_before_settling() {
        let (mut world, mut schedule) = harness();
        // Start the piece already sitting on the floor.
        let resting = Board::default().ghost(&Piece::spawn(PieceKind::O, SPAWN_X, SPAWN_Y));
        let board = add_board(&mut world, Board::default(), resting, true);

        step(&mut world, &mut schedule, LOCK_DELAY * 0.5);
        assert!(
            world.get::<Board>(board).expect("board exists").full_rows().is_empty()
                && world
                    .get::<Board>(board)
                    .expect("board exists")
                    .cell(SPAWN_X as usize, ROWS - 1)
                    .is_none(),
            "piece locked before the lock delay elapsed"
        );

        step(&mut world, &mut schedule, LOCK_DELAY);
        assert_eq!(
            world
                .get::<Board>(board)
                .expect("board exists")
                .cell(SPAWN_X as usize, ROWS - 1),
            Some(PieceKind::O),
            "piece never locked after the lock delay"
        );
    }

    #[test]
    fn completing_a_row_clears_it_and_scores_it() {
        let (mut world, mut schedule) = harness();
        let mut board = Board::default();
        // An O-piece dropped at the spawn column plugs exactly this gap.
        fill_row_except(&mut board, ROWS - 1, &[SPAWN_X as usize, SPAWN_X as usize + 1]);
        let entity = add_board(&mut world, board, Piece::spawn(PieceKind::O, SPAWN_X, SPAWN_Y), true);

        world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
        step(&mut world, &mut schedule, 1.0 / 60.0);

        let scoreboard = world.resource::<Scoreboard>();
        assert_eq!(scoreboard.lines, 1, "the completed row was not counted");
        let cleared = world.get::<Board>(entity).expect("board exists");
        assert!(
            cleared.full_rows().is_empty(),
            "a full row survived the clear:\n{cleared}"
        );
    }

    #[test]
    fn a_blocked_spawn_ends_the_run() {
        let (mut world, mut schedule) = harness();
        let mut board = Board::default();
        // Wall off the spawn area, leaving a gap so no row is complete and the
        // clear cannot rescue us.
        for y in 0..=SPAWN_Y as usize + 2 {
            fill_row_except(&mut board, y, &[COLS - 1]);
        }
        let resting = board.ghost(&Piece::spawn(PieceKind::O, SPAWN_X, ROWS as i32 - 4));
        add_board(&mut world, board, resting, true);

        world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
        step(&mut world, &mut schedule, 1.0 / 60.0);

        assert!(
            matches!(
                world.resource::<NextState<GameState>>(),
                NextState::Pending(GameState::GameOver)
            ),
            "a blocked spawn should end the run"
        );
    }

    #[test]
    fn the_unfocused_board_falls_at_half_speed() {
        let (mut world, mut schedule) = harness();
        let piece = Piece::spawn(PieceKind::O, SPAWN_X, SPAWN_Y);
        let focused = add_board(&mut world, Board::default(), piece, true);
        let idle = add_board(&mut world, Board::default(), piece, false);

        // Enough time for one step at the focused interval, but not at double it.
        let mut elapsed = 0.0;
        while elapsed < BASE_GRAVITY + 0.1 {
            step(&mut world, &mut schedule, 1.0 / 60.0);
            elapsed += 1.0 / 60.0;
        }

        assert_eq!(
            piece_of(&mut world, focused).y,
            piece.y + 1,
            "the focused board should have dropped one row"
        );
        assert_eq!(
            piece_of(&mut world, idle).y,
            piece.y,
            "the unfocused board should still be waiting"
        );
    }

    fn board_at(level: u32) -> Scoreboard {
        Scoreboard {
            level,
            combo: -1,
            ..Scoreboard::default()
        }
    }

    #[test]
    fn gravity_gets_faster_with_level_then_stops() {
        assert!((gravity_interval(1) - BASE_GRAVITY).abs() < f32::EPSILON);
        assert!(gravity_interval(5) < gravity_interval(1));
        assert!(gravity_interval(MAX_LEVEL) >= 0.05);
        assert!(
            (gravity_interval(MAX_LEVEL) - gravity_interval(MAX_LEVEL + 50)).abs() < f32::EPSILON,
            "levels past the cap should not keep accelerating"
        );
    }

    #[test]
    fn a_tetris_scores_more_than_four_singles() {
        let mut singles = board_at(1);
        let mut total = 0;
        for _ in 0..4 {
            total += score_clear(&mut singles, 1).0;
            singles.combo = -1; // clear the combo so we compare bases only
        }
        let mut tetris = board_at(1);
        assert!(score_clear(&mut tetris, 4).0 > total);
    }

    #[test]
    fn back_to_back_tetrises_pay_a_bonus() {
        let mut scoreboard = board_at(1);
        let first = score_clear(&mut scoreboard, 4);
        scoreboard.combo = -1; // isolate the back-to-back multiplier from the combo bonus
        let second = score_clear(&mut scoreboard, 4);
        assert_eq!(first.1, Some("TETRIS"));
        assert_eq!(second.1, Some("B2B TETRIS"));
        assert!(second.0 > first.0);
    }

    #[test]
    fn a_non_tetris_breaks_the_back_to_back_chain() {
        let mut scoreboard = board_at(1);
        score_clear(&mut scoreboard, 4);
        assert!(scoreboard.back_to_back);
        score_clear(&mut scoreboard, 2);
        assert!(!scoreboard.back_to_back);
    }

    #[test]
    fn ten_lines_advances_a_level() {
        let mut scoreboard = board_at(1);
        for _ in 0..5 {
            score_clear(&mut scoreboard, 2);
        }
        assert_eq!(scoreboard.lines, 10);
        assert_eq!(scoreboard.level, 2);
    }
}

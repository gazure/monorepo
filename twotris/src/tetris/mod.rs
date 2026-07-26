//! Twotris: two playfields, one falling-block game, one set of hands.
//!
//! Both boards run gravity at once; the unfocused board falls at half speed so
//! neglecting it is punishing rather than hopeless. `F` swaps which board takes
//! your input.

mod board;
mod effects;
mod game;
mod piece;
mod render;
mod theme;
mod ui;

use bevy::{
    camera::{Hdr, ScalingMode},
    post_process::bloom::Bloom,
    prelude::*,
};
use piece::{Piece, PieceKind};
use rand::RngExt;

/// Seeded RNG for a run, so a session is reproducible from its seed.
#[derive(Debug, Resource)]
pub struct RandomSource(rand_chacha::ChaCha8Rng);

impl Default for RandomSource {
    fn default() -> Self {
        RandomSource(rand::make_rng())
    }
}

impl RandomSource {
    /// Uniform in `[min, max)`.
    pub fn next(&mut self, min: u32, max: u32) -> u32 {
        self.0.random_range(min..max)
    }

    /// Uniform in `[min, max)`, as a float.
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.0.random_range(0.0..1.0f32)
    }
}

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Title,
    Playing,
    GameOver,
}

/// Pause is a resource rather than a state: as a state, resuming would re-fire
/// `OnEnter(Playing)` and silently restart the run.
#[derive(Debug, Default, Resource)]
pub struct Paused(pub bool);

fn running(state: Res<State<GameState>>, paused: Res<Paused>) -> bool {
    *state.get() == GameState::Playing && !paused.0
}

#[derive(Debug, Default, Resource)]
pub struct Scoreboard {
    pub score: u32,
    pub lines: u32,
    pub level: u32,
    /// Consecutive clearing locks, minus one. `-1` means the chain is broken.
    pub combo: i32,
    pub back_to_back: bool,
    pub best: u32,
}

impl Scoreboard {
    fn start_run(&mut self) {
        self.best = self.best.max(self.score);
        self.score = 0;
        self.lines = 0;
        self.level = 1;
        self.combo = -1;
        self.back_to_back = false;
    }
}

/// Fonts loaded once at startup and shared by every text entity.
#[derive(Debug, Resource)]
pub struct Fonts {
    pub bold: Handle<Font>,
    pub medium: Handle<Font>,
}

/// Root of everything spawned for a run. Despawned as one unit.
#[derive(Debug, Component)]
pub struct Arena;

/// Which of the two playfields this is.
#[derive(Debug, Component)]
pub struct BoardSlot(pub usize);

/// Marks the single board that receives input.
#[derive(Debug, Component)]
pub struct Focus;

/// The piece currently falling on a board, plus its gravity and lock timers.
#[derive(Debug, Component)]
pub struct Active {
    pub piece: Piece,
    pub fall: Timer,
    pub lock: Timer,
    pub lock_resets: u8,
    pub force_lock: bool,
}

impl Active {
    fn new(piece: Piece) -> Self {
        Self {
            piece,
            fall: Timer::from_seconds(game::BASE_GRAVITY, TimerMode::Repeating),
            lock: Timer::from_seconds(game::LOCK_DELAY, TimerMode::Once),
            lock_resets: 0,
            force_lock: false,
        }
    }

    /// Called after a successful move or rotation: refresh the lock timer so
    /// the player can keep adjusting, but only a bounded number of times.
    fn nudge(&mut self) {
        if self.lock_resets < game::MAX_LOCK_RESETS {
            self.lock_resets += 1;
            self.lock.reset();
        }
    }
}

/// The piece held in reserve on a board. `used` blocks a second hold until the
/// current piece locks.
#[derive(Debug, Default, Component)]
pub struct Hold {
    pub kind: Option<PieceKind>,
    pub used: bool,
}

/// Emitted for every lock that cleared at least one row.
#[derive(Debug, Message)]
pub struct LinesCleared {
    pub board: Entity,
    pub rows: Vec<usize>,
    /// Contents of each cleared row, captured before the clear, so the confetti
    /// can be thrown in the colours of the blocks that were standing there.
    pub swatches: Vec<board::Row>,
    pub points: u32,
    pub label: Option<&'static str>,
}

/// Emitted for every lock, cleared rows or not.
#[derive(Debug, Message)]
pub struct PieceLocked {
    pub board: Entity,
    pub piece: Piece,
}

/// Loaded during plugin build rather than in `Startup`: `bevy_state` registers
/// the initial `StateTransition` *before* `PreStartup`, so `OnEnter(Title)` — and
/// the text it spawns — runs before any startup system would have got here.
fn load_fonts(app: &App) -> Fonts {
    let asset_server = app
        .world()
        .get_resource::<AssetServer>()
        .expect("TetrisPlugin must be added after DefaultPlugins");
    Fonts {
        bold: asset_server.load("fonts/JetBrainsMono-ExtraBold.ttf"),
        medium: asset_server.load("fonts/JetBrainsMono-Medium.ttf"),
    }
}

/// World units the arena needs. The camera guarantees at least this much is
/// visible, so the layout survives any window size instead of being pinned to
/// one resolution in pixels.
const MIN_VIEW_WIDTH: f32 = 920.0;
const MIN_VIEW_HEIGHT: f32 = 700.0;

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        // The piece palette is authored above 1.0 in linear space; without an
        // HDR target those values would clip and bloom would have nothing to
        // pick up.
        Hdr,
        Bloom {
            intensity: 0.22,
            low_frequency_boost: 0.6,
            ..Bloom::NATURAL
        },
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: MIN_VIEW_WIDTH,
                min_height: MIN_VIEW_HEIGHT,
            },
            ..OrthographicProjection::default_2d()
        }),
    ));
}

/// Generic cleanup for tag components.
fn despawn_all<T: Component>(mut commands: Commands, entities: Query<Entity, With<T>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

pub struct TetrisPlugin;

impl Plugin for TetrisPlugin {
    fn build(&self, app: &mut App) {
        let fonts = load_fonts(app);
        app.insert_resource(fonts)
            .init_resource::<RandomSource>()
            .init_resource::<Scoreboard>()
            .init_resource::<Paused>()
            .init_resource::<game::Das>()
            .init_resource::<game::RestartRequest>()
            .init_resource::<effects::ScreenShake>()
            .init_state::<GameState>()
            .add_message::<LinesCleared>()
            .add_message::<PieceLocked>()
            .insert_resource(ClearColor(theme::BACKDROP))
            .add_systems(Startup, setup_camera)
            // --- title ---
            .add_systems(
                OnEnter(GameState::Title),
                (despawn_all::<Arena>, ui::spawn_title, effects::spawn_title_pieces),
            )
            .add_systems(
                OnExit(GameState::Title),
                (despawn_all::<ui::TitleUi>, despawn_all::<effects::TitlePiece>),
            )
            .add_systems(
                Update,
                (ui::title_input, effects::drift_title_pieces).run_if(in_state(GameState::Title)),
            )
            // --- a run ---
            .add_systems(OnEnter(GameState::Playing), render::spawn_arena)
            .add_systems(Update, render::restart_arena.run_if(in_state(GameState::Playing)))
            .add_systems(
                Update,
                (
                    game::swap_focus,
                    game::handle_input,
                    game::apply_gravity,
                    effects::spawn_clear_effects,
                    effects::spawn_lock_flash,
                )
                    .chain()
                    .run_if(running),
            )
            // Chained: the overlay has to reconcile *after* the toggle, or a pause
            // that exits straight to the title leaves the overlay behind.
            .add_systems(
                Update,
                (game::toggle_pause, ui::sync_pause_overlay)
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(OnExit(GameState::Playing), despawn_all::<ui::PauseUi>)
            // --- game over ---
            // Chained: `clear_active` banks the run's score into `best`, which the
            // overlay then reads.
            .add_systems(
                OnEnter(GameState::GameOver),
                (game::clear_active, ui::spawn_game_over).chain(),
            )
            .add_systems(OnExit(GameState::GameOver), despawn_all::<ui::GameOverUi>)
            .add_systems(Update, ui::game_over_input.run_if(in_state(GameState::GameOver)))
            // --- presentation, which keeps running while paused and after a top-out ---
            .add_systems(
                Update,
                (
                    render::draw_boards,
                    render::animate_board_frames,
                    ui::update_hud,
                    ui::update_previews,
                )
                    .run_if(in_state(GameState::Playing).or_else(in_state(GameState::GameOver))),
            )
            .add_systems(
                Update,
                (
                    effects::drive_particles,
                    effects::drive_popups,
                    effects::drive_flashes,
                    effects::apply_screen_shake,
                    ui::pulse_prompt,
                ),
            );
    }
}

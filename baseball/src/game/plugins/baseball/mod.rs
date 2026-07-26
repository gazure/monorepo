//! A playable nine innings.
//!
//! The player is the home team: they bat in the bottom of every inning and pitch
//! in the top. One pitch is one trip round [`Phase`], and the game ends when the
//! rules engine says it has.
//!
//! Two views share the world. The at-bat view looks in over the catcher's shoulder
//! and is where pitching and hitting happen; the field view is a wide overhead shot
//! used the moment a ball is put in play. They are kept apart by render layers
//! rather than by moving things around, so both scenes can sit at comfortable
//! coordinates and neither has to know the other exists.

mod ball;
mod bat;
mod effects;
mod field;
mod fielding;
mod flow;
mod hud;
mod pitch;
mod scene;
mod screens;
mod theme;
mod view;

use baseball_game_rules::{Game, GameOutcome, InningHalf, PlayerPosition};
use bevy::prelude::*;
use rand::RngExt;

/// Seeded RNG, so a session is reproducible from its seed.
#[derive(Debug, Resource)]
pub struct RandomSource(rand_chacha::ChaCha8Rng);

impl Default for RandomSource {
    fn default() -> Self {
        RandomSource(rand::make_rng())
    }
}

impl RandomSource {
    /// Uniform in `[min, max)`.
    pub fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.0.random_range(0.0..1.0f32)
    }

    /// True with the given probability.
    pub fn chance(&mut self, probability: f32) -> bool {
        self.0.random_range(0.0..1.0f32) < probability
    }

    /// Picks one of a slice's entries.
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        let index = self.0.random_range(0..items.len());
        items[index]
    }
}

/// The half the human plays on offence. Everything else follows from it: they
/// pitch in the other half, and the score bug knows which line is theirs.
pub const PLAYER_HALF: InningHalf = InningHalf::Bottom;

/// One pitch, start to finish.
#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    #[default]
    Title,
    /// Defence set, pitcher choosing what to throw.
    Windup,
    /// Ball on its way to the plate.
    Pitch,
    /// Ball struck and live in the field of play.
    BallInPlay,
    /// Showing what happened, and applying it to the rules engine.
    Resolve,
    /// Card between half innings.
    InningBreak,
    GameOver,
}

impl Phase {
    /// Whether a game is in progress, as opposed to a menu or a card.
    pub fn is_live(self) -> bool {
        matches!(self, Phase::Windup | Phase::Pitch | Phase::BallInPlay | Phase::Resolve)
    }
}

/// Pause is a resource rather than a state: as a state, resuming would re-fire
/// `OnEnter` for the phase and redo whatever it had set up.
#[derive(Debug, Default, Resource)]
pub struct Paused(pub bool);

/// The rules engine's view of the game. The only authority on the score.
#[derive(Debug, Resource)]
pub struct Diamond {
    pub outcome: GameOutcome,
}

impl Default for Diamond {
    fn default() -> Self {
        Self {
            outcome: GameOutcome::InProgress(Game::new()),
        }
    }
}

impl Diamond {
    pub fn game(&self) -> Option<&Game> {
        self.outcome.game_ref()
    }

    /// Which team is batting, or `None` once the game is over.
    pub fn batting_half(&self) -> Option<InningHalf> {
        self.game().map(|game| game.current_half_inning().half())
    }

    pub fn human_is_batting(&self) -> bool {
        self.batting_half() == Some(PLAYER_HALF)
    }

    /// What the defence needs to know to price a batted ball.
    pub fn situation(&self) -> fielding::Situation {
        let Some(game) = self.game() else {
            return fielding::Situation::default();
        };
        let half = game.current_half_inning();
        let runners = half.baserunners();
        fielding::Situation {
            runner_on_first: runners.first().is_some(),
            runner_on_third: runners.third().is_some(),
            outs: half.outs().as_number(),
        }
    }
}

/// Fonts loaded once and shared by every piece of text.
#[derive(Debug, Resource)]
pub struct Fonts {
    pub bold: Handle<Font>,
    pub medium: Handle<Font>,
}

/// Loaded during plugin build rather than in `Startup`, because `bevy_state`
/// registers the initial state transition before `PreStartup` — so `OnEnter(Title)`
/// and the text it spawns run before any startup system would have got here.
fn load_fonts(app: &App) -> Fonts {
    let assets = app
        .world()
        .get_resource::<AssetServer>()
        .expect("BaseballPlugin must be added after DefaultPlugins");
    Fonts {
        bold: assets.load("fonts/JetBrainsMono-ExtraBold.ttf"),
        medium: assets.load("fonts/JetBrainsMono-Medium.ttf"),
    }
}

/// How the batter intends to swing, chosen before the ball arrives.
#[derive(Debug, Default, Resource)]
pub struct BatterIntent {
    pub style: bat::SwingStyle,
}

/// What to print across the middle of the screen.
#[derive(Debug, Default, Resource)]
pub struct Banner {
    pub headline: String,
    pub detail: String,
    pub good_for_batter: bool,
}

/// Dwell timer for the phases that simply wait a moment.
#[derive(Debug, Resource)]
pub struct PhaseTimer(pub Timer);

impl Default for PhaseTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Once))
    }
}

impl PhaseTimer {
    pub fn set(&mut self, seconds: f32) {
        self.0 = Timer::from_seconds(seconds, TimerMode::Once);
    }
}

/// Marks everything belonging to a single game, so a restart can clear it out.
#[derive(Debug, Component)]
pub struct GameScoped;

/// One of the nine defenders.
#[derive(Debug, Component)]
pub struct Fielder {
    pub position: PlayerPosition,
    /// Where this fielder stands between pitches.
    pub home: Vec2,
    /// Where they are heading, while the ball is live.
    pub target: Option<Vec2>,
}

/// Run condition: a live game that is not paused.
fn playing(phase: Res<State<Phase>>, paused: Res<Paused>) -> bool {
    phase.get().is_live() && !paused.0
}

/// Run condition: anything that should keep drawing, including while paused.
fn on_the_field(phase: Res<State<Phase>>) -> bool {
    phase.get().is_live() || *phase.get() == Phase::InningBreak
}

/// Run condition: a live game, paused or not, for the pause toggle itself.
fn live(phase: Res<State<Phase>>) -> bool {
    phase.get().is_live()
}

/// Generic cleanup for tag components.
fn despawn_all<T: Component>(mut commands: Commands, entities: Query<Entity, With<T>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

pub struct BaseballPlugin;

impl Plugin for BaseballPlugin {
    fn build(&self, app: &mut App) {
        let fonts = load_fonts(app);
        app.insert_resource(fonts)
            .init_resource::<RandomSource>()
            .init_resource::<Diamond>()
            .init_resource::<Paused>()
            .init_resource::<BatterIntent>()
            .init_resource::<Banner>()
            .init_resource::<PhaseTimer>()
            .init_resource::<pitch::PitchPlan>()
            .init_resource::<pitch::LivePitch>()
            .init_resource::<ball::LiveBall>()
            .init_resource::<effects::ScreenShake>()
            .init_resource::<flow::PitchLoop>()
            .init_state::<Phase>()
            .add_message::<effects::Splash>()
            .insert_resource(ClearColor(theme::SKY))
            .add_systems(Startup, view::spawn_cameras)
            // --- title ---
            .add_systems(
                OnEnter(Phase::Title),
                (
                    despawn_all::<GameScoped>,
                    despawn_all::<screens::PauseUi>,
                    screens::spawn_title,
                ),
            )
            .add_systems(OnExit(Phase::Title), despawn_all::<screens::TitleUi>)
            .add_systems(Update, screens::title_input.run_if(in_state(Phase::Title)))
            // --- phase entries ---
            // Chained: the scene has to exist before anything reads from it, and
            // the at-bat view has to be showing before the pitcher winds up.
            .add_systems(
                OnEnter(Phase::Windup),
                (
                    scene::ensure_scene,
                    hud::ensure_hud,
                    flow::begin_windup,
                    view::show_at_bat,
                )
                    .chain(),
            )
            .add_systems(OnEnter(Phase::Pitch), flow::release_pitch)
            .add_systems(
                OnEnter(Phase::BallInPlay),
                (flow::begin_ball_in_play, view::show_field).chain(),
            )
            // Chained: the outcome has to be banked before the banner reads the
            // count it is going to print.
            .add_systems(
                OnEnter(Phase::Resolve),
                (flow::apply_outcome, screens::spawn_result_banner).chain(),
            )
            .add_systems(OnEnter(Phase::InningBreak), screens::spawn_inning_card)
            .add_systems(OnExit(Phase::InningBreak), despawn_all::<screens::InningUi>)
            .add_systems(OnEnter(Phase::GameOver), screens::spawn_game_over)
            .add_systems(OnExit(Phase::GameOver), despawn_all::<screens::GameOverUi>)
            .add_systems(Update, screens::game_over_input.run_if(in_state(Phase::GameOver)))
            // --- the pitch loop ---
            .add_systems(
                Update,
                (
                    flow::windup_input.run_if(in_state(Phase::Windup)),
                    flow::advance_pitch.run_if(in_state(Phase::Pitch)),
                    flow::advance_ball_in_play.run_if(in_state(Phase::BallInPlay)),
                    flow::advance_resolve.run_if(in_state(Phase::Resolve)),
                )
                    .run_if(playing),
            )
            .add_systems(Update, flow::advance_inning_break.run_if(in_state(Phase::InningBreak)))
            // Chained: the overlay must reconcile after the toggle, or pausing and
            // quitting to the title in one frame leaves the overlay behind.
            .add_systems(
                Update,
                (flow::toggle_pause, screens::sync_pause_overlay).chain().run_if(live),
            )
            // The banner has to go when the *result* stops being shown, not when the
            // next windup starts: between those two is the inning card, and leaving
            // it up printed the last play over the top of "BOTTOM 1ST".
            .add_systems(OnExit(Phase::Resolve), despawn_all::<screens::ResultBanner>)
            // --- drawing, which keeps running while paused ---
            .add_systems(
                Update,
                (
                    scene::draw_ball,
                    scene::draw_fielders,
                    scene::draw_runners,
                    scene::draw_at_bat,
                    hud::update_score_bug,
                    hud::update_pitch_panel,
                )
                    .run_if(on_the_field),
            )
            .add_systems(
                Update,
                (
                    effects::spawn_splashes,
                    effects::drive_particles,
                    effects::drive_popups,
                    effects::apply_screen_shake,
                    screens::pulse_prompt,
                ),
            );
    }
}

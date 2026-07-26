//! Full-screen furniture: the title, the pause overlay, the card between innings,
//! and the box score at the end.
//!
//! All of it is `bevy_ui`, pinned to the HUD camera so it sits on top of whichever
//! view happens to be showing.

use baseball_game_rules::{GameOutcome, GameWinner, InningHalf, LineScore};
use bevy::prelude::*;

use super::{Banner, Diamond, Fonts, GameScoped, PLAYER_HALF, Paused, Phase, PhaseTimer, theme};

#[derive(Debug, Component)]
pub struct TitleUi;

#[derive(Debug, Component)]
pub struct PauseUi;

#[derive(Debug, Component)]
pub struct InningUi;

#[derive(Debug, Component)]
pub struct GameOverUi;

/// Text that fades in and out to draw the eye.
#[derive(Debug, Component)]
pub struct Blinker;

fn text(content: impl Into<String>, font: Handle<Font>, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(content),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(colour),
    )
}

/// A full-screen column, dimming whatever is behind it. No camera is named: the
/// HUD camera is marked `IsDefaultUiCamera`, so anything spawned here finds it
/// whenever it happens to exist.
fn overlay(wash: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.05, wash)),
    )
}

// ------------------------------------------------------------------ title

pub fn spawn_title(mut commands: Commands, fonts: Res<Fonts>) {
    commands.spawn((
        overlay(0.82),
        TitleUi,
        children![
            text("BASEBALL", fonts.bold.clone(), 76.0, theme::TEXT),
            text(
                "nine innings, one set of hands",
                fonts.medium.clone(),
                18.0,
                theme::TEXT_DIM
            ),
            (
                Node {
                    margin: UiRect::top(Val::Px(28.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                },
                children![
                    text(
                        "YOU BAT IN THE BOTTOM HALF",
                        fonts.medium.clone(),
                        15.0,
                        theme::BUG_ACCENT
                    ),
                    text("BATTING", fonts.bold.clone(), 14.0, theme::TEXT),
                    text("  SPACE      swing", fonts.medium.clone(), 13.0, theme::TEXT_DIM),
                    text(
                        "  UP / DOWN  lift / level swing",
                        fonts.medium.clone(),
                        13.0,
                        theme::TEXT_DIM
                    ),
                    text("PITCHING", fonts.bold.clone(), 14.0, theme::TEXT),
                    text(
                        "  1 2 3 4    fastball slider curve change",
                        fonts.medium.clone(),
                        13.0,
                        theme::TEXT_DIM
                    ),
                    text("  ARROWS     aim", fonts.medium.clone(), 13.0, theme::TEXT_DIM),
                    text("  SPACE      throw", fonts.medium.clone(), 13.0, theme::TEXT_DIM),
                    text("  ESC        pause", fonts.medium.clone(), 13.0, theme::TEXT_DIM),
                ],
            ),
            (
                text("PRESS ENTER TO PLAY", fonts.bold.clone(), 20.0, theme::BUG_ACCENT),
                Node {
                    margin: UiRect::top(Val::Px(26.0)),
                    ..default()
                },
                Blinker,
            ),
        ],
    ));
}

pub fn title_input(keys: Res<ButtonInput<KeyCode>>, mut diamond: ResMut<Diamond>, mut next: ResMut<NextState<Phase>>) {
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::Space) {
        *diamond = Diamond::default();
        next.set(Phase::Windup);
    }
}

// ------------------------------------------------------------------ pause

/// Reconciles the pause overlay against the flag, rather than hooking a state
/// transition. Pause is a resource precisely so that resuming does not re-enter
/// the phase, which means there is no transition to hook.
pub fn sync_pause_overlay(
    mut commands: Commands,
    paused: Res<Paused>,
    fonts: Res<Fonts>,
    existing: Query<Entity, With<PauseUi>>,
) {
    match (paused.0, existing.single()) {
        (true, Err(_)) => {
            commands.spawn((
                overlay(0.72),
                PauseUi,
                children![
                    text("PAUSED", fonts.bold.clone(), 54.0, theme::TEXT),
                    text("ESC to resume", fonts.medium.clone(), 16.0, theme::TEXT_DIM),
                    text("T for the title screen", fonts.medium.clone(), 16.0, theme::TEXT_DIM),
                ],
            ));
        }
        (false, Ok(entity)) => {
            commands.entity(entity).despawn();
        }
        _ => {}
    }
}

// ------------------------------------------------------------------ innings

pub fn spawn_inning_card(
    mut commands: Commands,
    fonts: Res<Fonts>,
    diamond: Res<Diamond>,
    mut timer: ResMut<PhaseTimer>,
) {
    timer.set(super::flow::INNING_DWELL);

    let (headline, detail) = match diamond.game() {
        Some(game) => {
            let half = match game.current_half_inning().half() {
                InningHalf::Top => "TOP",
                InningHalf::Bottom => "BOTTOM",
            };
            let inning = ordinal(game.current_inning().as_number());
            let side = if game.current_half_inning().half() == PLAYER_HALF {
                "YOU'RE UP"
            } else {
                "TAKE THE MOUND"
            };
            (format!("{half} {inning}"), side.to_string())
        }
        None => ("FINAL".to_string(), String::new()),
    };

    commands.spawn((
        overlay(0.55),
        InningUi,
        children![
            text(headline, fonts.bold.clone(), 52.0, theme::TEXT),
            text(detail, fonts.medium.clone(), 20.0, theme::BUG_ACCENT),
        ],
    ));
}

fn ordinal(number: u8) -> String {
    let suffix = match number {
        1 => "ST",
        2 => "ND",
        3 => "RD",
        _ => "TH",
    };
    format!("{number}{suffix}")
}

// ------------------------------------------------------------------ game over

pub fn spawn_game_over(mut commands: Commands, fonts: Res<Fonts>, diamond: Res<Diamond>) {
    let summary = match &diamond.outcome {
        GameOutcome::Complete(summary) => summary,
        // Reaching game over without a completed game should not happen, but a
        // dead-end screen is better than a panic.
        GameOutcome::InProgress(_) => {
            commands.spawn((
                overlay(0.85),
                GameOverUi,
                children![text("GAME CALLED", fonts.bold.clone(), 52.0, theme::TEXT)],
            ));
            return;
        }
    };

    let score = summary.final_score();
    let verdict = match summary.winner() {
        GameWinner::Home if PLAYER_HALF == InningHalf::Bottom => "YOU WIN",
        GameWinner::Away if PLAYER_HALF == InningHalf::Top => "YOU WIN",
        _ => "YOU LOSE",
    };

    commands.spawn((
        overlay(0.88),
        GameOverUi,
        children![
            text(verdict, fonts.bold.clone(), 62.0, theme::BUG_ACCENT),
            text(
                format!("AWAY {}    HOME {}", score.away(), score.home()),
                fonts.bold.clone(),
                26.0,
                theme::TEXT
            ),
            line_score_grid(summary.line_score(), &fonts, score.away(), score.home()),
            (
                text(
                    "R to play again    T for the title",
                    fonts.medium.clone(),
                    16.0,
                    theme::TEXT_DIM
                ),
                Node {
                    margin: UiRect::top(Val::Px(22.0)),
                    ..default()
                },
                Blinker,
            ),
        ],
    ));
}

/// The line score, laid out as a grid of inning columns like a real box score.
fn line_score_grid(line: &LineScore, fonts: &Fonts, away_total: u8, home_total: u8) -> impl Bundle {
    let columns = line.columns();

    // Header row, then one row per team: name, each inning, runs, hits.
    let rows: Vec<(String, Vec<String>, String, String)> = vec![
        (
            String::new(),
            (1..=columns).map(|inning| inning.to_string()).collect(),
            "R".to_string(),
            "H".to_string(),
        ),
        (
            "AWAY".to_string(),
            cells(line.away_innings(), columns),
            away_total.to_string(),
            line.away_hits().to_string(),
        ),
        (
            "HOME".to_string(),
            cells(line.home_innings(), columns),
            home_total.to_string(),
            line.home_hits().to_string(),
        ),
    ];

    let medium = fonts.medium.clone();
    let bold = fonts.bold.clone();

    (
        Node {
            margin: UiRect::top(Val::Px(18.0)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            ..default()
        },
        Children::spawn(bevy::ecs::spawn::SpawnIter(rows.into_iter().enumerate().map(
            move |(index, (name, per_inning, runs, hits))| {
                let is_header = index == 0;
                let colour = if is_header { theme::TEXT_DIM } else { theme::TEXT };
                let font = if is_header { medium.clone() } else { bold.clone() };
                let mut cells: Vec<String> = Vec::with_capacity(per_inning.len() + 3);
                cells.push(name);
                cells.extend(per_inning);
                cells.push(runs);
                cells.push(hits);

                (
                    Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    Children::spawn(bevy::ecs::spawn::SpawnIter(cells.into_iter().enumerate().map({
                        let font = font.clone();
                        move |(column, value)| {
                            (
                                Node {
                                    width: Val::Px(if column == 0 { 54.0 } else { 26.0 }),
                                    justify_content: if column == 0 {
                                        JustifyContent::FlexStart
                                    } else {
                                        JustifyContent::Center
                                    },
                                    ..default()
                                },
                                children![(
                                    Text::new(value),
                                    TextFont {
                                        font: font.clone().into(),
                                        font_size: FontSize::Px(14.0),
                                        ..default()
                                    },
                                    TextColor(colour),
                                )],
                            )
                        }
                    }))),
                )
            },
        ))),
    )
}

/// One cell per inning, with a dash where a half was never played — which is what
/// a real scoreboard shows when the home team does not need to bat.
fn cells(runs: &[u8], columns: usize) -> Vec<String> {
    (0..columns)
        .map(|inning| match runs.get(inning) {
            Some(value) => value.to_string(),
            None => "-".to_string(),
        })
        .collect()
}

pub fn game_over_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut diamond: ResMut<Diamond>,
    mut next: ResMut<NextState<Phase>>,
) {
    if keys.just_pressed(KeyCode::KeyR) {
        *diamond = Diamond::default();
        next.set(Phase::Windup);
    } else if keys.just_pressed(KeyCode::KeyT) {
        next.set(Phase::Title);
    }
}

// ------------------------------------------------------------------ banner

/// The big word in the middle of the screen after a pitch.
#[derive(Debug, Component)]
pub struct ResultBanner;

pub fn spawn_result_banner(mut commands: Commands, fonts: Res<Fonts>, banner: Res<Banner>) {
    if banner.headline.is_empty() {
        return;
    }

    let colour = if banner.good_for_batter {
        theme::BANNER_GOOD
    } else {
        theme::BANNER_BAD
    };

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            top: Val::Percent(24.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(4.0),
            ..default()
        },
        ResultBanner,
        GameScoped,
        children![
            text(banner.headline.clone(), fonts.bold.clone(), 46.0, colour),
            text(banner.detail.clone(), fonts.medium.clone(), 18.0, theme::TEXT_DIM),
        ],
    ));
}

/// Gentle pulse on anything marked as a blinker.
pub fn pulse_prompt(time: Res<Time>, mut blinkers: Query<&mut TextColor, With<Blinker>>) {
    let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 3.0).sin().abs();
    for mut colour in blinkers.iter_mut() {
        colour.0 = colour.0.with_alpha(pulse);
    }
}

#[cfg(test)]
mod tests {
    use baseball_game_rules::{GameScore, GameSummary, InningNumber};
    use bevy::ecs::system::RunSystemOnce;

    use super::*;

    /// A bare world with just the resources the screen systems ask for.
    ///
    /// Deliberately has no camera in it. An earlier version of these systems
    /// looked the HUD camera up and returned early if it was missing, which meant
    /// the title screen silently drew nothing: `bevy_state` runs the first state
    /// transition before `Startup`, so `OnEnter(Title)` fires before the camera
    /// exists. Every test here would have caught that.
    fn harness() -> World {
        let mut world = World::new();
        world.insert_resource(Diamond::default());
        world.insert_resource(Paused::default());
        world.insert_resource(Banner::default());
        world.insert_resource(PhaseTimer::default());
        world.insert_resource(Fonts {
            bold: Handle::default(),
            medium: Handle::default(),
        });
        world
    }

    fn count<T: Component>(world: &mut World) -> usize {
        let mut query = world.query::<&T>();
        query.iter(world).count()
    }

    fn finished_game(away: u8, home: u8) -> GameOutcome {
        let score = GameScore::new().add_away_runs(away).add_home_runs(home);
        let winner = if home > away {
            GameWinner::Home
        } else {
            GameWinner::Away
        };
        GameOutcome::Complete(GameSummary::new(
            score,
            InningNumber::Ninth,
            winner,
            LineScore::default(),
        ))
    }

    #[test]
    fn the_title_screen_spawns_with_no_camera_in_the_world() {
        let mut world = harness();
        world.run_system_once(spawn_title).expect("system should run");
        assert_eq!(count::<TitleUi>(&mut world), 1, "the title screen drew nothing");
    }

    #[test]
    fn the_inning_card_spawns_and_sets_its_own_dwell() {
        let mut world = harness();
        world.run_system_once(spawn_inning_card).expect("system should run");

        assert_eq!(count::<InningUi>(&mut world), 1);
        let timer = world.resource::<PhaseTimer>();
        assert!(
            (timer.0.duration().as_secs_f32() - super::super::flow::INNING_DWELL).abs() < 1e-4,
            "the card has to set the timer it is displayed for"
        );
    }

    #[test]
    fn the_game_over_screen_spawns_for_a_finished_game() {
        let mut world = harness();
        world.insert_resource(Diamond {
            outcome: finished_game(2, 5),
        });
        world.run_system_once(spawn_game_over).expect("system should run");
        assert_eq!(count::<GameOverUi>(&mut world), 1);
    }

    #[test]
    fn the_game_over_screen_still_appears_if_the_game_never_completed() {
        // Should not be reachable, but a dead-end screen beats a blank one.
        let mut world = harness();
        world.run_system_once(spawn_game_over).expect("system should run");
        assert_eq!(count::<GameOverUi>(&mut world), 1);
    }

    #[test]
    fn the_result_banner_says_nothing_until_there_is_something_to_say() {
        let mut world = harness();
        world.run_system_once(spawn_result_banner).expect("system should run");
        assert_eq!(count::<ResultBanner>(&mut world), 0, "an empty banner should not spawn");

        world.resource_mut::<Banner>().headline = "STRIKE".to_string();
        world.run_system_once(spawn_result_banner).expect("system should run");
        assert_eq!(count::<ResultBanner>(&mut world), 1);
    }

    #[test]
    fn the_pause_overlay_follows_the_flag_in_both_directions() {
        let mut world = harness();

        // Not paused: nothing to show.
        world.run_system_once(sync_pause_overlay).expect("system should run");
        assert_eq!(count::<PauseUi>(&mut world), 0);

        world.resource_mut::<Paused>().0 = true;
        world.run_system_once(sync_pause_overlay).expect("system should run");
        assert_eq!(count::<PauseUi>(&mut world), 1);

        // Running again while still paused must not stack a second overlay.
        world.run_system_once(sync_pause_overlay).expect("system should run");
        assert_eq!(count::<PauseUi>(&mut world), 1, "the overlay was spawned twice");

        world.resource_mut::<Paused>().0 = false;
        world.run_system_once(sync_pause_overlay).expect("system should run");
        assert_eq!(count::<PauseUi>(&mut world), 0, "the overlay outlived the pause");
    }

    #[test]
    fn the_winner_is_read_from_the_half_the_player_bats_in() {
        // The verdict is the one thing on the screen that has to know which side
        // the human is on; getting it backwards would congratulate the loser.
        let mut world = harness();
        world.insert_resource(Diamond {
            outcome: finished_game(1, 9),
        });
        world.run_system_once(spawn_game_over).expect("system should run");

        let mut query = world.query::<&Text>();
        let shown: Vec<String> = query.iter(&world).map(|text| text.0.clone()).collect();
        assert!(
            shown.iter().any(|line| line == "YOU WIN"),
            "the home team won and the player is the home team, got {shown:?}"
        );
    }

    #[test]
    fn ordinals_read_correctly() {
        assert_eq!(ordinal(1), "1ST");
        assert_eq!(ordinal(2), "2ND");
        assert_eq!(ordinal(3), "3RD");
        assert_eq!(ordinal(4), "4TH");
        assert_eq!(ordinal(9), "9TH");
        assert_eq!(ordinal(11), "11TH");
    }

    #[test]
    fn an_unplayed_half_inning_shows_a_dash() {
        // The home team does not bat in the ninth if they are already ahead, and
        // the scoreboard has to say so rather than printing a zero.
        let filled = cells(&[0, 1, 0], 4);
        assert_eq!(filled, vec!["0", "1", "0", "-"]);
    }

    #[test]
    fn the_line_score_has_a_column_for_every_inning_played() {
        let mut line = LineScore::default();
        assert_eq!(line.columns(), 0);
        assert_eq!(cells(line.away_innings(), 0).len(), 0);

        // Extra innings widen the grid rather than overflowing it.
        line = LineScore::default();
        let padded = cells(line.home_innings(), 12);
        assert_eq!(padded.len(), 12);
        assert!(padded.iter().all(|cell| cell == "-"));
    }
}

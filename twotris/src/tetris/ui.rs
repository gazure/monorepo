//! The score bug, the per-board side panels, and the full-screen overlays.
//!
//! The score bug and panels are drawn in world space (`Text2d` + sprites) so
//! they sit in the same coordinate system as the boards and stay aligned with
//! them at any window size. Only the full-screen overlays use `bevy_ui`.

use bevy::{prelude::*, sprite::Anchor};

use super::{
    BoardSlot, Focus, Fonts, GameState, Hold, Paused, RandomSource, Scoreboard,
    game::RestartRequest,
    piece::{Bag, KINDS, PieceKind},
    render::{self, BOARD_CENTER_Y, BOARD_H, HUD_CENTER_Y, HUD_H, PANEL_W, TOTAL_W},
    theme,
};

const PANEL_H: f32 = 328.0;
/// Panel top edge is flush with the top of the playfield.
const PANEL_CENTER_Y: f32 = BOARD_CENTER_Y + BOARD_H / 2.0 - PANEL_H / 2.0;

const BOARD_NAMES: [&str; 2] = ["LEFT", "RIGHT"];

/// A value in the score bug that gets rewritten every frame.
#[derive(Debug, Clone, Copy, Component)]
pub enum HudField {
    Score,
    Level,
    Lines,
    Combo,
    BoardName(usize),
}

/// Underline beneath a board's name, lit for the focused board.
#[derive(Debug, Component)]
pub struct FocusUnderline(pub usize);

/// A preview thumbnail: either a queue position or the hold slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum PreviewSlot {
    Next { board: usize, index: usize },
    Hold { board: usize },
}

/// One of the four blocks making up a preview thumbnail.
#[derive(Debug, Component)]
pub struct PreviewCell {
    pub slot: PreviewSlot,
    pub index: usize,
}

#[derive(Debug, Component)]
pub struct TitleUi;

#[derive(Debug, Component)]
pub struct PauseUi;

#[derive(Debug, Component)]
pub struct GameOverUi;

/// Pulses the "press enter" prompt on the title screen.
#[derive(Debug, Component)]
pub struct Blinker;

fn text2d(
    content: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    color: Color,
    pos: Vec3,
    anchor: Anchor,
) -> impl Bundle {
    (
        Text2d::new(content.into()),
        TextFont::from_font_size(size).with_font(font.clone()),
        TextColor(color),
        anchor,
        Transform::from_translation(pos),
    )
}

/// Groups digits so a six-figure score stays readable at a glance.
fn commas(value: u32) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Cell size used for a preview thumbnail. The head of the queue and the hold
/// slot are drawn larger than the lookahead behind them.
fn preview_scale(slot: PreviewSlot) -> f32 {
    match slot {
        PreviewSlot::Next { index: 0, .. } | PreviewSlot::Hold { .. } => 14.0,
        PreviewSlot::Next { .. } => 10.0,
    }
}

/// Panel-local centre of a preview thumbnail.
fn preview_center(slot: PreviewSlot) -> Vec2 {
    match slot {
        PreviewSlot::Next { index, .. } => Vec2::new(0.0, 78.0 - index as f32 * 46.0),
        PreviewSlot::Hold { .. } => Vec2::new(0.0, -108.0),
    }
}

// --- the score bug --------------------------------------------------------

pub fn spawn_hud(commands: &mut Commands, fonts: &Fonts) -> Entity {
    let half = TOTAL_W / 2.0;

    commands
        .spawn((
            Sprite::from_color(theme::PANEL, Vec2::new(TOTAL_W, HUD_H)),
            Transform::from_xyz(0.0, HUD_CENTER_Y, 0.0),
            Visibility::default(),
        ))
        .with_children(|hud| {
            // Accent flash down the left edge, the way a broadcast bug is keyed.
            hud.spawn((
                Sprite::from_color(theme::ACCENT, Vec2::new(5.0, HUD_H - 24.0)),
                Transform::from_xyz(-half + 14.0, 0.0, 1.0),
            ));
            hud.spawn((
                Sprite::from_color(theme::PANEL_EDGE, Vec2::new(TOTAL_W, 1.0)),
                Transform::from_xyz(0.0, -HUD_H / 2.0, 1.0),
            ));

            let left = -half + 28.0;
            hud.spawn(text2d(
                "SCORE",
                &fonts.medium,
                13.0,
                theme::TEXT_DIM,
                Vec3::new(left, 20.0, 1.0),
                Anchor::CENTER_LEFT,
            ));
            hud.spawn((
                HudField::Score,
                text2d(
                    "0",
                    &fonts.bold,
                    40.0,
                    theme::TEXT,
                    Vec3::new(left, -11.0, 1.0),
                    Anchor::CENTER_LEFT,
                ),
            ));

            // Secondary stats are right-aligned against the far edge so the bug
            // spans its full width instead of bunching up on the left.
            let right = half - 26.0;
            for (x, caption, field) in [
                (right - 136.0, "LEVEL", HudField::Level),
                (right, "LINES", HudField::Lines),
            ] {
                hud.spawn(text2d(
                    caption,
                    &fonts.medium,
                    13.0,
                    theme::TEXT_DIM,
                    Vec3::new(x, 20.0, 1.0),
                    Anchor::CENTER_RIGHT,
                ));
                hud.spawn((
                    field,
                    text2d(
                        "0",
                        &fonts.bold,
                        26.0,
                        theme::TEXT,
                        Vec3::new(x, -11.0, 1.0),
                        Anchor::CENTER_RIGHT,
                    ),
                ));
            }

            // Combo sits dead centre, in the gap between score and stats.
            hud.spawn((
                HudField::Combo,
                text2d(
                    "",
                    &fonts.bold,
                    22.0,
                    theme::ACCENT_WARM,
                    Vec3::new(0.0, -2.0, 1.0),
                    Anchor::CENTER,
                ),
            ));
        })
        .id()
}

pub fn spawn_panel(commands: &mut Commands, slot: usize, fonts: &Fonts) -> Entity {
    let half_w = PANEL_W / 2.0;

    commands
        .spawn((
            Sprite::from_color(theme::PANEL, Vec2::new(PANEL_W, PANEL_H)),
            Transform::from_xyz(render::panel_x(slot), PANEL_CENTER_Y, 0.0),
            Visibility::default(),
        ))
        .with_children(|panel| {
            panel.spawn((
                Sprite::from_color(theme::PANEL_EDGE, Vec2::new(PANEL_W + 3.0, PANEL_H + 3.0)),
                Transform::from_xyz(0.0, 0.0, -1.0),
            ));

            panel.spawn((
                HudField::BoardName(slot),
                text2d(
                    BOARD_NAMES[slot],
                    &fonts.bold,
                    17.0,
                    theme::TEXT,
                    Vec3::new(0.0, PANEL_H / 2.0 - 22.0, 1.0),
                    Anchor::CENTER,
                ),
            ));
            panel.spawn((
                FocusUnderline(slot),
                Sprite::from_color(theme::PANEL_EDGE, Vec2::new(46.0, 3.0)),
                Transform::from_xyz(0.0, PANEL_H / 2.0 - 38.0, 1.0),
            ));

            panel.spawn(text2d(
                "NEXT",
                &fonts.medium,
                12.0,
                theme::TEXT_DIM,
                Vec3::new(-half_w + 12.0, PANEL_H / 2.0 - 58.0, 1.0),
                Anchor::CENTER_LEFT,
            ));
            panel.spawn(text2d(
                "HOLD",
                &fonts.medium,
                12.0,
                theme::TEXT_DIM,
                Vec3::new(-half_w + 12.0, -60.0, 1.0),
                Anchor::CENTER_LEFT,
            ));
            panel.spawn((
                Sprite::from_color(theme::PANEL_EDGE, Vec2::new(PANEL_W - 24.0, 1.0)),
                Transform::from_xyz(0.0, -76.0, 1.0),
            ));

            let slots = [
                PreviewSlot::Next { board: slot, index: 0 },
                PreviewSlot::Next { board: slot, index: 1 },
                PreviewSlot::Next { board: slot, index: 2 },
                PreviewSlot::Hold { board: slot },
            ];
            for preview in slots {
                for index in 0..4 {
                    panel.spawn((
                        PreviewCell { slot: preview, index },
                        Sprite::from_color(Color::NONE, Vec2::splat(preview_scale(preview) - 2.0)),
                        Transform::from_xyz(0.0, 0.0, 1.0),
                        Visibility::Hidden,
                    ));
                }
            }
        })
        .id()
}

pub fn update_hud(
    scoreboard: Res<Scoreboard>,
    boards: Query<(&BoardSlot, Option<&Focus>)>,
    mut fields: Query<(&HudField, &mut Text2d, &mut TextColor)>,
    mut underlines: Query<(&FocusUnderline, &mut Sprite)>,
) {
    let focused = boards.iter().find(|(_, focus)| focus.is_some()).map(|(slot, _)| slot.0);

    for (field, mut text, mut color) in &mut fields {
        match field {
            HudField::Score => text.0 = commas(scoreboard.score),
            HudField::Level => text.0 = scoreboard.level.to_string(),
            HudField::Lines => text.0 = scoreboard.lines.to_string(),
            HudField::Combo => {
                text.0 = if scoreboard.combo >= 1 {
                    format!("COMBO x{}", scoreboard.combo + 1)
                } else if scoreboard.back_to_back {
                    "B2B READY".to_owned()
                } else {
                    String::new()
                };
            }
            HudField::BoardName(slot) => {
                color.0 = if focused == Some(*slot) {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                };
            }
        }
    }

    for (underline, mut sprite) in &mut underlines {
        sprite.color = if focused == Some(underline.0) {
            theme::ACCENT
        } else {
            theme::PANEL_EDGE
        };
    }
}

pub fn update_previews(
    mut random: ResMut<RandomSource>,
    mut boards: Query<(&BoardSlot, &mut Bag, &Hold)>,
    mut cells: Query<(&PreviewCell, &mut Transform, &mut Sprite, &mut Visibility)>,
) {
    let mut queued: [Vec<PieceKind>; 2] = [Vec::new(), Vec::new()];
    let mut held: [Option<PieceKind>; 2] = [None; 2];
    for (slot, mut bag, hold) in &mut boards {
        if let Some(entry) = queued.get_mut(slot.0) {
            *entry = bag.peek(random.as_mut(), 3);
            held[slot.0] = hold.kind;
        }
    }

    for (cell, mut transform, mut sprite, mut visibility) in &mut cells {
        let kind = match cell.slot {
            PreviewSlot::Next { board, index } => queued.get(board).and_then(|q| q.get(index)).copied(),
            PreviewSlot::Hold { board } => held.get(board).copied().flatten(),
        };
        let Some(kind) = kind else {
            *visibility = Visibility::Hidden;
            continue;
        };

        let size = preview_scale(cell.slot);
        let center = preview_center(cell.slot);
        let (cells, width, height) = kind.preview_cells();
        let (bx, by) = cells[cell.index];

        *visibility = Visibility::Inherited;
        sprite.color = theme::piece_color(kind);
        sprite.custom_size = Some(Vec2::splat(size - 2.0));
        transform.translation = Vec3::new(
            center.x + (bx as f32 - (width as f32 - 1.0) / 2.0) * size,
            center.y - (by as f32 - (height as f32 - 1.0) / 2.0) * size,
            1.0,
        );
    }
}

// --- full-screen overlays -------------------------------------------------

fn overlay_root(tag: impl Component, wash: Color) -> impl Bundle {
    (
        tag,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(18.0),
            ..default()
        },
        BackgroundColor(wash),
    )
}

fn ui_text(content: impl Into<String>, font: &Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(content.into()),
        TextFont::from_font_size(size).with_font(font.clone()),
        TextColor(color),
    )
}

const CONTROLS: &str = "\
MOVE          LEFT   RIGHT
SOFT DROP     DOWN
HARD DROP     SPACE
ROTATE        Z   X   UP
HOLD          C
SWAP BOARD    F   TAB
PAUSE         ESC";

pub fn spawn_title(mut commands: Commands, fonts: Res<Fonts>) {
    commands
        .spawn(overlay_root(TitleUi, Color::NONE))
        .with_children(|root| {
            // "TWOTRIS" is seven letters and there are seven tetrominoes, so the
            // wordmark doubles as the piece legend.
            root.spawn((
                Text::default(),
                TextFont::from_font_size(96.0).with_font(fonts.bold.clone()),
            ))
            .with_children(|word| {
                for (letter, kind) in "TWOTRIS".chars().zip(KINDS) {
                    word.spawn((
                        TextSpan::new(letter.to_string()),
                        TextFont::from_font_size(96.0).with_font(fonts.bold.clone()),
                        TextColor(theme::piece_color(kind)),
                    ));
                }
            });

            root.spawn(ui_text(
                "tetris with two grids, lmao",
                &fonts.medium,
                20.0,
                theme::TEXT_DIM,
            ));

            root.spawn((
                Node {
                    margin: UiRect::top(Val::Px(14.0)),
                    padding: UiRect::axes(Val::Px(26.0), Val::Px(18.0)),
                    border_radius: BorderRadius::all(Val::Px(6.0)),
                    ..default()
                },
                BackgroundColor(theme::with_alpha(theme::PANEL, 0.85)),
            ))
            .with_child(ui_text(CONTROLS, &fonts.medium, 15.0, theme::TEXT_DIM));

            root.spawn((
                Blinker,
                ui_text("PRESS ENTER TO PLAY", &fonts.bold, 24.0, theme::ACCENT),
            ));
        });
}

pub fn pulse_prompt(time: Res<Time>, mut blinkers: Query<&mut TextColor, With<Blinker>>) {
    let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 3.4).sin();
    for mut color in &mut blinkers {
        color.0 = theme::with_alpha(theme::ACCENT, pulse);
    }
}

pub fn title_input(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<GameState>>) {
    if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
    }
}

/// Spawns or removes the pause overlay to match the `Paused` resource. Written
/// as a reconcile rather than a transition hook so it cannot get out of step.
pub fn sync_pause_overlay(
    mut commands: Commands,
    paused: Res<Paused>,
    fonts: Res<Fonts>,
    existing: Query<Entity, With<PauseUi>>,
) {
    match (paused.0, existing.iter().next()) {
        (true, None) => {
            commands
                .spawn(overlay_root(PauseUi, theme::with_alpha(theme::BACKDROP, 0.82)))
                .with_children(|root| {
                    root.spawn(ui_text("PAUSED", &fonts.bold, 64.0, theme::TEXT));
                    root.spawn(ui_text(
                        "ESC resume    R restart    T title",
                        &fonts.medium,
                        18.0,
                        theme::TEXT_DIM,
                    ));
                });
        }
        (false, Some(_)) => {
            for entity in &existing {
                commands.entity(entity).despawn();
            }
        }
        _ => {}
    }
}

pub fn spawn_game_over(mut commands: Commands, fonts: Res<Fonts>, scoreboard: Res<Scoreboard>) {
    commands
        .spawn(overlay_root(GameOverUi, theme::with_alpha(theme::BACKDROP, 0.86)))
        .with_children(|root| {
            root.spawn(ui_text("GAME OVER", &fonts.bold, 72.0, theme::ACCENT_WARM));
            root.spawn(ui_text(
                format!("SCORE   {}", commas(scoreboard.score)),
                &fonts.bold,
                32.0,
                theme::TEXT,
            ));
            root.spawn(ui_text(
                format!(
                    "BEST {}     LINES {}     LEVEL {}",
                    commas(scoreboard.best),
                    scoreboard.lines,
                    scoreboard.level
                ),
                &fonts.medium,
                18.0,
                theme::TEXT_DIM,
            ));
            root.spawn((
                Blinker,
                ui_text("ENTER play again    T title", &fonts.bold, 22.0, theme::ACCENT),
            ));
        });
}

pub fn game_over_input(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut restart: ResMut<RestartRequest>,
) {
    if input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::KeyR) {
        // The arena is rebuilt by `OnEnter(Playing)`; clear any stale request so
        // it does not fire a second rebuild on the following frame.
        restart.0 = false;
        next_state.set(GameState::Playing);
    }
    if input.just_pressed(KeyCode::KeyT) || input.just_pressed(KeyCode::Escape) {
        next_state.set(GameState::Title);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn scores_are_grouped_into_thousands() {
        assert_eq!(commas(0), "0");
        assert_eq!(commas(999), "999");
        assert_eq!(commas(1_000), "1,000");
        assert_eq!(commas(12_345), "12,345");
        assert_eq!(commas(1_234_567), "1,234,567");
    }

    /// Panel-local top and bottom edge of a thumbnail, for the tallest shape
    /// that could land in it.
    fn preview_extent(slot: PreviewSlot) -> (f32, f32) {
        let tallest = KINDS
            .iter()
            .map(|kind| kind.preview_cells().2)
            .max()
            .expect("KINDS is not empty") as f32;
        let half = tallest / 2.0 * preview_scale(slot);
        let center = preview_center(slot).y;
        (center + half, center - half)
    }

    #[test]
    fn previews_do_not_overlap_each_other() {
        let board = 0;
        let slots = [
            PreviewSlot::Next { board, index: 0 },
            PreviewSlot::Next { board, index: 1 },
            PreviewSlot::Next { board, index: 2 },
            PreviewSlot::Hold { board },
        ];

        let mut floor = f32::INFINITY;
        for slot in slots {
            let (top, bottom) = preview_extent(slot);
            assert!(top < floor, "{slot:?} overlaps the thumbnail above it");
            floor = bottom;
        }
    }

    #[test]
    fn previews_stay_inside_the_panel() {
        let board = 1;
        for slot in [PreviewSlot::Next { board, index: 0 }, PreviewSlot::Hold { board }] {
            let (top, bottom) = preview_extent(slot);
            assert!(top <= PANEL_H / 2.0, "{slot:?} pokes out of the panel top");
            assert!(bottom >= -PANEL_H / 2.0, "{slot:?} pokes out of the panel bottom");
        }
    }

    #[test]
    fn panels_stay_inside_the_playfield_height() {
        const { assert!(PANEL_H <= BOARD_H, "panel is taller than the board beside it") }
    }
}

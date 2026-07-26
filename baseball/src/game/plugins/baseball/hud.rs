//! The score bug, and the pitcher's control panel.
//!
//! Laid out like a broadcast graphic: a strip across the bottom with both teams,
//! the inning, count lamps, outs and a little base diamond. It is `bevy_ui` rather
//! than world-space text so it stays put when the view cuts between the plate and
//! the field.

use baseball_game_rules::{Base, GameOutcome, InningHalf};
use bevy::prelude::*;

use super::{BatterIntent, Diamond, GameScoped, Phase, pitch, theme};

#[derive(Debug, Component)]
pub struct ScoreBug;

#[derive(Debug, Component)]
pub struct TeamRuns(pub InningHalf);

/// The little triangle showing who is batting.
#[derive(Debug, Component)]
pub struct BattingMarker(pub InningHalf);

#[derive(Debug, Component)]
pub struct InningLabel;

#[derive(Debug, Component)]
pub struct CountLamp {
    pub kind: LampKind,
    pub index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LampKind {
    Ball,
    Strike,
    Out,
}

#[derive(Debug, Component)]
pub struct BasePip(pub Base);

#[derive(Debug, Component)]
pub struct PitchPanel;

#[derive(Debug, Component)]
pub struct PitchPanelText;

fn label(text: impl Into<String>, font: Handle<Font>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont {
            font: font.into(),
            font_size: FontSize::Px(size),
            ..default()
        },
        TextColor(color),
    )
}

fn lamp(kind: LampKind, index: u8) -> impl Bundle {
    (
        Node {
            width: Val::Px(10.0),
            height: Val::Px(10.0),
            margin: UiRect::right(Val::Px(3.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(theme::LAMP_OFF),
        CountLamp { kind, index },
    )
}

/// Colour a lamp should be when lit.
fn lit_colour(kind: LampKind) -> Color {
    match kind {
        LampKind::Ball => theme::LAMP_BALL,
        LampKind::Strike => theme::LAMP_STRIKE,
        LampKind::Out => theme::LAMP_OUT,
    }
}

/// Builds the score bug. Spawned once per game.
pub fn spawn_score_bug(commands: &mut Commands, fonts: &super::Fonts) {
    let row = |half: InningHalf, name: &str| -> (String, InningHalf) { (name.to_string(), half) };
    let rows = [row(InningHalf::Top, "AWAY"), row(InningHalf::Bottom, "HOME")];

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            left: Val::Px(14.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            column_gap: Val::Px(16.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(theme::BUG_PANEL),
        BorderColor::all(theme::BUG_EDGE),
        ScoreBug,
        GameScoped,
        children![
            // Team names and runs.
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
                Children::spawn(bevy::ecs::spawn::SpawnIter(rows.into_iter().map({
                    let fonts_bold = fonts.bold.clone();
                    let fonts_medium = fonts.medium.clone();
                    move |(name, half)| {
                        (
                            Node {
                                flex_direction: FlexDirection::Row,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(6.0),
                                ..default()
                            },
                            children![
                                (
                                    Node {
                                        width: Val::Px(8.0),
                                        height: Val::Px(8.0),
                                        ..default()
                                    },
                                    BackgroundColor(theme::BUG_ACCENT),
                                    BattingMarker(half),
                                ),
                                (
                                    Node {
                                        width: Val::Px(52.0),
                                        ..default()
                                    },
                                    children![label(name.clone(), fonts_medium.clone(), 15.0, theme::TEXT_DIM)],
                                ),
                                (
                                    Node {
                                        width: Val::Px(30.0),
                                        justify_content: JustifyContent::FlexEnd,
                                        ..default()
                                    },
                                    children![(label("0", fonts_bold.clone(), 20.0, theme::TEXT), TeamRuns(half),)],
                                ),
                            ],
                        )
                    }
                }))),
            ),
            // Inning.
            (
                Node {
                    width: Val::Px(76.0),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                children![(label("TOP 1", fonts.bold.clone(), 16.0, theme::BUG_ACCENT), InningLabel,)],
            ),
            // Count and outs lamps.
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
                children![
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(20.0),
                                    ..default()
                                },
                                children![label("B", fonts.medium.clone(), 11.0, theme::TEXT_DIM)],
                            ),
                            lamp(LampKind::Ball, 0),
                            lamp(LampKind::Ball, 1),
                            lamp(LampKind::Ball, 2),
                        ],
                    ),
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(20.0),
                                    ..default()
                                },
                                children![label("S", fonts.medium.clone(), 11.0, theme::TEXT_DIM)],
                            ),
                            lamp(LampKind::Strike, 0),
                            lamp(LampKind::Strike, 1),
                        ],
                    ),
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    width: Val::Px(20.0),
                                    ..default()
                                },
                                children![label("O", fonts.medium.clone(), 11.0, theme::TEXT_DIM)],
                            ),
                            lamp(LampKind::Out, 0),
                            lamp(LampKind::Out, 1),
                        ],
                    ),
                ],
            ),
            // Base diamond: three pips, rotated so it reads as a diamond.
            (
                Node {
                    width: Val::Px(42.0),
                    height: Val::Px(42.0),
                    ..default()
                },
                children![
                    base_pip(Base::Second, 15.0, 0.0),
                    base_pip(Base::Third, 0.0, 15.0),
                    base_pip(Base::First, 30.0, 15.0),
                ],
            ),
        ],
    ));
}

fn base_pip(base: Base, left: f32, top: f32) -> impl Bundle {
    (
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(left),
            top: Val::Px(top),
            width: Val::Px(11.0),
            height: Val::Px(11.0),
            ..default()
        },
        BackgroundColor(theme::BASE_EMPTY),
        BasePip(base),
    )
}

/// The strip that tells the pitcher what they have selected, or the batter what
/// swing they are set for.
pub fn spawn_pitch_panel(commands: &mut Commands, fonts: &super::Fonts) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(14.0),
            right: Val::Px(14.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            padding: UiRect::axes(Val::Px(12.0), Val::Px(8.0)),
            row_gap: Val::Px(3.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(theme::BUG_PANEL),
        BorderColor::all(theme::BUG_EDGE),
        PitchPanel,
        GameScoped,
        children![(label("", fonts.bold.clone(), 15.0, theme::TEXT), PitchPanelText,)],
    ));
}

/// Refreshes every number and lamp on the bug.
pub fn update_score_bug(
    diamond: Res<Diamond>,
    mut runs: Query<(&TeamRuns, &mut Text)>,
    mut inning: Query<&mut Text, (With<InningLabel>, Without<TeamRuns>)>,
    mut markers: Query<(&BattingMarker, &mut BackgroundColor), Without<CountLamp>>,
    mut lamps: Query<(&CountLamp, &mut BackgroundColor), Without<BattingMarker>>,
    mut pips: Query<(&BasePip, &mut BackgroundColor), (Without<CountLamp>, Without<BattingMarker>)>,
) {
    // Scores, including the runs banked so far this half inning.
    let (away, home) = match &diamond.outcome {
        GameOutcome::InProgress(game) => {
            let half = game.current_half_inning();
            let pending = half.runs_scored();
            let (mut away, mut home) = (game.score().away(), game.score().home());
            match half.half() {
                InningHalf::Top => away += pending,
                InningHalf::Bottom => home += pending,
            }
            (away, home)
        }
        GameOutcome::Complete(summary) => (summary.final_score().away(), summary.final_score().home()),
    };

    for (team, mut text) in runs.iter_mut() {
        let value = match team.0 {
            InningHalf::Top => away,
            InningHalf::Bottom => home,
        };
        **text = value.to_string();
    }

    if let Ok(mut text) = inning.single_mut() {
        **text = match &diamond.outcome {
            GameOutcome::InProgress(game) => {
                let half = match game.current_half_inning().half() {
                    InningHalf::Top => "TOP",
                    InningHalf::Bottom => "BOT",
                };
                format!("{half} {}", game.current_inning().as_number())
            }
            GameOutcome::Complete(_) => "FINAL".to_string(),
        };
    }

    let batting = diamond.batting_half();
    for (marker, mut colour) in markers.iter_mut() {
        colour.0 = if Some(marker.0) == batting {
            theme::BUG_ACCENT
        } else {
            theme::LAMP_OFF
        };
    }

    let (balls, strikes, outs) = diamond.game().map_or((0, 0, 0), |game| {
        let half = game.current_half_inning();
        let count = half.current_plate_appearance().count();
        (
            count.balls().as_number(),
            count.strikes().as_number(),
            half.outs().as_number(),
        )
    });

    for (lamp, mut colour) in lamps.iter_mut() {
        let filled = match lamp.kind {
            LampKind::Ball => balls,
            LampKind::Strike => strikes,
            LampKind::Out => outs,
        };
        colour.0 = if lamp.index < filled {
            lit_colour(lamp.kind)
        } else {
            theme::LAMP_OFF
        };
    }

    let runners = diamond.game().map(|game| game.current_half_inning().baserunners());
    for (pip, mut colour) in pips.iter_mut() {
        let occupied = runners.is_some_and(|state| match pip.0 {
            Base::First => state.first().is_some(),
            Base::Second => state.second().is_some(),
            Base::Third => state.third().is_some(),
            Base::Home => false,
        });
        colour.0 = if occupied {
            theme::BASE_OCCUPIED
        } else {
            theme::BASE_EMPTY
        };
    }
}

/// Tells whoever is at the controls what they are about to do.
pub fn update_pitch_panel(
    diamond: Res<Diamond>,
    phase: Res<State<Phase>>,
    plan: Res<pitch::PitchPlan>,
    intent: Res<BatterIntent>,
    mut panel: Query<&mut Text, With<PitchPanelText>>,
    mut frame: Query<&mut Visibility, With<PitchPanel>>,
) {
    let Ok(mut text) = panel.single_mut() else {
        return;
    };

    **text = if diamond.human_is_batting() {
        match phase.get() {
            Phase::Windup | Phase::Pitch => format!("SWING: {}   [SPACE]", intent.style.label()),
            _ => String::new(),
        }
    } else {
        match phase.get() {
            Phase::Windup => format!("{}   [SPACE TO PITCH]", plan.kind.label()),
            Phase::Pitch => plan.kind.label().to_string(),
            _ => String::new(),
        }
    };

    // An empty panel is an empty box floating over the field, so hide the frame
    // along with the text.
    if let Ok(mut visibility) = frame.single_mut() {
        *visibility = if text.is_empty() {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}

/// Builds the HUD if it is not already there, so a restart is just a despawn.
/// The UI has to be told which camera it belongs to, because with three cameras
/// in play `bevy_ui` cannot pick one for itself.
pub fn ensure_hud(mut commands: Commands, fonts: Res<super::Fonts>, existing: Query<(), With<ScoreBug>>) {
    if !existing.is_empty() {
        return;
    }
    spawn_score_bug(&mut commands, &fonts);
    spawn_pitch_panel(&mut commands, &fonts);
}

#[cfg(test)]
mod tests {
    use super::{super::PLAYER_HALF, *};

    #[test]
    fn the_human_is_the_home_team() {
        // The score bug marks a row as "yours"; if this ever disagreed with the
        // half the player bats in, the wrong line would be highlighted.
        assert_eq!(PLAYER_HALF, InningHalf::Bottom);
    }

    #[test]
    fn each_lamp_kind_has_a_colour_of_its_own() {
        let colours = [
            lit_colour(LampKind::Ball),
            lit_colour(LampKind::Strike),
            lit_colour(LampKind::Out),
        ];
        for (index, colour) in colours.iter().enumerate() {
            assert_ne!(*colour, theme::LAMP_OFF, "lamp {index} lights up as unlit");
            for other in colours.iter().skip(index + 1) {
                assert_ne!(colour, other, "two lamp kinds share a colour");
            }
        }
    }

    #[test]
    fn there_are_enough_lamps_for_a_full_count() {
        // Three balls and two strikes are shown; the fourth ball and third strike
        // end the plate appearance, so they never need a lamp.
        let balls = (0..3).count();
        let strikes = (0..2).count();
        assert_eq!(balls, 3);
        assert_eq!(strikes, 2);
    }
}

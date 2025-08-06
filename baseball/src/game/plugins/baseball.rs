use std::f32::consts::PI;

use bevy::{ecs::spawn::SpawnIter, prelude::*};

use crate::baseball::*;

const FIELD_BROWN: Color = Color::srgb(0.6, 0.4, 0.2);
const MOUND_BROWN: Color = Color::srgb(0.7, 0.5, 0.3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseballPlugin;

impl Plugin for BaseballPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, setup_field, setup_ui, setup_game_state))
            .add_systems(Update, (handle_input, update_game_display, check_game_events))
            .init_resource::<GameData>()
            .init_resource::<BallState>()
            .init_resource::<InputState>();
    }
}

#[derive(Resource, Default)]
struct GameData {
    game_result: Option<GameResult>,
    current_pitch_outcome: Option<PitchOutcome>,
    is_pitching: bool,
    swing_power: f32,
    swing_timing: f32,
}

#[derive(Resource, Default)]
struct BallState {
    position: Vec3,
    velocity: Vec3,
    is_in_play: bool,
    hit_type: Option<HitType>,
}

#[derive(Resource, Default)]
struct InputState {
    swing_charging: bool,
    swing_charge_time: f32,
}

#[derive(Debug, Clone, Copy)]
enum SwingOutcome {
    PitchOutcome(PitchOutcome),
    Hit(HitType),
}

// TODO: SEE IF I NEED THIS?
#[derive(Debug, Clone, Copy)]
enum HitType {
    Grounder,
    Fly,
    LineDrive,
    NoDoubt,
}

// Component tags
#[derive(Component)]
struct Ball;

#[derive(Component)]
struct PitcherMound;

#[derive(Component)]
struct HomePlate;

#[derive(Component)]
struct Player;

#[derive(Component)]
struct BaseMarker;

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct InningText;

#[derive(Component)]
struct CountText;

#[derive(Component)]
struct InstructionText;

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn setup_field(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
    // Field dimensions (scaled for visibility)
    let field_size = 400.0;

    // Create field background
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(field_size * 50.0, field_size * 30.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.2, 0.6, 0.2)))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    let base_positions = [
        (185.0, -185.0), // First base
        (185.0, 185.0),  // Second base
        (-185.0, 185.0), // Third base
    ];

    let base_mesh = meshes.add(Rectangle::new(12.0, 12.0));
    let base_material = materials.add(ColorMaterial::from(Color::WHITE));

    let infield_size = field_size * 0.6;
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::from_size(Vec2::splat(field_size)))),
        MeshMaterial2d(materials.add(ColorMaterial::from(FIELD_BROWN))),
        Transform::from_xyz(0.0, -infield_size / 2.0, 1.0).with_rotation(Quat::from_rotation_z(PI / 4.0)),
        Children::spawn((
            Spawn((
                Mesh2d(meshes.add(Circle::new(25.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(FIELD_BROWN))),
                Transform::from_xyz(-190.0, -190.0, 1.0),
            )),
            Spawn((
                Mesh2d(meshes.add(Circle::new(15.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(MOUND_BROWN))),
                Transform::from_xyz(-10.0, -10.0, 2.0),
                PitcherMound,
            )),
            Spawn((
                Mesh2d(meshes.add(Rectangle::new(8.0, 8.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(Color::WHITE))),
                Transform::from_xyz(-190.0, -190.0, 2.0),
                HomePlate,
            )),
            SpawnIter(base_positions.into_iter().map(move |(x, y)| {
                (
                    Mesh2d(base_mesh.clone()),
                    MeshMaterial2d(base_material.clone()),
                    Transform::from_xyz(x, y, 2.0),
                    BaseMarker,
                )
            })),
        )),
    ));

    // Create ball
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(4.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::WHITE))),
        Transform::from_xyz(0.0, -60.0, 10.0),
        Ball,
    ));

    // Create players
    let player_positions = [
        (0.0, -60.0, PlayerPosition::Pitcher),
        (0.0, -170.0, PlayerPosition::Catcher),
        (90.0, -60.0, PlayerPosition::FirstBase),
        (45.0, 0.0, PlayerPosition::SecondBase),
        (-45.0, 0.0, PlayerPosition::Shortstop),
        (-90.0, -60.0, PlayerPosition::ThirdBase),
        (-120.0, 60.0, PlayerPosition::LeftField),
        (0.0, 120.0, PlayerPosition::CenterField),
        (120.0, 60.0, PlayerPosition::RightField),
        (15.0, -150.0, PlayerPosition::DesignatedHitter),
    ];

    for (x, y, pos) in player_positions.iter() {
        let color = match pos {
            PlayerPosition::DesignatedHitter => Color::srgb(0.8, 0.2, 0.2),
            _ => Color::srgb(0.2, 0.2, 0.8),
        };

        commands.spawn((
            Mesh2d(meshes.add(Circle::new(6.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            Transform::from_xyz(*x, *y, 5.0),
            Player,
        ));
    }
}

fn setup_ui(mut commands: Commands) {
    // Score display
    commands.spawn((
        Text::new("Score: Away 0 - Home 0"),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ScoreText,
    ));

    // Inning display
    commands.spawn((
        Text::new("Top 1st"),
        TextFont {
            font_size: 20.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(40.0),
            left: Val::Px(10.0),
            ..default()
        },
        InningText,
    ));

    // Count display
    commands.spawn((
        Text::new("Count: 0-0"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(70.0),
            left: Val::Px(10.0),
            ..default()
        },
        CountText,
    ));

    // Instructions
    commands.spawn((
        Text::new("A to pitch | Hold SPACE to charge swing | ENTER to swing"),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        InstructionText,
    ));
}

fn setup_game_state(mut game_data: ResMut<GameData>) {
    let game = Game::new();
    game_data.game_result = Some(GameResult::InProgress(game));
    game_data.is_pitching = true;
}

fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut game_data: ResMut<GameData>,
    mut input_state: ResMut<InputState>,
    mut ball_state: ResMut<BallState>,
    time: Res<Time>,
) {
    if let Some(GameResult::InProgress(_)) = &game_data.game_result {
        if game_data.is_pitching {
            // Pitcher input
            if keyboard_input.just_pressed(KeyCode::KeyA) {
                // Pitch the ball
                ball_state.position = Vec3::new(0.0, -60.0, 10.0);
                ball_state.velocity = Vec3::new(0.0, -300.0, 0.0);
                ball_state.is_in_play = true;
                game_data.is_pitching = false;
            }
        } else if ball_state.is_in_play && ball_state.position.y > -160.0 {
            // Batter input - ball is approaching
            if keyboard_input.pressed(KeyCode::Space) {
                // Charge swing
                input_state.swing_charging = true;
                input_state.swing_charge_time += time.delta_secs();
                game_data.swing_power = (input_state.swing_charge_time * 2.0).min(3.0);
            }

            if keyboard_input.just_released(KeyCode::Space) || keyboard_input.just_pressed(KeyCode::Enter) {
                // Execute swing
                let swing_timing = calculate_swing_timing(&ball_state);
                game_data.swing_timing = swing_timing;

                let outcome = determine_swing_outcome(game_data.swing_power, swing_timing);
                info!(
                    "Swing power: {}, Swing timing: {}, Outcome: {:?}",
                    game_data.swing_power, swing_timing, outcome
                );
                match outcome {
                    SwingOutcome::Hit(hit) => {
                        ball_state.hit_type = Some(hit);
                        let velo = match hit {
                            HitType::Grounder => Vec3::new(1.0, 60.0, 0.0),
                            HitType::Fly => Vec3::new(0.0, 240.0, 0.0),
                            HitType::LineDrive => Vec3::new(0.0, 500.0, -1.0),
                            HitType::NoDoubt => Vec3::new(0.0, 800.0, -1.0),
                        };

                        ball_state.velocity = velo;
                    }
                    SwingOutcome::PitchOutcome(po) => {
                        game_data.current_pitch_outcome = Some(po);
                    }
                }

                // Reset swing state
                input_state.swing_charging = false;
                input_state.swing_charge_time = 0.0;
                game_data.swing_power = 0.0;
            }
        }
    }
}

fn update_game_display(
    mut score_query: Query<&mut Text, With<ScoreText>>,
    mut inning_query: Query<&mut Text, With<InningText>>,
    mut count_query: Query<&mut Text, With<CountText>>,
    game_data: Res<GameData>,
) {
    if let Some(GameResult::InProgress(game)) = &game_data.game_result {
        // Update score
        if let Ok(mut score_text) = score_query.single_mut() {
            **score_text = format!("Score: Away {} - Home {}", game.score().away(), game.score().home());
        }

        // Update inning
        if let Ok(mut inning_text) = inning_query.single_mut() {
            **inning_text = game.inning_description();
        }

        // Update count
        if let Ok(mut count_text) = count_query.single_mut() {
            let half_inning = game.current_half_inning();
            let pa = half_inning.current_plate_appearance();
            **count_text = format!(
                "Count: {}-{} | Outs: {}",
                pa.count().balls().as_number(),
                pa.count().strikes().as_number(),
                half_inning.outs().as_number()
            );
        }
    }
}

fn check_game_events(mut game_data: ResMut<GameData>, mut ball_state: ResMut<BallState>) {
    if let Some(outcome) = game_data.current_pitch_outcome.take() {
        if let Some(GameResult::InProgress(game)) = game_data.game_result.take() {
            let new_result = game.advance(outcome);
            info!("New result: {}", new_result);
            game_data.game_result = Some(new_result);

            // Reset for next pitch
            game_data.is_pitching = true;
            ball_state.position = Vec3::new(0.0, -60.0, 10.0);
            ball_state.velocity = Vec3::ZERO;
            ball_state.is_in_play = false;
            ball_state.hit_type = None;
        }
    }
}

fn calculate_swing_timing(ball_state: &BallState) -> f32 {
    // Calculate how close the ball is to the ideal swing point
    let ideal_y = -150.0; // Home plate Y position
    let current_y = ball_state.position.y;
    let distance_from_ideal = (current_y - ideal_y).abs();

    // Return timing score (1.0 = perfect, 0.0 = terrible)
    let score = (1.0 - (distance_from_ideal / 50.0)).max(0.0);
    info!("Timing score: {score}");
    score
}

fn determine_swing_outcome(swing_power: f32, swing_timing: f32) -> SwingOutcome {
    let combined_quality = swing_power * swing_timing;

    if combined_quality < 0.3 {
        // Poor contact
        if swing_timing < 0.5 {
            SwingOutcome::PitchOutcome(PitchOutcome::Strike) // Swung and missed
        } else {
            SwingOutcome::Hit(HitType::Grounder)
        }
    } else if combined_quality < 0.6 {
        // Decent contact
        if swing_power > swing_timing {
            SwingOutcome::Hit(HitType::Grounder)
        } else {
            // Create a simple flyout using the available API
            SwingOutcome::Hit(HitType::Fly)
        }
    } else if combined_quality < 0.8 {
        // Good contact
        SwingOutcome::Hit(HitType::LineDrive)
    } else {
        // Excellent contact
        if combined_quality > 0.9 {
            SwingOutcome::Hit(HitType::NoDoubt)
        } else {
            SwingOutcome::Hit(HitType::LineDrive)
        }
    }
}

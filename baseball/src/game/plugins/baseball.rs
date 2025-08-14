use std::f32::consts::PI;

use bevy::{ecs::spawn::SpawnIter, prelude::*};

use crate::baseball::*;

const FIELD_BROWN: Color = Color::srgb(0.6, 0.4, 0.2);
const MOUND_BROWN: Color = Color::srgb(0.7, 0.5, 0.3);

const BALL_START: Transform = Transform::from_xyz(0.0, -60.0, 10.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BaseballPlugin;

impl Plugin for BaseballPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_camera, setup_field, setup_ui, reset_pitch_timer))
            .add_systems(OnEnter(BallState::PrePitch), (reset_pitch_timer, reset_ball_position))
            .add_systems(
                Update,
                (
                    update_pitch_timer.run_if(in_state(BallState::PrePitch)),
                    (move_ball, swing).run_if(in_state(BallState::Pitch)),
                ),
            )
            // .add_systems(OnEnter(BallState::Pitch), (pitch_ball))
            .init_resource::<GameData>()
            .init_state::<BallState>()
            .init_resource::<PitchTimer>();
    }
}

#[derive(Resource)]
#[expect(dead_code)]
struct GameData {
    game_result: GameOutcome,
}

impl Default for GameData {
    fn default() -> Self {
        Self {
            game_result: GameOutcome::InProgress(Game::new()),
        }
    }
}

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[expect(dead_code)]
enum BallState {
    #[default]
    PrePitch,
    Pitch,
    InPlay,
}

fn reset_pitch_timer(mut timer: ResMut<PitchTimer>) {
    *timer = PitchTimer::default();
}

fn update_pitch_timer(time: Res<Time>, mut timer: ResMut<PitchTimer>, mut state: ResMut<NextState<BallState>>) {
    if timer.timer.tick(time.delta()).just_finished() {
        info!("Transitioning from PrePitch to Pitch");
        state.set(BallState::Pitch);
    }
}

fn reset_ball_position(mut ball: Query<&mut Transform, With<Ball>>) {
    if let Ok(mut tform) = ball.single_mut() {
        *tform = BALL_START
    }
}

#[derive(Resource, Debug)]
struct PitchTimer {
    timer: Timer,
}

impl Default for PitchTimer {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Once),
        }
    }
}

fn move_ball(mut ball: Query<&mut Transform, With<Ball>>) {
    if let Ok(mut tform) = ball.single_mut() {
        tform.translation.y -= 5.0;
    }
}

fn swing(input: Res<ButtonInput<KeyCode>>, mut ball: Query<&mut Transform, With<Ball>>) {
    if input.pressed(KeyCode::Space) {
        if let Ok(mut tform) = ball.single_mut() {
            tform.translation.y += 10.0;
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[expect(dead_code)]
enum SwingOutcome {
    PitchOutcome(PitchOutcome),
    Hit(HitType),
}

#[derive(Debug, Clone, Copy)]
#[expect(dead_code)]
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
#[expect(dead_code)]
struct Player(PlayerPosition);

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
                Mesh2d(meshes.add(Circle::new(45.0))),
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
        BALL_START,
        Ball,
    ));

    // Create players
    let player_positions = [
        (0.0, -130.0, PlayerPosition::Pitcher),
        (0.0, -415.0, PlayerPosition::Catcher),
        (270.0, -60.0, PlayerPosition::FirstBase),
        (125.0, 80.0, PlayerPosition::SecondBase),
        (-150.0, 70.0, PlayerPosition::Shortstop),
        (-260.0, -40.0, PlayerPosition::ThirdBase),
        (-400.0, 280.0, PlayerPosition::LeftField),
        (0.0, 450.0, PlayerPosition::CenterField),
        (450.0, 290.0, PlayerPosition::RightField),
        (15.0, -390.0, PlayerPosition::DesignatedHitter),
    ];

    for (x, y, pos) in player_positions.into_iter() {
        let color = match pos {
            PlayerPosition::DesignatedHitter => Color::srgb(0.8, 0.2, 0.2),
            _ => Color::srgb(0.2, 0.2, 0.8),
        };

        commands.spawn((
            Mesh2d(meshes.add(Circle::new(6.0))),
            MeshMaterial2d(materials.add(ColorMaterial::from(color))),
            Transform::from_xyz(x, y, 5.0),
            Player(pos.clone()),
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

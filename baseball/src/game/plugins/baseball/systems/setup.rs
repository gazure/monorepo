use std::f32::consts::PI;

use baseball_game_rules::PlayerPosition;
use bevy::{ecs::spawn::SpawnIter, prelude::*};

use crate::game::plugins::baseball::{components::*, constants::*};

pub fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub fn setup_field(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // Field dimensions (scaled for visibility)
    let field_size = 400.0;

    // Create field background
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(field_size * 50.0, field_size * 30.0))),
        MeshMaterial2d(materials.add(ColorMaterial::from(Color::srgb(0.2, 0.6, 0.2)))),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

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
            SpawnIter(BASE_POSITIONS.into_iter().map(move |(x, y)| {
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
        BallVelocity::zero(),
        Ball,
    ));

    // Create baserunner indicators (initially hidden)
    let runner_color = materials.add(ColorMaterial::from(Color::srgb(0.9, 0.7, 0.2))); // Gold/yellow
    let runner_mesh = meshes.add(Circle::new(8.0));

    // First base runner
    commands.spawn((
        Mesh2d(runner_mesh.clone()),
        MeshMaterial2d(runner_color.clone()),
        Transform::from_xyz(BASE_POSITIONS[0].0, BASE_POSITIONS[0].1, 6.0),
        Visibility::Hidden,
        FirstBaseRunner,
    ));

    // Second base runner
    commands.spawn((
        Mesh2d(runner_mesh.clone()),
        MeshMaterial2d(runner_color.clone()),
        Transform::from_xyz(BASE_POSITIONS[1].0, BASE_POSITIONS[1].1, 6.0),
        Visibility::Hidden,
        SecondBaseRunner,
    ));

    // Third base runner
    commands.spawn((
        Mesh2d(runner_mesh),
        MeshMaterial2d(runner_color),
        Transform::from_xyz(BASE_POSITIONS[2].0, BASE_POSITIONS[2].1, 6.0),
        Visibility::Hidden,
        ThirdBaseRunner,
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

        if matches!(pos, PlayerPosition::DesignatedHitter) {
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(6.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                Transform::from_xyz(x, y, 5.0),
                Player(pos),
                Batter,
            ));
        } else {
            commands.spawn((
                Mesh2d(meshes.add(Circle::new(6.0))),
                MeshMaterial2d(materials.add(ColorMaterial::from(color))),
                Transform::from_xyz(x, y, 5.0),
                Player(pos),
            ));
        }
    }
}

pub fn setup_ui(mut commands: Commands) {
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
        Text::new("SPACE to swing"),
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

    // Result text (centered, initially hidden)
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 48.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 1.0, 0.0)), // Yellow
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(30.0),
            left: Val::Percent(50.0),
            ..default()
        },
        Visibility::Hidden,
        ResultText,
    ));
}

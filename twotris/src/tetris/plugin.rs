#[cfg(not(target_arch = "wasm32"))]
use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use tracing::debug;

use super::components::TetrominoTimer;
use crate::tetris::components::{
    ControlledTetromino, Coordinate, DrawGrid, Focus, GameOver, Grid, GridTetromino, RowClearedEvent, Score, Shadow,
};

const FOCUS_COLOR: Color = Color::linear_rgba(1.0, 1.0, 1.0, 1.0);
const NON_FOCUS_COLOR: Color = Color::linear_rgba(0.5, 0.5, 0.5, 1.0);
const SHADOW_COLOR: Color = Color::linear_rgba(0.0, 0.0, 0.0, 0.1);
const CELL_SIZE: f32 = 20.0;

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
enum TetrisState {
    #[default]
    InGame,
    GameOver,
}

#[derive(Debug, Resource)]
pub struct RandomSource(rand_chacha::ChaCha8Rng);

impl Default for RandomSource {
    fn default() -> Self {
        RandomSource(rand_chacha::ChaCha8Rng::from_entropy())
    }
}

impl RandomSource {
    pub fn next(&mut self, min: u32, max: u32) -> u32 {
        self.0.gen_range(min..max)
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        Score(0),
        Text::new("Score: 0"),
        TextFont::from_font(asset_server.load("fonts/JetBrainsMono-Bold.ttf")).with_font_size(36.0),
        TextColor::from(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(200.0),
            bottom: Val::Px(800.0),
            ..default()
        },
    ));
}

fn init_spawn_tetrominos(
    mut commands: Commands,
    mut random_source: ResMut<RandomSource>,
    mut grid_query: Query<(Entity, &mut Grid)>,
    mut draw_grid: EventWriter<DrawGrid>,
) {
    for (entity, mut grid) in &mut grid_query {
        debug!("Spawning a tetromino");
        let tetromino = ControlledTetromino::new(random_source.as_mut());
        grid.set_tetromino(&tetromino);
        let shadow = grid.controlled_tetromino_shadow(&tetromino);
        commands.spawn((shadow, Shadow, GridTetromino::new(entity)));
        commands.spawn((TetrominoTimer::default(), tetromino, GridTetromino::new(entity)));
        draw_grid.write(DrawGrid(entity));
    }
}

fn swap_focus(
    input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut focus_grid: Query<Entity, With<Focus>>,
    mut non_focus_grid: Query<Entity, (With<Grid>, Without<Focus>)>,
    mut draw_grid: EventWriter<DrawGrid>,
) {
    if input.just_pressed(KeyCode::KeyF) {
        debug!("Swapping focus");
        for entity in &mut focus_grid {
            commands.entity(entity).remove::<Focus>();
            draw_grid.write(DrawGrid(entity));
        }
        for non_focus_entity in &mut non_focus_grid {
            commands.entity(non_focus_entity).insert(Focus);
            draw_grid.write(DrawGrid(non_focus_entity));
        }
    }
}

fn handle_input(
    input: Res<ButtonInput<KeyCode>>,
    mut grid: Query<(Entity, &mut Grid), With<Focus>>,
    mut tetromino: Query<(&GridTetromino, &mut ControlledTetromino), Without<Shadow>>,
    mut shadows: Query<(&GridTetromino, &mut ControlledTetromino), With<Shadow>>,
    mut draw_grid: EventWriter<DrawGrid>,
) {
    for (entity, mut grid) in &mut grid {
        for (grid_owner, mut tetromino) in &mut tetromino {
            if grid_owner.get() != entity {
                continue;
            }
            let shadow = shadows
                .iter_mut()
                .find(|(shadow_owner, _)| shadow_owner.get() == entity);

            if input.just_pressed(KeyCode::ArrowLeft) && !grid.is_tetromino_blocked_left(&tetromino) {
                debug!("Moving tetromino left");
                grid.unset_tetromino(tetromino.as_ref());
                tetromino.top_left.0 -= 1;
                grid.set_tetromino(tetromino.as_ref());
            }

            if input.just_pressed(KeyCode::ArrowRight) && !grid.is_tetromino_blocked_right(&tetromino) {
                debug!("Moving tetromino right");
                grid.unset_tetromino(tetromino.as_ref());
                tetromino.top_left.0 += 1;
                grid.set_tetromino(tetromino.as_ref());
            }

            if input.just_pressed(KeyCode::Space) {
                debug!("Rotating tetromino");
                let old_rotation = tetromino.rotation;
                grid.unset_tetromino(tetromino.as_ref());
                tetromino.rotate();
                if !grid.is_tetromino_space_open(&tetromino) {
                    tetromino.rotation = old_rotation;
                }
                grid.set_tetromino(tetromino.as_ref());
            }

            if let Some((_, mut shadow)) = shadow {
                *shadow = grid.controlled_tetromino_shadow(tetromino.as_ref());
            }

            draw_grid.write(DrawGrid(entity));
        }
    }
}

fn handle_timed_movement(
    mut commands: Commands,
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut random_source: ResMut<RandomSource>,
    mut grid: Query<(Entity, &mut Grid, Option<&Focus>)>,
    mut tetromino: Query<(Entity, &GridTetromino, &mut ControlledTetromino, &mut TetrominoTimer)>,
    mut next_state: ResMut<NextState<TetrisState>>,
    mut rows_cleared: EventWriter<RowClearedEvent>,
    mut draw_grid: EventWriter<DrawGrid>,
) {
    for (entity, mut grid, focus) in &mut grid {
        for (tetromino_id, grid_owner, mut tetromino, mut timer) in &mut tetromino {
            if grid_owner.get() != entity {
                continue;
            }
            timer.0.tick(time.delta());
            let should_force_to_bottom = input.just_pressed(KeyCode::ArrowDown) && focus.is_some();
            if should_force_to_bottom {
                grid.unset_tetromino(tetromino.as_ref());
                grid.force_tetromino_to_bottom(tetromino.as_mut());
                grid.set_tetromino(tetromino.as_ref());
            }

            if timer.0.finished() || should_force_to_bottom {
                if grid.is_tetromino_at_bottom(tetromino.as_ref()) {
                    debug!("Tetromino at bottom, despawning and spawning a new one");
                    rows_cleared.write(RowClearedEvent::new(grid.clear_full_grid_rows()));
                    commands.entity(tetromino_id).despawn();
                    let tetromino = ControlledTetromino::new(random_source.as_mut());
                    if grid.is_tetromino_space_open(&tetromino) {
                        grid.set_tetromino(&tetromino);
                        commands.spawn((TetrominoTimer::default(), tetromino, GridTetromino::new(entity)));
                    } else {
                        next_state.set(TetrisState::GameOver);
                    }
                } else {
                    debug!("Moving tetromino down");
                    grid.unset_tetromino(tetromino.as_ref());
                    tetromino.top_left.1 += 1;
                    grid.set_tetromino(tetromino.as_ref());
                }
                draw_grid.write(DrawGrid(entity));
            }
        }
    }
}

fn game_over(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    tetromino: Query<Entity, With<ControlledTetromino>>,
) {
    for entity_id in tetromino.iter() {
        commands.entity(entity_id).despawn();
    }
    commands.spawn((
        GameOver,
        Text::new("Game Over\nR: Restart"),
        TextFont::from_font(asset_server.load("fonts/JetBrainsMono-Bold.ttf")).with_font_size(72.0),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(500.0),
            left: Val::Px(600.0),
            ..default()
        },
    ));
}

fn reset(
    mut next_state: ResMut<NextState<TetrisState>>,
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    gameover: Query<Entity, With<GameOver>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        next_state.set(TetrisState::InGame);
        for entity_id in gameover.iter() {
            commands.entity(entity_id).despawn();
        }
        commands.remove_resource::<RandomSource>();
        commands.insert_resource(RandomSource::default());
    }
}

fn reset_grid(
    mut commands: Commands,
    _asset_server: Res<AssetServer>,
    mut grid: Query<&mut Grid>,
    mut score: Query<(&mut Score, &mut Text), Without<Grid>>,
    mut visibile_squares: Query<&mut Visibility, With<Coordinate>>,
) {
    if grid.iter().len() == 0 {
        for i in 0..2 {
            let grid = Grid::default();
            let height = grid.height();
            let width = grid.width();
            let mut entity = commands.spawn((
                grid,
                Transform::from_xyz(-500.0 + (i as f32 * 400.0), 260.0, 0.0),
                Visibility::default(),
            ));
            if i == 0 {
                entity.insert(Focus);
            }
            entity.with_children(|cb| {
                for i in 0..height {
                    for j in 0..width {
                        cb.spawn((
                            Coordinate(j, i),
                            Transform::from_xyz(j as f32 * CELL_SIZE, -(i as f32 * CELL_SIZE), 2.0),
                            Visibility::Hidden,
                            Sprite {
                                color: FOCUS_COLOR,
                                custom_size: Some(Vec2::splat(CELL_SIZE - 2.0)),
                                ..default()
                            },
                        ))
                        .with_children(|cb| {
                            cb.spawn((
                                Sprite::from_color(Color::srgb(0.0, 0.0, 0.0), Vec2::splat(CELL_SIZE)),
                                Transform::from_xyz(0.0, 0.0, -1.0),
                                Visibility::Inherited,
                            ));
                        });
                    }
                }
            });
        }
    } else {
        for mut grid in &mut grid {
            grid.clear();
            for mut visibility in &mut visibile_squares {
                *visibility = Visibility::Hidden;
            }
        }
    }

    for (mut score, mut text) in &mut score {
        score.reset();
        text.0 = format!("Score: {}", score.get());
    }
}

fn draw_grid(
    mut dg_events: EventReader<DrawGrid>,
    grid: Query<(Entity, &Grid, Option<&Focus>)>,
    mut visible_squares: Query<(&mut Visibility, &mut Sprite, &Coordinate, &ChildOf)>,
    shadows: Query<(&ControlledTetromino, &GridTetromino), With<Shadow>>,
) {
    for event in dg_events.read() {
        for (entity, grid, focus) in &grid {
            if entity != event.0 {
                continue;
            }
            let shadow_coords: Vec<_> = if let Some((shadow, _)) = shadows.iter().find(|(_, gt)| gt.get() == entity) {
                shadow.coords().collect()
            } else {
                vec![]
            };
            let set_coords: Vec<_> = grid.set_coords_iter().collect();
            for (mut visibility, mut sprite, coord, child_of) in &mut visible_squares {
                if child_of.parent() != entity {
                    continue;
                }
                let is_shadow = shadow_coords.contains(&coord.tuple());
                let color = if focus.is_some() { FOCUS_COLOR } else { NON_FOCUS_COLOR };

                if shadow_coords.contains(&coord.tuple()) {
                    *visibility = Visibility::Visible;
                    sprite.color = SHADOW_COLOR;
                }

                if set_coords.contains(&coord.tuple()) {
                    *visibility = Visibility::Visible;
                    sprite.color = color;
                } else if !is_shadow {
                    *visibility = Visibility::Hidden;
                    sprite.color = color;
                }
            }
        }
    }
}

fn update_score(mut score: Query<(&mut Score, &mut Text)>, mut event: EventReader<RowClearedEvent>) {
    for event in event.read() {
        for (mut score, mut text) in &mut score {
            score.add_cleared_rows(event.0);
            text.0 = format!("Score: {}", score.get());
        }
    }
}

pub struct TetrisPlugin;

impl Plugin for TetrisPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(RandomSource::default())
            .init_state::<TetrisState>()
            .add_event::<RowClearedEvent>()
            .add_event::<DrawGrid>()
            .add_systems(Startup, setup)
            .add_systems(
                OnEnter(TetrisState::InGame),
                (reset_grid, init_spawn_tetrominos).chain(),
            )
            .add_systems(
                Update,
                (swap_focus, handle_timed_movement, handle_input, update_score, draw_grid)
                    .run_if(in_state(TetrisState::InGame)),
            )
            .add_systems(OnEnter(TetrisState::GameOver), (game_over,))
            .add_systems(Update, (reset,).run_if(in_state(TetrisState::GameOver)));
    }
}

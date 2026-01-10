use std::collections::HashSet;

use bevy::prelude::*;

use super::{
    components::{
        ActiveGrid, CELL_SIZE, CHUNK_LOAD_RADIUS, Chunk, ChunkCell, DEAD_COLOR, GRID_HEIGHT, GRID_WIDTH, Grid,
        activation_count_color, binary_color, chunk_to_world, fire_color, generation_based_color, monochrome_color,
        neighbor_count_color, neon_color, ocean_color, pastel_rainbow_color, world_to_chunk, world_to_grid,
    },
    resources::{ChunkManager, ColorPattern, SimulationState},
};
use crate::GameState;

/// Spawns a single chunk at given chunk coordinates
fn spawn_chunk(
    commands: &mut Commands,
    chunk_x: i32,
    chunk_y: i32,
    _grid_query: &Query<&Grid, With<ActiveGrid>>,
) -> Entity {
    let (chunk_world_x, chunk_world_y) = chunk_to_world(chunk_x, chunk_y);

    let offset_x = -(GRID_WIDTH as f32 * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;
    let offset_y = -(GRID_HEIGHT as f32 * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;

    let mut chunk_entity = commands.spawn((
        Chunk { x: chunk_x, y: chunk_y },
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
    ));

    chunk_entity.with_children(|parent| {
        for y in 0..GRID_HEIGHT {
            for x in 0..GRID_WIDTH {
                let world_x = chunk_world_x + offset_x + x as f32 * CELL_SIZE;
                let world_y = chunk_world_y + offset_y + y as f32 * CELL_SIZE;

                parent.spawn((
                    Sprite {
                        color: DEAD_COLOR,
                        custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                        ..default()
                    },
                    Transform::from_xyz(world_x, world_y, 0.0),
                    ChunkCell { grid_x: x, grid_y: y },
                ));
            }
        }
    });

    chunk_entity.id()
}

/// Despawns a chunk and all its cell children
fn despawn_chunk(commands: &mut Commands, chunk_entity: Entity) {
    // Despawning the parent automatically despawns children in Bevy's hierarchy
    commands.entity(chunk_entity).despawn();
}

pub fn setup(mut commands: Commands) {
    let grid = Grid::default_randomized();

    // Spawn Grid entity WITHOUT children - just simulation data
    commands.spawn((
        grid,
        ActiveGrid,
        Transform::default(),
        GlobalTransform::default(),
        Visibility::default(),
        InheritedVisibility::default(),
    ));

    // Initialize chunk manager with center at (0, 0)
    commands.insert_resource(ChunkManager {
        active_chunks: std::collections::HashMap::new(),
        current_center_chunk: Some((0, 0)),
    });

    commands.insert_resource(SimulationState::default());
}

pub fn update_grid(
    mut grid_query: Query<&mut Grid, With<ActiveGrid>>,
    mut state: ResMut<SimulationState>,
    time: Res<Time>,
) {
    if state.paused && !state.step_mode {
        return;
    }

    state.update_timer.tick(time.delta());

    if !state.update_timer.just_finished() && !state.step_mode {
        return;
    }

    if state.step_mode {
        state.step_mode = false;
    }

    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let neighbors = grid.count_neighbors(x, y);
            let alive = grid.get(x, y);

            let new_state = matches!((alive, neighbors), (true, 2..=3) | (false, 3));
            grid.set_back(x, y, new_state, state.generation + 1);
        }
    }

    grid.swap_buffers();
    state.generation += 1;
}

pub fn render_cells(
    grid_query: Query<&Grid, With<ActiveGrid>>,
    mut cell_query: Query<(&ChunkCell, &mut Sprite)>,
    state: Res<SimulationState>,
) {
    let Ok(grid) = grid_query.single() else {
        return;
    };

    for (chunk_cell, mut sprite) in &mut cell_query {
        let alive = grid.get(chunk_cell.grid_x, chunk_cell.grid_y);
        let target_color = if alive {
            match state.color_pattern {
                ColorPattern::ActivationCount => {
                    activation_count_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y))
                }
                ColorPattern::Binary => binary_color(),
                ColorPattern::NeighborCount => {
                    let neighbors = grid.count_neighbors(chunk_cell.grid_x, chunk_cell.grid_y) as u8;
                    neighbor_count_color(neighbors)
                }
                ColorPattern::PastelRainbow => {
                    pastel_rainbow_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y))
                }
                ColorPattern::Neon => neon_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y)),
                ColorPattern::Monochrome => {
                    monochrome_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y))
                }
                ColorPattern::Ocean => ocean_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y)),
                ColorPattern::Fire => fire_color(grid.get_activation_count(chunk_cell.grid_x, chunk_cell.grid_y)),
                ColorPattern::GenerationBased => generation_based_color(
                    grid.get_last_toggled_generation(chunk_cell.grid_x, chunk_cell.grid_y),
                    state.generation,
                ),
            }
        } else {
            DEAD_COLOR
        };

        if sprite.color != target_color {
            sprite.color = target_color;
        }
    }
}

pub fn handle_keyboard_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut grid_query: Query<&mut Grid, With<ActiveGrid>>,
    mut state: ResMut<SimulationState>,
    mut next_game_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        next_game_state.set(GameState::Menu);
        info!("Returning to menu");
        return;
    }

    if keyboard.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
        info!("Simulation {}", if state.paused { "paused" } else { "running" });
    }

    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::KeyR) {
        grid.randomize();
        state.generation = 0;
        info!("Grid randomized");
    }

    if keyboard.just_pressed(KeyCode::KeyC) {
        grid.clear();
        state.generation = 0;
        info!("Grid cleared");
    }

    if keyboard.just_pressed(KeyCode::Period) {
        state.step_mode = true;
        info!("Stepping forward (generation {})", state.generation);
    }

    if keyboard.just_pressed(KeyCode::Equal) || keyboard.just_pressed(KeyCode::NumpadAdd) {
        let current = state.update_timer.duration().as_secs_f32();
        let new_duration = (current * 0.8).max(0.01);
        state
            .update_timer
            .set_duration(std::time::Duration::from_secs_f32(new_duration));
        info!("Speed increased (interval: {:.3}s)", new_duration);
    }

    if keyboard.just_pressed(KeyCode::Minus) || keyboard.just_pressed(KeyCode::NumpadSubtract) {
        let current = state.update_timer.duration().as_secs_f32();
        let new_duration = (current * 1.25).min(2.0);
        state
            .update_timer
            .set_duration(std::time::Duration::from_secs_f32(new_duration));
        info!("Speed decreased (interval: {:.3}s)", new_duration);
    }

    if keyboard.just_pressed(KeyCode::Digit1) {
        state.color_pattern = ColorPattern::ActivationCount;
        info!("Color pattern: Activation Count (Rainbow)");
    }

    if keyboard.just_pressed(KeyCode::Digit2) {
        state.color_pattern = ColorPattern::Binary;
        info!("Color pattern: Binary (White/Black)");
    }

    if keyboard.just_pressed(KeyCode::Digit3) {
        state.color_pattern = ColorPattern::NeighborCount;
        info!("Color pattern: Neighbor Count");
    }

    if keyboard.just_pressed(KeyCode::Digit4) {
        state.color_pattern = ColorPattern::PastelRainbow;
        info!("Color pattern: Pastel Rainbow");
    }

    if keyboard.just_pressed(KeyCode::Digit5) {
        state.color_pattern = ColorPattern::Neon;
        info!("Color pattern: Neon");
    }

    if keyboard.just_pressed(KeyCode::Digit6) {
        state.color_pattern = ColorPattern::Monochrome;
        info!("Color pattern: Monochrome");
    }

    if keyboard.just_pressed(KeyCode::Digit7) {
        state.color_pattern = ColorPattern::Ocean;
        info!("Color pattern: Ocean");
    }

    if keyboard.just_pressed(KeyCode::Digit8) {
        state.color_pattern = ColorPattern::Fire;
        info!("Color pattern: Fire");
    }

    if keyboard.just_pressed(KeyCode::Digit9) {
        state.color_pattern = ColorPattern::GenerationBased;
        info!("Color pattern: Generation Based");
    }
}

pub fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut grid_query: Query<&mut Grid, With<ActiveGrid>>,
    state: Res<SimulationState>,
) {
    let clicked = mouse_button.pressed(MouseButton::Left)
        || mouse_button.just_pressed(MouseButton::Left)
        || mouse_button.just_released(MouseButton::Left);

    if clicked {
        debug!("Mouse button clicked: {}", clicked);
    } else {
        return;
    }

    let Some(window) = windows.iter().next() else {
        return;
    };

    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };

    let Some((camera, camera_transform)) = camera_query.iter().next() else {
        return;
    };

    let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) else {
        return;
    };

    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };

    // Convert world position to chunk and grid coordinates
    let (chunk_coord, (grid_x, grid_y)) = world_to_grid(world_pos.x, world_pos.y);

    if !grid.get(grid_x, grid_y) {
        grid.set(grid_x, grid_y, true, state.generation);
        debug!("Cell set at chunk {:?}, grid ({}, {})", chunk_coord, grid_x, grid_y);
    }
}

pub fn handle_camera_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    const PAN_SPEED: f32 = 300.0;
    const ZOOM_SPEED: f32 = 2.0;

    let Some(mut camera_transform) = camera_query.iter_mut().next() else {
        return;
    };

    let mut direction = Vec2::ZERO;

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }

    if direction != Vec2::ZERO {
        direction = direction.normalize();
        camera_transform.translation.x += direction.x * PAN_SPEED * time.delta_secs();
        camera_transform.translation.y += direction.y * PAN_SPEED * time.delta_secs();
    }

    let mut zoom_delta = 0.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        zoom_delta += ZOOM_SPEED * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::KeyE) {
        zoom_delta -= ZOOM_SPEED * time.delta_secs();
    }

    if zoom_delta != 0.0 {
        let new_scale = (camera_transform.scale.x + zoom_delta).clamp(0.5, 5.0);
        camera_transform.scale = Vec3::splat(new_scale);
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        camera_transform.translation = Vec3::ZERO;
        camera_transform.scale = Vec3::ONE;
        info!("Camera reset to home position");
    }
}

/// Manages chunk loading/unloading based on camera position
pub fn manage_chunks(
    mut commands: Commands,
    camera_query: Query<&Transform, With<Camera>>,
    grid_query: Query<&Grid, With<ActiveGrid>>,
    mut chunk_manager: ResMut<ChunkManager>,
) {
    let Some(camera_transform) = camera_query.iter().next() else {
        return;
    };

    let camera_pos = camera_transform.translation;
    let current_chunk = world_to_chunk(camera_pos.x, camera_pos.y);

    // Check if camera moved to a different chunk
    if chunk_manager.current_center_chunk == Some(current_chunk) {
        return; // No change
    }

    info!("Camera moved to chunk {:?}", current_chunk);
    chunk_manager.current_center_chunk = Some(current_chunk);

    // Calculate 3×3 chunks around camera
    let mut required_chunks = HashSet::new();
    for dy in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
        for dx in -CHUNK_LOAD_RADIUS..=CHUNK_LOAD_RADIUS {
            required_chunks.insert((current_chunk.0 + dx, current_chunk.1 + dy));
        }
    }

    // Despawn chunks no longer needed
    let mut chunks_to_remove = Vec::new();
    for (&chunk_coord, &chunk_entity) in &chunk_manager.active_chunks {
        if !required_chunks.contains(&chunk_coord) {
            info!("Despawning chunk {:?}", chunk_coord);
            despawn_chunk(&mut commands, chunk_entity);
            chunks_to_remove.push(chunk_coord);
        }
    }
    for coord in chunks_to_remove {
        chunk_manager.active_chunks.remove(&coord);
    }

    // Spawn new chunks
    for &chunk_coord in &required_chunks {
        chunk_manager.active_chunks.entry(chunk_coord).or_insert_with(|| {
            info!("Spawning chunk {:?}", chunk_coord);
            spawn_chunk(&mut commands, chunk_coord.0, chunk_coord.1, &grid_query)
        });
    }
}

pub fn cleanup_game(
    mut commands: Commands,
    grid_query: Query<Entity, With<ActiveGrid>>,
    chunk_query: Query<Entity, With<Chunk>>,
) {
    info!("Cleaning up game");

    // Despawn all chunks first (cascades to cells)
    for entity in &chunk_query {
        commands.entity(entity).despawn();
    }

    // Despawn Grid entity
    for entity in &grid_query {
        commands.entity(entity).despawn();
    }

    // Remove resources
    commands.remove_resource::<SimulationState>();
    commands.remove_resource::<ChunkManager>();
}

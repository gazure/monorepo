use bevy::prelude::*;

use super::{
    components::Cell,
    resources::{CELL_SIZE, DEAD_COLOR, GRID_HEIGHT, GRID_WIDTH, Grid, SimulationState, cell_color},
};

pub fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let mut grid = Grid::new(GRID_WIDTH, GRID_HEIGHT);
    grid.randomize();

    let offset_x = -(grid.width() as f32 * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;
    let offset_y = -(grid.height() as f32 * CELL_SIZE) / 2.0 + CELL_SIZE / 2.0;

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let color = if grid.get(x, y) {
                cell_color(grid.get_activation_count(x, y))
            } else {
                DEAD_COLOR
            };

            commands.spawn((
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
                    ..default()
                },
                Transform::from_xyz(offset_x + x as f32 * CELL_SIZE, offset_y + y as f32 * CELL_SIZE, 0.0),
                Cell { x, y },
            ));
        }
    }

    commands.insert_resource(grid);
    commands.insert_resource(SimulationState::default());
}

pub fn update_grid(mut grid: ResMut<Grid>, mut state: ResMut<SimulationState>, time: Res<Time>) {
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

    for y in 0..grid.height() {
        for x in 0..grid.width() {
            let neighbors = grid.count_neighbors(x, y);
            let alive = grid.get(x, y);

            let new_state = matches!((alive, neighbors), (true, 2..=3) | (false, 3));
            grid.set_back(x, y, new_state);
        }
    }

    grid.swap_buffers();
    state.generation += 1;
}

pub fn render_cells(grid: Res<Grid>, mut query: Query<(&Cell, &mut Sprite)>) {
    for (cell, mut sprite) in &mut query {
        let alive = grid.get(cell.x, cell.y);
        let target_color = if alive {
            cell_color(grid.get_activation_count(cell.x, cell.y))
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
    mut grid: ResMut<Grid>,
    mut state: ResMut<SimulationState>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        state.paused = !state.paused;
        info!("Simulation {}", if state.paused { "paused" } else { "running" });
    }

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
}

pub fn handle_mouse_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut grid: ResMut<Grid>,
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

    let offset_x = -(grid.width() as f32 * CELL_SIZE) / 2.0;
    let offset_y = -(grid.height() as f32 * CELL_SIZE) / 2.0;

    let grid_x = ((world_pos.x - offset_x) / CELL_SIZE).floor() as i32;
    let grid_y = ((world_pos.y - offset_y) / CELL_SIZE).floor() as i32;

    if grid_x >= 0 && grid_x < grid.width() as i32 && grid_y >= 0 && grid_y < grid.height() as i32 {
        let x = grid_x as usize;
        let y = grid_y as usize;
        if !grid.get(x, y) {
            grid.set(x, y, true);
            debug!("Cell set at ({}, {})", x, y);
        }
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

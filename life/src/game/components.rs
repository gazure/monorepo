use bevy::prelude::{Color, Component, Vec, vec};
use rand::Rng;

pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 200;
pub const CELL_SIZE: f32 = 12.0;
pub const DEAD_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
pub const MAX_ACTIVATION_COLOR_SCALE: u32 = 25;
pub const CHUNK_SIZE: f32 = GRID_WIDTH as f32 * CELL_SIZE; // 4800.0
pub const CHUNK_LOAD_RADIUS: i32 = 2; // 5x5 = center + 2 in each direction

pub fn activation_count_color(activation_count: u32) -> Color {
    let hue = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32) * 360.0;
    Color::hsl(hue % 360.0, 1.0, 0.5)
}

pub fn binary_color() -> Color {
    Color::WHITE
}

pub fn neighbor_count_color(neighbor_count: u8) -> Color {
    let ratio = f32::from(neighbor_count) / 8.0;
    let hue = 180.0 - (ratio * 180.0);
    Color::hsl(hue, 1.0, 0.5)
}

pub fn pastel_rainbow_color(activation_count: u32) -> Color {
    let hue = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32) * 360.0;
    Color::hsl(hue % 360.0, 0.6, 0.75)
}

pub fn neon_color(activation_count: u32) -> Color {
    let hue = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32) * 360.0;
    Color::hsl(hue % 360.0, 1.0, 0.6)
}

pub fn monochrome_color(activation_count: u32) -> Color {
    let intensity = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32).min(1.0);
    let value = 0.3 + (intensity * 0.7);
    Color::srgb(value, value, value)
}

pub fn ocean_color(activation_count: u32) -> Color {
    let ratio = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32).min(1.0);
    let hue = 180.0 + (ratio * 30.0);
    let lightness = 0.4 + (ratio * 0.3);
    Color::hsl(hue, 0.8, lightness)
}

pub fn fire_color(activation_count: u32) -> Color {
    let ratio = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32).min(1.0);
    let hue = 60.0 - (ratio * 60.0);
    let lightness = 0.4 + (ratio * 0.2);
    Color::hsl(hue, 1.0, lightness)
}

pub fn generation_based_color(generation_born: u64, current_generation: u64) -> Color {
    let age = current_generation.saturating_sub(generation_born);
    let hue = (age as f32 * 20.0) % 360.0;
    Color::hsl(hue, 0.9, 0.55)
}

/// Converts world position to chunk coordinates
pub fn world_to_chunk(world_x: f32, world_y: f32) -> (i32, i32) {
    let chunk_x = (world_x / CHUNK_SIZE).floor() as i32;
    let chunk_y = (world_y / CHUNK_SIZE).floor() as i32;
    (chunk_x, chunk_y)
}

/// Converts chunk coordinates to world position (chunk origin)
pub fn chunk_to_world(chunk_x: i32, chunk_y: i32) -> (f32, f32) {
    (chunk_x as f32 * CHUNK_SIZE, chunk_y as f32 * CHUNK_SIZE)
}

/// Converts world position to (`chunk_coord`, `grid_coord`)
pub fn world_to_grid(world_x: f32, world_y: f32) -> ((i32, i32), (usize, usize)) {
    let chunk = world_to_chunk(world_x, world_y);
    let (chunk_world_x, chunk_world_y) = chunk_to_world(chunk.0, chunk.1);

    let offset_x = -(GRID_WIDTH as f32 * CELL_SIZE) / 2.0;
    let offset_y = -(GRID_HEIGHT as f32 * CELL_SIZE) / 2.0;

    let grid_x = ((world_x - chunk_world_x - offset_x) / CELL_SIZE).floor() as i32;
    let grid_y = ((world_y - chunk_world_y - offset_y) / CELL_SIZE).floor() as i32;

    let grid_x = grid_x.clamp(0, GRID_WIDTH as i32 - 1) as usize;
    let grid_y = grid_y.clamp(0, GRID_HEIGHT as i32 - 1) as usize;

    (chunk, (grid_x, grid_y))
}

// #[derive(Component)]
// pub struct Cell {
//     pub x: usize,
//     pub y: usize,
// }

#[expect(dead_code)]
#[derive(Component)]
pub struct Chunk {
    pub x: i32,
    pub y: i32,
}

#[derive(Component)]
pub struct ChunkCell {
    pub grid_x: usize,
    pub grid_y: usize,
}

#[derive(Component)]
pub struct ActiveGrid;

#[derive(Component)]
pub struct Grid {
    front: Vec<Vec<bool>>,
    back: Vec<Vec<bool>>,
    activation_counts: Vec<Vec<u32>>,
    last_toggled_generation: Vec<Vec<u64>>,
    width: usize,
    height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            front: vec![vec![false; width]; height],
            back: vec![vec![false; width]; height],
            activation_counts: vec![vec![0; width]; height],
            last_toggled_generation: vec![vec![0; width]; height],
            width,
            height,
        }
    }

    pub fn get(&self, x: usize, y: usize) -> bool {
        if x < self.width && y < self.height {
            self.front[y][x]
        } else {
            false
        }
    }

    pub fn get_activation_count(&self, x: usize, y: usize) -> u32 {
        if x < self.width && y < self.height {
            self.activation_counts[y][x]
        } else {
            0
        }
    }

    pub fn get_last_toggled_generation(&self, x: usize, y: usize) -> u64 {
        if x < self.width && y < self.height {
            self.last_toggled_generation[y][x]
        } else {
            0
        }
    }

    pub fn set(&mut self, x: usize, y: usize, alive: bool, generation: u64) {
        if x < self.width && y < self.height {
            if alive && !self.front[y][x] {
                self.activation_counts[y][x] += 1;
                self.last_toggled_generation[y][x] = generation;
            }
            self.front[y][x] = alive;
        }
    }

    pub fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = ((x as i32 + dx + self.width as i32) % self.width as i32) as usize;
                let ny = ((y as i32 + dy + self.height as i32) % self.height as i32) as usize;

                if self.front[ny][nx] {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn set_back(&mut self, x: usize, y: usize, alive: bool, generation: u64) {
        if alive && !self.front[y][x] {
            self.activation_counts[y][x] += 1;
            self.last_toggled_generation[y][x] = generation;
        }
        self.back[y][x] = alive;
    }

    pub fn swap_buffers(&mut self) {
        std::mem::swap(&mut self.front, &mut self.back);
    }

    pub fn randomize(&mut self) {
        let mut rng = rand::rng();
        for y in 0..self.height {
            for x in 0..self.width {
                let alive = rng.random_bool(0.3);
                self.front[y][x] = alive;
                self.activation_counts[y][x] = alive.into();
                self.last_toggled_generation[y][x] = 0;
            }
        }
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.front[y][x] = false;
                self.activation_counts[y][x] = 0;
                self.last_toggled_generation[y][x] = 0;
            }
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn default_randomized() -> Self {
        let mut grid = Self::default();
        grid.randomize();
        grid
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(GRID_WIDTH, GRID_HEIGHT)
    }
}

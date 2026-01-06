use bevy::prelude::{Color, Resource, Timer, TimerMode, Vec, vec};
use rand::Rng;

pub const GRID_WIDTH: usize = 200;
pub const GRID_HEIGHT: usize = 200;
pub const CELL_SIZE: f32 = 8.0;
pub const DEAD_COLOR: Color = Color::srgb(0.1, 0.1, 0.1);
pub const MAX_ACTIVATION_COLOR_SCALE: u32 = 25;

pub fn cell_color(activation_count: u32) -> Color {
    let hue = (activation_count as f32 / MAX_ACTIVATION_COLOR_SCALE as f32) * 360.0;
    Color::hsl(hue % 360.0, 1.0, 0.5)
}

#[derive(Resource)]
pub struct Grid {
    front: Vec<Vec<bool>>,
    back: Vec<Vec<bool>>,
    activation_counts: Vec<Vec<u32>>,
    width: usize,
    height: usize,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            front: vec![vec![false; width]; height],
            back: vec![vec![false; width]; height],
            activation_counts: vec![vec![0; width]; height],
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

    pub fn set(&mut self, x: usize, y: usize, alive: bool) {
        if x < self.width && y < self.height {
            if alive && !self.front[y][x] {
                self.activation_counts[y][x] += 1;
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

    pub fn set_back(&mut self, x: usize, y: usize, alive: bool) {
        if alive && !self.front[y][x] {
            self.activation_counts[y][x] += 1;
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
            }
        }
    }

    pub fn clear(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.front[y][x] = false;
                self.activation_counts[y][x] = 0;
            }
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

impl Default for Grid {
    fn default() -> Self {
        Self::new(GRID_WIDTH, GRID_HEIGHT)
    }
}

#[derive(Resource)]
pub struct SimulationState {
    pub paused: bool,
    pub step_mode: bool,
    pub generation: u64,
    pub update_timer: Timer,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            paused: true,
            step_mode: false,
            generation: 0,
            update_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        }
    }
}

use std::collections::{HashMap, VecDeque};

use bevy::prelude::{Entity, Resource, Timer, TimerMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorPattern {
    #[default]
    ActivationCount,
    Binary,
    NeighborCount,
    PastelRainbow,
    Neon,
    Monochrome,
    Ocean,
    Fire,
    GenerationBased,
}

#[derive(Resource)]
pub struct SimulationState {
    pub paused: bool,
    pub step_mode: bool,
    pub generation: u64,
    pub update_timer: Timer,
    pub color_pattern: ColorPattern,
    pub pan_speed: f32,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self {
            paused: true,
            step_mode: false,
            generation: 0,
            update_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
            color_pattern: ColorPattern::default(),
            pan_speed: 300.0,
        }
    }
}

/// Represents a pending chunk operation to be processed over multiple frames
#[derive(Debug, Clone, Copy)]
pub enum ChunkOperation {
    /// Spawn a new chunk at the given coordinates
    Spawn(i32, i32),
    /// Recycle an existing chunk to new coordinates (entity, `new_x`, `new_y`)
    Recycle(Entity, i32, i32),
}

#[derive(Resource)]
pub struct ChunkManager {
    /// Currently active and visible chunks mapped by their coordinates
    pub active_chunks: HashMap<(i32, i32), Entity>,
    /// Pool of inactive chunk entities ready for reuse (avoids despawn/spawn overhead)
    pub chunk_pool: Vec<Entity>,
    /// Queue of chunk operations to process (staggered over multiple frames)
    pub pending_operations: VecDeque<ChunkOperation>,
    /// Maximum number of chunk operations to process per frame
    pub operations_per_frame: usize,
    /// The chunk coordinate the camera is currently centered on
    pub current_center_chunk: Option<(i32, i32)>,
    /// Maximum pool size to prevent unbounded memory growth
    pub max_pool_size: usize,
}

impl Default for ChunkManager {
    fn default() -> Self {
        Self {
            active_chunks: HashMap::new(),
            chunk_pool: Vec::with_capacity(16),
            pending_operations: VecDeque::new(),
            operations_per_frame: 2, // Process up to 2 chunks per frame for smooth loading
            current_center_chunk: None,
            max_pool_size: 16,
        }
    }
}

impl ChunkManager {
    /// Get a chunk entity from the pool, if available
    pub fn take_from_pool(&mut self) -> Option<Entity> {
        self.chunk_pool.pop()
    }

    /// Return a chunk entity to the pool for later reuse
    pub fn return_to_pool(&mut self, entity: Entity) {
        if self.chunk_pool.len() < self.max_pool_size {
            self.chunk_pool.push(entity);
        }
    }

    /// Queue a spawn operation for staggered processing
    pub fn queue_spawn(&mut self, chunk_x: i32, chunk_y: i32) {
        self.pending_operations
            .push_back(ChunkOperation::Spawn(chunk_x, chunk_y));
    }

    /// Queue a recycle operation (reuse existing entity at new position)
    pub fn queue_recycle(&mut self, entity: Entity, new_x: i32, new_y: i32) {
        self.pending_operations
            .push_back(ChunkOperation::Recycle(entity, new_x, new_y));
    }

    /// Check if there are pending operations
    pub fn has_pending_operations(&self) -> bool {
        !self.pending_operations.is_empty()
    }

    /// Take up to `operations_per_frame` operations from the queue
    pub fn take_pending_batch(&mut self) -> Vec<ChunkOperation> {
        let count = self.operations_per_frame.min(self.pending_operations.len());
        self.pending_operations.drain(..count).collect()
    }
}

use voxel_rs_common::{
    block::Block,
    registry::Registry,
    world::{Chunk, ChunkPos, WorldGenerator},
};
use voxel_rs_common::worker::{WorkerState, Worker};

static WORLDGEN_QUEUE_SIZE: usize = 20;

pub fn start_worldgen_worker(
    block_registry: Registry<Block>,
    world_generator: Box<dyn WorldGenerator + Send>
) -> WorldGenerationWorker {
    Worker::new(WorldGenerationState::new(block_registry, world_generator), WORLDGEN_QUEUE_SIZE, "Worldgen".into())
}

pub struct WorldGenerationState {
    block_registry: Registry<Block>,
    world_generator: Box<dyn WorldGenerator + Send>,
}

impl WorldGenerationState {
    pub(self) fn new(block_registry: Registry<Block>, world_generator: Box<dyn WorldGenerator + Send>) -> Self {
        Self {
            block_registry,
            world_generator,
        }
    }
}

impl WorkerState<ChunkPos, Chunk> for WorldGenerationState {
    fn compute(&mut self, pos: ChunkPos) -> Chunk {
        self.world_generator.generate_chunk(pos, &self.block_registry)
    }
}

pub type WorldGenerationWorker = Worker<ChunkPos, Chunk, WorldGenerationState>;

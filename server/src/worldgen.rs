use voxel_rs_common::{
    block::Block,
    registry::Registry,
    world::chunk::{Chunk, ChunkPos},
    world::WorldGenerator,
};
use voxel_rs_common::worker::{WorkerState, Worker};

pub struct WorldGenerationState {
    block_registry: Registry<Block>,
    world_generator: Box<dyn WorldGenerator + Send>,
}

impl WorldGenerationState {
    pub fn new(block_registry: Registry<Block>, world_generator: Box<dyn WorldGenerator + Send>) -> Self {
        Self {
            block_registry,
            world_generator,
        }
    }
}

impl WorkerState<(), Chunk> for WorldGenerationState {
    fn compute(&mut self, pos: ChunkPos, _: ()) -> Chunk {
        self.world_generator.generate_chunk(pos, &self.block_registry)
    }
}

pub type WorldGenerationWorker = Worker<(), Chunk, WorldGenerationState>;

use voxel_rs_common::light::{FastBFSQueue, compute_light};
use voxel_rs_common::world::{LightChunk, HighestOpaqueBlock};
use voxel_rs_common::world::chunk::{Chunk, ChunkPos, CHUNK_SIZE};
use std::sync::Arc;
use voxel_rs_common::worker::{Worker, WorkerState};
use voxel_rs_common::collections::zero_initialized_vec;

/// The chunk-specific data that is needed to generate light for it.
pub struct ChunkLightingData {
    pub chunks: Vec<Option<Arc<Chunk>>>,
    pub highest_opaque_blocks: Vec<Arc<HighestOpaqueBlock>>,
}

pub struct ChunkLightingState {
    queue_reuse: FastBFSQueue,
    light_data_reuse: Vec<u8>,
    opaque_reuse: Vec<bool>,
}

impl ChunkLightingState {
    pub fn new() -> Self {
        Self {
            queue_reuse: FastBFSQueue::new(),
            light_data_reuse: unsafe { zero_initialized_vec((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize) },
            opaque_reuse: unsafe { zero_initialized_vec((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize) },
        }
    }
}

impl WorkerState<ChunkLightingData, Arc<LightChunk>> for ChunkLightingState {
    fn compute(&mut self, pos: ChunkPos, data: ChunkLightingData) -> Arc<LightChunk> {
        Arc::new(LightChunk {
            light: compute_light(
                data.chunks,
                data.highest_opaque_blocks,
                &mut self.queue_reuse,
                &mut self.light_data_reuse,
                &mut self.opaque_reuse,
            ).light_level.to_vec(),
            pos,
        })
    }
}

pub type ChunkLightingWorker = Worker<ChunkLightingData, Arc<LightChunk>, ChunkLightingState>;

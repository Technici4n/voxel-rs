use voxel_rs_common::world::{Chunk, CHUNK_SIZE};
use std::sync::Arc;

mod sunlight;
pub mod worker;

/// This data structure contains the y position of the highest opaque block
#[derive(Clone)]
pub struct HighestOpaqueBlock {
    pub y: [i64; (CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl HighestOpaqueBlock {
    pub fn new() -> Self {
        Self {
            y: [i64::MIN; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }

    pub fn from_chunk(chunk: &Arc<Chunk>) -> Self {
        let mut hob = Self {
            y: [i64::MIN; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        };
        for i in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                for j in (0..CHUNK_SIZE).rev() {
                    // TODO: use BlockRegistry
                    if chunk.get_block_at((i, j, k)) != 0 {
                        hob.y[(i*CHUNK_SIZE + k) as usize] = j as i64 + chunk.pos.py * CHUNK_SIZE as i64;
                        break;
                    }
                }
            }
        }
        hob
    }

    /// Merge with other HighestOpaqueBlock
    pub fn merge(&mut self, other: &HighestOpaqueBlock) {
        for i in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                let idx = (i*CHUNK_SIZE + k) as usize;
                self.y[idx] = Ord::max(self.y[idx], other.y[idx]);
            }
        }
    }
}

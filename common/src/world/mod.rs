use self::chunk::{Chunk, ChunkPos, CHUNK_SIZE};
use crate::{
    block::{Block, BlockId},
    registry::Registry,
};
use std::collections::HashMap;

pub mod chunk;

/// The position of a block in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockPos {
    pub px: i64,
    pub py: i64,
    pub pz: i64,
}

impl BlockPos {
    pub fn containing_chunk_pos(self) -> ChunkPos {
        ChunkPos {
            px: self.px.div_euclid(CHUNK_SIZE as i64),
            py: self.py.div_euclid(CHUNK_SIZE as i64),
            pz: self.pz.div_euclid(CHUNK_SIZE as i64),
        }
    }

    pub fn pos_in_containing_chunk(self) -> (u32, u32, u32) {
        (
            self.px.rem_euclid(CHUNK_SIZE as i64) as u32,
            self.py.rem_euclid(CHUNK_SIZE as i64) as u32,
            self.pz.rem_euclid(CHUNK_SIZE as i64) as u32,
        )
    }
}

impl From<(i64, i64, i64)> for BlockPos {
    fn from((px, py, pz): (i64, i64, i64)) -> Self {
        Self { px, py, pz }
    }
}

impl From<(f64, f64, f64)> for BlockPos {
    fn from((px, py, pz): (f64, f64, f64)) -> Self {
        Self {
            px: px.floor() as i64,
            py: py.floor() as i64,
            pz: pz.floor() as i64,
        }
    }
}

/// A game world
pub struct World {
    pub chunks: HashMap<ChunkPos, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    // TODO : Save the chunk
    /// Remove the chunk from the world
    pub fn drop_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
    }

    /// Return a reference to the chunk if it exists, None otherwise
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&pos)
    }

    /// Return a mutable reference to the chunk if it exists and None otherwise
    pub fn _get_chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk> {
        self.chunks.get_mut(&pos)
    }

    /// Return true if there exists a chunk at `pos`
    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        return self.chunks.contains_key(&pos);
    }

    /// Return block at position `pos` in the world. 0 is returned if the chunk does not exists/is not loaded
    pub fn get_block(&self, pos: BlockPos) -> BlockId {
        match self.get_chunk(pos.containing_chunk_pos()) {
            None => 0,
            Some(chunk) => chunk.get_block_at(pos.pos_in_containing_chunk()),
        }
    }

    /// Set block at position `pos`
    /// Create a new empty chunk if necessary
    pub fn set_block(&mut self, pos: BlockPos, block: BlockId) {
        self.get_add_chunk(pos.containing_chunk_pos())
            .set_block_at(pos.pos_in_containing_chunk(), block);
    }

    /// Create a new chunk at position `pos` if not already present
    /// Anyway, return the a mutable reference to the chunk created or existing
    pub fn get_add_chunk(&mut self, pos: ChunkPos) -> &mut Chunk {
        self.chunks.entry(pos).or_insert_with(|| Chunk::new(pos))
    }

    /// Set the chunk at position `pos`
    pub fn set_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.pos, chunk);
    }
}

/// A world generator
pub trait WorldGenerator {
    /// Generate the chunk at position `pos`. The result must always be the same,
    /// independently of the previous calls to this function!
    fn generate_chunk(&mut self, pos: ChunkPos, block_registry: &Registry<Block>) -> Chunk;
}

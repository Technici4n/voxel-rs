use self::chunk::{Chunk, ChunkPos, CHUNK_SIZE};
use crate::light::{compute_light, FastBFSQueue};
use crate::world::chunk::ChunkPosXZ;
use crate::{
    block::{Block, BlockId},
    registry::Registry,
};
use nalgebra::Vector3;
use std::collections::HashMap;
use std::sync::Arc;

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

impl From<Vector3<f64>> for BlockPos {
    fn from(vec: Vector3<f64>) -> Self {
        Self {
            px: vec[0].floor() as i64,
            py: vec[1].floor() as i64,
            pz: vec[2].floor() as i64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LightChunk {
    pub light: Vec<u8>,
    pub pos: ChunkPos,
}

impl LightChunk {
    pub fn new(pos: ChunkPos) -> Self {
        let mut light = Vec::new();
        light.resize((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize, 15);
        Self { light, pos }
    }

    /// Get light at some position
    #[inline(always)]
    pub fn get_light_at(&self, (px, py, pz): (u32, u32, u32)) -> u8 {
        self.light[(px * CHUNK_SIZE * CHUNK_SIZE + py * CHUNK_SIZE + pz) as usize]
    }
}

/// An RLE-compressed chunk
#[derive(Debug, Clone)]
pub struct CompressedLightChunk {
    pub pos: ChunkPos,
    pub data: Vec<(u16, u8)>,
}

impl CompressedLightChunk {
    /// Compress `chunk` using RLE
    pub fn from_chunk(chunk: &LightChunk) -> Self {
        let mut compressed_data = Vec::new();
        let mut current_block = chunk.light[0];
        let mut current_block_count = 0;
        for i in 0..(CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize {
            if chunk.light[i] != current_block {
                compressed_data.push((current_block_count, current_block));
                current_block = chunk.light[i];
                current_block_count = 0;
            }
            current_block_count += 1;
        }

        compressed_data.push((current_block_count, current_block));

        Self {
            pos: chunk.pos,
            data: compressed_data,
        }
    }

    /// Recover original chunk
    pub fn to_chunk(&self) -> LightChunk {
        let mut light = Vec::new();
        light.resize((CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize, 0);

        let mut i = 0;
        for &(len, block) in self.data.iter() {
            for j in 0..len {
                light[(i + j) as usize] = block;
            }
            i += len;
        }

        LightChunk {
            pos: self.pos,
            light,
        }
    }
}

/// A game world
pub struct World {
    pub chunks: HashMap<ChunkPos, Arc<Chunk>>,
    pub light: HashMap<ChunkPos, Arc<LightChunk>>,
    pub highest_opaque_block: HashMap<ChunkPosXZ, HighestOpaqueBlock>,
}

/// This data structure contains the y position of the highest opaque block
#[derive(Clone)]
pub struct HighestOpaqueBlock {
    pub pos: ChunkPosXZ,
    pub y: [i64; (CHUNK_SIZE * CHUNK_SIZE) as usize],
}

impl HighestOpaqueBlock {
    pub fn new(pos: ChunkPosXZ) -> Self {
        Self {
            pos,
            y: [0; (CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

impl World {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            light: HashMap::new(),
            highest_opaque_block: HashMap::new(),
        }
    }

    // TODO : Save the chunk
    /// Remove the chunk from the world
    pub fn drop_chunk(&mut self, pos: ChunkPos) {
        self.chunks.remove(&pos);
        self.light.remove(&pos);
    }

    /// Return a reference to the chunk if it exists, None otherwise
    pub fn get_chunk(&self, pos: ChunkPos) -> Option<Arc<Chunk>> {
        self.chunks.get(&pos).cloned()
    }

    /// Return a reference to the light chunk if it exists, None otherwise
    pub fn get_light_chunk(&self, pos: ChunkPos) -> Option<Arc<LightChunk>> {
        self.light.get(&pos).cloned()
    }

    /// Return true if there exists a chunk at `pos`
    pub fn has_chunk(&self, pos: ChunkPos) -> bool {
        self.chunks.contains_key(&pos)
    }

    /// Return true if there exists a light chunk at `pos`
    pub fn has_light_chunk(&self, pos: ChunkPos) -> bool {
        self.light.contains_key(&pos)
    }

    /// Return block at position `pos` in the world. 0 is returned if the chunk does not exists/is not loaded
    pub fn get_block(&self, pos: BlockPos) -> BlockId {
        match self.get_chunk(pos.containing_chunk_pos()) {
            None => 0,
            Some(chunk) => chunk.get_block_at(pos.pos_in_containing_chunk()),
        }
    }

    /// Create a new light chunk at position `pos` if not already present
    /// Anyway, return the a mutable reference to the chunk created or existing
    pub fn get_add_light_chunk(&mut self, pos: ChunkPos) -> Arc<LightChunk> {
        // TODO : remove this
        self.light
            .entry(pos)
            .or_insert_with(|| Arc::new(LightChunk::new(pos)))
            .clone()
    }

    /// Set the chunk at position `pos`
    pub fn set_chunk(&mut self, chunk: Chunk) {
        self.chunks.insert(chunk.pos, Arc::new(chunk));
    }

    /// Set the light chunk at position `pos`
    pub fn set_light_chunk(&mut self, chunk: LightChunk) {
        self.light.insert(chunk.pos, Arc::new(chunk));
    }

    /// Function to be called when updating a chunk to update highest
    /// Return if they must have a large light update over the 3x3 chunk column
    pub fn update_highest_opaque_block(&mut self, chunk_pos: ChunkPos) -> bool {
        let chunk_pos_xz: ChunkPosXZ = chunk_pos.clone().into();
        let mut highest_opaque_block = self
            .highest_opaque_block
            .entry(chunk_pos_xz)
            .or_insert_with(|| HighestOpaqueBlock::new(chunk_pos_xz))
            .clone();
        let mut check = false;
        let mut scan_all_chunk = false;

        {
            let chunk_opt = self.get_chunk(chunk_pos);

            match chunk_opt {
                None => return false, // no chunk at update position
                Some(chunk) => {
                    'ij_loop: for i in 0..CHUNK_SIZE {
                        for j in 0..CHUNK_SIZE {
                            if highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize]
                                < (chunk_pos.py + 1) * CHUNK_SIZE as i64
                            {
                                check = true;
                                break 'ij_loop;
                            }
                        }
                    }

                    if check {
                        // if the chunks is note entirely below the highest opaque block
                        for i in 0..CHUNK_SIZE {
                            for j in 0..CHUNK_SIZE {
                                if highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize]
                                    < (chunk_pos.py + 1) * CHUNK_SIZE as i64
                                {
                                    let mut new_max_in_the_chunk = false;
                                    for y in (0..CHUNK_SIZE).rev() {
                                        if chunk.get_block_at((i, y, j)) != 0 {
                                            // TODO : Replace by is opaque
                                            highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize] =
                                                chunk_pos.py * CHUNK_SIZE as i64 + y as i64;
                                            new_max_in_the_chunk = true;
                                            break;
                                        }
                                    }
                                    // if the old max was in the chunk but not the new one
                                    if !new_max_in_the_chunk
                                        && highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize]
                                            >= (chunk_pos.py) * CHUNK_SIZE as i64
                                    {
                                        scan_all_chunk = true;
                                        highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize] = 0;
                                        // default value
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if scan_all_chunk {
            // we must scan the whole chunk column
            for other_chunk in self.chunks.values() {
                if other_chunk.pos.px == chunk_pos.px
                    && other_chunk.pos.py < chunk_pos.py
                    && other_chunk.pos.pz == chunk_pos.pz
                {
                    for i in 0..CHUNK_SIZE {
                        for j in 0..CHUNK_SIZE {
                            if highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize]
                                < (other_chunk.pos.py + 1) * CHUNK_SIZE as i64
                            {
                                for y in CHUNK_SIZE..=0 {
                                    if other_chunk.get_block_at((i, y, j)) != 0 {
                                        // TODO : Replace by is opaque
                                        highest_opaque_block.y[(i * CHUNK_SIZE + j) as usize] =
                                            other_chunk.pos.py * CHUNK_SIZE as i64 + y as i64;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        self.highest_opaque_block
            .insert(chunk_pos_xz, highest_opaque_block);
        return true;
    }

    /// Update the light of a chunk at chunkPos
    /// Return true if they has been an update
    pub fn update_light(
        &mut self,
        pos: &ChunkPos,
        bfs_queue: &mut FastBFSQueue,
        light_data: &mut [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize],
        opaque: &mut [bool; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize]
    ) -> bool {
        if self.chunks.contains_key(&pos) {
            let light = {
                let mut vec_chunk: Vec<Option<Arc<Chunk>>> = Vec::new();
                let mut vec_highest_opaque_block: Vec<HighestOpaqueBlock> = Vec::new();

                // Creating the datastructure to be sent to the lightning algorithm
                for i in -1..=1 {
                    for k in -1..=1 {
                        let pos_act = pos.offset(i, 0, k);
                        let pos_xz: ChunkPosXZ = pos_act.into();
                        vec_highest_opaque_block.push(
                            (*self
                                .highest_opaque_block
                                .entry(pos_xz)
                                .or_insert_with(|| HighestOpaqueBlock::new(pos_xz)))
                            .clone(),
                        );
                    }
                }

                for i in -1..=1 {
                    for j in -1..=1 {
                        for k in -1..=1 {
                            let pos_act = pos.offset(i, j, k);
                            vec_chunk.push(self.get_chunk(pos_act));
                        }
                    }
                }
                compute_light(vec_chunk, vec_highest_opaque_block, bfs_queue, light_data, opaque).light_level
            };

            // updating the light
            self.light.insert(
                *pos,
                Arc::new(LightChunk {
                    light: light.to_vec(),
                    pos: *pos,
                }),
            );

            true
        } else {
            false
        }
    }
}

/// A world generator
pub trait WorldGenerator {
    /// Generate the chunk at position `pos`. The result must always be the same,
    /// independently of the previous calls to this function!
    fn generate_chunk(&mut self, pos: ChunkPos, block_registry: &Registry<Block>) -> Chunk;
}

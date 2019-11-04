use crate::{block::Block, perlin::perlin, registry::Registry};

/// Number of blocks of data in a chunk axis
pub const CHUNK_LEN: u32 = 16;
/// Number of data in a bloc axis
const GROUP_LEN: u32 = 2;
/// size of an axis of chunk (number of data)
pub const CHUNK_SIZE: u32 = GROUP_LEN * CHUNK_LEN;
/// number of data in a block
const BLOCK_SIZE: usize = (GROUP_LEN * GROUP_LEN * GROUP_LEN) as usize;

#[derive(Clone)]
pub(super) enum BlockGroup {
    Compressed(u16, u16, u16, u16), // (x, y), (x, Y), (X, y), (X, Y)
    // 1 bit (NxNxN) times the same data
    Uncompressed(Box<[u16; BLOCK_SIZE]>), // different data
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
/// position of the chunk in the world
pub struct ChunkPos {
    pub px: i64,
    pub py: i64,
    pub pz: i64,
}

#[derive(Clone)]
pub struct Chunk {
    pub pos: ChunkPos,
    pub(super) data: Vec<BlockGroup>, // data containde in the chunk
}

impl Chunk {
    pub fn new(x: i64, y: i64, z: i64) -> Chunk {
        Chunk {
            pos: ChunkPos {
                px: x,
                py: y,
                pz: z,
            },
            data: vec![
                BlockGroup::Compressed(0, 0, 0, 0);
                (CHUNK_LEN * CHUNK_LEN * CHUNK_LEN) as usize
            ],
            // chunk is empty
        }
    }

    pub fn get_data(&self, px: u32, py: u32, pz: u32) -> u16 {
        match &self.data[((px / GROUP_LEN) * CHUNK_LEN * CHUNK_LEN
            + (py / GROUP_LEN) * CHUNK_LEN
            + (pz / GROUP_LEN)) as usize]
        {
            BlockGroup::Compressed(bxz, bxzz, bxxz, bxxzz) => match (px % 2) * 2 + pz % 2 {
                0 => *bxz,
                1 => *bxzz,
                2 => *bxxz,
                3 => *bxxzz,
                _ => unreachable!(),
            }, // if compressed return the compressed type
            BlockGroup::Uncompressed(blocks) => {
                blocks[((px % GROUP_LEN) * 4 + (py % GROUP_LEN) * 2 + (pz % GROUP_LEN)) as usize]
            } // if not compressed, return the data stored in the full array
        }
    }

    pub fn set_data(&mut self, px: u32, py: u32, pz: u32, data: u16) {
        let x = &mut self.data[((px / GROUP_LEN) * CHUNK_LEN * CHUNK_LEN
            + (py / GROUP_LEN) * CHUNK_LEN
            + (pz / GROUP_LEN)) as usize];

        if let BlockGroup::Compressed(bxz, bxzz, bxxz, bxxzz) = x {
            let btype = match (px % 2) * 2 + pz % 2 {
                0 => *bxz,
                1 => *bxzz,
                2 => *bxxz,
                3 => *bxxzz,
                _ => unreachable!(),
            };
            if btype != data {
                // splitting the group into an new array
                let mut fill = [0; BLOCK_SIZE];

                // hardcoded for GROUP_LEN = 2
                fill[0] = *bxz;
                fill[2] = *bxz;
                fill[1] = *bxzz;
                fill[3] = *bxzz;
                fill[4] = *bxxz;
                fill[6] = *bxxz;
                fill[5] = *bxxzz;
                fill[7] = *bxxzz;

                fill[((px % GROUP_LEN) * GROUP_LEN * GROUP_LEN
                    + (py % GROUP_LEN) * GROUP_LEN
                    + (pz % GROUP_LEN)) as usize] = data;
                *x = BlockGroup::Uncompressed(Box::new(fill));
            }
        } else if let BlockGroup::Uncompressed(blocks) = x {
            blocks[((px % GROUP_LEN) * GROUP_LEN * GROUP_LEN
                + (py % GROUP_LEN) * GROUP_LEN
                + (pz % GROUP_LEN)) as usize] = data;

            if blocks[0] != blocks[2] {
                return;
            }
            if blocks[1] != blocks[3] {
                return;
            }
            if blocks[4] != blocks[6] {
                return;
            }
            if blocks[7] != blocks[5] {
                return;
            }
            *x = BlockGroup::Compressed(blocks[0], blocks[1], blocks[4], blocks[5]);
            // merging all block in four columns
        }
    }

    /// Fill the chunk with perlin noise
    pub fn fill_perlin(&mut self, block_registry: &Registry<Block>) {
        let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
        let px = (self.pos.px * CHUNK_SIZE as i64) as f32;
        let py = (self.pos.py * CHUNK_SIZE as i64) as f32;
        let pz = (self.pos.pz * CHUNK_SIZE as i64) as f32;
        let freq = 1.0 / 32.0;

        let s = (CHUNK_SIZE + 3) as usize;

        let noise = perlin(px, py, pz, s, freq, 4, 0.4, 42);

        for i in 0..32 {
            for j in 0..32 {
                for k in 0..32 {
                    // warning : indexing order
                    if noise[(k * s * s + j * s + i) as usize] > (py + j as f32 + 10.0) / 110.0
                    {
                        if noise[(k * s * s + (j+1) * s + i) as usize] > (py + j as f32 + 11.0) / 110.0{
                            if noise[(k * s * s + (j+2) * s + i) as usize] > (py + j as f32 + 12.0) / 110.0
                                && noise[(k * s * s + (j+3) * s + i) as usize] > (py + j as f32 + 13.0) / 110.0{
                                self.set_data(i as u32, j as u32, k as u32, stone_block);
                            }else{
                                self.set_data(i as u32, j as u32, k as u32, dirt_block);
                            }
                        }else{
                            self.set_data(i as u32, j as u32, k as u32, grass_block);
                        }
                    }
                }
            }
        }
    }
}

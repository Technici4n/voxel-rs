use crate::perlin::perlin;

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
pub struct ChunkPos {
    pub px: i64,
    // position of the chunkc in the world
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
            BlockGroup::Compressed(bxz, bxZ, bXz, bXZ) => match (px % 2) * 2 + pz % 2 {
                0 => *bxz,
                1 => *bxZ,
                2 => *bXz,
                3 => *bXZ,
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

        if let BlockGroup::Compressed(bxz, bxZ, bXz, bXZ) = x {
            let btype = match (px % 2) * 2 + pz % 2 {
                0 => *bxz,
                1 => *bxZ,
                2 => *bXz,
                3 => *bXZ,
                _ => unreachable!(),
            };
            if btype != data {
                // splitting the group into an new array
                let mut fill = [0; BLOCK_SIZE];

                // hardcoded for GROUP_LEN = 2
                fill[0] = *bxz;
                fill[2] = *bxz;
                fill[1] = *bxZ;
                fill[3] = *bxZ;
                fill[4] = *bXz;
                fill[6] = *bXz;
                fill[5] = *bXZ;
                fill[7] = *bXZ;

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
    pub fn fill_perlin(&mut self) {
        let px = (self.pos.px * CHUNK_SIZE as i64) as f32;
        let py = (self.pos.py * CHUNK_SIZE as i64) as f32;
        let pz = (self.pos.pz * CHUNK_SIZE as i64) as f32;
        let freq = 1.0 / 32.0;
        let noise = perlin(px, py, pz, CHUNK_SIZE as usize, freq, 4, 0.4, 42);

        for i in 0..32 {
            for j in 0..32 {
                for k in 0..32 {
                    if noise[(k * 32 * 32 + j * 32 + i) as usize] > 0.5
                    // warning : indexing order
                    {
                        self.set_data(i, j, k, 1);
                    }
                }
            }
        }
    }
}

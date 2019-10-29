/// Number of blocks of data in a chunk axis
const CHUNK_LEN: u32 = 16;
/// Number of data in a bloc axis
const GROUP_LEN: u32 = 2;
/// size of an axis of chunk (number of data)
pub const CHUNK_SIZE: u32 = GROUP_LEN * CHUNK_LEN;
/// number of data in a block
const BLOCK_SIZE: usize = (GROUP_LEN * GROUP_LEN * GROUP_LEN) as usize;

use crate::perlin::perlin;

#[derive(Clone)]
enum BlockGroup {
    Compressed(u32),                      // 1 bit (NxNxN) times the same data
    Uncompressed(Box<[u32; BLOCK_SIZE]>), // different data
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct ChunkPos {
    pub px: i64, // position of the chunkc in the world
    pub py: i64,
    pub pz: i64,
}

#[derive(Clone)]
pub struct Chunk {
    pub pos: ChunkPos,
    data: Vec<BlockGroup>, // data containde in the chunk
}

impl Chunk {
    pub fn new(x: i64, y: i64, z: i64) -> Chunk {
        Chunk {
            pos: ChunkPos {
                px: x,
                py: y,
                pz: z,
            },
            data: vec![BlockGroup::Compressed(0); (CHUNK_LEN * CHUNK_LEN * CHUNK_LEN) as usize],
            // chunk is empty
        }
    }

    pub fn get_data(&self, px: u32, py: u32, pz: u32) -> u32 {
        match &self.data[((px / GROUP_LEN) * CHUNK_LEN * CHUNK_LEN
            + (py / GROUP_LEN) * CHUNK_LEN
            + (pz / GROUP_LEN)) as usize]
        {
            BlockGroup::Compressed(block_type) => *block_type, // if compressed return the compressed type
            BlockGroup::Uncompressed(blocks) => {
                blocks[((px % GROUP_LEN) * 4 + (py % GROUP_LEN) * 2 + (pz % GROUP_LEN)) as usize]
            } // if not compressed, return the data stored in the full array
        }
    }

    pub fn set_data(&mut self, px: u32, py: u32, pz: u32, data: u32) {
        let x = &mut self.data[((px / GROUP_LEN) * CHUNK_LEN * CHUNK_LEN
            + (py / GROUP_LEN) * CHUNK_LEN
            + (pz / GROUP_LEN)) as usize];

        if let BlockGroup::Compressed(block_type) = x {
            if *block_type != data {
                // splitting the group into an new array
                let mut fill = [*block_type; BLOCK_SIZE];
                fill[((px % GROUP_LEN) * GROUP_LEN * GROUP_LEN
                    + (py % GROUP_LEN) * GROUP_LEN
                    + (pz % GROUP_LEN)) as usize] = data;
                *x = BlockGroup::Uncompressed(Box::new(fill));
            }
        } else if let BlockGroup::Uncompressed(blocks) = x {
            blocks[((px % GROUP_LEN) * GROUP_LEN * GROUP_LEN
                + (py % GROUP_LEN) * GROUP_LEN
                + (pz % GROUP_LEN)) as usize] = data;
            for i in 0..BLOCK_SIZE {
                // if all the data in the array are the same -> merge
                if blocks[i] != data {
                    return;
                }
            }
            *x = BlockGroup::Compressed(data); // mergin all block in one
        }
    }

    /// Fill the chunk with perlin noise
    pub fn fill_perlin(&mut self) {
        for i in 0..32 {
            for j in 0..32 {
                for k in 0..32 {
                    if perlin(
                        (i as f64 + (self.pos.px * CHUNK_SIZE as i64) as f64) / 16.0,
                        (j as f64 + (self.pos.py * CHUNK_SIZE as i64) as f64) / 16.0,
                        (k as f64 + (self.pos.pz * CHUNK_SIZE as i64) as f64) / 16.0,
                        7,
                        0.4,
                        42,
                    ) > 0.5
                    {
                        self.set_data(i, j, k, 1);
                    }
                }
            }
        }
    }
}

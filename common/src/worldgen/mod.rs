use crate::block::Block;
use crate::registry::Registry;
use crate::world::chunk::{Chunk, ChunkPos, CHUNK_SIZE};
use crate::world::WorldGenerator;

pub mod perlin;

pub struct DefaultWorldGenerator;

impl WorldGenerator for DefaultWorldGenerator {
    fn generate_chunk(&mut self, pos: ChunkPos, block_registry: &Registry<Block>) -> Chunk {
        let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
        let px = (pos.px * CHUNK_SIZE as i64) as f32;
        let py = (pos.py * CHUNK_SIZE as i64) as f32;
        let pz = (pos.pz * CHUNK_SIZE as i64) as f32;
        let freq = 1.0 / 64.0;

        let s = (CHUNK_SIZE + 3) as usize;

        let noise = perlin::perlin(px, py, pz, s, freq, freq * 2.0, freq, 5, 0.4, 42);

        let mut chunk = Chunk::new(pos);

        for i in 0..32 {
            for j in 0..32 {
                for k in 0..32 {
                    // warning : indexing order
                    if noise[(i * s * s + j * s + k) as usize] > (py + j as f32 + 10.0) / 110.0 {
                        if noise[(i * s * s + (j + 1) * s + k) as usize]
                            > (py + j as f32 + 11.0) / 110.0
                        {
                            if noise[(i * s * s + (j + 2) * s + k) as usize]
                                > (py + j as f32 + 12.0) / 110.0
                                && noise[(i * s * s + (j + 3) * s + k) as usize]
                                    > (py + j as f32 + 13.0) / 110.0
                            {
                                chunk.set_block_at((i as u32, j as u32, k as u32), stone_block);
                            } else {
                                chunk.set_block_at((i as u32, j as u32, k as u32), dirt_block);
                            }
                        } else {
                            chunk.set_block_at((i as u32, j as u32, k as u32), grass_block);
                        }
                    }
                }
            }
        }
        chunk
    }
}

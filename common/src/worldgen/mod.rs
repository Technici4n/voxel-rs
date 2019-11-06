use std::collections::HashMap;

use crate::{
    block::Block,
    registry::Registry,
    world::chunk::{Chunk, CHUNK_SIZE, ChunkPos},
    world::WorldGenerator,
};
use crate::world::BlockPos;
use crate::worldgen::perlin::rand_pos_int;

pub mod perlin;

pub struct DefaultWorldGenerator {
    pregenerated_chunks: HashMap<ChunkPos, Chunk>,
}

struct BlockToPlace {
    pub pos: BlockPos,
    pub id: u16,
}

impl BlockToPlace {
    pub fn new((x, y, z): (i64, i64, i64), id: u16) -> Self {
        Self { pos: BlockPos::from((x, y, z)), id }
    }
}

impl DefaultWorldGenerator {
    pub fn new() -> Self {
        Self {
            pregenerated_chunks: HashMap::new(),
        }
    }

    fn pregenerate_chunk(chunk: &mut Chunk, block_registry: &Registry<Block>) {
        let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
        let px = (chunk.pos.px * CHUNK_SIZE as i64) as f32;
        let py = (chunk.pos.py * CHUNK_SIZE as i64) as f32;
        let pz = (chunk.pos.pz * CHUNK_SIZE as i64) as f32;
        let freq = 1.0 / 64.0;

        let s = (CHUNK_SIZE + 3) as usize;

        let noise = perlin::perlin(px, py, pz, s, freq, freq * 2.0, freq, 5, 0.4, 42);


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
    }
}

impl WorldGenerator for DefaultWorldGenerator {
    fn generate_chunk(&mut self, pos: ChunkPos, block_registry: &Registry<Block>) -> Chunk {
        let mut chunks_vec = Vec::new();

        for i in -1..=1 {
            for j in -1..=1 {
                for k in -1..=1 {
                    chunks_vec.push(
                        match self.pregenerated_chunks.remove(&pos.offset(i, j, k)) {
                            Some(chunk) => chunk,
                            None => {
                                let mut chunk = Chunk::new(pos.offset(i, j, k));
                                DefaultWorldGenerator::pregenerate_chunk(&mut chunk, &block_registry);
                                chunk
                            }
                        });
                }
            }
        }


        self.decorate_chunk(&mut chunks_vec, &block_registry);

        for chunk in chunks_vec.drain(..) {
            self.pregenerated_chunks.insert(chunk.pos, chunk);
        }

        self.pregenerated_chunks.remove(&pos).unwrap()
    }

    fn decorate_chunk(&mut self, chunks: &mut Vec<Chunk>, block_registry: &Registry<Block>) {
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let leaves_block = block_registry.get_id_by_name(&"leaves".to_owned()).unwrap() as u16;
        let wood_block = block_registry.get_id_by_name(&"wood".to_owned()).unwrap() as u16;

        let mut blocks_to_place: Vec<BlockToPlace> = Vec::new();
        {
            let minX = chunks[0].pos.px * CHUNK_SIZE as i64;
            let maxX = (chunks[0].pos.px + 3) * CHUNK_SIZE as i64;
            let minY = chunks[0].pos.py * CHUNK_SIZE as i64;
            let maxY = (chunks[0].pos.py + 3) * CHUNK_SIZE as i64;
            let minZ = chunks[0].pos.pz * CHUNK_SIZE as i64;
            let maxZ = (chunks[0].pos.pz + 3) * CHUNK_SIZE as i64;

            let chunk_size_64 = CHUNK_SIZE as i64;

            for i in -1..=1 {
                for j in -1..=1 {
                    for k in -1..=1 {
                        for l in 0..128 {
                            let current_chunk = &chunks[((i + 1) * 9 + (j + 1) * 3 + (k + 1)) as usize];
                            let cc_pos = current_chunk.pos;
                            let cbx = cc_pos.px * chunk_size_64;
                            let cby = cc_pos.py * chunk_size_64;
                            let cbz = cc_pos.pz * chunk_size_64;

                            let mut tx = rand_pos_int(cc_pos.px as i32, cc_pos.py as i32, cc_pos.pz as i32, 3 * l) as i64;
                            let mut ty = rand_pos_int(cc_pos.px as i32, cc_pos.py as i32, cc_pos.pz as i32, 3 * l + 1) as i64;
                            let mut tz = rand_pos_int(cc_pos.px as i32, cc_pos.py as i32, cc_pos.pz as i32, 3 * l + 2) as i64;

                            tx = (tx % chunk_size_64 + chunk_size_64) % chunk_size_64;
                            ty = (ty % chunk_size_64 + chunk_size_64) % chunk_size_64;
                            tz = (tz % chunk_size_64 + chunk_size_64) % chunk_size_64;


                            if current_chunk.get_block_at((tx as u32, ty as u32, tz as u32)) == grass_block {
                                tx += cbx;
                                ty += cby;
                                tz += cbz;


                                let mut blocks_to_place_one: Vec<BlockToPlace> = Vec::new();

                                // generating the trees
                                for jj in 1..8 {
                                    let nl;
                                    if jj <= 2 {
                                        nl = 0;
                                    } else if jj > 2 && jj <= 5 {
                                        nl = 2;
                                    } else {
                                        nl = 1;
                                    }

                                    for ii in -nl..=nl {
                                        for kk in -nl..=nl {
                                            let pos = (tx + ii, ty + jj, tz + kk);
                                            if ii != 0 || kk != 0 {
                                                blocks_to_place_one.push(BlockToPlace::new(pos, leaves_block));
                                            } else {
                                                if jj <= 6 {
                                                    blocks_to_place_one.push(BlockToPlace::new(pos, wood_block));
                                                } else {
                                                    blocks_to_place_one.push(BlockToPlace::new(pos, leaves_block));
                                                }
                                            }
                                        }
                                    }
                                }


                                let mut place = true;

                                for blocks in blocks_to_place_one.iter() {
                                    if blocks.pos.px >= minX && blocks.pos.px < maxX && blocks.pos.py >= minY && blocks.pos.py < maxY && blocks.pos.pz >= minZ && blocks.pos.pz < maxZ {
                                        let cblock_pos = blocks.pos.containing_chunk_pos();
                                        let (x, y, z) = (cblock_pos.px - chunks[0].pos.px, cblock_pos.py - chunks[0].pos.py, cblock_pos.pz - chunks[0].pos.pz);
                                        let chunk = &chunks[(x * 9 + y * 3 + z) as usize];
                                        let (ux, uy, uz) = blocks.pos.pos_in_containing_chunk();
                                        if chunk.get_block_at((ux, uy, uz)) != 0 {
                                            place = false;
                                            break;
                                        }
                                    } else {
                                        place = false;
                                        break;
                                    }
                                }
                                if place {
                                    for blocks in blocks_to_place_one.drain(..) {
                                        blocks_to_place.push(blocks);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for blocks in blocks_to_place.drain(..) {
            let minX = (chunks[0].pos.px + 1) * CHUNK_SIZE as i64;
            let maxX = (chunks[0].pos.px + 2) * CHUNK_SIZE as i64;
            let minY = (chunks[0].pos.py + 1) * CHUNK_SIZE as i64;
            let maxY = (chunks[0].pos.py + 2) * CHUNK_SIZE as i64;
            let minZ = (chunks[0].pos.pz + 1) * CHUNK_SIZE as i64;
            let maxZ = (chunks[0].pos.pz + 2) * CHUNK_SIZE as i64;
            if blocks.pos.px >= minX && blocks.pos.px < maxX && blocks.pos.py >= minY && blocks.pos.py < maxY && blocks.pos.pz >= minZ && blocks.pos.pz < maxZ {
                let pos = blocks.pos.pos_in_containing_chunk();
                chunks[13].set_block_at(pos, blocks.id);
            }
        }
    }
}

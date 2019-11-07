use std::collections::HashMap;
use std::time::Instant;

use crate::world::BlockPos;
use crate::worldgen::perlin::rand_pos_int;
use crate::{
    block::Block,
    registry::Registry,
    world::chunk::{Chunk, ChunkPos, CHUNK_SIZE},
    world::WorldGenerator,
};

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
        Self {
            pos: BlockPos::from((x, y, z)),
            id,
        }
    }
}

impl DefaultWorldGenerator {
    pub fn new() -> Self {
        Self {
            pregenerated_chunks: HashMap::new(),
        }
    }

    fn pregenerate_chunk(chunk: &mut Chunk, block_registry: &Registry<Block>) {
        let t1 = Instant::now();
        let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
        let px = (chunk.pos.px * CHUNK_SIZE as i64) as f32;
        let py = (chunk.pos.py * CHUNK_SIZE as i64) as f32;
        let pz = (chunk.pos.pz * CHUNK_SIZE as i64) as f32;
        let freq = 1.0 / 64.0;

        if py > 100.0 {
            return;
        } else if py + CHUNK_SIZE as f32 + 13.0 < 0.0 {
            for i in 0..32 {
                for j in 0..32 {
                    for k in 0..32 {
                        chunk.set_block_at((i as u32, j as u32, k as u32), stone_block);
                    }
                }
            }
            return;
        }

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
        let t2 = Instant::now();
        println!("Time to generate chunk : {} ms", (t2 - t1).subsec_millis());
    }
}

impl WorldGenerator for DefaultWorldGenerator {
    fn generate_chunk(&mut self, pos: ChunkPos, block_registry: &Registry<Block>) -> Chunk {
        let mut chunks_vec = Vec::new();

        let mut vec_to_drop: Vec<ChunkPos> = Vec::new();

        for pos_to_drop in self.pregenerated_chunks.keys() {
            let dx = (pos.px - pos_to_drop.px).abs();
            let dy = (pos.py - pos_to_drop.py).abs();
            let dz = (pos.pz - pos_to_drop.pz).abs();
            if dx >= 16 || dy >= 8 || dz >= 16 {
                // TODO : use render distance value
                vec_to_drop.push(pos_to_drop.clone());
                println!("Dropping pregenerate chunks ...");
            }
        }

        for pos_to_drop in vec_to_drop.drain(..) {
            self.pregenerated_chunks.remove(&pos_to_drop);
        }

        for i in -1..=1 {
            for j in -1..=1 {
                for k in -1..=1 {
                    chunks_vec.push(
                        match self.pregenerated_chunks.remove(&pos.offset(i, j, k)) {
                            Some(chunk) => chunk,
                            None => {
                                let mut chunk = Chunk::new(pos.offset(i, j, k));
                                DefaultWorldGenerator::pregenerate_chunk(
                                    &mut chunk,
                                    &block_registry,
                                );
                                chunk
                            }
                        },
                    );
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
            let min_x = chunks[0].pos.px * CHUNK_SIZE as i64;
            let max_x = (chunks[0].pos.px + 3) * CHUNK_SIZE as i64;
            let min_y = chunks[0].pos.py * CHUNK_SIZE as i64;
            let max_y = (chunks[0].pos.py + 3) * CHUNK_SIZE as i64;
            let min_z = chunks[0].pos.pz * CHUNK_SIZE as i64;
            let max_z = (chunks[0].pos.pz + 3) * CHUNK_SIZE as i64;

            let chunk_size_64 = CHUNK_SIZE as i64;

            for i in -1..=1 {
                for j in -1..=1 {
                    for k in -1..=1 {
                        for l in 0..128 {
                            let current_chunk =
                                &chunks[((i + 1) * 9 + (j + 1) * 3 + (k + 1)) as usize];
                            let cc_pos = current_chunk.pos;
                            let cbx = cc_pos.px * chunk_size_64;
                            let cby = cc_pos.py * chunk_size_64;
                            let cbz = cc_pos.pz * chunk_size_64;

                            let mut tx = rand_pos_int(
                                cc_pos.px as i32,
                                cc_pos.py as i32,
                                cc_pos.pz as i32,
                                3 * l,
                            ) as i64;
                            let mut ty = rand_pos_int(
                                cc_pos.px as i32,
                                cc_pos.py as i32,
                                cc_pos.pz as i32,
                                3 * l + 1,
                            ) as i64;
                            let mut tz = rand_pos_int(
                                cc_pos.px as i32,
                                cc_pos.py as i32,
                                cc_pos.pz as i32,
                                3 * l + 2,
                            ) as i64;

                            tx = (tx % chunk_size_64 + chunk_size_64) % chunk_size_64;
                            ty = (ty % chunk_size_64 + chunk_size_64) % chunk_size_64;
                            tz = (tz % chunk_size_64 + chunk_size_64) % chunk_size_64;

                            if current_chunk.get_block_at((tx as u32, ty as u32, tz as u32))
                                == grass_block
                            {
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
                                                blocks_to_place_one
                                                    .push(BlockToPlace::new(pos, leaves_block));
                                            } else {
                                                if jj <= 6 {
                                                    blocks_to_place_one
                                                        .push(BlockToPlace::new(pos, wood_block));
                                                } else {
                                                    blocks_to_place_one
                                                        .push(BlockToPlace::new(pos, leaves_block));
                                                }
                                            }
                                        }
                                    }
                                }

                                let mut place = true;

                                for blocks in blocks_to_place_one.iter() {
                                    if blocks.pos.px >= min_x
                                        && blocks.pos.px < max_x
                                        && blocks.pos.py >= min_y
                                        && blocks.pos.py < max_y
                                        && blocks.pos.pz >= min_z
                                        && blocks.pos.pz < max_z
                                    {
                                        let cblock_pos = blocks.pos.containing_chunk_pos();
                                        let (x, y, z) = (
                                            cblock_pos.px - chunks[0].pos.px,
                                            cblock_pos.py - chunks[0].pos.py,
                                            cblock_pos.pz - chunks[0].pos.pz,
                                        );
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
            let min_x = (chunks[0].pos.px + 1) * CHUNK_SIZE as i64;
            let max_x = (chunks[0].pos.px + 2) * CHUNK_SIZE as i64;
            let min_y = (chunks[0].pos.py + 1) * CHUNK_SIZE as i64;
            let max_y = (chunks[0].pos.py + 2) * CHUNK_SIZE as i64;
            let min_z = (chunks[0].pos.pz + 1) * CHUNK_SIZE as i64;
            let max_z = (chunks[0].pos.pz + 2) * CHUNK_SIZE as i64;
            if blocks.pos.px >= min_x
                && blocks.pos.px < max_x
                && blocks.pos.py >= min_y
                && blocks.pos.py < max_y
                && blocks.pos.pz >= min_z
                && blocks.pos.pz < max_z
            {
                let pos = blocks.pos.pos_in_containing_chunk();
                chunks[13].set_block_at(pos, blocks.id);
            }
        }
    }
}

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::world::BlockPos;
use crate::worldgen::perlin::rand_pos_int;
use crate::{
    block::Block,
    registry::Registry,
    world::chunk::{Chunk, ChunkPos, CHUNK_SIZE},
    world::WorldGenerator,
};

use crate::worldgen::decorator::Decorator;
use crate::worldgen::decorator::DecoratorPass;

pub mod perlin;
#[macro_use]
pub mod decorator;

pub struct DefaultWorldGenerator {
    pregenerated_chunks: HashMap<ChunkPos, Chunk>,
    tree_decorator: Decorator,
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
    pub fn new(block_registry: &Registry<Block>) -> Self {
        let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
        let leaves_block = block_registry.get_id_by_name(&"leaves".to_owned()).unwrap() as u16;
        let wood_block = block_registry.get_id_by_name(&"wood".to_owned()).unwrap() as u16;

        let mut pass_leaves = DecoratorPass::new(leaves_block);
        let mut pass_wood = DecoratorPass::new(wood_block);
        pass_wood.block_whitelist.insert(leaves_block);

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
                    if ii != 0 || kk != 0 {
                        pass_leaves.block_pos.push(BlockPos::from((ii, jj, kk)));
                    } else {
                        if jj <= 6 {
                            pass_wood.block_pos.push(BlockPos::from((ii, jj, kk)));
                        } else {
                            pass_leaves.block_pos.push(BlockPos::from((ii, jj, kk)));
                        }
                    }
                }
            }
        }

        let tree_decorator = Decorator {
            number_of_try: 32,
            block_start_whitelist: set![grass_block],
            pass: vec![pass_leaves, pass_wood],
        };
        Self {
            tree_decorator,
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
            for i in 0..CHUNK_SIZE {
                for j in 0..CHUNK_SIZE {
                    for k in 0..CHUNK_SIZE {
                        chunk.set_block_at((i as u32, j as u32, k as u32), stone_block);
                    }
                }
            }
            return;
        }

        let s = CHUNK_SIZE + 3;
        let noise = perlin::perlin(px, py, pz, s as usize, freq, freq * 2.0, freq, 5, 0.4, 42);

        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
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
        println!(
            "Time to generate chunk : {} micros",
            (t2 - t1).subsec_micros()
        );
    }

    fn decorate_chunk(chunks: &mut Vec<Chunk>, decorator: &Decorator) {
        let min_x = chunks[0].pos.px * CHUNK_SIZE as i64;
        let max_x = (chunks[0].pos.px + 3) * CHUNK_SIZE as i64;
        let min_y = chunks[0].pos.py * CHUNK_SIZE as i64;
        let max_y = (chunks[0].pos.py + 3) * CHUNK_SIZE as i64;
        let min_z = chunks[0].pos.pz * CHUNK_SIZE as i64;
        let max_z = (chunks[0].pos.pz + 3) * CHUNK_SIZE as i64;

        let chunk_size_64 = CHUNK_SIZE as i64;
        let mut blocks_to_place: Vec<Vec<BlockToPlace>> = Vec::new();

        for _i in 0..decorator.pass.len() {
            blocks_to_place.push(Vec::new());
        }

        for i in -1..=1 {
            for j in -1..=1 {
                for k in -1..=1 {
                    for l in 0..decorator.number_of_try as i32 {
                        let current_chunk = &chunks[((i + 1) * 9 + (j + 1) * 3 + (k + 1)) as usize];
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

                        if decorator.block_start_whitelist.contains(
                            &current_chunk.get_block_at((tx as u32, ty as u32, tz as u32)),
                        ) {
                            tx += cbx;
                            ty += cby;
                            tz += cbz;

                            let mut place = true;
                            let mut blocks_to_place_one: Vec<Vec<BlockToPlace>> = Vec::new();

                            for _i in 0..decorator.pass.len() {
                                blocks_to_place_one.push(Vec::new());
                            }
                            let mut pass_count = 0;
                            for decorator_pass in decorator.pass.iter() {
                                for blocks_pos in decorator_pass.block_pos.iter() {
                                    let mut pos = blocks_pos.clone();
                                    pos.px += tx;
                                    pos.py += ty;
                                    pos.pz += tz;

                                    if pos.px >= min_x
                                        && pos.px < max_x
                                        && pos.py >= min_y
                                        && pos.py < max_y
                                        && pos.pz >= min_z
                                        && pos.pz < max_z
                                    {
                                        let cblock_pos = pos.containing_chunk_pos();
                                        let (x, y, z) = (
                                            cblock_pos.px - chunks[0].pos.px,
                                            cblock_pos.py - chunks[0].pos.py,
                                            cblock_pos.pz - chunks[0].pos.pz,
                                        );
                                        let chunk = &chunks[(x * 9 + y * 3 + z) as usize];
                                        let (ux, uy, uz) = pos.pos_in_containing_chunk();
                                        if decorator_pass
                                            .block_whitelist
                                            .contains(&chunk.get_block_at((ux, uy, uz)))
                                        {
                                            blocks_to_place_one[pass_count].push(
                                                BlockToPlace::new(
                                                    (pos.px, pos.py, pos.pz),
                                                    decorator_pass.block_type,
                                                ),
                                            );
                                        } else if !decorator_pass
                                            .block_non_blocking
                                            .contains(&chunk.get_block_at((ux, uy, uz)))
                                        {
                                            // still checking if not blocking block
                                            place = false;
                                            break;
                                        }
                                    } else {
                                        // outside the 3x3x3 chunks block -> cancel
                                        // no structure larger thant chunks size
                                        place = false;
                                        break;
                                    }
                                }
                                pass_count += 1;
                            }
                            if place {
                                // we add the block to full list of blocks to place
                                for w in 0..decorator.pass.len() {
                                    for blocks in blocks_to_place_one[w].drain(..) {
                                        blocks_to_place[w].push(blocks);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for w in 0..decorator.pass.len() {
                for blocks in blocks_to_place[w].drain(..) {
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

        let decorator = &self.tree_decorator;

        DefaultWorldGenerator::decorate_chunk(&mut chunks_vec, decorator);

        for chunk in chunks_vec.drain(..) {
            self.pregenerated_chunks.insert(chunk.pos, chunk);
        }

        self.pregenerated_chunks.remove(&pos).unwrap()
    }
}

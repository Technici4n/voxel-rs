use crate::world::chunk::{Chunk, CHUNK_SIZE};
use crate::world::HighestOpaqueBlock;
use std::collections::VecDeque;

// TODO : Add block that are source of light


pub struct LightData {
    pub light_level: [u8; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize]
}

impl LightData {
    pub fn new() -> Self {
        Self {
            light_level: [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize],
        }
    }
}

/// Take a 3x3x3 chunks bloc and 3x3 HighestOpaqueBlock and compute the light by using a BFS
pub fn compute_light(chunks: Vec<Option<&Chunk>>, highest_opaque_blocks: Vec<HighestOpaqueBlock>) -> LightData {
    let mut res = LightData::new();
    let mut bfs_queue: VecDeque<(usize, usize, usize, u8)> = VecDeque::new();

    let mut light_data = [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    let mut opaque = [false; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    let csize = CHUNK_SIZE as usize;


    let y0 = chunks[13].as_ref().unwrap().pos.py;

    for cx in 0..3 {
        for cy in 0..3 {
            for cz in 0..3 {
                let chunk = chunks[cx * 9 + cy * 3 + cz];
                let highest_opaque_block = highest_opaque_blocks[cx * 3 + cz];
                match chunk {
                    None => {
                        for i in 0..CHUNK_SIZE {
                            for k in 0..CHUNK_SIZE {
                                for j in (0..CHUNK_SIZE).rev() {
                                    let s = (cx * csize + i as usize) * csize * csize * 9 + (cy * csize + j as usize) * csize * 3 + (cz * csize + k as usize);
                                    if (y0 + cy as i64 - 1) * CHUNK_SIZE as i64 + j as i64 > highest_opaque_block.y[(i * CHUNK_SIZE + k) as usize] {
                                        light_data[s] = 15;
                                        bfs_queue.push_back((cx * csize + i as usize, cy * csize + j as usize, cz * csize + k as usize, 15));
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(c) => {
                        for i in 0..CHUNK_SIZE {
                            for j in 0..CHUNK_SIZE {
                                for k in 0..CHUNK_SIZE {
                                    let s = (cx * csize + i as usize) * csize * csize * 0 + (cy * csize + j as usize) * csize * 3 + (cz * csize + k as usize);
                                    if c.get_block_at((i, j, k)) != 0 { // TODO : replace by is opaque
                                        opaque[s] = true;
                                    } else if c.pos.py * CHUNK_SIZE as i64 + j as i64 > highest_opaque_block.y[(i * CHUNK_SIZE + k) as usize] {
                                        light_data[s] = 15;
                                        bfs_queue.push_back((cx * csize + i as usize, cy * csize + j as usize, cz * csize + k as usize, 15));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    while !bfs_queue.is_empty() {
        let (x, y, z, ll) = bfs_queue.pop_front().unwrap();
        let s = x * csize * csize * 9 + y * csize * 3 + z;

        if ll >= 1 {
            if x > 0 && !opaque[s - csize * csize * 9] && light_data[s - csize * csize * 9] < ll - 1 {
                light_data[s - csize * csize * 9] = ll - 1;
                bfs_queue.push_back((x - 1, y, z, ll - 1));
            }

            if x < 3 * csize - 1 && !opaque[s + csize * csize * 9] && light_data[s + csize * csize * 9] < ll - 1 {
                light_data[s + csize * csize * 9] = ll - 1;
                bfs_queue.push_back((x + 1, y, z, ll - 1));
            }


            if y > 0 && !opaque[s - csize * 3] && light_data[s - csize * 3] < ll - 1 {
                light_data[s - csize * 3] = ll - 1;
                bfs_queue.push_back((x, y - 1, z, ll - 1));
            }

            if y < 3 * csize - 1 && !opaque[s + csize * 3] && light_data[s + csize * 3] < ll - 1 {
                light_data[s + csize * 3] = ll - 1;
                bfs_queue.push_back((x, y + 1, z, ll - 1));
            }

            if z > 0 && !opaque[s - 1] && light_data[s - 1] < ll - 1 {
                light_data[s - 1] = ll - 1;
                bfs_queue.push_back((x, y, z - 1, ll - 1));
            }

            if z < 3 * csize - 1 && !opaque[s + 1] && light_data[s + 1] < ll - 1 {
                light_data[s + 1] = ll - 1;
                bfs_queue.push_back((x, y, z + 1, ll - 1));
            }
        }
    }

    for i in 0..csize {
        for j in 0..csize {
            for k in 0..csize {
                res.light_level[i * csize * csize + j * csize + k] = light_data[(i + csize) * csize * csize * 9 + (j + csize) * 3 * csize + (k + csize)];
            }
        }
    }

    return res;
}
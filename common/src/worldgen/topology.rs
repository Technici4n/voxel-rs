use crate::world::chunk::{CHUNK_SIZE, Chunk};
use crate::registry::Registry;
use crate::block::Block;
use crate::worldgen::perlin;

const MAX_DEPTH: usize = 5;


pub fn generate_ground_level(px: f32, pz: f32) -> Vec<f32> {
    let mut res = vec![0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize];

    let dx1 = perlin::perlin2d(px, pz, CHUNK_SIZE as usize, 1.0 / 64.0, 1.0 / 64.0, 5, 0.4, 0);
    let dy1 = perlin::perlin2d(px, pz, CHUNK_SIZE as usize, 1.0 / 64.0, 1.0 / 64.0, 5, 0.4, 1);

    let noise1 = perlin::perlin2d_with_displacement(&dx1, &dy1, CHUNK_SIZE as f32, px, pz, CHUNK_SIZE as usize, 1.0 / 64.0, 1.0 / 64.0, 5, 0.4, 2);
    let noise2 = perlin::perlin2d(px, pz, CHUNK_SIZE as usize, 1.0 / 256.0, 1.0 / 256.0, 6, 0.5, 3);

    for i in 0..(CHUNK_SIZE * CHUNK_SIZE) as usize {
        let a = noise2[i] * 130.0;
        let h1 = (noise1[i] - 0.3) * a;
        res[i] = h1;
    }

    return res;
}

/// Generate the topology of the chunk
pub fn generate_chunk_topology(chunk: &mut Chunk, block_registry: &Registry<Block>) {
    let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
    let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
    let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
    let dirt_grass = block_registry.get_id_by_name(&"dirt_grass".to_owned()).unwrap() as u16;
    let water_block = block_registry.get_id_by_name(&"water".to_owned()).unwrap() as u16;
    let sand_block = block_registry.get_id_by_name(&"sand".to_owned()).unwrap() as u16;

    let px = (chunk.pos.px * CHUNK_SIZE as i64) as f32;
    let py = (chunk.pos.py * CHUNK_SIZE as i64) as f32;
    let pz = (chunk.pos.pz * CHUNK_SIZE as i64) as f32;
    let freq = 1.0 / 32.0;

    let h = generate_ground_level(px, pz);
    let s = CHUNK_SIZE as usize + MAX_DEPTH;

    let mut only_air = true;
    let mut only_stone = true;

    'ijk_loop: for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                let id2 = (i * CHUNK_SIZE + k) as usize;
                for l in 0..MAX_DEPTH {
                    if !is_empty(j as usize, l, py, h[id2], 10.0 + h[id2] / 10.0, 1.0) || (py as i32 + j as i32 + l as i32) < 0 {
                        only_air = false;
                    }
                    if is_empty(j as usize, l, py, h[id2], 10.0 + h[id2] / 10.0, 0.0) {
                        only_stone = false;
                    }
                }
                if !(only_air || only_stone) {
                    break 'ijk_loop;
                }
            }
        }
    }


    if !(only_air || only_stone) {
        let noise = perlin::perlin(px, py, pz, s, freq, freq * 2.0, freq, 4, 0.7, 42);
        for i in 0..CHUNK_SIZE {
            for j in 0..CHUNK_SIZE {
                for k in 0..CHUNK_SIZE {
                    let id2 = (i * CHUNK_SIZE + k) as usize;
                    let q = depthness(&noise, (i as usize, j as usize, k as usize), py, h[id2], 10.0 + h[id2] / 10.0);
                    unsafe {
                        match q {
                            0 => { if (py as i32 + j as i32) < 0 { chunk.set_block_at_unsafe((i, j, k), water_block); } }
                            1 => { if h[id2] >= -2.0 { chunk.set_block_at_unsafe((i, j, k), grass_block); } else { chunk.set_block_at_unsafe((i, j, k), sand_block); } }
                            2 => { if h[id2] >= -2.0 { chunk.set_block_at_unsafe((i, j, k), dirt_grass); } else { chunk.set_block_at_unsafe((i, j, k), sand_block);  }}
                            3..=4 => { if h[id2] >= -2.0 { chunk.set_block_at_unsafe((i, j, k), dirt_block); } else { chunk.set_block_at_unsafe((i, j, k), sand_block);  }}
                            5 => { chunk.set_block_at_unsafe((i, j, k), stone_block); }
                            _ => {}
                        }
                    }
                }
            }
        }
    } else if only_stone {
        unsafe {
            chunk.fill_unsafe(stone_block);
        }
    }
}

pub fn depthness(noise: &Vec<f32>, (px, py, pz): (usize, usize, usize), chunk_pos_y: f32, ground_level: f32, max_delta: f32) -> usize {
    if ((py + MAX_DEPTH - 1) as f32 + chunk_pos_y - ground_level) / max_delta < 0.0 {
        return MAX_DEPTH;
    }
    let s = CHUNK_SIZE as usize + MAX_DEPTH;
    for i in 0..MAX_DEPTH {
        unsafe {
            if is_empty(py, i, chunk_pos_y, ground_level, max_delta, *noise.get_unchecked(px * s * s + (py + i) * s + pz)) {
                // noise low => air
                // noise high => full
                return i;
            }
        }
    }
    return MAX_DEPTH;
}

pub fn is_empty(py: usize, above: usize, chunk_pos_y: f32, ground_level: f32, max_delta: f32, noise_value: f32) -> bool {
    return noise_value < ((py + above) as f32 + chunk_pos_y - ground_level) / max_delta;
}
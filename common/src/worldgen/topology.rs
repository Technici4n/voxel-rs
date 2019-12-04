use crate::world::chunk::{CHUNK_SIZE, Chunk};
use crate::registry::Registry;
use crate::block::Block;
use crate::worldgen::perlin;


/// Generate the topology of the chunk
pub fn generate_chunk_topology(chunk: &mut Chunk, block_registry: &Registry<Block>) {
    let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
    let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
    let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
    let dirt_grass = block_registry.get_id_by_name(&"dirt_grass".to_owned()).unwrap() as u16;

    let px = (chunk.pos.px * CHUNK_SIZE as i64) as f32;
    let py = (chunk.pos.py * CHUNK_SIZE as i64) as f32;
    let pz = (chunk.pos.pz * CHUNK_SIZE as i64) as f32;
    let freq = 1.0 / 32.0;


    let s = CHUNK_SIZE as usize + 5;
    let noise = perlin::perlin(px, py, pz, s, freq, freq * 2.0, freq, 4, 0.4, 42);
    let ground_level = perlin::perlin2d(px, pz, CHUNK_SIZE as usize, 1.0 / 128.0, 1.0 / 128.0, 4, 0.7, 43);
    let h_scale = perlin::perlin2d(px, pz, CHUNK_SIZE as usize, 1.0 / 512.0, 1.0 / 512.0, 4, 0.4, 44);

    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                let i2d = (i * CHUNK_SIZE + k) as usize;
                let h2 = ((h_scale[i2d])*(600.0 as f32).powf(0.25)).powf(4.0);
                let h = ground_level[i2d] * (h2);
                let q = depthness(&noise, (i as usize, j as usize, k as usize),
                                  5, py, h - 10.0, 10.0 + h2/10.0);
                unsafe {
                    match q {
                        1 => { chunk.set_block_at_unsafe((i, j, k), grass_block); }
                        2 => { chunk.set_block_at_unsafe((i, j, k), dirt_grass); }
                        3..=4 => { chunk.set_block_at_unsafe((i, j, k), dirt_block); }
                        5 => { chunk.set_block_at_unsafe((i, j, k), stone_block); }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub fn depthness(noise: &Vec<f32>, (px, py, pz): (usize, usize, usize), max_check: usize, chunk_pos_y: f32, ground_level: f32, max_delta: f32) -> usize {
    if ((py + max_check - 1) as f32 + chunk_pos_y - ground_level) / max_delta < 0.0 {
        return max_check;
    }
    let s = CHUNK_SIZE as usize + max_check;
    for i in 0..max_check {
        unsafe {
            if *noise.get_unchecked(px * s * s + (py + i) * s + pz) < ((py + i) as f32 + chunk_pos_y - ground_level) / max_delta {
                return i;
            }
        }
    }
    return max_check;
}
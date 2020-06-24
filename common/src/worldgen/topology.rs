use crate::block::Block;
use crate::registry::Registry;
use crate::world::{Chunk, CHUNK_SIZE, ChunkPosXZ};
use crate::worldgen::perlin;
use std::collections::HashMap;

pub struct HeightMap {
    height_map: HashMap<ChunkPosXZ, Vec<i32>>,
}

impl  HeightMap {

    pub fn new() ->Self{
        return Self{
            height_map: HashMap::new(),
        };
    }

    pub fn get_chunk_height_map(&mut self, pos : ChunkPosXZ) -> &Vec<i32> {
         if !self.height_map.contains_key(&pos){
             let mut res = vec![-1; (CHUNK_SIZE*CHUNK_SIZE) as usize];
             let c = CHUNK_SIZE as f32;
             let s = generate_ground_level((pos.px as f32)*c, (pos.pz as f32)*c);
             for i in 0..(CHUNK_SIZE*CHUNK_SIZE)  as usize {
                 res[i]  = s[i] as i32;
             }
             self.height_map.insert(pos, res);
         }
        return self.height_map.get(&pos).unwrap();
    }

}

pub fn generate_ground_level(px: f32, pz: f32) -> Vec<f32> {
    let mut res = vec![0.0; (CHUNK_SIZE * CHUNK_SIZE) as usize];

    let dx1 = perlin::perlin2d(
        px,
        pz,
        CHUNK_SIZE as usize,
        1.0 / 64.0,
        1.0 / 64.0,
        5,
        0.5,
        0,
    );
    let dy1 = perlin::perlin2d(
        px,
        pz,
        CHUNK_SIZE as usize,
        1.0 / 64.0,
        1.0 / 64.0,
        5,
        0.5,
        1,
    );

    let noise1 = perlin::perlin2d_with_displacement(
        &dx1,
        &dy1,
        2.0*(CHUNK_SIZE as f32),
        px,
        pz,
        CHUNK_SIZE as usize,
        1.0 / 128.0,
        1.0 / 128.0,
        5,
        0.4,
        2,
    );
    let noise2 = perlin::perlin2d(
        px,
        pz,
        CHUNK_SIZE as usize,
        1.0 / 256.0,
        1.0 / 256.0,
        5,
        0.3,
        3,
    );

    for i in 0..(CHUNK_SIZE * CHUNK_SIZE) as usize {
        let a = noise2[i] * 130.0;
        let mut h1 = (noise1[i]) * a - 10.0;
        if h1 <= 0.0 {
            h1 *=3.0;
        }
        res[i] = h1;
    }

    return res;
}

/// Generate the topology of the chunk
pub fn generate_chunk_topology(chunk: &mut Chunk, block_registry: &Registry<Block>,height_map :  &mut HeightMap) {
    let stone_block = block_registry.get_id_by_name(&"stone".to_owned()).unwrap() as u16;
    let grass_block = block_registry.get_id_by_name(&"grass".to_owned()).unwrap() as u16;
    let dirt_block = block_registry.get_id_by_name(&"dirt".to_owned()).unwrap() as u16;
    let dirt_grass = block_registry
        .get_id_by_name(&"dirt_grass".to_owned())
        .unwrap() as u16;
    let water_block = block_registry.get_id_by_name(&"water".to_owned()).unwrap() as u16;
    let sand_block = block_registry.get_id_by_name(&"sand".to_owned()).unwrap() as u16;

    let h = height_map.get_chunk_height_map(chunk.pos.into());

    for i in 0..CHUNK_SIZE{
        for k in 0..CHUNK_SIZE{
            for j in 0..CHUNK_SIZE{
                let y = j as i32 + (CHUNK_SIZE as i32)*(chunk.pos.py as i32);
                let hm = h[(i*CHUNK_SIZE + k) as usize];
                if y > hm {
                    if y < 0{
                      unsafe{chunk.set_block_at_unsafe((i,j, k), water_block);}
                    }else {
                        break;
                    }
                }else{
                    unsafe {
                        chunk.set_block_at_unsafe((i,j, k),
                        match hm - y {
                            0 => if hm >= 1 {grass_block} else {sand_block},
                            1 => if hm >= 1 {dirt_grass} else {sand_block},
                            2..=4 => if hm >= 1 {dirt_block} else {sand_block},
                            _ => stone_block,
                        });
                    }
                }
            }
        }
    }

}



#![allow(dead_code)]
use std::collections::HashMap;

pub mod camera;
pub mod chunk;
pub mod meshing;
pub mod renderer;

use crate::world::chunk::{Chunk, ChunkPos, CHUNK_SIZE};
use crate::world::meshing::AdjChunkOccl;
use self::camera::Camera;



pub struct World {
    pub camera: Camera,
    pub chunks: HashMap<ChunkPos, Chunk>,
}

impl World {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
            chunks: HashMap::new(),
        }
    }

    /// If the chunk at position x,y,z does not exist, create and generate it
    pub fn gen_chunk(&mut self, x: i64, y: i64, z: i64){
        match self.get_chunk(x, y, z){
            Some(_chunk) =>(),
            None =>{
                let mut chunk = Chunk::new(x,y,z);
                chunk.fill_perlin();
                let pos = ChunkPos {
                    px: x,
                    py: y,
                    pz: z,
                };
                self.chunks.insert(pos, chunk);
            }
        }
    }

    ///Return a reference to the chunk if it exists, None otherwise
    pub fn get_chunk(&self, x: i64, y: i64, z: i64) -> Option<&Chunk> {
        self.chunks.get(&ChunkPos {
            px: x,
            py: y,
            pz: z,
        })
    }

    /// Return a mutable reference to the chunk if it exists None otherwise
    pub fn _get_chunk_mut(&mut self, x: i64, y: i64, z: i64) -> Option<&mut Chunk> {
        self.chunks.get_mut(&ChunkPos {
            px: x,
            py: y,
            pz: z,
        })
    }

    /// Return data at position x,y,z
    pub fn _get_data(&self, x: i64, y: i64, z: i64) -> u32 {
        let (cx, cy, cz) = World::get_chunk_coord(x, y, z);
        let (dx, dy, dz) = (
            (x - cx * CHUNK_SIZE as i64) as u32,
            (y - cy * CHUNK_SIZE as i64) as u32,
            (z - cz * CHUNK_SIZE as i64) as u32,
        );
        match self.get_chunk(cx, cy, cz) {
            None => 0,
            Some(chunk) => chunk.get_data(dx, dy, dz),
        }
    }

    /// Set data at position x,y,z
    /// Enventually create a new chunk if necessary
    pub fn _set_data(&mut self, x: i64, y: i64, z: i64, data: u32) {
        let (cx, cy, cz) = World::get_chunk_coord(x, y, z);
        let (dx, dy, dz) = (
            (x - cx * CHUNK_SIZE as i64) as u32,
            (y - cy * CHUNK_SIZE as i64) as u32,
            (z - cz * CHUNK_SIZE as i64) as u32,
        );
        self.get_add_chunk(cx, cy, cz).set_data(dx, dy, dz, data);
    }

    /// Create a new chunk at position (x, y, z) if not already present
    /// Anyway, return the a mutable reference to the chunk created or existing
    pub fn get_add_chunk(&mut self, x: i64, y: i64, z: i64) -> &mut Chunk {
        let pos = &ChunkPos {
            px: x,
            py: y,
            pz: z,
        };
        if self.chunks.contains_key(pos) {
            self.chunks.get_mut(pos).unwrap()
        } else {
            self.chunks.insert(*pos, Chunk::new(x, y, z));
            self.chunks.get_mut(pos).unwrap()
        }
    }

    /// Convert the world block coordinates into the chunk coordinates
    pub fn get_chunk_coord(ix: i64, iy: i64, iz: i64) -> (i64, i64, i64) {
        let (x, y, z);
        if ix >= 0 {
            x = ix / CHUNK_SIZE as i64;
        } else {
            x = ix / CHUNK_SIZE as i64 - 1;
        }
        if iy >= 0 {
            y = iy / CHUNK_SIZE as i64;
        } else {
            y = iy / CHUNK_SIZE as i64 - 1;
        }
        if iz >= 0 {
            z = iz / CHUNK_SIZE as i64;
        } else {
            z = iz / CHUNK_SIZE as i64 - 1;
        }
        (x, y, z)
    }

    /// Generate the AdjChunkOccl struct used in the meshing containing the
    /// informations about adjacent chunks
    pub fn create_adj_chunk_occl(&self, ix: i64, iy: i64, iz: i64) -> AdjChunkOccl{
        let chunk_size = CHUNK_SIZE as i64;
        // faces
        let da = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0,0, 1], [0, 0, -1]];
        let mut faces = [[[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6];
        for i in 0..6 {
            faces[i] =
                match self.get_chunk(ix + da[i][0], iy + da[i][1], iz + da[i][2]) {
                    Some(chunk) => {
                        let mut res = [[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize];
                        for j in 0..CHUNK_SIZE {
                            for k in 0..CHUNK_SIZE {
                                let (ux, uy, uz);
                                if i / 2 == 0 {
                                    ux = (chunk_size + da[i][0]) as u32 % CHUNK_SIZE;
                                    uy = j;
                                    uz = k;
                                } else if i / 2 == 1 {
                                    ux = j;
                                    uy = (chunk_size + da[i][1]) as u32 % CHUNK_SIZE;
                                    uz = k;
                                } else {
                                    ux = j;
                                    uy = k;
                                    uz = (chunk_size + da[i][2]) as u32 % CHUNK_SIZE;
                                }
                                res[j as usize ][k as usize] = chunk.get_data(ux, uy, uz) != 0;
                            }
                        }
                        res
                    }
                    None => [[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]
                };
             }
            // edges
            let mut edges = [[false; CHUNK_SIZE as usize]; 12];
            let de = [
                [0, 1, 1], [0, 1, -1], [0, -1, 1], [0, -1, -1],
                [1, 0, 1], [1, 0, -1], [-1, 0, 1], [-1, 0, -1],
                [1, 1, 0], [1, -1, 0], [-1, 1, 0], [-1, -1, 0],
            ];
            for i in 0..12{
                edges[i] =
                    match self.get_chunk(ix+de[i][0], iy+de[i][1], iz+de[i][2]){
                        Some(chunk) =>{
                            let mut res = [false; CHUNK_SIZE as usize];
                            for j in 0..CHUNK_SIZE{
                                let (ux, uy, uz);
                                if i / 4 == 0{
                                    ux = j;
                                    uy = (chunk_size + de[i][1]) as u32 %CHUNK_SIZE;
                                    uz = (chunk_size + de[i][2]) as u32 %CHUNK_SIZE;
                                }else if i /4 == 1{
                                    ux = (chunk_size + de[i][0]) as u32 %CHUNK_SIZE;
                                    uy = j;
                                    uz = (chunk_size + de[i][2]) as u32 %CHUNK_SIZE;
                                }else{
                                    ux = (chunk_size + de[i][0]) as u32 %CHUNK_SIZE;
                                    uy = (chunk_size + de[i][1]) as u32 %CHUNK_SIZE;
                                    uz = j;
                                }
                                res[j as usize] = chunk.get_data(ux, uy, uz) != 0;

                            }
                            res
                        }
                        None => [false; CHUNK_SIZE as usize]
                    };

        }
        // coins
        let mut coins = [false;8];
        let dc = [
            [1, 1, 1], [1, 1, -1], [1, -1, 1], [1, -1, -1],
            [-1, 1, 1], [-1, 1, -1], [-1, -1, 1], [-1, -1, -1]];
        for i in 0..8{
            coins[i] =
                match self.get_chunk(ix+dc[i][0], iy+dc[i][1], iz+dc[i][2]) {
                    None => false,
                    Some(chunk) =>{
                        let ux = (chunk_size + dc[i][0]) as u32 %CHUNK_SIZE;
                        let uy = (chunk_size + dc[i][1]) as u32 %CHUNK_SIZE;
                        let uz = (chunk_size + dc[i][2]) as u32 %CHUNK_SIZE;
                        chunk.get_data(ux, uy, uz) != 0
                    }
                };
        }
        AdjChunkOccl{ faces, edges, coins}

    }


}

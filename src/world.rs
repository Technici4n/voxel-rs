use std::collections::HashMap;

use crate::world::chunk::{Chunk, CHUNK_SIZE, ChunkPos};

use self::camera::Camera;

pub mod camera;
pub mod chunk;
pub mod meshing;
pub mod renderer;

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

    ///Return a reference to the chunk if it exists, None otherwise
    pub fn get_chunk(&self, x: i64, y: i64, z: i64) -> Option<&Chunk> {
        self.chunks.get(&ChunkPos {
            px: x,
            py: y,
            pz: z,
        })
    }

    /// Return a mutable reference to the chunk if it exists None otherwise
    pub fn get_chunk_mut(&mut self, x: i64, y: i64, z: i64) -> Option<&mut Chunk> {
        self.chunks.get_mut(&ChunkPos {
            px: x,
            py: y,
            pz: z,
        })
    }

    /// Return data at position x,y,z
    pub fn get_data(&self, x: i64, y: i64, z: i64) -> u32 {
        let (cx, cy, cz) = World::get_chunk_coord(x, y, z);
        let (dx, dy, dz) = ((x - cx * CHUNK_SIZE as i64) as u32, (y - cy * CHUNK_SIZE as i64) as u32, (z - cz * CHUNK_SIZE as i64) as u32);
        match self.get_chunk(cx,cy, cz){
            None => 0,
            Some(chunk) => chunk.get_data(dx, dy, dz)
        }
    }

    /// Set data at position x,y,z
    /// Enventually create a new chunk if necessary
    pub fn set_data(&mut self, x: i64, y: i64, z: i64, data : u32) {
        let (cx, cy, cz) = World::get_chunk_coord(x, y, z);
        let (dx, dy, dz) = ((x - cx * CHUNK_SIZE as i64) as u32, (y - cy * CHUNK_SIZE as i64) as u32, (z - cz * CHUNK_SIZE as i64) as u32);
        self.get_add_chunk(cx,cy,cz).set_data(dx, dy, dz, data);
    }

    /// Create a new chunk at position (x, y, z) if not already present
    /// Anyway, return the a mutable reference to the chunk created or existing
    pub fn get_add_chunk(&mut self, x: i64, y: i64, z: i64) -> &mut Chunk {
        let pos = &ChunkPos {
            px: x,
            py: y,
            pz: z,
        };
        if self.chunks.contains_key(pos){
            self.chunks.get_mut(pos).unwrap()
        }else{
            self.chunks.insert(*pos, Chunk::new(x,y,z));
            self.chunks.get_mut(pos).unwrap()
        }
    }



    /// Convert the world block coordinates into the chunk coordinates
    pub fn get_chunk_coord(ix: i64, iy: i64, iz: i64) -> (i64, i64, i64) {
        let ( x,  y,  z);
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
}

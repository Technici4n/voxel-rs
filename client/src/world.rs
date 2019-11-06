use voxel_rs_common::{
    world::chunk::{ChunkPos, CHUNK_SIZE},
    world::World,
};

pub mod camera;
pub mod meshing;
pub mod renderer;
pub mod skybox;

use self::meshing::AdjChunkOccl;

/// Generate the AdjChunkOccl struct used in the meshing containing the
/// informations about adjacent chunks
/// TODO: use meshing data instead of `!= 0`
pub fn create_adj_chunk_occl(world: &World, pos: ChunkPos) -> AdjChunkOccl {
    let chunk_size = CHUNK_SIZE as i64;
    // faces
    let da = [
        [1, 0, 0],
        [-1, 0, 0],
        [0, 1, 0],
        [0, -1, 0],
        [0, 0, 1],
        [0, 0, -1],
    ];
    let mut faces = [[[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6];
    for i in 0..6 {
        faces[i] = match world.get_chunk(pos.offset(da[i][0], da[i][1], da[i][2])) {
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
                        res[j as usize][k as usize] = chunk.get_block_at((ux, uy, uz)) != 0;
                    }
                }
                res
            }
            None => [[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize],
        };
    }
    // edges
    let mut edges = [[false; CHUNK_SIZE as usize]; 12];
    let de = [
        [0, 1, 1],
        [0, 1, -1],
        [0, -1, 1],
        [0, -1, -1],
        [1, 0, 1],
        [1, 0, -1],
        [-1, 0, 1],
        [-1, 0, -1],
        [1, 1, 0],
        [1, -1, 0],
        [-1, 1, 0],
        [-1, -1, 0],
    ];
    for i in 0..12 {
        edges[i] = match world.get_chunk(pos.offset(de[i][0], de[i][1], de[i][2])) {
            Some(chunk) => {
                let mut res = [false; CHUNK_SIZE as usize];
                for j in 0..CHUNK_SIZE {
                    let (ux, uy, uz);
                    if i / 4 == 0 {
                        ux = j;
                        uy = (chunk_size + de[i][1]) as u32 % CHUNK_SIZE;
                        uz = (chunk_size + de[i][2]) as u32 % CHUNK_SIZE;
                    } else if i / 4 == 1 {
                        ux = (chunk_size + de[i][0]) as u32 % CHUNK_SIZE;
                        uy = j;
                        uz = (chunk_size + de[i][2]) as u32 % CHUNK_SIZE;
                    } else {
                        ux = (chunk_size + de[i][0]) as u32 % CHUNK_SIZE;
                        uy = (chunk_size + de[i][1]) as u32 % CHUNK_SIZE;
                        uz = j;
                    }
                    res[j as usize] = chunk.get_block_at((ux, uy, uz)) != 0;
                }
                res
            }
            None => [false; CHUNK_SIZE as usize],
        };
    }
    // coins
    let mut coins = [false; 8];
    let dc = [
        [1, 1, 1],
        [1, 1, -1],
        [1, -1, 1],
        [1, -1, -1],
        [-1, 1, 1],
        [-1, 1, -1],
        [-1, -1, 1],
        [-1, -1, -1],
    ];
    for i in 0..8 {
        coins[i] = match world.get_chunk(pos.offset(dc[i][0], dc[i][1], dc[i][2])) {
            None => false,
            Some(chunk) => {
                let ux = (chunk_size + dc[i][0]) as u32 % CHUNK_SIZE;
                let uy = (chunk_size + dc[i][1]) as u32 % CHUNK_SIZE;
                let uz = (chunk_size + dc[i][2]) as u32 % CHUNK_SIZE;
                chunk.get_block_at((ux, uy, uz)) != 0
            }
        };
    }
    AdjChunkOccl {
        faces,
        edges,
        coins,
    }
}

use super::renderer::Vertex;
use voxel_rs_common::world::LightChunk;
use voxel_rs_common::{
    block::BlockMesh,
    collections::zero_initialized_vec,
    world::chunk::{Chunk, ChunkPos, CHUNK_SIZE},
    world::World,
};

/// Structure containing information about adjacent chunks for the meshing
/// Order of face 1x, -1x, 1y, -1y, 1z, -1z => the two order component are in the (x,y,z) order
/// Order of edges (yz), (y-z), (-y z), (-y - z), (xz), (x -z), (-x z), (x - z), (xy), (x - y), (-x y) (-x - y)
/// ( xy means variation along z with x, y = (1+chunk_size, 1+chunk_size), -x y means variation along z with x, y= (-1, 1)
/// Order of coins (1,1,1), (1, 1 -1), (1, -1, 1), (1, -1, -1),
///  ... (-1,1,1), (-1, 1 -1), (-1, -1, 1), (-1, -1, -1),
#[derive(Clone)]
pub struct AdjChunkOccl {
    faces: [[[bool; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6],
    edges: [[bool; CHUNK_SIZE as usize]; 12],
    corners: [bool; 8],
}

/// Same as AdjChunkOccl but for the light, need only the face
#[derive(Clone)]
pub struct  AdjChunkLight{
    faces: [[[u8; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6],
}

#[derive(Clone, Copy, Default)]
pub struct Quad {
    v1: u32,
    // i = 0 j = 0 Ex si 1x => (y, z) = 0
    v2: u32,
    // i = 0 j = 1  => (y, z) = (0, 1)
    v3: u32,
    // i = 1 j = 0 => (y, z) = (1, 0)
    v4: u32, // i = 1 j = 1 => (y, z) = (1, 1)
    block_id: u16,
}

impl Quad {
    fn is_same(&self) -> bool {
        return self.v1 == self.v2 && self.v2 == self.v3 && self.v3 == self.v4;
    }
}

impl AdjChunkOccl {
    /// Generate the AdjChunkOccl struct used in the meshing containing the
    /// informations about adjacent chunks
    pub fn create_from_world(
        world: &World,
        pos: ChunkPos,
        meshes: &Vec<BlockMesh>,
    ) -> AdjChunkOccl {
        const ICHUNK_SIZE: i64 = CHUNK_SIZE as i64;
        // Transform every number to 0 except -1.
        // This allows us to transform chunk deltas into block deltas.
        #[inline(always)]
        fn f(x: i64) -> i64 {
            if x == -1 {
                -1
            } else {
                0
            }
        }
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
            faces[i] = match world.get_chunk(pos.offset_by_pos(da[i].into())) {
                Some(chunk) => {
                    let mut res = [[false; CHUNK_SIZE as usize]; CHUNK_SIZE as usize];
                    for j in 0..CHUNK_SIZE {
                        for k in 0..CHUNK_SIZE {
                            let (ux, uy, uz);
                            if i / 2 == 0 {
                                ux = (ICHUNK_SIZE + f(da[i][0])) as u32 % CHUNK_SIZE;
                                uy = j;
                                uz = k;
                            } else if i / 2 == 1 {
                                ux = j;
                                uy = (ICHUNK_SIZE + f(da[i][1])) as u32 % CHUNK_SIZE;
                                uz = k;
                            } else {
                                ux = j;
                                uy = k;
                                uz = (ICHUNK_SIZE + f(da[i][2])) as u32 % CHUNK_SIZE;
                            }
                            res[j as usize][k as usize] =
                                meshes[chunk.get_block_at((ux, uy, uz)) as usize].is_opaque();
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
            edges[i] = match world.get_chunk(pos.offset_by_pos(de[i].into())) {
                Some(chunk) => {
                    let mut res = [false; CHUNK_SIZE as usize];
                    for j in 0..CHUNK_SIZE {
                        let (ux, uy, uz);
                        if i / 4 == 0 {
                            ux = j;
                            uy = (ICHUNK_SIZE + f(de[i][1])) as u32 % CHUNK_SIZE;
                            uz = (ICHUNK_SIZE + f(de[i][2])) as u32 % CHUNK_SIZE;
                        } else if i / 4 == 1 {
                            ux = (ICHUNK_SIZE + f(de[i][0])) as u32 % CHUNK_SIZE;
                            uy = j;
                            uz = (ICHUNK_SIZE + f(de[i][2])) as u32 % CHUNK_SIZE;
                        } else {
                            ux = (ICHUNK_SIZE + f(de[i][0])) as u32 % CHUNK_SIZE;
                            uy = (ICHUNK_SIZE + f(de[i][1])) as u32 % CHUNK_SIZE;
                            uz = j;
                        }
                        res[j as usize] =
                            meshes[chunk.get_block_at((ux, uy, uz)) as usize].is_opaque();
                    }
                    res
                }
                None => [false; CHUNK_SIZE as usize],
            };
        }
        // corners
        let mut corners = [false; 8];
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
            corners[i] = match world.get_chunk(pos.offset_by_pos(dc[i].into())) {
                None => false,
                Some(chunk) => {
                    let ux = (ICHUNK_SIZE + f(dc[i][0])) as u32 % CHUNK_SIZE;
                    let uy = (ICHUNK_SIZE + f(dc[i][1])) as u32 % CHUNK_SIZE;
                    let uz = (ICHUNK_SIZE + f(dc[i][2])) as u32 % CHUNK_SIZE;
                    meshes[chunk.get_block_at((ux, uy, uz)) as usize].is_opaque()
                }
            };
        }
        AdjChunkOccl {
            faces,
            edges,
            corners,
        }
    }
    /// x, y, z are the position relative to the chunk (0, 0, 0)
    /// Return if the block outside the chunk is opaque
    pub fn is_full(&self, x: i32, y: i32, z: i32) -> bool {
        fn delta(x: i32) -> usize {
            if x == CHUNK_SIZE as i32 {
                0
            } else if x == -1 {
                1
            } else {
                unreachable!()
            }
        }

        let mut n_outside = 0;
        if x == -1 || x == CHUNK_SIZE as i32 {
            n_outside += 1;
        }
        if y == -1 || y == CHUNK_SIZE as i32 {
            n_outside += 1;
        }
        if z == -1 || z == CHUNK_SIZE as i32 {
            n_outside += 1;
        }

        if n_outside == 1 {
            if x == CHUNK_SIZE as i32 {
                return self.faces[0][y as usize][z as usize];
            } else if x == -1 {
                return self.faces[1][y as usize][z as usize];
            } else if y == CHUNK_SIZE as i32 {
                return self.faces[2][x as usize][z as usize];
            } else if y == -1 {
                return self.faces[3][x as usize][z as usize];
            } else if z == CHUNK_SIZE as i32 {
                return self.faces[4][x as usize][y as usize];
            } else if z == -1 {
                return self.faces[5][x as usize][y as usize];
            }
        } else if n_outside == 2 {
            if x >= 0 && x < CHUNK_SIZE as i32 {
                let i = delta(y) * 2 + delta(z);
                return self.edges[i][x as usize];
            } else if y >= 0 && y < CHUNK_SIZE as i32 {
                let i = delta(x) * 2 + delta(z);
                return self.edges[i + 4][y as usize];
            } else if z >= 0 && z < CHUNK_SIZE as i32 {
                let i = delta(x) * 2 + delta(y);
                return self.edges[i + 8][z as usize];
            }
        } else if n_outside == 3 {
            let i = delta(x) * 4 + delta(y) * 2 + delta(z);
            return self.corners[i];
        }
        unreachable!();
    }
}

impl AdjChunkLight {
    /// Generate the AdjChunkOccl struct used in the meshing containing the
    /// informations about adjacent chunks
    pub fn create_from_world(
        world: &World,
        pos: ChunkPos,
    ) -> AdjChunkLight {
        const ICHUNK_SIZE: i64 = CHUNK_SIZE as i64;
        // Transform every number to 0 except -1.
        // This allows us to transform chunk deltas into block deltas.
        #[inline(always)]
        fn f(x: i64) -> i64 {
            if x == -1 {
                -1
            } else {
                0
            }
        }
        // faces
        let da = [
            [1, 0, 0],
            [-1, 0, 0],
            [0, 1, 0],
            [0, -1, 0],
            [0, 0, 1],
            [0, 0, -1],
        ];
        let mut faces = [[[0; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6];
        for i in 0..6 {
            faces[i] = match world.get_light_chunk(pos.offset_by_pos(da[i].into())) {
                Some(chunk_light) => {
                    let mut res = [[15; CHUNK_SIZE as usize]; CHUNK_SIZE as usize];
                    for j in 0..CHUNK_SIZE {
                        for k in 0..CHUNK_SIZE {
                            let (ux, uy, uz);
                            if i / 2 == 0 {
                                ux = (ICHUNK_SIZE + f(da[i][0])) as u32 % CHUNK_SIZE;
                                uy = j;
                                uz = k;
                            } else if i / 2 == 1 {
                                ux = j;
                                uy = (ICHUNK_SIZE + f(da[i][1])) as u32 % CHUNK_SIZE;
                                uz = k;
                            } else {
                                ux = j;
                                uy = k;
                                uz = (ICHUNK_SIZE + f(da[i][2])) as u32 % CHUNK_SIZE;
                            }
                            res[j as usize][k as usize] = chunk_light.get_light_at((ux,uy, uz));
                        }
                    }
                    res
                }
                None => [[15; CHUNK_SIZE as usize]; CHUNK_SIZE as usize], // no chunk => full light
            };
        }

        AdjChunkLight {
            faces,
        }
    }
    /// Return the light at some position (only on the face)
    pub fn get_light(&self, x: i32, y: i32, z: i32) -> u8 {

        let mut n_outside = 0;
        if x == -1 || x == CHUNK_SIZE as i32 {
            n_outside += 1;
        }
        if y == -1 || y == CHUNK_SIZE as i32 {
            n_outside += 1;
        }
        if z == -1 || z == CHUNK_SIZE as i32 {
            n_outside += 1;
        }

        if n_outside == 1 {
            if x == CHUNK_SIZE as i32 {
                return self.faces[0][y as usize][z as usize];
            } else if x == -1 {
                return self.faces[1][y as usize][z as usize];
            } else if y == CHUNK_SIZE as i32 {
                return self.faces[2][x as usize][z as usize];
            } else if y == -1 {
                return self.faces[3][x as usize][z as usize];
            } else if z == CHUNK_SIZE as i32 {
                return self.faces[4][x as usize][y as usize];
            } else if z == -1 {
                return self.faces[5][x as usize][y as usize];
            }else{
                unreachable!()
            }
        }
        unreachable!()
    }

}


const D: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Return True if full block (taking into account adjacent chunks)
fn is_full(
    chunk: &Chunk,
    (i, j, k): (i32, i32, i32),
    adj: &Option<AdjChunkOccl>,
    meshes: &Vec<BlockMesh>,
) -> bool {
    let size = CHUNK_SIZE as i32;
    if i >= 0 && j >= 0 && k >= 0 && i < size && j < size && k < size {
        return meshes[chunk.get_block_at((i as u32, j as u32, k as u32)) as usize].is_opaque();
    } else {
        match adj {
            Some(_adj) => _adj.is_full(i, j, k),
            None => false,
        }
    }
}

/// Return true if pos (x,y,z) is in block (i,j,k)
fn _in_block((i, j, k): (i32, i32, i32), (x, y, z): (f32, f32, f32)) -> bool {
    let dx = x - i as f32;
    let dy = y - j as f32;
    let dz = z - k as f32;
    dx >= 0.0 && dx <= 1.0 && dy >= 0.0 && dy <= 1.0 && dz >= 0.0 && dz <= 1.0
}

/// Ambient occlusion code (cf : https://0fps.net/2013/07/03/ambient-occlusion-for-minecraft-like-worlds/)
fn ambiant_occl(corners: u32, edge: u32) -> u32 {
    if edge == 2 {
        return 0;
    } else if edge == 1 && corners == 1 {
        return 1;
    } else if edge + corners == 1 {
        return 2;
    } else {
        return 3;
    }
}

/// Greedy meshing : compressed adjacent quads, return the number of uncompressed and compressed quads
///
/// `quads`: Buffer that is reused every time.
pub fn greedy_meshing(
    chunk: &Chunk,
    light_chunk: &LightChunk,
    adj: Option<AdjChunkOccl>,
    adj_light : AdjChunkLight,
    meshes: &Vec<BlockMesh>,
    quads: &mut Vec<Quad>,
) -> (Vec<Vertex>, Vec<u32>, u32, u32) {
    let mut res_vertex: Vec<Vertex> = Vec::new();
    let mut res_index: Vec<usize> = Vec::new();

    let mut tot_quad = 0;
    let mut act_quad = 0;

    let mut n_of_different_vertex = 0;

    const N_SIZE: usize = (CHUNK_SIZE + 2) as usize;
    let mut chunk_mask = [false; N_SIZE * N_SIZE * N_SIZE];

    #[inline(always)]
    fn ind(x: i32, y: i32, z: i32) -> usize {
        let (a, b, c) = (x as usize, y as usize, z as usize);
        (a * N_SIZE * N_SIZE + b * N_SIZE + c) as usize
    }

    const IN_SIZE: i32 = N_SIZE as i32;
    for i in 0..IN_SIZE {
        for j in 0..IN_SIZE {
            for k in 0..IN_SIZE {
                if i == 0
                    || i == IN_SIZE - 1
                    || j == 0
                    || j == IN_SIZE - 1
                    || k == 0
                    || k == IN_SIZE - 1
                {
                    chunk_mask[ind(i, j, k)] = is_full(chunk, (i - 1, j - 1, k - 1), &adj, meshes);
                }
            }
        }
    }


    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                chunk_mask[ind(i as i32 + 1, j as i32 + 1, k as i32 + 1)] =
                    meshes[chunk.get_block_at((i, j, k)) as usize].is_opaque();
            }
        }
    }

    // Generating the light levels
    let mut light_levels = [15; N_SIZE * N_SIZE * N_SIZE];
    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                light_levels[ind(i as i32+1, j as i32+1, k as i32+1)] = light_chunk.get_light_at((i, j, k));
            }
        }
    }
    for i in 0..IN_SIZE {
        for j in 0..IN_SIZE {
            for k in 0..IN_SIZE {
                if ((i == 0
                    || i == IN_SIZE - 1)
                    ^ (j == 0
                    || j == IN_SIZE - 1)
                     ^ (k == 0
                    || k == IN_SIZE - 1))
                    && !((i == 0
                    || i == IN_SIZE - 1)
                    && (j == 0
                    || j == IN_SIZE - 1)
                    && (k == 0
                    || k == IN_SIZE - 1
                    )) // exclusive or with 3 value (check if only on a face and note an edge or corner)
                {
                    light_levels[ind(i as i32 , j as i32, k as i32)] = adj_light.get_light(i as i32 -1 , j as i32 - 1, k as i32 - 1);
                }
            }
        }
    }


    const D_DELTA0: [[i32; 3]; 6] = [
        [1, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [0, 0, 1],
    ];
    const D_DELTA1: [[i32; 3]; 6] = [
        [0, 1, 0],
        [0, 1, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
    ];
    const D_DELTA2: [[i32; 3]; 6] = [
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 1, 0],
        [0, 1, 0],
    ];

    quads.resize(
        6 * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize,
        Quad::default(),
    );
    let mut to_mesh =
        unsafe { zero_initialized_vec(6 * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize) };

    #[inline(always)]
    fn ind_mesh(s: usize, x: i32, y: i32, z: i32) -> usize {
        let (s2, a, b, c) = (s as u32, x as u32, y as u32, z as u32);
        (((s2 * CHUNK_SIZE + a) * CHUNK_SIZE + b) * CHUNK_SIZE + c) as usize
    }

    for s in 0..6 {
        // each direction
        for i in 0..(CHUNK_SIZE as i32) {
            for j in 0..(CHUNK_SIZE as i32) {
                for k in 0..(CHUNK_SIZE as i32) {
                    if chunk_mask[ind(i + 1, j + 1, k + 1)] {
                        //checking if not void
                        if !chunk_mask[ind(i + 1 + D[s][0], j + 1 + D[s][1], k + 1 + D[s][2])] {
                            let mut coins = [0; 4];
                            let mut edge = [0; 4];

                            for i2 in -1..=1 {
                                for j2 in -1..=1 {
                                    let dx =
                                        1 + D[s][0] + D_DELTA1[s][0] * i2 + D_DELTA2[s][0] * j2;
                                    let dy =
                                        1 + D[s][1] + D_DELTA1[s][1] * i2 + D_DELTA2[s][1] * j2;
                                    let dz =
                                        1 + D[s][2] + D_DELTA1[s][2] * i2 + D_DELTA2[s][2] * j2;

                                    if chunk_mask[ind(i + dx, j + dy, k + dz)] {
                                        match (i2, j2) {
                                            (-1, -1) => {
                                                coins[0] += 1;
                                            }
                                            (-1, 1) => {
                                                coins[1] += 1;
                                            }
                                            (1, -1) => {
                                                coins[2] += 1;
                                            }
                                            (1, 1) => {
                                                coins[3] += 1;
                                            }
                                            (-1, 0) => {
                                                edge[0] += 1;
                                                edge[1] += 1;
                                            }
                                            (1, 0) => {
                                                edge[2] += 1;
                                                edge[3] += 1;
                                            }
                                            (0, -1) => {
                                                edge[0] += 1;
                                                edge[2] += 1;
                                            }
                                            (0, 1) => {
                                                edge[1] += 1;
                                                edge[3] += 1;
                                            }
                                            _ => (),
                                        }
                                    }
                                }
                            }

                            let light_level = light_levels
                                [ind(i + 1 + D[s][0], j + 1 + D[s][1], k + 1 + D[s][2])];
                            let quad = Quad {
                                v1: (s as u32)
                                    + (ambiant_occl(coins[0], edge[0]) << 3)
                                    + ((light_level as u32)
                                    << 5),
                                v2: (s as u32)
                                    + (ambiant_occl(coins[1], edge[1]) << 3)
                                    + ((light_level as u32)
                                    << 5),
                                v3: (s as u32)
                                    + (ambiant_occl(coins[2], edge[2]) << 3)
                                    + ((light_level as u32)
                                    << 5),
                                v4: (s as u32)
                                    + (ambiant_occl(coins[3], edge[3]) << 3)
                                    + ((light_level as u32)
                                    << 5),
                                block_id: chunk.get_block_at((i as u32, j as u32, k as u32)),
                            };
                            quads[ind_mesh(s, i, j, k)] = quad;
                            to_mesh[ind_mesh(s, i, j, k)] = true;
                            tot_quad += 1;
                        }
                    }
                }
            }
        }
    }

    let order1 = [
        [0, 2, 1, 1, 2, 3],
        [0, 1, 2, 1, 3, 2],
        [0, 1, 2, 1, 3, 2],
        [0, 2, 1, 1, 2, 3],
        [3, 1, 2, 2, 1, 0],
        [3, 2, 1, 2, 0, 1],
    ];

    let order2 = [
        [0, 2, 3, 0, 3, 1],
        [0, 3, 2, 0, 1, 3],
        [0, 3, 2, 0, 1, 3],
        [0, 2, 3, 0, 3, 1],
        [1, 0, 3, 2, 3, 0],
        [1, 3, 0, 2, 0, 3],
    ];

    let uvs = [
        [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
        [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]],
        [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
        [[1.0, 1.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
    ];
    let uv_directions = [[1, 0], [1, 0], [0, 1], [0, 1], [0, 1], [0, 1]];

    for s in 0..6 {
        // each direction

        #[inline(always)]
        fn ijk_to_pos(s: usize, i: i32, j: i32, k: i32) -> (i32, i32, i32) {
            let x = i * D_DELTA0[s][0] + j * D_DELTA1[s][0] + k * D_DELTA2[s][0];
            let y = i * D_DELTA0[s][1] + j * D_DELTA1[s][1] + k * D_DELTA2[s][1];
            let z = i * D_DELTA0[s][2] + j * D_DELTA1[s][2] + k * D_DELTA2[s][2];
            (x, y, z)
        };

        for i in 0..(CHUNK_SIZE as i32) {
            // x x y y z z
            for j in 0..(CHUNK_SIZE as i32) {
                // y y x x x x
                for k in 0..(CHUNK_SIZE as i32) {
                    // z z z z y y
                    let (px, py, pz) = ijk_to_pos(s, i, j, k);
                    if to_mesh[ind_mesh(s, px, py, pz)] {
                        to_mesh[ind_mesh(s, px, py, pz)] = false;
                        let current_quad = quads[ind_mesh(s, px, py, pz)];
                        let mut j_end = j + 1; // + y + x + x
                        let mut k_end = k + 1; // +z + z + x

                        if current_quad.v1 == current_quad.v3 && current_quad.v2 == current_quad.v4
                        {
                            // meshing along j
                            let mut j2 = j + 1;
                            let mut pos = ijk_to_pos(s, i, j2, k);

                            while j2 < CHUNK_SIZE as i32
                                && to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)]
                            {
                                let next_quad = quads[ind_mesh(s, pos.0, pos.1, pos.2)];
                                if next_quad.v1 == current_quad.v1
                                    && next_quad.v2 == current_quad.v2
                                    && next_quad.v1 == next_quad.v3
                                    && next_quad.v2 == next_quad.v4
                                    && current_quad.block_id == next_quad.block_id
                                {
                                    to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)] = false;
                                    j2 += 1;
                                    pos = ijk_to_pos(s, i, j2, k);
                                } else {
                                    break;
                                }
                            }
                            j_end = j2;

                            if current_quad.v1 == current_quad.v2 {
                                // 2D greedy meshing
                                let mut k2 = k + 1;
                                'wloop: while k2 < CHUNK_SIZE as i32 {
                                    for j3 in j..j_end {
                                        let pos = ijk_to_pos(s, i, j3, k2);
                                        let next_quad = quads[ind_mesh(s, pos.0, pos.1, pos.2)];
                                        if !(to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)]
                                            && next_quad.is_same()
                                            && next_quad.v1 == current_quad.v1
                                            && next_quad.block_id == current_quad.block_id)
                                        {
                                            break 'wloop;
                                        }
                                    }

                                    for j3 in j..j_end {
                                        let pos = ijk_to_pos(s, i, j3, k2);
                                        to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)] = false;
                                    }
                                    k2 += 1;
                                }
                                k_end = k2;
                            }
                        } else if current_quad.v1 == current_quad.v2
                            && current_quad.v3 == current_quad.v4
                        {
                            // meshing along k
                            let mut k2 = k + 1;
                            let mut pos = ijk_to_pos(s, i, j, k2);
                            while k2 < CHUNK_SIZE as i32
                                && to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)]
                            {
                                let next_quad = quads[ind_mesh(s, pos.0, pos.1, pos.2)];
                                if next_quad.v1 == current_quad.v1
                                    && next_quad.v3 == current_quad.v3
                                    && next_quad.v1 == next_quad.v2
                                    && next_quad.v3 == next_quad.v4
                                    && next_quad.block_id == current_quad.block_id
                                {
                                    to_mesh[ind_mesh(s, pos.0, pos.1, pos.2)] = false;
                                    k2 += 1;
                                    pos = ijk_to_pos(s, i, j, k2);
                                } else {
                                    break;
                                }
                            }
                            k_end = k2;
                        }

                        let (px2, py2, pz2) = ijk_to_pos(s, i, j, k_end);
                        let (px3, py3, pz3) = ijk_to_pos(s, i, j_end, k);
                        let (px4, py4, pz4) = ijk_to_pos(s, i, j_end, k_end);

                        let mut px_ = [px as f32, px2 as f32, px3 as f32, px4 as f32];
                        let mut py_ = [py as f32, py2 as f32, py3 as f32, py4 as f32];
                        let mut pz_ = [pz as f32, pz2 as f32, pz3 as f32, pz4 as f32];
                        let v = [
                            current_quad.v1,
                            current_quad.v2,
                            current_quad.v3,
                            current_quad.v4,
                        ];

                        if s == 0 {
                            // 1x
                            for kk in 0..4 {
                                px_[kk] = px_[kk] + 1.0;
                            }
                        } else if s == 2 {
                            // 1y
                            for kk in 0..4 {
                                py_[kk] = py_[kk] + 1.0;
                            }
                        } else if s == 4 {
                            // 1z
                            for kk in 0..4 {
                                pz_[kk] = pz_[kk] + 1.0;
                            }
                        }

                        let uv = match meshes[current_quad.block_id as usize] {
                            BlockMesh::Empty => continue,
                            BlockMesh::FullCube { textures } => textures[s],
                        };

                        let uv_pos = [uv.x, uv.y];
                        let uv_size = [uv.width, uv.height];
                        let uv_factors = [(j_end - j) as f32, (k_end - k) as f32];
                        let uv_factors = [
                            uv_factors[uv_directions[s][0]],
                            uv_factors[uv_directions[s][1]],
                        ];
                        let uvs = [
                            [
                                uvs[s][0][0] * uv.width * uv_factors[0],
                                uvs[s][0][1] * uv.height * uv_factors[1],
                            ],
                            [
                                uvs[s][1][0] * uv.width * uv_factors[0],
                                uvs[s][1][1] * uv.height * uv_factors[1],
                            ],
                            [
                                uvs[s][2][0] * uv.width * uv_factors[0],
                                uvs[s][2][1] * uv.height * uv_factors[1],
                            ],
                            [
                                uvs[s][3][0] * uv.width * uv_factors[0],
                                uvs[s][3][1] * uv.height * uv_factors[1],
                            ],
                        ];

                        for kk in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [px_[kk], py_[kk], pz_[kk]],
                                uv_pos,
                                uv_offset: uvs[kk],
                                uv_size,
                                normal: v[kk],
                            });
                        }

                        let a00 = v[0] >> 3;
                        let a11 = v[3] >> 3;
                        let a01 = v[1] >> 3;
                        let a10 = v[2] >> 3;

                        for kk in 0..6 {
                            if a00 + a11 < a01 + a10 {
                                res_index.push(n_of_different_vertex + order1[s][kk]);
                            } else {
                                res_index.push(n_of_different_vertex + order2[s][kk]);
                            }
                        }
                        n_of_different_vertex += 4;
                        act_quad += 1;
                    }
                }
            }
        }
    }

    let res_index: Vec<u32> = res_index.iter().map(|x| *x as u32).collect();
    (res_vertex, res_index, tot_quad, act_quad)
}

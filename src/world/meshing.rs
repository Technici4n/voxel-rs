use super::chunk::{Chunk, CHUNK_SIZE};
use super::renderer::Vertex;
use crate::block::mesh::BlockMesh;

// The constant associated to the normal direction
/*
const EAST: u32 = 0;
// 1x
const WEST: u32 = 1;
// -1x
const UP: u32 = 2;
// 1y
const DOWN: u32 = 3;
// -1y
const SOUTH: u32 = 4;
// 1z
const NORTH: u32 = 5; // -1z*/

/// Structure containing information about adjacent chunks for the meshing
/// Order of face 1x, -1x, 1y, -1y, 1z, -1z => the two order component are in the (x,y,z) order
/// Order of edges (yz), (y-z), (-y z), (-y - z), (xz), (x -z), (-x z), (x - z), (xy), (x - y), (-x y) (-x - y)
/// ( xy means variation along z with x, y = (1+chunk_size, 1+chunk_size), -x y means variation along z with x, y= (-1, 1)
/// Order of coins (1,1,1), (1, 1 -1), (1, -1, 1), (1, -1, -1),
///  ... (-1,1,1), (-1, 1 -1), (-1, -1, 1), (-1, -1, -1),
#[derive(Clone, Copy)]
pub struct AdjChunkOccl {
    pub faces: [[[bool; CHUNK_SIZE as usize]; CHUNK_SIZE as usize]; 6],
    pub edges: [[bool; CHUNK_SIZE as usize]; 12],
    pub coins: [bool; 8],
}

#[derive(Clone, Copy, Default)]
struct Quad {
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

fn delta(x: i32) -> usize {
    if x == CHUNK_SIZE as i32 {
        0
    } else if x == -1 {
        1
    } else {
        0 // unreachable
    }
}

impl AdjChunkOccl {
    /// x, y, z are the position relative to the chunk (0, 0, 0)
    /// Return if the block outside the chunk is opaque
    pub fn is_full(&self, x: i32, y: i32, z: i32) -> bool {
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
            return self.coins[i];
        }
        return false;
    }
}

const MESH_EAST: [[f32; 3]; 4] = [
    [1.0, 0.0, 0.0],
    [1.0, 1.0, 0.0],
    [1.0, 0.0, 1.0],
    [1.0, 1.0, 1.0],
];

const MESH_EAST_INDEX: [usize; 6] = [0, 1, 2, 2, 1, 3];

const MESH_WEST: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 1.0, 1.0],
];

const MESH_WEST_INDEX: [usize; 6] = [0, 2, 1, 2, 3, 1];

const MESH_UP: [[f32; 3]; 4] = [
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
    [0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];

const MESH_UP_INDEX: [usize; 6] = [0, 2, 1, 2, 3, 1];

const MESH_DOWN: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
];

const MESH_DOWN_INDEX: [usize; 6] = [0, 1, 2, 2, 1, 3];

const MESH_NORTH: [[f32; 3]; 4] = [
    [0.0, 0.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [1.0, 1.0, 0.0],
];

const MESH_NORTH_INDEX: [usize; 6] = [0, 2, 1, 2, 3, 1];

const MESH_SOUTH: [[f32; 3]; 4] = [
    [0.0, 0.0, 1.0],
    [1.0, 0.0, 1.0],
    [0.0, 1.0, 1.0],
    [1.0, 1.0, 1.0],
];
const MESH_SOUTH_INDEX: [usize; 6] = [0, 1, 2, 2, 1, 3];

const D: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];
const MESH_DIR: [[[f32; 3]; 4]; 6] = [
    MESH_EAST, MESH_WEST, MESH_UP, MESH_DOWN, MESH_SOUTH, MESH_NORTH,
];
const MESH_INDEX: [[usize; 6]; 6] = [
    MESH_EAST_INDEX,
    MESH_WEST_INDEX,
    MESH_UP_INDEX,
    MESH_DOWN_INDEX,
    MESH_SOUTH_INDEX,
    MESH_NORTH_INDEX,
];

// good luck understanding this, future me !
const OCC_POS_CHECK: [[[(i32, i32, i32, bool); 3]; 4]; 6] = [
    [
        [(1, -1, -1, false), (1, -1, 0, true), (1, 0, -1, true)],
        [(1, 0, -1, true), (1, 1, -1, false), (1, 1, 0, true)],
        [(1, -1, 0, true), (1, -1, 1, false), (1, 0, 1, true)],
        [(1, 0, 1, true), (1, 1, 0, true), (1, 1, 1, false)],
    ],
    [
        [(-1, -1, -1, false), (-1, -1, 0, true), (-1, 0, -1, true)],
        [(-1, 0, -1, true), (-1, 1, -1, false), (-1, 1, 0, true)],
        [(-1, -1, 0, true), (-1, -1, 1, false), (-1, 0, 1, true)],
        [(-1, 0, 1, true), (-1, 1, 0, true), (-1, 1, 1, false)],
    ],
    [
        [(-1, 1, -1, false), (-1, 1, 0, true), (0, 1, -1, true)],
        [(0, 1, -1, true), (1, 1, -1, false), (1, 1, 0, true)],
        [(-1, 1, 0, true), (-1, 1, 1, false), (0, 1, 1, true)],
        [(0, 1, 1, true), (1, 1, 0, true), (1, 1, 1, false)],
    ],
    [
        [(-1, -1, -1, false), (-1, -1, 0, true), (0, -1, -1, true)],
        [(0, -1, -1, true), (1, -1, -1, false), (1, -1, 0, true)],
        [(-1, -1, 0, true), (-1, -1, 1, false), (0, -1, 1, true)],
        [(0, -1, 1, true), (1, -1, 0, true), (1, -1, 1, false)],
    ],
    [
        [(-1, -1, 1, false), (-1, 0, 1, true), (0, -1, 1, true)],
        [(0, -1, 1, true), (1, -1, 1, false), (1, 0, 1, true)],
        [(-1, 0, 1, true), (-1, 1, 1, false), (0, 1, 1, true)],
        [(0, 1, 1, true), (1, 0, 1, true), (1, 1, 1, false)],
    ],
    [
        [(-1, -1, -1, false), (-1, 0, -1, true), (0, -1, -1, true)],
        [(0, -1, -1, true), (1, -1, -1, false), (1, 0, -1, true)],
        [(-1, 0, -1, true), (-1, 1, -1, false), (0, 1, -1, true)],
        [(0, 1, -1, true), (1, 0, -1, true), (1, 1, -1, false)],
    ],
];

/// Return True if full block (taking into account adjacent chunks)
fn is_full(
    chunk: &Chunk,
    (i, j, k): (i32, i32, i32),
    adj: Option<AdjChunkOccl>,
    meshes: &Vec<BlockMesh>,
) -> bool {
    let size = CHUNK_SIZE as i32;
    if i >= 0 && j >= 0 && k >= 0 && i < size && j < size && k < size {
        return meshes[chunk.get_data(i as u32, j as u32, k as u32) as usize].is_opaque();
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
fn ambiant_occl(coins: u32, edge: u32) -> u32 {
    if edge == 2 {
        return 0;
    } else if edge == 1 && coins == 1 {
        return 1;
    } else if edge + coins == 1 {
        return 2;
    } else {
        return 3;
    }
}

/*
/// Return a list of vertex a (3*n) indexes array (for n quads)
/// which contains the index of the corresponding quads
/// in the first array
/// Each vertex contains its position and the normal associated to the quad
pub fn meshing(chunk: &Chunk, adj: Option<AdjChunkOccl>) -> (Vec<Vertex>, Vec<u32>) {
    let mut res_vertex: Vec<Vertex> = Vec::new();
    let mut res_index: Vec<usize> = Vec::new();

    let mut n_of_different_vertex = 0;

    /*
    let d_delta1 = [[0, 1, 0], [0, 1, 0], [1, 0, 0], [1, 0, 0], [1, 0, 0], [1, 0, 0]];
    let d_delta2 = [[0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 0, 1], [0, 1, 0], [0, 1, 0]];
    let mut occ_pos_check: [[Vec<(i32, i32, i32, bool)>; 4]; 6] = Default::default();

    for i in 0..6 {
        for j in 0..4 {
            let px = mesh_dir[i][j][0];
            let py = mesh_dir[i][j][1];
            let pz = mesh_dir[i][j][2];
            for delta1 in -1..=1 {
                for delta2 in -1..=1 {
                    if delta1 != delta2 || delta1 != 0 {
                        let d1 = D[i][0] + delta1 * d_delta1[i][0] + delta2 * d_delta2[i][0];
                        let d2 = D[i][1] + delta1 * d_delta1[i][1] + delta2 * d_delta2[i][1];
                        let d3 = D[i][2] + delta1 * d_delta1[i][2] + delta2 * d_delta2[i][2];
                        if in_block((d1, d2, d3), (px, py, pz)) {
                            occ_pos_check[i][j].push((d1, d2, d3, (delta1.abs() + delta2.abs()) == 1));
                        }
                    }
                }
            }
        }
    }
    dbg!(&occ_pos_check); => code used to generate the OCC_POS_CHECK struct*/

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
                    chunk_mask[ind(i, j, k)] = is_full(chunk, (i - 1, j - 1, k - 1), adj);
                }
            }
        }
    }

    const UCHUNK_LEN: usize = super::chunk::CHUNK_LEN as usize;
    const UN_SIZE: usize = N_SIZE as usize;
    for i in 0..UCHUNK_LEN {
        for j in 0..UCHUNK_LEN {
            for k in 0..UCHUNK_LEN {
                let index = (i * UCHUNK_LEN + j) * UCHUNK_LEN + k;
                let world_index = ((2 * i + 1) * UN_SIZE + 2 * j + 1) * UN_SIZE + 2 * k + 1;
                use super::chunk::BlockGroup;
                match &chunk.data[index] {
                    BlockGroup::Compressed(bxz, bxZ, bXz, bXZ) => {
                        let obs = [*bxz != 0, *bxZ != 0, *bXz != 0, *bXZ != 0];
                        for i2 in 0..2 {
                            for k2 in 0..2 {
                                if obs[i2 * 2 + k2] {
                                    for j2 in 0..2 {
                                        chunk_mask[world_index
                                            + UN_SIZE * UN_SIZE * i2
                                            + UN_SIZE * j2
                                            + k2] = true;
                                    }
                                }
                            }
                        }
                    }
                    BlockGroup::Uncompressed(data) => {
                        for i2 in 0..2 {
                            for j2 in 0..2 {
                                for k2 in 0..2 {
                                    chunk_mask[world_index
                                        + UN_SIZE * UN_SIZE * i2
                                        + UN_SIZE * j2
                                        + k2] = data[i2 * 4 + j2 * 2 + k2] != 0;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    for i in 0..(CHUNK_SIZE as i32) {
        for j in 0..(CHUNK_SIZE as i32) {
            for k in 0..(CHUNK_SIZE as i32) {
                if chunk_mask[ind(i + 1, j + 1, k + 1)] {
                    //checking if not void

                    for s in 0..6 {
                        // each direction
                        if !chunk_mask[ind(i + 1 + D[s][0], j + 1 + D[s][1], k + 1 + D[s][2])] {
                            for l in 0..4 {
                                let px = i as f32 + MESH_DIR[s][l][0];
                                let py = j as f32 + MESH_DIR[s][l][1];
                                let pz = k as f32 + MESH_DIR[s][l][2];
                                res_vertex.push(Vertex {
                                    pos: [px, py, pz],
                                    normal: (s as u32)
                                        + ({
                                            let mut coins = 0;
                                            let mut edge = 0;
                                            for (p1, p2, p3, is_edge) in OCC_POS_CHECK[s][l].iter()
                                            {
                                                if chunk_mask
                                                    [ind(i + 1 + *p1, j + 1 + *p2, k + 1 + *p3)]
                                                {
                                                    if *is_edge {
                                                        edge += 1;
                                                    } else {
                                                        coins += 1;
                                                    }
                                                }
                                            }
                                            ambiant_occl(coins, edge)
                                        } << 3),
                                });
                            }

                            for l in 0..6 {
                                res_index.push(n_of_different_vertex + MESH_INDEX[s][l]);
                            }
                            n_of_different_vertex += 4;
                        }
                    }
                }
            }
        }
    }

    let res_index: Vec<u32> = res_index.iter().map(|x| *x as u32).collect();
    (res_vertex, res_index)
}
*/

// Greedy meshing : compressed adjacent quads, return the number of uncompressed and compressed quads
pub fn greedy_meshing(
    chunk: &Chunk,
    adj: Option<AdjChunkOccl>,
    meshes: &Vec<BlockMesh>,
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
                    chunk_mask[ind(i, j, k)] = is_full(chunk, (i - 1, j - 1, k - 1), adj, meshes);
                }
            }
        }
    }

    const UCHUNK_LEN: usize = super::chunk::CHUNK_LEN as usize;
    const UN_SIZE: usize = N_SIZE as usize;
    for i in 0..UCHUNK_LEN {
        for j in 0..UCHUNK_LEN {
            for k in 0..UCHUNK_LEN {
                let index = (i * UCHUNK_LEN + j) * UCHUNK_LEN + k;
                let world_index = ((2 * i + 1) * UN_SIZE + 2 * j + 1) * UN_SIZE + 2 * k + 1;
                use super::chunk::BlockGroup;
                match &chunk.data[index] {
                    BlockGroup::Compressed(bxz, bxzz, bxxz, bxxzz) => {
                        let obs = [
                            meshes[*bxz as usize].is_opaque(),
                            meshes[*bxzz as usize].is_opaque(),
                            meshes[*bxxz as usize].is_opaque(),
                            meshes[*bxxzz as usize].is_opaque(),
                        ];
                        for i2 in 0..2 {
                            for k2 in 0..2 {
                                if obs[i2 * 2 + k2] {
                                    for j2 in 0..2 {
                                        chunk_mask[world_index
                                            + UN_SIZE * UN_SIZE * i2
                                            + UN_SIZE * j2
                                            + k2] = true;
                                    }
                                }
                            }
                        }
                    }
                    BlockGroup::Uncompressed(data) => {
                        for i2 in 0..2 {
                            for j2 in 0..2 {
                                for k2 in 0..2 {
                                    chunk_mask[world_index
                                        + UN_SIZE * UN_SIZE * i2
                                        + UN_SIZE * j2
                                        + k2] =
                                        meshes[data[i2 * 4 + j2 * 2 + k2] as usize].is_opaque();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let d_delta0 = [
        [1, 0, 0],
        [1, 0, 0],
        [0, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [0, 0, 1],
    ];
    let d_delta1 = [
        [0, 1, 0],
        [0, 1, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
        [1, 0, 0],
    ];
    let d_delta2 = [
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 0, 1],
        [0, 1, 0],
        [0, 1, 0],
    ];

    let mut quads = vec![
        Quad {
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
            block_id: 0,
        };
        6 * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize
    ];
    let mut to_mesh = vec![false; 6 * (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];

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
                                        1 + D[s][0] + d_delta1[s][0] * i2 + d_delta2[s][0] * j2;
                                    let dy =
                                        1 + D[s][1] + d_delta1[s][1] * i2 + d_delta2[s][1] * j2;
                                    let dz =
                                        1 + D[s][2] + d_delta1[s][2] * i2 + d_delta2[s][2] * j2;

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

                            let quad = Quad {
                                v1: (s as u32) + (ambiant_occl(coins[0], edge[0]) << 3),
                                v2: (s as u32) + (ambiant_occl(coins[1], edge[1]) << 3),
                                v3: (s as u32) + (ambiant_occl(coins[2], edge[2]) << 3),
                                v4: (s as u32) + (ambiant_occl(coins[3], edge[3]) << 3),
                                block_id: chunk.get_data(i as u32, j as u32, k as u32),
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

    let order_pair = [0, 1, 2, 1, 3, 2];
    let order_impair = [0, 2, 1, 1, 2, 3];
    let order = [
        order_impair,
        order_pair,
        order_pair,
        order_impair,
        order_impair,
        order_pair,
    ];

    for s in 0..6 {
        // each direction

        let ijk_to_pos = |i: i32, j: i32, k: i32| {
            let x = i * d_delta0[s][0] + j * d_delta1[s][0] + k * d_delta2[s][0];
            let y = i * d_delta0[s][1] + j * d_delta1[s][1] + k * d_delta2[s][1];
            let z = i * d_delta0[s][2] + j * d_delta1[s][2] + k * d_delta2[s][2];
            (x, y, z)
        };

        for i in 0..(CHUNK_SIZE as i32) {
            // x x y y z z
            for j in 0..(CHUNK_SIZE as i32) {
                // y y x x x x
                for k in 0..(CHUNK_SIZE as i32) {
                    // z z z z y y
                    let (px, py, pz) = ijk_to_pos(i, j, k);
                    if to_mesh[ind_mesh(s, px, py, pz)] {
                        to_mesh[ind_mesh(s, px, py, pz)] = false;
                        let current_quad = quads[ind_mesh(s, px, py, pz)];
                        let mut j_end = j + 1; // + y + x + x
                        let mut k_end = k + 1; // +z + z + x

                        if current_quad.v1 == current_quad.v3 && current_quad.v2 == current_quad.v4
                        {
                            // meshing along j
                            let mut j2 = j + 1;
                            let mut pos = ijk_to_pos(i, j2, k);

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
                                    pos = ijk_to_pos(i, j2, k);
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
                                        let pos = ijk_to_pos(i, j3, k2);
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
                                        let pos = ijk_to_pos(i, j3, k2);
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
                            let mut pos = ijk_to_pos(i, j, k2);
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
                                    pos = ijk_to_pos(i, j, k2);
                                } else {
                                    break;
                                }
                            }
                            k_end = k2;
                        }

                        let (px2, py2, pz2) = ijk_to_pos(i, j, k_end);
                        let (px3, py3, pz3) = ijk_to_pos(i, j_end, k);
                        let (px4, py4, pz4) = ijk_to_pos(i, j_end, k_end);

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
                        let uvs = [
                            [0.0, 0.0],
                            [0.0, uv.height * (k_end - k) as f32],
                            [uv.width * (j_end - j) as f32, 0.0],
                            [
                                uv.width * (j_end - j) as f32,
                                uv.height * (k_end - k) as f32,
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

                        for kk in 0..6 {
                            res_index.push(n_of_different_vertex + order[s][kk]);
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

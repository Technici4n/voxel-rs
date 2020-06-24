//! Meshing code
use super::ChunkVertex;
use std::sync::Arc;
use voxel_rs_common::world::LightChunk;
use voxel_rs_common::{
    block::BlockMesh,
    collections::zero_initialized_vec,
    world::{Chunk, CHUNK_SIZE},
};

#[derive(Clone, Copy, Default)]
pub struct Quad {
    v1: u32,
    // i = 0 j = 0 Ex si 1x => (y, z) = 0
    v2: u32,
    // i = 0 j = 1  => (y, z) = (0, 1)
    v3: u32,
    // i = 1 j = 0 => (y, z) = (1, 0)
    v4: u32,
    // i = 1 j = 1 => (y, z) = (1, 1)
    block_id: u16,
}

impl Quad {
    fn is_same(&self) -> bool {
        return self.v1 == self.v2 && self.v2 == self.v3 && self.v3 == self.v4;
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

/// The chunk-specific data that is needed to mesh it.
pub struct ChunkMeshData {
    /// The chunk to mesh
    pub chunk: Arc<Chunk>,
    /// The chunks that are adjacent to the current chunk (the value at position 9+3+1, i.e. the current chunk, doesn't matter)
    pub all_chunks: [Option<Arc<Chunk>>; 27],
    /// The light chunk of the current chunk
    pub light_chunk: Arc<LightChunk>,
    /// The light chunks that are adjacent to the current light chunk
    pub all_light_chunks: [Option<Arc<LightChunk>>; 27],
}

/// Greedy meshing : compressed adjacent quads, return the number of uncompressed and compressed quads
///
/// `quads`: Buffer that is reused every time.
pub fn greedy_meshing(
    chunk_data: ChunkMeshData,
    meshes: &Vec<BlockMesh>,
    quads: &mut Vec<Quad>,
) -> (Vec<ChunkVertex>, Vec<u32>, u32, u32) {
    let chunk_pos = chunk_data.chunk.pos;
    let offset_x = chunk_pos.px as f32 * CHUNK_SIZE as f32;
    let offset_y = chunk_pos.py as f32 * CHUNK_SIZE as f32;
    let offset_z = chunk_pos.pz as f32 * CHUNK_SIZE as f32;

    let mut res_vertex: Vec<ChunkVertex> = Vec::new();
    let mut res_index: Vec<usize> = Vec::new();

    let mut tot_quad = 0;
    let mut act_quad = 0;

    let mut n_of_different_vertex = 0;

    const N_SIZE: usize = (CHUNK_SIZE + 2) as usize;
    let mut chunk_mask = [false; N_SIZE * N_SIZE * N_SIZE];
    let mut light_levels = [15; N_SIZE * N_SIZE * N_SIZE];

    #[inline(always)]
    fn ind(x: i32, y: i32, z: i32) -> usize {
        let (a, b, c) = (x as usize, y as usize, z as usize);
        uind(a, b, c)
    }

    #[inline(always)]
    fn uind(a: usize, b: usize, c: usize) -> usize {
        (a * N_SIZE * N_SIZE + b * N_SIZE + c) as usize
    }

    #[inline(always)]
    fn chunk_index(x: usize, y: usize, z: usize) -> usize {
        #[inline(always)]
        fn f(x: usize) -> usize {
            if x == 0 {
                0
            } else if x == N_SIZE - 1 {
                2
            } else {
                1
            }
        }
        9 * f(x) + 3 * f(y) + f(z)
    }

    #[inline(always)]
    fn outside_position(x: usize, y: usize, z: usize) -> (u32, u32, u32) {
        #[inline(always)]
        fn f(x: usize) -> u32 {
            if x == 0 {
                CHUNK_SIZE - 1
            } else if x == N_SIZE - 1 {
                0
            } else {
                x as u32 - 1
            }
        }
        (f(x), f(y), f(z))
    }

    // TODO: for light, we don't need the 8 corners

    let mut opaque_blocks_count = 0;

    for i in 0..N_SIZE {
        for j in 0..N_SIZE {
            for k in 0..N_SIZE {
                let ci = chunk_index(i, j, k);
                if ci == 13 {
                    unsafe {
                        let u_ind = uind(i, j, k);

                        let masked = (*meshes.get_unchecked(chunk_data.chunk.get_block_at_unsafe((
                            i as u32 - 1,
                            j as u32 - 1,
                            k as u32 - 1,
                        )) as usize))
                            .is_opaque();
                        // 13 = 9 + 3 + 1 is the current chunk
                        *chunk_mask.get_unchecked_mut(u_ind) = masked;

                        if masked {
                            opaque_blocks_count += 1;
                        }

                        *light_levels.get_unchecked_mut(u_ind) = chunk_data.light_chunk.get_light_at_unsafe((
                            i as u32 - 1,
                            j as u32 - 1,
                            k as u32 - 1,
                        ));
                    }
                } else {
                    unsafe {
                        if let Some(c) = &chunk_data.all_chunks[ci] {
                            *chunk_mask.get_unchecked_mut(uind(i, j, k)) =
                                (*meshes.get_unchecked(c.get_block_at_unsafe(outside_position(i, j, k)) as usize)).is_opaque();
                        }
                        if let Some(lc) = &chunk_data.all_light_chunks[ci] {
                            *light_levels.get_unchecked_mut(uind(i, j, k)) = lc.get_light_at_unsafe(outside_position(i, j, k));
                        }
                    }
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

    let mut to_mesh_faces = [0, 0, 0, 0, 0, 0];

    for s in 0..6 {
        let mut opaque_blocks_count_pass = opaque_blocks_count;
        // each direction
        'faces: for j in 0..(CHUNK_SIZE as i32) {
            for i in 0..(CHUNK_SIZE as i32) {
                for k in 0..(CHUNK_SIZE as i32) {
                    unsafe {
                        if *chunk_mask.get_unchecked(ind(i + 1, j + 1, k + 1)) {
                            opaque_blocks_count_pass -= 1;
                            *to_mesh_faces.get_unchecked_mut(s) += 1;
                            //checking if not void
                            if !*chunk_mask.get_unchecked(ind(i + 1 + D[s][0], j + 1 + D[s][1], k + 1 + D[s][2])) {
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

                                        if *chunk_mask.get_unchecked(ind(i + dx, j + dy, k + dz)) {
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

                                let light_level = *light_levels
                                    .get_unchecked(ind(i + 1 + D[s][0], j + 1 + D[s][1], k + 1 + D[s][2]));
                                let quad = Quad {
                                    v1: (s as u32)
                                        + (ambiant_occl(coins[0], edge[0]) << 3)
                                        + ((light_level as u32) << 5),
                                    v2: (s as u32)
                                        + (ambiant_occl(coins[1], edge[1]) << 3)
                                        + ((light_level as u32) << 5),
                                    v3: (s as u32)
                                        + (ambiant_occl(coins[2], edge[2]) << 3)
                                        + ((light_level as u32) << 5),
                                    v4: (s as u32)
                                        + (ambiant_occl(coins[3], edge[3]) << 3)
                                        + ((light_level as u32) << 5),
                                    block_id: chunk_data
                                        .chunk
                                        .get_block_at((i as u32, j as u32, k as u32)),
                                };
                                *quads.get_unchecked_mut(ind_mesh(s, i, j, k)) = quad;
                                *to_mesh.get_unchecked_mut(ind_mesh(s, i, j, k)) = true;
                                tot_quad += 1;
                            }
                        } else if opaque_blocks_count_pass == 0 {
                            break 'faces;
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
        unsafe fn ijk_to_pos(s: usize, i: i32, j: i32, k: i32) -> (i32, i32, i32) {
            let delta0 = D_DELTA0.get_unchecked(s);
            let delta1 = D_DELTA1.get_unchecked(s);
            let delta2 = D_DELTA2.get_unchecked(s);
            let x = i * *delta0.get_unchecked(0) + j * *delta1.get_unchecked(0) + k * *delta2.get_unchecked(0);
            let y = i * *delta0.get_unchecked(1) + j * *delta1.get_unchecked(1) + k * *delta2.get_unchecked(1);
            let z = i * *delta0.get_unchecked(2) + j * *delta1.get_unchecked(2) + k * *delta2.get_unchecked(2);
            (x, y, z)
        };


        let delta0 = D_DELTA0[s];
        let delta1 = D_DELTA1[s];
        let delta2 = D_DELTA2[s];
        let dix = delta0[0];
        let diy = delta0[1];
        let diz = delta0[2];
        let djx = delta1[0];
        let djy = delta1[1];
        let djz = delta1[2];
        let dkx = delta2[0];
        let dky = delta2[1];
        let dkz = delta2[2];

        'quads: for i in 0..(CHUNK_SIZE as i32) {
            // x x y y z z
            for j in 0..(CHUNK_SIZE as i32) {
                // y y x x x x

                for k in 0..(CHUNK_SIZE as i32) {
                    //zz zz yy

                    unsafe {
                        let px = i * dix + j * djx + k * dkx;
                        let py = i * diy + j * djy + k * dky;
                        let pz = i * diz + j * djz + k * dkz;

                        if *to_mesh.get_unchecked(ind_mesh(s, px, py, pz)) {
                            *to_mesh.get_unchecked_mut(ind_mesh(s, px, py, pz)) = false;
                            let current_quad = *quads.get_unchecked(ind_mesh(s, px, py, pz));
                            let mut j_end = j + 1; // + y + x + x
                            let mut k_end = k + 1; // +z + z + x

                            if current_quad.v1 == current_quad.v3 && current_quad.v2 == current_quad.v4
                            {
                                // meshing along j
                                let mut j2 = j + 1;
                                let mut pos = ijk_to_pos(s, i, j2, k);

                                while j2 < CHUNK_SIZE as i32
                                    && *to_mesh.get_unchecked(ind_mesh(s, pos.0, pos.1, pos.2))
                                    {
                                        let next_quad = *quads.get_unchecked(ind_mesh(s, pos.0, pos.1, pos.2));
                                        if next_quad.v1 == current_quad.v1
                                            && next_quad.v2 == current_quad.v2
                                            && next_quad.v1 == next_quad.v3
                                            && next_quad.v2 == next_quad.v4
                                            && current_quad.block_id == next_quad.block_id
                                        {
                                            *to_mesh.get_unchecked_mut(ind_mesh(s, pos.0, pos.1, pos.2)) = false;
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
                                            if !(*to_mesh.get_unchecked(ind_mesh(s, pos.0, pos.1, pos.2))
                                                && next_quad.is_same()
                                                && next_quad.v1 == current_quad.v1
                                                && next_quad.block_id == current_quad.block_id)
                                            {
                                                break 'wloop;
                                            }
                                        }

                                        for j3 in j..j_end {
                                            let pos = ijk_to_pos(s, i, j3, k2);
                                            *to_mesh.get_unchecked_mut(ind_mesh(s, pos.0, pos.1, pos.2)) = false;
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
                                    && *to_mesh.get_unchecked(ind_mesh(s, pos.0, pos.1, pos.2))
                                    {
                                        let next_quad = *quads.get_unchecked(ind_mesh(s, pos.0, pos.1, pos.2));
                                        if next_quad.v1 == current_quad.v1
                                            && next_quad.v3 == current_quad.v3
                                            && next_quad.v1 == next_quad.v2
                                            && next_quad.v3 == next_quad.v4
                                            && next_quad.block_id == current_quad.block_id
                                        {
                                            *to_mesh.get_unchecked_mut(ind_mesh(s, pos.0, pos.1, pos.2)) = false;
                                            k2 += 1;
                                            pos = ijk_to_pos(s, i, j, k2);
                                        } else {
                                            break;
                                        }
                                    }
                                k_end = k2;
                            }

                            *to_mesh_faces.get_unchecked_mut(s) -= (j_end - j)*(k_end - k);



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

                            let texture_top_left = [uv.x, uv.y];
                            let texture_size = [uv.width, uv.height];
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
                            let texture_max_uv = [uv.width * uv_factors[0], uv.height * uv_factors[1]];

                            for kk in 0..4 {
                                res_vertex.push(ChunkVertex {
                                    pos: [px_[kk] + offset_x, py_[kk] + offset_y, pz_[kk] + offset_z],
                                    texture_top_left,
                                    texture_uv: uvs[kk],
                                    texture_max_uv,
                                    texture_size,
                                    occl_and_face: v[kk],
                                });
                            }

                            let a00 = (v[0] >> 3) & 0x3;
                            let a11 = (v[3] >> 3) & 0x3;
                            let a01 = (v[1] >> 3) & 0x3;
                            let a10 = (v[2] >> 3) & 0x3;

                            for kk in 0..6 {
                                if a00 + a11 < a01 + a10 {
                                    res_index.push(n_of_different_vertex + order1[s][kk]);
                                } else {
                                    res_index.push(n_of_different_vertex + order2[s][kk]);
                                }
                            }
                            n_of_different_vertex += 4;
                            act_quad += 1;
                        } else if *to_mesh_faces.get_unchecked(s) == 0 {
                            break 'quads;
                        }
                    }
                }
            }
        }
    }

    let res_index: Vec<u32> = res_index.iter().map(|x| *x as u32).collect();
    (res_vertex, res_index, tot_quad, act_quad)
}

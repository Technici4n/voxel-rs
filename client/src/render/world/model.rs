use super::RgbVertex;
use voxel_rs_common::data::vox::VoxelModel;

/// Data structure used to draw a pre-loaded model
/// Contains the position, scale and its id in the model registry
pub struct Model {
    /// Id in the model registry
    pub mesh_id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    /// Model scaling
    pub scale: f32,
    /// Model rotation (after scaling)
    pub rot_y: f32,
    /// Offset to apply before rotating the model
    pub rot_offset: [f32; 3],
}

const D: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

#[derive(Clone, Copy, Default)]
pub struct Quad {
    v1: u32,
    // i = 0 j = 0 Ex si 1x => (y, z) = 0
    v2: u32,
    // i = 0 j = 1  => (y, z) = (0, 1)
    v3: u32,
    // i = 1 j = 0 => (y, z) = (1, 0)
    v4: u32, // i = 1 j = 1 => (y, z) = (1, 1)
}

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

pub fn mesh_model(model: &VoxelModel) -> (Vec<RgbVertex>, Vec<u32>) {
    let mut res_vertex: Vec<RgbVertex> = Vec::new();
    let mut res_index: Vec<usize> = Vec::new();

    let size_x = model.size_x;
    let size_y = model.size_y;
    let size_z = model.size_z;
    let block = &model.full;
    let color = &model.voxels;

    let n_size_x = size_x + 2;
    let n_size_y = size_y + 2;
    let n_size_z = size_z + 2;

    let ind = |x: usize, y: usize, z: usize| -> usize {
        (x * n_size_y * n_size_z + y * n_size_z + z) as usize
    };

    let ind_mesh = |s: usize, x: usize, y: usize, z: usize| -> usize {
        (s * size_x * size_y * size_z + x * size_y * size_z + y * size_z + z) as usize
    };

    let mut occl = vec![false; (n_size_x) * (n_size_y) * (n_size_z)];
    for i in 0..size_x {
        for j in 0..size_y {
            for k in 0..size_z {
                occl[ind(i + 1, j + 1, k + 1)] = block[i * size_y * size_z + j * size_z + k];
            }
        }
    }

    let mut to_mesh = vec![false; 6 * size_x * size_y * size_z];
    let mut quads: Vec<Quad> = Vec::new();
    for _i in 0..6 * size_x * size_y * size_z {
        quads.push(Quad {
            v1: 0,
            v2: 0,
            v3: 0,
            v4: 0,
        });
    }

    for s in 0..6 {
        // each direction
        for i in 0..size_x {
            for j in 0..size_y {
                for k in 0..size_z {
                    if occl[ind(i + 1, j + 1, k + 1)] {
                        //checking if not void
                        if !occl[ind(
                            i + (1 + D[s][0]) as usize,
                            j + (1 + D[s][1]) as usize,
                            k + (1 + D[s][2]) as usize,
                        )] {
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

                                    let xx = ((i as i32) + dx) as usize;
                                    let yy = ((j as i32) + dy) as usize;
                                    let zz = ((k as i32) + dz) as usize;

                                    if occl[ind(xx, yy, zz)] {
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

                            let c = color[i * size_y * size_z + j * size_z + k] & 0x00FFFFFF;
                            let quad = Quad {
                                v1: ((s as u32) << 24)
                                    + (ambiant_occl(coins[0], edge[0]) << 27)
                                    + c,
                                v2: ((s as u32) << 24)
                                    + (ambiant_occl(coins[1], edge[1]) << 27)
                                    + c,
                                v3: ((s as u32) << 24)
                                    + (ambiant_occl(coins[2], edge[2]) << 27)
                                    + c,
                                v4: ((s as u32) << 24)
                                    + (ambiant_occl(coins[3], edge[3]) << 27)
                                    + c,
                            };
                            quads[ind_mesh(s, i, j, k)] = quad;
                            to_mesh[ind_mesh(s, i, j, k)] = true;
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
    let mut n_of_different_vertex = 0;

    const DX: [[i32; 6]; 3] = [[0, 0, 0, 0, 0, 0], [0, 0, 1, 1, 1, 1], [0, 0, 1, 1, 1, 1]];

    const DY: [[i32; 6]; 3] = [[0, 0, 0, 0, 1, 1], [1, 1, 0, 0, 0, 0], [1, 1, 0, 0, 1, 1]];

    const DZ: [[i32; 6]; 3] = [[1, 1, 1, 1, 0, 0], [0, 0, 0, 0, 0, 0], [1, 1, 1, 1, 0, 0]];

    for s in 0..6 {
        // each direction

        for i in 0..size_x {
            // x x y y z z
            for j in 0..size_y {
                // y y x x x x
                for k in 0..size_z {
                    // z z z z y y
                    if to_mesh[ind_mesh(s, i, j, k)] {
                        let current_quad = quads[ind_mesh(s, i, j, k)];

                        let (px, py, pz) = (i as i32, j as i32, k as i32);
                        let (px2, py2, pz2) = (px + DX[0][s], py + DY[0][s], pz + DZ[0][s]);
                        let (px3, py3, pz3) = (px + DX[1][s], py + DY[1][s], pz + DZ[1][s]);
                        let (px4, py4, pz4) = (px + DX[2][s], py + DY[2][s], pz + DZ[2][s]);

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

                        for kk in 0..4 {
                            res_vertex.push(RgbVertex {
                                position: [px_[kk], py_[kk], pz_[kk]],
                                info: v[kk],
                            });
                        }
                        let a00 = v[0] >> 27;
                        let a11 = v[3] >> 27;
                        let a01 = v[1] >> 27;
                        let a10 = v[2] >> 27;

                        for kk in 0..6 {
                            if a00 + a11 < a01 + a10 {
                                res_index.push(n_of_different_vertex + order1[s][kk]);
                            } else {
                                res_index.push(n_of_different_vertex + order2[s][kk]);
                            }
                        }
                        n_of_different_vertex += 4;
                    }
                }
            }
        }
    }
    let res_index: Vec<u32> = res_index.iter().map(|x| *x as u32).collect();
    (res_vertex, res_index)
}

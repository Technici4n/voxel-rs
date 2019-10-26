// The constant associated to the normal direction
const EAST: u32 = 0; // 1x
const WEST: u32 = 1; // -1x
const UP: u32 = 2; // 1y
const DOWN: u32 = 3; // -1y
const SOUTH: u32 = 4; // 1z
const NORTH: u32 = 5; // -1z

use crate::chunk::{Chunk, CHUNK_SIZE};
use crate::world::Vertex;

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

/// Return a list of vertex a (3*n) indexes array (for n quads)
/// which contains the index of the corresponding quads
/// in the first array
/// Each vertex contains its position and the normal associated to the quad
pub fn meshing(chunk: &mut Chunk) -> (Vec<Vertex>, Vec<u32>) {
    let mut res_vertex: Vec<Vertex> = Vec::new();
    let mut res_index: Vec<usize> = Vec::new();

    let mut n_of_different_vertex = 0;

    for i in 0..CHUNK_SIZE {
        for j in 0..CHUNK_SIZE {
            for k in 0..CHUNK_SIZE {
                if chunk.get_data(i, j, k) != 0 {
                    //checking if not void
                    // 1x -- EAST
                    if i == CHUNK_SIZE - 1 || chunk.get_data(i + 1, j, k) == 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_EAST[l][0],
                                    j as f32 + MESH_EAST[l][1],
                                    k as f32 + MESH_EAST[l][2],
                                ],
                                normal: EAST,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_EAST_INDEX[l]);
                        }
                        n_of_different_vertex += 4;
                    }
                    // -1x -- WEST
                    if i == 0 || chunk.get_data(i - 1, j, k) == 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_WEST[l][0],
                                    j as f32 + MESH_WEST[l][1],
                                    k as f32 + MESH_WEST[l][2],
                                ],
                                normal: WEST,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_EAST_INDEX[l]);
                        }
                        n_of_different_vertex += 4;
                    }
                    // 1y -- UP
                    if j == CHUNK_SIZE - 1 || chunk.get_data(i, j + 1, k) == 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_UP[l][0],
                                    j as f32 + MESH_UP[l][1],
                                    k as f32 + MESH_UP[l][2],
                                ],
                                normal: UP,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_UP_INDEX[l]);
                        }
                        n_of_different_vertex += 4;
                    }
                    // -1y -- DOWN
                    if j == 0 || chunk.get_data(i, j - 1, k) == 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_DOWN[l][0],
                                    j as f32 + MESH_DOWN[l][1],
                                    k as f32 + MESH_DOWN[l][2],
                                ],
                                normal: DOWN,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_DOWN_INDEX[l]);
                        }
                        n_of_different_vertex += 4;
                    }
                    // 1z -- SOUTH
                    if k == CHUNK_SIZE - 1 || chunk.get_data(i, j, k + 1) == 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_SOUTH[l][0],
                                    j as f32 + MESH_SOUTH[l][1],
                                    k as f32 + MESH_SOUTH[l][2],
                                ],
                                normal: SOUTH,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_SOUTH_INDEX[l]);
                        }
                        n_of_different_vertex += 4;
                    }
                    // -1z -- NORTH
                    if k == 0 || chunk.get_data(i, j, k - 1) != 0 {
                        for l in 0..4 {
                            res_vertex.push(Vertex {
                                pos: [
                                    i as f32 + MESH_NORTH[l][0],
                                    j as f32 + MESH_NORTH[l][1],
                                    k as f32 + MESH_NORTH[l][2],
                                ],
                                normal: NORTH,
                            });
                        }

                        for l in 0..6 {
                            res_index.push(n_of_different_vertex + MESH_NORTH_INDEX[l]);
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

pub fn desindex_meshing((indexed_vertex, index): (Vec<Vertex>, Vec<usize>)) -> (Vec<Vertex>) {
    let mut res_vertex: Vec<Vertex> = Vec::new();
    for i in index.into_iter() {
        res_vertex.push(indexed_vertex[i].clone());
    }
    res_vertex
}

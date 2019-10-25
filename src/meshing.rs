const UP : u8 = 1; // 1y
const DOWN : u8 = 2; // -1y
const NORTH : u8 = 1; // -1z
const SOUTH : u8 = 1; // 1z
const EST : u8 = 1; // 1x
const WEST : u8 = 1; // -1x

use crate::chunk::{Chunk, CHUNK_SIZE};

pub struct Vertex {
    pub pos : [f32; 3],
    pub normal : u8,
}

const meshEst : [[f32; 3]; 4] =
[
[1.0, 0.0, 0.0],
[1.0, 1.0, 0.0],
[1.0, 0.0, 1.0],
[1.0, 1.0, 1.0],
];

const meshEstIndex : [usize; 6] = [0, 1, 2, 2, 1, 3];

const meshWest : [[f32; 3]; 4] =
[
[0.0, 0.0, 0.0],
[0.0, 1.0, 0.0],
[0.0, 0.0, 1.0],
[0.0, 1.0, 1.0],
];

const meshWestIndex : [usize; 6] = [0, 2, 1, 2, 3, 1];

const meshUp : [[f32; 3]; 4] =
[
[0.0, 1.0, 0.0],
[1.0, 1.0, 0.0],
[0.0, 1.0, 1.0],
[1.0, 1.0, 1.0],
];

const meshUpIndex : [usize; 6] = [0, 2, 1, 2, 3, 1];

const meshDown : [[f32; 3]; 4] =
[
[0.0, 0.0, 0.0],
[1.0, 0.0, 0.0],
[0.0, 0.0, 1.0],
[1.0, 0.0, 1.0],
];

const meshDownIndex : [usize; 6] = [0, 1, 2, 2, 1, 3];

const meshNorth : [[f32; 3]; 4] =
[
[0.0, 0.0, 0.0],
[1.0, 0.0, 0.0],
[0.0, 1.0, 0.0],
[1.0, 1.0, 0.0],
];

const meshNorthIndex : [usize; 6] = [0, 2, 1, 2, 3, 1];

const meshSouth : [[f32; 3]; 4] =
[
[0.0, 0.0, 1.0],
[1.0, 0.0, 1.0],
[0.0, 1.0, 1.0],
[1.0, 1.0, 1.0],
];
const meshSouthIndex : [usize; 6] = [0, 1, 2, 2, 1, 3];


pub fn meshing(chunk : Chunk) -> (Vec<Vertex>, Vec<usize>){
    let mut resVertex: Vec<Vertex> = Vec::new();
    let mut resIndex : Vec<usize> = Vec::new();

    let mut n_of_different_vertex = 0;

    for i in 0..CHUNK_SIZE{
        for j in 0..CHUNK_SIZE{
            for k in 0..CHUNK_SIZE{
                if chunk.get_data(i,j,k) != 0{
                    // 1x -- est
                    if i == CHUNK_SIZE - 1 || chunk.get_data(i+1,j,k) != 0{

                        for l in 0..4{
                            resVertex.push(
                                Vertex{
                                    pos: [i as f32 + meshEst[l][0],
                                          j as f32 + meshEst[l][1],
                                          k as f32 + meshEst[l][2]],
                                    normal : EST,
                                }
                            );
                        }

                        for l in 0..6{
                            resIndex.push(n_of_different_vertex + meshEstIndex[l]);
                        }
                        n_of_different_vertex += 4;


                    }
                    // -1x -- WEST
                    if i == 0 || chunk.get_data(i-1,j,k) != 0{

                        for l in 0..4{
                            resVertex.push(
                                Vertex{
                                    pos: [i as f32 + meshWest[l][0],
                                          j as f32 + meshWest[l][1],
                                          k as f32 + meshWest[l][2]],
                                    normal : WEST,
                                }
                            );
                        }

                        for l in 0..6{
                            resIndex.push(n_of_different_vertex + meshEstIndex[l]);
                        }
                        n_of_different_vertex += 4;


                    }
                    // 1y -- UP
                    if j == CHUNK_SIZE - 1 || chunk.get_data(i,j+1,k) != 0{

                        for l in 0..4{
                            resVertex.push(
                                Vertex{
                                    pos: [i as f32 + meshUp[l][0],
                                          j as f32 + meshUp[l][1],
                                          k as f32 + meshUp[l][2]],
                                    normal : UP,
                                }
                            );
                        }

                        for l in 0..6{
                            resIndex.push(n_of_different_vertex + meshUpIndex[l]);
                        }
                        n_of_different_vertex += 4;

                    }
                    // -1y -- DOWN
                    if j == 0 || chunk.get_data(i,j+1,k) != 0{

                        for l in 0..4{
                            resVertex.push(
                                Vertex{
                                    pos: [i as f32 + meshDown[l][0],
                                          j as f32 + meshDown[l][1],
                                          k as f32 + meshDown[l][2]],
                                    normal : DOWN,
                                }
                            );
                        }

                        for l in 0..6{
                            resIndex.push(n_of_different_vertex + meshDownIndex[l]);
                        }
                        n_of_different_vertex += 4;


                    }
                    // 1z -- SOUTH
                    if k == CHUNK_SIZE - 1 || chunk.get_data(i,j,k+1) != 0{
                        for l in 0..4{
                            resVertex.push(
                                Vertex{
                                    pos: [i as f32 + meshSouth[l][0],
                                          j as f32 + meshSouth[l][1],
                                          k as f32 + meshSouth[l][2]],
                                    normal : SOUTH,
                                }
                            );
                        }

                        for l in 0..6{
                            resIndex.push(n_of_different_vertex + meshSouthIndex[l]);
                        }
                        n_of_different_vertex += 4;


                    }
                    // -1z -- NORTH
                    if k == 0 || chunk.get_data(i,j,k-1) != 0{
                        if k == CHUNK_SIZE - 1 || chunk.get_data(i,j,k+1) != 0{
                            for l in 0..4{
                                resVertex.push(
                                    Vertex{
                                        pos: [i as f32 + meshNorth[l][0],
                                              j as f32 + meshNorth[l][1],
                                              k as f32 + meshNorth[l][2]],
                                        normal : UP,
                                    }
                                );
                            }

                            for l in 0..6{
                                resIndex.push(n_of_different_vertex + meshNorthIndex[l]);
                            }
                            n_of_different_vertex += 4;


                    }

                }
            }

            }
        }
    }


    (resVertex, resIndex)
}

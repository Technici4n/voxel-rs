//! Skybox rendering

use super::SkyboxVertex;

const FAR: f32 = 900.0;

const EAST: [[f32; 3]; 4] = [
    [FAR, -FAR, -FAR],
    [FAR, -FAR, FAR],
    [FAR, FAR, -FAR],
    [FAR, FAR, FAR],
];

const MESH_INDEX: [u32; 6] = [0, 1, 2, 3, 2, 1];

const WEST: [[f32; 3]; 4] = [
    [-FAR, -FAR, -FAR],
    [-FAR, -FAR, FAR],
    [-FAR, FAR, -FAR],
    [-FAR, FAR, FAR],
];

const UP: [[f32; 3]; 4] = [
    [-FAR, FAR, -FAR],
    [-FAR, FAR, FAR],
    [FAR, FAR, -FAR],
    [FAR, FAR, FAR],
];

const DOWN: [[f32; 3]; 4] = [
    [-FAR, -FAR, -FAR],
    [-FAR, -FAR, FAR],
    [FAR, -FAR, -FAR],
    [FAR, -FAR, FAR],
];

const SOUTH: [[f32; 3]; 4] = [
    [-FAR, -FAR, FAR],
    [-FAR, FAR, FAR],
    [FAR, -FAR, FAR],
    [FAR, FAR, FAR],
];

const NORTH: [[f32; 3]; 4] = [
    [-FAR, -FAR, -FAR],
    [-FAR, FAR, -FAR],
    [FAR, -FAR, -FAR],
    [FAR, FAR, -FAR],
];

const POS: [[[f32; 3]; 4]; 6] = [EAST, WEST, UP, DOWN, SOUTH, NORTH];

/// Create the vertex and the index buffer for the skybox.
pub fn create_skybox(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for i in 0..6 {
        for l in 0..4 {
            vertices.push(SkyboxVertex {
                position: [POS[i][l][0], POS[i][l][1], POS[i][l][2]],
            });
        }
        for l in 0..6 {
            indices.push(MESH_INDEX[l] + (i as u32 * 4));
        }
    }

    (
        device
            .create_buffer_mapped(vertices.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertices),
        device
            .create_buffer_mapped(indices.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&indices),
    )
}

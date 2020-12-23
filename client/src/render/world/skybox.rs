//! Skybox rendering

use wgpu::util::{BufferInitDescriptor, DeviceExt};
use super::{ SkyboxVertex, to_u8_slice };

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
        {
            let vertices_slice = to_u8_slice(&vertices);
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("skybox_vertices"),
                usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::VERTEX,
                contents: &vertices_slice
            })
        },
        {
            let indices_slice = to_u8_slice(&indices);
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("skybox_indicies"),
                usage: wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::INDEX,
                contents: &indices_slice
            })
        }
    )
}

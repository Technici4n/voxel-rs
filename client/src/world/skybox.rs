use gfx::handle::Buffer;
use gfx::traits::FactoryExt;
use gfx::Slice;
use gfx_device_gl::Factory;
use gfx_device_gl::Resources;

use super::renderer::VertexSkybox;

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

pub struct Skybox {
    pub v_buffer: Buffer<Resources, VertexSkybox>,
    pub indices: Slice<Resources>,
}

impl Skybox {
    pub fn new(factory: &mut Factory) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for i in 0..6 {
            for l in 0..4 {
                vertices.push(VertexSkybox {
                    pos: [POS[i][l][0], POS[i][l][1], POS[i][l][2]],
                });
            }
            for l in 0..6 {
                indices.push(MESH_INDEX[l] + (i as u32 * 4));
            }
        }

        let (handle, buffer) = factory.create_vertex_buffer_with_slice(&vertices, &indices[..]);

        Skybox {
            v_buffer: handle,
            indices: buffer,
        }
    }
}

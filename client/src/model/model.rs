use crate::world::renderer::VertexRGB;
use gfx::handle::Buffer;
use gfx_device_gl::Resources;

/// Data structure used to draw a predifined model
/// Contains the position, scale and its id in the model registry
pub struct Model {
    pub model_mesh_id: u32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,
    pub scale: f32,
}

/// Structure containing the model buffer
pub struct ModelBuffer {
    pub vertex_buffer: Buffer<Resources, VertexRGB>,
    pub index_buffer: Buffer<Resources, u32>,
    pub n_index: u32,
}

use crate::world::renderer::Vertex;
use gfx::handle::Buffer;
use gfx_device_gl::Resources;

// TODO : Implement mesh rotation
/// Contains the position of the mesh and the reference to the
/// vertex buffer and index buffer needed for rendering it
pub struct Mesh {
    pub pos_x: f32,
    pub pos_y: f32,
    pub pos_z: f32,

    pub vertex_buffer: Buffer<Resources, Vertex>,
    pub index_buffer: Buffer<Resources, u32>,
    /// Number of elements in the index buffer
    pub index_len: usize,
}

use gfx::traits::FactoryExt;
use crate::world::renderer::Vertex;
use gfx_device_gl::Resources;
use gfx::Slice;
use gfx::handle::Buffer;
use gfx_device_gl::Factory;

// TODO : Implement mesh rotation
/// Contains the position of the mesh and the reference to the
/// vertex buffer (and indices) needed for rendering it
pub struct Mesh{

    pub pos_x : f32,
    pub pos_y : f32,
    pub pos_z : f32,

    pub v_buffer : Buffer<Resources, Vertex>,
    pub indices : Slice<Resources>,


}

impl Mesh{

    /// Create the mesh with the position and the array of vertices (and indices)
    /// Need an gfx_device_gl::Factory
    pub fn new((x,y,z) : (f32, f32, f32), vertices: Vec<Vertex>, indices : Vec<u32>, factory : &mut Factory) -> Self {
        // Send buffer to the GPU and save the reference to these buffer
        let (handle, buffer) = factory.create_vertex_buffer_with_slice(&vertices, &indices[..]);

        Mesh{
            pos_x : x,
            pos_y : y,
            pos_z : z,
            v_buffer : handle,
            indices : buffer,
        }
    }

}
use crate::{
    mesh::Mesh,
    perlin::perlin,
    window::{ColorFormat, DepthFormat, Gfx, WindowData},
    world::{World, chunk::CHUNK_SIZE},
};
use anyhow::Result;
use gfx;
use gfx::traits::FactoryExt;
use nalgebra::{convert, Matrix4};
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx_device_gl::Resources;
use gfx::handle::Buffer;
use gfx::format::{R8_G8_B8_A8, Srgb, D24_S8, Unorm};


// TODO: add images

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "a_Pos",
        normal: u32 = "a_Norm",
    }

    constant Transform {
        view_proj: [[f32; 4]; 4] = "u_ViewProj",
        model: [[f32; 4]; 4] = "u_Model",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::ConstantBuffer<Transform> = "Transform",
        //player_data: gfx::ConstantBuffer<PlayerData> = "PlayerData",
        //image: gfx::TextureSampler<[f32; 4]> = "t_Image",
        color_buffer: gfx::RenderTarget<ColorFormat> = "ColorBuffer",
        depth_buffer: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type PipeDataType = pipe::Data<gfx_device_gl::Resources>;
type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;

pub struct WorldRenderer {
    pub pso: PsoType,
    //pub data: PipeDataType,
    pub meshes : Vec<Mesh>,
    pub transform : Buffer<Resources, Transform>,
    pub color_buffer : RenderTargetView<Resources, (R8_G8_B8_A8, Srgb)>,
    pub depth_buffer : DepthStencilView<Resources, (D24_S8, Unorm)>,
}

impl WorldRenderer {
    pub fn new(gfx: &mut Gfx) -> Result<Self> {
        use super::chunk::Chunk;
        use super::meshing::meshing;

        let Gfx {
            ref mut factory,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;
        let shader_set = factory.create_shader_set(
            include_bytes!("../../shader/world.vert"),
            include_bytes!("../../shader/world.frag"),
        )?;
        let pso = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill().with_cull_back(),
            pipe::new(),
        )?;

        let mut meshes = Vec::new(); // all the mesh to be rendered

        for i in -1..1{
            for j in -1..1{
                for k in -1..1{
                    // generating the chunk
                    let mut chunk = Chunk::new(i, j, k);
                    chunk.fill_perlin();

                    // meshing of the chunk
                    let (vertices, indices) = meshing(&mut chunk, None);
                    let pos = ((i*CHUNK_SIZE as i64) as f32, (j*CHUNK_SIZE as i64) as f32, (k*CHUNK_SIZE as i64) as f32);
                    let chunk_mesh = Mesh::new(pos, vertices, indices, factory);

                    meshes.push(chunk_mesh);
                }
            }
        }

        Ok(Self { pso, meshes,
            transform: factory.create_constant_buffer(1),
            color_buffer : color_buffer.clone(),
            depth_buffer : depth_buffer.clone() })
    }

    pub fn render(&mut self, gfx: &mut Gfx, data: &WindowData, world: &World) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        self.data.color_buffer = color_buffer.clone();
        self.data.depth_buffer = depth_buffer.clone();

        let aspect_ratio = {
            let glutin::dpi::PhysicalSize { width: win_w, height: win_h } = data.physical_window_size;
            win_w / win_h
        };

        let camera = &world.camera;
        let view_proj = convert::<Matrix4<f64>, Matrix4<f32>>(
            camera.get_view_projection(aspect_ratio), ).into();

        // drawing all the mesh
        for mesh in &self.meshes {
            let transform = Transform {
                view_proj,
                model: [ // warning matrix is transposed
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [mesh.pos_x, mesh.pos_y, mesh.pos_z, 1.0], // model matrix to account mesh position
                ],
            };

            let data = pipe::Data { // data object controlling the rendering
                    vbuf: mesh.v_buffer.clone(), // set the vertex buffer to be drawn
                    transform : self.transform.clone(),
                    color_buffer: self.color_buffer.clone(),
                    depth_buffer: self.depth_buffer.clone(),
                };


            encoder.update_buffer(&data.transform, &[transform], 0)?;
            // (index buffer, pso, full data with vertex buffer and uniform buffer inside)
            encoder.draw(&mesh.indices, &self.pso, &data);
        }

        Ok(())
    }
}

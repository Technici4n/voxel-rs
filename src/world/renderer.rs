use crate::{
    mesh::Mesh,
    window::{ColorFormat, DepthFormat, Gfx, WindowData},
    world::{chunk::CHUNK_SIZE, World},
};
use anyhow::Result;
use gfx;
use gfx::handle::Buffer;
use gfx::traits::FactoryExt;
use gfx_device_gl::Resources;
use nalgebra::{convert, Matrix4};

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

type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;

pub struct WorldRenderer {
    pub pso: PsoType,
    pub meshes: Vec<Mesh>,
    pub transform: Buffer<Resources, Transform>,
}

impl WorldRenderer {
    pub fn new(gfx: &mut Gfx) -> Result<Self> {
        use super::meshing::meshing;

        let Gfx {
            ref mut factory, ..
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
        let mut world = World::new();

        for i in -1..1 {
            for j in -1..1 {
                for k in -1..1 {
                    // generating the chunks
                    world.gen_chunk(i,j,k);
                }
            }
        }
        for chunk in world.chunks.values() {
            let (vertices, indices) = meshing(chunk,
                                              Some(world.create_adj_chunk_occl(
                                                  chunk.pos.px,chunk.pos.py, chunk.pos.pz
                                              )));
            let pos = (
                (chunk.pos.px * CHUNK_SIZE as i64) as f32,
                (chunk.pos.py * CHUNK_SIZE as i64) as f32,
                (chunk.pos.pz * CHUNK_SIZE as i64) as f32,
            );
            let chunk_mesh = Mesh::new(pos, vertices, indices, factory);
            meshes.push(chunk_mesh);
        }


        Ok(Self {
            pso,
            meshes,
            transform: factory.create_constant_buffer(1),
        })
    }

    pub fn render(&mut self, gfx: &mut Gfx, data: &WindowData, world: &World) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        let aspect_ratio = {
            let glutin::dpi::PhysicalSize {
                width: win_w,
                height: win_h,
            } = data.physical_window_size;
            win_w / win_h
        };

        let camera = &world.camera;
        let view_proj =
            convert::<Matrix4<f64>, Matrix4<f32>>(camera.get_view_projection(aspect_ratio)).into();

        // drawing all the mesh
        for mesh in &self.meshes {
            let transform = Transform {
                view_proj,
                model: [
                    // warning matrix is transposed
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [mesh.pos_x, mesh.pos_y, mesh.pos_z, 1.0], // model matrix to account mesh position
                ],
            };

            let data = pipe::Data {
                // data object controlling the rendering
                vbuf: mesh.v_buffer.clone(), // set the vertex buffer to be drawn
                transform: self.transform.clone(),
                color_buffer: color_buffer.clone(),
                depth_buffer: depth_buffer.clone(),
            };

            encoder.update_buffer(&data.transform, &[transform], 0)?;
            // (index buffer, pso, full data with vertex buffer and uniform buffer inside)
            encoder.draw(&mesh.indices, &self.pso, &data);
        }

        Ok(())
    }
}

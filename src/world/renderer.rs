use crate::{
    perlin::perlin,
    window::{ColorFormat, DepthFormat, Gfx, WindowData},
    world::World,
};
use anyhow::Result;
use gfx;
use gfx::traits::FactoryExt;
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

type PipeDataType = pipe::Data<gfx_device_gl::Resources>;
type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;

pub struct WorldRenderer {
    pub pso: PsoType,
    pub data: PipeDataType,
    pub buffer: gfx::Slice<gfx_device_gl::Resources>,
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
        let mut chunk = Chunk::new(0, 0, 0);
        for i in 0..32 {
            for j in 0..32 {
                for k in 0..32 {
                    if perlin(
                        (i as f64) / 16.0,
                        (j as f64) / 16.0,
                        (k as f64) / 16.0,
                        7,
                        0.4,
                        42,
                    ) > 0.5
                    {
                        chunk.set_data(i, j, k, 1);
                    }
                }
            }
        }
        let (vertices, indices) = meshing(&mut chunk, None);
        //  (vertex buffer handle, index buffer data)
        //  (this goes inside data, this goes in the draw() call)
        let (handle, buffer) = factory.create_vertex_buffer_with_slice(&vertices, &indices[..]);

        let data = {
            pipe::Data {
                vbuf: handle,
                transform: factory.create_constant_buffer(1),
                color_buffer: color_buffer.clone(),
                depth_buffer: depth_buffer.clone(),
            }
        };
        Ok(Self { pso, data, buffer })
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
        let transform = Transform {
            view_proj: convert::<Matrix4<f64>, Matrix4<f32>>(
                camera.get_view_projection(aspect_ratio),
            )
            .into(),
            model: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        };

        encoder.update_buffer(&self.data.transform, &[transform], 0)?;
        // (index buffer, pso, full data with vertex buffer and uniform buffer inside)
        encoder.draw(&self.buffer, &self.pso, &self.data);

        Ok(())
    }
}

use crate::perlin::perlin;
use crate::window::{ColorFormat, DepthFormat, Gfx, RenderInfo};
use crate::world::World;
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
            // TODO: cull backfaces
            gfx::state::Rasterizer::new_fill(), //.with_cull_back(),
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
                        0.5,
                        42,
                    ) > 0.5
                    {
                        chunk.set_data(i, j, k, 1);
                    }
                }
            }
        }
        let (vertices, indices) = meshing(&mut chunk);
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

    pub fn render(&self, gfx: &mut Gfx, render_info: RenderInfo, world: &World) -> Result<()> {
        let Gfx {
            ref mut encoder, ..
        } = gfx;

        let aspect_ratio = {
            let (win_w, win_h) = render_info.window_dimensions;
            win_w as f64 / win_h as f64
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
        encoder.draw(&self.buffer, &self.pso, &self.data);

        Ok(())
    }

    pub fn on_resize(
        &mut self,
        color_buffer: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        depth_buffer: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, DepthFormat>,
    ) {
        self.data.color_buffer = color_buffer;
        self.data.depth_buffer = depth_buffer;
    }
}

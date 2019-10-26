use self::transform::Camera;
use crate::window::{ColorFormat, DepthFormat, Gfx};
use anyhow::Result;
use gfx;
use gfx::traits::FactoryExt;
use log::debug;
use nalgebra::{convert, Matrix4};
use crate::perlin::perlin;

pub mod transform;

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
    pub camera: Camera,
}

impl WorldRenderer {
    pub fn new(gfx: &mut Gfx) -> Result<Self> {
        use crate::chunk::Chunk;
        use crate::meshing::{desindex_meshing, meshing};

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
                    if perlin((i as f64)/16.0, (j as f64)/16.0, (k as f64)/16.0, 7, 0.5, 42 ) > 0.5{
                            chunk.set_data(i, j, k, 1);
                    }
                }
            }
        }
        // TODO: use indexed meshing
        let (vertices, indices) = meshing(&mut chunk);
        for i in 0..10 {
            debug!("vertices[{}] = {:?}", i, vertices[i]);
        }
        let (handle, buffer) = factory.create_vertex_buffer_with_slice(&vertices, &indices[..]);

        let data = {
            pipe::Data {
                vbuf: handle,
                transform: factory.create_constant_buffer(1),
                color_buffer: color_buffer.clone(),
                depth_buffer: depth_buffer.clone(),
            }
        };
        Ok(Self {
            pso,
            data,
            buffer,
            camera: Camera::new(),
        })
    }

    pub fn render(&self, gfx: &mut Gfx) -> Result<()> {
        let Gfx {
            ref mut encoder, ..
        } = gfx;

        // TODO: refactor camera somewhere else
        let transform = Transform {
            view_proj: convert::<Matrix4<f64>, Matrix4<f32>>(self.camera.get_view_projection())
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
}

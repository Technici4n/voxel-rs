use crate::world::chunk::ChunkPos;
use crate::{
    block::mesh::BlockMesh,
    mesh::Mesh,
    window::{ColorFormat, DepthFormat, Gfx, WindowData},
    world::{camera::Camera, chunk::CHUNK_SIZE, World},
};
use anyhow::Result;
use gfx;
use gfx::handle::Buffer;
use gfx::state::RasterMethod;
use gfx::traits::{Factory, FactoryExt};
use gfx_device_gl::Resources;
use log::info;
use nalgebra::{convert, Matrix4};
use std::collections::HashMap;
use std::time::Instant;

// TODO: add images

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "a_Pos",
        uv_pos: [f32; 2] = "a_UvPos",
        uv_offset: [f32; 2] = "a_UvOffset",
        uv_size: [f32; 2] = "a_UvSize",
        normal: u32 = "a_Norm",
    }

    constant Transform {
        view_proj: [[f32; 4]; 4] = "u_ViewProj",
        model: [[f32; 4]; 4] = "u_Model",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::ConstantBuffer<Transform> = "Transform",
        texture_atlas: gfx::TextureSampler<[f32; 4]> = "TextureAtlas",
        color_buffer: gfx::RenderTarget<ColorFormat> = "ColorBuffer",
        depth_buffer: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;

pub struct WorldRenderer {
    pub pso_fill: PsoType,
    pub pso_wireframe: PsoType,
    pub chunk_meshes: HashMap<ChunkPos, Mesh>,
    pub transform: Buffer<Resources, Transform>,
    pub block_meshes: Vec<BlockMesh>,
    pub texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    pub texture_sampler: gfx::handle::Sampler<gfx_device_gl::Resources>,
}

impl WorldRenderer {
    pub fn new(
        gfx: &mut Gfx,
        world: &World,
        block_meshes: Vec<BlockMesh>,
        texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    ) -> Result<Self> {
        use super::meshing::greedy_meshing as meshing;

        let Gfx {
            ref mut factory, ..
        } = gfx;

        let shader_set = factory.create_shader_set(
            include_bytes!("../../shader/world.vert"),
            include_bytes!("../../shader/world.frag"),
        )?;
        let pso_fill = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill().with_cull_back(),
            pipe::new(),
        )?;

        let shader_set_wireframe = factory.create_shader_set(
            include_bytes!("../../shader/world.vert"),
            include_bytes!("../../shader/world_wireframe.frag"),
        )?;
        let pso_wireframe = factory.create_pipeline_state(
            &shader_set_wireframe,
            gfx::Primitive::TriangleList,
            {
                let mut r = gfx::state::Rasterizer::new_fill().with_cull_back();
                r.method = RasterMethod::Line(1);
                r
            },
            pipe::new(),
        )?;

        let mut chunk_meshes: HashMap<ChunkPos, Mesh> = HashMap::new(); // all the mesh to be rendered

        let t1 = Instant::now();

        for chunk in world.chunks.values() {
            let pos = (
                (chunk.pos.px * CHUNK_SIZE as i64) as f32,
                (chunk.pos.py * CHUNK_SIZE as i64) as f32,
                (chunk.pos.pz * CHUNK_SIZE as i64) as f32,
            );

            let (vertices, indices, _, _) = meshing(
                chunk,
                Some(world.create_adj_chunk_occl(chunk.pos.px, chunk.pos.py, chunk.pos.pz)),
                &block_meshes,
            );

            let chunk_mesh = Mesh::new(pos, vertices, indices, factory);
            chunk_meshes.insert(chunk.pos, chunk_mesh);
        }

        let t2 = Instant::now();
        info!(
            "Creating the first part of the meshes took {} ms",
            (t2 - t1).subsec_millis()
        );

        let texture_sampler = {
            use gfx::texture::*;
            factory.create_sampler(SamplerInfo {
                filter: FilterMethod::Scale,
                wrap_mode: (WrapMode::Clamp, WrapMode::Clamp, WrapMode::Clamp),
                lod_bias: 0f32.into(),
                lod_range: (0f32.into(), 1f32.into()),
                comparison: None,
                border: PackedColor(0xffffffff),
            })
        };

        Ok(Self {
            pso_fill,
            pso_wireframe,
            chunk_meshes,
            transform: factory.create_constant_buffer(1),
            block_meshes,
            texture_atlas,
            texture_sampler,
        })
    }

    pub fn render(&mut self, gfx: &mut Gfx, data: &WindowData, camera: &Camera) -> Result<()> {
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

        let view_proj =
            convert::<Matrix4<f64>, Matrix4<f32>>(camera.get_view_projection(aspect_ratio)).into();

        // drawing all the mesh
        for mesh in self.chunk_meshes.values() {
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
                texture_atlas: (self.texture_atlas.clone(), self.texture_sampler.clone()),
                color_buffer: color_buffer.clone(),
                depth_buffer: depth_buffer.clone(),
            };

            encoder.update_buffer(&data.transform, &[transform], 0)?;
            // (index buffer, pso, full data with vertex buffer and uniform buffer inside)
            encoder.draw(&mesh.indices, &self.pso_fill, &data);
        }

        Ok(())
    }

    /// Add a new chunk mesh to the rendering or update one if already exists
    pub fn update_chunk_mesh(&mut self, pos: ChunkPos, mesh: Mesh) {
        self.chunk_meshes.insert(pos, mesh);
    }

    /// Drop the mesh of the chunk at the position given (if the chunk exists)
    pub fn drop_mesh(&mut self, pos: &ChunkPos) {
        self.chunk_meshes.remove(pos);
    }
}

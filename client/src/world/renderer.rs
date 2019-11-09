use crate::{
    mesh::Mesh,
    window::{ColorFormat, DepthFormat, Gfx, WindowData},
    world::{frustum::Frustum, meshing_worker::MeshingWorker, skybox::Skybox},
};
use anyhow::Result;
use gfx;
use gfx::handle::Buffer;
use gfx::state::RasterMethod;
use gfx::traits::{Factory, FactoryExt};
use gfx_device_gl::Resources;
use nalgebra::{convert, Matrix4};
use std::collections::HashMap;
use voxel_rs_common::{block::BlockMesh, world::chunk::ChunkPos};
use voxel_rs_common::debug::send_debug_info;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "a_Pos",
        uv_pos: [f32; 2] = "a_UvPos",
        uv_offset: [f32; 2] = "a_UvOffset",
        uv_size: [f32; 2] = "a_UvSize",
        normal: u32 = "a_Norm",
    }

    vertex VertexSkybox {
        pos: [f32; 3] = "a_Pos",
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

    pipeline pipe_skybox {
        vbuf: gfx::VertexBuffer<VertexSkybox> = (),
        transform: gfx::ConstantBuffer<Transform> = "Transform",
        color_buffer: gfx::RenderTarget<ColorFormat> = "ColorBuffer",
        depth_buffer: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;
type PsoSkyboxType = gfx::PipelineState<gfx_device_gl::Resources, pipe_skybox::Meta>;

pub struct WorldRenderer {
    pub pso_fill: PsoType,
    pub pso_wireframe: PsoType,
    pub pso_skybox: PsoSkyboxType,
    pub chunk_meshes: HashMap<ChunkPos, Mesh>,
    pub transform: Buffer<Resources, Transform>,
    pub block_meshes: Vec<BlockMesh>,
    pub texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    pub texture_sampler: gfx::handle::Sampler<gfx_device_gl::Resources>,
    pub skybox: Skybox,
    pub meshing_worker: MeshingWorker,
}

impl WorldRenderer {
    pub fn new(
        gfx: &mut Gfx,
        block_meshes: Vec<BlockMesh>,
        texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    ) -> Result<Self> {
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

        let shader_set_skybox = factory.create_shader_set(
            include_bytes!("../../shader/skybox.vert"),
            include_bytes!("../../shader/skybox.frag"),
        )?;
        let pso_skybox = factory.create_pipeline_state(
            &shader_set_skybox,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            pipe_skybox::new(),
        )?;

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

        let skybox = Skybox::new(factory);

        Ok(Self {
            pso_fill,
            pso_wireframe,
            pso_skybox,
            chunk_meshes: HashMap::new(),
            transform: factory.create_constant_buffer(1),
            block_meshes: block_meshes.clone(),
            texture_atlas,
            texture_sampler,
            skybox,
            meshing_worker: MeshingWorker::new(block_meshes),
        })
    }

    pub fn render(&mut self, gfx: &mut Gfx, data: &WindowData, frustum: &Frustum, enable_culling: bool) -> Result<()> {
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

        let view_mat = frustum.get_view_matrix();
        let planes = frustum.get_planes(aspect_ratio);
        let view_proj_mat = frustum.get_view_projection(aspect_ratio);
        let view_proj =
            convert::<Matrix4<f64>, Matrix4<f32>>(view_proj_mat).into();

        // drawing all the meshes
        let mut count = 0;
        for (pos, mesh) in self.chunk_meshes.iter() {
            if !enable_culling || Frustum::contains_chunk(&planes, &view_mat, *pos) {
                count += 1;
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
        }
        send_debug_info("Render", "renderedchunks", format!("{} chunks were rendered", count));

        // drawing the Skybox
        let transform = Transform {
            view_proj,
            model: [
                // warning matrix is transposed
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [
                    frustum.position.x as f32,
                    frustum.position.y as f32,
                    frustum.position.z as f32,
                    1.0,
                ], // set skybox center at camera
            ],
        };

        let data = pipe_skybox::Data {
            vbuf: self.skybox.v_buffer.clone(),
            transform: self.transform.clone(),
            color_buffer: color_buffer.clone(),
            depth_buffer: depth_buffer.clone(),
        };

        encoder.update_buffer(&data.transform, &[transform], 0)?;
        encoder.draw(&self.skybox.indices, &self.pso_skybox, &data);

        Ok(())
    }

    /// Add a new chunk mesh to the rendering or update one if already exists
    pub fn update_chunk_mesh(&mut self, pos: ChunkPos, mesh: Mesh) {
        self.chunk_meshes.insert(pos, mesh);
    }
}

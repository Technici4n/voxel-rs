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
use log::info;
use nalgebra::{convert, Matrix4};
use std::collections::HashMap;
use std::path::Path;
use voxel_rs_common::debug::send_debug_info;
use voxel_rs_common::world::BlockPos;
use voxel_rs_common::{block::BlockMesh, world::chunk::ChunkPos};

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

    vertex VertexTarget {
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

    pipeline pipe_target {
        vbuf: gfx::VertexBuffer<VertexTarget> = (),
        transform: gfx::ConstantBuffer<Transform> = "Transform",
        color_buffer: gfx::RenderTarget<ColorFormat> = "ColorBuffer",
        depth_buffer: gfx::DepthTarget<DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type PsoType = gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>;
type PsoSkyboxType = gfx::PipelineState<gfx_device_gl::Resources, pipe_skybox::Meta>;
type PsoTargetType = gfx::PipelineState<gfx_device_gl::Resources, pipe_target::Meta>;

pub struct WorldRenderer {
    pub pso_fill: PsoType,
    pub pso_wireframe: PsoType,
    pub pso_skybox: PsoSkyboxType,
    pub pso_target: PsoTargetType,
    pub chunk_meshes: HashMap<ChunkPos, Mesh>,
    pub transform: Buffer<Resources, Transform>,
    pub block_meshes: Vec<BlockMesh>,
    pub texture_atlas: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
    pub texture_sampler: gfx::handle::Sampler<gfx_device_gl::Resources>,
    pub skybox: Skybox,
    pub meshing_worker: MeshingWorker,
}

pub fn load_shader<P: AsRef<Path>>(path: P) -> String {
    info!("Loading shader from {}", path.as_ref().display());
    std::fs::read_to_string(path).expect("Couldn't read shader from file")
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
            load_shader("assets/shaders/world.vert").as_bytes(),
            load_shader("assets/shaders/world.frag").as_bytes(),
        )?;
        let pso_fill = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill().with_cull_back(),
            pipe::new(),
        )?;

        let shader_set_wireframe = factory.create_shader_set(
            load_shader("assets/shaders/world.vert").as_bytes(),
            load_shader("assets/shaders/world_wireframe.frag").as_bytes(),
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
            load_shader("assets/shaders/skybox.vert").as_bytes(),
            load_shader("assets/shaders/skybox.frag").as_bytes(),
        )?;
        let pso_skybox = factory.create_pipeline_state(
            &shader_set_skybox,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
            pipe_skybox::new(),
        )?;

        let shader_set_target = factory.create_shader_set(
            load_shader("assets/shaders/target.vert").as_bytes(),
            load_shader("assets/shaders/target.frag").as_bytes(),
        )?;
        let pso_target = factory.create_pipeline_state(
            &shader_set_target,
            gfx::Primitive::LineList,
            {
                let mut r = gfx::state::Rasterizer::new_fill();
                r.method = RasterMethod::Line(2);
                r
            },
            pipe_target::new(),
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
            pso_target,
            chunk_meshes: HashMap::new(),
            transform: factory.create_constant_buffer(1),
            block_meshes: block_meshes.clone(),
            texture_atlas,
            texture_sampler,
            skybox,
            meshing_worker: MeshingWorker::new(block_meshes),
        })
    }

    pub fn render(
        &mut self,
        gfx: &mut Gfx,
        data: &WindowData,
        frustum: &Frustum,
        enable_culling: bool,
        pointed_block: Option<(BlockPos, usize)>,
    ) -> Result<()> {
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
        let view_proj = convert::<Matrix4<f64>, Matrix4<f32>>(view_proj_mat).into();

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
        send_debug_info(
            "Render",
            "renderedchunks",
            format!("{} chunks were rendered", count),
        );

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

        // drawing the target block
        if let Some((x, face)) = pointed_block {
            let mut vertices = Vec::new();
            fn vpos(i: i32, j: i32, k: i32, face: usize) -> VertexTarget {
                let mut v = [i as f32, j as f32, k as f32];
                for i in 0..3 {
                    if i == face / 2 {
                        // Move face forward
                        v[i] += 0.001 * (if face % 2 == 0 { 1.0 } else { -1.0 });
                    } else {
                        // Move edges inside the face
                        if v[i] == 1.0 {
                            v[i] = 0.999;
                        } else {
                            v[i] = 0.001;
                        }
                    }
                }
                VertexTarget { pos: v }
            }
            let end_coord = [
                if face == 1 { 1 } else { 2 },
                if face == 3 { 1 } else { 2 },
                if face == 5 { 1 } else { 2 },
            ];
            let start_coord = [
                if face == 0 { 1 } else { 0 },
                if face == 2 { 1 } else { 0 },
                if face == 4 { 1 } else { 0 },
            ];
            for i in start_coord[0]..end_coord[0] {
                for j in start_coord[1]..end_coord[1] {
                    for k in start_coord[2]..end_coord[2] {
                        let mut id = [i, j, k];
                        for i in 0..3 {
                            if id[i] > start_coord[i] {
                                let v1 = vpos(id[0], id[1], id[2], face);
                                id[i] = 0;
                                let v2 = vpos(id[0], id[1], id[2], face);
                                id[i] = 1;
                                vertices.extend([v1, v2].into_iter());
                            }
                        }
                    }
                }
            }
            let (buffer, slice) = gfx
                .factory
                .create_vertex_buffer_with_slice(&vertices[..], ());
            let data = pipe_target::Data {
                vbuf: buffer,
                transform: self.transform.clone(),
                color_buffer: color_buffer.clone(),
                depth_buffer: depth_buffer.clone(),
            };
            let transform = Transform {
                view_proj,
                model: [
                    // warning matrix is transposed
                    [1.0, 0.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0, 0.0],
                    [x.px as f32, x.py as f32, x.pz as f32, 1.0],
                ],
            };
            encoder.update_buffer(&data.transform, &[transform], 0)?;
            encoder.draw(&slice, &self.pso_target, &data);
        }

        Ok(())
    }

    /// Add a new chunk mesh to the rendering or update one if already exists
    pub fn update_chunk_mesh(&mut self, pos: ChunkPos, mesh: Mesh) {
        self.chunk_meshes.insert(pos, mesh);
    }
}

//! World rendering

use super::buffers::MultiBuffer;
use super::frustum::Frustum;
use super::init::{create_default_pipeline, load_glsl_shader, ShaderStage};
use super::{ to_u8_slice, buffer_from_slice };
use crate::texture::load_image;
use crate::window::WindowBuffers;
use image::{ImageBuffer, Rgba};
use nalgebra::{Matrix4, Similarity3, Translation3, UnitQuaternion, Vector3};
use voxel_rs_common::data::vox::VoxelModel;
use voxel_rs_common::debug::send_debug_info;
use voxel_rs_common::registry::Registry;
use voxel_rs_common::world::{BlockPos, ChunkPos};

mod meshing;
mod meshing_worker;
mod model;
mod skybox;
pub use self::model::Model;
pub use self::meshing::ChunkMeshData;
pub use self::meshing_worker::{ChunkMesh, MeshingWorker, start_meshing_worker};

/// All the state necessary to render the world.
pub struct WorldRenderer {
    // View-projection matrix
    uniform_view_proj: wgpu::Buffer,
    // Model matrix
    uniform_model: wgpu::Buffer,
    // Chunk rendering
    chunk_index_buffers: MultiBuffer<ChunkPos, u32>,
    chunk_vertex_buffers: MultiBuffer<ChunkPos, ChunkVertex>,
    chunk_pipeline: wgpu::RenderPipeline,
    chunk_bind_group: wgpu::BindGroup,
    // Skybox rendering
    skybox_index_buffer: wgpu::Buffer,
    skybox_vertex_buffer: wgpu::Buffer,
    skybox_pipeline: wgpu::RenderPipeline,
    // View-proj and model bind group
    vpm_bind_group: wgpu::BindGroup,
    // Targeted block rendering
    target_vertex_buffer: wgpu::Buffer,
    target_pipeline: wgpu::RenderPipeline,
    // Model rendering
    model_index_buffers: MultiBuffer<u32, u32>,
    model_vertex_buffers: MultiBuffer<u32, RgbVertex>,
    model_pipeline: wgpu::RenderPipeline,
}

impl WorldRenderer {
    pub fn new(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_atlas: ImageBuffer<Rgba<u8>, Vec<u8>>,
        models: &Registry<VoxelModel>,
    ) -> Self {
        // Load texture atlas
        let texture_atlas = load_image(device, encoder, texture_atlas);
        let texture_atlas_view = texture_atlas.create_view(&wgpu::TextureViewDescriptor::default());

        // Create uniform buffers
        let uniform_view_proj = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: 64,
            usage: (wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST),
        });
        let uniform_model = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: 64,
            usage: (wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST),
        });

        // Create uniform bind group
        let chunk_bind_group_layout = device.create_bind_group_layout(&CHUNK_BIND_GROUP_LAYOUT);
        let chunk_bind_group = create_chunk_bind_group(
            device,
            &chunk_bind_group_layout,
            &texture_atlas_view,
            &uniform_view_proj,
        );

        // Create chunk pipeline
        let chunk_pipeline = {
            let vertex_shader_bytes = load_glsl_shader(ShaderStage::Vertex, "assets/shaders/world.vert");
            let vertex_shader = wgpu::util::make_spirv(&vertex_shader_bytes);
            let fragment_shader_bytes = load_glsl_shader(ShaderStage::Fragment, "assets/shaders/world.frag");
            let fragment_shader = wgpu::util::make_spirv(&fragment_shader_bytes);

            create_default_pipeline(
                device,
                &chunk_bind_group_layout,
                vertex_shader,
                fragment_shader,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<ChunkVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &CHUNK_VERTEX_ATTRIBUTES,
                },
                true,
            )
        };

        // Create skybox vertex and index buffers
        let (skybox_vertex_buffer, skybox_index_buffer) = self::skybox::create_skybox(device);

        // Create skybox bind group
        let vpm_bind_group_layout = device.create_bind_group_layout(&SKYBOX_BIND_GROUP_LAYOUT);
        let vpm_bind_group = create_vpm_bind_group(
            device,
            &vpm_bind_group_layout,
            &uniform_view_proj,
            &uniform_model,
        );

        // Create skybox pipeline
        let skybox_pipeline = {
            let vertex_shader_bytes = load_glsl_shader(ShaderStage::Vertex, "assets/shaders/skybox.vert");
            let vertex_shader = wgpu::util::make_spirv(&vertex_shader_bytes);
            let fragment_shader_bytes = load_glsl_shader(ShaderStage::Fragment, "assets/shaders/skybox.frag");
            let fragment_shader = wgpu::util::make_spirv(&fragment_shader_bytes);

            create_default_pipeline(
                device,
                &vpm_bind_group_layout,
                vertex_shader,
                fragment_shader,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<SkyboxVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &SKYBOX_VERTEX_ATTRIBUTES,
                },
                false,
            )
        };

        // Create target buffer and pipeline
        let target_vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            mapped_at_creation: false,
            label: None,
            size: 8 * std::mem::size_of::<SkyboxVertex>() as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });
        let target_pipeline = {
            let vertex_shader_bytes = load_glsl_shader(ShaderStage::Vertex, "assets/shaders/target.vert");
            let vertex_shader = wgpu::util::make_spirv(&vertex_shader_bytes);
            let fragment_shader_bytes = load_glsl_shader(ShaderStage::Fragment, "assets/shaders/target.frag");
            let fragment_shader = wgpu::util::make_spirv(&fragment_shader_bytes);

            create_default_pipeline(
                device,
                &vpm_bind_group_layout,
                vertex_shader,
                fragment_shader,
                wgpu::PrimitiveTopology::LineList,
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<SkyboxVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &SKYBOX_VERTEX_ATTRIBUTES,
                },
                false,
            )
        };

        // Create model pipeline
        let model_pipeline = {
            let vertex_shader_bytes = load_glsl_shader(ShaderStage::Vertex, "assets/shaders/model.vert");
            let vertex_shader = wgpu::util::make_spirv(&vertex_shader_bytes);
            let fragment_shader_bytes = load_glsl_shader(ShaderStage::Fragment, "assets/shaders/model.frag");
            let fragment_shader = wgpu::util::make_spirv(&fragment_shader_bytes);

            create_default_pipeline(
                device,
                &vpm_bind_group_layout,
                vertex_shader,
                fragment_shader,
                wgpu::PrimitiveTopology::TriangleList,
                wgpu::VertexBufferDescriptor {
                    stride: std::mem::size_of::<RgbVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &RGB_VERTEX_ATTRIBUTES,
                },
                true,
            )
        };

        // Mesh models
        let mut model_index_buffers =
            MultiBuffer::with_capacity(device, 1, wgpu::BufferUsage::INDEX);
        let mut model_vertex_buffers =
            MultiBuffer::with_capacity(device, 1, wgpu::BufferUsage::VERTEX);
        for mesh_id in 0..models.get_number_of_ids() {
            let (vertices, indices) =
                self::model::mesh_model(models.get_value_by_id(mesh_id).unwrap());
            model_index_buffers.update(device, encoder, mesh_id, &indices);
            model_vertex_buffers.update(device, encoder, mesh_id, &vertices);
        }

        Self {
            uniform_view_proj,
            uniform_model,
            chunk_index_buffers: MultiBuffer::with_capacity(device, 1000, wgpu::BufferUsage::INDEX),
            chunk_vertex_buffers: MultiBuffer::with_capacity(
                device,
                1000,
                wgpu::BufferUsage::VERTEX,
            ),
            chunk_pipeline,
            chunk_bind_group,
            skybox_vertex_buffer,
            skybox_index_buffer,
            skybox_pipeline,
            vpm_bind_group,
            target_vertex_buffer,
            target_pipeline,
            model_pipeline,
            model_index_buffers,
            model_vertex_buffers,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        buffers: WindowBuffers,
        data: &crate::window::WindowData,
        frustum: &Frustum,
        enable_culling: bool,
        pointed_block: Option<(BlockPos, usize)>,
        models: &[model::Model],
    ) {
        //============= RENDER =============//
        // TODO: what if win_h is 0 ?
        let aspect_ratio = {
            let winit::dpi::PhysicalSize {
                width: win_w,
                height: win_h,
            } = data.physical_window_size;
            win_w as f64 / win_h as f64
        };

        let view_mat = frustum.get_view_matrix();
        let planes = frustum.get_planes(aspect_ratio);
        let view_proj_mat = frustum.get_view_projection(aspect_ratio);
        let opengl_to_wgpu = nalgebra::Matrix4::from([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ]);
        let view_proj: [[f32; 4]; 4] = nalgebra::convert::<
            nalgebra::Matrix4<f64>,
            nalgebra::Matrix4<f32>,
        >(opengl_to_wgpu * view_proj_mat)
        .into();

        // Update view_proj matrix
        let src_buffer = buffer_from_slice(
            device,
            wgpu::BufferUsage::COPY_SRC,
            to_u8_slice(&view_proj)
        );
        encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.uniform_view_proj, 0, 64);

        // Draw all the chunks
        {
            let mut rpass = super::render::create_default_render_pass(encoder, buffers);
            rpass.set_pipeline(&self.chunk_pipeline);
            rpass.set_bind_group(0, &self.chunk_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.chunk_vertex_buffers.get_buffer().slice(..));
            rpass.set_index_buffer(self.chunk_index_buffers.get_buffer().slice(..));
            let mut count = 0;
            for chunk_pos in self.chunk_index_buffers.keys() {
                if !enable_culling || Frustum::contains_chunk(&planes, &view_mat, chunk_pos) {
                    count += 1;
                    let (index_pos, index_len) =
                        self.chunk_index_buffers.get_pos_len(&chunk_pos).unwrap();
                    let (vertex_pos, _) =
                        self.chunk_vertex_buffers.get_pos_len(&chunk_pos).unwrap();
                    rpass.draw_indexed(
                        (index_pos as u32)..((index_pos + index_len) as u32),
                        vertex_pos as i32,
                        0..1,
                    );
                }
            }
            send_debug_info(
                "Render",
                "renderedchunks",
                format!("{} chunks were rendered", count),
            );
        }

        // Draw the skybox
        {
            // Update model buffer
            let src_buffer = buffer_from_slice(
                device,
                wgpu::BufferUsage::COPY_SRC,
                to_u8_slice(&[
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    frustum.position.x as f32,
                    frustum.position.y as f32,
                    frustum.position.z as f32,
                    1.0,
                ])
            );
            encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.uniform_model, 0, 64);
            let mut rpass = super::render::create_default_render_pass(encoder, buffers);
            rpass.set_pipeline(&self.skybox_pipeline);
            rpass.set_bind_group(0, &self.vpm_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.skybox_vertex_buffer.slice(..));
            rpass.set_index_buffer(self.skybox_index_buffer.slice(..));
            rpass.draw_indexed(0..36, 0, 0..1);
        }

        // Draw the target if necessary
        if let Some((target_pos, target_face)) = pointed_block {
            // Generate the vertices
            // TODO: maybe check if they changed since last frame
            let src_buffer = buffer_from_slice(
                device,
                wgpu::BufferUsage::COPY_SRC,
                to_u8_slice(&create_target_vertices(target_face))
            );
            encoder.copy_buffer_to_buffer(
                &src_buffer,
                0,
                &self.target_vertex_buffer,
                0,
                8 * std::mem::size_of::<SkyboxVertex>() as u64,
            );
            // Update model buffer
            let src_buffer = buffer_from_slice(
                device,
                wgpu::BufferUsage::COPY_SRC,
                to_u8_slice(&[
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    1.0,
                    0.0,
                    target_pos.px as f32,
                    target_pos.py as f32,
                    target_pos.pz as f32,
                    1.0,
                ])
            );
            encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.uniform_model, 0, 64);
            let mut rpass = super::render::create_default_render_pass(encoder, buffers);
            rpass.set_pipeline(&self.target_pipeline);
            rpass.set_bind_group(0, &self.vpm_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.target_vertex_buffer.slice(..));
            rpass.draw(0..8, 0..1);
        }

        // Draw the models
        for model in models {
            // Compute model matrix
            let mut transform = Similarity3::identity();
            transform.append_scaling_mut(model.scale);
            let offset_translation = Translation3::from(-Vector3::from(model.rot_offset));
            transform.append_translation_mut(&offset_translation);
            transform.append_rotation_mut(&UnitQuaternion::from_axis_angle(
                &Vector3::y_axis(),
                model.rot_y,
            ));
            transform.append_translation_mut(&Translation3::from(
                Vector3::new(model.pos_x, model.pos_y, model.pos_z)
                    + &Vector3::from(model.rot_offset),
            ));
            let transformation_matrix: Matrix4<f32> = nalgebra::convert(transform);
            // Update model buffer
            let src_buffer = buffer_from_slice(
                device,
                wgpu::BufferUsage::COPY_SRC,
                to_u8_slice(transformation_matrix.as_ref())
            );
            encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.uniform_model, 0, 64);
            // Draw model
            let mut rpass = super::render::create_default_render_pass(encoder, buffers);
            rpass.set_pipeline(&self.model_pipeline);
            rpass.set_bind_group(0, &self.vpm_bind_group, &[]);
            rpass.set_vertex_buffer(0, self.model_vertex_buffers.get_buffer().slice(..));
            rpass.set_index_buffer(self.model_index_buffers.get_buffer().slice(..));
            let (index_pos, index_len) = self
                .model_index_buffers
                .get_pos_len(&model.mesh_id)
                .unwrap();
            let (vertex_pos, _) = self
                .model_vertex_buffers
                .get_pos_len(&model.mesh_id)
                .unwrap();
            rpass.draw_indexed(
                (index_pos as u32)..((index_pos + index_len) as u32),
                vertex_pos as i32,
                0..1,
            );
        }
    }

    pub fn update_chunk_mesh(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        chunk_mesh: ChunkMesh,
    ) {
        let (pos, vertices, indices) = chunk_mesh;
        if vertices.len() > 0 && indices.len() > 0 {
            self.chunk_vertex_buffers
                .update(device, encoder, pos, &vertices[..]);
            self.chunk_index_buffers
                .update(device, encoder, pos, &indices[..]);
        }
    }

    pub fn remove_chunk_mesh(&mut self, pos: ChunkPos) {
        self.chunk_vertex_buffers.remove(&pos);
        self.chunk_index_buffers.remove(&pos);
    }
}

/*========== CHUNK RENDERING ==========*/
/// Chunk vertex
#[derive(Debug, Clone, Copy)]
pub struct ChunkVertex {
    pub pos: [f32; 3],
    pub texture_top_left: [f32; 2],
    pub texture_size: [f32; 2],
    pub texture_max_uv: [f32; 2],
    pub texture_uv: [f32; 2],
    pub occl_and_face: u32,
}

/// Chunk vertex attributes
const CHUNK_VERTEX_ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 6] = [
    wgpu::VertexAttributeDescriptor {
        shader_location: 0,
        format: wgpu::VertexFormat::Float3,
        offset: 0,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 1,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * 3,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 2,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 3,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 4,
        format: wgpu::VertexFormat::Float2,
        offset: 4 * (3 + 2 + 2 + 2),
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 5,
        format: wgpu::VertexFormat::Uint,
        offset: 4 * (3 + 2 + 2 + 2 + 2),
    },
];

const CHUNK_BIND_GROUP_LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> =
    wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: None },
                count: None
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Sampler { comparison: true },
                count: None
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::SampledTexture {
                    component_type: wgpu::TextureComponentType::Uint,
                    multisampled: false,
                    dimension: wgpu::TextureViewDimension::D2,
                },
                count: None
            },
        ],
    };

/// Create chunk bind group
fn create_chunk_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    texture_atlas_view: &wgpu::TextureView,
    uniform_view_proj: &wgpu::Buffer,
) -> wgpu::BindGroup {
    // Create texture sampler
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Linear,
        lod_min_clamp: 0.0,
        lod_max_clamp: 5.0,
        compare: Some(wgpu::CompareFunction::Always),
        anisotropy_clamp: None
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    uniform_view_proj.slice(0..64)
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(texture_atlas_view),
            },
        ],
    })
}

/*========== SKYBOX RENDERING ==========*/
/// Skybox vertex
#[derive(Debug, Clone, Copy)]
pub struct SkyboxVertex {
    pub position: [f32; 3],
}

/// Skybox vertex attributes
const SKYBOX_VERTEX_ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 1] =
    [wgpu::VertexAttributeDescriptor {
        shader_location: 0,
        format: wgpu::VertexFormat::Float3,
        offset: 0,
    }];

const SKYBOX_BIND_GROUP_LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> =
    wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                // view proj
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: None },
                count: None
            },
            wgpu::BindGroupLayoutEntry {
                // model
                binding: 1,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: None },
                count: None
            },
        ],
    };

/// Create skybox bind group
fn create_vpm_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_view_proj: &wgpu::Buffer,
    uniform_model: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    uniform_view_proj.slice(0..64)
                ),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(
                    uniform_model.slice(0..64)
                ),
            },
        ],
    })
}

/*========== TARGET RENDERING ==========*/
// `SkyboxVertex` is shamelessly stolen to also draw the targeted block

/// Create target vertices for some given face
fn create_target_vertices(face: usize) -> Vec<SkyboxVertex> {
    // TODO: simplify this
    let mut vertices = Vec::new();
    fn vpos(i: i32, j: i32, k: i32, face: usize) -> SkyboxVertex {
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
        SkyboxVertex { position: v }
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
                        vertices.extend([v1, v2].iter());
                    }
                }
            }
        }
    }
    vertices
}

/*========== MODEL RENDERING ==========*/
#[derive(Debug, Clone, Copy)]
pub struct RgbVertex {
    pub position: [f32; 3],
    pub info: u32,
}

const RGB_VERTEX_ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 2] = [
    wgpu::VertexAttributeDescriptor {
        shader_location: 0,
        format: wgpu::VertexFormat::Float3,
        offset: 0,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 1,
        format: wgpu::VertexFormat::Uint,
        offset: 4 * 3,
    },
];

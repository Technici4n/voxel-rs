use voxel_rs_common::world::chunk::ChunkPos;
use image::{ImageBuffer, Rgba};
use voxel_rs_common::debug::send_debug_info;
use voxel_rs_common::block::BlockMesh;
use crate::{
    world::meshing::ChunkMeshData,
    world::meshing_worker::MeshingWorker,
    world::frustum::Frustum,
    window::WindowBuffers,
    render::{MultiBuffer, load_glsl_shader, create_default_pipeline, create_default_render_pass},
    texture::load_image,
};

pub struct WorldRenderer {
    // Chunk meshing
    meshing_worker: MeshingWorker,
    // Chunk rendering
    transform_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    index_buffers: MultiBuffer<ChunkPos, u32>,
    vertex_buffers: MultiBuffer<ChunkPos, ChunkVertex>,
}

impl WorldRenderer {
    pub fn new(
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_atlas: ImageBuffer<Rgba<u8>, Vec<u8>>,
        block_meshes: Vec<BlockMesh>,
    ) -> Self {
        // TODO: split
        // Load texture
        let texture = load_image(device, encoder, texture_atlas);
        let texture_view = texture.create_default_view();

        // Create uniform buffer
        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: 64,
            usage: (wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST),
        });

        // Create texture sampler
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Linear,
            lod_min_clamp: 0.0,
            lod_max_clamp: 5.0,
            compare_function: wgpu::CompareFunction::Always,
        });

        let uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::Sampler,
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 2,
                        visibility: wgpu::ShaderStage::FRAGMENT,
                        ty: wgpu::BindingType::SampledTexture {
                            multisampled: false,
                            dimension: wgpu::TextureViewDimension::D2,
                        },
                    },
                ],
            });

        let uniforms = device.create_bind_group( &wgpu::BindGroupDescriptor {
            layout: &uniform_layout,
            bindings: &[
                wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &transform_buffer,
                        range: 0..64,
                    },
                },
                wgpu::Binding {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::Binding {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
            ],
        });

        // Create shader modules
        let mut compiler = shaderc::Compiler::new().expect("Failed to create shader compiler");
        let vertex_shader =
            load_glsl_shader(&mut compiler, shaderc::ShaderKind::Vertex, "assets/shaders/world.vert");
        let fragment_shader =
            load_glsl_shader(&mut compiler, shaderc::ShaderKind::Fragment, "assets/shaders/world.frag");

        let pipeline = create_default_pipeline(
            device,
            &uniform_layout,
            vertex_shader.as_binary(),
            fragment_shader.as_binary(),
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<ChunkVertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &CHUNK_VERTEX_ATTRIBUTES,
            },
            false,
        );

        Self {
            meshing_worker: MeshingWorker::new(block_meshes),
            transform_buffer,
            uniforms_bind_group: uniforms,
            pipeline,
            index_buffers: MultiBuffer::with_capacity(device, 1000, wgpu::BufferUsage::INDEX),
            vertex_buffers: MultiBuffer::with_capacity(device, 1000, wgpu::BufferUsage::VERTEX),
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
    ) {
        //============= RECEIVE CHUNK MESHES =============//
        for (pos, vertices, indices) in self.meshing_worker.get_processed_chunks() {
            if vertices.len() > 0 && indices.len() > 0 {
                self.vertex_buffers.update(
                    device,
                    encoder,
                    pos,
                    &vertices[..],
                );
                self.index_buffers.update(
                    device,
                    encoder,
                    pos,
                    &indices[..],
                );
            }
        }

        //============= RENDER =============//
        let aspect_ratio = {
            let winit::dpi::PhysicalSize {
                width: win_w,
                height: win_h,
            } = data.physical_window_size;
            win_w / win_h
        };

        let view_mat = frustum.get_view_matrix();
        let planes = frustum.get_planes(aspect_ratio);
        let view_proj_mat = frustum.get_view_projection(aspect_ratio);
        let opengl_to_wgpu = nalgebra::Matrix4::from([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, -1.0, 0.0, 0.0],
            [0.0, 0.0, 0.5, 0.0],
            [0.0, 0.0, 0.5, 1.0],
        ]);
        let view_proj: [[f32; 4]; 4] = nalgebra::convert::<nalgebra::Matrix4<f64>, nalgebra::Matrix4<f32>>(opengl_to_wgpu * view_proj_mat).into();

        // Update view_proj matrix
        // TODO: create helper function
        let src_buffer = device
            .create_buffer_mapped(4, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&view_proj);
        encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.transform_buffer, 0, 64);

        let mut rpass = create_default_render_pass(encoder, buffers);
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.uniforms_bind_group, &[]);
        rpass.set_vertex_buffers(0, &[(&self.vertex_buffers.get_buffer(), 0)]);
        rpass.set_index_buffer(&self.index_buffers.get_buffer(), 0);
        // Draw all the chunks
        let mut count = 0;
        for chunk_pos in self.index_buffers.keys() {
            if !enable_culling || Frustum::contains_chunk(&planes, &view_mat, chunk_pos) {
                count += 1;
                let (index_pos, index_len) = self.index_buffers.get_pos_len(&chunk_pos).unwrap();
                let (vertex_pos, _) = self.vertex_buffers.get_pos_len(&chunk_pos).unwrap();
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

    pub fn update_chunk(
        &mut self,
        data: ChunkMeshData,
    ) {
        self.meshing_worker.enqueue_chunk(data);
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) {
        self.meshing_worker.dequeue_chunk(pos);
        self.vertex_buffers.remove(&pos);
        self.index_buffers.remove(&pos);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChunkVertex {
    pub pos: [f32; 3],
    pub texture_top_left: [f32; 2],
    pub texture_size: [f32; 2],
    pub texture_max_uv: [f32; 2],
    pub texture_uv: [f32; 2],
    pub occl_and_face: u32,
}

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
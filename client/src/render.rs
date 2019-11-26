// TODO: refactor this into separate submodules
//! Various rendering utilities
use log::info;
use std::path::Path;
use std::hash::Hash;
use std::collections::HashMap;

/// Load a GLSL shader from a file and compile it to SPIR-V
pub fn load_glsl_shader<P: AsRef<Path>>(compiler: &mut shaderc::Compiler, shader_kind: shaderc::ShaderKind, path: P) -> shaderc::CompilationArtifact {
    let path_display = path.as_ref().display().to_string();
    info!("Loading GLSL shader from {}", path_display);
    let glsl_source = std::fs::read_to_string(path).expect("Couldn't read shader from file");

    // TODO: handle warnings
    compiler.compile_into_spirv(
        &glsl_source,
        shader_kind,
        &path_display,
        &"main",
        None,
    ).expect("Failed to compile GLSL shader into SPIR-V shader")
}

/// Default `RasterizationStateDescriptor` with no backface culling
pub const RASTERIZER_NO_CULLING: wgpu::RasterizationStateDescriptor = wgpu::RasterizationStateDescriptor {
    front_face: wgpu::FrontFace::Ccw,
    cull_mode: wgpu::CullMode::None,
    depth_bias: 0,
    depth_bias_slope_scale: 0.0,
    depth_bias_clamp: 0.0,
};

/// Default `ColorStateDescriptor`
pub const DEFAULT_COLOR_STATE_DESCRIPTOR: [wgpu::ColorStateDescriptor; 1] = [wgpu::ColorStateDescriptor {
    format: crate::window::COLOR_FORMAT,
    color_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    alpha_blend: wgpu::BlendDescriptor {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
        operation: wgpu::BlendOperation::Add,
    },
    write_mask: wgpu::ColorWrite::ALL,
}];

/// Default `DepthStencilStateDescriptor`
pub const DEFAULT_DEPTH_STENCIL_STATE_DESCRIPTOR: wgpu::DepthStencilStateDescriptor = wgpu::DepthStencilStateDescriptor {
    format: crate::window::DEPTH_FORMAT,
    depth_write_enabled: true,
    depth_compare: wgpu::CompareFunction::Less,
    stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
    stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
    stencil_read_mask: 0,
    stencil_write_mask: 0,
};

/// Create a default pipeline, comprised of
pub fn create_default_pipeline(
    device: &wgpu::Device,
    uniform_layout: &wgpu::BindGroupLayout,
    vertex_shader: &[u32],
    fragment_shader: &[u32],
    primitive_topology: wgpu::PrimitiveTopology,
    vertex_buffer_descriptor: wgpu::VertexBufferDescriptor,
) -> wgpu::RenderPipeline {
    // Shaders
    let vertex_shader_module = device.create_shader_module(vertex_shader);
    let fragment_shader_module = device.create_shader_module(fragment_shader);
    // Pipeline
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        bind_group_layouts: &[uniform_layout],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        layout: &pipeline_layout,
        vertex_stage: wgpu::ProgrammableStageDescriptor {
            module: &vertex_shader_module,
            entry_point: "main",
        },
        fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
            module: &fragment_shader_module,
            entry_point: "main",
        }),
        rasterization_state: Some(RASTERIZER_NO_CULLING),
        primitive_topology,
        color_states: &DEFAULT_COLOR_STATE_DESCRIPTOR,
        depth_stencil_state: Some(DEFAULT_DEPTH_STENCIL_STATE_DESCRIPTOR),
        index_format: wgpu::IndexFormat::Uint16,
        vertex_buffers: &[vertex_buffer_descriptor],
        sample_count: 1,
        sample_mask: !0,
        alpha_to_coverage_enabled: false,
    })
}

/// Create the default render pass
pub fn create_default_render_pass<'a>(encoder: &'a mut wgpu::CommandEncoder, color_target: &wgpu::TextureView, depth_target: &wgpu::TextureView) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: color_target,
            resolve_target: None,
            load_op: wgpu::LoadOp::Load,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::GREEN, // TODO: use debugging color ?
        }],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
            attachment: depth_target,
            depth_load_op: wgpu::LoadOp::Load,
            depth_store_op: wgpu::StoreOp::Store,
            clear_depth: 0.0, // TODO: use debugging depth ?
            stencil_load_op: wgpu::LoadOp::Clear,
            stencil_store_op: wgpu::StoreOp::Clear,
            clear_stencil: 0,
        }),
    })
}

/// A buffer that will automatically resize itself when necessary
pub struct DynamicBuffer<T: Copy> {
    pub buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    capacity: usize,
    pub len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T: Copy + 'static> DynamicBuffer<T> {
    pub fn with_capacity(device: &wgpu::Device, initial_capacity: usize, mut usage: wgpu::BufferUsage) -> Self {
        usage |= wgpu::BufferUsage::COPY_DST;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                size: (initial_capacity * std::mem::size_of::<T>()) as u64,
                usage,
            }),
            usage,
            capacity: initial_capacity,
            len: 0,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn upload(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, data: &[T]) {
        if data.is_empty() {
            self.len = 0;
            return;
        }

        if data.len() > self.capacity {
            self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
                size: (data.len() * std::mem::size_of::<T>()) as u64,
                usage: self.usage,
            });
            self.capacity = data.len();
        }

        let src_buffer = device
            .create_buffer_mapped(data.len(), wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(data);

        encoder.copy_buffer_to_buffer(
            &src_buffer,
            0,
            &self.buffer,
            0,
            (data.len() * std::mem::size_of::<T>()) as u64,
        );
        self.len = data.len();
    }
}

/// A buffer that can contain multiple objects
pub struct MultiBuffer<K: Hash + Eq, T: Copy + 'static> {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    objects: HashMap<K, usize>,
    segments: Vec<MultiBufferSegment>,
    len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<K: Hash + Eq, T: Copy + 'static> MultiBuffer<K, T> {
    /// Create a new `MultiBuffer` with enough capacity for `initial_capacity` elements of type `T`
    pub fn with_capacity(device: &wgpu::Device, initial_capacity: usize, mut usage: wgpu::BufferUsage) -> Self {
        usage |= wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (initial_capacity * std::mem::size_of::<T>()) as u64,
            usage,
        });
        let segments = vec![MultiBufferSegment { free: true, pos: 0, len: initial_capacity }];

        Self {
            buffer,
            usage,
            objects: HashMap::new(),
            segments,
            len: initial_capacity,
            phantom: std::marker::PhantomData,
        }
    }

    /// Remove object `object` from the buffer
    pub fn remove(&mut self, object: &K) {
        if let Some(start_position) = self.objects.remove(object) {
            let mut segment_position = self.segments.iter_mut().position(|seg| seg.pos == start_position).expect("logic error!");
            assert_eq!(false, self.segments[segment_position].free, "logic error!");
            self.segments[segment_position].free = true;
            // Merge with the segment before if possible
            if segment_position > 0 {
                if self.segments[segment_position - 1].free {
                    self.segments[segment_position - 1].len += self.segments[segment_position].len;
                    self.segments.remove(segment_position);
                    segment_position -= 1;
                }
            }
            // Merge with the segment after if possible
            if segment_position < self.segments.len() - 1 {
                if self.segments[segment_position + 1].free {
                    self.segments[segment_position].len += self.segments[segment_position + 1].len;
                    self.segments.remove(segment_position + 1);
                }
            }
        }
    }

    /// Update the data for object `object` in the buffer
    // TODO: handle memory fragmentation
    pub fn update(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, object: K, data: &[T]) {
        // Remove the object if it's already in the buffer
        self.remove(&object);
        // Try to find the position to insert
        let insert_position = self.segments.iter_mut().position(|seg| seg.len >= data.len() && seg.free);
        let insert_position = insert_position.unwrap_or_else(|| {
            // Reallocate at least twice the size
            self.reallocate(device, encoder, (self.len + data.len()).max(2 * self.len));
            self.segments.len() - 1
        });
        // Copy data into the buffer
        let src_buffer =
            device
                .create_buffer_mapped(data.len() * std::mem::size_of::<T>(), wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(data);
        encoder.copy_buffer_to_buffer(
            &src_buffer,
            0,
            &self.buffer,
            self.segments[insert_position].pos as u64,
            (data.len() * std::mem::size_of::<T>()) as u64,
        );
        // Update current segment
        self.segments[insert_position].free = false;
        // Split the segment if necessary
        let extra_length = self.segments[insert_position].len - data.len();
        if extra_length > 0 {
            self.segments[insert_position].len -= extra_length;
            self.segments.insert(insert_position + 1, MultiBufferSegment {
                free: true,
                pos: self.segments[insert_position].pos + self.segments[insert_position].len,
                len: extra_length,
            });
        }
        // Update the map
        self.objects.insert(object, self.segments[insert_position].pos);
    }

    fn reallocate(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, new_len: usize) {
        // Create new buffer and copy data
        let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (new_len * std::mem::size_of::<T>()) as u64,
            usage: self.usage,
        });
        encoder.copy_buffer_to_buffer(&self.buffer, 0, &new_buffer, 0, (self.len * std::mem::size_of::<T>()) as u64);
        self.buffer = new_buffer;
        // Update segments and len
        let last_segment = self.segments.last_mut().expect("logic error!");
        if last_segment.free {
            last_segment.len += new_len - self.len;
        } else {
            self.segments.push(MultiBufferSegment {
                free: true,
                pos: self.len,
                len: new_len,
            });
        }
        self.len = new_len;
    }

    /// Get the position and the length of object `object` in the buffer
    pub fn get_pos_len(&self, object: &K) -> Option<(usize, usize)> {
        let pos = self.objects.get(object);
        match pos {
            None => None,
            Some(pos) => {
                let seg = self.segments[*pos];
                Some((seg.pos, seg.len))
            }
        }
    }

    /// Get the buffer. Please don't modify it.
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}

#[derive(Debug, Clone, Copy)]
struct MultiBufferSegment {
    pub free: bool,
    pub pos: usize,
    pub len: usize,
}
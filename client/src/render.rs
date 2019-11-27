// TODO: refactor this into separate submodules
//! Various rendering utilities
use log::info;
use std::path::Path;
use std::hash::Hash;
use std::collections::HashMap;
use crate::window::WindowBuffers;

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

/// Default `RasterizationStateDescriptor` with backface culling
pub const RASTERIZER_WITH_CULLING: wgpu::RasterizationStateDescriptor = wgpu::RasterizationStateDescriptor {
    cull_mode: wgpu::CullMode::Back,
    ..RASTERIZER_NO_CULLING
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
    cull_back_faces: bool,
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
        rasterization_state: Some(if cull_back_faces {RASTERIZER_WITH_CULLING} else {RASTERIZER_NO_CULLING}),
        primitive_topology,
        color_states: &DEFAULT_COLOR_STATE_DESCRIPTOR,
        depth_stencil_state: Some(DEFAULT_DEPTH_STENCIL_STATE_DESCRIPTOR),
        index_format: wgpu::IndexFormat::Uint32,
        vertex_buffers: &[vertex_buffer_descriptor],
        sample_count: crate::window::SAMPLE_COUNT,
        sample_mask: 0xFFFFFFFF,
        alpha_to_coverage_enabled: false,
    })
}

pub fn create_default_depth_stencil_attachment(depth_buffer: &wgpu::TextureView) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<&wgpu::TextureView> {
    wgpu::RenderPassDepthStencilAttachmentDescriptor {
        attachment: depth_buffer,
        depth_load_op: wgpu::LoadOp::Load,
        depth_store_op: wgpu::StoreOp::Store,
        clear_depth: 0.0, // TODO: use debugging depth ?
        stencil_load_op: wgpu::LoadOp::Clear,
        stencil_store_op: wgpu::StoreOp::Clear,
        clear_stencil: 0,
    }
}

/// Create the default render pass
pub fn create_default_render_pass<'a>(encoder: &'a mut wgpu::CommandEncoder, buffers: WindowBuffers<'a>) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &[wgpu::RenderPassColorAttachmentDescriptor {
            attachment: buffers.multisampled_texture_buffer,
            resolve_target: Some(buffers.texture_buffer),
            load_op: wgpu::LoadOp::Load,
            store_op: wgpu::StoreOp::Store,
            clear_color: wgpu::Color::GREEN, // TODO: use debugging color ?
        }],
        depth_stencil_attachment: Some(create_default_depth_stencil_attachment(buffers.depth_buffer)),
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
pub struct MultiBuffer<K: Hash + Eq + Clone, T: Copy + 'static> {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsage,
    objects: HashMap<K, usize>,
    segments: Vec<MultiBufferSegment>,
    len: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<K: Hash + Eq + Clone + std::fmt::Debug, T: Copy + std::fmt::Debug + 'static> MultiBuffer<K, T> {
    /// Create a new `MultiBuffer` with enough capacity for `initial_capacity` elements of type `T`
    pub fn with_capacity(device: &wgpu::Device, initial_capacity: usize, mut usage: wgpu::BufferUsage) -> Self {
        usage |= wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::COPY_SRC;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            size: (initial_capacity * std::mem::size_of::<T>()) as u64,
            usage,
        });
        let segments = vec![MultiBufferSegment { free: true, pos: 0, len: initial_capacity }];

        let res = Self {
            buffer,
            usage,
            objects: HashMap::new(),
            segments,
            len: initial_capacity,
            phantom: std::marker::PhantomData,
        };
        res.assert_invariants();
        res
    }

    /// Remove object `object` from the buffer
    pub fn remove(&mut self, object: &K) {
        self.assert_invariants();
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
        self.assert_invariants();
    }

    /// Update the data for object `object` in the buffer
    ///
    /// # Panics
    /// Will panic if `data` is empty.
    // TODO: handle memory fragmentation
    pub fn update(&mut self, device: &wgpu::Device, encoder: &mut wgpu::CommandEncoder, object: K, data: &[T]) {
        assert!(data.len() > 0, "cannot add an empty slice to a MultiBuffer");
        self.assert_invariants();
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
                .create_buffer_mapped(data.len(), wgpu::BufferUsage::COPY_SRC)
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
            if insert_position < self.segments.len() - 1 && self.segments[insert_position+1].free {
                self.segments[insert_position+1].pos -= extra_length;
                self.segments[insert_position+1].len += extra_length;
            } else {
                self.segments.insert(insert_position + 1, MultiBufferSegment {
                    free: true,
                    pos: self.segments[insert_position].pos + data.len(),
                    len: extra_length,
                });
            }
        }
        // Update the map
        self.objects.insert(object.clone(), self.segments[insert_position].pos);
        self.assert_invariants();
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
                len: new_len - self.len,
            });
        }
        self.len = new_len;
        self.assert_invariants();
    }

    fn assert_invariants(&self) {
        assert_eq!(self.segments.first().unwrap().pos, 0);
        assert_eq!(self.segments.last().unwrap().pos + self.segments.last().unwrap().len, self.len);
        for i in 0..(self.segments.len()-1) {
            assert_eq!(self.segments[i].pos + self.segments[i].len, self.segments[i+1].pos);
            assert!(!(self.segments[i].free && self.segments[i+1].free));
        }
        for v in self.objects.values() {
            let mut segment_position = self.segments.iter().enumerate().find(|(_, seg)| seg.pos == *v).expect("logic error!").0;
            assert_eq!(false, self.segments[segment_position].free, "logic error!");
        }
        for s in self.segments.iter() {
            let pos_cnt = self.objects.values().filter(|v| **v == s.pos).count();
            if s.free {
                assert_eq!(pos_cnt, 0);
            } else {
                assert_eq!(pos_cnt, 1);
            }
        }
    }

    /// Get the position and the length of object `object` in the buffer
    pub fn get_pos_len(&self, object: &K) -> Option<(usize, usize)> {
        self.assert_invariants();
        let pos = self.objects.get(object);
        match pos {
            None => None,
            Some(pos) => {
                for seg in self.segments.iter() {
                    if *pos == seg.pos {
                        return Some((seg.pos, seg.len))
                    }
                }
                None
            }
        }
    }

    /// Get the buffer. Please don't modify it.
    pub fn get_buffer(&self) -> &wgpu::Buffer {
        self.assert_invariants();
        &self.buffer
    }

    /// Get the keys
    pub fn keys(&self) -> impl Iterator<Item = K> {
        self.assert_invariants();
        self.objects.keys().cloned().collect::<Vec<K>>().into_iter()
    }
}

#[derive(Debug, Clone, Copy)]
struct MultiBufferSegment {
    pub free: bool,
    pub pos: usize,
    pub len: usize,
}
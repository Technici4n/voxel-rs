//! Helpers for pipeline creation and initialization
use std::path::Path;

/// Shader stage
pub enum ShaderStage {
    Vertex,
    Fragment,
}

/// Load a GLSL shader from a file and compile it to SPIR-V
pub fn load_glsl_shader<P: AsRef<Path>>(stage: ShaderStage, path: P) -> Vec<u32> {
    let ty = match stage {
        ShaderStage::Vertex => glsl_to_spirv::ShaderType::Vertex,
        ShaderStage::Fragment => glsl_to_spirv::ShaderType::Fragment,
    };
    let path_display = path.as_ref().display().to_string();
    log::info!("Loading GLSL shader from {}", path_display);
    let glsl_source = std::fs::read_to_string(path).expect("Couldn't read shader from file");

    wgpu::read_spirv(
        glsl_to_spirv::compile(&glsl_source, ty).expect("Failed to compile GLSL shader"),
    ).expect("Failed to read SPIR-V")
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

/// Create a default pipeline
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
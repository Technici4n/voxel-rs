//! Ui rendering

use super::{ buffer_from_slice, to_u8_slice };
use super::buffers::DynamicBuffer;
use super::init::{load_glsl_shader, ShaderStage};
use crate::ui::PrimitiveBuffer;
use crate::window::{WindowBuffers, WindowData};
use std::collections::{BTreeMap, HashMap};
use wgpu_glyph::{FontId, ab_glyph::FontVec};

pub struct UiRenderer {
    // Glyph rendering
    glyph_brush: wgpu_glyph::GlyphBrush<(), FontVec>,
    fonts: HashMap<String, FontId>,
    // Rectangle rendering
    transform_buffer: wgpu::Buffer,
    uniforms_bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: DynamicBuffer<UiVertex>,
    index_buffer: DynamicBuffer<u32>,
}

impl<'a> UiRenderer {
    pub fn new(device: &mut wgpu::Device) -> Self {
        // Load fonts
        let default_font = FontVec::try_from_vec(
            include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf").to_vec()
        ).expect("Failed to load default font.");
        let mut glyph_brush_builder = wgpu_glyph::GlyphBrushBuilder::using_font(default_font);
        log::info!("Loading fonts from assets/fonts/list.toml");
        let mut fonts = HashMap::new();
        let font_list = std::fs::read_to_string("assets/fonts/list.toml")
            .expect("Couldn't read font list file");
        let font_files: BTreeMap<String, String> =
            toml::de::from_str(&font_list).expect("Couldn't parse font list file");
        for (font_name, font_file) in font_files.into_iter() {
            use std::io::Read;
            log::info!("Loading font {} from file {}", font_name, font_file);
            let mut font_bytes = vec![];
            let mut file = std::fs::File::open(font_file).expect("Couldn't open font file");
            file.read_to_end(&mut font_bytes)
                .expect("Couldn't read font file");
            let font = FontVec::try_from_vec(font_bytes).expect("Couldn't read font file");
            fonts.insert(font_name, glyph_brush_builder.add_font(font));
        }
        log::info!("Fonts successfully loaded");
        let glyph_brush = glyph_brush_builder
            //.depth_stencil_state(DEFAULT_DEPTH_STENCIL_STATE_DESCRIPTOR)
            .build(device, crate::window::COLOR_FORMAT);

        // Create uniform buffer
        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui_transform_buffer"),
            mapped_at_creation: false,
            size: 64,
            usage: (wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST),
        });

        // Create bind group layout
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::VERTEX,
                ty: wgpu::BindingType::UniformBuffer { dynamic: false, min_binding_size: None },
                count: None
            }],
        });

        // Create bind group
        let uniforms_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(
                    transform_buffer.slice(0..16)
                ),
            }],
        });

        // Create shader modules
        let vertex_shader_bytes = load_glsl_shader(ShaderStage::Vertex, "assets/shaders/gui-rect.vert");
        let vertex_shader = wgpu::util::make_spirv(&vertex_shader_bytes);
        let fragment_shader_bytes = load_glsl_shader(ShaderStage::Fragment, "assets/shaders/gui-rect.frag");
        let fragment_shader = wgpu::util::make_spirv(&fragment_shader_bytes);

        log::trace!("Creating pipeline.");

        let pipeline = super::init::create_default_pipeline(
            device,
            &uniform_layout,
            vertex_shader,
            fragment_shader,
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexBufferDescriptor {
                stride: std::mem::size_of::<UiVertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &UI_VERTEX_ATTRIBUTES,
            },
            false,
        );

        log::trace!("Created pipeline.");

        Self {
            glyph_brush,
            fonts,
            transform_buffer,
            uniforms_bind_group,
            pipeline,
            vertex_buffer: DynamicBuffer::with_capacity(device, 64, wgpu::BufferUsage::VERTEX),
            index_buffer: DynamicBuffer::with_capacity(device, 64, wgpu::BufferUsage::INDEX),
        }
    }

    pub fn render<Message>(
        &mut self,
        buffers: WindowBuffers<'a>,
        device: &mut wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        data: &WindowData,
        ui: &quint::Ui<PrimitiveBuffer, Message>,
        gui: &mut crate::gui::Gui,
        draw_crosshair: bool,
    ) {
        // Render test dropdown
        let mut primitive_buffer = gui.drain_primitives();

        //ui.render(&mut primitive_buffer);

        // Render primitives
        let mut rect_vertices: Vec<UiVertex> = Vec::new();
        let mut rect_indices: Vec<u32> = Vec::new();

        use crate::ui::{RectanglePrimitive, TextPrimitive, TrianglesPrimitive};

        // Rectangles
        for RectanglePrimitive {
            layout: l,
            color,
            z,
        } in primitive_buffer.rectangle.into_iter()
        {
            let a = UiVertex {
                position: [l.x, l.y, z],
                color: color.clone(),
            };
            let b = UiVertex {
                position: [l.x + l.width, l.y, z],
                color: color.clone(),
            };
            let c = UiVertex {
                position: [l.x, l.y + l.height, z],
                color: color.clone(),
            };
            let d = UiVertex {
                position: [l.x + l.width, l.y + l.height, z],
                color: color.clone(),
            };
            let a_index = rect_vertices.len() as u32;
            let b_index = a_index + 1;
            let c_index = b_index + 1;
            let d_index = c_index + 1;
            rect_vertices.extend([a, b, c, d].iter());
            rect_indices.extend([b_index, a_index, c_index, b_index, c_index, d_index].iter());
        }
        // Triangles
        for TrianglesPrimitive {
            vertices,
            indices,
            color,
        } in primitive_buffer.triangles.into_iter()
        {
            let index_offset = rect_vertices.len() as u32;
            rect_vertices.extend(
                vertices
                    .into_iter()
                    .map(|v| UiVertex { position: v, color }),
            );
            rect_indices.extend(indices.into_iter().map(|id| id + index_offset));
        }
        // Text
        for TextPrimitive {
            x, y, w, h,
            mut parts,
            z,
            center_horizontally, center_vertically,
        } in primitive_buffer.text.into_iter()
        {
            let dpi = data.hidpi_factor as f32;

            // Apply DPI to font size
            for p in parts.iter_mut() {
                p.font_size.x *= dpi;
                p.font_size.y *= dpi;
            }
            // Get font IDs
            let Self { ref fonts, .. } = &self;
            let parts: Vec<wgpu_glyph::Text> = parts
                .iter()
                .map(|part| wgpu_glyph::Text::new(&part.text)
                    .with_scale(part.font_size)
                    .with_color(part.color)
                    .with_font_id(part
                        .font
                        .clone()
                        .and_then(|f| fonts.get(&f).cloned())
                        .unwrap_or_default())
                )
                .collect();
            // Calculate positions
            let mut x = x as f32;
            let mut y = y as f32;
            let mut w = match w {
                Some(w) => w as f32,
                None => std::f32::INFINITY,
            };
            let mut h = match h {
                Some(h) => h as f32,
                None => std::f32::INFINITY,
            };
            if center_horizontally {
                x += w/2.0;
            }
            if center_vertically {
                y += h/2.0;
            }
            // Apply DPI to positions
            x *= dpi;
            y *= dpi;
            w *= dpi;
            h *= dpi;
            let v_align = if center_vertically {
                wgpu_glyph::VerticalAlign::Center
            } else {
                wgpu_glyph::VerticalAlign::Top
            };
            let h_align = if center_horizontally {
                wgpu_glyph::HorizontalAlign::Center
            } else {
                wgpu_glyph::HorizontalAlign::Left
            };
            let section = wgpu_glyph::Section::default()
                .with_screen_position((x, y))
                .with_bounds((w, h))
                .with_layout(wgpu_glyph::Layout::Wrap {
                    line_breaker: Default::default(),
                    v_align,
                    h_align,
                })
                .with_text(parts);
            self.glyph_brush.queue(section);
        }
        // Crosshair
        if draw_crosshair {
            let (cx, cy) = (
                data.logical_window_size.width as f32 / 2.0,
                data.logical_window_size.height as f32 / 2.0,
            );
            const HALF_HEIGHT: f32 = 15.0;
            const HALF_WIDTH: f32 = 2.0;
            const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
            let v1 = UiVertex {
                position: [cx - HALF_WIDTH, cy - HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v2 = UiVertex {
                position: [cx + HALF_WIDTH, cy - HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v3 = UiVertex {
                position: [cx - HALF_WIDTH, cy + HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v4 = UiVertex {
                position: [cx + HALF_WIDTH, cy + HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v5 = UiVertex {
                position: [cx - HALF_HEIGHT, cy - HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v6 = UiVertex {
                position: [cx + HALF_HEIGHT, cy - HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v7 = UiVertex {
                position: [cx - HALF_HEIGHT, cy + HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v8 = UiVertex {
                position: [cx + HALF_HEIGHT, cy + HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let voffset = rect_vertices.len() as u32;
            rect_vertices.extend([v1, v2, v3, v4, v5, v6, v7, v8].iter());
            rect_indices.extend(
                [0, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6, 7]
                    .iter()
                    .map(|id| id + voffset),
            );
        }

        // Draw rectangles
        {
            let (win_w, win_h) = (
                data.logical_window_size.width,
                data.logical_window_size.height,
            );
            // Update the uniform buffer to map (w, h) coordinates to [-1, 1]
            let transformation_matrix = [
                2.0 / win_w as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                -2.0 / win_h as f32,
                0.0,
                0.0,
                0.0,
                0.0,
                0.5,
                0.0,
                -1.0,
                1.0,
                0.5,
                1.0,
            ];
            let src_buffer = buffer_from_slice(
                device,
                wgpu::BufferUsage::COPY_SRC,
                to_u8_slice(&transformation_matrix[..])
            );
            encoder.copy_buffer_to_buffer(&src_buffer, 0, &self.transform_buffer, 0, 16 * 4);
            // Update vertex buffer
            self.vertex_buffer.upload(device, encoder, &rect_vertices);
            // Update index buffer
            self.index_buffer.upload(device, encoder, &rect_indices);
            // Draw
            {
                let mut rpass = super::render::create_default_render_pass(encoder, buffers);
                rpass.set_pipeline(&self.pipeline);
                rpass.set_bind_group(0, &self.uniforms_bind_group, &[]);
                rpass.set_vertex_buffer(0, self.vertex_buffer.get_buffer().slice(..));
                rpass.set_index_buffer(self.index_buffer.get_buffer().slice(..));
                rpass.draw_indexed(0..(self.index_buffer.len() as u32), 0, 0..1);
            }
        }

        // Resolve !
        super::render::encode_resolve_render_pass(encoder, buffers);

        // Draw text
        // TODO: use depth buffer
        let mut staging_belt = wgpu::util::StagingBelt::new(128);
        self.glyph_brush
            .draw_queued(
                device,
                &mut staging_belt,
                encoder,
                buffers.texture_buffer,
                //create_default_depth_stencil_attachment(buffers.depth_buffer),
                data.physical_window_size.width,
                data.physical_window_size.height,
            )
            .expect("couldn't draw queued glyphs");
    }
}

#[derive(Debug, Clone, Copy)]
struct UiVertex {
    position: [f32; 3],
    color: [f32; 4],
}

const UI_VERTEX_ATTRIBUTES: [wgpu::VertexAttributeDescriptor; 2] = [
    wgpu::VertexAttributeDescriptor {
        shader_location: 0,
        format: wgpu::VertexFormat::Float3,
        offset: 0,
    },
    wgpu::VertexAttributeDescriptor {
        shader_location: 1,
        format: wgpu::VertexFormat::Float4,
        offset: 12,
    },
];

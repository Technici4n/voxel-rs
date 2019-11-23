//use crate::render::ensure_buffer_capacity;
use crate::window::WindowData;
//use crate::render::load_shader;
use anyhow::Result;
use log::info;
use quint::Layout;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use wgpu_glyph::FontId;

#[derive(Debug, Clone)]
struct RectanglePrimitive {
    pub layout: Layout,
    pub color: [f32; 4],
    pub z: f32,
}

#[derive(Debug, Clone)]
struct TextPrimitive {
    pub layout: Layout,
    pub parts: Vec<TextPart>,
    pub z: f32,
    pub centered: bool,
}

#[derive(Debug, Clone)]
struct TrianglesPrimitive {
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
    pub color: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct TextPart {
    pub text: String,
    pub font_size: wgpu_glyph::Scale,
    pub color: [f32; 4],
    pub font: Option<String>,
}

#[derive(Default, Debug)]
pub struct PrimitiveBuffer {
    pub(self) rectangle: Vec<RectanglePrimitive>,
    pub(self) text: Vec<TextPrimitive>,
    pub(self) triangles: Vec<TrianglesPrimitive>,
}

impl PrimitiveBuffer {
    pub fn draw_rectangle(&mut self, color: [f32; 4], layout: Layout, z: f32) {
        self.rectangle.push(RectanglePrimitive { color, layout, z });
    }

    pub fn draw_text(&mut self, parts: Vec<TextPart>, layout: Layout, z: f32, centered: bool) {
        self.text.push(TextPrimitive {
            layout,
            parts,
            z,
            centered,
        })
    }

    pub fn draw_triangles(&mut self, vertices: Vec<[f32; 3]>, indices: Vec<u32>, color: [f32; 4]) {
        self.triangles.push(TrianglesPrimitive {
            vertices,
            indices,
            color,
        });
    }
}

pub struct UiRenderer {
    // Glyph rendering
    glyph_brush: wgpu_glyph::GlyphBrush<'static, ()>,
    fonts: HashMap<String, FontId>,
    /*// Rectangle rendering
    pso: PsoType,
    data: PipeDataType,
    rect_vertex_buffer: gfx::handle::Buffer<R, Vertex>,
    rect_index_buffer: gfx::handle::Buffer<R, u32>,*/
}

impl UiRenderer {
    pub fn new(device: &mut wgpu::Device) -> Result<Self> {
        // Load fonts
        let default_font: &'static [u8] =
            include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf");
        let mut glyph_brush_builder = wgpu_glyph::GlyphBrushBuilder::using_font_bytes(default_font);
        info!("Loading fonts from assets/fonts/list.toml");
        let mut fonts = HashMap::new();
        let font_list = std::fs::read_to_string("assets/fonts/list.toml")
            .expect("Couldn't read font list file");
        let font_files: BTreeMap<String, String> =
            toml::de::from_str(&font_list).expect("Couldn't parse font list file");
        for (font_name, font_file) in font_files.into_iter() {
            info!("Loading font {} from file {}", font_name, font_file);
            let mut font_bytes = Vec::new();
            let mut file = std::fs::File::open(font_file).expect("Couldn't open font file");
            file.read_to_end(&mut font_bytes)
                .expect("Couldn't read font file");
            fonts.insert(font_name, glyph_brush_builder.add_font_bytes(font_bytes));
        }
        info!("Fonts successfully loaded");
        let glyph_brush = glyph_brush_builder.build(device, crate::window::COLOR_FORMAT);

        /*// Create rectangle drawing pipeline
        let shader_set = factory.create_shader_set(
            load_shader("assets/shaders/gui-rect.vert").as_bytes(),
            load_shader("assets/shaders/gui-rect.frag").as_bytes(),
        )?;
        let pso = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            {
                let mut r = gfx::state::Rasterizer::new_fill();
                r.samples = Some(gfx::state::MultiSample);
                r
            },
            pipe::new(),
        )?;
        let buffer_bind = {
            use gfx::memory::Bind;
            let mut bind = Bind::empty();
            bind.insert(Bind::SHADER_RESOURCE);
            bind.insert(Bind::TRANSFER_DST);
            bind
        };
        let rect_vertex_buffer = factory.create_buffer(
            1,
            gfx::buffer::Role::Vertex,
            gfx::memory::Usage::Dynamic,
            buffer_bind.clone(),
        )?;
        let rect_index_buffer = factory.create_buffer(
            1,
            gfx::buffer::Role::Index,
            gfx::memory::Usage::Dynamic,
            buffer_bind.clone(),
        )?;
        let data = pipe::Data {
            vbuf: rect_vertex_buffer.clone(),
            transform: factory.create_constant_buffer(1),
            color_buffer: color_buffer.clone(),
            depth_buffer: depth_buffer.clone(),
        };*/

        Ok(Self {
            glyph_brush,
            fonts,
            /*pso,
            data,
            rect_vertex_buffer,
            rect_index_buffer,*/
        })
    }

    pub fn render<Message>(
        &mut self,
        target: &wgpu::TextureView,
        device: &mut wgpu::Device,
        data: &WindowData,
        ui: &quint::Ui<PrimitiveBuffer, Message>,
        _draw_crosshair: bool,
    ) -> Result<wgpu::CommandBuffer> {
        let mut primitive_buffer = PrimitiveBuffer::default();
        ui.render(&mut primitive_buffer);

        /*// Render primitives
        let mut rect_vertices: Vec<Vertex> = Vec::new();
        let mut rect_indices: Vec<u32> = Vec::new();

        // Rectangles
        for RectanglePrimitive {
            layout: l,
            color,
            z,
        } in primitive_buffer.rectangle.into_iter()
        {
            let a = Vertex {
                pos: [l.x, l.y, z],
                color: color.clone(),
            };
            let b = Vertex {
                pos: [l.x + l.width, l.y, z],
                color: color.clone(),
            };
            let c = Vertex {
                pos: [l.x, l.y + l.height, z],
                color: color.clone(),
            };
            let d = Vertex {
                pos: [l.x + l.width, l.y + l.height, z],
                color: color.clone(),
            };
            let a_index = rect_vertices.len() as u32;
            let b_index = a_index + 1;
            let c_index = b_index + 1;
            let d_index = c_index + 1;
            rect_vertices.extend([a, b, c, d].into_iter());
            rect_indices.extend([b_index, a_index, c_index, b_index, c_index, d_index].into_iter());
        }
        // Triangles
        for TrianglesPrimitive {
            vertices,
            indices,
            color,
        } in primitive_buffer.triangles.into_iter()
        {
            let index_offset = rect_vertices.len() as u32;
            rect_vertices.extend(vertices.into_iter().map(|v| Vertex { pos: v, color }));
            rect_indices.extend(indices.into_iter().map(|id| id + index_offset));
        }*/
        // Text
        for TextPrimitive {
            layout: l,
            mut parts,
            z,
            centered,
        } in primitive_buffer.text.into_iter()
        {
            let dpi = data.hidpi_factor as f32;

            for p in parts.iter_mut() {
                p.font_size.x *= dpi;
                p.font_size.y *= dpi;
            }
            let Self { ref fonts, .. } = &self;
            let parts = parts
                .iter()
                .map(|part| wgpu_glyph::SectionText {
                    text: &part.text,
                    scale: part.font_size,
                    color: part.color,
                    font_id: part
                        .font
                        .clone()
                        .and_then(|f| fonts.get(&f).cloned())
                        .unwrap_or_default(),
                })
                .collect();
            let section = if centered {
                wgpu_glyph::VariedSection {
                    text: parts,
                    screen_position: ((l.x + l.width / 2.0) * dpi, (l.y + l.height / 2.0) * dpi),
                    bounds: (l.width * dpi, l.height * dpi),
                    z,
                    layout: wgpu_glyph::Layout::Wrap {
                        line_breaker: Default::default(),
                        v_align: wgpu_glyph::VerticalAlign::Center,
                        h_align: wgpu_glyph::HorizontalAlign::Center,
                    },
                }
            } else {
                wgpu_glyph::VariedSection {
                    text: parts,
                    screen_position: (l.x * dpi, l.y * dpi),
                    bounds: (l.width * dpi, l.height * dpi),
                    z,
                    layout: Default::default(),
                }
            };
            self.glyph_brush.queue(section);
        }
        /*// Crosshair
        if draw_crosshair {
            let (cx, cy) = (
                data.logical_window_size.width as f32 / 2.0,
                data.logical_window_size.height as f32 / 2.0,
            );
            const HALF_HEIGHT: f32 = 15.0;
            const HALF_WIDTH: f32 = 2.0;
            const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
            let v1 = Vertex {
                pos: [cx - HALF_WIDTH, cy - HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v2 = Vertex {
                pos: [cx + HALF_WIDTH, cy - HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v3 = Vertex {
                pos: [cx - HALF_WIDTH, cy + HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v4 = Vertex {
                pos: [cx + HALF_WIDTH, cy + HALF_HEIGHT, -1.0],
                color: COLOR,
            };
            let v5 = Vertex {
                pos: [cx - HALF_HEIGHT, cy - HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v6 = Vertex {
                pos: [cx + HALF_HEIGHT, cy - HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v7 = Vertex {
                pos: [cx - HALF_HEIGHT, cy + HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let v8 = Vertex {
                pos: [cx + HALF_HEIGHT, cy + HALF_WIDTH, -1.0],
                color: COLOR,
            };
            let voffset = rect_vertices.len() as u32;
            rect_vertices.extend([v1, v2, v3, v4, v5, v6, v7, v8].into_iter());
            rect_indices.extend(
                [0, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6, 7]
                    .into_iter()
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
                [2.0 / win_w as f32, 0.0, 0.0, 0.0],
                [0.0, -2.0 / win_h as f32, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [-1.0, 1.0, 0.0, 1.0],
            ];
            encoder.update_constant_buffer(
                &self.data.transform,
                &Transform {
                    transform: transformation_matrix,
                    debug: false,
                },
            );
            // Update vertex buffer
            ensure_buffer_capacity(&mut self.rect_vertex_buffer, rect_vertices.len(), factory)?;
            encoder
                .update_buffer(&self.rect_vertex_buffer, &rect_vertices, 0)
                .context("Updating rectangle vertex buffer in UiRenderer")?;
            // Update index buffer
            ensure_buffer_capacity(&mut self.rect_index_buffer, rect_indices.len(), factory)?;
            encoder
                .update_buffer(&self.rect_index_buffer, &rect_indices, 0)
                .context("Updating rectangle index buffer in UiRenderer")?;
            // Create the Slice that dictates how to render the vertex buffer
            let slice = Slice {
                start: 0,                       // start at 0 in the index buffer
                end: rect_indices.len() as u32, // end at last element in the index buffer
                base_vertex: 0,                 // start at 0 in the vertex buffer
                instances: None,                // don't use instancing
                buffer: gfx::IndexBuffer::Index32(self.rect_index_buffer.clone()), // index buffer to use
            };
            // Update the vertex buffer in the pipeline data
            self.data.vbuf = self.rect_vertex_buffer.clone();
            self.data.color_buffer = color_buffer.clone();
            self.data.depth_buffer = depth_buffer.clone();
            // Draw the vertex buffer
            encoder.draw(&slice, &self.pso, &self.data);
        }*/

        // Draw text
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });
        self.glyph_brush
            .draw_queued(device, &mut encoder, target, data.physical_window_size.width.round() as u32, data.physical_window_size.height.round() as u32)
            .expect("couldn't draw queued glyphs");
        Ok(encoder.finish())
    }
}

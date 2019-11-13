use crate::window::{Gfx, WindowData};
use anyhow::{Context, Result};
use gfx;
use gfx::{
    traits::{Factory, FactoryExt},
    Slice,
};
use gfx_glyph::{GlyphBrush, GlyphBrushBuilder, Scale, FontId, VariedSection, SectionText};
use log::info;
use quint::Layout;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "a_Pos",
        color: [f32; 4] = "a_Color",
    }

    constant Transform {
        transform: [[f32; 4]; 4] = "u_Transform",
        debug: bool = "u_Debug",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::ConstantBuffer<Transform> = "Transform",
        color_buffer: gfx::BlendTarget<crate::window::ColorFormat> = ("ColorBuffer", gfx::state::ColorMask::all(), gfx::state::Blend {
            color: gfx::state::BlendChannel {
                equation: gfx::state::Equation::Add,
                source: gfx::state::Factor::ZeroPlus(gfx::state::BlendValue::SourceAlpha),
                destination: gfx::state::Factor::OneMinus(gfx::state::BlendValue::SourceAlpha),
            },
            alpha: gfx::state::BlendChannel {
                equation: gfx::state::Equation::Add,
                source: gfx::state::Factor::Zero,
                destination: gfx::state::Factor::One,
            },
        }),
        depth_buffer: gfx::DepthTarget<crate::window::DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type R = gfx_device_gl::Resources;
type PipeDataType = pipe::Data<R>;
type PsoType = gfx::PipelineState<R, pipe::Meta>;

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
    pub font_size: Scale,
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
            vertices, indices, color,
        });
    }
}

pub struct UiRenderer {
    // Glyph rendering
    glyph_brush: GlyphBrush<'static, R, gfx_device_gl::Factory>,
    // Rectangle rendering
    pso: PsoType,
    data: PipeDataType,
    rect_vertex_buffer: gfx::handle::Buffer<R, Vertex>,
    rect_index_buffer: gfx::handle::Buffer<R, u32>,
    fonts: HashMap<String, FontId>,
}

impl UiRenderer {
    pub fn new(gfx: &mut Gfx) -> Result<Self> {
        let Gfx {
            ref mut factory,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        // Load fonts
        let default_font: &'static [u8] = include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf");
        let mut glyph_brush_builder = GlyphBrushBuilder::using_font_bytes(default_font);
        info!("Loading fonts from assets/fonts/list.toml");
        let mut fonts = HashMap::new();
        let font_list = std::fs::read_to_string("assets/fonts/list.toml").expect("Couldn't read font list file");
        let font_files: BTreeMap<String, String> = toml::de::from_str(&font_list).expect("Couldn't parse font list file");
        for (font_name, font_file) in font_files.into_iter() {
            info!("Loading font {} from file {}", font_name, font_file);
            let mut font_bytes = Vec::new();
            let mut file = std::fs::File::open(font_file).expect("Couldn't open font file");
            file.read_to_end(&mut font_bytes).expect("Couldn't read font file");
            fonts.insert(font_name, glyph_brush_builder.add_font_bytes(font_bytes));
        }
        info!("Fonts successfully loaded");
        let glyph_brush = glyph_brush_builder.build(factory.clone());

        // Create rectangle drawing pipeline
        let shader_set = factory.create_shader_set(
            include_bytes!("../../shader/gui-rect.vert"),
            include_bytes!("../../shader/gui-rect.frag"),
        )?;
        let pso = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill(),
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
        };

        Ok(Self {
            glyph_brush,
            pso,
            data,
            rect_vertex_buffer,
            rect_index_buffer,
            fonts,
        })
    }

    pub fn render<Message>(
        &mut self,
        gfx: &mut Gfx,
        data: &WindowData,
        ui: &quint::Ui<PrimitiveBuffer, Message>,
        draw_crosshair: bool,
    ) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref mut factory,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        let mut primitive_buffer = PrimitiveBuffer::default();
        ui.render(&mut primitive_buffer);

        // Render primitives
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
        } in primitive_buffer.triangles.into_iter() {
            let index_offset = rect_vertices.len() as u32;
            rect_vertices.extend(vertices.into_iter().map(|v| Vertex { pos: v, color, }));
            rect_indices.extend(indices.into_iter().map(|id| id + index_offset));
        }
        // Text
        for TextPrimitive {
            layout: l,
            mut parts,
            z,
            centered,
        } in primitive_buffer.text.into_iter()
        {
            use gfx_glyph::{HorizontalAlign, Layout, VerticalAlign};
            let dpi = data.hidpi_factor as f32;

            for p in parts.iter_mut() {
                p.font_size.x *= dpi;
                p.font_size.y *= dpi;
            }
            let Self { ref fonts, .. } = &self;
            let parts = parts.iter().map(|part| {
                SectionText {
                    text: &part.text,
                    scale: part.font_size,
                    color: part.color,
                    font_id: part.font.clone().and_then(|f| fonts.get(&f).cloned()).unwrap_or_default(),
                }
            }).collect();
            let section = if centered {
                VariedSection {
                    text: parts,
                    screen_position: ((l.x + l.width / 2.0) * dpi, (l.y + l.height / 2.0) * dpi),
                    bounds: (l.width * dpi, l.height * dpi),
                    z,
                    layout: Layout::Wrap {
                        line_breaker: Default::default(),
                        v_align: VerticalAlign::Center,
                        h_align: HorizontalAlign::Center,
                    },
                }
            } else {
                VariedSection {
                    text: parts,
                    screen_position: (l.x * dpi, l.y * dpi),
                    bounds: (l.width, l.height),
                    z,
                    layout: Default::default(),
                }
            };
            self.glyph_brush.queue(section);
        }
        // Crosshair
        if draw_crosshair {
            let (cx, cy) = (data.logical_window_size.width as f32/2.0, data.logical_window_size.height as f32/2.0);
            const HALF_HEIGHT: f32 = 15.0;
            const HALF_WIDTH: f32 = 2.0;
            const COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.5];
            let v1 = Vertex { pos: [cx - HALF_WIDTH, cy - HALF_HEIGHT, -1.0], color: COLOR };
            let v2 = Vertex { pos: [cx + HALF_WIDTH, cy - HALF_HEIGHT, -1.0], color: COLOR };
            let v3 = Vertex { pos: [cx - HALF_WIDTH, cy + HALF_HEIGHT, -1.0], color: COLOR };
            let v4 = Vertex { pos: [cx + HALF_WIDTH, cy + HALF_HEIGHT, -1.0], color: COLOR };
            let v5 = Vertex { pos: [cx - HALF_HEIGHT, cy - HALF_WIDTH, -1.0], color: COLOR };
            let v6 = Vertex { pos: [cx + HALF_HEIGHT, cy - HALF_WIDTH, -1.0], color: COLOR };
            let v7 = Vertex { pos: [cx - HALF_HEIGHT, cy + HALF_WIDTH, -1.0], color: COLOR };
            let v8 = Vertex { pos: [cx + HALF_HEIGHT, cy + HALF_WIDTH, -1.0], color: COLOR };
            let voffset = rect_vertices.len() as u32;
            rect_vertices.extend([v1, v2, v3, v4, v5, v6, v7, v8].into_iter());
            rect_indices.extend([0, 1, 2, 1, 2, 3, 4, 5, 6, 5, 6, 7].into_iter().map(|id| id + voffset));
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
        }

        // Draw text
        self.glyph_brush
            .use_queue()
            .draw(encoder, color_buffer)
            .expect("couldn't draw queued glyphs");
        Ok(())
    }
}

pub fn ensure_buffer_capacity<T>(
    buffer: &mut gfx::handle::Buffer<R, T>,
    min_num: usize,
    factory: &mut gfx_device_gl::Factory,
) -> Result<(), gfx::buffer::CreationError> {
    let info = buffer.get_info().clone();
    let buffer_num = info.size / std::mem::size_of::<T>();
    if buffer_num < min_num {
        let new_buffer = factory.create_buffer(min_num, info.role, info.usage, info.bind)?;
        *buffer = new_buffer;
    }
    Ok(())
}

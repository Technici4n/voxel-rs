use crate::{
    ui::Ui,
    window::{Gfx, WindowData},
};
use anyhow::{Context, Result};
use gfx;
use gfx::{
    traits::{Factory, FactoryExt},
    Slice,
};
use gfx_glyph::{GlyphBrush, GlyphBrushBuilder, Scale, Section};
use quint::Layout;

#[derive(Debug)]
pub struct UiRenderingError {
    pub what: String,
}

impl std::fmt::Display for UiRenderingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Some error happened during rendering of the ui text: {}",
            self.what
        )
    }
}

impl std::error::Error for UiRenderingError {}

impl From<String> for UiRenderingError {
    fn from(string: String) -> Self {
        Self { what: string }
    }
}

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
pub struct RectanglePrimitive {
    pub layout: Layout,
    pub color: [f32; 4],
    pub z: f32,
}

#[derive(Debug, Clone)]
pub struct TextPrimitive {
    pub layout: Layout,
    pub text: String,
    pub font_size: Scale,
    pub z: f32,
    pub centered: bool,
}

#[derive(Default, Debug)]
pub struct PrimitiveBuffer {
    pub(self) rectangle: Vec<RectanglePrimitive>,
    pub(self) text: Vec<TextPrimitive>,
}

impl PrimitiveBuffer {
    pub fn draw_rectangle(&mut self, color: [f32; 4], layout: Layout, z: f32) {
        self.rectangle.push(RectanglePrimitive { color, layout, z });
    }

    pub fn draw_text(&mut self, text: String, font_size: Scale, layout: Layout, z: f32) {
        self.text.push(TextPrimitive { text, font_size, layout, z, centered: false });
    }

    pub fn draw_text_centered(&mut self, text: String, font_size: Scale, layout: Layout, z: f32) {
        self.text.push(TextPrimitive { text, font_size, layout, z, centered: true });
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
}

impl UiRenderer {
    pub fn new(gfx: &mut Gfx) -> Result<Self> {
        let Gfx {
            ref mut factory,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        // Create glyph renderer
        let ubuntu: &'static [u8] = include_bytes!("../../assets/fonts/Ubuntu-R.ttf");
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(ubuntu).build(factory.clone());

        // Create rectangle drawing pipeline
        let shader_set = factory.create_shader_set(
            include_bytes!("../../shader/gui-rect.vert"),
            include_bytes!("../../shader/gui-rect.frag"),
        )?;
        let pso = factory.create_pipeline_state(
            &shader_set,
            gfx::Primitive::TriangleList,
            gfx::state::Rasterizer::new_fill().with_cull_back(),
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
        })
    }

    pub fn render(&mut self, gfx: &mut Gfx, data: &WindowData, ui: &mut Ui) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref mut factory,
            ref color_buffer,
            ref depth_buffer,
            ..
        } = gfx;

        let mut primitive_buffer = PrimitiveBuffer::default();
        ui.ui.render(&mut primitive_buffer);

        // Render primitives
        let mut rect_vertices: Vec<Vertex> = Vec::new();
        let mut rect_indices: Vec<u32> = Vec::new();

        // Rectangles
        for RectanglePrimitive { layout: l, color, z } in primitive_buffer.rectangle.into_iter() {
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
            rect_indices.extend(
                [b_index, a_index, c_index, b_index, c_index, d_index].into_iter(),
            );
        }
        // Text
        for TextPrimitive { layout: l, text, font_size, z, centered } in primitive_buffer.text.into_iter() {
            use gfx_glyph::{ HorizontalAlign, Layout, VerticalAlign };
            // TODO: use HiDPI factor to adjust the position of the text
            // (gfx_glyph probably expects the physical size, but currently it's given the logical size)
            let section = if centered {
                Section {
                    text: &text,
                    screen_position: (l.x + l.width/2.0, l.y + l.height/2.0),
                    bounds: (l.width, l.height),
                    scale: font_size,
                    z,
                    layout: Layout::Wrap {
                        line_breaker: Default::default(),
                        v_align: VerticalAlign::Center,
                        h_align: HorizontalAlign::Center,
                    },
                    ..Section::default()
                }
            } else {
                Section {
                    text: &text,
                    screen_position: (l.x, l.y),
                    bounds: (l.width, l.height),
                    scale: font_size,
                    z,
                    ..Section::default()
                }
            };
            self.glyph_brush.queue(section);
        }

        // Draw rectangles
        {
            let (win_w, win_h) = (data.logical_window_size.width, data.logical_window_size.height);
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
            .map_err(UiRenderingError::from)
            .context("Drawing text glyphs in UiRenderer")?;
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

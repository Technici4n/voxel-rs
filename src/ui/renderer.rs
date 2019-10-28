use crate::{
    ui::Ui,
    window::{Gfx, RenderInfo},
};
use anyhow::Result;
use gfx;
use gfx::traits::{Factory, FactoryExt};
use gfx_glyph::{GlyphBrush, GlyphBrushBuilder, Section};
use stretch::{
    geometry::Size,
    number::Number,
};
use gfx::Slice;

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
        Self {
            what: string,
        }
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
        color_buffer: gfx::RenderTarget<crate::window::ColorFormat> = "ColorBuffer",
        depth_buffer: gfx::DepthTarget<crate::window::DepthFormat> =
            gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

type R = gfx_device_gl::Resources;
type PipeDataType = pipe::Data<R>;
type PsoType = gfx::PipelineState<R, pipe::Meta>;

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
    pub fn new(gfx: &mut Gfx, _render_info: &RenderInfo) -> Result<Self> {
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
        let rect_vertex_buffer = factory.create_vertex_buffer(&[]);
        let index_buffer_bind = {
            use gfx::memory::Bind;
            let mut bind = Bind::empty();
            bind.insert(Bind::SHADER_RESOURCE);
            bind.insert(Bind::TRANSFER_DST);
            bind
        };
        let rect_index_buffer = factory.create_buffer(1, gfx::buffer::Role::Index, gfx::memory::Usage::Dynamic, index_buffer_bind)?;
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

    pub fn render(&mut self, gfx: &mut Gfx, render_info: RenderInfo, ui: &mut Ui) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref color_buffer,
            ..
        } = gfx;

        let root_node = match ui.root_node {
            Some(root_node) => root_node,
            None => return Ok(()),
        };

        // Rebuild Ui
        let (win_w, win_h) = render_info.window_dimensions;
        let layout_size = Size {
            width: Number::Defined(win_w as f32),
            height: Number::Defined(win_h as f32),
        };
        ui.stretch.compute_layout(root_node, layout_size).map_err(super::UiError::from)?;

        // Recursively render every child of the root_node
        let mut rect_vertices: Vec<Vertex> = Vec::new();
        let mut rect_indices: Vec<u32> = Vec::new();
        let mut nodes = vec![root_node];
        while let Some(current_node) = nodes.pop() {
            // Enqueue children nodes
            if let Ok(children) = ui.stretch.children(current_node) {
                nodes.extend(children.into_iter());
            }
            // Process current node (if there is an associated primitive)
            if let Some(primitive) = ui.primitives.get(&current_node) {
                if let Ok(l) = ui.stretch.layout(current_node) {
                    use super::Primitive::*;
                    match primitive {
                        Nothing => {},
                        Rectangle { color } => {
                            // a --- b
                            // |  /  |
                            // c --- d
                            let a = Vertex {
                                pos: [ l.location.x, l.location.y, 0.0 ],
                                color: (*color).clone(),
                            };
                            let b = Vertex {
                                pos: [ l.location.x + l.size.width, l.location.y, 0.0 ],
                                color: (*color).clone(),
                            };
                            let c = Vertex {
                                pos: [ l.location.x, l.location.y + l.size.height, 0.0 ],
                                color: (*color).clone(),
                            };
                            let d = Vertex {
                                pos: [ l.location.x + l.size.width, l.location.y + l.size.height, 0.0 ],
                                color: (*color).clone(),
                            };
                            let a_index = rect_vertices.len() as u32;
                            let b_index = a_index+1;
                            let c_index = b_index+1;
                            let d_index = c_index+1;
                            rect_vertices.extend([a, b, c, d].into_iter());
                            rect_indices.extend([b_index, a_index, c_index, b_index, c_index, d_index].into_iter());
                        },
                        Text { text, font_size } => {
                            let section = Section {
                                text: &*text,
                                screen_position: (l.location.x, l.location.y),
                                bounds: (l.size.width, l.size.height),
                                scale: font_size.clone(),
                                ..Section::default()
                            };
                            self.glyph_brush.queue(section);
                        },
                    }
                }
            }
        }

        // Draw rectangles
        // TODO: update uniform buffer
        {
            // Update vertex buffer
            encoder.update_buffer(&self.rect_vertex_buffer, &rect_vertices, 0)?;
            // Update index buffer
            encoder.update_buffer(&self.rect_index_buffer, &rect_indices, 0)?;
            // Create the Slice that dictates how to render the vertex buffer
            let slice = Slice {
                start: 0, // start at 0 in the index buffer
                end: rect_indices.len() as u32, // end at last element in the index buffer
                base_vertex: 0, // start at 0 in the vertex buffer
                instances: None, // don't use instancing
                buffer: gfx::IndexBuffer::Index32(self.rect_index_buffer.clone()), // index buffer to use
            };
            // Note: the pipeline already contains the vertex buffer!
            // Draw the vertex buffer
            encoder.draw(&slice, &self.pso, &self.data);
        }

        // Draw text
        self.glyph_brush.use_queue().draw(encoder, color_buffer).map_err(UiRenderingError::from)?;
        Ok(())
    }
}

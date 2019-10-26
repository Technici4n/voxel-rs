use crate::{
    ui::Ui,
    window::{Gfx, RenderInfo},
};
use anyhow::Result;
use gfx_glyph::{GlyphBrush, GlyphBrushBuilder, Section};

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

pub struct UiRenderer {
    glyph_brush: GlyphBrush<'static, gfx_device_gl::Resources, gfx_device_gl::Factory>,
}

impl UiRenderer {
    pub fn new(gfx: &mut Gfx, _render_info: &RenderInfo) -> Result<Self> {
        let Gfx {
            ref mut factory, ..
        } = gfx;

        let ubuntu: &'static [u8] = include_bytes!("../../assets/fonts/Ubuntu-R.ttf");
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(ubuntu).build(factory.clone());

        Ok(Self { glyph_brush })
    }

    pub fn render(&mut self, gfx: &mut Gfx, _render_info: RenderInfo, ui: &mut Ui) -> Result<()> {
        let Gfx {
            ref mut encoder,
            ref color_buffer,
            ..
        } = gfx;

        let section = Section {
            text: ui.get_text(),
            ..Section::default()
        };
        self.glyph_brush.queue(section);
        self.glyph_brush
            .use_queue()
            .draw(encoder, color_buffer)
            .map_err(|what| UiRenderingError { what })?;

        Ok(())
    }
}

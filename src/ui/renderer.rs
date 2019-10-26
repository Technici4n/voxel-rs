use anyhow::Result;
use conrod_core::{image::Map, render::Primitives};
use conrod_gfx::Renderer;
use gfx_core::handle::ShaderResourceView;
use crate::{
    ui::Ui,
    window::{Gfx, RenderInfo},
};

pub type ImageMap = Map<(ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>, (u32, u32))>;

pub struct UiRenderer {
    pub renderer: Renderer<'static, gfx_device_gl::Resources>,
    image_map: ImageMap,
}

impl UiRenderer {
    pub fn new(gfx: &mut Gfx, renderInfo: &RenderInfo) -> Result<Self> {
        let Gfx { ref mut factory, ref color_buffer, .. } = gfx;

        Ok(Self {
            renderer: Renderer::new(factory, color_buffer, renderInfo.dpi_factor)?,
            image_map: ImageMap::new(),
        })
    }

    pub fn render(&mut self, gfx: &mut Gfx, renderInfo: RenderInfo, ui: &mut Ui) -> Result<()> {
        let Gfx { ref mut encoder, ref mut factory, .. } = gfx;

        // Rebuild primitives if necessary
        if let Some(primitives) = ui.draw_if_changed() {
            let (win_w, win_h) = renderInfo.window_dimensions;
            self.renderer.fill(encoder, (win_w as f32, win_h as f32), renderInfo.dpi_factor as f64, primitives, &self.image_map);
        }

        self.renderer.draw(factory, encoder, &self.image_map);

        Ok(())
    }
}
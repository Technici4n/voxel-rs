use gfx_glyph::Scale;
use quint::{Layout, Style, Widget};
use super::renderer::PrimitiveBuffer;

pub struct Rectangle {
    pub color: [f32; 4],
}

pub struct Text {
    pub text: String,
    pub font_size: Scale,
}

pub struct WithStyle {
    pub style: Style,
}

impl Widget<PrimitiveBuffer, ()> for Rectangle {
    fn style(&self) -> Style {
        Style::default().percent_size(1.0, 1.0)
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, layout: Layout) {
        buffer.draw_rectangle(self.color.clone(), layout);
    }
}

impl Widget<PrimitiveBuffer, ()> for Text {
    fn style(&self) -> Style {
        Style::default().percent_size(1.0, 1.0)
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, layout: Layout) {
        buffer.draw_text(self.text.clone(), self.font_size, layout);
    }
}

impl Widget<PrimitiveBuffer, ()> for WithStyle {
    fn style(&self) -> Style {
        self.style.clone()
    }

    fn render(&self, _buffer: &mut PrimitiveBuffer, _layout: Layout) {}
}
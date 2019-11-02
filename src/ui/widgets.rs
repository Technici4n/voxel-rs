use gfx_glyph::Scale;
use quint::{Event, Layout, Style, Widget, Position};
use super::renderer::PrimitiveBuffer;

pub struct Rectangle {
    pub color: [f32; 4],
    pub z: f32,
}

pub struct Text {
    pub text: String,
    pub font_size: Scale,
}

pub struct WithStyle {
    pub style: Style,
}

pub struct Button<Message>
    where Message: Clone {
    pub message: Message,
    pub text: String,
    pub font_size: Scale,
    pub style: Style,
}

impl<T> Widget<PrimitiveBuffer, T> for Rectangle {
    fn style(&self) -> Style {
        Style::default().percent_size(1.0, 1.0)
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, _cursor_position: Position, layout: Layout) {
        buffer.draw_rectangle(self.color.clone(), layout, self.z);
    }
}

impl<T> Widget<PrimitiveBuffer, T> for Text {
    fn style(&self) -> Style {
        Style::default().percent_size(1.0, 1.0)
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, _cursor_position: Position, layout: Layout) {
        buffer.draw_text(self.text.clone(), self.font_size, layout, 0.0);
    }
}

impl<T> Widget<PrimitiveBuffer, T> for WithStyle {
    fn style(&self) -> Style {
        self.style.clone()
    }
}

impl<T> Widget<PrimitiveBuffer, T> for Button<T>
    where T: Clone {
    fn style(&self) -> Style { self.style.clone() }

    fn render(&self, buffer: &mut PrimitiveBuffer, cursor_position: Position, layout: Layout) {
        let background_color = if layout.is_position_inside(cursor_position) {
            [0.7, 0.9, 0.7, 1.0]
        } else {
            [0.7, 0.7, 0.7, 1.0]
        };
        buffer.draw_rectangle([0.0, 0.0, 0.0, 1.0], layout, 0.01);
        buffer.draw_rectangle(background_color, layout.with_padding(4.0), 0.0);
        buffer.draw_text_centered(self.text.clone(), self.font_size, layout, 0.1);
    }

    fn on_event(&self, event: Event, layout: Layout, cursor_position: Position, messages: &mut Vec<T>) {
        if let Event::MouseInput { button, state } = event {
            if let quint::MouseButton::Left = button {
                if let quint::ButtonState::Pressed = state {
                    if layout.is_position_inside(cursor_position) {
                        messages.push(self.message.clone());
                    }
                }
            }
        }
    }
}
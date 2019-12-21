use super::{PrimitiveBuffer, TextPart};
use quint::{Event, Layout, Position, Style, Widget};

pub struct Text {
    pub text: Vec<TextPart>,
}

pub struct WithStyle {
    pub style: Style,
}

pub struct Button<Message>
where
    Message: Clone,
{
    pub message: Message,
    pub text: Vec<TextPart>,
    pub style: Style,
}

impl<T> Widget<PrimitiveBuffer, T> for Text {
    fn style(&self) -> Style {
        Style::default().percent_size(1.0, 1.0)
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, _cursor_position: Position, layout: Layout) {
        //buffer.draw_text(self.text.clone(), layout, 0.0, false);
    }
}

impl<T> Widget<PrimitiveBuffer, T> for WithStyle {
    fn style(&self) -> Style {
        self.style.clone()
    }
}

impl<T> Widget<PrimitiveBuffer, T> for Button<T>
where
    T: Clone,
{
    fn style(&self) -> Style {
        self.style.clone()
    }

    fn render(&self, buffer: &mut PrimitiveBuffer, cursor_position: Position, mut l: Layout) {
        let hovering = l.is_position_inside(cursor_position);
        // Padded Layout
        let mut pl = l.with_padding(6.0);
        if hovering {
            l.y += 2.0;
            pl.y += 2.0;
        }

        let main_color = if hovering {
            [0.75, 0.22, 0.22, 1.0]
        } else {
            [0.8, 0.2, 0.2, 1.0]
        };
        let dark_shade = if hovering {
            [0.55, 0.12, 0.12, 1.0]
        } else {
            [0.6, 0.1, 0.1, 1.0]
        };
        let light_shade = if hovering {
            [0.95, 0.32, 0.32, 1.0]
        } else {
            [1.0, 0.3, 0.3, 1.0]
        };

        // Top-left lighter shade
        buffer.draw_triangles(
            vec![
                [l.x, l.y + l.height, 0.0],
                [l.x, l.y, 0.0],
                [l.x + l.width, l.y, 0.0],
                [pl.x, pl.y + pl.height, 0.0],
                [pl.x, pl.y, 0.0],
                [pl.x + pl.width, pl.y, 0.0],
            ],
            vec![0, 3, 1, 1, 3, 4, 4, 5, 1, 1, 5, 2],
            light_shade,
        );
        // Bottom-right darker shade
        buffer.draw_triangles(
            vec![
                [l.x + l.width, l.y, 0.0],
                [l.x + l.width, l.y + l.height, 0.0],
                [l.x, l.y + l.height, 0.0],
                [pl.x + pl.width, pl.y, 0.0],
                [pl.x + pl.width, pl.y + pl.height, 0.0],
                [pl.x, pl.y + pl.height, 0.0],
            ],
            vec![0, 3, 1, 1, 3, 4, 4, 5, 1, 1, 5, 2],
            dark_shade,
        );
        buffer.draw_rectangle(main_color, pl, 0.0);

        if hovering {
            l.y += 2.0;
        }
        //buffer.draw_text(self.text.clone(), l, 0.1, true);
    }

    fn on_event(
        &self,
        event: Event,
        layout: Layout,
        cursor_position: Position,
        messages: &mut Vec<T>,
    ) {
        let Event::MouseInput { button, state } = event;
        if let quint::MouseButton::Left = button {
            if let quint::ButtonState::Pressed = state {
                if layout.is_position_inside(cursor_position) {
                    messages.push(self.message.clone());
                }
            }
        }
    }
}

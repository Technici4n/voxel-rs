use self::widgets::{Text, WithStyle};
use crate::ui::widgets::Button;
use crate::window::WindowData;
use crate::world::camera::Camera;
use anyhow::Result;
use gfx_glyph::Scale;
use glutin::dpi::LogicalPosition;
use quint::{wt, Size, Style, WidgetTree};

pub mod renderer;
pub mod widgets;

#[derive(Debug, Clone, Copy)]
pub enum Message {
    ExitMenu,
    ExitGame,
}

pub struct Ui {
    pub ui: quint::Ui<renderer::PrimitiveBuffer, Message>,
    messages: Vec<Message>,
    show_menu: bool,
    should_exit: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            ui: quint::Ui::new(),
            messages: Vec::new(),
            show_menu: false,
            should_exit: false,
        }
    }

    pub fn cursor_moved(&mut self, p: LogicalPosition) {
        self.ui.set_cursor_position(quint::Position {
            x: p.x as f32,
            y: p.y as f32,
        });
    }

    pub fn should_update_camera(&self) -> bool {
        !self.show_menu
    }

    /// Rebuild the Ui if it changed
    pub fn rebuild(&mut self, camera: &Camera, fps: usize, data: &WindowData) -> Result<()> {
        self.update();

        let mut layers = Vec::new();

        // Always draw debug info
        {
            layers.push(self.draw_debug_info(camera, fps));
        }

        // Draw menu
        if self.show_menu {
            layers.push(self.draw_menu());
        }

        let (win_w, win_h) = (
            data.logical_window_size.width,
            data.logical_window_size.height,
        );
        self.ui.rebuild(
            layers,
            Size {
                width: win_w as f32,
                height: win_h as f32,
            },
        );

        Ok(())
    }

    fn draw_debug_info(
        &self,
        camera: &Camera,
        fps: usize,
    ) -> WidgetTree<renderer::PrimitiveBuffer, Message> {
        let text = format!(
            "\
Welcome to voxel-rs

FPS = {}

yaw = {:4.0}
pitch = {:4.0}

x = {:.2}
y = {:.2}
z = {:.2}
",
            fps, camera.yaw, camera.pitch, camera.position.x, camera.position.y, camera.position.z
        );

        wt! {
            WithStyle { style: Style::default().percent_size(1.0, 1.0) },
            wt! {
                Text { text: text, font_size: Scale::uniform(20.0) },
            },
        }
    }

    fn draw_menu(&self) -> WidgetTree<renderer::PrimitiveBuffer, Message> {
        let menu_button = |text: &'static str, message| {
            wt! {
                Button {
                    text: text.to_owned(),
                    font_size: Scale::uniform(40.0),
                    message,
                    style: Style::default().absolute_size(400.0, 100.0),
                },
            }
        };

        let buttons_container = WidgetTree::new(
            Box::new(WithStyle {
                style: Style::default()
                    .percent_size(1.0, 1.0)
                    .center_cross()
                    .center_main()
                    .vertical(),
            }),
            vec![
                menu_button("Resume", Message::ExitMenu),
                menu_button("Exit", Message::ExitGame),
            ],
        );
        buttons_container
    }

    pub fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(glutin::MouseButton, glutin::ElementState)>,
    ) {
        let changes = changes
            .into_iter()
            .map(|(button, state)| quint::Event::MouseInput {
                button: quint_mouse_button(button),
                state: quint_element_state(state),
            })
            .collect();
        self.messages.extend(self.ui.update(changes));
    }

    pub fn handle_key_state_changes(&mut self, changes: Vec<(u32, glutin::ElementState)>) {
        for (key, state) in changes.into_iter() {
            // Escape key
            if key == 1 {
                if let glutin::ElementState::Pressed = state {
                    self.show_menu = !self.show_menu;
                }
            }
        }
    }

    fn update(&mut self) {
        for message in self.messages.drain(..) {
            match message {
                Message::ExitMenu => self.show_menu = false,
                Message::ExitGame => self.should_exit = true,
            }
        }
    }

    pub fn should_capture_mouse(&self) -> bool {
        !self.show_menu
    }

    pub fn should_exit(&self) -> bool {
        self.should_exit
    }
}

pub fn quint_mouse_button(button: glutin::MouseButton) -> quint::MouseButton {
    use glutin::MouseButton::*;
    match button {
        Left => quint::MouseButton::Left,
        Right => quint::MouseButton::Right,
        Middle => quint::MouseButton::Middle,
        Other(x) => quint::MouseButton::Other(x),
    }
}

pub fn quint_element_state(state: glutin::ElementState) -> quint::ButtonState {
    match state {
        glutin::ElementState::Pressed => quint::ButtonState::Pressed,
        glutin::ElementState::Released => quint::ButtonState::Released,
    }
}

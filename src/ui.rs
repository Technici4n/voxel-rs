use crate::{window::WindowData, world::World};
use anyhow::Result;
use gfx_glyph::Scale;
use glutin::dpi::LogicalPosition;
use quint::{Style, WidgetTree, Size};
use self::{
    widgets::{Text, WithStyle},
};

pub mod renderer;
pub mod widgets;

/// A Ui primitive
#[derive(Debug, Clone)]
pub enum Primitive {
    Nothing,
    Rectangle { color: [f32; 4], hover_color: [f32; 4], hovered: bool },
    Text { text: String, font_size: Scale },
}

impl Primitive {
    pub fn set_hover(&mut self, hover: bool) {
        match self {
            Self::Nothing => (),
            Self::Rectangle { ref mut hovered, .. } => *hovered = true,
            Self::Text { .. } => (),
        }
    }
}

#[derive(Debug)]
pub struct UiError {
    pub what: String,
}

impl std::fmt::Display for UiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Some error happened during creation of the Ui: {}",
            self.what
        )
    }
}

/// The user interface. Every element is represented by an id of type `Node`.
/// It is layouted using flexbox
pub struct Ui {
    pub(self) ui: quint::Ui<renderer::PrimitiveBuffer, ()>,
    cursor_position: LogicalPosition,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            ui: quint::Ui::new(),
            cursor_position: (10000, 10000).into(),
        }
    }

    pub fn cursor_moved(&mut self, p: LogicalPosition) {
        self.cursor_position = p;
    }

    pub(self) fn update_hover(&mut self) {

    }

    pub fn mouse_input(&mut self, state: glutin::ElementState, mouse: glutin::MouseButton) {

    }

    /// Rebuild the Ui if it changed
    pub fn rebuild(&mut self, world: &World, fps: usize, data: &WindowData) -> Result<()> {
        let camera = &world.camera;
        let text = {
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
            let text_tree = WidgetTree::new_leaf(Box::new(Text {
                text,
                font_size: Scale::uniform(20.0),
            }));
            WidgetTree::new(
                Box::new(WithStyle { style: Style::default().percent_width(0.5) }),
                vec![text_tree],
            )
        };
        let tree = WidgetTree::new(
            Box::new(WithStyle { style: Style::default().percent_size(1.0, 1.0) }),
            vec![text],
        );

        let (win_w, win_h) = (data.logical_window_size.width, data.logical_window_size.height);
        self.ui.rebuild(vec![tree], Size { width: win_w as f32, height: win_h as f32 });

        Ok(())
    }
}


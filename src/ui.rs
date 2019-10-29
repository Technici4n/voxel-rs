use crate::world::World;
use anyhow::Result;
use gfx_glyph::Scale;
use glutin::dpi::LogicalPosition;
use std::collections::HashMap;
use stretch::{node::Node, style::Style, Stretch};

pub mod renderer;

/// A Ui primitive
pub enum Primitive {
    Nothing,
    Rectangle { color: [f32; 4] },
    Text { text: String, font_size: Scale },
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

impl std::error::Error for UiError {}

impl From<stretch::Error> for UiError {
    fn from(error: stretch::Error) -> Self {
        Self {
            what: format!("{}", error),
        }
    }
}

/// The user interface. Every element is represented by an id of type `Node`.
/// It is layouted using flexbox
pub struct Ui {
    pub(self) stretch: Stretch,
    pub(self) primitives: HashMap<Node, Primitive>,
    pub(self) root_node: Option<Node>,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            stretch: Stretch::new(),
            primitives: HashMap::new(),
            root_node: None,
        }
    }

    pub fn new_node(
        &mut self,
        style: Style,
        children: Vec<Node>,
        primitive: Primitive,
    ) -> Result<Node, stretch::Error> {
        let node = self.stretch.new_node(style, children)?;
        self.primitives.insert(node, primitive);
        Ok(node)
    }

    pub fn cursor_moved(&mut self, cursor_position: LogicalPosition) {

    }

    pub fn mouse_input(&mut self, state: glutin::ElementState, mouse: glutin::MouseButton) {

    }

    /// Rebuild the Ui if it changed
    pub fn build_if_changed(&mut self, world: &World, fps: usize) -> Result<()> {
        let camera = &world.camera;
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

        use stretch::geometry::*;
        use stretch::style::*;

        let container_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::RowReverse,
            size: Size {
                width: Dimension::Percent(1.0),
                height: Dimension::Percent(1.0),
            },
            ..Style::default()
        };
        let text_style = Style {
            size: Size {
                width: Dimension::Percent(0.5),
                height: Dimension::Percent(1.0),
            },
            ..Style::default()
        };
        let subcontainer_style = Style {
            size: Size {
                width: Dimension::Percent(0.5),
                height: Dimension::Percent(1.0),
            },
            display: Display::Flex,
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::SpaceBetween,
            ..Style::default()
        };

        // Clear nodes
        self.stretch.clear();
        self.primitives.clear();

        let rect_style = Style {
            size: Size {
                width: Dimension::Points(100.0),
                height: Dimension::Points(100.0),
            },
            margin: Rect {
                start: Dimension::Points(10.0),
                end: Dimension::Points(10.0),
                top: Dimension::Points(10.0),
                bottom: Dimension::Points(10.0),
            },
            ..Style::default()
        };
        // Add small rectangles
        let rectangles: Vec<_> = (0..20)
            .into_iter()
            .map(|_| {
                let node = self.stretch.new_node(rect_style, vec![]).unwrap();
                //self.primitives.insert(node, Primitive::Text { text: format!("{}", i+1), font_size: Scale::uniform(40.0) });
                self.primitives.insert(
                    node,
                    Primitive::Rectangle {
                        color: [1.0, 0.0, 0.0, 0.5],
                    },
                );
                node
            })
            .collect();

        // Register stretch nodes
        let text_node = self
            .stretch
            .new_node(text_style, vec![])
            .map_err(UiError::from)?;
        let subcontainer_node = self
            .stretch
            .new_node(subcontainer_style, rectangles)
            .map_err(UiError::from)?;
        let root_node = self
            .stretch
            .new_node(container_style, vec![text_node, subcontainer_node])
            .map_err(UiError::from)?;
        self.root_node = Some(root_node);

        // Register primitive
        self.primitives.insert(
            text_node,
            Primitive::Text {
                text,
                font_size: Scale::uniform(20.0),
            },
        );

        Ok(())
    }
}

/*

/// Wrapper around the ui
pub struct Ui {
    /// The text that is shown
    text: String,
}


impl Ui {
    /// Create a new ui
    pub fn new() -> Result<Self> {
        Ok(Self {
            text: String::from("Welcome to voxel-rs"),
        })
    }

    /// Handle a glutin event
    pub fn handle_event(&mut self, _event: glutin::Event, _window: &glutin::Window) {
        // TODO: remove or implement
    }

    /// Rebuild the Ui if it changed
    pub fn build_if_changed(&mut self, world: &World) {
        let camera = &world.camera;
        self.text = format!(
            "\
Welcome to voxel-rs

yaw = {:4.0}
pitch = {:4.0}

x = {:.2}
y = {:.2}
z = {:.2}
",
            camera.yaw, camera.pitch, camera.position.x, camera.position.y, camera.position.z
        );
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Should the cursor be automatically centered and hidden?
    pub fn should_hide_and_center_cursor(&self) -> bool {
        true
    }
}
*/

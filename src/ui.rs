use crate::world::World;
use anyhow::Result;
use gfx_glyph::Scale;
use glutin::dpi::LogicalPosition;
use std::collections::HashMap;
use stretch::{node::Node, style::Style, Stretch};

pub mod renderer;

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
    pub(self) hovered_nodes: Vec<Node>,
    cursor_position: LogicalPosition,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            stretch: Stretch::new(),
            primitives: HashMap::new(),
            root_node: None,
            hovered_nodes: Vec::new(),
            cursor_position: (10000, 10000).into(),
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

    pub fn cursor_moved(&mut self, p: LogicalPosition) {
        self.cursor_position = p;
    }

    pub(self) fn update_hover(&mut self) {
        let p = self.cursor_position.clone();
        // Clear hover flags
        for hovered_node in self.hovered_nodes.iter() {
            self.primitives.entry(*hovered_node).and_modify(|primitive| primitive.set_hover(false));
        }
        // Recompute hovered nodes
        self.hovered_nodes.clear();
        // Recursively check position
        let root_node = match self.root_node {
            Some(root_node) => root_node,
            None => return,
        };
        let mut nodes = vec![root_node];
        while let Some(current_node) = nodes.pop() {
            if let Ok(children) = self.stretch.children(current_node) {
                nodes.extend(children.into_iter());
            }
            if let Ok(l) = self.stretch.layout(current_node) {
                if l.location.x <= p.x as f32 && (p.x as f32) < l.location.x + l.size.width && l.location.y <= p.y as f32 && (p.y as f32) < l.location.y + l.size.height {
                    self.hovered_nodes.push(current_node);
                }
            }
        }
        // Set hover flags
        for hovered_node in self.hovered_nodes.iter() {
            self.primitives.entry(*hovered_node).and_modify(|primitive| primitive.set_hover(true));
        }
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
                        hover_color: [0.0, 1.0, 0.0, 0.5],
                        hovered: false,
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

use crate::{Event, Layout, Position, Size, Style};
use std::collections::HashMap;
use stretch::{node::Node, Stretch};

struct UiLayer<Renderer, Message> {
    pub(self) root_node: Node,
    pub(self) stretch: Stretch,
    pub(self) widgets: HashMap<Node, Box<dyn Widget<Renderer, Message>>>,
}

/// User interface rendered using a `Renderer` and with widgets sending messages of type `Message`.
///
/// Every frame, you should first update and then you rebuild the Ui.
/// Don't forget to set the cursor position whenever it changes.
pub struct Ui<Renderer, Message> {
    cursor_position: Position,
    layers: Vec<UiLayer<Renderer, Message>>,
}

impl<Renderer, Message> Ui<Renderer, Message> {
    pub fn new() -> Self {
        Self {
            cursor_position: Position::default(),
            layers: Vec::new(),
        }
    }

    /// Set the cursor position
    pub fn set_cursor_position(&mut self, position: Position) {
        self.cursor_position = position;
    }

    /// Process some events
    pub fn update(&mut self, events: Vec<Event>) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in events.into_iter() {
            self.propagate_event(event, &mut messages);
        }
        messages
    }

    fn propagate_event(&self, event: Event, messages: &mut Vec<Message>) {
        for layer in self.layers.iter() {
            let mut node_stack = vec![layer.root_node];
            while let Some(current_node) = node_stack.pop() {
                // Update widget if it exists
                if let Some(widget) = layer.widgets.get(&current_node) {
                    let layout = layer
                        .stretch
                        .layout(current_node)
                        .expect("Couldn't get Node layout");
                    widget.on_event(
                        event,
                        Layout::from_stretch(*layout),
                        self.cursor_position,
                        messages,
                    );
                }

                // Push child widgets onto the stack
                node_stack.extend(
                    layer
                        .stretch
                        .children(current_node)
                        .expect("Couldn't get Node children")
                        .into_iter(),
                );
            }
        }
    }

    /// Recursively register a WidgetTree
    fn register_widget_tree(
        stretch: &mut Stretch,
        widgets: &mut HashMap<Node, Box<dyn Widget<Renderer, Message>>>,
        widget_tree: WidgetTree<Renderer, Message>,
    ) -> Node {
        let WidgetTree { root, children } = widget_tree;
        let child_nodes: Vec<Node> = children
            .into_iter()
            .map(|child| Self::register_widget_tree(stretch, widgets, child))
            .collect();
        let style = root.style();
        let node_id = stretch
            .new_node(style.style, child_nodes)
            .expect("Couldn't create node");
        widgets.insert(node_id, root);
        node_id
    }

    /// Rebuild the Ui using the provided layers. The layers are rendered last-to-first.
    pub fn rebuild(&mut self, layers: Vec<WidgetTree<Renderer, Message>>, dimensions: Size) {
        self.layers = layers
            .into_iter()
            .map(|tree| {
                let mut stretch = Stretch::new();
                let mut widgets = HashMap::new();
                let root_node = Self::register_widget_tree(&mut stretch, &mut widgets, tree);
                stretch
                    .compute_layout(root_node, dimensions.into_stretch())
                    .expect("Couldn't compute layout");
                UiLayer {
                    stretch,
                    widgets,
                    root_node,
                }
            })
            .collect();
    }

    /// Render the Ui using the provided `Renderer`.
    pub fn render(&self, renderer: &mut Renderer) {
        // Recursively render every widget of every layer, the last layer being rendered first
        for layer in self.layers.iter().rev() {
            let mut render_stack = vec![layer.root_node];
            while let Some(current_node) = render_stack.pop() {
                // Draw widget if it exists
                if let Some(widget) = layer.widgets.get(&current_node) {
                    let layout = layer
                        .stretch
                        .layout(current_node)
                        .expect("Couldn't get Node layout");
                    widget.render(
                        renderer,
                        self.cursor_position,
                        Layout::from_stretch(*layout),
                    );
                }

                // Push child widgets onto the stack
                let children = layer
                    .stretch
                    .children(current_node)
                    .expect("Couldn't get Node children");
                render_stack.extend(children.into_iter());
            }
        }
    }
}

/// A generic Widget.
pub trait Widget<Renderer, Message> {
    // TODO: add screen size
    /// Compute the expected style of the widget
    fn style(&self) -> Style;
    /// Render the widget using the renderer
    fn render(&self, _renderer: &mut Renderer, _cursor_position: Position, _layout: Layout) {}
    /// Process one event
    fn on_event(
        &self,
        _event: Event,
        _layout: Layout,
        _cursor_position: Position,
        _messages: &mut Vec<Message>,
    ) {
    }
}

/// A tree of widgets
pub struct WidgetTree<Renderer, Message> {
    pub(self) root: Box<dyn Widget<Renderer, Message>>,
    pub(self) children: Vec<WidgetTree<Renderer, Message>>,
}

impl<Renderer, Message> WidgetTree<Renderer, Message> {
    /// Create a new leaf
    pub fn new_leaf(root: Box<dyn Widget<Renderer, Message>>) -> Self {
        Self {
            root,
            children: Vec::new(),
        }
    }

    /// Create a new node
    pub fn new(
        root: Box<dyn Widget<Renderer, Message>>,
        children: Vec<WidgetTree<Renderer, Message>>,
    ) -> Self {
        Self { root, children }
    }
}

/// A macro to quickly generate a `WidgetTree` without having to use `Box::new` and `vec!`
#[macro_export]
macro_rules! wt {
    ($root:expr,) => {{
        WidgetTree::new_leaf(Box::new($root))
    }};
    ($root:expr, $children:expr,) => {{
        WidgetTree::new(Box::new($root), vec![$children])
    }};
}

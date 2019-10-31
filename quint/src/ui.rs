use crate::{
    Event,
    Layout,
    Position,
    Size,
    Style,
};
use std::collections::HashMap;
use stretch::{
    node::Node,
    Stretch,
};

pub struct Ui<Renderer, Message> {
    //cursor_position: Position,
    root_nodes: Vec<Node>,
    stretch: Stretch,
    widgets: HashMap<Node, Box<dyn Widget<Renderer, Message>>>,
}

// 1) process events on the previous layout
// 2) rebuild, relayout, redraw
impl<Renderer, Message> Ui<Renderer, Message> {
    pub fn new() -> Self {
        Self {
            //cursor_position: Position::default(),
            root_nodes: Vec::new(),
            stretch: Stretch::new(),
            widgets: HashMap::new(),
        }
    }

    pub fn rebuild(&mut self, layers: Vec<WidgetTree<Renderer, Message>>, dimensions: Size) {
        self.root_nodes.clear();
        self.stretch.clear();
        self.widgets.clear();

        // Register one widget
        let mut register_widget = |widget, style: Style, child_nodes| {
            let node_id = self.stretch.new_node(style.style, child_nodes).expect("Couldn't create node");
            self.widgets.insert(node_id, widget);
            node_id
        };

        // Recursively register a full widget tree
        fn register_nodes<Renderer, Message>(register_widget: &mut impl FnMut(Box<dyn Widget<Renderer, Message>>, Style, Vec<Node>) -> Node, widget_tree: WidgetTree<Renderer, Message>) -> Node {
            let WidgetTree {
                root,
                children,
            } = widget_tree;
            let child_nodes: Vec<Node> = children.into_iter().map(|child| register_nodes(register_widget, child)).collect();
            let style = root.style();
            register_widget(root, style, child_nodes)
        }

        // Register all widget trees and keep track of the `Node`s
        self.root_nodes = layers.into_iter().map(|tree| register_nodes(&mut register_widget, tree)).collect();

        for root_node in self.root_nodes.iter() {
            self.stretch.compute_layout(*root_node, dimensions.into_stretch()).expect("Couldn't compute layout");
        }
    }

    pub fn render(&self, renderer: &mut Renderer) {
        // Recursively render every widget of every layer, the last layer being rendered first
        let mut render_stack = self.root_nodes.clone();
        while let Some(current_node) = render_stack.pop() {
            // Draw widget if it exists
            if let Some(widget) = self.widgets.get(&current_node) {
                let layout = self.stretch.layout(current_node).expect("Couldn't get Node layout");
                widget.render(renderer, Layout::from_stretch(*layout));
            }

            // Push child widgets onto the stack
            render_stack.extend(self.stretch.children(current_node).expect("Couldn't get Node children").into_iter());
        }
    }
}


// What can a widget do ?
// - receive events and send messages
// - compute its style and somehow register children to be styled and layouted
//
pub trait Widget<Renderer, Message> {
    // TODO: add screen size
    fn style(&self) -> Style;
    fn render(&self, renderer: &mut Renderer, layout: Layout);
    //fn on_event(&self, event: Event, layout: Layout, cursor_position: Position, messages: &mut Vec<Message>);
}

pub struct WidgetTree<Renderer, Message> {
    pub root: Box<dyn Widget<Renderer, Message>>,
    pub children: Vec<WidgetTree<Renderer, Message>>,
}

impl<Renderer, Message> WidgetTree<Renderer, Message> {
    pub fn new_leaf(root: Box<dyn Widget<Renderer, Message>>) -> Self {
        Self {
            root,
            children: Vec::new(),
        }
    }

    pub fn new(root: Box<dyn Widget<Renderer, Message>>, children: Vec<WidgetTree<Renderer, Message>>) -> Self {
        Self {
            root, children
        }
    }
}
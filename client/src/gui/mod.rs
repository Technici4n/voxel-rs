use crate::ui::PrimitiveBuffer;

pub mod experiments;

/// Immediate-mode GUI
pub struct Gui {
    mouse_x: i32,
    mouse_y: i32,
    mouse_down: bool,

    hot_item: u32,
    /// Active item. Ids 0 and 1 are reserved.
    /// 0 means "no item but can be assigned"
    /// 1 means "no item and cannot be assigned"
    active_item: u32,

    primitives: PrimitiveBuffer,
}

impl Gui {
    /// Create a new Gui
    pub fn new() -> Self {
        Self {
            mouse_x: 0,
            mouse_y: 0,
            mouse_down: false,
            hot_item: 0,
            active_item: 0,
            primitives: Default::default(),
        }
    }

    /// Update the mouse position
    pub fn update_mouse_position(&mut self, new_x: i32, new_y: i32) {
        self.mouse_x = new_x;
        self.mouse_y = new_y;
    }

    /// Update the state of the mouse button
    pub fn update_mouse_button(&mut self, is_down: bool) {
        self.mouse_down = is_down;
    }

    /// Drain stores primitives
    pub fn drain_primitives(&mut self) -> PrimitiveBuffer {
        std::mem::replace(&mut self.primitives, PrimitiveBuffer::default())
    }

    /// Prepare for frame drawing
    pub fn prepare(&mut self) {
        self.hot_item = 0;
    }

    /// Finish the frame
    pub fn finish(&mut self) {
        if !self.mouse_down {
            // If the mouse button is not down, then we allow an item to become active
            // when the mouse button will be pressed.
            self.active_item = 0;
        } else {
            // If the mouse button is down but there is no active item, we make sure that
            // an item cannot become active if the mouse is dragged onto it.
            if self.active_item == 0 {
                self.active_item = 1;
            }
        }
    }

    /// Is the mouse inside the rectangle
    pub fn is_mouse_inside(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        x <= self.mouse_x && self.mouse_x < x + w && y <= self.mouse_y && self.mouse_y < y + h
    }

    /// Draw a button, returning whether the button was pressed
    pub fn button(&mut self, mut id: u32, x: i32, y: i32, w: i32, h: i32) -> bool {
        id += 2;
        // Check if the mouse is inside the button
        if self.is_mouse_inside(x, y, w, h) {
            // Then the button is hot
            self.hot_item = id;
            // Maybe the button should also be active
            if self.active_item == 0 && self.mouse_down {
                self.active_item = id;
            }
        }
        // Draw the button
        self.primitives.draw_rect(x + 3, y + 3, w, h, [0.0, 0.0, 0.0, 1.0], 0.02);
        if self.hot_item == id {
            if self.active_item == id {
                // Hot and active
                self.primitives.draw_rect(x + 2, y + 2, w, h, [0.7, 0.7, 0.7, 1.0], 0.01);
            } else {
                // Just hot
                self.primitives.draw_rect(x, y, w, h, [0.7, 0.7, 0.7, 1.0], 0.01);
            }
        } else {
            // Not hot but might be active
            self.primitives.draw_rect(x, y, w, h, [0.8, 0.8, 0.8, 1.0], 0.01);
        }
        // If the mouse button is down but this button is both hot and active, it must have been clicked
        if !self.mouse_down && self.active_item == id && self.hot_item == id {
            return true
        }
        false
    }

    /// Draw text, aligned to the left but centered vertically
    pub fn text(&mut self, x: i32, y: i32, h: i32, text: String, color: [f32; 4], z: f32) {
        self.primitives.draw_text_simple(x, y, h, text, color, z);
    }
}
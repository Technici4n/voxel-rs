/// The state of a button.
#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Pressed,
    Released,
}

/// A mouse button.
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// A Ui event.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    /// A change in the state of a mouse button.
    MouseInput {
        state: ButtonState,
        button: MouseButton,
    },
}

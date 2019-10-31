#[derive(Debug, Clone, Copy)]
pub enum ButtonState {
    Pressed,
    Released,
}

pub type ScanCode = u32;

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    KeyboardInput { state: ButtonState, code: ScanCode },
    CharInput(char),
    CursorEntered,
    CursorExited,
    CursorMoved { x: f32, y: f32 },
    MouseInput { state: ButtonState, button: MouseButton },
}
/// Some size.
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub(crate) fn into_stretch(self) -> stretch::geometry::Size<stretch::number::Number> {
        stretch::geometry::Size {
            width: stretch::number::Number::Defined(self.width),
            height: stretch::number::Number::Defined(self.height),
        }
    }
}

/// Some position.
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

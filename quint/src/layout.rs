#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Layout {
    pub(crate) fn from_stretch(l: stretch::result::Layout) -> Self {
        Self {
            x: l.location.x,
            y: l.location.y,
            width: l.size.width,
            height: l.size.height,
        }
    }
}
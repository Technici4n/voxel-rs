/// The computed layout of a `Widget`.
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

    /// Check if a position is inside this `Layout`.
    pub fn is_position_inside(&self, p: crate::geometry::Position) -> bool {
        self.x <= p.x && p.x <= self.x + self.width && self.y <= p.y && p.y <= self.y + self.height
    }

    /// Pad this `Layout` by some logical pixels
    pub fn with_padding(&self, padding_pixels: f32) -> Self {
        Self {
            x: self.x + padding_pixels,
            y: self.y + padding_pixels,
            width: self.width - 2.0 * padding_pixels,
            height: self.height - 2.0 * padding_pixels,
        }
    }
}

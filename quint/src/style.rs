use stretch::style::*;

#[derive(Debug, Clone, Default)]
pub struct Style {
    pub(crate) style: stretch::style::Style,
}

/// Style of a `Widget`
impl Style {
    /// Set wrapping in the main direction
    pub fn wrap(mut self) -> Self {
        self.style.flex_wrap = FlexWrap::Wrap;
        self
    }
    /// Set width relative to parent in percent (from 0.0 to 1.0)
    pub fn percent_width(mut self, width: f32) -> Self {
        self.style.size.width = Dimension::Percent(width);
        self
    }
    /// Set height relative to parent in percent (from 0.0 to 1.0)
    pub fn percent_height(mut self, height: f32) -> Self {
        self.style.size.height = Dimension::Percent(height);
        self
    }
    /// Set size relative to parent in percent (from 0.0 to 1.0)
    pub fn percent_size(self, width: f32, height: f32) -> Self {
        self.percent_width(width).percent_height(height)
    }
}
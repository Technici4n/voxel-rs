use stretch::style::*;

#[derive(Debug, Clone)]
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
    /// Set direction
    pub fn vertical(mut self) -> Self {
        self.style.flex_direction = FlexDirection::Column;
        self
    }
    /// Center cross axis
    pub fn center_cross(mut self) -> Self {
        self.style.align_items = AlignItems::Center;
        self
    }
    /// Center main axis by spacing around
    pub fn center_main(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceAround;
        self
    }
    /// Center main axis by spacing between
    pub fn space_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceBetween;
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
    /// Set absolue width in logical pixels
    pub fn absolute_width(mut self, width: f32) -> Self {
        self.style.size.width = Dimension::Points(width);
        self
    }
    /// Set absolue height in logical pixels
    pub fn absolute_height(mut self, height: f32) -> Self {
        self.style.size.height = Dimension::Points(height);
        self
    }
    /// Set absolute size in logical pixels
    pub fn absolute_size(self, width: f32, height: f32) -> Self {
        self.absolute_width(width).absolute_height(height)
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            style: stretch::style::Style {
                ..stretch::style::Style::default()
            },
        }
    }
}

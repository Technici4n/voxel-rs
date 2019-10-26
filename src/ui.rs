use crate::settings::SETTINGS;
use crate::world::transform::Camera;
use anyhow::Result;
use conrod_core::widget_ids;
use std::path::Path;

pub mod renderer;

/// Wrapper around the ui
pub struct Ui {
    /// The ui state managed by conrod
    ui: conrod_core::Ui,
    /// The generated ids for the widgets
    ids: Ids,
}

impl Ui {
    /// Create a new ui
    pub fn new() -> Result<Self> {
        use conrod_core::position::Scalar;
        // Create Ui
        let (win_w, win_h) = SETTINGS.read().unwrap().window_size;
        let mut ui = conrod_core::UiBuilder::new([win_h as Scalar, win_w as Scalar])
            .theme(conrod_core::Theme::default())
            .build();
        let ids = Ids::new(ui.widget_id_generator());

        // Load font
        let assets_path = Path::new("assets");
        let font_path = assets_path.join("fonts/Ubuntu-R.ttf");
        ui.fonts.insert_from_file(font_path)?;

        Ok(Self { ui, ids })
    }

    /// Handle a glutin event
    pub fn handle_event(&mut self, event: glutin::Event, window: &winit::Window) {
        if let Some(event) = convert_event(event, &WindowRef(window)) {
            self.ui.handle_event(event);
        }
    }

    /// Rebuild the Ui if it changed
    pub fn build_if_changed(&mut self, camera: &Camera) {
        if self.ui.global_input().events().next().is_some() {
            let mut ui_cell = self.ui.set_widgets();
            gui(&mut ui_cell, &self.ids, camera);
        }
    }

    /// Redraw the Ui if it changed
    pub fn draw_if_changed(&mut self) -> Option<conrod_core::render::Primitives> {
        self.ui.draw_if_changed()
    }
}

// A wrapper around the winit window that allows us to implement the trait necessary for enabling
// the winit <-> conrod conversion functions.
struct WindowRef<'a>(&'a winit::Window);

// Implement the `WinitWindow` trait for `WindowRef` to allow for generating compatible conversion
// functions.
impl<'a> conrod_winit::WinitWindow for WindowRef<'a> {
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        winit::Window::get_inner_size(&self.0).map(Into::into)
    }
    fn hidpi_factor(&self) -> f32 {
        winit::Window::get_hidpi_factor(&self.0) as _
    }
}

conrod_winit::conversion_fns!();

widget_ids! {
    struct Ids {
        canvas,
        title,
        yaw_pitch,
    }
}

// Create the gui
fn gui(ui: &mut conrod_core::UiCell, ids: &Ids, camera: &Camera) {
    // TODO: use a real gui
    use conrod_core::color::{Color, TRANSPARENT, WHITE};
    use conrod_core::position::Positionable;
    use conrod_core::text::Justify;
    use conrod_core::widget::primitive::text::Style as TextStyle;
    use conrod_core::widget::{self, Widget};

    let canvas_style = widget::canvas::Style {
        color: Some(TRANSPARENT),
        ..widget::canvas::Style::default()
    };
    widget::Canvas::new()
        .scroll_kids_vertically()
        .with_style(canvas_style)
        .set(ids.canvas, ui);
    let title_style = TextStyle {
        font_size: None,
        color: Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        maybe_wrap: None,
        line_spacing: None,
        justify: Some(Justify::Center),
        font_id: None,
    };
    widget::Text::new("Welcome to voxel-rs")
        .with_style(title_style)
        .font_size(42)
        .mid_top_of(ids.canvas)
        .set(ids.title, ui);

    widget::Text::new(&format!("Yaw = {}\nPitch = {}", camera.yaw, camera.pitch))
        .with_style(TextStyle {
            color: Some(WHITE),
            ..TextStyle::default()
        })
        .font_size(30)
        .mid_bottom_of(ids.title)
        .set(ids.yaw_pitch, ui);
}

use anyhow::Result;
use conrod_core::widget_ids;
use gfx_core::Device;
use log::debug;
use std::path::Path;

/// Color format of the window's color buffer
type ColorFormat = gfx::format::Srgba8;
/// Format of the window's depth buffer
type DepthFormat = gfx::format::DepthStencil;

/// Wrapper around the game window
pub struct Window {
    /// Glutin's event loop
    events_loop: glutin::EventsLoop,
    /// A boolean indicating if the game should still be running
    pub running: bool,
    gfx: Gfx,
    ui: Ui,
}

/// A wrapper around the winit window that allows us to implement the trait necessary for enabling
/// the winit <-> conrod conversion functions.
struct WindowRef<'a>(&'a winit::Window);

/// Implement the `WinitWindow` trait for `WindowRef` to allow for generating compatible conversion
/// functions.
impl<'a> conrod_winit::WinitWindow for WindowRef<'a> {
    fn get_inner_size(&self) -> Option<(u32, u32)> {
        winit::Window::get_inner_size(&self.0).map(Into::into)
    }
    fn hidpi_factor(&self) -> f32 {
        winit::Window::get_hidpi_factor(&self.0) as _
    }
}

conrod_winit::conversion_fns!();

widget_ids!{
    pub struct Ids {
        canvas,
        title,
    }
}

impl Window {
    pub fn new() -> Result<Self> {
        // init window and opengl
        let events_loop = glutin::EventsLoop::new();
        let (context, device, mut factory, color_buffer, depth_buffer) = {
            let window_builder = glutin::WindowBuilder::new().with_title("voxel-rs".to_owned());
            let context_builder = glutin::ContextBuilder::new()
                .with_vsync(false)
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)));
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context_builder, &events_loop)?
        };
        let encoder = factory.create_command_buffer().into();
        // init ui;
        let mut ui = conrod_core::UiBuilder::new([512.0, 512.0])
            .theme(conrod_core::Theme::default())
            .build();
        let ids = Ids::new(ui.widget_id_generator());

        let assets_path = Path::new("assets");
        let font_path = assets_path.join("fonts/Ubuntu-R.ttf");
        ui.fonts.insert_from_file(font_path)?;

        Ok(Self {
            events_loop,
            running: true,
            gfx: Gfx {
                context,
                device,
                factory,
                encoder,
                color_buffer,
                depth_buffer,
            },
            ui: Ui {
                ui,
                ids,
            },
        })
    }

    pub fn process_events(&mut self) {
        let Self { ref mut events_loop, ref mut running, ref mut gfx, ref mut ui } = self;
        events_loop.poll_events(|event| {
            use glutin::Event::*;
            use glutin::WindowEvent::*;

            if let Some(event) = convert_event(event.clone(), &WindowRef(gfx.context.window())) {
                ui.ui.handle_event(event);
            }

            match event {
                WindowEvent { event: window_event, .. } => match window_event {
                    CloseRequested => {
                        *running = false;
                    },
                    Resized(logical_size) => {
                        let hidpi_factor = gfx.context.window().get_hidpi_factor();
                        let physical_size = logical_size.to_physical(hidpi_factor);
                        gfx.context.resize(physical_size);
                        let (new_color, new_depth) = gfx_window_glutin::new_views::<ColorFormat, DepthFormat>(&gfx.context);
                        gfx.color_buffer = new_color;
                        gfx.depth_buffer = new_depth;
                    }
                    _ => {},
                },
                _ => {},
            }
        });

        if ui.ui.global_input().events().next().is_some() {
            let mut ui_cell = ui.ui.set_widgets();
            gui(&mut ui_cell, &ui.ids);
        }
    }

    pub fn render(&mut self) -> Result<()> {
        let (win_w, win_h): (u32, u32) = match self.gfx.context.window().get_inner_size() {
            Some(s) => s.into(),
            None => {
                self.running = false;
                return Ok(());
            }
        };

        let dpi_factor = self.gfx.context.window().get_hidpi_factor() as f32;

        if let Some(primitives) = self.ui.ui.draw_if_changed() {
            debug!("Redrawing the Ui because it changed");
            let dims = (win_w as f32 * dpi_factor, win_h as f32 * dpi_factor);
            let Gfx { ref context, ref mut device, ref mut factory, ref mut encoder, ref color_buffer, .. } = &mut self.gfx;
            let mut ui_renderer = conrod_gfx::Renderer::new(factory, color_buffer, context.window().get_hidpi_factor())?;
            let image_map = conrod_core::image::Map::new();
            ui_renderer.clear(encoder, CLEAR_COLOR);
            ui_renderer.fill(encoder, dims, dpi_factor as f64, primitives, &image_map);
            ui_renderer.draw(factory, encoder, &image_map);
            encoder.flush(device);
            context.swap_buffers()?;
            device.cleanup();
        }

        Ok(())
    }
}

struct Gfx {
    pub context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub device: gfx_device_gl::Device,
    pub factory: gfx_device_gl::Factory,
    pub encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub color_buffer: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    pub depth_buffer: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, DepthFormat>,
}

struct Ui {
    pub ui: conrod_core::Ui,
    pub ids: Ids,
}

const CLEAR_COLOR: [f32; 4] = [1.0, 0.0, 0.0, 1.0];

fn gui(ui: &mut conrod_core::UiCell, ids: &Ids) {
    use conrod_core::color::Color;
    use conrod_core::position::Positionable;
    use conrod_core::text::Justify;
    use conrod_core::widget::{self, Widget};

    widget::Canvas::new().scroll_kids_vertically().set(ids.canvas, ui);
    let title_style = widget::primitive::text::Style {
        font_size: None,
        color: Some(Color::Rgba(1.0, 1.0, 1.0, 1.0)),
        maybe_wrap: None,
        line_spacing: None,
        justify: Some(Justify::Center),
        font_id: None,
    };
    widget::Text::new("Welcome to voxel-rs").with_style(title_style).font_size(42).mid_top_of(ids.canvas).set(ids.title, ui);
}
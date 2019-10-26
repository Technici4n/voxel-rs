use crate::{settings::SETTINGS, ui::{renderer::UiRenderer, Ui}, world::WorldRenderer};
use anyhow::Result;
use gfx_core::Device;
use log::debug;

/// Color format of the window's color buffer
pub type ColorFormat = gfx::format::Srgba8;
/// Format of the window's depth buffer
pub type DepthFormat = gfx::format::DepthStencil;

const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];
const CLEAR_DEPTH: f32 = 1.0;

/// Wrapper around the game window
pub struct Window {
    /// Glutin's event loop
    events_loop: glutin::EventsLoop,
    /// A boolean indicating if the game should still be running
    pub running: bool,
    /// Rendering-related data storage
    gfx: Gfx,
    /// User interface
    ui: Ui,
    /// User interface renderer
    ui_renderer: UiRenderer,
    /// World rendering
    world_renderer: WorldRenderer,
}

/// Useful information for renderers
#[derive(Clone, Copy)]
pub struct RenderInfo {
    pub window_dimensions: (u32, u32),
    pub dpi_factor: f64,
}

fn get_context_render_info(context: &glutin::WindowedContext<glutin::PossiblyCurrent>) -> Option<RenderInfo> {
    let window_dimensions: Option<(u32, u32)> = context.window().get_inner_size().map(Into::into);

    let dpi_factor = context.window().get_hidpi_factor();

    window_dimensions.map(|window_dimensions| {
        RenderInfo {
            window_dimensions,
            dpi_factor,
        }
    })
}

impl Window {
    /// Create a new game window
    pub fn new() -> Result<Self> {
        // Init window, OpenGL and gfx
        let events_loop = glutin::EventsLoop::new();
        let (context, device, mut factory, color_buffer, depth_buffer) = {
            let window_builder = glutin::WindowBuilder::new()
                .with_title("voxel-rs".to_owned())
                .with_dimensions(SETTINGS.read().unwrap().window_size.into());
            let context_builder = glutin::ContextBuilder::new()
                .with_vsync(false)
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)));
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(
                window_builder,
                context_builder,
                &events_loop,
            )?
        };
        let encoder = factory.create_command_buffer().into();

        // Init Ui
        let ui = Ui::new()?;

        let mut gfx = Gfx {
            context,
            device,
            factory,
            encoder,
            color_buffer,
            depth_buffer,
        };

        let render_info = get_context_render_info(&gfx.context).expect("Newly created OpenGL context has no size");

        let ui_renderer = UiRenderer::new(&mut gfx, &render_info)?;
        let world_renderer = WorldRenderer::new(&mut gfx)?;

        Ok(Self {
            events_loop,
            running: true,
            gfx,
            ui,
            ui_renderer,
            world_renderer,
        })
    }

    /// Get the render info from the underlying context.
    /// This function stops the application and returns `None` if the window was closed.
    pub fn get_render_info(&mut self) -> Option<RenderInfo> {
        if let Some(render_info) = get_context_render_info(&self.gfx.context) {
            Some(render_info)
        } else {
            self.running = false;
            None
        }
    }

    /// Process all incoming events
    pub fn process_events(&mut self) {
        let Self {
            ref mut events_loop,
            ref mut running,
            ref mut gfx,
            ref mut ui,
            ref mut world_renderer,
            ..
        } = self;
        events_loop.poll_events(|event| {
            use glutin::DeviceEvent::*;
            use glutin::Event::*;
            use glutin::WindowEvent::*;

            ui.handle_event(event.clone(), gfx.context.window());

            match event {
                WindowEvent {
                    event: window_event,
                    ..
                } => match window_event {
                    CloseRequested => {
                        *running = false;
                    }
                    Resized(logical_size) => {
                        let hidpi_factor = gfx.context.window().get_hidpi_factor();
                        let physical_size = logical_size.to_physical(hidpi_factor);
                        gfx.context.resize(physical_size);
                        let (new_color, new_depth) =
                            gfx_window_glutin::new_views::<ColorFormat, DepthFormat>(&gfx.context);
                        gfx.color_buffer = new_color;
                        gfx.depth_buffer = new_depth;
                    }
                    _ => {}
                },
                DeviceEvent {
                    event: device_event,
                    ..
                } => match device_event {
                    Motion { axis, value } => match axis {
                        0 => world_renderer.camera.update_cursor(value, 0.0),
                        1 => world_renderer.camera.update_cursor(0.0, value),
                        _ => panic!("Unknown axis. Expected 0 or 1, found {}.", axis),
                    },
                    _ => {}
                },
                _ => {}
            }
        });

        ui.build_if_changed(&world_renderer.camera);
    }

    /// Render the game
    pub fn render(&mut self) -> Result<()> {
        // Get the current window dimensions if they are available or stop the game otherwise.
        let render_info = match self.get_render_info() {
            Some(render_info) => render_info,
            None => return Ok(()),
        };

        // Clear buffers
        {
            let Gfx { ref mut encoder, ref color_buffer, ref depth_buffer, .. } = &mut self.gfx;
            encoder.clear(color_buffer, CLEAR_COLOR);
            encoder.clear_depth(depth_buffer, CLEAR_DEPTH);
        }

        // Draw World
        self.world_renderer.render(&mut self.gfx)?;
        // Clear depth buffer to draw Ui on top of the world
        {
            let Gfx { ref mut encoder, ref depth_buffer, .. } = &mut self.gfx;
            encoder.clear_depth(depth_buffer, CLEAR_DEPTH);
        }
        // Draw Ui
        self.ui_renderer.render(&mut self.gfx, render_info, &mut self.ui)?;

        // Flush and swap buffers
        {
            let Gfx {
                ref context,
                ref mut device,
                ref mut encoder,
                ..
            } = &mut self.gfx;

            encoder.flush(device);
            context.swap_buffers()?;
            device.cleanup();
        }

        Ok(())
    }
}

/// Store for all rendering-related data
pub struct Gfx {
    pub context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub device: gfx_device_gl::Device,
    pub factory: gfx_device_gl::Factory,
    pub encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub color_buffer: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    pub depth_buffer: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, DepthFormat>,
}

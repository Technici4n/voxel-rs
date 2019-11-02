use crate::{input::InputState, settings::Settings};
use anyhow::{Context, Result};
use glutin::{ElementState, MouseButton};
use log::info;
use std::time::Instant;

/// A closure that creates a new instance of `State`.
pub type StateFactory = Box<dyn FnOnce(&mut Settings, &mut Gfx) -> Result<Box<dyn State>>>;

/// A transition from one state to another.
pub enum StateTransition {
    /// Don't transition, keep the current state.
    KeepCurrent,
    /// Transition to another state using its `StateFactory`.
    ReplaceCurrent(StateFactory),
    /// Don't transition, close the current window.
    CloseWindow,
}

/// Read-only data that is provided to the states.
#[derive(Debug, Clone)]
pub struct WindowData {
    /// Logical size of the window. See [the glutin documentation](glutin::dpi).
    pub logical_window_size: glutin::dpi::LogicalSize,
    /// Physical size of the window.
    pub physical_window_size: glutin::dpi::PhysicalSize,
    /// HiDpi factor of the window.
    pub hidpi_factor: f64,
    /// `true` if the window is currently focused
    pub focused: bool,
}

/// Read-write data of the window that the states can modify.
#[derive(Debug, Clone)]
pub struct WindowFlags {
    /// `true` if the cursor should be hidden and centered.
    pub hide_and_center_cursor: bool,
    /// Window title
    pub window_title: String,
}

/// A window state. It has full control over the rendered content.
pub trait State {
    /// Update using the given time delta.
    fn update(
        &mut self,
        settings: &mut Settings,
        input_state: &InputState,
        data: &WindowData,
        flags: &mut WindowFlags,
        seconds_delta: f64,
    ) -> Result<StateTransition>;
    /// Render.
    ///
    /// Note: The state is responsible for swapping buffers.
    fn render(
        &mut self,
        settings: &Settings,
        gfx: &mut Gfx,
        data: &WindowData,
    ) -> Result<StateTransition>;
    /// Mouse motion
    fn handle_mouse_motion(&mut self, settings: &Settings, delta: (f64, f64));
    /// Cursor moved
    fn handle_cursor_movement(&mut self, logical_position: glutin::dpi::LogicalPosition);
    /// Mouse clicked
    fn handle_mouse_state_changes(&mut self, changes: Vec<(MouseButton, ElementState)>);
    /// Key pressed
    fn handle_key_state_changes(&mut self, changes: Vec<(u32, ElementState)>);
}

/// Color format of the window's color buffer
pub type ColorFormat = gfx::format::Srgba8;
/// Format of the window's depth buffer
pub type DepthFormat = gfx::format::DepthStencil;

/// An error that can happen when manipulating the window.
#[derive(Debug, Clone)]
pub enum WindowError {
    /// The window was closed unexpectedly.
    ClosedUnexpectedly,
    /// Setting the cursor position failed.
    CouldntSetCursorPosition(String),
}

/// Open a new window with the given settings and the given initial state
pub fn open_window(settings: &mut Settings, initial_state: StateFactory) -> Result<()> {
    info!("Opening new window...");
    // Initialize window and OpenGL context
    let window_title = "voxel-rs".to_owned();
    let mut events_loop = glutin::EventsLoop::new();
    let (context, device, mut factory, color_buffer, depth_buffer) = {
        let window_builder = glutin::WindowBuilder::new()
            .with_title(window_title.clone())
            .with_dimensions(settings.window_size.into());
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(false)
            .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)));
        gfx_window_glutin::init::<ColorFormat, DepthFormat>(
            window_builder,
            context_builder,
            &events_loop,
        )
        .context("Failed to initialize the window and the OpenGL context")?
    };
    let encoder = factory.create_command_buffer().into();

    let mut gfx = Gfx {
        context,
        device,
        factory,
        encoder,
        color_buffer,
        depth_buffer,
    };

    let mut window_data = {
        let logical_window_size = match gfx.context.window().get_inner_size() {
            Some(logical_window_size) => logical_window_size,
            None => return Err(WindowError::ClosedUnexpectedly)?,
        };
        let hidpi_factor = gfx.context.window().get_hidpi_factor();
        let physical_window_size = logical_window_size.to_physical(hidpi_factor);
        WindowData {
            logical_window_size,
            physical_window_size,
            hidpi_factor,
            focused: false,
        }
    };

    let mut input_state = InputState::new();

    let mut window_flags = WindowFlags {
        hide_and_center_cursor: false,
        window_title,
    };

    info!("Done initializing the window. Moving on to the first state...");

    let mut state =
        initial_state(settings, &mut gfx).context("Failed to create initial window state")?;

    let mut previous_time = std::time::Instant::now();

    loop {
        let mut keep_running = true;
        let mut window_resized = false;
        let mut mouse_state_changes = Vec::new();
        let mut key_state_changes = Vec::new();
        // Handle events
        events_loop.poll_events(|event| {
            use glutin::Event::*;
            match event {
                WindowEvent { event, .. } => {
                    use glutin::WindowEvent::*;
                    match event {
                        Resized(_) | HiDpiFactorChanged(_) => window_resized = true,
                        Moved(_) => (),
                        CloseRequested | Destroyed => keep_running = false,
                        DroppedFile(_) | HoveredFile(_) | HoveredFileCancelled => (),
                        ReceivedCharacter(_) => (),
                        Focused(focused) => {
                            window_data.focused = focused;
                            input_state.clear();
                        }
                        KeyboardInput { input, .. } => {
                            if input_state.process_keyboard_input(input) {
                                key_state_changes.push((input.scancode, input.state));
                            }
                        },
                        CursorMoved { position, .. } => state.handle_cursor_movement(position),
                        CursorEntered { .. }
                        | CursorLeft { .. }
                        | MouseWheel { .. } => (),
                        MouseInput { button, state: element_state, modifiers, .. } => {
                            if input_state.process_mouse_input(element_state, button, modifiers) {
                                mouse_state_changes.push((button, element_state));
                            }
                        },
                        // weird events
                        TouchpadPressure { .. } | AxisMotion { .. } | Touch(..) => (),
                        Refresh => (),
                    }
                }
                // There is no need to handle device events,
                // all relevant should already be received by the window.
                DeviceEvent { event, .. } => {
                    if !window_data.focused {
                        return;
                    }
                    use glutin::DeviceEvent::*;
                    match event {
                        MouseMotion { delta } => state.handle_mouse_motion(settings, delta),
                        _ => (),
                    }
                }
                Awakened | Suspended(_) => {
                    // TODO: implement this ?
                    unimplemented!("Awakening and suspending is not currently handled")
                }
            }
        });
        if !keep_running {
            return Ok(());
        }
        if window_resized {
            window_data.logical_window_size = match gfx.context.window().get_inner_size() {
                Some(logical_window_size) => logical_window_size,
                None => return Err(WindowError::ClosedUnexpectedly)?,
            };
            window_data.hidpi_factor = gfx.context.window().get_hidpi_factor();
            window_data.physical_window_size = window_data
                .logical_window_size
                .to_physical(window_data.hidpi_factor);
            gfx_window_glutin::update_views(
                &gfx.context,
                &mut gfx.color_buffer,
                &mut gfx.depth_buffer,
            );
        }

        // Update state
        state.handle_mouse_state_changes(mouse_state_changes);
        state.handle_key_state_changes(key_state_changes);
        let seconds_delta = {
            let current_time = Instant::now();
            let delta = current_time - previous_time;
            previous_time = current_time;
            delta.as_secs() as f64 + delta.subsec_nanos() as f64 / 1e9
        };
        let state_transition = state
            .update(
                settings,
                &input_state,
                &window_data,
                &mut window_flags,
                seconds_delta,
            )
            .context("Failed to `update` the current window state")?;

        // Update window flags
        gfx.context.window().set_title(&window_flags.window_title);
        if window_flags.hide_and_center_cursor && window_data.focused {
            gfx.context.window().hide_cursor(true);
            let sz = window_data.logical_window_size;
            gfx.context
                .window()
                .set_cursor_position(glutin::dpi::LogicalPosition {
                    x: sz.width / 2.0,
                    y: sz.height / 2.0,
                })
                .map_err(|why| WindowError::CouldntSetCursorPosition(why))?;
        } else {
            gfx.context.window().hide_cursor(false);
        }

        // Transition if necessary
        match state_transition {
            StateTransition::KeepCurrent => (),
            StateTransition::ReplaceCurrent(new_state) => {
                state =
                    new_state(settings, &mut gfx).context("Failed to create next window state")?;
                continue;
            }
            StateTransition::CloseWindow => {
                return Ok(());
            }
        }

        // Render frame
        match state
            .render(settings, &mut gfx, &window_data)
            .context("Failed to `render` the current window state")?
        {
            StateTransition::KeepCurrent => (),
            StateTransition::ReplaceCurrent(new_state) => {
                state =
                    new_state(settings, &mut gfx).context("Failed to create next window state")?;
            }
            StateTransition::CloseWindow => {
                return Ok(());
            }
        }
    }
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowError::ClosedUnexpectedly => write!(f, "The window was closed unexpectedly"),
            WindowError::CouldntSetCursorPosition(why) => {
                write!(f, "Couldn't set cursor position: {}", why)
            }
        }
    }
}

impl std::error::Error for WindowError {}

pub const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];
pub const CLEAR_DEPTH: f32 = 1.0;

/// Store for all rendering-related data
pub struct Gfx {
    pub context: glutin::WindowedContext<glutin::PossiblyCurrent>,
    pub device: gfx_device_gl::Device,
    pub factory: gfx_device_gl::Factory,
    pub encoder: gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    pub color_buffer: gfx_core::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    pub depth_buffer: gfx_core::handle::DepthStencilView<gfx_device_gl::Resources, DepthFormat>,
}

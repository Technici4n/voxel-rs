use crate::{input::InputState, settings::Settings};
use anyhow::Result;
use log::info;
use std::time::Instant;
use wgpu::Device;
use winit::window::Window;
use winit::event::{MouseButton, ElementState};
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event_loop::ControlFlow;

/// A closure that creates a new instance of `State`.
pub type StateFactory = Box<dyn FnOnce(&mut Settings, &mut Device) -> Result<Box<dyn State>> >;

/// A transition from one state to another.
pub enum StateTransition {
    /// Don't transition, keep the current state.
    KeepCurrent,
    /// Transition to another state using its `StateFactory`.
    #[allow(dead_code)] // TODO: remove when it will be used again
    ReplaceCurrent(StateFactory),
    /// Don't transition, close the current window.
    CloseWindow,
}

/// Read-only data that is provided to the states.
#[derive(Debug, Clone)]
pub struct WindowData {
    /// Logical size of the window. See [the winit documentation](winit::dpi).
    pub logical_window_size: LogicalSize,
    /// Physical size of the window.
    pub physical_window_size: PhysicalSize,
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
        device: &mut Device,
    ) -> Result<StateTransition>;
    /// Render.
    ///
    /// Note: The state is responsible for swapping buffers.
    fn render(
        &mut self,
        settings: &Settings,
        frame: &wgpu::SwapChainOutput,
        device: &mut Device,
        data: &WindowData,
        input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)>;
    /// Mouse motion
    fn handle_mouse_motion(&mut self, settings: &Settings, delta: (f64, f64));
    /// Cursor moved
    fn handle_cursor_movement(&mut self, logical_position: LogicalPosition);
    /// Mouse clicked
    fn handle_mouse_state_changes(&mut self, changes: Vec<(MouseButton, ElementState)>);
    /// Key pressed
    fn handle_key_state_changes(&mut self, changes: Vec<(u32, ElementState)>);
}

/// Color format of the window's color buffer
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
/// Format of the window's depth buffer
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// Open a new window with the given settings and the given initial state
pub fn open_window(mut settings: Settings, initial_state: StateFactory) -> ! {
    info!("Opening new window...");
    // Create the window
    let window_title = "voxel-rs".to_owned();
    let event_loop = winit::event_loop::EventLoop::new();
    let window = Window::new(&event_loop).expect("Failed to create window");
    window.set_title(&window_title);
    // Create the Surface, i.e. the render target of the program
    let hidpi_factor = window.hidpi_factor();
    let physical_window_size = window.inner_size().to_physical(hidpi_factor);
    info!("Creating the rendering surface");
    let surface = wgpu::Surface::create(&window);
    // Get the Device and the render Queue
    let adapter = wgpu::Adapter::request(
        &wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, // TODO: configure this?
            backends: wgpu::BackendBit::PRIMARY,
        },
    ).expect("Failed to create adapter");
    // TODO: device should be immutable
    let (mut device, mut queue) = adapter.request_device(&wgpu::DeviceDescriptor {
        extensions: wgpu::Extensions {
            anisotropic_filtering: false,
        },
        limits: wgpu::Limits::default(),
    });
    // Create the SwapChain
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: physical_window_size.width.round() as u32,
        height: physical_window_size.height.round() as u32,
        present_mode: wgpu::PresentMode::NoVsync,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);

    let mut window_data = {
        let logical_window_size = window.inner_size();
        let hidpi_factor = window.hidpi_factor();
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
        initial_state(&mut settings, &mut device).expect("Failed to create initial window state");

    let mut previous_time = std::time::Instant::now();

    let mut window_resized = false;
    let mut mouse_state_changes = Vec::new();
    let mut key_state_changes = Vec::new();

    // Main loop
    event_loop.run(move |event, _, control_flow| {
        use winit::event::Event::*;
        match event {
            /* NORMAL EVENT HANDLING */
            WindowEvent { event, .. } => {
                use winit::event::WindowEvent::*;
                match event {
                    Resized(_) | HiDpiFactorChanged(_) => window_resized = true,
                    Moved(_) => (),
                    CloseRequested | Destroyed => *control_flow = ControlFlow::Exit,
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
                    }
                    CursorMoved { position, .. } => state.handle_cursor_movement(position),
                    CursorEntered { .. } | CursorLeft { .. } | MouseWheel { .. } => (),
                    MouseInput {
                        button,
                        state: element_state,
                        modifiers,
                        ..
                    } => {
                        if input_state.process_mouse_input(element_state, button, modifiers) {
                            mouse_state_changes.push((button, element_state));
                        }
                    }
                    // weird events
                    TouchpadPressure { .. } | AxisMotion { .. } | Touch(..) => (),
                    ModifiersChanged { .. } | RedrawRequested => () // TODO: handle these
                }
            }
            DeviceEvent { event, .. } => {
                if !window_data.focused {
                    return;
                }
                use winit::event::DeviceEvent::*;
                match event {
                    MouseMotion { delta } => state.handle_mouse_motion(&settings, delta),
                    _ => (),
                }
            }
            /* MAIN LOOP TICK */
            EventsCleared => {
                // If the window was resized, update the SwapChain and the window data
                if window_resized {
                    info!("The window was resized, adjusting buffers...");
                    window_data.logical_window_size = window.inner_size();
                    window_data.hidpi_factor = window.hidpi_factor();
                    let phys = window_data
                        .logical_window_size
                        .to_physical(window_data.hidpi_factor);
                    window_data.physical_window_size = phys;
                    sc_desc.width = phys.width.round() as u32;
                    sc_desc.height = phys.height.round() as u32;
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                }
                window_resized = false;

                // Update state
                let (v1, v2) = (Vec::new(), Vec::new()); // TODO: clean up
                state.handle_mouse_state_changes(std::mem::replace(&mut mouse_state_changes, v1));
                state.handle_key_state_changes(std::mem::replace(&mut key_state_changes, v2));
                let seconds_delta = {
                    let current_time = Instant::now();
                    let delta = current_time - previous_time;
                    previous_time = current_time;
                    delta.as_secs() as f64 + delta.subsec_nanos() as f64 / 1e9
                };
                let state_transition = state
                    .update(
                        &mut settings,
                        &input_state,
                        &window_data,
                        &mut window_flags,
                        seconds_delta,
                        &mut device,
                    )
                    .expect("Failed to `update` the current window state"); // TODO: remove this

                // Update window flags
                window.set_title(&window_flags.window_title);
                if window_flags.hide_and_center_cursor && window_data.focused {
                    window.set_cursor_visible(false);
                    let sz = window_data.logical_window_size;
                    window
                        .set_cursor_position(winit::dpi::LogicalPosition {
                            x: sz.width / 2.0,
                            y: sz.height / 2.0,
                        })
                        .expect("Failed to center cursor"); // TODO: warn instead of panic ?
                } else {
                    window.set_cursor_visible(true);
                }

                // Transition if necessary
                match state_transition {
                    StateTransition::KeepCurrent => (),
                    StateTransition::ReplaceCurrent(new_state) => {
                        info!("Transitioning to a new window state...");
                        state =
                            new_state(&mut settings, &mut device).expect("Failed to create next window state");
                        return;
                    }
                    StateTransition::CloseWindow => {
                        *control_flow = ControlFlow::Exit;
                    }
                }

                // Render frame
                let frame = swap_chain.get_next_texture();
                let (state_transition, commands) =
                    state
                    .render(&settings, &frame, &mut device, &window_data, &input_state)
                    .expect("Failed to `render` the current window state");
                queue.submit(&[commands]);
                match state_transition {
                    StateTransition::KeepCurrent => (),
                    StateTransition::ReplaceCurrent(new_state) => {
                        state =
                            new_state(&mut settings, &mut device).expect("Failed to create next window state");
                    }
                    StateTransition::CloseWindow => {
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            LoopDestroyed => {
                // TODO: cleanup relevant stuff
            }
            _ => ()
        }
    });
}

pub const CLEAR_COLOR: [f32; 4] = [0.2, 0.2, 0.2, 1.0];
pub const CLEAR_DEPTH: f32 = 1.0;

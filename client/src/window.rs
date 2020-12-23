use crate::{input::InputState, settings::Settings};
use anyhow::Result;
use log::{info, warn};
use std::time::Instant;
use wgpu::Device;
use futures::executor::block_on;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton};
use winit::event_loop::ControlFlow;
use winit::window::Window;

/// A closure that creates a new instance of `State`.
pub type StateFactory =
    Box<dyn FnOnce(&mut Settings, &mut Device) -> Result<(Box<dyn State>, wgpu::CommandBuffer)>>;

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
    pub logical_window_size: LogicalSize<f64>,
    /// Physical size of the window.
    pub physical_window_size: PhysicalSize<u32>,
    /// HiDpi factor of the window.
    pub hidpi_factor: f64,
    /// `true` if the window is currently focused
    pub focused: bool,
}

/// Read-write data of the window that the states can modify.
#[derive(Debug, Clone)]
pub struct WindowFlags {
    /// `true` if the cursor should be hidden and centered.
    pub grab_cursor: bool,
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
    fn render<'a>(
        &mut self,
        settings: &Settings,
        buffers: WindowBuffers<'a>,
        device: &mut Device,
        data: &WindowData,
        input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)>;
    /// Mouse motion
    fn handle_mouse_motion(&mut self, settings: &Settings, delta: (f64, f64));
    /// Cursor moved
    fn handle_cursor_movement(&mut self, logical_position: LogicalPosition<f64>);
    /// Mouse clicked
    fn handle_mouse_state_changes(&mut self, changes: Vec<(MouseButton, ElementState)>);
    /// Key pressed
    fn handle_key_state_changes(&mut self, changes: Vec<(u32, ElementState)>);
}

/// Color format of the window's color buffer
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8Unorm;
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
    let hidpi_factor = window.scale_factor();
    let physical_window_size = window.inner_size();
    info!("Creating the swap chain");
    let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
    let surface = unsafe { instance.create_surface(&window) };
    // Get the Device and the render Queue
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance, // TODO: configure this?
        compatible_surface: Some(&surface),
    }))
    .expect("Failed to create adapter");
    // TODO: device should be immutable
    let (mut device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        features: wgpu::Features::empty(),
        limits: wgpu::Limits::default(),
        shader_validation: true
    }, None))
    .expect("Failed to request device");
    // Create the SwapChain
    let mut sc_desc = wgpu::SwapChainDescriptor {
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
        format: COLOR_FORMAT,
        width: physical_window_size.width,
        height: physical_window_size.height,
        present_mode: wgpu::PresentMode::Mailbox,
    };
    let mut swap_chain = device.create_swap_chain(&surface, &sc_desc);
    info!("Creating the multisampled texture buffer");
    let texture_view_descriptor = wgpu::TextureViewDescriptor::default();
    let mut msaa_texture_descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: sc_desc.format,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    let mut msaa_texture = device.create_texture(&msaa_texture_descriptor);
    let mut msaa_texture_view = msaa_texture.create_view(&texture_view_descriptor);
    info!("Creating the depth buffer");
    let mut depth_texture_descriptor = wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth: 1,
        },
        mip_level_count: 1,
        sample_count: SAMPLE_COUNT,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
    };
    let mut depth_texture = device.create_texture(&depth_texture_descriptor);
    let mut depth_texture_view = depth_texture.create_view(&texture_view_descriptor);

    let mut window_data = {
        let physical_window_size = window.inner_size();
        let hidpi_factor = window.scale_factor();
        let logical_window_size = physical_window_size.to_logical(hidpi_factor);
        WindowData {
            logical_window_size,
            physical_window_size,
            hidpi_factor,
            focused: false,
        }
    };

    let mut input_state = InputState::new();

    let mut window_flags = WindowFlags {
        grab_cursor: false,
        window_title,
    };

    info!("Done initializing the window. Moving on to the first state...");

    let (mut state, cmd) =
        initial_state(&mut settings, &mut device).expect("Failed to create initial window state");
    queue.submit(vec![cmd]);

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
                    Resized(_) | ScaleFactorChanged { .. } => window_resized = true,
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
                    CursorMoved { position, .. } => state.handle_cursor_movement(position.to_logical(hidpi_factor)),
                    CursorEntered { .. } | CursorLeft { .. } | MouseWheel { .. } => (),
                    MouseInput {
                        button,
                        state: element_state,
                        ..
                    } => {
                        if input_state.process_mouse_input(element_state, button) {
                            mouse_state_changes.push((button, element_state));
                        }
                    }
                    // weird events
                    TouchpadPressure { .. } | AxisMotion { .. } | Touch(..) | ThemeChanged(_) => (),
                    ModifiersChanged(modifiers_state) => input_state.set_modifiers_state(modifiers_state),
                }
            },
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
            MainEventsCleared => {
                // If the window was resized, update the SwapChain and the window data
                if window_resized {
                    info!("The window was resized, adjusting buffers...");
                    // Update window data
                    window_data.physical_window_size = window.inner_size();
                    window_data.hidpi_factor = window.scale_factor();
                    window_data.logical_window_size = window_data.physical_window_size.to_logical(window_data.hidpi_factor);
                    // Update SwapChain
                    sc_desc.width = window_data.physical_window_size.width;
                    sc_desc.height = window_data.physical_window_size.height;
                    swap_chain = device.create_swap_chain(&surface, &sc_desc);
                    // TODO: remove copy/paste
                    // Update depth buffer
                    depth_texture_descriptor.size.width = sc_desc.width;
                    depth_texture_descriptor.size.height = sc_desc.height;
                    depth_texture = device.create_texture(&depth_texture_descriptor);
                    depth_texture_view = depth_texture.create_view(&texture_view_descriptor);
                    // Udate MSAA frame buffer
                    msaa_texture_descriptor.size.width = sc_desc.width;
                    msaa_texture_descriptor.size.height = sc_desc.height;
                    msaa_texture = device.create_texture(&msaa_texture_descriptor);
                    msaa_texture_view = msaa_texture.create_view(&texture_view_descriptor);
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
                if window_flags.grab_cursor && window_data.focused {
                    window.set_cursor_visible(false);
                    let PhysicalSize { width, height } = window_data.physical_window_size;
                    let center_pos = PhysicalPosition { x : width / 2, y : height / 2 };
                    match window.set_cursor_grab(true) {
                        Err(err) => warn!("Failed to grab cursor ({:?})", err),
                        _ => (),
                    }
                    match window.set_cursor_position(center_pos) {
                        Err(err) => warn!("Failed to center cursor ({:?})", err),
                        _ => (),
                    }
                } else {
                    window.set_cursor_visible(true);
                    match window.set_cursor_grab(false) {
                        Err(err) => warn!("Failed to ungrab cursor ({:?})", err),
                        _ => (),
                    }
                }

                // Transition if necessary
                match state_transition {
                    StateTransition::KeepCurrent => (),
                    StateTransition::ReplaceCurrent(new_state) => {
                        info!("Transitioning to a new window state...");
                        let (new_state, cmd) = new_state(&mut settings, &mut device)
                            .expect("Failed to create next window state");
                        state = new_state;
                        queue.submit(vec![cmd]);
                        return;
                    }
                    StateTransition::CloseWindow => {
                        *control_flow = ControlFlow::Exit;
                    }
                }

                // Render frame
                let swap_chain_output = swap_chain.get_current_frame().expect("Failed to unwrap swap chain output.");
                let (state_transition, commands) = state
                    .render(
                        &settings,
                        WindowBuffers {
                            texture_buffer: &swap_chain_output.output.view,
                            multisampled_texture_buffer: &msaa_texture_view,
                            depth_buffer: &depth_texture_view,
                        },
                        &mut device,
                        &window_data,
                        &input_state,
                    )
                    .expect("Failed to `render` the current window state");
                queue.submit(vec![commands]);
                match state_transition {
                    StateTransition::KeepCurrent => (),
                    StateTransition::ReplaceCurrent(new_state) => {
                        let (new_state, cmd) = new_state(&mut settings, &mut device)
                            .expect("Failed to create next window state");
                        state = new_state;
                        queue.submit(vec![cmd]);
                    }
                    StateTransition::CloseWindow => {
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }
            RedrawRequested(_) => (), // TODO: handle this
            LoopDestroyed => {
                // TODO: cleanup relevant stuff
            }
            _ => (),
        }
    });
}

pub const CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.2,
    g: 0.2,
    b: 0.2,
    a: 1.0,
};
pub const CLEAR_DEPTH: f32 = 1.0;
pub const SAMPLE_COUNT: u32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct WindowBuffers<'a> {
    pub texture_buffer: &'a wgpu::TextureView,
    pub multisampled_texture_buffer: &'a wgpu::TextureView,
    pub depth_buffer: &'a wgpu::TextureView,
}

use anyhow::Result;
use crate::{
    input::KeyboardState,
    settings::Settings,
    ui::{Ui, renderer::UiRenderer},
    window::{Gfx, State, StateTransition, WindowData, WindowFlags},
    world::{World, renderer::WorldRenderer},
};
use gfx::Device;

pub struct SinglePlayer {
    ui: Ui,
    ui_renderer: UiRenderer,
    world: World,
    world_renderer: WorldRenderer,
}

impl SinglePlayer {
    pub fn new(_settings: &mut Settings, gfx: &mut Gfx) -> Result<Box<dyn State>> {
        Ok(Box::new(Self {
            ui: Ui::new(),
            ui_renderer: UiRenderer::new(gfx)?,
            world: World::new(),
            world_renderer: WorldRenderer::new(gfx)?,
        }))
    }
}

impl State for SinglePlayer {
    fn update(&mut self, _settings: &mut Settings, keyboard_state: &KeyboardState, _data: &WindowData, flags: &mut WindowFlags, seconds_delta: f64) -> Result<StateTransition> {
        self.world.camera.tick(seconds_delta, keyboard_state);
        self.ui.build_if_changed(&self.world)?;
        flags.hide_and_center_cursor = true;
        Ok(StateTransition::KeepCurrent)
    }

    fn render(&mut self, _settings: &Settings, gfx: &mut Gfx, data: &WindowData) -> Result<StateTransition> {
        // Clear buffers
        gfx.encoder.clear(&gfx.color_buffer, crate::window::CLEAR_COLOR);
        gfx.encoder.clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw world
        self.world_renderer.render(gfx, data, &self.world)?;
        // Clear depth
        gfx.encoder.clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw ui
        self.ui_renderer.render(gfx, &data, &mut self.ui)?;
        // Flush and swap buffers
        gfx.encoder.flush(&mut gfx.device);
        gfx.context.swap_buffers()?;
        gfx.device.cleanup();

        Ok(StateTransition::KeepCurrent)
    }

    fn handle_mouse_motion(&mut self, settings: &Settings, delta: (f64, f64)) {
        self.world.camera.update_cursor(delta.0, delta.1);
    }
}
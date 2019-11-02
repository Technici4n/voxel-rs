use crate::{
    fps::FpsCounter,
    input::KeyboardState,
    settings::Settings,
    ui::{renderer::UiRenderer, Ui},
    window::{Gfx, State, StateTransition, WindowData, WindowFlags},
    world::{renderer::WorldRenderer, World},
};
use anyhow::Result;
use gfx::Device;
use crate::world::camera::Camera;
use std::time::Instant;

/// State of a singleplayer world
pub struct SinglePlayer {
    fps_counter: FpsCounter,
    ui: Ui,
    ui_renderer: UiRenderer,
    world: World,
    world_renderer: WorldRenderer,
    camera: Camera,
}

impl SinglePlayer {
    pub fn new(_settings: &mut Settings, gfx: &mut Gfx) -> Result<Box<dyn State>> {
        // Generating the world
        let mut world = World::new();

        let t1 = Instant::now();
        println!("Generating the world ...");
        for i in -4..4 {
            for j in -4..4 {
                for k in -4..4 {
                    // generating the chunks
                    world.gen_chunk(i,j,k);
                }
            }
        }
        let t2 = Instant::now();
        println!("Generating the world : {} ms", (t2 -t1).subsec_millis());

        let mut world_renderer = WorldRenderer::new(gfx, &world);

        Ok(Box::new(Self {
            fps_counter: FpsCounter::new(),
            ui: Ui::new(),
            ui_renderer: UiRenderer::new(gfx)?,
            world,
            world_renderer : world_renderer?,
            camera: Camera::new(),
        }))
    }
}

impl State for SinglePlayer {
    fn update(
        &mut self,
        _settings: &mut Settings,
        keyboard_state: &KeyboardState,
        _data: &WindowData,
        _flags: &mut WindowFlags,
        seconds_delta: f64,
    ) -> Result<StateTransition> {
        self.camera.tick(seconds_delta, keyboard_state);
        //flags.hide_and_center_cursor = true;
        Ok(StateTransition::KeepCurrent)
    }

    fn render(
        &mut self,
        _settings: &Settings,
        gfx: &mut Gfx,
        data: &WindowData,
    ) -> Result<StateTransition> {
        // Count fps
        self.fps_counter.add_frame();

        // Clear buffers
        gfx.encoder
            .clear(&gfx.color_buffer, crate::window::CLEAR_COLOR);
        gfx.encoder
            .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw world
        self.world_renderer.render(gfx, data, &self.camera)?;
        // Clear depth
        gfx.encoder
            .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw ui
        self.ui.rebuild(&self.camera, self.fps_counter.fps(), data)?;
        self.ui_renderer.render(gfx, &data, &mut self.ui)?;
        // Flush and swap buffers
        gfx.encoder.flush(&mut gfx.device);
        gfx.context.swap_buffers()?;
        gfx.device.cleanup();

        Ok(StateTransition::KeepCurrent)
    }

    fn handle_mouse_motion(&mut self, _settings: &Settings, delta: (f64, f64)) {
        self.camera.update_cursor(delta.0, delta.1);
    }

    fn handle_cursor_movement(&mut self, logical_position: glutin::dpi::LogicalPosition) {
        self.ui.cursor_moved(logical_position);
    }
}
use std::time::Instant;

use anyhow::Result;
use gfx::Device;
use log::info;
use nalgebra::Vector3;

use crate::world::camera::Camera;
use crate::{
    block::Block,
    fps::FpsCounter,
    input::InputState,
    mesh::Mesh,
    physics::aabb::AABB,
    registry::Registry,
    settings::Settings,
    ui::{renderer::UiRenderer, Ui},
    window::{Gfx, State, StateTransition, WindowData, WindowFlags},
    world::{
        chunk::CHUNK_SIZE, meshing::greedy_meshing as meshing, renderer::WorldRenderer, World,
    },
};

/// State of a singleplayer world
pub struct SinglePlayer {
    fps_counter: FpsCounter,
    ui: Ui,
    ui_renderer: UiRenderer,
    world: World,
    world_renderer: WorldRenderer,
    camera: Camera,
    player: AABB,
    block_registry: Registry<Block>,
}

impl SinglePlayer {
    pub fn new(_settings: &mut Settings, gfx: &mut Gfx) -> Result<Box<dyn State>> {
        // Load data
        let data = crate::data::load_data(&mut gfx.factory, "data/".into())?;

        // Generating the world
        let mut world = World::new();

        let t1 = Instant::now();
        for i in -6..6 {
            for j in -1..3 {
                for k in -6..6 {
                    // generating the chunks
                    world.gen_chunk(i, j, k, &data.blocks);
                }
            }
        }

        let t2 = Instant::now();
        info!(
            "Generating the first part of the world took {} ms",
            (t2 - t1).subsec_millis()
        );

        let world_renderer = WorldRenderer::new(gfx, &world, data.meshes, data.texture_atlas);

        Ok(Box::new(Self {
            fps_counter: FpsCounter::new(),
            ui: Ui::new(),
            ui_renderer: UiRenderer::new(gfx)?,
            world,
            world_renderer: world_renderer?,
            camera: {
                let mut cam = Camera::new();
                cam.position = Vector3::new(0.4, 1.6, 0.4);
                cam
            },
            player: AABB::new(Vector3::new(0.0, 0.0, 0.0), (0.8, 1.8, 0.8)),
            block_registry: data.blocks,
        }))
    }
}

impl State for SinglePlayer {
    fn update(
        &mut self,
        _settings: &mut Settings,
        keyboard_state: &InputState,
        _data: &WindowData,
        flags: &mut WindowFlags,
        seconds_delta: f64,
        gfx: &mut Gfx,
    ) -> Result<StateTransition> {
        if self.ui.should_update_camera() {
            let delta_move = self.camera.get_movement(seconds_delta, keyboard_state);
            let new_delta = self.player.move_check_collision(&self.world, delta_move);

            self.camera.position += new_delta;

            // Generating new chunks
            let ix = self.player.pos.x.floor() as i64;
            let iy = self.player.pos.y.floor() as i64;
            let iz = self.player.pos.z.floor() as i64;

            let (cx, cy, cz) = World::get_chunk_coord(ix, iy, iz);

            for i in -1..=1 {
                for j in -1..=1 {
                    for k in -1..=1 {
                        if !self.world.has_chunk(cx + i, cy + j, cz + k) {
                            self.world
                                .gen_chunk(cx + i, cy + j, cz + k, &self.block_registry);
                        }
                    }
                }
            }

            // Unloading the chunk too far
            let mut chunks_to_dispose = Vec::new();
            for chunk in self.world.chunks.values() {
                let dx = self.player.pos.x - (chunk.pos.px as f64 * CHUNK_SIZE as f64);
                let dy = self.player.pos.y - (chunk.pos.py as f64 * CHUNK_SIZE as f64);
                let dz = self.player.pos.z - (chunk.pos.pz as f64 * CHUNK_SIZE as f64);
                if (dx * dx + dy * dy + dz * dz).sqrt() > 500.0 {
                    chunks_to_dispose.push(chunk.pos.clone());
                }
            }

            for pos in chunks_to_dispose {
                self.world.drop_chunk(pos.px, pos.py, pos.pz);
                self.world_renderer.drop_mesh(&pos);
            }

            // Update meshing
            let chunks_to_remesh = self.world.chunks_to_remesh.clone();
            self.world.chunks_to_remesh.clear();

            for chunk_pos in chunks_to_remesh.iter() {
                let chunk = self
                    .world
                    .get_chunk(chunk_pos.px, chunk_pos.py, chunk_pos.pz);
                match chunk {
                    None => (),
                    Some(chunk) => {
                        let Gfx {
                            ref mut factory, ..
                        } = gfx;

                        let (vertices, indices, _, _) = meshing(
                            chunk,
                            Some(self.world.create_adj_chunk_occl(
                                chunk.pos.px,
                                chunk.pos.py,
                                chunk.pos.pz,
                            )),
                            &self.world_renderer.block_meshes,
                        );

                        let pos = (
                            (chunk.pos.px * CHUNK_SIZE as i64) as f32,
                            (chunk.pos.py * CHUNK_SIZE as i64) as f32,
                            (chunk.pos.pz * CHUNK_SIZE as i64) as f32,
                        );
                        let chunk_mesh = Mesh::new(pos, vertices, indices, factory);
                        self.world_renderer.update_chunk_mesh(chunk.pos, chunk_mesh);
                    }
                }
            }
        }

        flags.hide_and_center_cursor = self.ui.should_capture_mouse();

        if self.ui.should_exit() {
            Ok(StateTransition::ReplaceCurrent(Box::new(crate::mainmenu::MainMenu::new)))
        } else {
            Ok(StateTransition::KeepCurrent)
        }
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
        self.ui
            .rebuild(&self.camera, self.fps_counter.fps(), data)?;
        self.ui_renderer.render(gfx, &data, &self.ui.ui)?;
        // Flush and swap buffers
        gfx.encoder.flush(&mut gfx.device);
        gfx.context.swap_buffers()?;
        gfx.device.cleanup();

        Ok(StateTransition::KeepCurrent)
    }

    fn handle_mouse_motion(&mut self, _settings: &Settings, delta: (f64, f64)) {
        if self.ui.should_update_camera() {
            self.camera.update_cursor(delta.0, delta.1);
        }
    }

    fn handle_cursor_movement(&mut self, logical_position: glutin::dpi::LogicalPosition) {
        self.ui.cursor_moved(logical_position);
    }

    fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(glutin::MouseButton, glutin::ElementState)>,
    ) {
        self.ui.handle_mouse_state_changes(changes);
    }

    fn handle_key_state_changes(&mut self, changes: Vec<(u32, glutin::ElementState)>) {
        self.ui.handle_key_state_changes(changes);
    }
}

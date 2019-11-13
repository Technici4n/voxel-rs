use anyhow::Result;
use gfx::Device;
use log::info;

use voxel_rs_common::{
    block::Block,
    network::{messages::ToClient, messages::ToServer, Client, ClientEvent},
    player::RenderDistance,
    registry::Registry,
    world::{chunk::CHUNK_SIZE, BlockPos, World},
};

use crate::input::YawPitch;
use crate::{
    fps::FpsCounter,
    input::InputState,
    mesh::Mesh,
    settings::Settings,
    ui::{renderer::UiRenderer, Ui},
    window::{Gfx, State, StateTransition, WindowData, WindowFlags},
    world::{frustum::Frustum, meshing::AdjChunkOccl, renderer::WorldRenderer},
};
use nalgebra::Vector3;
use std::collections::HashSet;
use std::time::Instant;
use voxel_rs_common::debug::{send_debug_info, DebugInfo};
use voxel_rs_common::physics::simulation::{ClientPhysicsSimulation, PhysicsState, ServerState};
use crate::world::meshing::AdjChunkLight;

/// State of a singleplayer world
pub struct SinglePlayer {
    fps_counter: FpsCounter,
    ui: Ui,
    ui_renderer: UiRenderer,
    world: World,
    world_renderer: WorldRenderer,
    #[allow(dead_code)] // TODO: remove this
    block_registry: Registry<Block>,
    client: Box<dyn Client>,
    render_distance: RenderDistance, // TODO: put this in the settigs
    physics_simulation: ClientPhysicsSimulation,
    yaw_pitch: YawPitch,
    debug_info: DebugInfo,
}

impl SinglePlayer {
    pub fn new_factory(client: Box<dyn Client>) -> crate::window::StateFactory {
        Box::new(move |settings, gfx| Self::new(settings, gfx, client))
    }

    pub fn new(
        _settings: &mut Settings,
        gfx: &mut Gfx,
        mut client: Box<dyn Client>,
    ) -> Result<Box<dyn State>> {
        info!("Launching singleplayer");
        // Wait for data and player_id from the server
        let (data, player_id) = {
            let mut data = None;
            let mut player_id = None;
            loop {
                if data.is_some() && player_id.is_some() {
                    break (data.unwrap(), player_id.unwrap());
                }
                match client.receive_event() {
                    ClientEvent::ServerMessage(ToClient::GameData(game_data)) => {
                        data = Some(game_data)
                    }
                    ClientEvent::ServerMessage(ToClient::CurrentId(id)) => player_id = Some(id),
                    _ => (),
                }
            }
        };
        info!("Received game data from the server");

        // Set render distance
        let render_distance = RenderDistance {
            x_max: 6,
            x_min: 6,
            y_max: 3,
            y_min: 3,
            z_max: 6,
            z_min: 6,
        };
        client.send(ToServer::SetRenderDistance(render_distance));

        // Load texture atlas
        let texture_atlas = crate::texture::load_image(&mut gfx.factory, data.texture_atlas)?;

        let world_renderer = WorldRenderer::new(gfx, data.meshes, texture_atlas);

        Ok(Box::new(Self {
            fps_counter: FpsCounter::new(),
            ui: Ui::new(),
            ui_renderer: UiRenderer::new(gfx)?,
            world: World::new(),
            world_renderer: world_renderer?,
            block_registry: data.blocks,
            client,
            render_distance,
            physics_simulation: ClientPhysicsSimulation::new(
                ServerState {
                    physics_state: PhysicsState::default(),
                    server_time: Instant::now(),
                    input: Default::default(),
                },
                player_id,
            ),
            yaw_pitch: Default::default(),
            debug_info: DebugInfo::new_current(),
        }))
    }
}

impl State for SinglePlayer {
    fn update(
        &mut self,
        _settings: &mut Settings,
        input_state: &InputState,
        _data: &WindowData,
        flags: &mut WindowFlags,
        _seconds_delta: f64,
        gfx: &mut Gfx,
    ) -> Result<StateTransition> {
        let mut chunk_updates = HashSet::new();
        // Handle server messages
        loop {
            match self.client.receive_event() {
                ClientEvent::NoEvent => break,
                ClientEvent::ServerMessage(message) => match message {
                    ToClient::Chunk(chunk, light_chunk) => {
                        // TODO: make sure this only happens once
                        self.world.set_chunk(chunk.to_chunk());
                        self.world.set_light_chunk(light_chunk.to_chunk());
                        // Queue chunks for meshing
                        for i in -1..=1 {
                            for j in -1..=1 {
                                for k in -1..=1 {
                                    chunk_updates.insert(chunk.pos.offset(i, j, k));
                                }
                            }
                        }
                    }
                    ToClient::UpdatePhysics(server_state) => {
                        self.physics_simulation.receive_server_update(server_state);
                    }
                    ToClient::GameData(_) => {}
                    ToClient::CurrentId(_) => {}
                },
                ClientEvent::Disconnected => unimplemented!("server disconnected"),
                ClientEvent::Connected => {}
            }
        }

        // Collect input
        let frame_input =
            input_state.get_physics_input(self.yaw_pitch, self.ui.should_update_camera());
        // Send input to server
        self.client.send(ToServer::UpdateInput(frame_input));
        // Update physics
        self.physics_simulation
            .step_simulation(frame_input, Instant::now(), &self.world);

        let p = self.physics_simulation.get_camera_position();
        let player_chunk = BlockPos::from(p).containing_chunk_pos();

        // Debug current player position, yaw and pitch
        send_debug_info(
            "Player",
            "position",
            format!(
                "x = {:.2}\ny = {:.2}\nz = {:.2}\nchunk x = {}\nchunk y={}\nchunk z = {}",
                p[0], p[1], p[2], player_chunk.px, player_chunk.py, player_chunk.pz
            ),
        );
        send_debug_info(
            "Player",
            "yawpitch",
            format!(
                "yaw = {:.0}\npitch = {:.0}",
                self.yaw_pitch.yaw, self.yaw_pitch.pitch
            ),
        );

        // Remove chunks that are too far
        // damned borrow checker :(
        let Self {
            ref mut world,
            ref mut world_renderer,
            ref render_distance,
            ..
        } = self;
        world.chunks.retain(|chunk_pos, _| {
            if render_distance.is_chunk_visible(p, *chunk_pos) {
                true
            } else {
                world_renderer.chunk_meshes.remove(chunk_pos);
                world_renderer.meshing_worker.dequeue_chunk(*chunk_pos);
                false
            }
        });

        // Update meshing
        // TODO: put this in the renderer ?
        let mut chunk_updates: Vec<_> = chunk_updates.into_iter().collect();
        // Sort the chunks so that the nearest ones are meshed first
        chunk_updates.sort_unstable_by_key(|pos| pos.squared_euclidian_distance(player_chunk));
        for chunk_pos in chunk_updates.into_iter() {
            self.world.get_add_light_chunk(chunk_pos);
            if let Some(chunk) = self.world.get_chunk(chunk_pos) {
                self.world_renderer.meshing_worker.enqueue_chunk(
                    chunk.clone(),
                    self.world.get_light_chunk(chunk_pos).cloned().unwrap(),
                    AdjChunkOccl::create_from_world(
                        &self.world,
                        chunk_pos,
                        &self.world_renderer.block_meshes,
                    ),
                    AdjChunkLight::create_from_world(
                        &self.world,
                        chunk_pos,
                    )
                );
            }
        }

        // Send new chunks to the GPU
        for (chunk_pos, vertices, indices) in self
            .world_renderer
            .meshing_worker
            .get_processed_chunks()
            .into_iter()
        {
            // Add the mesh if the chunk is still loaded
            if self.world.has_chunk(chunk_pos) {
                let world_pos = (
                    (chunk_pos.px * CHUNK_SIZE as i64) as f32,
                    (chunk_pos.py * CHUNK_SIZE as i64) as f32,
                    (chunk_pos.pz * CHUNK_SIZE as i64) as f32,
                );
                // TODO: reuse existing meshes when possible if that bottlenecks
                let chunk_mesh = Mesh::new(world_pos, vertices, indices, &mut gfx.factory);
                self.world_renderer.update_chunk_mesh(chunk_pos, chunk_mesh);
            }
        }

        flags.hide_and_center_cursor = self.ui.should_capture_mouse();

        if self.ui.should_exit() {
            //Ok(StateTransition::ReplaceCurrent(Box::new(crate::mainmenu::MainMenu::new)))
            Ok(StateTransition::CloseWindow)
        } else {
            Ok(StateTransition::KeepCurrent)
        }
    }

    fn render(
        &mut self,
        _settings: &Settings,
        gfx: &mut Gfx,
        data: &WindowData,
        input_state: &InputState,
    ) -> Result<StateTransition> {
        // Count fps
        self.fps_counter.add_frame();
        send_debug_info("Player", "fps", format!("fps = {}", self.fps_counter.fps()));

        let frustum = Frustum::new(
            self.physics_simulation.get_camera_position(),
            self.yaw_pitch,
        );

        // Try raytracing
        let pp = self.physics_simulation.get_player();
        let pointed_block = {
            let y = self.yaw_pitch.yaw.to_radians();
            let p = self.yaw_pitch.pitch.to_radians();
            let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
            pp.get_pointed_at(dir, 10.0, &self.world)
        };
        if let Some((x, face)) = pointed_block {
            send_debug_info(
                "Player",
                "pointedat",
                format!(
                    "Pointed block: Some({}, {}, {}), face: {}",
                    x.px, x.py, x.pz, face
                ),
            );
        } else {
            send_debug_info("Player", "pointedat", "Pointed block: None");
        }

        // Clear buffers
        gfx.encoder
            .clear(&gfx.color_buffer, crate::window::CLEAR_COLOR);
        gfx.encoder
            .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw world
        self.world_renderer.render(
            gfx,
            data,
            &frustum,
            input_state.enable_culling,
            pointed_block,
        )?;
        // Clear depth
        gfx.encoder
            .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw ui
        self.ui.rebuild(&mut self.debug_info, data)?;
        self.ui_renderer
            .render(gfx, &data, &self.ui.ui, self.ui.should_capture_mouse())?;
        // Flush and swap buffers
        gfx.encoder.flush(&mut gfx.device);
        gfx.context.swap_buffers()?;
        gfx.device.cleanup();

        Ok(StateTransition::KeepCurrent)
    }

    fn handle_mouse_motion(&mut self, _settings: &Settings, delta: (f64, f64)) {
        if self.ui.should_update_camera() {
            self.yaw_pitch.update_cursor(delta.0, delta.1);
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

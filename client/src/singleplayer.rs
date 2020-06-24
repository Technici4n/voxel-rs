use anyhow::Result;
use log::info;

use voxel_rs_common::{
    block::Block,
    network::{messages::ToClient, messages::ToServer, Client, ClientEvent},
    player::RenderDistance,
    registry::Registry,
    world::BlockPos,
};

use crate::input::YawPitch;
//use crate::model::model::Model;
//use crate::world::meshing::ChunkMeshData;
use crate::render::{Frustum, UiRenderer, WorldRenderer};
use crate::window::WindowBuffers;
use crate::{
    fps::FpsCounter,
    input::InputState,
    settings::Settings,
    ui::Ui,
    window::{State, StateTransition, WindowData, WindowFlags},
    world::World,
};
use nalgebra::Vector3;
use std::time::Instant;
use voxel_rs_common::data::vox::VoxelModel;
use voxel_rs_common::debug::{send_debug_info, send_perf_breakdown, DebugInfo};
use voxel_rs_common::item::{Item, ItemMesh};
use voxel_rs_common::physics::simulation::{ClientPhysicsSimulation, PhysicsState, ServerState};
use voxel_rs_common::time::BreakdownCounter;
use winit::event::{ElementState, MouseButton};
use crate::gui::Gui;

/// State of a singleplayer world
pub struct SinglePlayer {
    fps_counter: FpsCounter,
    ui: Ui,
    ui_renderer: UiRenderer,
    gui: Gui,
    world: World,
    #[allow(dead_code)] // TODO: remove this
    block_registry: Registry<Block>,
    item_registry: Registry<Item>,
    item_meshes: Vec<ItemMesh>,
    model_registry: Registry<VoxelModel>,
    client: Box<dyn Client>,
    render_distance: RenderDistance,
    // TODO: put this in the settigs
    physics_simulation: ClientPhysicsSimulation,
    yaw_pitch: YawPitch,
    debug_info: DebugInfo,
    start_time: Instant,
    client_timing: BreakdownCounter,
}

impl SinglePlayer {
    pub fn new_factory(client: Box<dyn Client>) -> crate::window::StateFactory {
        Box::new(move |settings, device| Self::new(settings, device, client))
    }

    pub fn new(
        settings: &mut Settings,
        device: &mut wgpu::Device,
        mut client: Box<dyn Client>,
    ) -> Result<(Box<dyn State>, wgpu::CommandBuffer)> {
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
        let (x1, x2, y1, y2, z1, z2) = settings.render_distance;
        let render_distance = RenderDistance {
            x_max: x1,
            x_min: x2,
            y_max: y1,
            y_min: y2,
            z_max: z1,
            z_min: z2,
        };
        client.send(ToServer::SetRenderDistance(render_distance));
        // Create the renderers
        let ui_renderer = UiRenderer::new(device);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        let world_renderer = WorldRenderer::new(
            device,
            &mut encoder,
            data.texture_atlas,
            &data.models,
        );

        Ok((
            Box::new(Self {
                fps_counter: FpsCounter::new(),
                ui: Ui::new(),
                ui_renderer,
                gui: Gui::new(),
                world: World::new(data.meshes.clone(), world_renderer),
                block_registry: data.blocks,
                model_registry: data.models,
                item_registry: data.items,
                item_meshes: data.item_meshes,
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
                start_time: Instant::now(),
                client_timing: BreakdownCounter::new(),
            }),
            encoder.finish(),
        ))
    }

    fn handle_server_messages(&mut self) {
        loop {
            match self.client.receive_event() {
                ClientEvent::NoEvent => break,
                ClientEvent::ServerMessage(message) => match message {
                    ToClient::Chunk(chunk, light_chunk) => {
                        self.world.add_chunk(chunk, light_chunk);
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
        _device: &mut wgpu::Device,
    ) -> Result<StateTransition> {
        self.client_timing.start_frame();
        // Handle server messages
        self.handle_server_messages();
        self.client_timing.record_part("Network events");

        // Collect input
        let frame_input =
            input_state.get_physics_input(self.yaw_pitch, self.ui.should_update_camera());
        // Send input to server
        self.client.send(ToServer::UpdateInput(frame_input));
        self.client_timing.record_part("Collect and send input");

        // Update physics
        self.physics_simulation
            .step_simulation(frame_input, Instant::now(), &self.world);
        self.client_timing.record_part("Update physics");

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
        self.world.remove_far_chunks(player_chunk, &self.render_distance);
        self.client_timing.record_part("Drop far chunks");

        // Send chunks to meshing
        self.world.enqueue_chunks_for_meshing(player_chunk, &self.render_distance);
        self.client_timing.record_part("Send chunks to meshing");

        send_debug_info("Chunks", "clientloaded", format!("Client loaded {} chunks", self.world.num_loaded_chunks()));

        flags.grab_cursor = self.ui.should_capture_mouse();

        if self.ui.should_exit() {
            //Ok(StateTransition::ReplaceCurrent(Box::new(crate::mainmenu::MainMenu::new)))
            Ok(StateTransition::CloseWindow)
        } else {
            Ok(StateTransition::KeepCurrent)
        }
    }

    fn render<'a>(
        &mut self,
        _settings: &Settings,
        buffers: WindowBuffers<'a>,
        device: &mut wgpu::Device,
        data: &WindowData,
        input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)> {
        // Count fps TODO: move this to update
        self.fps_counter.add_frame();
        send_debug_info("Player", "fps", format!("fps = {}", self.fps_counter.fps()));

        let frustum = Frustum::new(
            self.physics_simulation.get_camera_position(),
            self.yaw_pitch,
        );

        // Try raytracing TODO: move this to update
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
        self.client_timing.record_part("Raytrace");

        // Begin rendering
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        crate::render::clear_color_and_depth(&mut encoder, buffers);

        let mut models_to_draw = Vec::new();
        models_to_draw.push(crate::render::Model {
            mesh_id: self
                .model_registry
                .get_id_by_name(&"knight".to_owned())
                .unwrap(),
            pos_x: 0.0,
            pos_y: 55.0,
            pos_z: 0.0,
            scale: 0.3,
            rot_offset: [0.0, 0.0, 0.0],
            rot_y: 0.0,
        });
        let item_rotation = (Instant::now() - self.start_time).as_secs_f32(); // TODO: use f64
        models_to_draw.push(crate::render::Model {
            mesh_id: self
                .model_registry
                .get_id_by_name(&"item:ingot_iron".to_owned())
                .unwrap(),
            pos_x: 30.0,
            pos_y: 55.0,
            pos_z: 30.0,
            scale: 1.0 / 32.0,
            rot_offset: [0.5, 0.5, 1.0 / 64.0],
            rot_y: item_rotation,
        });
        // Draw chunks
        self.world.render_chunks(
            device,
            &mut encoder,
            buffers,
            data,
            &frustum,
            input_state.enable_culling,
            pointed_block,
            &models_to_draw,
        );
        self.client_timing.record_part("Render chunks");

        crate::render::clear_depth(&mut encoder, buffers);

        // Draw ui
        self.ui.rebuild(&mut self.debug_info, data)?;
        self.gui.prepare();
        crate::gui::experiments::render_debug_info(&mut self.gui, &mut self.debug_info);
        self.gui.finish();
        self.ui_renderer.render(
            buffers,
            device,
            &mut encoder,
            &data,
            &self.ui.ui,
            &mut self.gui,
            self.ui.should_capture_mouse(),
        );
        self.client_timing.record_part("Render UI");

        send_perf_breakdown("Client performance", "mainloop", "Client main loop", self.client_timing.extract_part_averages());

        Ok((StateTransition::KeepCurrent, encoder.finish()))
    }

    fn handle_mouse_motion(&mut self, _settings: &Settings, delta: (f64, f64)) {
        if self.ui.should_update_camera() {
            self.yaw_pitch.update_cursor(delta.0, delta.1);
        }
    }

    fn handle_cursor_movement(&mut self, logical_position: winit::dpi::LogicalPosition<f64>) {
        self.ui.cursor_moved(logical_position);
        let (x, y) = logical_position.into();
        self.gui.update_mouse_position(x, y);
    }

    fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(winit::event::MouseButton, winit::event::ElementState)>,
    ) {
        for (button, state) in changes.iter() {
            let pp = self.physics_simulation.get_player();
            let y = self.yaw_pitch.yaw;
            let p = self.yaw_pitch.pitch;
            match *button {
                MouseButton::Left => match *state {
                    ElementState::Pressed => {
                        self.client.send(ToServer::BreakBlock(pp.aabb.pos, y, p));
                    }
                    _ => {}
                },
                MouseButton::Right => match *state {
                    ElementState::Pressed => {
                        self.client.send(ToServer::PlaceBlock(pp.aabb.pos, y, p));
                    }
                    _ => {}
                },
                MouseButton::Middle => match *state {
                    ElementState::Pressed => {
                        self.client.send(ToServer::SelectBlock(pp.aabb.pos, y, p));
                    }
                    _ => {}
                },
                _ => {}
            }
            match *button {
                MouseButton::Left => match *state {
                    ElementState::Pressed => {
                        self.gui.update_mouse_button(true);
                    }
                    ElementState::Released => {
                        self.gui.update_mouse_button(false);
                    }
                },
                _ => {}
            }
        }
        self.ui.handle_mouse_state_changes(changes);
    }

    fn handle_key_state_changes(&mut self, changes: Vec<(u32, winit::event::ElementState)>) {
        self.ui.handle_key_state_changes(changes);
    }
}

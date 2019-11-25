use anyhow::Result;
use log::info;

use voxel_rs_common::{
    block::Block,
    network::{messages::ToClient, messages::ToServer, Client, ClientEvent},
    player::RenderDistance,
    registry::Registry,
    world::{BlockPos, World},
};

use crate::input::YawPitch;
//use crate::model::model::Model;
//use crate::world::meshing::ChunkMeshData;
use crate::{
    fps::FpsCounter,
    input::InputState,
    settings::Settings,
    ui::{renderer::UiRenderer, Ui},
    window::{State, StateTransition, WindowData, WindowFlags},
    world::{frustum::Frustum, /*renderer::WorldRenderer*/},
};
use winit::event::{ElementState, MouseButton};
use nalgebra::Vector3;
use std::collections::HashSet;
use std::time::Instant;
use voxel_rs_common::data::vox::VoxelModel;
use voxel_rs_common::debug::{send_debug_info, DebugInfo};
use voxel_rs_common::physics::simulation::{ClientPhysicsSimulation, PhysicsState, ServerState};
use voxel_rs_common::world::chunk::ChunkPos;

/// State of a singleplayer world
pub struct SinglePlayer {
    fps_counter: FpsCounter,
    ui: Ui,
    ui_renderer: UiRenderer,
    world: World,
    //world_renderer: WorldRenderer,
    #[allow(dead_code)] // TODO: remove this
    block_registry: Registry<Block>,
    model_regitry: Registry<VoxelModel>,
    client: Box<dyn Client>,
    render_distance: RenderDistance,
    // TODO: put this in the settigs
    physics_simulation: ClientPhysicsSimulation,
    yaw_pitch: YawPitch,
    debug_info: DebugInfo,
    chunks_to_mesh: HashSet<ChunkPos>,
}

impl SinglePlayer {
    pub fn new_factory(client: Box<dyn Client>) -> crate::window::StateFactory {
        Box::new(move |settings, device| Self::new(settings, device, client))
    }

    pub fn new(
        settings: &mut Settings,
        device: &mut wgpu::Device,
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
        let ui_renderer = UiRenderer::new(device)?;

        // Load texture atlas
        //let texture_atlas = crate::texture::load_image(&mut gfx.factory, data.texture_atlas)?;

        //let world_renderer = WorldRenderer::new(gfx, data.meshes, texture_atlas, &data.models);

        Ok(Box::new(Self {
            fps_counter: FpsCounter::new(),
            ui: Ui::new(),
            ui_renderer,
            world: World::new(),
            //world_renderer: world_renderer?,
            block_registry: data.blocks,
            model_regitry: data.models,
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
            chunks_to_mesh: Default::default(),
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
        _device: &mut wgpu::Device,
    ) -> Result<StateTransition> {
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
                                    self.chunks_to_mesh.insert(chunk.pos.offset(i, j, k));
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
            //ref mut world_renderer,
            ref render_distance,
            ..
        } = self;
        let World {
            ref mut chunks,
            ref mut light,
            ..
        } = world;
        chunks.retain(|chunk_pos, _| {
            if render_distance.is_chunk_visible(p, *chunk_pos) {
                true
            } else {
                //world_renderer.chunk_meshes.remove(chunk_pos);
                //world_renderer.meshing_worker.dequeue_chunk(*chunk_pos);
                light.remove(chunk_pos);
                false
            }
        });

        // Update meshing (for roughly 10 milliseconds)
        // TODO: put this in the renderer ?
        let meshing_start = Instant::now();
        let mut chunk_updates: Vec<_> = self.chunks_to_mesh.iter().cloned().collect();
        // Sort the chunks so that the nearest ones are meshed first
        chunk_updates.sort_unstable_by_key(|pos| pos.squared_euclidian_distance(player_chunk));
        for chunk_pos in chunk_updates.into_iter() {
            if (Instant::now() - meshing_start).subsec_millis() > 10 {
                break;
            }
            // Only mesh the chunks if it needs updating
            self.chunks_to_mesh.remove(&chunk_pos);
            if self.world.has_chunk(chunk_pos) {
                assert_eq!(self.world.has_light_chunk(chunk_pos), true);
                //self.world_renderer
                //    .meshing_worker
                //    .enqueue_chunk(ChunkMeshData::create_from_world(&self.world, chunk_pos));
            }
        }

        // Send new chunks to the GPU
        /*for (chunk_pos, vertices, indices) in self
            .world_renderer
            .meshing_worker
            .get_processed_chunks()
            .into_iter()
        {
            // Add the mesh if the chunk is still loaded
            if self.world.has_chunk(chunk_pos) {
                self.world_renderer
                    .update_chunk_mesh(gfx, chunk_pos, vertices, indices);
            }
        }*/

        flags.hide_and_center_cursor = self.ui.should_capture_mouse();

        send_debug_info(
            "Chunks",
            "client",
            format!(
                "Client loaded chunks = {}\nClient loaded light chunks = {}",
                self.world.chunks.len(),
                self.world.light.len()
            ),
        );

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
        frame: &wgpu::SwapChainOutput,
        depth_buffer_view: &wgpu::TextureView,
        device: &mut wgpu::Device,
        data: &WindowData,
        _input_state: &InputState,
    ) -> Result<(StateTransition, wgpu::CommandBuffer)> {
        // Count fps TODO: move this to update
        self.fps_counter.add_frame();
        send_debug_info("Player", "fps", format!("fps = {}", self.fps_counter.fps()));

        let _frustum = Frustum::new(
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

        // Begin rendering
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { todo: 0 });

        // Clear color and depth buffers
        {
            // We want this render pass to access the color and depth buffers, so we have to attach them
            let rpass_descriptor = wgpu::RenderPassDescriptor {
                color_attachments: &[
                    // This is the color buffer attachment
                    wgpu::RenderPassColorAttachmentDescriptor {
                        // The color buffer
                        attachment: &frame.view,
                        // TODO: what is this?
                        resolve_target: None,
                        // We want to clear the color at the start of the pass
                        load_op: wgpu::LoadOp::Clear,
                        // We want to store the color at the end of the pass
                        store_op: wgpu::StoreOp::Store,
                        // The clear color
                        clear_color: wgpu::Color {
                            r: crate::window::CLEAR_COLOR[0] as f64,
                            g: crate::window::CLEAR_COLOR[1] as f64,
                            b: crate::window::CLEAR_COLOR[2] as f64,
                            a: crate::window::CLEAR_COLOR[3] as f64,
                        },
                    }
                ],
                // This is the depth buffer attachment
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachmentDescriptor {
                    // The depth buffer
                    attachment: &depth_buffer_view,
                    // We want to clear the depth buffer at the start of the pass
                    depth_load_op: wgpu::LoadOp::Clear,
                    // We want to store the depth at the end of the pass
                    depth_store_op: wgpu::StoreOp::Store,
                    // The clear depth
                    clear_depth: crate::window::CLEAR_DEPTH,
                    // We don't care about these because we don't use the stencil buffer
                    stencil_load_op: wgpu::LoadOp::Clear,
                    stencil_store_op: wgpu::StoreOp::Clear,
                    clear_stencil: 0,
                }),
            };
            // This starts the pass, renders nothing and ends the pass, thus clearing then storing the color and the depth
            let rpass = encoder.begin_render_pass(&rpass_descriptor);
        }
        // Draw world

        /*let mut model_to_draw = Vec::new();
        model_to_draw.push(Model {
            model_mesh_id: self
                .model_regitry
                .get_id_by_name(&"knight".to_owned())
                .unwrap(),
            pos_x: 0.0,
            pos_y: 55.0,
            pos_z: 0.0,
            scale: 0.3,
        });*/
        /*self.world_renderer.render(
            gfx,
            data,
            &frustum,
            input_state.enable_culling,
            pointed_block,
            model_to_draw,
        )?;*/
        // Clear depth
        //gfx.encoder
        //    .clear_depth(&gfx.depth_buffer, crate::window::CLEAR_DEPTH);
        // Draw ui
        self.ui.rebuild(&mut self.debug_info, data)?;
        self.ui_renderer
            .render(&frame.view, device, &mut encoder, &data, &self.ui.ui, self.ui.should_capture_mouse())?;

        Ok((StateTransition::KeepCurrent, encoder.finish()))
    }

    fn handle_mouse_motion(&mut self, _settings: &Settings, delta: (f64, f64)) {
        if self.ui.should_update_camera() {
            self.yaw_pitch.update_cursor(delta.0, delta.1);
        }
    }

    fn handle_cursor_movement(&mut self, logical_position: winit::dpi::LogicalPosition) {
        self.ui.cursor_moved(logical_position);
    }

    fn handle_mouse_state_changes(
        &mut self,
        changes: Vec<(winit::event::MouseButton, winit::event::ElementState)>,
    ) {
        for (button, state) in changes.iter() {
            match *button {
                MouseButton::Left => match *state {
                    ElementState::Pressed => {
                        let pp = self.physics_simulation.get_player();
                        let y = self.yaw_pitch.yaw;
                        let p = self.yaw_pitch.pitch;
                        self.client.send(ToServer::BreakBlock(pp.aabb.pos, y, p));
                    }
                    _ => {}
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

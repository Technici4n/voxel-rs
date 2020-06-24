use crate::world::World;
use anyhow::Result;
use log::info;
use nalgebra::Vector3;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use voxel_rs_common::block::BlockId;
use voxel_rs_common::physics::aabb::AABB;
use voxel_rs_common::physics::player::PhysicsPlayer;
use voxel_rs_common::{
    data::load_data,
    debug::{send_debug_info, send_perf_breakdown},
    network::{
        messages::{ToClient, ToServer},
        Server, ServerEvent,
    },
    physics::simulation::ServerPhysicsSimulation,
    player::{CloseChunks, RenderDistance},
    world::{
        ChunkPos,
        BlockPos,
    },
    worldgen::DefaultWorldGenerator,
};
use voxel_rs_common::time::BreakdownCounter;

mod light;
mod world;
mod worldgen;

// TODO: refactor
const D: [[i64; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// The data that the server stores for every player.
pub struct PlayerData {
    loaded_chunks: HashMap<ChunkPos, u64>,
    render_distance: RenderDistance,
    close_chunks: CloseChunks,
    block_to_place: BlockId,
}

impl Default for PlayerData {
    fn default() -> Self {
        let render_distance = Default::default();
        let close_chunks = CloseChunks::new(&render_distance);
        Self {
            loaded_chunks: Default::default(),
            render_distance,
            close_chunks,
            block_to_place: 1,
        }
    }
}

/// Start a new server instance.
pub fn launch_server(mut server: Box<dyn Server>) -> Result<()> {
    info!("Starting server");

    let mut server_timing = BreakdownCounter::new();

    // Load data
    let game_data = load_data("data".into())?;

    let mut world = World::new(
        game_data.blocks.clone(),
        Box::new(DefaultWorldGenerator::new(&game_data.blocks.clone())),
    );
    let mut players = HashMap::new();
    let mut physics_simulation = ServerPhysicsSimulation::new();
    let mut close_chunks_merged = Vec::new();

    info!("Server initialized successfully! Starting server loop");
    loop {
        server_timing.start_frame();

        // Handle messages
        loop {
            match server.receive_event() {
                ServerEvent::NoEvent => break,
                ServerEvent::ClientConnected(id) => {
                    info!("Client connected to the server!");
                    physics_simulation.set_player_input(id, Default::default());
                    players.insert(id, PlayerData::default());
                    server.send(id, ToClient::GameData(game_data.clone()));
                    server.send(id, ToClient::CurrentId(id));
                }
                ServerEvent::ClientDisconnected(id) => {
                    physics_simulation.remove(id);
                    players.remove(&id);
                }
                ServerEvent::ClientMessage(id, message) => match message {
                    ToServer::UpdateInput(input) => {
                        assert!(players.contains_key(&id));
                        physics_simulation.set_player_input(id, input);
                    }
                    ToServer::SetRenderDistance(render_distance) => {
                        assert!(players.contains_key(&id));
                        players.entry(id).and_modify(move |player_data| {
                            player_data.render_distance = render_distance
                        });
                    }
                    ToServer::BreakBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        let y = yaw.to_radians();
                        let p = pitch.to_radians();
                        let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
                        // TODO: don't hardcode max dist
                        if let Some((block, _face)) =
                            physics_player.get_pointed_at(dir, 10.0, &world)
                        {
                            let chunk_pos = block.containing_chunk_pos();
                            if let Some(chunk) = world.get_chunk(chunk_pos) {
                                let mut new_chunk = (*chunk).clone();
                                new_chunk.set_block_at(block.pos_in_containing_chunk(), 0);
                                world.set_chunk(Arc::new(new_chunk));
                            }
                        }
                    }
                    ToServer::SelectBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        let y = yaw.to_radians();
                        let p = pitch.to_radians();
                        let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
                        // TODO: don't hardcode max dist
                        if let Some((block, _face)) =
                            physics_player.get_pointed_at(dir, 10.0, &world)
                        {
                            // TODO: careful with more complicated blocks
                            players.get_mut(&id).unwrap().block_to_place = world.get_block(block);
                        }
                    }
                    ToServer::PlaceBlock(player_pos, yaw, pitch) => {
                        // TODO: check player pos and block
                        let physics_player = PhysicsPlayer {
                            aabb: AABB {
                                pos: player_pos,
                                size_x: 0.0,
                                size_y: 0.0,
                                size_z: 0.0,
                            },
                            velocity: Vector3::zeros(),
                        };
                        let y = yaw.to_radians();
                        let p = pitch.to_radians();
                        let dir = Vector3::new(-y.sin() * p.cos(), p.sin(), -y.cos() * p.cos());
                        // TODO: don't hardcode max dist
                        if let Some((mut block, face)) =
                        physics_player.get_pointed_at(dir, 10.0, &world)
                        {
                            block.px += D[face][0];
                            block.py += D[face][1];
                            block.pz += D[face][2];
                            let chunk_pos = block.containing_chunk_pos();
                            if let Some(chunk) = world.get_chunk(chunk_pos) {
                                let mut new_chunk = (*chunk).clone();
                                new_chunk.set_block_at(block.pos_in_containing_chunk(), players.get(&id).unwrap().block_to_place);
                                world.set_chunk(Arc::new(new_chunk));
                            }
                        }
                    }
                },
            }
        }
        server_timing.record_part("Network events");

        // Receive generated chunks
        world.get_new_generated_chunks();
        server_timing.record_part("Receive generated chunks");

        // Receive lighted chunks
        world.get_new_light_chunks();
        server_timing.record_part("Receive lighted chunks");

        // Tick game
        physics_simulation.step_simulation(Instant::now(), &world);
        server_timing.record_part("Update physics");

        // Send physics updates to players
        for (&player, _) in players.iter() {
            server.send(
                player,
                ToClient::UpdatePhysics((*physics_simulation.get_state()).clone()),
            );
        }
        server_timing.record_part("Send physics updates to players");

        // Send chunks to players
        let mut player_positions = Vec::new();
        for (player, data) in players.iter_mut() {
            let player_pos = BlockPos::from(physics_simulation
                .get_state()
                .physics_state
                .players
                .get(player)
                .unwrap()
                .get_camera_position()
            );
            let player_chunk = player_pos.containing_chunk_pos();
            player_positions.push((player_chunk, data.render_distance));
            // Send new chunks
            let updates = world.send_chunks_to_player(player_chunk, data);
            for (chunk, light_chunk) in updates {
                server.send(*player, ToClient::Chunk(chunk, light_chunk));
            }
            // Drop chunks that are too far away
            let render_distance = data.render_distance;
            data.loaded_chunks
                .retain(|chunk_pos, _| render_distance.is_chunk_visible(player_chunk, *chunk_pos));
        }
        server_timing.record_part("Send chunks to players");

        // Compute close chunks
        for (_, data) in players.iter_mut() {
            data.close_chunks.update(&data.render_distance);
        }
        let all_close_chunks = players
            .iter()
            .map(|(id, data)| {
                let player = physics_simulation.get_state().physics_state.players.get(id).unwrap();
                let player_chunk = BlockPos::from(player.aabb.pos).containing_chunk_pos(); // TODO: have this in the physics state?
                data.close_chunks.get_close_chunks().iter().map(|chunk_pos| CloseChunkPos::new(*chunk_pos, player_chunk)).collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        voxel_rs_common::collections::merge_arrays(&mut close_chunks_merged, &all_close_chunks[..]);
        let close_chunks = close_chunks_merged.iter().map(|&ccp| ccp.pos).collect::<Vec<_>>();
        server_timing.record_part("Compute close chunks");
        
        // Update light
        world.enqueue_chunks_for_lighting(&close_chunks);
        server_timing.record_part("Send chunks to light worker");

        // Update worldgen
        world.enqueue_chunks_for_worldgen(&close_chunks);
        server_timing.record_part("Send chunks to worldgen worker");

        // Drop chunks that are far from all players
        world.drop_far_chunks(&player_positions);
        server_timing.record_part("Drop far chunks");

        send_debug_info("Chunks", "server",
                        format!(
                            "Server loaded chunks = {}\nServer loaded chunk columns = {}\n",
                            world.num_loaded_chunks(),
                            world.num_loaded_chunk_columns(),
                        ));

        // Nothing else to do for now :-)
        send_perf_breakdown("Server", "mainloop", "Server main loop", server_timing.extract_part_averages());
    }
}

#[derive(Clone, Copy)]
struct CloseChunkPos {
    square_dist: u64,
    pub pos: ChunkPos,
}

impl CloseChunkPos {
    pub fn new(relative_pos: ChunkPos, reference_chunk: ChunkPos) -> Self {
        let absolute_pos = relative_pos.offset_by_pos(reference_chunk);
        Self {
            square_dist: absolute_pos.squared_euclidian_distance(reference_chunk),
            pos: absolute_pos,
        }
    }
}

impl PartialEq for CloseChunkPos {
    fn eq(&self, other: &CloseChunkPos) -> bool {
        self.square_dist == other.square_dist
    }
}

impl PartialOrd for CloseChunkPos {
    fn partial_cmp(&self, other: &CloseChunkPos) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CloseChunkPos {}
impl Ord for CloseChunkPos {
    fn cmp(&self, other: &CloseChunkPos) -> std::cmp::Ordering {
        self.square_dist.cmp(&other.square_dist)
    }
}

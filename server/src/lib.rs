use crate::worldgen::WorldGenerationWorker;
use anyhow::Result;
use log::info;
use nalgebra::Vector3;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use voxel_rs_common::physics::aabb::AABB;
use voxel_rs_common::physics::player::PhysicsPlayer;
use voxel_rs_common::{
    data::load_data,
    debug::send_debug_info,
    network::{
        messages::{ToClient, ToServer},
        Server, ServerEvent,
    },
    physics::simulation::ServerPhysicsSimulation,
    player::RenderDistance,
    time::AverageTimeCounter,
    world::CompressedLightChunk,
    world::{
        chunk::{ChunkPos, CompressedChunk},
        BlockPos, World,
    },
    worldgen::DefaultWorldGenerator,
};
use voxel_rs_common::world::chunk::CHUNK_SIZE;
use voxel_rs_common::light::FastBFSQueue;

mod worldgen;

/// The data that the server stores for every player.
#[derive(Debug, Clone, Default)]
struct PlayerData {
    loaded_chunks: HashSet<ChunkPos>,
    render_distance: RenderDistance,
}

/// Start a new server instance.
pub fn launch_server(mut server: Box<dyn Server>) -> Result<()> {
    info!("Starting server");

    // Load data
    let game_data = load_data("data".into())?;

    let mut world_generator = WorldGenerationWorker::new(
        Box::new(DefaultWorldGenerator::new(&game_data.blocks.clone())),
        game_data.blocks.clone(),
    );

    let mut world = World::new();
    let mut players = HashMap::new();
    let mut physics_simulation = ServerPhysicsSimulation::new();
    // Chunks that are currently generating.
    let mut generating_chunks = HashSet::new();
    // Pending light updates
    let mut update_lightning_chunks = HashSet::new();
    let mut update_lightning_chunks_vec = Vec::new();
    // Light update BFS queue
    let mut light_bfs_queue = FastBFSQueue::new();
    let mut ligth_data_reuse = [0; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    let mut opaque_reuse = [false; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE * 27) as usize];
    let mut light_timing = AverageTimeCounter::new();

    info!("Server initialized successfully! Starting server loop");
    loop {
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
                            if world.has_chunk(chunk_pos) {
                                let mut new_chunk = (*world.get_chunk(chunk_pos).unwrap()).clone();
                                new_chunk.set_block_at(block.pos_in_containing_chunk(), 0);
                                world.set_chunk(new_chunk);

                                // TODO: remove copy paste
                                if world.update_highest_opaque_block(chunk_pos) {
                                    // recompute the light of the 3x3 columns
                                    for &c_pos in world.chunks.keys() {
                                        if c_pos.py <= chunk_pos.py
                                            && (c_pos.px - chunk_pos.px).abs() <= 1
                                            && (c_pos.pz - chunk_pos.pz).abs() <= 1
                                        {
                                            if !update_lightning_chunks.contains(&c_pos) {
                                                update_lightning_chunks.insert(c_pos);
                                                update_lightning_chunks_vec.push(c_pos);
                                            }
                                        }
                                    }
                                } else {
                                    // compute only the light for the chunk
                                    for &c_pos in world.chunks.keys() {
                                        if (c_pos.py - chunk_pos.py).abs() <= 1
                                            && (c_pos.px - chunk_pos.px).abs() <= 1
                                            && (c_pos.pz - chunk_pos.pz).abs() <= 1
                                        {
                                            if !update_lightning_chunks.contains(&c_pos) {
                                                update_lightning_chunks.insert(c_pos);
                                                update_lightning_chunks_vec.push(c_pos);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }

        for chunk in world_generator.get_processed_chunks().into_iter() {
            // Only insert the chunk in the world if it was still being generated.
            if generating_chunks.remove(&chunk.pos) {
                let pos = chunk.pos.clone();
                world.set_chunk(chunk);
                if world.update_highest_opaque_block(pos) {
                    // recompute the light of the 3x3 columns
                    for &c_pos in world.chunks.keys() {
                        if c_pos.py <= pos.py
                            && (c_pos.px - pos.px).abs() <= 1
                            && (c_pos.pz - pos.pz).abs() <= 1
                        {
                            if !update_lightning_chunks.contains(&c_pos) {
                                update_lightning_chunks.insert(c_pos);
                                update_lightning_chunks_vec.push(c_pos);
                            }
                        }
                    }
                } else {
                    // compute only the light for the chunk
                    for &c_pos in world.chunks.keys() {
                        if (c_pos.py - pos.py).abs() <= 1
                            && (c_pos.px - pos.px).abs() <= 1
                            && (c_pos.pz - pos.pz).abs() <= 1
                        {
                            if !update_lightning_chunks.contains(&c_pos) {
                                update_lightning_chunks.insert(c_pos);
                                update_lightning_chunks_vec.push(c_pos);
                            }
                        }
                    }
                }
            }
        }

        let mut lightning_prob_pass = HashMap::new(); // probable number of re-update of light
        for c_pos in update_lightning_chunks.iter(){

            for i in -1..=1{
                for j in -1..=1{
                    for k in -1..=1{
                        let pos_adj = c_pos.offset(i,j, k);
                        if generating_chunks.contains(&pos_adj){
                            let u = match lightning_prob_pass.remove(&pos_adj){
                                None => 0,
                                Some(value) => value+1,
                            };
                            lightning_prob_pass.insert(pos_adj, u);
                        }
                    }
                }
            }

        }



        // Update light of one chunk at the time
        update_lightning_chunks_vec.sort_unstable_by_key(|pos| {
            let u = match lightning_prob_pass.get(&pos){
                None => 0,
                Some(i) => *i,
            };

            let mut min_distance = 1_000_000_000;
            for (player, _) in players.iter() {
                if let Some(pl) = physics_simulation
                    .get_state()
                    .physics_state
                    .players
                    .get(player)
                {
                    min_distance = min_distance.min(pos.squared_euclidian_distance(
                        BlockPos::from(pl.aabb.pos).containing_chunk_pos(),
                    ));
                }
            }
            (u, -pos.py, -(min_distance as i64))
        });

        let t0 = Instant::now();
        while (Instant::now() - t0).subsec_millis() < 25 {
            if let Some(pos) = update_lightning_chunks_vec.pop() {
                let t1 = Instant::now();
                world.update_light(&pos, &mut light_bfs_queue, &mut ligth_data_reuse, &mut opaque_reuse);
                update_lightning_chunks.remove(&pos);
                let t2 = Instant::now();
                light_timing.add_time(t2 - t1);
                for (_, data) in players.iter_mut() {
                    data.loaded_chunks.remove(&pos);
                }
            }else{
                break;
            }
        }
        send_debug_info("Server", "avglight", format!("Average time to compute light: {} μs", light_timing.average_time_micros()));

        // Tick game
        physics_simulation.step_simulation(Instant::now(), &world);
        // Send updates to players
        for (&player, _) in players.iter() {
            server.send(
                player,
                ToClient::UpdatePhysics((*physics_simulation.get_state()).clone()),
            );
        }

        // Send chunks to players
        let mut player_positions = Vec::new();
        for (player, data) in players.iter_mut() {
            let player_pos = physics_simulation
                .get_state()
                .physics_state
                .players
                .get(player)
                .unwrap()
                .get_camera_position();
            player_positions.push((player_pos, data.render_distance));
            // Send new chunks
            for chunk_pos in data.render_distance.iterate_around_player(player_pos) {
                // The player hasn't received the chunk yet
                if !data.loaded_chunks.contains(&chunk_pos) {
                    if let Some(chunk) = world.get_chunk(chunk_pos) {
                        // Send it to the player if it's in the world
                        server.send(
                            *player,
                            ToClient::Chunk(
                                CompressedChunk::from_chunk(&chunk),
                                CompressedLightChunk::from_chunk(
                                    &world.get_add_light_chunk(chunk_pos),
                                ),
                            ),
                        );
                        data.loaded_chunks.insert(chunk_pos);
                    } else {
                        // Generate the chunk if it's not already generating
                        let actually_inserted = generating_chunks.insert(chunk_pos);
                        if actually_inserted {
                            world_generator.enqueue_chunk(chunk_pos);
                        }
                    }
                }
            }
            // Drop chunks that are too far away
            let render_distance = data.render_distance;
            data.loaded_chunks
                .retain(|chunk_pos| render_distance.is_chunk_visible(player_pos, *chunk_pos));
        }

        // Drop chunks that are far from all players (and update chunk priorities)
        let World {
            ref mut chunks,
            ref mut light,
            ..
        } = world;
        chunks.retain(|chunk_pos, _| {
            for (player_position, render_distance) in player_positions.iter() {
                if render_distance.is_chunk_visible(*player_position, *chunk_pos) {
                    return true;
                }
            }
            light.remove(chunk_pos);
            false
        });
        generating_chunks.retain(|chunk_pos| {
            let mut min_distance = 1_000_000_000;
            let mut retain = false;
            for (player_position, render_distance) in player_positions.iter() {
                if render_distance.is_chunk_visible(*player_position, *chunk_pos) {
                    min_distance = min_distance.min(chunk_pos.squared_euclidian_distance(
                        BlockPos::from(*player_position).containing_chunk_pos(),
                    ));
                    retain = true;
                }
            }
            if !retain {
                world_generator.dequeue_chunk(*chunk_pos);
            } else {
                world_generator.set_chunk_priority(*chunk_pos, min_distance);
            }
            retain
        });

        send_debug_info("Chunks", "server",
                        format!(
                            "Server loaded chunks = {}\nServer loaded light chunks = {}\nServer generating chunks = {}\nServer pending lighting chunks = {}",
                            world.chunks.len(),
                            world.light.len(),
                            generating_chunks.len(),
                            update_lightning_chunks_vec.len(),
                        ));

        // Nothing else to do for now :-)
    }
}

use crate::worldgen::WorldGenerationWorker;
use anyhow::Result;
use log::info;
use std::collections::{HashMap, HashSet};
use voxel_rs_common::{
    worldgen::DefaultWorldGenerator,
    data::load_data,
    network::{
        messages::{ToClient, ToServer},
        Server, ServerEvent,
    },
    world::{
        BlockPos,
        chunk::{ChunkPos, CompressedChunk},
        World,
    },
    player::RenderDistance
};

mod worldgen;

/// The data that the server stores for every player.
#[derive(Debug, Clone, Default)]
struct PlayerData {
    position: (f64, f64, f64),
    loaded_chunks: HashSet<ChunkPos>,
    render_distance: RenderDistance,
}

/// Start a new server instance.
pub fn launch_server(mut server: Box<dyn Server>) -> Result<()> {
    info!("Starting server");

    // Load data
    let game_data = load_data("data".into())?;

    let mut world_generator =
        WorldGenerationWorker::new(Box::new(DefaultWorldGenerator), game_data.blocks.clone());

    let mut world = World::new();
    let mut players = HashMap::new();
    // Chunks that are currently generating.
    let mut generating_chunks = HashSet::new();

    info!("Server initialized successfully! Starting server loop");
    loop {
        // Handle messages
        loop {
            match server.receive_event() {
                ServerEvent::NoEvent => break,
                ServerEvent::ClientConnected(id) => {
                    info!("Client connected to the server!");
                    server.send(id, ToClient::GameData(game_data.clone()));
                }
                ServerEvent::ClientDisconnected(_id) => {}
                ServerEvent::ClientMessage(id, message) => match message {
                    ToServer::SetPos(pos) => {
                        players.entry(id).or_insert(PlayerData::default()).position = pos;
                    }
                    ToServer::SetRenderDistance(render_distance) => {
                        players.entry(id).or_insert(PlayerData::default()).render_distance = render_distance;
                    }
                },
            }
        }

        for chunk in world_generator.get_processed_chunks().into_iter() {
            // Only insert the chunk in the world if it was still being generated.
            if generating_chunks.contains(&chunk.pos) {
                world.set_chunk(chunk);
            }
        }

        // Tick game
        // TODO: move cameras properly

        // Send chunks to players
        let mut player_positions = Vec::new();
        for (player, data) in players.iter_mut() {
            player_positions.push((data.position, data.render_distance));
            // Send new chunks
            for chunk_pos in data.render_distance.iterate_around_player(data.position) {
                // The player hasn't received the chunk yet
                if !data.loaded_chunks.contains(&chunk_pos) {
                    if let Some(chunk) = world.get_chunk(chunk_pos) {
                        // Send it to the player if it's in the world
                        server.send(
                            *player,
                            ToClient::Chunk(CompressedChunk::from_chunk(chunk)),
                        );
                        data.loaded_chunks.insert(chunk_pos);
                    }
                    else {
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
            let position = data.position;
            data.loaded_chunks.retain(|chunk_pos| {
                render_distance.is_chunk_visible(position, *chunk_pos)
            });
        }

        // Drop chunks that are far from all players (and update chunk priorities)
        world.chunks.retain(|chunk_pos, _| {
            for (player_position, render_distance) in player_positions.iter() {
                if render_distance.is_chunk_visible(*player_position, *chunk_pos) {
                    return true;
                }
            }
            false
        });
        generating_chunks.retain(|chunk_pos| {
            let mut min_distance = 1_000_000_000;
            let mut retain = false;
            for (player_position, render_distance) in player_positions.iter() {
                if render_distance.is_chunk_visible(*player_position, *chunk_pos) {
                    min_distance = min_distance.min(
                        chunk_pos.squared_euclidian_distance(BlockPos::from(*player_position).containing_chunk_pos())
                    );
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

        // Nothing else to do for now :-)
    }
}

use crate::worldgen::WorldGenerationWorker;
use anyhow::Result;
use log::info;
use std::collections::{HashMap, HashSet};
use voxel_rs_common::worldgen::DefaultWorldGenerator;
use voxel_rs_common::{
    data::load_data,
    network::{
        messages::{ToClient, ToServer},
        Server, ServerEvent,
    },
    world::{
        chunk::{ChunkPos, CompressedChunk},
        BlockPos, World,
    },
};

mod worldgen;

/// The data that the server stores for every player.
#[derive(Debug, Clone, Default)]
struct PlayerData {
    position: (f64, f64, f64),
    loaded_chunks: HashSet<ChunkPos>,
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
            let position = BlockPos {
                px: data.position.0 as i64,
                py: data.position.1 as i64,
                pz: data.position.2 as i64,
            };
            let position = position.containing_chunk_pos();
            player_positions.push(position);
            // TODO: render distance check
            // Send new chunks
            for i in -1..=1 {
                for j in -1..=1 {
                    for k in -1..=1 {
                        let position = ChunkPos {
                            px: position.px + i,
                            py: position.py + j,
                            pz: position.pz + k,
                        };
                        // The player hasn't received the chunk yet
                        if !data.loaded_chunks.contains(&position) {
                            // Generate it if it's not in the world
                            if !world.has_chunk(position) {
                                // Generate the chunk if it's not already generating
                                let actually_inserted = generating_chunks.insert(position);
                                if actually_inserted {
                                    world_generator.enqueue_chunk(position);
                                }
                            } else {
                                // Send it to the player
                                server.send(
                                    *player,
                                    ToClient::Chunk(CompressedChunk::from_chunk(
                                        world.get_chunk(position).unwrap(),
                                    )),
                                );
                                data.loaded_chunks.insert(position);
                            }
                        }
                    }
                }
            }
            // TODO: render distance check
            // Drop chunks that are too far away
            data.loaded_chunks.retain(|chunk_pos| {
                (chunk_pos.px - position.px)
                    .abs()
                    .max((chunk_pos.py - position.py).abs())
                    .max((chunk_pos.pz - position.pz).abs())
                    <= 1
            })
        }

        // Drop chunks that are far from all players
        // TODO: render distance check
        world.chunks.retain(|&chunk_pos, _| {
            for player_position in player_positions.iter() {
                if (chunk_pos.px - player_position.px)
                    .abs()
                    .max((chunk_pos.py - player_position.py).abs())
                    .max((chunk_pos.pz - player_position.pz).abs())
                    <= 1
                {
                    return true;
                }
            }
            false
        });
        generating_chunks.retain(|&chunk_pos| {
            for player_position in player_positions.iter() {
                if (chunk_pos.px - player_position.px)
                    .abs()
                    .max((chunk_pos.py - player_position.py).abs())
                    .max((chunk_pos.pz - player_position.pz).abs())
                    <= 1
                {
                    return true;
                }
            }
            world_generator.dequeue_chunk(chunk_pos);
            false
        })

        // Nothing else to do for now :-)
    }
}

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
        BlockPos, World, WorldGenerator,
    },
};

#[derive(Debug, Clone, Default)]
struct PlayerData {
    position: (f64, f64, f64),
    loaded_chunks: HashSet<ChunkPos>,
}

pub fn launch_server(mut server: Box<dyn Server>) -> Result<()> {
    info!("Starting server");

    let mut world = World::new();
    let mut world_generator = Box::new(DefaultWorldGenerator);

    let mut players = HashMap::new();

    // Load data
    let game_data = load_data("data".into())?;

    info!("Starting server loop");
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

        // Tick game
        // TODO: move cameras properly

        // Send chunks to players
        for (player, data) in players.iter_mut() {
            let position = BlockPos {
                px: data.position.0 as i64,
                py: data.position.1 as i64,
                pz: data.position.2 as i64,
            };
            let position = position.containing_chunk_pos();
            for i in -1..=1 {
                for j in -1..=1 {
                    for k in -1..=1 {
                        let position = ChunkPos {
                            px: position.px + i,
                            py: position.py + j,
                            pz: position.pz + k,
                        };
                        if !data.loaded_chunks.contains(&position) {
                            if !world.has_chunk(position) {
                                world.set_chunk(
                                    world_generator.generate_chunk(position, &game_data.blocks),
                                );
                            }
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

        // TODO: drop chunks that are far from all players

        // Nothing else to do for now :-)
    }
}

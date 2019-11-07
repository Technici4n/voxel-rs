use crate::player::PlayerId;
use crate::{
    data::Data,
    physics::simulation::ServerState,
    player::{PlayerInput, RenderDistance},
    world::chunk::CompressedChunk,
};

/// A message sent to the server by the client
#[derive(Debug, Clone)]
pub enum ToServer {
    /// Update player render distance
    SetRenderDistance(RenderDistance),
    /// Update the player's input
    UpdateInput(PlayerInput),
}

/// A message sent to the client by the server
#[derive(Debug, Clone)]
pub enum ToClient {
    /// Send the game data
    GameData(Data),
    /// Send the chunk at some position
    Chunk(CompressedChunk),
    /// Update the whole of the physics simulation
    // TODO: only send part of the physics simulation
    UpdatePhysics(ServerState),
    /// Set the id of a player
    CurrentId(PlayerId),
}

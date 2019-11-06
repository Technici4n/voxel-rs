use crate::{
    world::chunk::CompressedChunk,
    data::Data,
    player::RenderDistance
};

/// A message sent to the server by the client
#[derive(Debug, Clone)]
pub enum ToServer {
    /// Update player position TODO: remove this
    SetPos((f64, f64, f64)),
    /// Update player render distance
    SetRenderDistance(RenderDistance),
    /*/// Update the current player's input
    UpdateInput,*/
}

/// A message sent to the client by the server
#[derive(Debug, Clone)]
pub enum ToClient {
    /// Send the game data
    GameData(Data),
    /// Send the chunk at some position
    Chunk(CompressedChunk),
    /*/// Update part of the physics simulation
    UpdatePhysics,*/
}

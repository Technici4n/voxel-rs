use crate::{
    data::Data,
    physics::simulation::ServerState,
    player::PlayerId,
    player::{PlayerInput, RenderDistance},
    world::{Chunk, LightChunk},
};
use nalgebra::Vector3;
use std::sync::Arc;

/// A message sent to the server by the client
#[derive(Debug, Clone)]
pub enum ToServer {
    /// Update player render distance
    SetRenderDistance(RenderDistance),
    /// Update the player's input
    UpdateInput(PlayerInput),
    /// Break a block (player pos, yaw, pitch)
    BreakBlock(Vector3<f64>, f64, f64),
    /// Select a block
    SelectBlock(Vector3<f64>, f64, f64),
    /// Place a block
    PlaceBlock(Vector3<f64>, f64, f64),
}

/// A message sent to the client by the server
#[derive(Debug, Clone)]
pub enum ToClient {
    /// Send the game data
    GameData(Data),
    /// Send the chunk at some position
    Chunk(Arc<Chunk>, Arc<LightChunk>),
    /// Update the whole of the physics simulation
    // TODO: only send part of the physics simulation
    UpdatePhysics(ServerState),
    /// Set the id of a player
    CurrentId(PlayerId),
}

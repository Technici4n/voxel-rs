use crate::{
    physics::camera::default_camera,
    physics::player::PhysicsPlayer,
    physics::BlockContainer,
    player::{PlayerId, PlayerInput},
};
use nalgebra::Vector3;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

/// Input of the whole simulation.
#[derive(Debug, Clone, Default)]
pub struct Input {
    pub(self) player_inputs: HashMap<PlayerId, PlayerInput>,
}

/// Physics state of the whole simulation.
#[derive(Debug, Clone, Default)]
pub struct PhysicsState {
    pub players: HashMap<PlayerId, PhysicsPlayer>,
}

impl PhysicsState {
    /// Step the full physics simulation.
    /// For now, it just moves all connected players.
    pub fn step_simulation<BC: BlockContainer>(&mut self, input: &Input, dt: Duration, world: &BC) {
        let seconds_delta = dt.as_secs_f64();
        for (&id, input) in input.player_inputs.iter() {
            let player = self.players.entry(id).or_insert(Default::default());
            default_camera(player, *input, seconds_delta, world);
        }
        // Remove players that don't exist anymore
        self.players
            .retain(|id, _| input.player_inputs.contains_key(id));
    }
}

/// A physics state sent by the server.
#[derive(Debug, Clone)]
pub struct ServerState {
    pub physics_state: PhysicsState,
    pub server_time: Instant,
    pub input: Input,
}

/// The client's physics simulation
pub struct ClientPhysicsSimulation {
    /// Previous client inputs
    client_inputs: Vec<(Instant, PlayerInput)>,
    /// Last state validated by the server
    last_server_state: ServerState,
    /// Current simulation state
    current_state: PhysicsState,
    /// Dirty flag: whether the physics need to be computed again starting from the last server state.
    needs_recomputing: bool,
    /// Id of the current player
    player_id: PlayerId,
}

impl ClientPhysicsSimulation {
    /// Create a new simulation from some `ServerState` and the client's id
    pub fn new(server_state: ServerState, player_id: PlayerId) -> Self {
        Self {
            client_inputs: Vec::new(),
            last_server_state: server_state.clone(),
            current_state: server_state.physics_state,
            needs_recomputing: false,
            player_id,
        }
    }

    /// Process a server update
    pub fn receive_server_update(&mut self, state: ServerState) {
        // Save state
        self.last_server_state = state;
        // Drop inputs anterior to this server state
        let last_server_time = self.last_server_state.server_time;
        self.client_inputs
            .retain(|(time, _)| *time > last_server_time);
        // Mark dirty
        self.needs_recomputing = true;
    }

    /// Get the camera position of the client
    pub fn get_camera_position(&self) -> Vector3<f64> {
        self.current_state
            .players
            .get(&self.player_id)
            .unwrap()
            .get_camera_position()
    }

    /// Get the client player
    pub fn get_player(&self) -> &PhysicsPlayer {
        self.current_state.players.get(&self.player_id).unwrap()
    }

    /// Step the simulation according to the current input and time
    pub fn step_simulation<BC: BlockContainer>(&mut self, input: PlayerInput, time: Instant, world: &BC) {
        // Recompute simulation if necessary
        if self.needs_recomputing {
            self.needs_recomputing = false;
            self.current_state = self.last_server_state.physics_state.clone();

            let mut previous_time = self.last_server_state.server_time;
            for &(time, player_input) in self.client_inputs.iter() {
                // First, we have to apply the current client input to the server's input
                self.last_server_state
                    .input
                    .player_inputs
                    .insert(self.player_id, player_input);
                // Only then can we step the simulation
                self.current_state.step_simulation(
                    &self.last_server_state.input,
                    time - previous_time,
                    world,
                );
                previous_time = time;
            }
        }

        let previous_instant = match self.client_inputs.last() {
            Some((time, _)) => *time,
            None => self.last_server_state.server_time,
        };

        // Store input for future processing
        self.client_inputs.push((time, input));
        self.last_server_state
            .input
            .player_inputs
            .insert(self.player_id, input);

        // Step local simulation
        self.current_state.step_simulation(
            &self.last_server_state.input,
            time - previous_instant,
            world,
        );
    }
}

/// The server's physics simulation
pub struct ServerPhysicsSimulation {
    /// The current state of the simulation
    server_state: ServerState,
}

impl ServerPhysicsSimulation {
    /// Create a new simulation with no connected players starting at the current time
    pub fn new() -> Self {
        Self {
            server_state: ServerState {
                physics_state: PhysicsState::default(),
                server_time: Instant::now(),
                input: Default::default(),
            },
        }
    }

    /// Update the input of a player
    pub fn set_player_input(&mut self, player_id: PlayerId, input: PlayerInput) {
        self.server_state
            .input
            .player_inputs
            .insert(player_id, input);
    }

    /// Remove a player from the simulation
    pub fn remove(&mut self, player_id: PlayerId) {
        self.server_state.input.player_inputs.remove(&player_id);
    }

    /// Step the simulation according to the current input and time
    pub fn step_simulation<BC: BlockContainer>(&mut self, time: Instant, world: &BC) {
        self.server_state.physics_state.step_simulation(
            &self.server_state.input,
            time - self.server_state.server_time,
            world,
        );
        self.server_state.server_time = time;
    }

    /// Get a reference to the current state of the simulation
    pub fn get_state(&self) -> &ServerState {
        &self.server_state
    }
}

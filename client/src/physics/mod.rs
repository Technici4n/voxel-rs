pub mod aabb;

/*
use crate::{
    world::World,
};
use std::time::Instant;

pub struct Input {}

pub struct PhysicsState {}

pub struct ServerState {
    physics_state: PhysicsState,
    server_time: Instant,
}

/// The client's physics simulation
pub struct ClientPhysicsSimulation {
    /// Previous client inputs
    client_inputs: Vec<(Instant, Input)>,
    /// Last state validated by the server
    last_server_state: ServerState,
    /// Current simulation state
    current_state: PhysicsState,
    /// Dirty flag: whether the physics need to be computed again starting from the last server state.
    needs_recomputing: bool,
}

impl ClientPhysicsSimulation {
    /// Process a server update
    pub fn receive_server_update(&mut self, state: ServerState) {
        // Save state
        self.last_server_state = state;
        // Drop inputs anterior to this server state
        self.client_inputs.retain(|(time, _)| time > self.last_server_state.server_time);
        // Mark dirty
        self.needs_recomputing = true;
    }

    /// Step the simulation according to the current input and time
    pub fn step_simulation(&mut self, input: Input, time: Instant, world: &mut World) {
        // Recompute simulation if necessary
        if self.needs_recomputing {
            self.needs_recomputing = false;
            self.current_state = self.last_server_state.physics_state.clone();

            let mut previous_time = self.last_server_state.server_time;
            for (time, client_input) in self.client_inputs.iter() {
                self.current_state.step_simulation(client_input.clone(), time - previous_time, world);
                previous_time = time.clone();
            }
        }

        // Store input for further processing
        self.client_inputs.push((time.clone(), input.clone()));
        // Step local simulation
        self.current_state.step_simulation(input, time, world);
    }
}*/

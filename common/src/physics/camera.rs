//! This module contains the definition of the `Camera`s.
//!
//! A `Camera` defines how a player's entity reacts to that player's inputs.

use crate::physics::aabb::AABB;
use crate::{player::PlayerInput, world::World};
use nalgebra::Vector3;

/// The default camera. It doesn't let you go inside blocks unless you are already inside blocks.
pub fn default_camera(
    position: Vector3<f64>,
    input: PlayerInput,
    seconds_delta: f64,
    world: &World,
) -> Vector3<f64> {
    // Unit vector in the `angle` direction
    fn movement_direction(yaw: f64, angle: f64) -> Vector3<f64> {
        let yaw = yaw + angle;
        Vector3::new(-yaw.to_radians().sin(), 0.0, -yaw.to_radians().cos()).normalize()
    }
    // Compute the expected movement of the player, i.e. assuming there are no collisions.
    const SPEED: f64 = 10.0;
    let mut expected_movement = Vector3::new(0.0, 0.0, 0.0);
    if input.key_move_forward {
        expected_movement += movement_direction(input.yaw, 0.0) * (seconds_delta * SPEED) as f64;
    }
    if input.key_move_left {
        expected_movement += movement_direction(input.yaw, 90.0) * (seconds_delta * SPEED) as f64;
    }
    if input.key_move_backward {
        expected_movement += movement_direction(input.yaw, 180.0) * (seconds_delta * SPEED) as f64;
    }
    if input.key_move_right {
        expected_movement += movement_direction(input.yaw, 270.0) * (seconds_delta * SPEED) as f64;
    }
    if input.key_move_up {
        expected_movement.y += (seconds_delta * SPEED) as f64;
    }
    if input.key_move_down {
        expected_movement.y -= (seconds_delta * SPEED) as f64;
    }
    // Move taking collisions into account
    // TODO: add a noclip camera mode
    // TODO: use constants for player AABB
    let aabb_offset = Vector3::new(0.4, 1.6, 0.4);
    let mut player_aabb = AABB::new(position - aabb_offset, (0.8, 1.8, 0.8));
    player_aabb.move_check_collision(world, expected_movement);
    player_aabb.pos + aabb_offset
}

//! This module contains the definition of the `Camera`s.
//!
//! A `Camera` defines how a player's entity reacts to that player's inputs.

use crate::{
    debug::send_debug_info, physics::player::PhysicsPlayer, player::PlayerInput,
};
use super::BlockContainer;
use nalgebra::Vector3;

/// The default camera. It doesn't let you go inside blocks unless you are already inside blocks.
// TODO: use better integrator (RK4 ?)
pub fn default_camera<BC: BlockContainer>(
    player: &mut PhysicsPlayer,
    input: PlayerInput,
    seconds_delta: f64,
    world: &BC,
) {
    // Unit vector in the `angle` direction
    fn movement_direction(yaw: f64, angle: f64) -> Vector3<f64> {
        let yaw = yaw + angle;
        Vector3::new(-yaw.to_radians().sin(), 0.0, -yaw.to_radians().cos()).normalize()
    }
    // Normalize the vector if it can be normalized or return 0 othersize
    fn normalize_or_zero(v: Vector3<f64>) -> Vector3<f64> {
        if v.norm() > 1e-9f64 {
            v.normalize()
        } else {
            Vector3::zeros()
        }
    }
    // Compute the expected movement of the player, i.e. assuming there are no collisions.
    if input.flying || player.aabb.intersect_world(world) {
        const ACCELERATION: f64 = 50.0;
        const MAX_SPEED: f64 = 30.0;
        player.velocity.y = 0.0;
        // If the player is flying, then we update its velocity. By default, it falls off to 0
        let mut player_acceleration = Vector3::zeros();
        if input.key_move_forward {
            player_acceleration += movement_direction(input.yaw, 0.0);
        }
        if input.key_move_left {
            player_acceleration += movement_direction(input.yaw, 90.0);
        }
        if input.key_move_backward {
            player_acceleration += movement_direction(input.yaw, 180.0);
        }
        if input.key_move_right {
            player_acceleration += movement_direction(input.yaw, 270.0);
        }
        let auto_acceleration = -normalize_or_zero(player.velocity);
        let player_acceleration = normalize_or_zero(player_acceleration);
        let player_acceleration =
            (player_acceleration * 1.5 + auto_acceleration * 0.5) * ACCELERATION;
        player.velocity += player_acceleration * seconds_delta;
        if player.velocity.norm() > MAX_SPEED {
            player.velocity *= MAX_SPEED / player.velocity.norm();
        }
        let mut expected_movement = player.velocity * seconds_delta;
        if input.key_move_up {
            expected_movement.y += (seconds_delta * MAX_SPEED) as f64;
        }
        if input.key_move_down {
            expected_movement.y -= (seconds_delta * MAX_SPEED) as f64;
        }
        player.aabb.move_check_collision(world, expected_movement);
    } else {
        const JUMP_SPEED: f64 = 8.0;
        const GRAVITY_ACCELERATION: f64 = 25.0;
        const MAX_DOWN_SPEED: f64 = 30.0;
        const HORIZONTAL_SPEED: f64 = 7.0;
        player.velocity.x = 0.0;
        player.velocity.z = 0.0;
        let mut horizontal_velocity = Vector3::zeros();
        if input.key_move_forward {
            horizontal_velocity += movement_direction(input.yaw, 0.0);
        }
        if input.key_move_left {
            horizontal_velocity += movement_direction(input.yaw, 90.0);
        }
        if input.key_move_backward {
            horizontal_velocity += movement_direction(input.yaw, 180.0);
        }
        if input.key_move_right {
            horizontal_velocity += movement_direction(input.yaw, 270.0);
        }
        let horizontal_velocity = normalize_or_zero(horizontal_velocity) * HORIZONTAL_SPEED;
        if player.aabb.is_on_the_ground(world) {
            player.velocity.y = if input.key_move_up { JUMP_SPEED } else { 0.0 };
        } else {
            player.velocity.y -= GRAVITY_ACCELERATION * seconds_delta;
            if player.velocity.y < -MAX_DOWN_SPEED {
                player.velocity.y = -MAX_DOWN_SPEED;
            }
        };
        let expected_movement = (player.velocity + horizontal_velocity) * seconds_delta;
        player.aabb.move_check_collision(world, expected_movement);
    }
    // TODO: add a noclip camera mode
    send_debug_info(
        "Physics",
        "ontheground",
        format!(
            "Player 0 on the ground? {}",
            player.aabb.is_on_the_ground(world)
        ),
    );
    let [vx, vy, vz]: [f64; 3] = player.velocity.into();
    send_debug_info(
        "Physics",
        "velocity",
        format!("velocity: {:.2} {:.2} {:.2}", vx, vy, vz),
    );
}

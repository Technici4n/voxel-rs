use nalgebra::Vector3;
use crate::physics::aabb::AABB;

const PLAYER_SIDE: f64 = 0.8;
const PLAYER_HEIGHT: f64 = 1.8;
const CAMERA_OFFSET: [f64; 3] = [0.4, 1.6, 0.4];

/// The physics representation of a player
#[derive(Debug, Clone)]
pub struct PhysicsPlayer {
    /// The aabb of the player
    pub aabb: AABB,
    /// The current velocity of the player
    pub velocity: Vector3<f64>,
}

impl PhysicsPlayer {
    /// Get the position of the camera
    pub fn get_camera_position(&self) -> Vector3<f64> {
        self.aabb.pos + Vector3::from(CAMERA_OFFSET)
    }
}

impl Default for PhysicsPlayer {
    fn default() -> Self {
        Self {
            aabb: AABB::new(Vector3::zeros(), (PLAYER_SIDE, PLAYER_HEIGHT, PLAYER_SIDE)),
            velocity: Vector3::zeros(),
        }
    }
}
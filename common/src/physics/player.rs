use crate::physics::aabb::AABB;
use crate::world::BlockPos;
use super::BlockContainer;
use nalgebra::Vector3;

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

    /// Ray trace to find the pointed block. Return the position of the block and the face (x/-x/y/-y/z/-z)
    // TODO: use block registry
    pub fn get_pointed_at<BC: BlockContainer>(
        &self,
        dir: Vector3<f64>,
        mut max_dist: f64,
        world: &BC,
    ) -> Option<(BlockPos, usize)> {
        let dir = dir.normalize();
        let mut pos = self.get_camera_position();
        // Check current block first
        let was_inside = world.is_block_full(BlockPos::from(pos));
        let dirs = [
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 0.0, -1.0),
            Vector3::new(0.0, 0.0, 1.0),
        ];
        loop {
            let targets = [
                pos.x.floor(),
                pos.x.ceil(),
                pos.y.floor(),
                pos.y.ceil(),
                pos.z.floor(),
                pos.z.ceil(),
            ];

            let mut curr_min = 1e9;
            let mut face = 0;

            for i in 0..6 {
                let effective_movement = dir.dot(&dirs[i]);
                if effective_movement > 1e-6 {
                    let dir_offset = (targets[i].abs() - pos.dot(&dirs[i]).abs()).abs();
                    let dist = dir_offset / effective_movement;
                    if curr_min > dist {
                        curr_min = dist;
                        face = i;
                    }
                }
            }

            if was_inside {
                return Some((BlockPos::from(pos), face ^ 1));
            }

            if curr_min > max_dist {
                return None;
            } else {
                curr_min += 1e-5;
                max_dist -= curr_min;
                pos += curr_min * dir;
                let block_pos = BlockPos::from(pos);
                if world.is_block_full(block_pos) {
                    return Some((block_pos, face));
                }
            }
        }
    }
}

impl Default for PhysicsPlayer {
    fn default() -> Self {
        Self {
            aabb: AABB::new(
                Vector3::new(1.46, 52.6, 1.85),
                (PLAYER_SIDE, PLAYER_HEIGHT, PLAYER_SIDE),
            ),
            velocity: Vector3::zeros(),
        }
    }
}

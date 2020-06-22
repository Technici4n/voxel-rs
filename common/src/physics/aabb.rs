use super::BlockContainer;
use nalgebra::Vector3;

#[derive(Debug, Clone)]
pub struct AABB {
    pub pos: Vector3<f64>,
    pub size_x: f64,
    pub size_y: f64,
    pub size_z: f64,
}

impl AABB {
    /// Create a new AABB box
    pub fn new(pos: Vector3<f64>, (s_x, s_y, s_z): (f64, f64, f64)) -> Self {
        AABB {
            pos,
            size_x: s_x,
            size_y: s_y,
            size_z: s_z,
        }
    }

    /// Create an AABB box of cubic shape
    pub fn _new_cube(pos: Vector3<f64>, size: f64) -> Self {
        AABB {
            pos,
            size_x: size,
            size_y: size,
            size_z: size,
        }
    }

    /// return true is the AABB box intersect with the other box
    pub fn _intersect(&self, other: &AABB) -> bool {
        if (other.pos.x >= self.pos.x + self.size_x)
            || (other.pos.x + other.size_x <= self.pos.x)
            || (other.pos.y >= self.pos.y + self.size_y)
            || (other.pos.y + other.size_y <= self.pos.y)
            || (other.pos.z >= self.pos.z + self.size_z)
            || (other.pos.z + other.size_z <= self.pos.z)
        {
            return false;
        } else {
            return true;
        }
    }

    /// Return true if point (px, py, pz) is in the AABB box
    pub fn _intersect_point(&self, (px, py, pz): (f64, f64, f64)) -> bool {
        if px >= self.pos.x
            && px <= self.pos.x + self.size_x
            && py >= self.pos.y
            && py <= self.pos.y + self.size_y
            && pz >= self.pos.z
            && pz <= self.pos.z + self.size_z
        {
            return true;
        } else {
            return false;
        }
    }

    /// Return true if the box intersect some block
    pub fn intersect_world<BC: BlockContainer>(&self, world: &BC) -> bool {
        let min_x = self.pos.x.floor() as i64;
        let max_x = (self.pos.x + self.size_x).ceil() as i64;
        let min_y = self.pos.y.floor() as i64;
        let max_y = (self.pos.y + self.size_y).ceil() as i64;
        let min_z = self.pos.z.floor() as i64;
        let max_z = (self.pos.z + self.size_z).ceil() as i64;

        for i in min_x..max_x {
            for j in min_y..max_y {
                for k in min_z..max_z {
                    if world.is_block_full((i, j, k).into()) {
                        return true;
                    }
                }
            }
        }
        return false;
    }

    /// Try to move the box in the world and stop the movement if it goes trough a block
    /// Return the actual deplacement
    pub fn move_check_collision<BC: BlockContainer>(&mut self, world: &BC, delta: Vector3<f64>) -> Vector3<f64> {
        if self.intersect_world(world) {
            self.pos += delta;
            return delta;
        }
        let mut res = Vector3::new(0.0, 0.0, 0.0);

        let x_step = (delta.x.abs() / self.size_x).ceil() as u32;
        let y_step = (delta.y.abs() / self.size_y).ceil() as u32;
        let z_step = (delta.z.abs() / self.size_z).ceil() as u32;

        let ddx = delta.x / (x_step as f64);
        let ddy = delta.y / (y_step as f64);
        let ddz = delta.z / (z_step as f64);

        let old_x = self.pos.x;

        for _ in 0..x_step {
            self.pos.x += ddx;
            if self.intersect_world(world) {
                self.pos.x -= ddx; // canceling the last step

                let mut min_d = 0.0;
                let mut max_d = ddx.abs();

                while max_d - min_d > 0.001 {
                    // binary search the max delta
                    let med = (min_d + max_d) / 2.0;
                    self.pos.x += med * ddx.signum();
                    if self.intersect_world(world) {
                        max_d = med;
                    } else {
                        min_d = med;
                    }
                    self.pos.x -= med * ddx.signum();
                }

                self.pos.x += ddx.signum() * (min_d) / 2.0;
                break;
            }
        }

        res.x = self.pos.x - old_x;
        let old_y = self.pos.y;

        for _ in 0..y_step {
            self.pos.y += ddy;
            if self.intersect_world(world) {
                self.pos.y -= ddy;
                let mut min_d = 0.0;
                let mut max_d = ddy.abs();

                while max_d - min_d > 0.001 {
                    let med = (min_d + max_d) / 2.0;
                    self.pos.y += med * ddy.signum();
                    if self.intersect_world(world) {
                        max_d = med;
                    } else {
                        min_d = med;
                    }
                    self.pos.y -= med * ddy.signum();
                }

                self.pos.y += ddy.signum() * (min_d) / 2.0;
                break;
            }
        }

        res.y = self.pos.y - old_y;
        let old_z = self.pos.z;

        for _ in 0..z_step {
            self.pos.z += ddz;
            if self.intersect_world(world) {
                self.pos.z -= ddz;

                let mut min_d = 0.0;
                let mut max_d = ddz.abs();

                while max_d - min_d > 0.001 {
                    let med = (min_d + max_d) / 2.0;
                    self.pos.z += med * ddz.signum();
                    if self.intersect_world(world) {
                        max_d = med;
                    } else {
                        min_d = med;
                    }
                    self.pos.z -= med * ddz.signum();
                }

                self.pos.z += ddz.signum() * (min_d) / 2.0;
                break;
            }
        }

        res.z = self.pos.z - old_z;
        return res;
    }

    /// Check whether the bounding box is touching the ground
    pub fn is_on_the_ground<BC: BlockContainer>(&mut self, world: &BC) -> bool {
        self.pos.y -= 0.0021;
        let would_intersect_down = self.intersect_world(world);
        self.pos.y += 0.0021;
        !self.intersect_world(world) && would_intersect_down
    }
}

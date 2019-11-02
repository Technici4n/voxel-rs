struct AABB {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub size_x: f32,
    pub size_y: f32,
    pub size_z: f32,

}

impl AABB {
    /// Create a new AABB box
    pub fn new((px, py, pz): (f32, f32, f32), (sX, sY, sZ): (f32, f32, f32)) -> Self {
        AABB {
            x: px,
            y: py,
            z: pz,
            size_x: sX,
            size_y: sY,
            size_z: sZ,
        }
    }

    /// Create an AABB box of cubic shape
    pub fn new_cube((px, py, pz): (f32, f32, f32), size: f32) -> Self {
        AABB {
            x: px,
            y: py,
            z: pz,
            size_x: size,
            size_y: size,
            size_z: size,
        }
    }

    /// return true is the AABB box intersect with the other box
    pub fn intersect(&self, other: &AABB) -> bool {
        if (other.x >= self.x + self.size_x)
            || (other.x + other.size_x <= self.x)
            || (other.y >= self.y + self.size_y)
            || (other.y + other.size_y <= self.y)
            || (other.z >= self.z + self.size_z)
            || (other.z + other.size <= self.z) {
            return false;
        } else {
            return true;
        }
    }

    /// Return true if point (px, py, pz) is in the AABB box
    pub fn intersect_point(&self, (px, py, pz): (f32, f32, f32)) -> bool {
        if px >= self.x && px <= self.x + self.size_x
            && py >= self.y && py <= self.y + self.size_y
            && pz >= self.z && pz <= self.z + self.size_z {
            return true;
        } else {
            return false;
        }
    }

    /// Return true if the box intersect some block
    pub fn intersect_world(&self, world: &World) -> bool {
        let min_x = self.x.floor() as i64;
        let max_x = (self.x + self.size_x).ceil() as i64;
        let min_y = self.size_y.floor() as i64;
        let max_y = (self.size_y + self.size_y).ceil() as i64;
        let min_z = self.size_z.floor() as i64;
        let max_z = (self.size_z + self.size_z).ceil() as i64;

        for i in min_x..max_x {
            for j in min_y..max_y {
                for k in min_z..max_z {
                    if world.get_data(i,j,k) != 0{
                        return true;
                    }
                }
            }
        }
        return false;
    }

    /// Try to move the box in the world and stop the movement if it goes trough a block
    pub fn move_check_collision(&mut self, world: &World, (dx, dy, dz) : (f32, f32, f32)){

        let x_step = (dx.abs()/self.size_x).ceil() as u32;
        let y_step = (dy.abs()/self.size_y).ceil() as u32;
        let z_step = (dz.abs()/self.size_z).ceil() as u32;

        let ddx = dx /(x_step as f32);
        let ddy = dy /(y_step as f32);
        let ddz = dz /(z_step as f32);

        for i in 0..x_step{
            self.posX += ddx;
            if self.intersect_world(){
                self.posX -= ddx; // canceling the last step

                let mut min_d = 0.0;
                let mut max_d = ddx.abs();

                while max_d - min_d > 0.01{ // binary search the max delta
                    let med = (min_d + max_d)/2.0;
                    self.posX += med*ddx.signum();
                    if self.intersect_world(){
                        max_d = med;
                    }else{
                        min_d = med;
                    }
                    self.posX -= med*ddx.signum();
                }

                self.posX += ddx.signum()*(min_d +max_d)/2.0;

                break;
            }

        }

        for j in 0..y_step{
            self.posY += ddy;
            if self.intersect_world(){
                self.posY -= ddy;
                let mut min_d = 0.0;
                let mut max_d = ddy.abs();

                while max_d - min_d > 0.01{
                    let med = (min_d + max_d)/2.0;
                    self.posY += med*ddy.signum();
                    if self.intersect_world(){
                        max_d = med;
                    }else{
                        min_d = med;
                    }
                    self.posY -= med*ddy.signum();
                }

                self.posY += ddy.signum()*(min_d +max_d)/2.0;

                break;
            }
        }

        for k in 0..z_step{
            self.posZ += ddz;
            if self.intersect_world(){
                self.posZ -= ddz;

                let mut min_d = 0.0;
                let mut max_d = ddz.abs();

                while max_d - min_d > 0.01{
                    let med = (min_d + max_d)/2.0;
                    self.posZ += med*ddz.signum();
                    if self.intersect_world(){
                        max_d = med;
                    }else{
                        min_d = med;
                    }
                    self.posZ -= med*ddz.signum();
                }

                self.posZ += ddz.signum()*(min_d +max_d)/2.0;

                break;
            }
        }


    }
}
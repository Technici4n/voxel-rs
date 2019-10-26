pub mod camera;
pub mod chunk;
pub mod meshing;
pub mod renderer;

use self::camera::Camera;

pub struct World {
    pub camera: Camera,
}

impl World {
    pub fn new() -> Self {
        Self {
            camera: Camera::new(),
        }
    }
}

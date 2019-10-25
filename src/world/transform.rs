use crate::settings::SETTINGS;
use nalgebra::{Matrix4, Perspective3, Vector3};

pub struct Camera {
    /// Position of the camera
    position: Vector3<f64>,
    /// Yaw in degrees
    pub yaw: f64,
    /// Yaw in degrees
    pub pitch: f64,
}

impl Camera {
    pub fn new() -> Camera {
        Self {
            position: Vector3::from([-5.0, -5.0, -5.0]),
            yaw: 100.0,
            pitch: 20.0,
        }
    }

    // TODO: Allow mouse inverting

    pub fn update_cursor(&mut self, dx: f64, dy: f64) {
        // TODO: remove this
        let mouse_speed: f64 = 0.2;
        self.yaw += mouse_speed * (dx as f64);
        self.pitch += mouse_speed * (dy as f64);

        // Ensure the pitch stays within [-90; 90]
        if self.pitch < -90.0 {
            self.pitch = -90.0;
        }
        if self.pitch > 90.0 {
            self.pitch = 90.0;
        }
    }

    // TODO: take resizing into account

    pub fn get_view_projection(&self) -> Matrix4<f64> {
        let aspect_ratio = {
            let (win_w, win_h) = SETTINGS.window_size;
            win_w as f64 / win_h as f64
        };
        // TODO: remove hardcoded constants
        let proj = Perspective3::new(aspect_ratio, (90.0f64).to_radians(), 0.1, 400.0);

        let rotation = Matrix4::from_euler_angles(-self.pitch.to_radians(), 0.0, 0.0)
            * Matrix4::from_euler_angles(0.0, -self.yaw.to_radians(), 0.0);
        let translation = Matrix4::new_translation(&-self.position);

        proj.as_matrix() * rotation * translation
    }
}

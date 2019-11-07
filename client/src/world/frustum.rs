use crate::input::YawPitch;
use nalgebra::{Matrix4, Perspective3, Vector3};

/// The player's frustum
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    /// Position of the camera
    pub position: Vector3<f64>,
    /// Yaw in degrees
    pub yaw: f64,
    /// Yaw in degrees
    pub pitch: f64,
}

impl Frustum {
    pub fn new(position: Vector3<f64>, yaw_pitch: YawPitch) -> Frustum {
        Self {
            position,
            yaw: yaw_pitch.yaw,
            pitch: yaw_pitch.pitch,
        }
    }

    pub fn get_view_projection(&self, aspect_ratio: f64) -> Matrix4<f64> {
        // TODO: remove hardcoded constants
        let proj = Perspective3::new(aspect_ratio, (60.0f64).to_radians(), 0.1, 3000.0);

        let rotation = Matrix4::from_euler_angles(-self.pitch.to_radians(), 0.0, 0.0)
            * Matrix4::from_euler_angles(0.0, -self.yaw.to_radians(), 0.0);
        let translation = Matrix4::new_translation(&-self.position);

        proj.as_matrix() * rotation * translation
    }
}

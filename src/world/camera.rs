use crate::input::InputState;
use glutin::ElementState;
use nalgebra::{Matrix4, Perspective3, Vector3};

pub const MOVE_FORWARD: u32 = 17;
pub const MOVE_LEFT: u32 = 30;
pub const MOVE_BACKWARD: u32 = 31;
pub const MOVE_RIGHT: u32 = 32;
pub const MOVE_UP: u32 = 57;
pub const MOVE_DOWN: u32 = 42;

pub struct Camera {
    /// Position of the camera
    pub position: Vector3<f64>,
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
        self.yaw -= mouse_speed * (dx as f64);
        self.pitch -= mouse_speed * (dy as f64);

        // Ensure the yaw stays within [-180; 180]
        if self.yaw < -180.0 {
            self.yaw += 360.0;
        }
        if self.yaw > 180.0 {
            self.yaw -= 360.0;
        }

        // Ensure the pitch stays within [-90; 90]
        if self.pitch < -90.0 {
            self.pitch = -90.0;
        }
        if self.pitch > 90.0 {
            self.pitch = 90.0;
        }
    }

    pub fn tick(&mut self, dt: f64, keyboard_state: &InputState) {
        const SPEED: f64 = 10.0;
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_FORWARD) {
            self.position += self.movement_direction(0.0) * (dt * SPEED) as f64;
        }
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_LEFT) {
            self.position += self.movement_direction(90.0) * (dt * SPEED) as f64;
        }
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_BACKWARD) {
            self.position += self.movement_direction(180.0) * (dt * SPEED) as f64;
        }
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_RIGHT) {
            self.position += self.movement_direction(270.0) * (dt * SPEED) as f64;
        }
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_UP) {
            self.position.y += (dt * SPEED) as f64;
        }
        if let ElementState::Pressed = keyboard_state.get_key_state(MOVE_DOWN) {
            self.position.y -= (dt * SPEED) as f64;
        }
    }

    pub fn get_view_projection(&self, aspect_ratio: f64) -> Matrix4<f64> {
        // TODO: remove hardcoded constants
        let proj = Perspective3::new(aspect_ratio, (60.0f64).to_radians(), 0.1, 1000.0);

        let rotation = Matrix4::from_euler_angles(-self.pitch.to_radians(), 0.0, 0.0)
            * Matrix4::from_euler_angles(0.0, -self.yaw.to_radians(), 0.0);
        let translation = Matrix4::new_translation(&-self.position);

        proj.as_matrix() * rotation * translation
    }

    /// Unit vector in the `angle` direction
    fn movement_direction(&self, angle: f64) -> Vector3<f64> {
        let yaw = self.yaw + angle;
        Vector3::new(-yaw.to_radians().sin(), 0.0, -yaw.to_radians().cos()).normalize()
    }
}

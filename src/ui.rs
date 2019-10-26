use crate::world::World;
use anyhow::Result;

pub mod renderer;

/// Wrapper around the ui
pub struct Ui {
    /// The text that is shown
    text: String,
}

impl Ui {
    /// Create a new ui
    pub fn new() -> Result<Self> {
        Ok(Self {
            text: String::from("Welcome to voxel-rs"),
        })
    }

    /// Handle a glutin event
    pub fn handle_event(&mut self, _event: glutin::Event, _window: &glutin::Window) {
        // TODO: remove or implement
    }

    /// Rebuild the Ui if it changed
    pub fn build_if_changed(&mut self, world: &World) {
        let camera = &world.camera;
        self.text = format!(
            "\
Welcome to voxel-rs

yaw = {:4.0}
pitch = {:4.0}

x = {:.2}
y = {:.2}
z = {:.2}
",
            camera.yaw, camera.pitch, camera.position.x, camera.position.y, camera.position.z
        );
    }

    pub fn get_text(&self) -> &str {
        &self.text
    }

    /// Should the cursor be automatically centered and hidden?
    pub fn should_hide_and_center_cursor(&self) -> bool {
        true
    }
}

use glutin::{ElementState, KeyboardInput, ModifiersState, MouseButton};
use std::collections::HashMap;
use voxel_rs_common::player::PlayerInput;

/// A helper struct to keep track of the yaw and pitch of a player
#[derive(Debug, Clone, Copy, Default)]
pub struct YawPitch {
    pub yaw: f64,
    pub pitch: f64,
}

impl YawPitch {
    // TODO: Allow mouse inverting
    pub fn update_cursor(&mut self, dx: f64, dy: f64) {
        // TODO: don't hardcode this
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
}

/// The state of the keyboard and mouse buttons.
pub struct InputState {
    keys: HashMap<u32, ElementState>,
    mouse_buttons: HashMap<MouseButton, ElementState>,
    modifiers_state: ModifiersState,
    flying: bool, // TODO: reset this on game start
}

impl InputState {
    pub fn new() -> InputState {
        Self {
            keys: HashMap::new(),
            mouse_buttons: HashMap::new(),
            modifiers_state: ModifiersState::default(),
            flying: false,
        }
    }

    /// Process a keyboard input, returning whether the state of the key changed or not
    pub fn process_keyboard_input(&mut self, input: KeyboardInput) -> bool {
        self.modifiers_state = input.modifiers;
        let previous_state = self.keys.get(&input.scancode).cloned();
        self.keys.insert(input.scancode, input.state);
        if input.scancode == TOGGLE_FLIGHT {
            if let &Some(ElementState::Pressed) = &previous_state {
                self.flying = !self.flying;
            }
        }
        previous_state != Some(input.state)
    }

    /// Process a mouse input, returning whether the state of the button changed or not
    pub fn process_mouse_input(
        &mut self,
        state: ElementState,
        button: MouseButton,
        modifiers: ModifiersState,
    ) -> bool {
        self.modifiers_state = modifiers;
        let previous_state = self.mouse_buttons.get(&button).cloned();
        self.mouse_buttons.insert(button, state);
        previous_state != Some(state)
    }

    pub fn _get_modifiers_state(&self) -> ModifiersState {
        self.modifiers_state
    }

    pub fn get_key_state(&self, scancode: u32) -> ElementState {
        self.keys
            .get(&scancode)
            .cloned()
            .unwrap_or(ElementState::Released)
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.mouse_buttons.clear();
        self.modifiers_state = ModifiersState::default();
    }

    fn is_key_pressed(&self, scancode: u32) -> bool {
        match self.get_key_state(scancode) {
            ElementState::Pressed => true,
            ElementState::Released => false,
        }
    }

    // TODO: add configuration for this
    pub fn get_physics_input(&self, yaw_pitch: YawPitch, allow_movement: bool) -> PlayerInput {
        PlayerInput {
            key_move_forward: allow_movement && self.is_key_pressed(MOVE_FORWARD),
            key_move_left: allow_movement && self.is_key_pressed(MOVE_LEFT),
            key_move_backward: allow_movement && self.is_key_pressed(MOVE_BACKWARD),
            key_move_right: allow_movement && self.is_key_pressed(MOVE_RIGHT),
            key_move_up: allow_movement && self.is_key_pressed(MOVE_UP),
            key_move_down: allow_movement && self.is_key_pressed(MOVE_DOWN),
            yaw: yaw_pitch.yaw,
            pitch: yaw_pitch.pitch,
            flying: self.flying,
        }
    }
}

pub const MOVE_FORWARD: u32 = 17;
pub const MOVE_LEFT: u32 = 30;
pub const MOVE_BACKWARD: u32 = 31;
pub const MOVE_RIGHT: u32 = 32;
pub const MOVE_UP: u32 = 57;
pub const MOVE_DOWN: u32 = 42;
pub const TOGGLE_FLIGHT: u32 = 33;

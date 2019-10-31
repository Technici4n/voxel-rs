use glutin::{ElementState, KeyboardInput, ModifiersState};
use std::collections::HashMap;

pub struct KeyboardState {
    keys: HashMap<u32, ElementState>,
    modifiers_state: ModifiersState,
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        Self {
            keys: HashMap::new(),
            modifiers_state: ModifiersState::default(),
        }
    }

    pub fn process_input(&mut self, input: KeyboardInput) {
        self.modifiers_state = input.modifiers;
        self.keys.insert(input.scancode, input.state);
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
        self.modifiers_state = ModifiersState::default();
    }
}

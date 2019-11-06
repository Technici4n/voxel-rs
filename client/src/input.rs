use glutin::{ElementState, KeyboardInput, ModifiersState, MouseButton};
use std::collections::HashMap;

pub struct InputState {
    keys: HashMap<u32, ElementState>,
    mouse_buttons: HashMap<MouseButton, ElementState>,
    modifiers_state: ModifiersState,
}

impl InputState {
    pub fn new() -> InputState {
        Self {
            keys: HashMap::new(),
            mouse_buttons: HashMap::new(),
            modifiers_state: ModifiersState::default(),
        }
    }

    /// Process a keyboard input, returning whether the state of the key changed or not
    pub fn process_keyboard_input(&mut self, input: KeyboardInput) -> bool {
        self.modifiers_state = input.modifiers;
        let previous_state = self.keys.get(&input.scancode).cloned();
        self.keys.insert(input.scancode, input.state);
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
}

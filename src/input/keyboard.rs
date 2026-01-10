use std::collections::HashMap;

use winit::keyboard::KeyCode;

#[derive(Debug)]
pub struct Keyboard {
    states: HashMap<KeyCode, KeyState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// The key is up. e.g. not pressed.
    Up,
    /// The key was pressed this frame.
    Pressed,
    /// The key was released this frame.
    Released,
    /// The key is being held down.
    /// Specifically, the key was pressed in a previous frame and has not been released yet.
    Held,
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn set_key_state(&mut self, key: KeyCode, state: KeyState) {
        self.states.insert(key, state);
    }

    pub fn get_key_state(&self, key: KeyCode) -> Option<KeyState> {
        self.states.get(&key).copied()
    }

    /// Returns true if the key was pressed this frame.
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        matches!(self.get_key_state(key), Some(KeyState::Pressed))
    }

    /// Returns true if the key is currently being held down.
    pub fn is_key_held(&self, key: KeyCode) -> bool {
        matches!(self.get_key_state(key), Some(KeyState::Held))
    }

    pub fn press_key(&mut self, key: KeyCode) {
        self.set_key_state(key, KeyState::Pressed);
    }

    pub fn release_key(&mut self, key: KeyCode) {
        self.set_key_state(key, KeyState::Released);
    }

    pub fn update_keys(&mut self) {
        for state in self.states.values_mut() {
            if *state == KeyState::Pressed {
                *state = KeyState::Held;
            } else if *state == KeyState::Released {
                *state = KeyState::Up;
            }
        }
    }
}

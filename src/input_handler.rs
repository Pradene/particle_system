use {std::collections::HashSet, winit::keyboard::KeyCode};

pub struct InputHandler {
    keys: HashSet<KeyCode>,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            keys: HashSet::new(),
        }
    }

    pub fn set_key(&mut self, key: KeyCode, pressed: bool) {
        if pressed {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys.contains(&key)
    }
}

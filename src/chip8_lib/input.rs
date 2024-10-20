#[derive(PartialEq, Eq)]
pub enum KeyStatus {
    Pressed,
    Unpressed,
}

#[derive(Default)]
pub struct InputController {
    // Bit flag representing the state of keys '0' (0x01) - 'F' (0x80)
    // Set bit means pressed, unset not pressed
    key_state: u16,
}

impl InputController {
    // Checks whether numerical key from 0-F is pressed
    // Assumes key is max 4 bits long
    pub fn key_pressed(&self, key: u8) -> bool {
        (self.key_state & (1 << key)) > 0
    }
    pub fn press_key(&mut self, key: u8) {
        self.key_state |= 1 << key;
    }
    pub fn unpress_key(&mut self, key: u8) {
        self.key_state ^= 1 << key;
    }
    pub fn keys(&self) -> u16 {
        self.key_state
    }
    pub fn update_key(&mut self, key: u8, state: &KeyStatus) {
        match state {
            KeyStatus::Pressed => self.press_key(key),
            KeyStatus::Unpressed => self.unpress_key(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_pressed() {
        let ict = InputController { key_state: 0xAAAA };
        assert!(!ict.key_pressed(0x0));
        assert!(ict.key_pressed(0x1));
        assert!(!ict.key_pressed(0x2));
        assert!(ict.key_pressed(0x3));
        assert!(!ict.key_pressed(0x4));
        assert!(ict.key_pressed(0x5));
        assert!(!ict.key_pressed(0x6));
        assert!(ict.key_pressed(0x7));
        assert!(!ict.key_pressed(0x8));
        assert!(ict.key_pressed(0x9));
        assert!(!ict.key_pressed(0xA));
        assert!(ict.key_pressed(0xB));
        assert!(!ict.key_pressed(0xC));
        assert!(ict.key_pressed(0xD));
        assert!(!ict.key_pressed(0xE));
        assert!(ict.key_pressed(0xF));
    }

    #[test]
    fn press_unpress_key() {
        let mut ict = InputController::default();
        ict.press_key(0xA);
        assert!(ict.key_pressed(0xA));
        ict.unpress_key(0xA);
        assert!(!ict.key_pressed(0xA));
    }
}

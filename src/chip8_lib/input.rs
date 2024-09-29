#[derive(Default)]
pub struct InputController {
    // Bit flag representing the state of keys '0' (0x01) - 'F' (0x80)
    // Set bit means pressed, unset not pressed
    keys: u16,
}

impl InputController {
    // Checks whether numerical key from 0-F is pressed
    // Assumes key is max 4 bits long
    pub fn key_pressed(&self, key: u8) -> bool {
        (self.keys & (1 << key)) > 0
    }
    pub fn press_key(&mut self, key: u8) {
        self.keys |= 1 << key;
    }
    pub fn unpress_key(&mut self, key: u8) {
        self.keys ^= 1 << key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_pressed() {
        let ict = InputController { keys: 0xAAAA };
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

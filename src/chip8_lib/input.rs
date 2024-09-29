use std::collections::HashMap;

enum KeyState {
    Pressed,
    NotPressed
}

pub struct InputController {
    keys: HashMap::<char, KeyState>,
}

impl Default for InputController {
    fn default() -> Self {
        Self {
            keys: HashMap::<char, KeyState>::from([
                ('1', KeyState::NotPressed),
                ('2', KeyState::NotPressed),
                ('3', KeyState::NotPressed),
                ('4', KeyState::NotPressed),
                ('5', KeyState::NotPressed),
                ('6', KeyState::NotPressed),
                ('7', KeyState::NotPressed),
                ('8', KeyState::NotPressed),
                ('9', KeyState::NotPressed),
                ('0', KeyState::NotPressed),
                ('A', KeyState::NotPressed),
                ('B', KeyState::NotPressed),
                ('C', KeyState::NotPressed),
                ('D', KeyState::NotPressed),
                ('E', KeyState::NotPressed),
                ('F', KeyState::NotPressed),
            ])
        }
    }
}
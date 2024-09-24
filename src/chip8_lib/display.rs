pub const X_RES: usize = 64;
pub const Y_RES: usize = 32;

pub struct DisplayController {
    frame_buffer: [u8; X_RES * Y_RES],
}

impl Default for DisplayController {
    fn default() -> Self {
        Self {
            frame_buffer : [0; X_RES * Y_RES],
        }
    }
}

impl DisplayController {
    pub fn clear_screen(&mut self) {
        for i in self.frame_buffer {
            self.frame_buffer[i as usize] = 0;
        }
    }
}
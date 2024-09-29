pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;
const NUM_COLS: usize = SCREEN_WIDTH / 8;
const NUM_ROWS: usize = SCREEN_HEIGHT / 8;
const PIXEL_COUNT: usize = NUM_COLS * NUM_ROWS;

pub struct DisplayController {
    frame_buffer: [u8; PIXEL_COUNT],
}

enum Direction {
    Left,
    Right,
}

impl Default for DisplayController {
    fn default() -> Self {
        Self {
            frame_buffer: [0; NUM_COLS * NUM_ROWS],
        }
    }
}

impl DisplayController {
    pub fn clear_screen(&mut self) {
        for i in self.frame_buffer {
            self.frame_buffer[i as usize] = 0;
        }
    }

    // Return the index in frame_buffer of the given x and y coordinates
    fn get_idx(&self, x: usize, y: usize) -> usize {
        (y * NUM_COLS + x) / 8
    }

    // XOR byte1 with byte2, retaining bits of byte1 either left or right of offset.
    // 'side' parameter refers to direction which is subject to XOR.
    // Returns resulting byte as u8
    fn xor_side_from_offset(&self, byte1: u8, byte2: u8, offset: u8, side: Direction) -> u8 {
        let save_mask: u8;
        let mut ret: u8;
        // Create a mask to retain bits right or left of offset
        match side {
            Direction::Left => {
                save_mask = 0xFF >> offset;
                ret = byte1 ^ (byte2 << (8 - offset));
            },
            Direction::Right => {
                save_mask = 0xFF << (8 - offset);
                ret = byte1 ^ (byte2 >> offset);
            },
        }
        // Restore saved bits
        let save_bits: u8 = byte1 & save_mask;
        ret &= !save_mask;
        ret += save_bits;
        ret
    }

    // Returns true if a bit in byte1 has been unset in byte2
    fn bit_unset(&self, byte1: u8, byte2: u8) -> bool {
        for j in 0..8 {
            // Original bit was 0 anyway, so cannot be unset
            if (1 << j) & byte1 == 0 {
                continue;
            }
            // Is frame buffer bit now at 0?
            if (1 << j) & byte2 == 0 {
                return true;
            }
        }
        false
    }

    // Copy the given sprite to the frame buffer, starting from position (x, y)
    // If sprite is outside bounds of display, wrap it around.
    // If any pixel goes from 1 to 0, set Vf to 1. Else, 0.
    // Returns value of Vf.
    pub fn draw(&mut self, start_x: usize, start_y: usize, sprite: Vec<u8>) -> u8 {
        assert!(start_x < SCREEN_WIDTH && start_y < SCREEN_HEIGHT);
        let mut collision = false;
        // Check if x will wrap to next byte in frame_buffer
        // if it does, do XOR in two steps
        let x_offset = (start_x % 8) as u8;
        if x_offset != 0 {
            // Start with first frame_buffer chunk, i.e. left side of sprite
            for (i, &s_byte) in sprite.iter().enumerate() {
                let y = (start_y + i) % SCREEN_HEIGHT; 
                let chunk_idx: usize = self.get_idx(start_x, y);
                let orig_chunk: u8 = self.frame_buffer[chunk_idx];
                self.frame_buffer[chunk_idx] = self.xor_side_from_offset(orig_chunk, s_byte, x_offset, Direction::Right);
                // Check if bit was unset
                if !collision {
                    collision = self.bit_unset(orig_chunk, self.frame_buffer[chunk_idx]);
                }
            }
            // Blit second frame_buffer chunk, i.e. right side of sprite
            for (i, &s_byte) in sprite.iter().enumerate() {
                let y = (start_y + i) % SCREEN_HEIGHT; 
                let chunk_idx: usize = self.get_idx(start_x + (8 - x_offset as usize), y);
                let orig_chunk: u8 = self.frame_buffer[chunk_idx];
                self.frame_buffer[chunk_idx] = self.xor_side_from_offset(orig_chunk, s_byte, x_offset, Direction::Left);
                // Check if bit was unset
                if !collision {
                    collision = self.bit_unset(orig_chunk, self.frame_buffer[chunk_idx]);
                }
            }
        }
        // Else, simply XOR the sprite onto the frame buffer
        else {
            // For each row (y)
            for (i, s_byte) in sprite.iter().enumerate() {
                let y = (start_y + i) % SCREEN_HEIGHT; 
                // Index of current chunk of frame buffer to be XORed
                let chunk_idx: usize = self.get_idx(start_x, y);
                let orig_chunk: u8 = self.frame_buffer[chunk_idx];
                self.frame_buffer[chunk_idx] ^= s_byte;
                // For each pixel in row, check if bit was unset
                if !collision {
                    collision = self.bit_unset(orig_chunk, self.frame_buffer[chunk_idx]);
                }
                
            }
        }
        match collision {
            true => 1,
            false => 0,
        }
    }
}

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::cpu::FONT;

        // Draw a sprite to frame buffer that evenly fits into a single byte
        #[test]
        fn draw_even() {
            let mut dct = DisplayController::default();
            let chunk_idx: usize = dct.get_idx(0, 0);
            // '0'
            let sprite: Vec<u8> = Vec::from(&FONT[0..5]);
            let vf = dct.draw(0, 0, sprite);
            // Since frame buffer starts zeroed, there can be no collisions
            assert_eq!(vf, 0);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 0)], 0xF0);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 1)], 0x90);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 2)], 0x90);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 3)], 0x90);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 4)], 0xF0);
        }

        // Draw a sprite to frame buffer that overflows into a second byte
        #[test]
        fn draw_offset() {
            let mut dct = DisplayController::default();
            // '0'
            let sprite: Vec<u8> = Vec::from(&FONT[0..5]);
            let vf = dct.draw(1, 0, sprite);
            // Since frame buffer starts zeroed, there can be no collisions
            assert_eq!(vf, 0);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 0)], 0x78);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 1)], 0x48);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 2)], 0x48);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 3)], 0x48);
            assert_eq!(dct.frame_buffer[dct.get_idx(0, 4)], 0x78);
        }

        // Draw a sprite to frame buffer that collides with a set pixel
        #[test]
        fn draw_collision() {
            let mut dct = DisplayController::default();
            // '0'
            let sprite: Vec<u8> = Vec::from(&FONT[0..5]);
            _ = dct.draw(1, 0, sprite);
            let sprite: Vec<u8> = Vec::from(&FONT[0..5]);
            let vf = dct.draw(1, 0, sprite);
            // Since two sprites with identical properties were blitted to the same coordinates,
            // there was a collision and Vf must be 1.
            assert_eq!(vf, 1);
        }
    }

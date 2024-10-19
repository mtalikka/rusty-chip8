use chip8_lib::display::{SCREEN_HEIGHT, SCREEN_WIDTH};
use sdl2::pixels::Color;

// Simulated pixel grid resolution
pub const GRID_SIZE: (usize, usize) = (SCREEN_WIDTH, SCREEN_HEIGHT);
// Size of each pixel
pub const GRID_CELL_SIZE: (u32, u32) = (16, 16);
// True resolution
pub const SCREEN_SIZE: (u32, u32) = (
    GRID_SIZE.0 as u32 * GRID_CELL_SIZE.0,
    GRID_SIZE.1 as u32 * GRID_CELL_SIZE.1,
);
pub const RENDER_FPS: u32 = 60;
pub const BG_COLOR: Color = Color::BLACK;
pub const FG_COLOR: Color = Color::GREEN;

use ggez::graphics::Color;
use chip8_lib::display;

// Simulated pixel grid resolution
pub const GRID_SIZE: (usize, usize) = (display::X_RES, display::Y_RES);
// Size of each pixel
pub const GRID_CELL_SIZE: (u32, u32) = (3, 3);
// True resolution
pub const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);
pub const RENDER_FPS: u32 = 60;
pub const BG_COLOR: Color = Color::BLACK;
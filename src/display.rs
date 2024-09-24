use ggez::{ContextBuilder, Context, event::EventLoop, conf, event};



// Simulated pixel grid resolution
pub const GRID_SIZE: (usize, usize) = (64, 32);
// Size of each pixel
pub const GRID_CELL_SIZE: (u32, u32) = (3, 3);
// True resolution
pub const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);
pub const DESIRED_FPS: u32 = 60;

pub struct DisplayController {
    frame_buffer: [u8; GRID_SIZE.0 * GRID_SIZE.1],
    ctx: Context,
    event_loop: EventLoop<()>,
    state: State,
}

impl DisplayController {
    pub fn new() -> Self {
        let (ctx_tmp, event_loop_tmp) = ContextBuilder::new("rusty-chip8", "Mikko")
            .window_setup(conf::WindowSetup::default().title("rusty-chip8"))
            .window_mode(conf::WindowMode::default().dimensions(SCREEN_SIZE.0+1., SCREEN_SIZE.1+1.))
            .build()
            .unwrap();
        let state_tmp = State::new(&mut ctx_tmp);
        Self {
            frame_buffer : [0; GRID_SIZE.0 * GRID_SIZE.1],
            ctx : ctx_tmp,
            event_loop : event_loop_tmp,
            state : state_tmp,
        }
    }

    pub fn start_event_loop(&mut self) {
        event::run(self.ctx, self.event_loop, self.state);
    }

    pub fn clear_screen(&mut self) {
        for i in self.frame_buffer {
            self.frame_buffer[i as usize] = 0;
        }
    }
}

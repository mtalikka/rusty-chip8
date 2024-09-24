use ggez::{
    conf,
    error::{GameError, GameResult},
    event::{self, EventHandler, EventLoop},
    graphics,
    input::{
        gamepad::Event,
        keyboard::{KeyCode, KeyInput},
    },
    Context, ContextBuilder,
};

use crate::screen::*;

/// Contains information about the current state of the emulator, e.g. frame buffer
pub struct State {
    frame_buffer: [u8; GRID_SIZE.0 * GRID_SIZE.1],
    emulation_paused: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            frame_buffer: [0; GRID_SIZE.0 * GRID_SIZE.1],
            emulation_paused: false,
        }
    }
}

impl State {
    fn update_state(&mut self) {}
}

impl EventHandler<GameError> for State {
    #[inline]
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        while ctx.time.check_update_time(RENDER_FPS) {
            if !self.emulation_paused {
                self.update_state();
            }
        }
        Ok(())
    }

    #[inline]
    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let canvas = graphics::Canvas::from_frame(ctx, BG_COLOR);
        canvas.finish(ctx)?;
        ggez::timer::yield_now();
        Ok(())
    }

    #[inline]
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: KeyInput,
        _repeat: bool,
    ) -> std::result::Result<(), ggez::GameError> {
        match input.keycode {
            Some(KeyCode::Space) => {
                self.emulation_paused = match self.emulation_paused {
                    true => false,
                    false => true,
                };
            }
            Some(KeyCode::Escape) => _ctx.request_quit(),
            _ => (),
        }
        Ok(())
    }
}

pub struct EventController {
    ctx: Context,
    event_loop: EventLoop<()>,
    state: State,
}

impl Default for EventController {
    fn default() -> Self {
        let (ctx_tmp, event_loop_tmp) = ContextBuilder::new("rusty-chip8", "Mikko")
            .window_setup(conf::WindowSetup::default().title("rusty-chip8"))
            .window_mode(
                conf::WindowMode::default().dimensions(SCREEN_SIZE.0 + 1., SCREEN_SIZE.1 + 1.),
            )
            .build()
            .unwrap();
        let state_tmp = State::default();
        Self {
            ctx: ctx_tmp,
            event_loop: event_loop_tmp,
            state: state_tmp,
        }
    }
}

impl EventController {
    pub fn start_event_loop(self) {
        event::run(self.ctx, self.event_loop, self.state);
    }
}

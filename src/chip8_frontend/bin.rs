mod screen;

use crate::screen::GRID_CELL_SIZE;
use chip8_lib::chip8::Chip8;
use chip8_lib::display::PIXEL_COUNT;
use chip8_lib::input::InputController;
use chip8_lib::config::Cfg;
use sdl2::event::Event;
use sdl2::render::TextureAccess;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::{self,Sender, Receiver};
use log::warn;

const CFG_FILE_PATH: &str = "../../cfg/config.ini";

fn main() -> Result<(), String> {
    env_logger::init();
    // Backend will run in its own separate thread, reacting to keypresses sent by message from
    // the main thread (SDL2 context). Backend will send frame buffer to frontend in similar way.
    let mut chip8 = Chip8::default();
    chip8.load_config(CFG_FILE_PATH);
    let (input_tx, input_rx): (Sender<u16>, Receiver<u16>) = mpsc::channel();
    let (display_tx, display_rx): (Sender<[u8; PIXEL_COUNT]>, Receiver<[u8; PIXEL_COUNT]>) = mpsc::channel();
    let (quit_tx, quit_rx): (Sender<bool>, Receiver<bool>) = mpsc::channel();

    thread::spawn(move || {
        chip8.connect(input_rx, quit_rx, display_tx);
        chip8.main_loop();
    });

    let mut current_keyboard_state = InputController::default();

    println!("Initializing SDL2 context...");
    let sdl_context = sdl2::init()?;
    let conf = Cfg::default();
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("CHIP-8", screen::SCREEN_SIZE.0, screen::SCREEN_SIZE.1)
        .position_centered()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas().build().unwrap();
    canvas.set_draw_color(screen::BG_COLOR);
    canvas.clear();
    canvas.present();
    let texture_creator = canvas.texture_creator();
    let fg_texture = &texture_creator.create_texture(None, TextureAccess::Static, GRID_CELL_SIZE.0, GRID_CELL_SIZE.1);
    let bg_texture = &texture_creator.create_texture(None, TextureAccess::Static, GRID_CELL_SIZE.0, GRID_CELL_SIZE.1);
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        // Handle input
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => 
                    {
                        if let Err(e) = quit_tx.send(true) {
                            warn!("Failed to send quit message to backend: {e}");
                        };
                        break 'running;
                    },
                // If a key is pressed, see if it corresponds to a key in the layout defind in config,
                // then update internal keyboard state
                Event::KeyDown {keycode: k, ..} 
                => {
                    let send = &conf.get_u8_from_keycode(&k.unwrap());
                    match send {
                        Some(val) => { current_keyboard_state.press_key(*val)}
                        None => {}
                    }
                },
                Event::KeyUp {keycode: k, ..} 
                => {
                    let send = &conf.get_u8_from_keycode(&k.unwrap());
                    match send {
                        Some(val) => { current_keyboard_state.unpress_key(*val)}
                        None => {}
                    }
                }
                _ => {}
            }
        }

        // TODO: Draw the screen from frame buffer

        if let Err(e) = input_tx.send(current_keyboard_state.keys()) {
            warn!("Failed to send keyboard state to backend: {e}");
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}

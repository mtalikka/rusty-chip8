mod screen;
mod config;

use chip8_lib::cpu::Cpu;
use crate::config::Cfg;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use std::thread;
use std::sync::Mutex;

fn main() -> Result<(), String> {
    println!("Initializing SDL2 context...");
    let sdl_context = sdl2::init()?;
    let cpu = Cpu::default();
    let conf = Cfg::default();
    let keyboard_state = Mutex::new(&cpu.ict);
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
    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => 
                    { break 'running },
                Event::KeyDown {keycode: k, ..} 
                => {
                    match k {
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}

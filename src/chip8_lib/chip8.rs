
use crate::cpu::Cpu;
use crate::config::Cfg;
use crate::display::PIXEL_COUNT;
use std::sync::mpsc::{Sender, Receiver};
use std::time::{Duration, SystemTime, Instant};
use log::warn;

#[derive(Default)]
pub struct Chip8 {
    cpu: Cpu,
    config: Cfg,
    // Receiver which updates input controller from main thread
    input_receiver: Option<Receiver<u16>>,
    // Receiver which receives message to quit from main thread
    quit_receiver: Option<Receiver<bool>>,
    // Transmitter which sends frame buffer state
    display_transmitter: Option<Sender<[u8; PIXEL_COUNT]>>,
}


impl Chip8 {
    pub fn new() -> Self {
        Self {
            cpu: Cpu::default(),
            config: Cfg::default(),
            input_receiver: None,
            quit_receiver: None,
            display_transmitter: None,
        }
    }

    pub fn load_config(&mut self, filename: &str) -> &mut Self {
        self.config.load_config(filename);
        self
    }

    pub fn connect(&mut self, input_rx: Receiver<u16>, quit_rx: Receiver<bool>, display_tx: Sender<[u8; PIXEL_COUNT]>) -> &mut Self {
        self.input_receiver = Some(input_rx);
        self.quit_receiver = Some(quit_rx);
        self.display_transmitter = Some(display_tx);
        self
    }

    pub fn main_loop(&mut self) {
        'main: loop {
            // Check for new keyboard state from main thread
            match &self.input_receiver {
                Some(rx) =>  {
                    if let Ok(val) = rx.try_recv() {self.cpu.ict.update_keys(val)}
                },
                // Interpreter has not been connected with main thread
                None => {
                    warn!("Warning: input_receiver has not been connected with main thread.")
                }
            }

            // Check for quit message from main thread
            match &self.quit_receiver {
                Some(rx) =>  {
                    if rx.try_recv().is_ok() {break 'main}
                },
                None => {
                    warn!("Warning: quit_receiver has not been connected with main thread.")
                }
            }
            let start = Instant::now();

            let end = Instant::now();
        }
    }
}
mod cpu;
mod display;

use crate::cpu::Cpu;
use crate::display::DisplayController;

fn main() {
    let mut dct = DisplayController::new();
    let cpu = Cpu::new(dct);
    dct.start_event_loop();
}

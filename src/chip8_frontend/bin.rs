pub mod screen;
pub mod state;

use chip8_lib::cpu::Cpu;
use crate::state::EventController;

fn main() {
    let cpu = Cpu::default();
    let evc = EventController::default();
    evc.start_event_loop();
}

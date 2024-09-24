pub mod screen;
pub mod state;

use crate::state::EventController;
use chip8_lib::cpu::Cpu;

fn main() {
    let cpu = Cpu::default();
    let evc = EventController::default();
    evc.start_event_loop();
}

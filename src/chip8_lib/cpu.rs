use thiserror::Error;

use crate::display;

// Memory address from where the font is stored; by convention this is 0x50
const FONT_START_ADDR: usize = 0x50;

const FONT: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

// Error handling
#[derive(Error, Debug)]
pub enum CpuError {
    #[error("encountered unknown opcode")]
    UnknownOpcodeError,
}

pub struct Cpu {
    // Program counter
    pc: u16,
    // Stack pointer
    sp: i16,
    // Delay timer
    dt: u8,
    // Sound timer
    st: u8,
    // Index register
    i: u16,
    // General purpose registers
    reg: [u8; 16],
    // Memory; 4kB
    mem: [u8; 4096],
    // Stack; holds maximum of 16 addresses
    stk: Vec<u16>,
    // Display controller
    dct: display::DisplayController,
}

impl Default for Cpu {
    fn default() -> Self {
        let mut ret = Self {
            pc : 0,
            sp : 0,
            dt : 0,
            st : 0,
            i : 0,
            reg : [0; 16],
            mem : [0; 4096],
            stk: vec![],
            dct: display::DisplayController::default(),
        };
        // Map font to memory
        for i in FONT_START_ADDR..FONT.len() {
            ret.mem[i] = FONT[i-FONT_START_ADDR];
        }
        ret
    }
}

impl Cpu {
    /// Run the current instruction pointed to by PC
    pub fn exec_routine(&mut self) -> Result<(), CpuError> {
        match self.mem[self.pc as usize] {
            0x00E0 => Ok(self.cls()),
            0x00EE => Ok(self.ret()),
            ..u8::MAX => Err(CpuError::UnknownOpcodeError),
            u8::MAX => Err(CpuError::UnknownOpcodeError),
        }
    }

    /// Opcode 0x00E0   -   CLS
    /// 
    /// Clears the screen
    fn cls(&mut self) {
        self.dct.clear_screen();
    }

    /// Opcode 0x00EE   -   RET
    /// 
    /// The interpreter sets the program counter to the address at the top of the stack,
    /// then subtracts 1 from the stack pointer.
    fn ret(&mut self) {
        self.sp -= 1;
    }
}
use thiserror::Error;

use crate::display;

const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
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
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

// Error handling
#[derive(Error, Debug)]
pub enum CpuError {
    #[error("encountered unknown opcode")]
    UnknownOpcode,
    #[error("attempted to pop from empty stack")]
    EmptyStack,
    #[error("attempted to increment program counter beyond memory constraints")]
    MemoryOutOfBounds,
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
    reg: [u8; REGISTER_COUNT],
    // Memory; 4kB
    mem: [u8; MEMORY_SIZE],
    // Stack; holds maximum of 16 addresses
    stk: Vec<u16>,
    // Display controller
    dct: display::DisplayController,
}

impl Default for Cpu {
    fn default() -> Self {
        let mut ret = Self {
            pc: 0,
            sp: 0,
            dt: 0,
            st: 0,
            i: 0,
            reg: [0; REGISTER_COUNT],
            mem: [0; MEMORY_SIZE],
            stk: vec![],
            dct: display::DisplayController::default(),
        };
        // Map font to memory
        for i in FONT_START_ADDR..FONT.len() {
            ret.mem[i] = FONT[i - FONT_START_ADDR];
        }
        ret
    }
}

impl Cpu {
    /// Run the current instruction pointed to by PC
    pub fn exec_routine(&mut self) -> Result<(), CpuError> {
        let result: Result<(), CpuError>;
        // Pack two contiguous 8-bit segments in memory into 16-bit instruction
        let mut instruction: u16 = self.mem[self.pc as usize] as u16;
        instruction <<= 8;
        instruction |= self.mem[self.pc as usize + 1] as u16;
        match instruction {
            0x00E0 => { result = self.cls() },
            0x00EE => { result = self.ret() },

            ..u16::MAX => return Err(CpuError::UnknownOpcode),
            u16::MAX => return Err(CpuError::UnknownOpcode),
        }
        // Advance program counter by 16 bits
        self.pc += 2;
        if self.pc >= MEMORY_SIZE as u16 {return Err(CpuError::MemoryOutOfBounds) }
        result
    }

    /// Opcode 0x00E0 - CLS
    ///
    /// Clears the screen
    fn cls(&mut self) -> Result<(), CpuError> {
        self.dct.clear_screen();
        Ok(())
    }

    /// Opcode 0x00EE - RET
    ///
    /// The interpreter sets the program counter to the address at the top of the stack,
    /// then subtracts 1 from the stack pointer.
    fn ret(&mut self) -> Result<(), CpuError> {
        match self.stk.pop() {
            Some(val) => self.pc = val,
            None => return Err(CpuError::EmptyStack),
        }
        self.sp -= 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Executing a known opcode loaded to address 0x0000
    #[test]
    fn exec_routine_success() {
        let mut c = Cpu::default();
        c.mem[0] = 0x00;
        c.mem[1] = 0xE0;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2, "testing incrementation of program counter");
    }

    // Executing an unknown opcode loaded to address 0x0000
    #[test]
    #[should_panic]
    fn exec_routine_failure() {
        let mut c = Cpu::default();
        c.mem[0] = 0xFF;
        c.mem[1] = 0xFF;
        c.exec_routine().unwrap();
    }

    // Executing an unknown opcode loaded to address 0x0000
    #[test]
    #[should_panic]
    fn exec_routine_out_of_memory() {
        let mut c = Cpu::default();
        c.pc = 4094;
        c.mem[4094] = 0x00;
        c.mem[4095] = 0xE0;
        c.exec_routine().unwrap();
    }
}
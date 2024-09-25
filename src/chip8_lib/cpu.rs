use thiserror::Error;

use crate::display;

const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
// Maximum 16 nested subroutines
const STACK_SIZE: usize = 16;
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
    #[error("stack nesting limit exceeded")]
    StackOverflow,
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
    // Memory space; maximum 4KB
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
            0x1000..0x1FFF => { result = self.jp(instruction) },
            0x2000..0x2FFF => { result = self.call(instruction) },
            ..u16::MAX => return Err(CpuError::UnknownOpcode),
            u16::MAX => return Err(CpuError::UnknownOpcode),
        }
        result
    }

    // Advance program counter by 16 bits
    // Constraints: PC must not be greater 4096, as this exceeds the memory limit of 4KB.
    fn increment_pc(&mut self) -> Result<(), CpuError> {
        self.pc += 2;
        if self.pc >= MEMORY_SIZE as u16 {return Err(CpuError::MemoryOutOfBounds)}
        Ok(())
    }

    // Increment stack pointer by 1
    // Constraints: SP must not exceed 15, because only 16 nested subroutines are allowed.
    fn increment_sp(&mut self) -> Result<(), CpuError> {
        self.sp += 1;
        if self.sp >= STACK_SIZE as i16 {return Err(CpuError::StackOverflow)}
        Ok(())
    }

    /// Opcode 0x00E0 - CLS
    ///
    /// Clears the screen.
    fn cls(&mut self) -> Result<(), CpuError> {
        self.dct.clear_screen();
        self.increment_pc()
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

    /// Opcode 0x1nnn - JP
    ///
    /// The interpreter sets the program counter to nnn.
    fn jp(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.pc = addr;
        Ok(())
    }

    /// Opcode 0x2nnn - CALL
    /// 
    /// Call subroutine at nnn.
    ///
    /// The interpreter increments the stack pointer, then puts the current PC on the top of the stack.
    /// PC is then set to nnn.
    fn call(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.increment_sp().unwrap();
        self.stk.push(self.pc);
        self.pc = addr;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Execute a known opcode loaded to address 0x0000
    #[test]
    fn exec_routine_success() {
        let mut c = Cpu::default();
        c.mem[0] = 0x00;
        c.mem[1] = 0xE0;
        c.exec_routine().expect("exec_routine failed");
    }

    // Execute an unknown opcodeloaded to address 0x0000
    #[test]
    #[should_panic]
    fn exec_routine_failure() {
        let mut c = Cpu::default();
        c.mem[0] = 0xFF;
        c.mem[1] = 0xFF;
        c.exec_routine().unwrap();
    }

    // Execute a known opcode loaded to address 0xFFE,
    // causing program counter to increment beyond available memory
    #[test]
    #[should_panic]
    fn exec_routine_out_of_memory() {
        let mut c = Cpu { pc: 4094, ..Default::default()};
        c.mem[4094] = 0x00;
        c.mem[4095] = 0xE0;
        c.exec_routine().unwrap();
    }

    // Execute the jp instruction 
    #[test]
    fn exec_routine_jp() {
        let mut c = Cpu::default();
        c.mem[0] = 0x1B;
        c.mem[1] = 0xEE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 0xBEE, "testing of jp instruction");
    }

    // Execute the call instruction 
    #[test]
    fn exec_routine_call() {
        let mut c = Cpu::default();
        c.mem[0] = 0x2B;
        c.mem[1] = 0xEE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.stk.pop(), Some(0), "testing if PC has been saved on stack");
        assert_eq!(c.pc, 0xBEE, "testing if new PC has been set");
    }
}
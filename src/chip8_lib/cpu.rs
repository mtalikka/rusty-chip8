use thiserror::Error;

use crate::{display, input};

const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
// Maximum 16 nested subroutines
const STACK_SIZE: usize = 16;
// Memory address from where the font is stored; by convention this is 0x50
pub const FONT_START_ADDR: usize = 0x50;

pub const FONT: [u8; 80] = [
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
    #[error("attempted to access a register which does not exist")]
    InvalidRegister,
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
    // Input controller
    ict: input::InputController,
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
            ict: input::InputController::default(),
        };
        // Map font to memory
        for i in FONT_START_ADDR..FONT_START_ADDR + FONT.len() {
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
        let mut inst: u16 = self.mem[self.pc as usize] as u16;
        inst <<= 8;
        inst |= self.mem[self.pc as usize + 1] as u16;
        match inst {
            0x00E0 => result = self.cls(),
            0x00EE => result = self.ret(),
            0x1000..0x1FFF => result = self.jp(inst),
            0x2000..0x2FFF => result = self.call(inst),
            0x3000..0x3FFF => result = self.sexb(inst),
            0x4000..0x4FFF => result = self.snexb(inst),
            0x5000..0x5FFF => {
                if inst & 0x000F != 0 {
                    return Err(CpuError::UnknownOpcode);
                };
                result = self.sexy(inst);
            }
            0x6000..0x6FFF => result = self.ldxb(inst),
            0x7000..0x7FFF => result = self.addxb(inst),
            0x8000..0x8FFF => match inst & 0x000F {
                0x0 => result = self.ldxy(inst),
                0x1 => result = self.orxy(inst),
                0x2 => result = self.andxy(inst),
                0x3 => result = self.xorxy(inst),
                0x4 => result = self.addxy(inst),
                0x5 => result = self.subxy(inst),
                0x6 => result = self.shrx(inst),
                0x7 => result = self.subnxy(inst),
                0xE => result = self.shlx(inst),
                _ => return Err(CpuError::UnknownOpcode),
            },
            0x9000..0x9FFF => {
                if inst & 0x000F != 0 {
                    return Err(CpuError::UnknownOpcode);
                };
                result = self.snexy(inst);
            }
            0xA000..0xAFFF => result = self.ldi(inst),
            0xB000..0xBFFF => result = self.jp0(inst),
            0xC000..0xCFFF => result = self.rndx(inst),
            0xD000..0xDFFF => result = self.drwxy(inst),
            0xE000..0xEFFF => match inst & 0x00FF {
                0x009E => result = self.skpx(inst),
                0x00A1 => result = self.sknpx(inst),
                _ => return Err(CpuError::UnknownOpcode),
            },

            ..=u16::MAX => return Err(CpuError::UnknownOpcode),
        }
        result
    }

    // Advance program counter by 16 bits
    // Constraints: PC must not be greater 4096, as this exceeds the memory limit of 4KB.
    fn increment_pc(&mut self) -> Result<(), CpuError> {
        self.pc += 2;
        if self.pc >= MEMORY_SIZE as u16 {
            return Err(CpuError::MemoryOutOfBounds);
        }
        Ok(())
    }

    // Increment stack pointer by 1
    // Constraints: SP must not exceed 15, because only 16 nested subroutines are allowed.
    fn increment_sp(&mut self) -> Result<(), CpuError> {
        self.sp += 1;
        if self.sp >= STACK_SIZE as i16 {
            return Err(CpuError::StackOverflow);
        }
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

    /// Opcode 0x1nnn - JP addr
    ///
    /// The interpreter sets the program counter to nnn.
    fn jp(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.pc = addr;
        Ok(())
    }

    /// Opcode 0x2nnn - CALL addr
    ///
    /// Call subroutine at nnn.
    /// The interpreter increments the stack pointer, then puts the current PC on the top of the stack.
    /// PC is then set to nnn.
    fn call(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.increment_sp()?;
        self.stk.push(self.pc);
        self.pc = addr;
        Ok(())
    }

    /// Opcode 0x3xkk - SE Vx, byte
    ///
    /// Skip next instruction if Vx = kk.
    /// The interpreter compares register Vx to kk, and if they are equal,
    /// increments the program counter by 2.
    fn sexb(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let kk = inst as u8;
        if self.reg[x] == kk {
            self.increment_pc()?;
            self.increment_pc()?;
        }
        Ok(())
    }

    /// Opcode 0x4xkk - SNE Vx, byte
    ///
    /// Skip next instruction if Vx != kk.
    /// The interpreter compares register Vx to kk, and if they are not equal,
    /// increments the program counter by 2.
    fn snexb(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = (inst & 0x0F00) >> 8;
        let kk = inst & 0x00FF;
        if self.reg[x as usize] != kk as u8 {
            self.increment_pc()?;
            self.increment_pc()?;
        }
        Ok(())
    }

    /// Opcode 0x5xy0 - SE Vx, Vy
    ///
    /// Skip next instruction if Vx = Vy.
    /// The interpreter compares register Vx to register Vy, and if they are equal,
    /// increments the program counter by 2.
    fn sexy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        if self.reg[x] == self.reg[y] {
            self.increment_pc()?;
            self.increment_pc()?;
        }
        Ok(())
    }

    /// Opcode 0x6xkk - LD Vx, byte
    ///
    /// Set Vx = kk.
    /// The interpreter puts the value kk into register Vx.
    fn ldxb(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let kk = inst as u8;
        self.reg[x] = kk;
        Ok(())
    }

    /// Opcode 0x7xkk - ADD Vx, byte
    ///
    /// Set Vx = Vx + kk.
    /// Adds the value kk to the value of register Vx, then stores the result in Vx.
    fn addxb(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let kk = inst as u8;
        self.reg[x] += kk;
        Ok(())
    }

    /// Opcode 0x8xy0 - LD Vx, Vy
    ///
    /// Set Vx = Vy.
    /// Stores the value of register Vy in register Vx.
    fn ldxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        self.reg[x] = self.reg[y];
        Ok(())
    }

    /// Opcode 0x8xy1 - OR Vx, Vy
    ///
    /// Set Vx = Vx OR Vy.
    /// Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    fn orxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        self.reg[x] |= self.reg[y];
        Ok(())
    }

    /// Opcode 0x8xk2 - AND Vx, Vy
    ///
    /// Set Vx = Vx AND Vy.
    /// Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    fn andxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        self.reg[x] &= self.reg[y];
        Ok(())
    }

    /// Opcode 0x8xy3 - XOR Vx, Vy
    ///
    /// Set Vx = Vx XOR Vy.
    /// Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
    fn xorxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        self.reg[x] ^= self.reg[y];
        Ok(())
    }

    /// Opcode 0x8xy4 - ADD Vx, Vy
    ///
    /// Set Vx = Vx + Vy, set VF = carry.
    /// The values of Vx and Vy are added together.
    /// If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
    /// Only the lowest 8 bits of the result are kept, and stored in Vx.
    fn addxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        let res = self.reg[x] as u16 + self.reg[y] as u16;
        if res > 255 {
            self.reg[0xF] = 1
        } else {
            self.reg[0xF] = 0
        }
        self.reg[x] = res as u8;
        Ok(())
    }

    /// Opcode 0x8xy5 - SUB Vx, Vy
    ///
    /// Set Vx = Vx - Vy, set VF = NOT borrow.
    /// If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results stored in Vx.
    fn subxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        // Use wrapping_sub instead of regular operator to allow overflow
        let res = self.reg[x].wrapping_sub(self.reg[y]);
        if self.reg[x] > self.reg[y] {
            self.reg[0xF] = 1
        } else {
            self.reg[0xF] = 0
        }
        self.reg[x] = res;
        Ok(())
    }

    /// Opcode 0x8xy6 - SHR Vx, {, Vy}
    ///
    /// Set Vx = Vx SHR 1.
    /// If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0. Then Vx is divided by 2.
    fn shrx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        if self.reg[x] % 2 == 0 {
            self.reg[0xF] = 0
        } else {
            self.reg[0xF] = 1
        }
        self.reg[x] /= 2;
        Ok(())
    }

    /// Opcode 0x8xy7 - SUBN Vx, Vy
    ///
    /// Set Vx = Vy - Vx, set VF = NOT borrow.
    /// If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results stored in Vx.
    fn subnxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        // Use wrapping_sub instead of regular operator to allow overflow
        let res = self.reg[y].wrapping_sub(self.reg[x]);
        if self.reg[y] > self.reg[x] {
            self.reg[0xF] = 1
        } else {
            self.reg[0xF] = 0
        }
        self.reg[x] = res;
        Ok(())
    }

    /// Opcode 0x8xyE - SHL Vx, {, Vy}
    ///
    /// Set Vx = Vx SHL 1.
    /// If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to 0. Then Vx is multiplied by 2.
    fn shlx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        if self.reg[x] >> 7 == 1 {
            self.reg[0xF] = 1
        } else {
            self.reg[0xF] = 0
        }
        self.reg[x] = self.reg[x].wrapping_mul(2);
        Ok(())
    }

    /// Opcode 0x9xy0 - SNE Vx, Vy
    ///
    /// Skip next instruction if Vx != Vy.
    /// The interpreter compares register Vx to register Vy, and if they are not equal,
    /// increments the program counter by 2.
    fn snexy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        if self.reg[x] != self.reg[y] {
            self.increment_pc()?;
            self.increment_pc()?;
        }
        Ok(())
    }

    /// Opcode 0xAnnn - LD I, addr
    ///
    /// Set value of register I to nnn.
    fn ldi(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.i = addr;
        Ok(())
    }

    /// Opcode 0xBnnn - JP V0, addr
    ///
    /// Set program counter to nnn + value in V0.
    fn jp0(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.pc = addr + self.reg[0x0] as u16;
        Ok(())
    }

    /// Opcode 0xCxkk - RND Vx, byte
    ///
    /// Set Vx = random byte AND kk.
    /// The interpreter generates a random number from 0 to 255, which is then ANDed with the value kk.
    /// The results are stored in Vx.
    fn rndx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let kk = inst as u8;
        let val: u8 = rand::random();
        self.reg[x] = val & kk;
        Ok(())
    }

    /// Opcode 0xDxyn - DRW Vx, Vy, nibble
    ///
    /// Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.
    /// The interpreter reads n bytes from memory, starting at the address stored in I.
    /// These bytes are then displayed as sprites on screen at coordinates (Vx, Vy).
    /// Sprites are XORed onto the existing screen. If this causes any pixels to be erased,
    /// VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it is
    /// outside the coordinates of the display, it wraps around to the opposite side of the screen.
    fn drwxy(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let y = ((inst & 0x00F0) >> 4) as usize;
        let n = (inst & 0x000F) as usize;
        let x_coord = self.reg[x] as usize;
        let y_coord = self.reg[y] as usize;
        let mut sprite: Vec<u8> = vec![];
        for j in 0..n {
            sprite.push(self.mem[self.i as usize + j])
        }
        #[cfg(test)]
        assert_eq!(sprite, [0xF0, 0x90, 0x90, 0x90, 0xF0]);
        self.reg[0xF] = self.dct.draw(x_coord, y_coord, sprite);
        Ok(())
    }

    /// Opcode 0xEx9E - SKP Vx
    ///
    /// Skip next instruction if key with the value of Vx is pressed.
    /// Checks the keyboard, and if the key corresponding to the value of Vx is
    /// currently in the down position, PC is increased by 2.
    fn skpx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let key = self.reg[x];
        if self.ict.key_pressed(key) {
            self.increment_pc()?;
            self.increment_pc()?;
        }
        Ok(())
    }

    /// Opcode 0xExA1 - SKNP Vx
    ///
    /// Skip next instruction if key with the value of Vx is not pressed.
    /// Checks the keyboard, and if the key corresponding to the value of Vx is
    /// currently in the up position, PC is increased by 2.
    fn sknpx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let key = self.reg[x];
        if !self.ict.key_pressed(key) {
            self.increment_pc()?;
            self.increment_pc()?;
        }
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

    // Execute an unknown opcode loaded to address 0x0000
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
        let mut c = Cpu {
            pc: 4094,
            ..Default::default()
        };
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
        assert_eq!(
            c.stk.pop(),
            Some(0),
            "testing if PC has been saved on stack"
        );
        assert_eq!(c.pc, 0xBEE, "testing if new PC has been set");
    }

    // Execute the sexb instruction
    #[test]
    fn exec_routine_sexb() {
        let mut c = Cpu::default();
        c.reg[0xA] = 0xBE;
        c.mem[0] = 0x3A;
        c.mem[1] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4, "testing of se instruction");
    }

    // Execute the snexb instruction
    #[test]
    fn exec_routine_snexb() {
        let mut c = Cpu::default();
        c.reg[0xA] = 0xBE;
        c.mem[0] = 0x4A;
        c.mem[1] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 0, "testing of sne instruction");
    }

    // Execute the sexy instruction
    // Ha, ha.
    #[test]
    fn exec_routine_sexy_success() {
        let mut c = Cpu::default();
        c.mem[0] = 0x5A;
        c.mem[1] = 0xC0;
        c.reg[0xA] = 0xBE;
        c.reg[0xC] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4, "testing of sexy instruction");
    }

    // Execute the sexy instruction and fail
    #[test]
    #[should_panic]
    fn exec_routine_sexy_failure() {
        let mut c = Cpu::default();
        c.mem[0] = 0x5A;
        c.mem[1] = 0xC1;
        c.reg[0xA] = 0xBE;
        c.reg[0xC] = 0xBE;
        c.exec_routine().unwrap();
    }

    // Execute the ldxb instruction
    #[test]
    fn exec_routine_ldxb() {
        let mut c = Cpu::default();
        c.mem[0] = 0x6A;
        c.mem[1] = 0x22;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0A], 0x22);
    }

    // Execute the addxb instruction
    #[test]
    fn exec_routine_addxb() {
        let mut c = Cpu::default();
        c.mem[0] = 0x7A;
        c.mem[1] = 0x15;
        c.reg[0xA] = 2;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0A], 0x17);
    }

    // Execute the ldxy instruction
    #[test]
    fn exec_routine_ldxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC0;
        c.reg[0xC] = 2;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0B], 2);
    }

    // Execute the orxy instruction
    #[test]
    fn exec_routine_orxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC1;
        c.reg[0xB] = 4;
        c.reg[0xC] = 2;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0B], 6);
    }

    // Execute the andxy instruction
    #[test]
    fn exec_routine_andxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC2;
        c.reg[0xB] = 4;
        c.reg[0xC] = 2;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0B], 0);
    }

    // Execute the xorxy instruction
    #[test]
    fn exec_routine_xorxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC3;
        c.reg[0xB] = 4;
        c.reg[0xC] = 3;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0B], 7);
    }

    // Execute the addxy instruction
    #[test]
    fn exec_routine_addxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC4;
        c.reg[0xB] = 255;
        c.reg[0xC] = 20;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0F], 1);
        assert_eq!(c.reg[0x0B], 19);
    }

    // Execute the subxy instruction
    #[test]
    fn exec_routine_subxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC5;
        c.reg[0xB] = 10;
        c.reg[0xC] = 100;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0F], 0);
        assert_eq!(c.reg[0x0B], 166);
    }

    // Execute the shrx instruction
    #[test]
    fn exec_routine_shrx() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0x06;
        c.reg[0xB] = 11;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0F], 1);
        assert_eq!(c.reg[0x0B], 5);
    }

    // Execute the subnxy instruction
    #[test]
    fn exec_routine_subnxy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0xC7;
        c.reg[0xB] = 100;
        c.reg[0xC] = 10;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0F], 0);
        assert_eq!(c.reg[0x0B], 166);
    }

    // Execute the shlx instruction
    #[test]
    fn exec_routine_shlx() {
        let mut c = Cpu::default();
        c.mem[0] = 0x8B;
        c.mem[1] = 0x0E;
        c.reg[0xB] = 0x80;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0F], 1);
        assert_eq!(c.reg[0x0B], 0);
    }

    // Execute the snexy instruction
    #[test]
    fn exec_routine_snexy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x9A;
        c.mem[1] = 0xC0;
        c.reg[0xA] = 0x20;
        c.reg[0xC] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4);
    }

    // Execute the ldi instruction
    #[test]
    fn exec_routine_ldi() {
        let mut c = Cpu::default();
        c.mem[0] = 0xAB;
        c.mem[1] = 0xBB;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.i, 0xBBB);
    }

    // Execute the jp0 instruction
    #[test]
    fn exec_routine_jp0() {
        let mut c = Cpu::default();
        c.mem[0] = 0xBC;
        c.mem[1] = 0xBC;
        c.reg[0] = 1;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 0xCBD);
    }

    // Execute the drwxy instruction
    #[test]
    fn exec_routine_drwxy() {
        // Set I to '0' of the system font
        let mut c = Cpu {
            i: FONT_START_ADDR as u16,
            ..Default::default()
        };
        c.mem[0] = 0xD0;
        c.mem[1] = 0x05;
        c.exec_routine().expect("exec_routine failed");
        // Frame buffer starts empty, so collision should not occur
        assert_eq!(c.reg[0xF], 0);
    }
}

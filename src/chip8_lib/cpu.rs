use log::{error, info, warn};
use std::fs::File;
use std::io::Read;
use std::time::Duration;
use thiserror::Error;

use crate::display::DisplayController;
use crate::input::InputController;

const MEMORY_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
// Maximum 16 nested subroutines
const STACK_SIZE: usize = 16;
// Memory address from where the font is stored; by convention this is 0x50
pub const FONT_START_ADDR: usize = 0x50;
pub const PROGRAM_ENTRY_POINT: usize = 0x200;

// CHIP-8 runs at approx. 600hz
pub const CLOCK_SPEED: Duration = Duration::from_nanos(1_000_000_000 / 600);
// Timers run at 60hz
pub const TIMER_TICK: i64 = 1_000_000_000 / 60;

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

// Error handling
#[derive(Error, Debug)]
pub enum IOError {
    #[error("could not open file")]
    FileOpenError,
    #[error("could not read file")]
    FileReadError,
}

pub struct Cpu {
    // Program counter
    pc: u16,
    // Stack pointer
    sp: i16,
    // Delay timer
    dt: u8,
    dt_delta: i64,
    // Sound timer
    st: u8,
    st_delta: i64,
    // Index register
    i: u16,
    // General purpose registers
    reg: [u8; REGISTER_COUNT],
    // Memory space; maximum 4KB
    mem: [u8; MEMORY_SIZE],
    // Stack; holds maximum of 16 addresses
    stk: Vec<u16>,
    pub dct: DisplayController,
    pub ict: InputController,
    paused: bool,
    blocking: bool,
    reg_to_write: Option<u8>
}

impl Default for Cpu {
    fn default() -> Self {
        let mut ret = Self {
            pc: 0,
            sp: 0,
            dt: 0,
            dt_delta: TIMER_TICK,
            st: 0,
            st_delta: TIMER_TICK,
            i: 0,
            reg: [0; REGISTER_COUNT],
            mem: [0; MEMORY_SIZE],
            stk: vec![],
            dct: DisplayController::default(),
            ict: InputController::default(),
            paused: false,
            blocking: false,
            reg_to_write: None, 
        };
        &ret.load_font();
        ret
    }
}

impl Cpu {
    // Map font to memory
    fn load_font(&mut self) {
        for i in FONT_START_ADDR..FONT_START_ADDR + FONT.len() {
            self.mem[i] = FONT[i - FONT_START_ADDR];
        }
    }

    /// Takes a filename string and attempts to load the binary instructions
    /// to the usual entry point, 0x200
    pub fn load_program(&mut self, filename: &str) -> Result<(), IOError> {
        let mut buffer: [u8; MEMORY_SIZE - PROGRAM_ENTRY_POINT] =
            [0; MEMORY_SIZE - PROGRAM_ENTRY_POINT];
        let mut file = File::open(filename);
        match file {
            Ok(f) => file = Ok(f),
            _ => {
                return Err(IOError::FileOpenError);
            }
        }

        match file.unwrap().read(&mut buffer) {
            Ok(b) => {
                info!("Read {b} bytes from {filename}.");
            }
            Err(_) => {
                return Err(IOError::FileReadError);
            }
        };
        self.mem[PROGRAM_ENTRY_POINT..MEMORY_SIZE].copy_from_slice(&buffer);
        Ok(())
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn is_blocking(&self) -> bool {
        self.blocking
    }

    pub fn unblock(&mut self, key: u8) {
        match self.reg_to_write {
            Some(r) => self.reg[r as usize] = key,
            None => {
                error!("Something has gone wrong here. Unblock called but register to write is not set.")
            }
        }
        self.reg_to_write = None;
        self.blocking = false;
    }

    pub fn timer_tick(&mut self, delta: Duration) {
        self.dt_delta -= delta.as_nanos() as i64;
        self.st_delta -= delta.as_nanos() as i64;
        if self.dt_delta <= 0 && self.dt > 0 {
            self.dt_delta = TIMER_TICK;
            self.dt -= 1;
        }
        if self.st_delta <= 0 && self.st > 0 {
            self.st_delta = TIMER_TICK;
            self.st -= 1;
        }
    }

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
            0xF000..0xFFFF => match inst & 0x00FF {
                0x0007 => result = self.ldxdt(inst),
                0x000A => result = self.ldxk(inst),
                0x0015 => result = self.lddtx(inst),
                0x0018 => result = self.ldstx(inst),
                0x001E => result = self.addix(inst),
                0x0029 => result = self.ldfx(inst),
                0x0033 => result = self.ldbx(inst),
                0x0055 => result = self.ldiax(inst),
                0x0065 => result = self.ldxia(inst), 
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
            Some(val) => {
                self.pc = val;
                self.sp -= 1;
            }
            None => return Err(CpuError::EmptyStack),
        }
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
        }
        self.increment_pc()?;
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
        }
        self.increment_pc()?;
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
        }
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        }
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xAnnn - LD I, addr
    ///
    /// Set value of register I to nnn.
    fn ldi(&mut self, inst: u16) -> Result<(), CpuError> {
        let addr = inst & 0x0FFF;
        self.i = addr;
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        self.increment_pc()?;
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
        }
        self.increment_pc()?;
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
        }
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx07 - LD Vc, DT
    ///
    /// Set Vx = delay timer value.
    /// The value of DT is placed into Vx.
    fn ldxdt(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        self.reg[x] = self.dt;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx0A - LD Vx, K
    ///
    /// Wait for a key press, store the value of the key in Vx.
    /// All execution stops until a key is pressed, then the value of that key is stored in Vx.
    /// 
    /// Set some state that is checked in main loop
    fn ldxk(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as u8;
        self.reg_to_write = Some(x);
        self.blocking = true;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx15 - LD DT, Vx
    ///
    /// Set delay timer = Vx.
    /// DT is set equal to the value of Vx.
    fn lddtx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as u8;
        self.dt = x;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx18 - LD ST, Vx
    ///
    /// Set sound timer = Vx.
    /// ST is set equal to the value of Vx.
    fn ldstx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as u8;
        self.st = x;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx1E - ADD I, Vx
    ///
    /// Set I = I + Vx.
    /// The values of I and Vx are added, and the results are stored in I.
    fn addix(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        self.i += self.reg[x] as u16;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx29 - LD F, Vx
    ///
    /// Set I = location of sprite for digit Vx.
    /// The value of I is set to the location for the hexadecimal sprite corresponding to the value of Vx.
    fn ldfx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        self.i = FONT_START_ADDR as u16 + (self.reg[x] * 5) as u16;
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx33 - LD B, Vx
    ///
    /// Store BCD representation of Vx in memory locations I, I+1, and I+2.
    /// The interpreter takes the decimal value of Vx, and places the hundreds digit in memory at location in I,
    /// the tens digit at location I+1, and the ones digit at location I+2.
    fn ldbx(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        let mut num = self.reg[x];
        let mut j = 3;
        while num != 0 && j != 0 {
            j -= 1;
            self.mem[self.i as usize + j] = num % 10;
            num /= 10;
        }
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx55 - LD [I], Vx
    ///
    /// Store registers V0 through Vx in memory starting at location I.
    /// The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
    fn ldiax(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        for j in 0..x + 1 {
            self.mem[self.i as usize + j] = self.reg[j]
        }
        self.increment_pc()?;
        Ok(())
    }

    /// Opcode 0xFx65 - LD Vx, [I]
    ///
    /// Read registers V0 through Vx from memory starting at location I.
    /// The interpreter reads values from memory starting at location I into registers V0 through Vx.
    fn ldxia(&mut self, inst: u16) -> Result<(), CpuError> {
        let x = ((inst & 0x0F00) >> 8) as usize;
        for j in 0..x + 1{
            self.reg[j] = self.mem[self.i as usize + j]
        }
        self.increment_pc()?;
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
        assert_eq!(c.pc, 2);
    }

    // Execute an unknown opcode loaded to address 0x0000
    #[test]
    #[should_panic]
    fn exec_routine_failure() {
        let mut c = Cpu::default();
        c.mem[0] = 0xFF;
        c.mem[1] = 0xFF;
        c.exec_routine().unwrap();
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 0xBEE);
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
        assert_eq!(c.pc, 0xBEE);
    }

    // Execute the sexb instruction
    #[test]
    fn exec_routine_sexb() {
        let mut c = Cpu::default();
        c.reg[0xA] = 0xBE;
        c.mem[0] = 0x3A;
        c.mem[1] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4);
    }

    // Execute the snexb instruction
    #[test]
    fn exec_routine_snexb() {
        let mut c = Cpu::default();
        c.reg[0xA] = 0xBE;
        c.mem[0] = 0x4A;
        c.mem[1] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
    }

    // Execute the sexy instruction
    // Ha, ha.
    #[test]
    fn exec_routine_sexy() {
        let mut c = Cpu::default();
        c.mem[0] = 0x5A;
        c.mem[1] = 0xC0;
        c.reg[0xA] = 0xBE;
        c.reg[0xC] = 0xBE;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4);
    }

    // Execute the ldxb instruction
    #[test]
    fn exec_routine_ldxb() {
        let mut c = Cpu::default();
        c.mem[0] = 0x6A;
        c.mem[1] = 0x22;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.reg[0x0A], 0x22);
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
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
        assert_eq!(c.pc, 2);
    }

    // Execute the addix instruction
    #[test]
    fn exec_routine_addix() {
        let mut c = Cpu::default();
        c.mem[0] = 0xF0;
        c.mem[1] = 0x1E;
        c.i = 0x700;
        c.reg[0] = 5;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
        assert_eq!(c.i as usize, 0x705);
    }

    // Execute the ldfx instruction
    #[test]
    fn exec_routine_ldfx() {
        let mut c = Cpu::default();
        c.mem[0] = 0xF0;
        c.mem[1] = 0x29;
        c.reg[0] = 1;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
        assert_eq!(c.i as usize, 0x55);
        c.mem[2] = 0xF0;
        c.mem[3] = 0x29;
        c.reg[0] = 2;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 4);
        assert_eq!(c.i as usize, 0x5A);
    }

    // Execute the ldbx instruction
    #[test]
    fn exec_routine_ldbx() {
        let mut c = Cpu::default();
        c.mem[0] = 0xF0;
        c.mem[1] = 0x33;
        c.reg[0] = 123;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
        assert_eq!(c.mem[c.i as usize], 1);
        assert_eq!(c.mem[c.i as usize + 1], 2);
        assert_eq!(c.mem[c.i as usize + 2], 3);
    }

    // Execute the ldiax instruction
    #[test]
    fn exec_routine_ldiax() {
        let mut c = Cpu::default();
        c.mem[0] = 0xF2;
        c.mem[1] = 0x55;
        c.reg[0] = 1;
        c.reg[1] = 2;
        c.reg[2] = 3;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
        assert_eq!(c.mem[c.i as usize], 1);
        assert_eq!(c.mem[c.i as usize + 1], 2);
        assert_eq!(c.mem[c.i as usize + 2], 3);
    }

    // Execute the ldxia instruction
    #[test]
    fn exec_routine_ldxia() {
        let mut c = Cpu::default();
        c.mem[0] = 0xF2;
        c.mem[1] = 0x65;
        c.i = 0x700;
        c.mem[0x700] = 1;
        c.mem[0x701] = 2;
        c.mem[0x702] = 3;
        c.exec_routine().expect("exec_routine failed");
        assert_eq!(c.pc, 2);
        assert_eq!(c.reg[0], 1);
        assert_eq!(c.reg[1], 2);
        assert_eq!(c.reg[2], 3);
    }
}

use crate::display::DisplayController;

// Memory address from where the font is stored, by convention
const FONT_START_ADDR: usize = 0x50;

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
    display_controller: DisplayController,
}

impl Cpu {
    pub fn new(dct: DisplayController) -> Self {
        let mut ret = Self {
            pc : 0,
            sp : 0,
            dt : 0,
            st : 0,
            i : 0,
            reg : [0; 16],
            mem : [0; 4096],
            stk: vec![],
            display_controller : dct,
        };
        // Map font to memory
        let font: [u8; 80] = [
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
        for i in FONT_START_ADDR..font.len() {
            ret.mem[i] = font[i-FONT_START_ADDR];
        }
        ret
    }

    // Run the current instruction pointed to by PC
    pub fn exec_routine(&mut self) {
        match self.mem[self.pc as usize] {
            0x00E0 => self.cls(),
            0x00EE => self.ret(),
        }
    }

    fn cls(&mut self) {
        self.display_controller.clear_screen();
    }

    // The interpreter sets the program counter to the address at the top of the stack,
    // then subtracts 1 from the stack pointer.
    fn ret(&mut self) {
        self.sp -= 1;
    }
}
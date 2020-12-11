use crate::display::Display;
use crate::errors::Chip8Error;
use crate::ram::Ram;

#[derive(Debug)]
pub struct Cpu {
    vx: [u8; 16], //General purpose registers
    pc: u16,      //Program counter
    i: u16,       //Another register, mostly for memory addresses
    stack: Vec<u16>,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            vx: [0u8; 16],
            pc: 0x200, //Points to the start of the executable by default
            i: 0,
            stack: Vec::with_capacity(16),
        }
    }

    pub fn tick(&mut self, ram: &mut Ram, display: &mut Display) -> Result<(), Chip8Error> {
        if self.pc >= (ram.length() - 1) as u16 {
            return Err(Chip8Error::EOF);
        }
        let instr_u = ram.read_byte(self.pc); //Upper (most significant) byte of the instruction
        let instr_l = ram.read_byte(self.pc + 1); //Lower (leasz significant) byte of the instruction
        let instr = ((instr_u as u16) << 8) | (instr_l as u16);

        //Clears the display
        if instr == 0x00E0 {
            display.clear();

        //Returns from a subroutine
        } else if instr == 0x00EE {
            if self.stack.len() == 0 {
                return Err(Chip8Error::ReturnOnEmptyStack(self.pc));
            }
            self.pc = *self.stack.last().unwrap();
            self.stack.pop();
            return Ok(());

        //Jumps to another location
        } else if instr & 0xF000 == 0x1000 {
            let l = instr & 0x0FFF;
            self.pc = l;
            return Ok(());

        //Calls a subroutine
        } else if instr & 0xF000 == 0x2000 {
            self.stack.push(self.pc);
            let l = instr & 0x0FFF;
            self.pc = l;
            return Ok(());

        //Skips the next instruction if the register is equal to the value of the lower 8 bits
        } else if instr & 0xF000 == 0x3000 {
            let index = (instr & 0x0F00) >> 8;
            let value = instr & 0x00FF;
            if self.vx[index as usize] == value as u8 {
                self.pc += 2;
            }

        //Skips the next instruction if the register is not equal to the value of the lower 8 bits
        } else if instr & 0xF000 == 0x4000 {
            let index = (instr & 0x0F00) >> 8;
            let value = instr & 0x00FF;
            if self.vx[index as usize] != value as u8 {
                self.pc += 2;
            }

        //Skip the next intstruction if the registers are equal
        } else if instr & 0xF000 == 0x5000 {
            let r1 = (instr & 0x0F00) >> 8;
            let r2 = (instr & 0x00F0) >> 4;
            if self.vx[r1 as usize] == self.vx[r2 as usize] {
                self.pc += 2;
            }

        //Put the value of the lower 8 bits into the register
        } else if instr & 0xF000 == 0x6000 {
            let r = (instr & 0x0F00) >> 8;
            let val = instr & 0x00FF;
            self.vx[r as usize] = val as u8;

        //Adds the value of the lower 8 bits to the register and stores it in the register
        } else if instr & 0xF000 == 0x7000 {
            let r = (instr & 0x0F00) >> 8;
            let val = instr & 0x00FF;
            self.vx[r as usize] = ((self.vx[r as usize] as u16 + val) % 256) as u8;

        //Stores the value of register y into register x
        } else if instr & 0xF00F == 0x8000 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[x as usize] = self.vx[y as usize];

        //Bitwise OR on registers x and y, storing the result in register x
        } else if instr & 0xF00F == 0x8001 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[x as usize] = self.vx[x as usize] | self.vx[y as usize];

        //Bitwise AND on registers x and y, storing the result in register x
        } else if instr & 0xF00F == 0x8002 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[x as usize] = self.vx[x as usize] & self.vx[y as usize];

        //Bitwise XOR on registers x and y, storing the result in register x
        } else if instr & 0xF00F == 0x8003 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[x as usize] = self.vx[x as usize] ^ self.vx[y as usize];

        //Addition of registers x + y, storing into x. If the value overflows, set vF to 1, else to 0.
        } else if instr & 0xF00F == 0x8004 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            let sum = self.vx[x as usize] as u16 + self.vx[y as usize] as u16;
            if sum > 255 {
                self.vx[0xF] = 1;
            } else {
                self.vx[0xF] = 0;
            }
            self.vx[x as usize] = sum as u8;

        //Subtraction of registers x - y, storing into x. If value x > value y, then vF is set to 1, otherwise 0.
        } else if instr & 0xF00F == 0x8005 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            if self.vx[x as usize] > self.vx[y as usize] {
                self.vx[0xF] = 1;
            } else {
                self.vx[0xF] = 0;
            }
            let diff = self.vx[x as usize] as i16 - self.vx[y as usize] as i16;
            self.vx[x as usize] = diff as u8;

        //Shift value of register y one bit to the right, store in register x.
        //vF is set to the least significant bit prior to the shift.
        } else if instr & 0xF00F == 0x8006 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[0xF] = self.vx[y as usize] & 0x1;
            self.vx[x as usize] = self.vx[y as usize] >> 1;

        //Shift value of register y one bit to the left, store in register x.
        //vF is set to the least significant bit prior to the shift.
        } else if instr & 0xF00F == 0x800E {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            self.vx[0xF] = self.vx[y as usize] & 0x80;
            self.vx[x as usize] = self.vx[y as usize] << 1;

        //Subtraction of registers y - x, storing into x. If value x > value y, then vF is set to 1, otherwise 0.
        } else if instr & 0xF00F == 0x8007 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            if self.vx[y as usize] > self.vx[x as usize] {
                self.vx[0xF] = 1;
            } else {
                self.vx[0xF] = 0;
            }
            let diff = self.vx[y as usize] as i16 - self.vx[x as usize] as i16;
            self.vx[x as usize] = diff as u8;

        //Skip the next instruction if value x != value y.
        } else if instr & 0xF00F == 0x9000 {
            let x = (instr & 0x0F00) >> 8;
            let y = (instr & 0x00F0) >> 4;
            if self.vx[x as usize] != self.vx[y as usize] {
                self.pc += 2;
            }
        } else {
            return Err(Chip8Error::UnsupportedInstr(instr, self.pc));
        }

        self.pc += 2;
        Ok(())
    }
}

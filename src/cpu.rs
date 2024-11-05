// Notes:
// NES CPU uses Little Endian:
// Read Address: 0x80000
// Address packed in big-endian: 80 00
// Address Packed in little-endian: 00 80

use std::collections::HashMap;
use crate::opcodes;

// Defining a CPU
pub struct CPU {
    pub register_a: u8,
    pub status: u8,
    pub program_counter: u16, 
    pub register_x: u8,
    pub register_y: u8,
    memory: [u8; 0xFFFF]
}

#[derive(Debug)]
#[allow(non_camel_case_types)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPage_X,
    ZeroPage_Y,
    Absolute,
    Absolute_X,
    Absolute_Y,
    Indirect_X,
    Indirect_Y,
    NoneAddressing,
}

trait Mem {
    fn mem_read(&self, addr: u16) -> u8;

    fn mem_write(&mut self, addr: u16, data: u8); 

    fn mem_read_u16(&self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;
        let hi = self.mem_read(pos + 1) as u16; 
        (hi << 8) | (lo as u16)
    }

    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8; 
        let lo = (data & 0xff) as u8; 
        self.mem_write(pos, lo); 
        self.mem_write(pos + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}

impl CPU {

    // Create a new CPU
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            status: 0,
            program_counter: 0,
            register_x: 0,
            register_y: 0,
            memory: [0; 0xFFFF] // Program ROM
        }
    }

    // Read data from given memory location
    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    // Write given data into given memory location
    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data
    }
    
    // Read 16 bit address from memory and interpret little endian
    fn mem_read_u16(&mut self, pos: u16) -> u16 {
        let lo = self.mem_read(pos) as u16;         // lower
        let hi = self.mem_read(pos + 1) as u16;     // higher
        (hi << 8) | (lo as u16)                     // ho00000000 + 00000000lo
    }
    
    // Write 16 bit address to memory in little endian
    fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8; 

        self.mem_write(pos, lo);
        self.mem_write(pos + 1, hi)
    }
    
    // Restore state of all registers and initialize program_counter by 2 byte value stored at
    // 0xFFFC (default place where NES CPU stores the value of program counter when cartrige is
    // inserted)
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0; 
        self.register_y = 0;
        self.status = 0;

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    // Load instructions from a Vector and place them in correct memory locations 
    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x80000 .. (0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000)
    }

    // Load instructions from a Vector, reset the state of the CPU and run it
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    // Function for 0xA9 Opscode - Load Value into Accumulator (A)
    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_and_negative_flags(self.register_a);
    }
 
    // Function for 0xAA Opscode - Transfer value of A to X
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
   
    // Function for 0xE8 Opscode - Increment value of X
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x)
    }
  
    // Function for updating zero and negative flags
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101;
        }

        if result & 0b1000_0000 != 0 {
            self.status = self.status | 0b1000_0000;
        } else {
            self.status = self.status & 0b0111_1111;
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        
        match mode {
            // Value is directly given: LAD #$10
            AddressingMode::Immediate => self.program_counter,

            // Loads value from given memory location - from $00 - $FF (first 256 bytes of memory)
            // Only single byte is required to load the data
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,

            // Loads value from given memory location anywhere in the memory
            // 2 byte is required to load the data
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),

            // MOV AL, [0x10 + DX]  ; Load the value from (0x10 + X) in zero page into AL 
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            
            // MOV AL, [0x10 + DY]  ; Load the value from (0x10 + X) in zero page into AL
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.program_counter);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            
            // MOV AL, [0x2000 + DX]  ; Load the value from (0x2000 + X) into AL
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            
            // MOV AL, [0x2000 + DY]  ; Load the value from (0x2000 + Y) into AL
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.program_counter);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }
            
            // Read the memory address from a given address as operand + offset stored on X
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }

            // Read the memory address from a given address as operand + offset stored in Y 
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.program_counter);

                let ptr: u8 = (base as u8).wrapping_add(self.register_y);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);

                (hi as u16) << 8 | (lo as u16) 
            }

            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }   
    }

    // Interpret Opscode and Excute
    pub fn run(&mut self) {
        // Looping through all instructions
        
        // Program counter is initialized in load() with value 0x8000
        loop {

            // Opscode would be read from memory
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;

            match code {
                0xA9 => {
                    self.lda(&AddressingMode::Immediate);
                    self.program_counter += 1;
                }

                0xAA => self.tax(),

                0xE8 => self.inx(),
                
                0x00 => return,
                
                _ => todo!(),
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 5);
        assert!(cpu.status & 0b0000_0010 == 0);
        assert!(cpu.status & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0A,0xaa, 0x00]);

        assert_eq!(cpu.register_x, 10)
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa,0xe8, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }
}

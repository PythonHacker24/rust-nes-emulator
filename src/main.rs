// Defining a CPU
pub struct CPU {
    pub register_a: u8,
    pub status: u8,
    pub program_counter: u16, 
    pub register_x: u8,
    memory: [u8; 0xFFFF]
}

impl CPU {

    // Create a new CPU
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            status: 0,
            program_counter: 0,
            register_x: 0,
        }
    }

    // Function for 0xA9 Opscode - Load Value into Accumulator (A)
    fn lda(&mut self, value: u8) {
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

    // Interpret Opscode and Excute
    pub fn interpret(&mut self, program: Vec<u8>) {
        self.program_counter = 0;

        // Looping through all instructions
        loop {
            let opscode = program[self.program_counter as usize];
            self.program_counter += 1;

            match opscode {
                0xA9 => {
                    let param = program[self.program_counter as usize];
                    self.program_counter += 1;

                    self.lda(param)
                }

                0xAA => self.tax(),

                0xE8 => self.inx(),
                
                0x00 => return,
                
                _ => todo!(),
            }
        }
    }
}

#[test]
fn test_5_ops_working_together() {
   let mut cpu = CPU::new();
   cpu.interpret(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

   assert_eq!(cpu.register_x, 0xc1)
}

#[test]
fn test_inx_overflow() {
    let mut cpu = CPU::new();
    cpu.register_x = 0xff;
    cpu.interpret(vec![0xe8, 0xe8, 0x00]);

    assert_eq!(cpu.register_x, 1)
}

fn main() {
    println!("NES Emulator");
}

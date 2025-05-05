use std::collections::HashMap;

use crate::op_codes;

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

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub pc: u16,
    memory: [u8; 0xFFFF],
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            pc: 0,
            memory: [0; 0xFFFF],
        }
    }

    // CPU specific

    // Resets the CPU state and sets the PC value to Reset Vector
    fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.status = 0;

        self.pc = self.mem_read_u16(0xFFFC);
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_zero_flag(self.register_a);
        self.update_negative_flag(self.register_a);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_flag(self.register_x);
        self.update_negative_flag(self.register_x);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_flag(self.register_x);
        self.update_negative_flag(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_flag(self.register_y);
        self.update_negative_flag(self.register_y);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_flag(self.register_y);
        self.update_negative_flag(self.register_y);
    }

    fn update_zero_flag(&mut self, result: u8) {
        if result == 0 {
            self.status = self.status | 0b0000_0010;
        } else {
            self.status = self.status & 0b1111_1101;
        }
    }

    fn update_negative_flag(&mut self, result: u8) {
        if result & 0b1000_0000 != 0 {
            self.status = self.status | 0b1000_0000;
        } else {
            self.status = self.status & 0b0111_1111;
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.pc,
            AddressingMode::ZeroPage => self.mem_read(self.pc) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.pc),
            AddressingMode::ZeroPage_X => {
                let pos = self.mem_read(self.pc);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPage_Y => {
                let pos = self.mem_read(self.pc);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }
            AddressingMode::Absolute_X => {
                let base = self.mem_read_u16(self.pc);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::Absolute_Y => {
                let base = self.mem_read_u16(self.pc);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }
            AddressingMode::Indirect_X => {
                let base = self.mem_read(self.pc);
                let ptr = base.wrapping_add(self.register_x);
                let addr = self.mem_read_u16(ptr as u16);
                addr
            }
            AddressingMode::Indirect_Y => {
                let base = self.mem_read(self.pc);
                let deref_base = self.mem_read_u16(base as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }
            AddressingMode::NoneAddressing => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    // Memory specific

    fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }

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

    // Execution

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run()
    }

    fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }

    fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static op_codes::OpCode> = *op_codes::OP_CODES_MAP;

        loop {
            let code = self.mem_read(self.pc);
            self.pc += 1;

            let old_pc = self.pc;

            let opcode = opcodes.get(&code).expect("opcode ${code:x} not valid");

            match code {
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.mode);
                }
                0x85 | 0x95 | 0x8D | 0x9D | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }
                0xAA => self.tax(),
                0xA8 => self.tay(),
                0xE8 => self.inx(),
                0xC8 => self.iny(),
                0x00 => return,
                _ => todo!(),
            }

            if old_pc == self.pc {
                self.pc += (opcode.len - 1) as u16
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
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status & 0b0000_0010 == 0b00);
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
        cpu.load_and_run(vec![0xa9, 0x0a, 0xaa, 0x00]);

        assert_eq!(cpu.register_x, 0x0a)
    }

    #[test]
    fn test_0xa8_tay_move_a_to_y() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0x0a, 0xa8, 0x00]);

        assert_eq!(cpu.register_y, 0x0a)
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
        cpu.load_and_run(vec![0xa9, 0xff, 0xaa, 0xe8, 0xe8, 0x00]);
        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_iny_overflow() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xff, 0xa8, 0xc8, 0xc8, 0x00]);
        assert_eq!(cpu.register_y, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);

        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_lda_indirect_x() {
        // Runs this program:
        // LDX #$01
        // LDA #$05
        // STA $01
        // LDA #$07
        // STA $02
        // LDY #$0a
        // STY $0705
        // LDA ($00,X)

        let mut cpu = CPU::new();

        // LDA #$05
        // STA $01
        cpu.mem_write(0x01, 0x05);

        // LDA #$07
        // STA $02
        cpu.mem_write(0x02, 0x07);

        // LDY #$0a
        // STY $0705
        cpu.mem_write(0x0705, 0x0a);

        //                    LDA  #$0x01 TAX   LDA ($00, X) BRK
        cpu.load_and_run(vec![0xa9, 0x01, 0xaa, 0xa1, 0x00, 0x00]);

        assert_eq!(cpu.register_a, 0x0a);
    }

    #[test]
    fn test_lda_indirect_y() {
        // Runs this program:
        // LDY #$01
        // LDA #$03
        // STA $01
        // LDA #$07
        // STA $02
        // LDX #$0a
        // STX $0704
        // LDA ($01),Y

        let mut cpu = CPU::new();

        // LDA #$03
        // STA $01
        cpu.mem_write(0x01, 0x03);

        // LDA #$07
        // STA $02
        cpu.mem_write(0x02, 0x07);

        // LDX #$0a
        // STX $0704
        cpu.mem_write(0x0704, 0x0a);

        //                    LDA  #$0x01 TAY   LDA ($01),Y BRK
        cpu.load_and_run(vec![0xa9, 0x01, 0xa8, 0xb1, 0x01, 0x00]);

        assert_eq!(cpu.register_a, 0x0a);
    }

    #[test]
    fn test_sta_zeropage_x() {
        // LDX #$01   ;X is $01
        // LDA #$aa   ;A is $aa
        // STA $a0,X ;Store the value of A at memory location $a1
        // INX        ;Increment X
        // STA $a0,X ;Store the value of A at memory location $a2
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![
            0xa9, 0x01, 0xaa, 0xa9, 0xaa, 0x95, 0xa0, 0xe8, 0x95, 0xa0, 0x00,
        ]);

        assert_eq!(cpu.mem_read_u16(0xa1), 0xaaaa)
    }
}

use crate::mmu::MMU;
use crate::registers::Flags;
use crate::registers::Registers;
use std::cell::RefCell;
use std::rc::Rc;

pub struct CPU {
    pub registers: Registers,
    pub mmu: Rc<RefCell<MMU>>,
    pub interrupt_master: bool,
    pub is_halted: bool,
}

impl CPU {
    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        let mut cpu = Self {
            registers: Registers {
                a: 0,
                f: 0,
                b: 0,
                c: 0,
                d: 0,
                e: 0,
                h: 0,
                l: 0,
                sp: 0xFFFE,
                pc: 0x100,
            },
            mmu,
            interrupt_master: true,
            is_halted: false,
        };

        cpu.registers.set_af(0x01B0);
        cpu.registers.set_bc(0x0013);
        cpu.registers.set_de(0x00D8);
        cpu.registers.set_hl(0x014D);

        cpu
    }

    pub fn fetch_byte(&mut self) -> u8 {
        let b = self.mmu.borrow_mut().rb(self.registers.pc);
        self.registers.pc += 1;
        b
    }

    fn fetch_signed_byte(&mut self) -> i8 {
        let b: i8 = self.mmu.borrow_mut().rb(self.registers.pc) as i8;
        self.registers.pc += 1;
        b
    }

    fn fetch_word(&mut self) -> u16 {
        (self.fetch_byte() as u16) + ((self.fetch_byte() as u16) << 8)
    }

    pub fn do_interrupts(&mut self) {
        if self.interrupt_master == true {
            let interrupt_request_register: u8 = self.mmu.borrow_mut().rb(0xFF0F); // IF
            let interrupt_enabled_register: u8 = self.mmu.borrow_mut().rb(0xFFFF); // IE
            for i in 0..5 {
                if interrupt_request_register & (1 << i) > 0
                    && interrupt_enabled_register & (1 << i) > 0
                {
                    self.service_interrupt(i);
                }
            }
        }
    }

    pub fn service_interrupt(&mut self, index: u8) {

        if self.is_halted == true {
            // println!("NOT HALTING");
            self.is_halted = false;
        }
        self.interrupt_master = false;
        let interrupt_request_register: u8 = self.mmu.borrow().rb(0xFF0F) & !(1 << index);
        self.mmu.borrow_mut().wb(0xFF0F, interrupt_request_register);
        self.stack_push(self.registers.pc);
        self.registers.pc = match index {
            0x00 => 0x40,
            0x01 => 0x48,
            0x02 => 0x50,
            0x04 => 0x60,
            _ => 0x00,
        };
    }

    pub fn execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            0x00 => 1,
            0x01 => { let w: u16 = self.fetch_word(); self.registers.set_bc(w); 3 }
            0x02 => { self.mmu.borrow_mut().wb(self.registers.bc(), self.registers.a); 2 }
            0x03 => { self.registers.set_bc(self.registers.bc().wrapping_add(1)); 2 }
            0x04 => { self.registers.b = self.alu_inc(self.registers.b); 1 }
            0x05 => { self.registers.b = self.alu_dec(self.registers.b); 1 }
            0x06 => {  self.registers.b = self.fetch_byte(); 2 }
            0x07 => { self.registers.a = self.alu_rlc(self.registers.a); self.registers.clear_flag(Flags::Zero); 1 }
            0x08 => { let w: u16 = self.fetch_word(); self.mmu.borrow_mut().ww(w, self.registers.sp); 5 }
            0x09 => { self.alu_add16(self.registers.bc()); 2 }
            0x0A => { self.registers.a = self.mmu.borrow_mut().rb(self.registers.bc()); 2 }
            0x0B => { self.registers.set_bc(self.registers.bc().wrapping_sub(1)); 2 }
            0x0C => { self.registers.c = self.alu_inc(self.registers.c); 1 }
            0x0D => { self.registers.c = self.alu_dec(self.registers.c); 1 }
            0x0E => { self.registers.c = self.fetch_byte(); 2 }
            0x0F => { self.registers.a = self.alu_rrc(self.registers.a); self.registers.clear_flag(Flags::Zero); 1 }
            0x10 => { 1 } // STOP, Check Implementation
            0x11 => { let w: u16 = self.fetch_word(); self.registers.set_de(w); 3 }
            0x12 => { self.mmu.borrow_mut().wb(self.registers.de(), self.registers.a); 2 }
            0x13 => { self.registers.set_de(self.registers.de().wrapping_add(1)); 2 }
            0x14 => { self.registers.d = self.alu_inc(self.registers.d); 1 }
            0x15 => { self.registers.d = self.alu_dec(self.registers.d); 1 }
            0x16 => { self.registers.d = self.fetch_byte(); 2 }
            0x17 => { self.registers.a = self.alu_rl(self.registers.a); self.registers.clear_flag(Flags::Zero); 1 }
            0x18 => { let b: i8 = self.fetch_signed_byte(); self.registers.pc = ((self.registers.pc as u32 as i32) + (b as i32)) as u16; 3 }
            0x19 => { self.alu_add16(self.registers.de()); 2 }
            0x1A => { self.registers.a = self.mmu.borrow_mut().rb(self.registers.de()); 2 }
            0x1B => { self.registers.set_de(self.registers.de().wrapping_sub(1)); 2 }
            0x1C => { self.registers.e = self.alu_inc(self.registers.e); 1 }
            0x1D => { self.registers.e = self.alu_dec(self.registers.e); 1 }
            0x1E => { self.registers.e = self.fetch_byte(); 2 }
            0x1F => { self.registers.a = self.alu_rr(self.registers.a); self.registers.clear_flag(Flags::Zero); 1 }
            0x20 => { let b: u8 = self.fetch_byte(); self.cpu_jr_nz_s8(b as i8) }
            0x21 => { let w: u16 = self.fetch_word(); self.registers.set_hl(w); 3 }
            0x22 => {
                self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                2
            }
            0x23 => { self.registers.set_hl(self.registers.hl().wrapping_add(1)); 2 }
            0x24 => { self.registers.h = self.alu_inc(self.registers.h); 1 }
            0x25 => { self.registers.h = self.alu_dec(self.registers.h); 1 }
            0x26 => { self.registers.h = self.fetch_byte(); 2 }
            0x27 => { self.alu_daa(); 1 }
            0x28 => { let b: i8 = self.fetch_byte() as i8; self.cpu_jr_z_s8(b) }
            0x29 => { self.alu_add16(self.registers.hl()); 2 }
            0x2A => {
                self.registers.a = self.mmu.borrow_mut().rb(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_add(1));
                2
            }
            0x2B => { self.registers.set_hl(self.registers.hl().wrapping_sub(1)); 2 }
            0x2C => { self.registers.l = self.alu_inc(self.registers.l); 1 }
            0x2D => { self.registers.l = self.alu_dec(self.registers.l); 1 }
            0x2E => { self.registers.l = self.fetch_byte(); 2 }
            0x2F => {
                self.registers.a = !self.registers.a;
                self.registers.set_flag(Flags::Subtract);
                self.registers.set_flag(Flags::HalfCarry);
                1
            }
            0x30 => {
                let b = self.fetch_byte() as i8;
                let carry: u8 = self.registers.get_flag(Flags::Carry);
                if carry == 0 {
                    self.registers.pc = ((self.registers.pc as u32 as i32) + (b as i32)) as u16;
                    return 3;
                } else {
                    return 2;
                }
            }
            0x31 => { self.registers.sp = self.fetch_word(); 3 }
            0x32 => {
                self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.a);
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                2
            }
            0x33 => { self.registers.sp = self.registers.sp.wrapping_add(1); 2 }
            0x34 => {
                let num: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                let res: u8 = self.alu_inc(num);
                self.mmu.borrow_mut().wb(self.registers.hl(), res);
                3
            }
            0x35 => {
                let num: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                let res: u8 = self.alu_dec(num);
                self.mmu.borrow_mut().wb(self.registers.hl(), res);
                3
            }
            0x36 => { let b: u8 = self.fetch_byte(); self.mmu.borrow_mut().wb(self.registers.hl(), b); 3 }
            0x37 => {
                self.registers.clear_flag(Flags::Subtract);
                self.registers.clear_flag(Flags::HalfCarry);
                self.registers.set_flag(Flags::Carry);
                1
            }
            0x38 => { let b: u8 = self.fetch_byte(); return self.cpu_c_s8(b); }
            0x39 => { self.alu_add16(self.registers.sp); 2 }
            0x3A => {
                self.registers.a = self.mmu.borrow().rb(self.registers.hl());
                self.registers.set_hl(self.registers.hl().wrapping_sub(1));
                2
            }
            0x3B => { self.registers.sp = self.registers.sp.wrapping_sub(1); 2 }
            0x3C => { self.registers.a = self.alu_inc(self.registers.a); 1 }
            0x3D => { self.registers.a = self.alu_dec(self.registers.a); 1 }
            0x3E => { self.registers.a = self.fetch_byte(); 2 }
            0x3F => {
                self.registers.clear_flag(Flags::Subtract);
                self.registers.clear_flag(Flags::HalfCarry);
                if self.registers.get_flag(Flags::Carry) > 0 {
                    self.registers.clear_flag(Flags::Carry);
                } else {
                    self.registers.set_flag(Flags::Carry);
                }
                1
            }
            0x40 => { self.registers.b = self.registers.b; 1 }
            0x41 => { self.registers.b = self.registers.c; 1 }
            0x42 => { self.registers.b = self.registers.d; 1 }
            0x43 => { self.registers.b = self.registers.e; 1 }
            0x44 => { self.registers.b = self.registers.h; 1 }
            0x45 => { self.registers.b = self.registers.l; 1 }
            0x46 => { self.registers.b = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x47 => { self.registers.b = self.registers.a; 1 }
            0x48 => { self.registers.c = self.registers.b; 1 }
            0x49 => { self.registers.c = self.registers.c; 1 }
            0x4A => { self.registers.c = self.registers.d; 1 }
            0x4B => { self.registers.c = self.registers.e; 1 }
            0x4C => { self.registers.c = self.registers.h; 1 }
            0x4D => { self.registers.c = self.registers.l; 1 }
            0x4E => { self.registers.c = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x4F => { self.registers.c = self.registers.a; 1 }
            0x50 => { self.registers.d = self.registers.b; 1 }
            0x51 => { self.registers.d = self.registers.c; 1 }
            0x52 => { self.registers.d = self.registers.d; 1 }
            0x53 => { self.registers.d = self.registers.e; 1 }
            0x54 => { self.registers.d = self.registers.h; 1 }
            0x55 => { self.registers.d = self.registers.l; 1 }
            0x56 => { self.registers.d = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x57 => { self.registers.d = self.registers.a; 1 }
            0x58 => { self.registers.e = self.registers.b; 1 }
            0x59 => { self.registers.e = self.registers.c; 1 }
            0x5A => { self.registers.e = self.registers.d; 1 }
            0x5B => { self.registers.e = self.registers.e; 1 }
            0x5C => { self.registers.e = self.registers.h; 1 }
            0x5D => { self.registers.e = self.registers.l; 1 }
            0x5E => { self.registers.e = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x5F => { self.registers.e = self.registers.a; 1 }
            0x60 => { self.registers.h = self.registers.b; 1 }
            0x61 => { self.registers.h = self.registers.c; 1 }
            0x62 => { self.registers.h = self.registers.d; 1 }
            0x63 => { self.registers.h = self.registers.e; 1 }
            0x64 => { self.registers.h = self.registers.h; 1 }
            0x65 => { self.registers.h = self.registers.l; 1 }
            0x66 => { self.registers.h = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x67 => { self.registers.h = self.registers.a; 1 }
            0x68 => { self.registers.l = self.registers.b; 1 }
            0x69 => { self.registers.l = self.registers.c; 1 }
            0x6A => { self.registers.l = self.registers.d; 1 }
            0x6B => { self.registers.l = self.registers.e; 1 }
            0x6C => { self.registers.l = self.registers.h; 1 }
            0x6D => { self.registers.l = self.registers.l; 1 }
            0x6E => { self.registers.l = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x6F => { self.registers.l = self.registers.a; 1 }
            0x70 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.b); 2 }
            0x71 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.c); 2 }
            0x72 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.d); 2 }
            0x73 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.e); 2 }
            0x74 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.h); 2 }
            0x75 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.l); 2 }
            0x76 => { self.is_halted = true; 1 }
            0x77 => { self.mmu.borrow_mut().wb(self.registers.hl(), self.registers.a); 2 }
            0x78 => { self.registers.a = self.registers.b; 1 }
            0x79 => { self.registers.a = self.registers.c; 1 }
            0x7A => { self.registers.a = self.registers.d; 1 }
            0x7B => { self.registers.a = self.registers.e; 1 }
            0x7C => { self.registers.a = self.registers.h; 1 }
            0x7D => { self.registers.a = self.registers.l; 1 }
            0x7E => { self.registers.a = self.mmu.borrow().rb(self.registers.hl()); 2 }
            0x7F => { self.registers.a = self.registers.a; 1 }
            0x80 => { self.alu_add(self.registers.b); 1 }
            0x81 => { self.alu_add(self.registers.c); 1 }
            0x82 => { self.alu_add(self.registers.d); 1 }
            0x83 => { self.alu_add(self.registers.e); 1 }
            0x84 => { self.alu_add(self.registers.h); 1 }
            0x85 => { self.alu_add(self.registers.l); 1 }
            0x86 => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_add(v); 2 }
            0x87 => { self.alu_add(self.registers.a); 1 }
            0x88 => { self.alu_adc(self.registers.b); 1 }
            0x89 => { self.alu_adc(self.registers.c); 1 }
            0x8A => { self.alu_adc(self.registers.d); 1 }
            0x8B => { self.alu_adc(self.registers.e); 1 }
            0x8C => { self.alu_adc(self.registers.h); 1 }
            0x8D => { self.alu_adc(self.registers.l); 1 }
            0x8E => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_adc(v); 2 }
            0x8F => { self.alu_adc(self.registers.a); 1 }
            0x90 => { self.alu_sub(self.registers.b); 1 }
            0x91 => { self.alu_sub(self.registers.c); 1 }
            0x92 => { self.alu_sub(self.registers.d); 1 }
            0x93 => { self.alu_sub(self.registers.e); 1 }
            0x94 => { self.alu_sub(self.registers.h); 1 }
            0x95 => { self.alu_sub(self.registers.l); 1 }
            0x96 => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_sub(v); 2 }
            0x97 => { self.alu_sub(self.registers.a); 1 }
            0x98 => { self.alu_sbc(self.registers.b); 1 }
            0x99 => { self.alu_sbc(self.registers.c); 1 }
            0x9A => { self.alu_sbc(self.registers.d); 1 }
            0x9B => { self.alu_sbc(self.registers.e); 1 }
            0x9C => { self.alu_sbc(self.registers.h); 1 }
            0x9D => { self.alu_sbc(self.registers.l); 1 }
            0x9E => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_sbc(v); 2 }
            0x9F => { self.alu_sbc(self.registers.a); 1 }
            0xA0 => { self.alu_and(self.registers.b); 1 }
            0xA1 => { self.alu_and(self.registers.c); 1 }
            0xA2 => { self.alu_and(self.registers.d); 1 }
            0xA3 => { self.alu_and(self.registers.e); 1 }
            0xA4 => { self.alu_and(self.registers.h); 1 }
            0xA5 => { self.alu_and(self.registers.l); 1 }
            0xA6 => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_and(v); 2 }
            0xA7 => { self.alu_and(self.registers.a); 1 }
            0xA8 => { self.alu_xor(self.registers.b); 1 }
            0xA9 => { self.alu_xor(self.registers.c); 1 }
            0xAA => { self.alu_xor(self.registers.d); 1 }
            0xAB => { self.alu_xor(self.registers.e); 1 }
            0xAC => { self.alu_xor(self.registers.h); 1 }
            0xAD => { self.alu_xor(self.registers.l); 1 }
            0xAE => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_xor(v); 2 }
            0xAF => { self.alu_xor(self.registers.a); 1 }
            0xB0 => { self.alu_or(self.registers.b); 1 }
            0xB1 => { self.alu_or(self.registers.c); 1 }
            0xB2 => { self.alu_or(self.registers.d); 1 }
            0xB3 => { self.alu_or(self.registers.e); 1 }
            0xB4 => { self.alu_or(self.registers.h); 1 }
            0xB5 => { self.alu_or(self.registers.l); 1 }
            0xB6 => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_or(v); 2 }
            0xB7 => { self.alu_or(self.registers.a); 1 }
            0xB8 => { self.alu_cp(self.registers.b); 1 }
            0xB9 => { self.alu_cp(self.registers.c); 1 }
            0xBA => { self.alu_cp(self.registers.d); 1 }
            0xBB => { self.alu_cp(self.registers.e); 1 }
            0xBC => { self.alu_cp(self.registers.h); 1 }
            0xBD => { self.alu_cp(self.registers.l); 1 }
            0xBE => { let v = self.mmu.borrow().rb(self.registers.hl()); self.alu_cp(v); 2 }
            0xBF => { self.alu_cp(self.registers.a); 1 }
            0xC0 => self.cpu_ret_nz(),
            0xC1 => { let v: u16 = self.stack_pop(); self.registers.set_bc(v); 3 }
            0xC2 => { let v: u16 = self.fetch_word(); self.jp_nz_a16(v) }
            0xC3 => { let w = self.fetch_word(); self.jp_a16(w); 4 }
            0xC4 => { let w: u16 = self.fetch_word(); self.call_nz_a16(w) }
            0xC5 => { self.stack_push(self.registers.bc()); 4 }
            0xC6 => { let b: u8 = self.fetch_byte(); self.alu_add(b); 2 }
            0xC7 => self.rst(opcode),
            0xC8 => self.ret_z(),
            0xC9 => { self.ret(); 4 }
            0xCA => { let w: u16 = self.fetch_word(); self.jp_z_a16(w) }
            0xCB => self.call_cb(),
            0xCC => { let w: u16 = self.fetch_word(); self.call_z_a16(w) }
            0xCD => { let w: u16 = self.fetch_word(); self.call_a16(w); 6 }
            0xCE => { let b: u8 = self.fetch_byte(); self.alu_adc(b); 2 }
            0xCF => self.rst(opcode),
            0xD0 => self.ret_nc(),
            0xD1 => { let v: u16 = self.stack_pop(); self.registers.set_de(v); 3 }
            0xD2 => { let v: u16 = self.fetch_word(); self.jp_nc_a16(v) }
            0xD4 => { let v: u16 = self.fetch_word(); self.call_nc_a16(v) }
            0xD5 => { self.stack_push(self.registers.de()); 4 }
            0xD6 => { let b: u8 = self.fetch_byte();self.alu_sub(b); 2 }
            0xD7 => self.rst(opcode),
            0xD8 => self.ret_c(),
            0xD9 => { self.interrupt_master = true; self.registers.pc = self.stack_pop(); 4 }
            0xDA => { let w: u16 = self.fetch_word(); self.jp_c_a16(w) }
            0xDC => { let w: u16 = self.fetch_word(); self.call_c_a16(w) }
            0xDE => { let b: u8 = self.fetch_byte(); self.alu_sbc(b); 2 }
            0xDF => self.rst(opcode),
            0xE0 => {
                let b: u16 = self.fetch_byte() as u16;
                let res: u16 = (0xFF << 8) | b as u16;
                self.mmu.borrow_mut().wb(res, self.registers.a);
                3
            }
            0xE1 => { let v: u16 = self.stack_pop(); self.registers.set_hl(v); 3 }
            0xE2 => { let v: u16 = (0xFF << 8) | self.registers.c as u16; self.mmu.borrow_mut().wb(v, self.registers.a); 2 }
            0xE5 => { self.stack_push(self.registers.hl()); 4 }
            0xE6 => { let b: u8 = self.fetch_byte(); self.alu_and(b); 2 }
            0xE7 => self.rst(opcode),
            0xE8 => {
                let b: u16 = self.fetch_signed_byte() as i8 as i16 as u16;
                self.registers.clear_flag(Flags::Zero);
                self.registers.clear_flag(Flags::Subtract);
                if (self.registers.sp & 0x000F) + (b & 0x000F) > 0x000F {
                    self.registers.set_flag(Flags::HalfCarry);
                } else {
                    self.registers.clear_flag(Flags::HalfCarry);
                }

                if (self.registers.sp & 0x00FF) + (b & 0x00FF) > 0x00FF {
                    self.registers.set_flag(Flags::Carry);
                } else {
                    self.registers.clear_flag(Flags::Carry);
                }

                self.registers.sp = self.registers.sp.wrapping_add(b);
                4
            }
            0xE9 => { self.registers.pc = self.registers.hl(); 1 }
            0xEA => { let w: u16 = self.fetch_word(); self.mmu.borrow_mut().wb(w, self.registers.a); 4 }
            0xEE => { let b: u8 = self.fetch_byte(); self.alu_xor(b); 2 }
            0xEF => self.rst(opcode),
            0xF0 => { let v: u16 = 0xFF00 | self.fetch_byte() as u16; self.registers.a = self.mmu.borrow().rb(v); 3 }
            0xF1 => { let v: u16 = self.stack_pop(); self.registers.set_af(v); 3 }
            0xF2 => { let v: u16 = 0xFF00 | self.registers.c as u16; self.registers.a = self.mmu.borrow().rb(v); 2 }
            0xF3 => { self.interrupt_master = false; 1 }
            0xF5 => { self.stack_push(self.registers.af()); 4 }
            0xF6 => { let b: u8 = self.fetch_byte(); self.alu_or(b); 2 }
            0xF7 => self.rst(opcode),
            0xF8 => {
                let b: u16 = self.fetch_signed_byte() as i8 as i16 as u16;
                self.registers.clear_flag(Flags::Zero);
                self.registers.clear_flag(Flags::Subtract);
                if (self.registers.sp & 0x000F) + (b & 0x000F) > 0x000F {
                    self.registers.set_flag(Flags::HalfCarry);
                } else {
                    self.registers.clear_flag(Flags::HalfCarry);
                }

                if (self.registers.sp & 0x00FF) + (b & 0x00FF) > 0x00FF {
                    self.registers.set_flag(Flags::Carry);
                } else {
                    self.registers.clear_flag(Flags::Carry);
                }

                self.registers.set_hl(self.registers.sp.wrapping_add(b));
                3
            }
            0xF9 => { self.registers.sp = self.registers.hl(); 2 }
            0xFA => { let w: u16 = self.fetch_word(); self.registers.a = self.mmu.borrow().rb(w); 4 }
            0xFB => { self.interrupt_master = true; 1 }
            0xFE => {
                let b: u8 = self.fetch_byte();
                let res = self.registers.a.wrapping_sub(b);

                if res == 0 {
                    self.registers.set_flag(Flags::Zero);
                } else {
                    self.registers.clear_flag(Flags::Zero);
                }

                self.registers.set_flag(Flags::Subtract);

                if (self.registers.a & 0x0F) < (b & 0x0F) {
                    self.registers.set_flag(Flags::HalfCarry);
                } else {
                    self.registers.clear_flag(Flags::HalfCarry);
                }

                if (self.registers.a as u16) < (b as u16) {
                    self.registers.set_flag(Flags::Carry);
                } else {
                    self.registers.clear_flag(Flags::Carry);
                }

                2
            }
            0xFF => self.rst(opcode),
            _ => {
                panic!("UNKNOWN INSTRUCTION {:x}", opcode);
            }
        }
    }

    fn call_cb(&mut self) -> u8 {
        let opcode = self.fetch_byte();
        match opcode {
            0x00 => { self.registers.b = self.alu_rlc(self.registers.b); 2 }
            0x01 => { self.registers.c = self.alu_rlc(self.registers.c); 2 }
            0x02 => { self.registers.d = self.alu_rlc(self.registers.d); 2 }
            0x03 => { self.registers.e = self.alu_rlc(self.registers.e); 2 }
            0x04 => { self.registers.h = self.alu_rlc(self.registers.h); 2 }
            0x05 => { self.registers.l = self.alu_rlc(self.registers.l); 2 }
            0x06 => {
                // Try to minimise this
                let mut v: u8 = self.mmu.borrow().rb(self.registers.hl());
                v = self.alu_rlc(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x07 => { self.registers.a = self.alu_rlc(self.registers.a); 2 }
            0x08 => { self.registers.b = self.alu_rrc(self.registers.b); 2 }
            0x09 => { self.registers.c = self.alu_rrc(self.registers.c); 2 }
            0x0A => { self.registers.d = self.alu_rrc(self.registers.d); 2 }
            0x0B => { self.registers.e = self.alu_rrc(self.registers.e); 2 }
            0x0C => { self.registers.h = self.alu_rrc(self.registers.h); 2 }
            0x0D => { self.registers.l = self.alu_rrc(self.registers.l); 2 }
            0x0E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_rrc(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x0F => { self.registers.a = self.alu_rrc(self.registers.a); 2 }
            0x10 => { self.registers.b = self.alu_rl(self.registers.b); 2 }
            0x11 => { self.registers.c = self.alu_rl(self.registers.c); 2 }
            0x12 => { self.registers.d = self.alu_rl(self.registers.d); 2 }
            0x13 => { self.registers.e = self.alu_rl(self.registers.e); 2 }
            0x14 => { self.registers.h = self.alu_rl(self.registers.h); 2 }
            0x15 => { self.registers.l = self.alu_rl(self.registers.l); 2 }
            0x16 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_rl(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x17 => { self.registers.a = self.alu_rl(self.registers.a); 2 }
            0x18 => { self.registers.b = self.alu_rr(self.registers.b); 2 }
            0x19 => { self.registers.c = self.alu_rr(self.registers.c); 2 }
            0x1A => { self.registers.d = self.alu_rr(self.registers.d); 2 }
            0x1B => { self.registers.e = self.alu_rr(self.registers.e); 2 }
            0x1C => { self.registers.h = self.alu_rr(self.registers.h); 2 }
            0x1D => { self.registers.l = self.alu_rr(self.registers.l); 2 }
            0x1E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_rr(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x1F => { self.registers.a = self.alu_rr(self.registers.a); 2 }
            0x20 => { self.registers.b = self.alu_sla(self.registers.b); 2 }
            0x21 => { self.registers.c = self.alu_sla(self.registers.c); 2 }
            0x22 => { self.registers.d = self.alu_sla(self.registers.d); 2 }
            0x23 => { self.registers.e = self.alu_sla(self.registers.e); 2 }
            0x24 => { self.registers.h = self.alu_sla(self.registers.h); 2 }
            0x25 => { self.registers.l = self.alu_sla(self.registers.l); 2 }
            0x26 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_sla(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x27 => { self.registers.a = self.alu_sla(self.registers.a); 2 }
            0x28 => { self.registers.b = self.alu_sra(self.registers.b); 2 }
            0x29 => { self.registers.c = self.alu_sra(self.registers.c); 2 }
            0x2A => { self.registers.d = self.alu_sra(self.registers.d); 2 }
            0x2B => { self.registers.e = self.alu_sra(self.registers.e); 2 }
            0x2C => { self.registers.h = self.alu_sra(self.registers.h); 2 }
            0x2D => { self.registers.l = self.alu_sra(self.registers.l); 2 }
            0x2E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_sra(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x2F => { self.registers.a = self.alu_sra(self.registers.a); 2 }
            0x30 => { self.registers.b = self.alu_swap(self.registers.b); 2 }
            0x31 => { self.registers.c = self.alu_swap(self.registers.c); 2 }
            0x32 => { self.registers.d = self.alu_swap(self.registers.d); 2 }
            0x33 => { self.registers.e = self.alu_swap(self.registers.e); 2 }
            0x34 => { self.registers.h = self.alu_swap(self.registers.h); 2 }
            0x35 => { self.registers.l = self.alu_swap(self.registers.l); 2 }
            0x36 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_swap(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x37 => { self.registers.a = self.alu_swap(self.registers.a); 2 }
            0x38 => { self.registers.b = self.alu_srl(self.registers.b); 2 }
            0x39 => { self.registers.c = self.alu_srl(self.registers.c); 2 }
            0x3A => { self.registers.d = self.alu_srl(self.registers.d); 2 }
            0x3B => { self.registers.e = self.alu_srl(self.registers.e); 2 }
            0x3C => { self.registers.h = self.alu_srl(self.registers.h); 2 }
            0x3D => { self.registers.l = self.alu_srl(self.registers.l); 2 }
            0x3E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_srl(v);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x3F => { self.registers.a = self.alu_srl(self.registers.a); 2 }
            0x40 => { self.alu_bit(self.registers.b, 0); 2 }
            0x41 => { self.alu_bit(self.registers.c, 0); 2 }
            0x42 => { self.alu_bit(self.registers.d, 0); 2 }
            0x43 => { self.alu_bit(self.registers.e, 0); 2 }
            0x44 => { self.alu_bit(self.registers.h, 0); 2 }
            0x45 => { self.alu_bit(self.registers.l, 0); 2 }
            0x46 => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 0);
                3
            }
            0x47 => {  self.alu_bit(self.registers.a, 0); 2 }
            0x48 => { self.alu_bit(self.registers.b, 1); 2 }
            0x49 => { self.alu_bit(self.registers.c, 1); 2 }
            0x4A => { self.alu_bit(self.registers.d, 1); 2 }
            0x4B => { self.alu_bit(self.registers.e, 1); 2 }
            0x4C => { self.alu_bit(self.registers.h, 1); 2 }
            0x4D => { self.alu_bit(self.registers.l, 1); 2 }
            0x4E => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 1);
                3
            }
            0x4F => { self.alu_bit(self.registers.a, 1); 2 }
            0x50 => { self.alu_bit(self.registers.b, 2); 2 }
            0x51 => { self.alu_bit(self.registers.c, 2); 2 }
            0x52 => { self.alu_bit(self.registers.d, 2); 2 }
            0x53 => { self.alu_bit(self.registers.e, 2); 2 }
            0x54 => { self.alu_bit(self.registers.h, 2); 2 }
            0x55 => { self.alu_bit(self.registers.l, 2); 2 }
            0x56 => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 2);
                3
            }
            0x57 => { self.alu_bit(self.registers.a, 2); 2 }
            0x58 => { self.alu_bit(self.registers.b, 3); 2 }
            0x59 => { self.alu_bit(self.registers.c, 3); 2 }
            0x5A => { self.alu_bit(self.registers.d, 3); 2 }
            0x5B => { self.alu_bit(self.registers.e, 3); 2 }
            0x5C => { self.alu_bit(self.registers.h, 3); 2 }
            0x5D => { self.alu_bit(self.registers.l, 3); 2 }
            0x5E => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 3);
                3
            }
            0x5F => { self.alu_bit(self.registers.a, 3); 2 }
            0x60 => { self.alu_bit(self.registers.b, 4); 2 }
            0x61 => { self.alu_bit(self.registers.c, 4); 2 }
            0x62 => { self.alu_bit(self.registers.d, 4); 2 }
            0x63 => { self.alu_bit(self.registers.e, 4); 2 }
            0x64 => { self.alu_bit(self.registers.h, 4); 2 }
            0x65 => { self.alu_bit(self.registers.l, 4); 2 }
            0x66 => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 4);
                3
            }
            0x67 => { self.alu_bit(self.registers.a, 4); 2 }
            0x68 => { self.alu_bit(self.registers.b, 5); 2 }
            0x69 => { self.alu_bit(self.registers.c, 5); 2 }
            0x6A => { self.alu_bit(self.registers.d, 5); 2 }
            0x6B => { self.alu_bit(self.registers.e, 5); 2 }
            0x6C => { self.alu_bit(self.registers.h, 5); 2 }
            0x6D => { self.alu_bit(self.registers.l, 5); 2 }
            0x6E => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 5);
                3
            }
            0x6F => { self.alu_bit(self.registers.a, 5); 2 }
            0x70 => { self.alu_bit(self.registers.b, 6); 2 }
            0x71 => { self.alu_bit(self.registers.c, 6); 2 }
            0x72 => { self.alu_bit(self.registers.d, 6); 2 }
            0x73 => { self.alu_bit(self.registers.e, 6); 2 }
            0x74 => { self.alu_bit(self.registers.h, 6); 2 }
            0x75 => { self.alu_bit(self.registers.l, 6); 2 }
            0x76 => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 6);
                3
            }
            0x77 => { self.alu_bit(self.registers.a, 6); 2 }
            0x78 => { self.alu_bit(self.registers.b, 7); 2 }
            0x79 => { self.alu_bit(self.registers.c, 7); 2 }
            0x7A => { self.alu_bit(self.registers.d, 7); 2 }
            0x7B => { self.alu_bit(self.registers.e, 7); 2 }
            0x7C => { self.alu_bit(self.registers.h, 7); 2 }
            0x7D => { self.alu_bit(self.registers.l, 7); 2 }
            0x7E => {
                let v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                self.alu_bit(v, 7);
                3
            }
            0x7F => { self.alu_bit(self.registers.a, 7); 2 }
            0x80 => { self.registers.b = self.alu_res(self.registers.b, 0); 2 }
            0x81 => { self.registers.c = self.alu_res(self.registers.c, 0); 2 }
            0x82 => { self.registers.d = self.alu_res(self.registers.d, 0); 2 }
            0x83 => { self.registers.e = self.alu_res(self.registers.e, 0); 2 }
            0x84 => { self.registers.h = self.alu_res(self.registers.h, 0); 2 }
            0x85 => { self.registers.l = self.alu_res(self.registers.l, 0); 2 }
            0x86 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 0);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x87 => { self.registers.a = self.alu_res(self.registers.a, 0); 2 }
            0x88 => { self.registers.b = self.alu_res(self.registers.b, 1); 2 }
            0x89 => { self.registers.c = self.alu_res(self.registers.c, 1); 2 }
            0x8A => { self.registers.d = self.alu_res(self.registers.d, 1); 2 }
            0x8B => { self.registers.e = self.alu_res(self.registers.e, 1); 2 }
            0x8C => { self.registers.h = self.alu_res(self.registers.h, 1); 2 }
            0x8D => { self.registers.l = self.alu_res(self.registers.l, 1); 2 }
            0x8E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 1);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x8F => { self.registers.a = self.alu_res(self.registers.a, 1); 2 }
            0x90 => { self.registers.b = self.alu_res(self.registers.b, 2); 2 }
            0x91 => { self.registers.c = self.alu_res(self.registers.c, 2); 2 }
            0x92 => { self.registers.d = self.alu_res(self.registers.d, 2); 2 }
            0x93 => { self.registers.e = self.alu_res(self.registers.e, 2); 2 }
            0x94 => { self.registers.h = self.alu_res(self.registers.h, 2); 2 }
            0x95 => { self.registers.l = self.alu_res(self.registers.l, 2); 2 }
            0x96 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 2);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x97 => { self.registers.a = self.alu_res(self.registers.a, 2); 2 }
            0x98 => { self.registers.b = self.alu_res(self.registers.b, 3); 2 }
            0x99 => { self.registers.c = self.alu_res(self.registers.c, 3); 2 }
            0x9A => { self.registers.d = self.alu_res(self.registers.d, 3); 2 }
            0x9B => { self.registers.e = self.alu_res(self.registers.e, 3); 2 }
            0x9C => { self.registers.h = self.alu_res(self.registers.h, 3); 2 }
            0x9D => { self.registers.l = self.alu_res(self.registers.l, 3); 2 }
            0x9E => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 3);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0x9F => { self.registers.a = self.alu_res(self.registers.a, 3); 2 }
            0xA0 => { self.registers.b = self.alu_res(self.registers.b, 4); 2 }
            0xA1 => { self.registers.c = self.alu_res(self.registers.c, 4); 2 }
            0xA2 => { self.registers.d = self.alu_res(self.registers.d, 4); 2 }
            0xA3 => { self.registers.e = self.alu_res(self.registers.e, 4); 2 }
            0xA4 => { self.registers.h = self.alu_res(self.registers.h, 4); 2 }
            0xA5 => { self.registers.l = self.alu_res(self.registers.l, 4); 2 }
            0xA6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 4);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xA7 => { self.registers.a = self.alu_res(self.registers.a, 4); 2 }
            0xA8 => { self.registers.b = self.alu_res(self.registers.b, 5); 2 }
            0xA9 => { self.registers.c = self.alu_res(self.registers.c, 5); 2 }
            0xAA => { self.registers.d = self.alu_res(self.registers.d, 5); 2 }
            0xAB => { self.registers.e = self.alu_res(self.registers.e, 5); 2 }
            0xAC => { self.registers.h = self.alu_res(self.registers.h, 5); 2 }
            0xAD => { self.registers.l = self.alu_res(self.registers.l, 5); 2 }
            0xAE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 5);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xAF => { self.registers.a = self.alu_res(self.registers.a, 5); 2 }
            0xB0 => { self.registers.b = self.alu_res(self.registers.b, 6); 2 }
            0xB1 => { self.registers.c = self.alu_res(self.registers.c, 6); 2 }
            0xB2 => { self.registers.d = self.alu_res(self.registers.d, 6); 2 }
            0xB3 => { self.registers.e = self.alu_res(self.registers.e, 6); 2 }
            0xB4 => { self.registers.h = self.alu_res(self.registers.h, 6); 2 }
            0xB5 => { self.registers.l = self.alu_res(self.registers.l, 6); 2 }
            0xB6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 6);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xB7 => { self.registers.a = self.alu_res(self.registers.a, 6); 2 }
            0xB8 => { self.registers.b = self.alu_res(self.registers.b, 7); 2 }
            0xB9 => { self.registers.c = self.alu_res(self.registers.c, 7); 2 }
            0xBA => { self.registers.d = self.alu_res(self.registers.d, 7); 2 }
            0xBB => { self.registers.e = self.alu_res(self.registers.e, 7); 2 }
            0xBC => { self.registers.h = self.alu_res(self.registers.h, 7); 2 }
            0xBD => { self.registers.l = self.alu_res(self.registers.l, 7); 2 }
            0xBE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_res(v, 7);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xBF => { self.registers.a = self.alu_res(self.registers.a, 7); 2 }
            0xC0 => { self.registers.b = self.alu_set(self.registers.b, 0); 2 }
            0xC1 => { self.registers.c = self.alu_set(self.registers.c, 0); 2 }
            0xC2 => { self.registers.d = self.alu_set(self.registers.d, 0); 2 }
            0xC3 => { self.registers.e = self.alu_set(self.registers.e, 0); 2 }
            0xC4 => { self.registers.h = self.alu_set(self.registers.h, 0); 2 }
            0xC5 => { self.registers.l = self.alu_set(self.registers.l, 0); 2 }
            0xC6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 0);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xC7 => { self.registers.a = self.alu_set(self.registers.a, 0); 2 }
            0xC8 => { self.registers.b = self.alu_set(self.registers.b, 1); 2 }
            0xC9 => { self.registers.c = self.alu_set(self.registers.c, 1); 2 }
            0xCA => { self.registers.d = self.alu_set(self.registers.d, 1); 2 }
            0xCB => { self.registers.e = self.alu_set(self.registers.e, 1); 2 }
            0xCC => { self.registers.h = self.alu_set(self.registers.h, 1); 2 }
            0xCD => { self.registers.l = self.alu_set(self.registers.l, 1); 2 }
            0xCE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 1);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xCF => { self.registers.a = self.alu_set(self.registers.a, 1); 2 }
            0xD0 => { self.registers.b = self.alu_set(self.registers.b, 2); 2 }
            0xD1 => { self.registers.c = self.alu_set(self.registers.c, 2); 2 }
            0xD2 => { self.registers.d = self.alu_set(self.registers.d, 2); 2 }
            0xD3 => { self.registers.e = self.alu_set(self.registers.e, 2); 2 }
            0xD4 => { self.registers.h = self.alu_set(self.registers.h, 2); 2 }
            0xD5 => { self.registers.l = self.alu_set(self.registers.l, 2); 2 }
            0xD6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 2);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xD7 => { self.registers.a = self.alu_set(self.registers.a, 2); 2 }
            0xD8 => { self.registers.b = self.alu_set(self.registers.b, 3); 2 }
            0xD9 => { self.registers.c = self.alu_set(self.registers.c, 3); 2 }
            0xDA => { self.registers.d = self.alu_set(self.registers.d, 3); 2 }
            0xDB => { self.registers.e = self.alu_set(self.registers.e, 3); 2 }
            0xDC => { self.registers.h = self.alu_set(self.registers.h, 3); 2 }
            0xDD => { self.registers.l = self.alu_set(self.registers.l, 3); 2 }
            0xDE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 3);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xDF => { self.registers.a = self.alu_set(self.registers.a, 3); 2 }
            0xE0 => { self.registers.b = self.alu_set(self.registers.b, 4); 2 }
            0xE1 => { self.registers.c = self.alu_set(self.registers.c, 4); 2 }
            0xE2 => { self.registers.d = self.alu_set(self.registers.d, 4); 2 }
            0xE3 => { self.registers.e = self.alu_set(self.registers.e, 4); 2 }
            0xE4 => { self.registers.h = self.alu_set(self.registers.h, 4); 2 }
            0xE5 => { self.registers.l = self.alu_set(self.registers.l, 4); 2 }
            0xE6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 4);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xE7 => { self.registers.a = self.alu_set(self.registers.a, 4); 2 }
            0xE8 => { self.registers.b = self.alu_set(self.registers.b, 5); 2 }
            0xE9 => { self.registers.c = self.alu_set(self.registers.c, 5); 2 }
            0xEA => { self.registers.d = self.alu_set(self.registers.d, 5); 2 }
            0xEB => { self.registers.e = self.alu_set(self.registers.e, 5); 2 }
            0xEC => { self.registers.h = self.alu_set(self.registers.h, 5); 2 }
            0xED => { self.registers.l = self.alu_set(self.registers.l, 5); 2 }
            0xEE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 5);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xEF => { self.registers.a = self.alu_set(self.registers.a, 5); 2 }
            0xF0 => { self.registers.b = self.alu_set(self.registers.b, 6); 2 }
            0xF1 => { self.registers.c = self.alu_set(self.registers.c, 6); 2 }
            0xF2 => { self.registers.d = self.alu_set(self.registers.d, 6); 2 }
            0xF3 => { self.registers.e = self.alu_set(self.registers.e, 6); 2 }
            0xF4 => { self.registers.h = self.alu_set(self.registers.h, 6); 2 }
            0xF5 => { self.registers.l = self.alu_set(self.registers.l, 6); 2 }
            0xF6 => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 6);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xF7 => { self.registers.a = self.alu_set(self.registers.a, 6); 2 }
            0xF8 => { self.registers.b = self.alu_set(self.registers.b, 7); 2 }
            0xF9 => { self.registers.c = self.alu_set(self.registers.c, 7); 2 }
            0xFA => { self.registers.d = self.alu_set(self.registers.d, 7); 2 }
            0xFB => { self.registers.e = self.alu_set(self.registers.e, 7); 2 }
            0xFC => { self.registers.h = self.alu_set(self.registers.h, 7); 2 }
            0xFD => { self.registers.l = self.alu_set(self.registers.l, 7); 2 }
            0xFE => {
                let mut v: u8 = self.mmu.borrow_mut().rb(self.registers.hl());
                v = self.alu_set(v, 7);
                self.mmu.borrow_mut().wb(self.registers.hl(), v);
                4
            }
            0xFF => { self.registers.a = self.alu_set(self.registers.a, 7); 2 }
            // _ => 1,
        }
    }

    fn alu_inc(&mut self, v: u8) -> u8 {
        let res: u8 = v.wrapping_add(1);
        self.cpu_zero_check(res);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (v & 0xf) + (1 & 0xf) > 0xf);
        res
    }

    fn alu_dec(&mut self, v: u8) -> u8 {
        let res = v.wrapping_sub(1);
        self.cpu_zero_check(res);
        self.registers.set_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (v & 0x0F) == 0);
        res
    }

    fn alu_add16(&mut self, v: u16) {
        let res = self.registers.hl().wrapping_add(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (v & 0x07FF) + (self.registers.hl() & 0x07FF) > 0x07FF);
        self.registers.flag(Flags::Carry, self.registers.hl() > 0xFFFF - v);
        self.registers.set_hl(res);
    }

    fn alu_rlc(&mut self, mut v: u8) -> u8 {
        let carry: u8 = v >> 7;
        v = v << 1;
        self.registers.change_flag(Flags::Carry, carry);
        v = (v & (!(1 << 0))) | (carry << 0);
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Subtract);
        v
    }

    fn alu_rrc(&mut self, v: u8) -> u8 {
        let c = v & 0x01;
        let r = (v >> 1) | (if c > 0 { 0x80 } else { 0 });
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.cpu_zero_check(r);
        self.registers.flag(Flags::Carry, c == 0x01);
        return r;
    }

    fn alu_rl(&mut self, mut v: u8) -> u8 {
        let new_carry: u8 = v & 0x80;
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        v = v << 1;
        if carry > 0 { v |= 0x1; }
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.flag(Flags::Carry, new_carry > 0);
        v
    }

    fn alu_rr(&mut self, v: u8) -> u8 {
        let c = v & 0x01 == 0x01;
        let r = (v >> 1)
            | (if self.registers.get_flag(Flags::Carry) > 0 {
                0x80
            } else {
                0
            });
        self.cpu_zero_check(r);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.flag(Flags::Carry, c);
        return r;
    }

    fn alu_add(&mut self, b: u8) {
        let temp = self.registers.a.wrapping_add(b);
        self.cpu_zero_check(temp);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (self.registers.a & 0xf) + (b & 0xf) > 0xf);
        self.registers.flag(Flags::Carry, (self.registers.a as u16) + (b as u16) > 0xFF);
        self.registers.a = temp;
    }

    fn alu_adc(&mut self, b: u8) {
        let mut carry = 0;

        if self.registers.get_flag(Flags::Carry) > 0 {
            carry = 1;
        }
        let temp = self.registers.a.wrapping_add(b).wrapping_add(carry);
        self.cpu_zero_check(temp);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (self.registers.a & 0xf) + (b & 0xf) + (carry & 0xf) > 0xf);
        self.registers.flag(Flags::Carry, (self.registers.a as u16) + (b as u16) + (carry as u16) > 0xFF);
        self.registers.a = temp;
    }

    fn alu_sub(&mut self, b: u8) {
        let temp = self.registers.a.wrapping_sub(b);
        self.cpu_zero_check(temp);
        self.registers.set_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (self.registers.a & 0xf) < (b & 0xf));
        self.registers.flag(Flags::Carry, (self.registers.a as u16) < (b as u16));
        self.registers.a = temp;
    }

    fn alu_sbc(&mut self, b: u8) {
        let mut carry = 0;
        if self.registers.get_flag(Flags::Carry) > 0 { carry = 1; }
        let temp = self.registers.a.wrapping_sub(b).wrapping_sub(carry);
        self.cpu_zero_check(temp);
        self.registers.set_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (self.registers.a & 0x0F) < (b & 0x0F) + carry);
        self.registers.flag(Flags::Carry, (self.registers.a as u16) < (b as u16) + (carry as u16));
        self.registers.a = temp;
    }

    fn alu_and(&mut self, register_b: u8) {
        let temp = self.registers.a & register_b;
        self.cpu_zero_check(temp);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.set_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Carry);
        self.registers.a = temp;
    }

    fn alu_xor(&mut self, register_b: u8) {
        let temp = self.registers.a ^ register_b;
        self.cpu_zero_check(temp);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Carry);
        self.registers.a = temp;
    }

    fn alu_or(&mut self, register_b: u8) {
        let temp = self.registers.a | register_b;
        self.cpu_zero_check(temp);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Carry);
        self.registers.a = temp;
    }

    fn alu_cp(&mut self, b: u8) {
        let temp = self.registers.a.wrapping_sub(b);
        self.cpu_zero_check(temp);
        self.registers.set_flag(Flags::Subtract);
        self.registers.flag(Flags::HalfCarry, (self.registers.a & 0xf) < (b & 0xf));
        self.registers.flag(Flags::Carry, (self.registers.a as u16) < (b as u16));
    }

    fn alu_sla(&mut self, mut v: u8) -> u8 {
        let new_carry: u8 = v & 0x80;
        v = (v << 1) & !(1 << 0);
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.flag(Flags::Carry, new_carry > 0);
        v
    }

    fn alu_sra(&mut self, mut v: u8) -> u8 {
        let carry = v & (1 << 0);
        self.registers.change_flag(Flags::Carry, carry);
        v = v >> 1;
        if (v >> 7) > 0 { v  |= 0x80; }
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        v
    }

    fn alu_swap(&mut self, v: u8) -> u8 {
        let res: u8 = (v << 4) | (v >> 4);
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.clear_flag(Flags::Carry);
        res
    }

    fn alu_srl(&mut self, mut v: u8) -> u8 {
        let carry = v & (1 << 0);
        self.registers.change_flag(Flags::Carry, carry);
        v >>= 1;
        self.cpu_zero_check(v);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.clear_flag(Flags::HalfCarry);
        v
    }

    fn alu_bit(&mut self, a: u8, b: u8) {
        let res: bool = a & (1 << (b as u32)) == 0;
        self.registers.flag(Flags::Zero, res);
        self.registers.clear_flag(Flags::Subtract);
        self.registers.set_flag(Flags::HalfCarry);
    }

    fn alu_res(&mut self, mut a: u8, b: u8) -> u8 {
        a &= !(1 << b);
        a
    }

    fn alu_set(&mut self, mut a: u8, b: u8) -> u8 {
        a |= 1 << b;
        a
    }

    fn alu_daa(&mut self) {
        let subtract = self.registers.get_flag(Flags::Subtract);
        let half_carry = self.registers.get_flag(Flags::HalfCarry);
        let carry = self.registers.get_flag(Flags::Carry);
        let mut a = self.registers.a;
        let mut adjustment = 0;

        if carry > 0 {
            adjustment = 0x60;
        }
        if half_carry > 0 {
            adjustment |= 0x06;
        }

        if subtract > 0 {
            a = a.wrapping_sub(adjustment);
        } else {
            if a & 0x0F > 0x09 {
                adjustment |= 0x06;
            }
            if a > 0x99 {
                adjustment |= 0x60;
            }
            a = a.wrapping_add(adjustment);
        }

        self.cpu_zero_check(a);
        self.registers.clear_flag(Flags::HalfCarry);
        self.registers.flag(Flags::Carry, adjustment >= 0x60);
        self.registers.a = a;
    }

    fn cpu_jr_nz_s8(&mut self, s8: i8) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero == 0 {
            self.registers.pc = ((self.registers.pc as i16) + (s8 as i16)) as u16;
            return 3;
        }
        2
    }

    fn cpu_jr_z_s8(&mut self, s8: i8) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero > 1 {
            self.registers.pc = ((self.registers.pc as i16) + (s8 as i16)) as u16;
            return 3;
        }
        2
    }

    fn cpu_c_s8(&mut self, s8: u8) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry > 0 {
            self.registers.pc = (self.registers.pc as i16 + (s8) as i16) as u16;
            return 3;
        }
        2
    }

    fn cpu_ret_nz(&mut self) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero == 0 {
            self.registers.pc = self.stack_pop();
            return 5;
        }
        2
    }

    fn stack_pop(&mut self) -> u16 {
        let result = self.mmu.borrow().rw(self.registers.sp);
        self.registers.sp += 2;
        result
    }

    fn jp_nz_a16(&mut self, a16: u16) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero == 0 {
            self.registers.pc = a16;
            return 4;
        }
        3
    }

    fn jp_a16(&mut self, a16: u16) {
        self.registers.pc = a16;
    }

    fn call_nz_a16(&mut self, a16: u16) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero == 0 {
            self.stack_push(self.registers.pc);
            self.registers.pc = a16;
            return 6;
        }
        3
    }

    pub fn stack_push(&mut self, register: u16) {
        self.registers.sp -= 2;
        self.mmu.borrow_mut().ww(self.registers.sp, register);
    }

    fn ret_z(&mut self) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero > 0 {
            self.registers.pc = self.stack_pop();
            return 5;
        }
        2
    }

    fn ret(&mut self) {
        self.registers.pc = self.stack_pop();
    }

    fn jp_z_a16(&mut self, a16: u16) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero > 0 {
            self.registers.pc = a16;
            return 4;
        }
        return 3;
    }

    fn call_z_a16(&mut self, a16: u16) -> u8 {
        let zero: u8 = self.registers.get_flag(Flags::Zero);
        if zero > 1 {
            self.stack_push(self.registers.pc);
            self.registers.pc = a16;
            return 6;
        }
        3
    }

    fn call_a16(&mut self, a16: u16) {
        self.stack_push(self.registers.pc);
        self.registers.pc = a16;
    }

    fn ret_nc(&mut self) -> u8 {
        let carry = self.registers.get_flag(Flags::Carry);
        if carry == 0 {
            self.registers.pc = self.stack_pop();
            return 5;
        }
        2
    }

    fn jp_nc_a16(&mut self, a16: u16) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry == 0 {
            self.registers.pc = a16;
            return 4;
        }
        3
    }

    fn call_nc_a16(&mut self, a16: u16) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry == 0 {
            self.stack_push(self.registers.pc);
            self.registers.pc = a16;
            return 6;
        }
        3
    }

    fn ret_c(&mut self) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry > 0 {
            self.registers.pc = self.stack_pop();
            return 5;
        }
        2
    }

    fn jp_c_a16(&mut self, a16: u16) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry > 0 {
            self.registers.pc = a16;
            return 4;
        }
        3
    }

    fn call_c_a16(&mut self, a16: u16) -> u8 {
        let carry: u8 = self.registers.get_flag(Flags::Carry);
        if carry > 0 {
            self.stack_push(self.registers.pc);
            self.registers.pc = a16;
            return 6;
        }
        3
    }

    fn rst(&mut self, opcode: u8) -> u8 {
        self.stack_push(self.registers.pc);
        self.registers.pc = match opcode {
            0xC7 => 0x00,
            0xCF => 0x08,
            0xD7 => 0x10,
            0xDF => 0x18,
            0xE7 => 0x20,
            0xEF => 0x28,
            0xF7 => 0x30,
            0xFF => 0x38,
            _ => 0,
        };
        4
    }

    fn cpu_zero_check(&mut self, value: u8) {
        if value == 0 {
            self.registers.set_flag(Flags::Zero);
        } else {
            self.registers.clear_flag(Flags::Zero);
        }
    }
}

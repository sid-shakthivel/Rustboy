// Refactor

pub enum Flags {
    Zero,
    Subtract,
    HalfCarry,
    Carry,
}

pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn af(&self) -> u16 {
        (self.a as u16) << 8 | (self.f as u16)
    }

    pub fn bc(&self) -> u16 {
        (self.b as u16) << 8 | (self.c as u16)
    }

    pub fn de(&self) -> u16 {
        (self.d as u16) << 8 | (self.e as u16)
    }

    pub fn hl(&self) -> u16 {
        (self.h as u16) << 8 | (self.l as u16)
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.f = (value & 0x00F0) as u8;
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = (value & 0x00FF) as u8;
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = (value & 0x00FF) as u8;
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = (value & 0x00FF) as u8;
    }

    pub fn clear_bit(&mut self, flag: Flags) {
        let n = match flag {
            Flags::Zero => 7,
            Flags::Subtract => 6,
            Flags::HalfCarry => 5,
            Flags::Carry => 4,
        };
        self.f &= !(1 << n);
    }

    pub fn set_bit(&mut self, flag: Flags) {
        let n = match flag {
            Flags::Zero => 7,
            Flags::Subtract => 6,
            Flags::HalfCarry => 5,
            Flags::Carry => 4,
        };

        self.f |= 1 << n;
    }

    pub fn change_bit(&mut self, flag: Flags, x: u8) {
        let n = match flag {
            Flags::Zero => 7,
            Flags::Subtract => 6,
            Flags::HalfCarry => 5,
            Flags::Carry => 4,
        };

        self.f = (self.f & (!(1 << n))) | (x << n);
    }

    pub fn flip_bit(&mut self, flag: Flags) {
        match flag {
            Flags::Zero => self.f |= !(self.f & 0x80),
            Flags::Subtract => self.f |= !(self.f & 0x40),
            Flags::HalfCarry => self.f |= !(self.f & 0x20),
            Flags::Carry => self.f |= !(self.f & 0x10),
        };
    }

    pub fn get_bit(&mut self, flag: Flags) -> u8 {
        match flag {
            Flags::Zero => self.f & 0x80,
            Flags::Subtract => self.f & 0x40,
            Flags::HalfCarry => self.f & 0x20,
            Flags::Carry => self.f & 0x10,
        }
    }

    pub fn compose_flags(&mut self) -> String {
        let mut str = String::from("");
        if self.get_bit(Flags::Zero) != 0 {
            str.push('Z');
        } else {
            str.push('-');
        }
        if self.get_bit(Flags::Subtract) != 0 {
            str.push('N');
        } else {
            str.push('-');
        }
        if self.get_bit(Flags::HalfCarry) != 0 {
            str.push('H');
        } else {
            str.push('-');
        }
        if self.get_bit(Flags::Carry) != 0 {
            str.push('C');
        } else {
            str.push('-');
        }
        str
    }
}

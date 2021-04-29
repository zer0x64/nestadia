use super::{Mapper, CartridgeReadTarget, Mirroring};

pub struct Mapper002 {
    prg_bank_selector: u8,
    prg_banks: u8,
    mirroring: Mirroring,
}

impl Mapper002 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            prg_bank_selector: 0,
            prg_banks,
            mirroring,
        }
    }
}

impl Mapper for Mapper002 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x8000..=0xBFFF => CartridgeReadTarget::PrgRom((self.prg_bank_selector as usize) * 0x4000 + (addr & 0x3FFF) as usize),
            _ => CartridgeReadTarget::PrgRom((self.prg_banks as usize - 1) * 0x4000 + (addr & 0x3FFF) as usize),
        }
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        self.prg_bank_selector = data;
    }

    fn ppu_map_read(&self, addr: u16) -> usize {
        addr as usize
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<usize> {
        None
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

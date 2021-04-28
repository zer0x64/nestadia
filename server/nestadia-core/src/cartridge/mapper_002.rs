use super::Mapper;
use super::CartridgeReadTarget;

pub struct Mapper002 {
    prg_bank_selector: u8,
    prg_banks: u8,
}

impl Mapper002 {
    pub fn new(prg_banks: u8) -> Self {
        Self {
            prg_bank_selector: 0,
            prg_banks,
        }
    }
}

impl Mapper for Mapper002 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x8000..=0xBFFF => CartridgeReadTarget::PrgRom((self.prg_bank_selector as u16) * 0x4000 + (addr & 0x3FFF)),
            _ => CartridgeReadTarget::PrgRom((self.prg_banks as u16 - 1) * 0x4000 + (addr & 0x3FFF)),
        }
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        self.prg_bank_selector = data;
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        addr
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

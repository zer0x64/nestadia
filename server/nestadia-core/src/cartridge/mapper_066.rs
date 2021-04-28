use super::Mapper;
use super::CartridgeReadTarget;

pub struct Mapper066 {
    prg_bank_selector: u8,
    chr_bank_selector: u8,
}

impl Mapper066 {
    pub fn new() -> Self {
        Self {
            prg_bank_selector: 0,
            chr_bank_selector: 0,
        }
    }
}

impl Mapper for Mapper066 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        CartridgeReadTarget::PrgRom((self.prg_bank_selector as u16) * 0x8000 + (addr & 0x7FFF))
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        self.chr_bank_selector = data & 0x03;
        self.prg_bank_selector = (data & 0x30) >> 4;
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        (self.chr_bank_selector as u16) * 0x2000 + addr
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

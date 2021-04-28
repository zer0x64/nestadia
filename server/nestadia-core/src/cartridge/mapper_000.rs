use super::Mapper;
use super::CartridgeReadTarget;

pub struct Mapper000 {
    prg_banks: u8,
}

impl Mapper000 {
    pub fn new(prg_banks: u8) -> Self {
        Self { prg_banks }
    }
}

impl Mapper for Mapper000 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        let mask = if self.prg_banks > 1 { 0x7fff } else { 0x3fff };

        CartridgeReadTarget::PrgRom(addr & mask)
    }

    fn cpu_map_write(&mut self, _addr: u16, _data: u8) {}

    fn ppu_map_read(&self, addr: u16) -> u16 {
        addr
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

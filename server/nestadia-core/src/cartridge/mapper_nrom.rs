use super::Mapper;

pub struct NRom {
    prg_banks: u8,
}

impl NRom {
    pub fn new(prg_banks: u8) -> Self {
        Self { prg_banks }
    }
}

impl Mapper for NRom {
    fn cpu_map_read(&self, addr: u16) -> u16 {
        let mask = if self.prg_banks > 1 { 0x7fff } else { 0x3fff };

        addr & mask
    }

    fn cpu_map_write(&self, addr: u16) -> Option<u16> {
        let mask = if self.prg_banks > 1 { 0x7fff } else { 0x3fff };

        None
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        addr
    }

    fn ppu_map_write(&self, addr: u16) -> Option<u16> {
        None
    }
}
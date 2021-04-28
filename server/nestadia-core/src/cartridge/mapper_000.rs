use super::Mapper;

pub struct Mapper000 {
    prg_banks: u8,
}

impl Mapper000 {
    pub fn new(prg_banks: u8) -> Self {
        Self { prg_banks }
    }
}

impl Mapper for Mapper000 {
    fn cpu_map_read(&self, addr: u16) -> u16 {
        let mask = if self.prg_banks > 1 { 0x7fff } else { 0x3fff };
        addr & mask
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        log::warn!(
            "attempted to write {:#X} on PRG memory at {:#X}, but this is not supported by this mapper",
            data, addr
        );
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        addr
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

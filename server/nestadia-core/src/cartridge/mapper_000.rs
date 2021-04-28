use super::{Mapper, CartridgeReadTarget, Mirroring};

pub struct Mapper000 {
    prg_banks: u8,
    mirroring: Mirroring,
}

impl Mapper000 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            prg_banks,
            mirroring,
        }
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

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

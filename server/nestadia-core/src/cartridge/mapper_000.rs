use super::{CartridgeReadTarget, Mapper, Mirroring};

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
        CartridgeReadTarget::PrgRom((addr & mask) as usize)
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        log::warn!(
            "attempted to write {:#X} on PRG memory at {:#X}, but this is not supported by this mapper",
            data, addr
        );
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

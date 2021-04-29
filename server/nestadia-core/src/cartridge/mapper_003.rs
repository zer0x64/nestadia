use super::{CartridgeReadTarget, Mapper, Mirroring};

pub struct Mapper003 {
    chr_bank_selector: u8,
    prg_banks: u8,
    mirroring: Mirroring,
}

impl Mapper003 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            chr_bank_selector: 0,
            prg_banks,
            mirroring,
        }
    }
}

impl Mapper for Mapper003 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        let mask = if self.prg_banks > 1 { 0x7fff } else { 0x3fff };

        CartridgeReadTarget::PrgRom((addr & mask) as usize)
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        self.chr_bank_selector = data;
    }

    fn ppu_map_read(&self, addr: u16) -> usize {
        (self.chr_bank_selector as usize) * 0x2000 + addr as usize
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<usize> {
        None
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

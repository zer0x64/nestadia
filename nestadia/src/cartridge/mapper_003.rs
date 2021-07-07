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
        self.chr_bank_selector = data & 0x03;
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        (self.chr_bank_selector as usize) * 0x2000 + (addr & 0x1fff) as usize
    }

    fn ppu_map_write(&self, addr: u16) -> Option<usize> {
        Some((self.chr_bank_selector as usize) * 0x2000 + (addr & 0x1fff) as usize)
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn get_sram(&self) -> Option<&[u8]> {
        None
    }

    #[cfg(feature = "debugger")]
    fn get_prg_bank(&self, addr: u16) -> Option<u8> {
        match addr {
            0x8000..=0xBFFF => Some(0),
            0xC000..=0xFFFF => Some(1),
            _ => None,
        }
    }
}

use super::{CartridgeReadTarget, Mapper, Mirroring};

pub struct Mapper066 {
    prg_bank_selector: u8,
    chr_bank_selector: u8,
    mirroring: Mirroring,
}

impl Mapper066 {
    pub fn new(mirroring: Mirroring) -> Self {
        Self {
            prg_bank_selector: 0,
            chr_bank_selector: 0,
            mirroring,
        }
    }
}

impl Mapper for Mapper066 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        CartridgeReadTarget::PrgRom(
            (self.prg_bank_selector as usize) * 0x8000 + (addr & 0x7FFF) as usize,
        )
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        self.chr_bank_selector = data & 0x03;
        self.prg_bank_selector = (data & 0x30) >> 4;
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        (self.chr_bank_selector as usize) * 0x2000 + addr as usize
    }

    fn ppu_map_write(&self, addr: u16) -> Option<usize> {
        Some((self.chr_bank_selector as usize) * 0x2000 + addr as usize)
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
            0x8000..=0xFFFF => Some(self.prg_bank_selector),
            _ => None,
        }
    }
}

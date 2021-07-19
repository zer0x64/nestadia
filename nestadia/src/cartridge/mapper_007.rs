use super::{CartridgeReadTarget, Mapper, Mirroring};

pub struct Mapper007 {
    prg_bank_selector: u8,
    mirroring: Mirroring,
}

impl Mapper007 {
    pub fn new() -> Self {
        Self {
            prg_bank_selector: 0,
            mirroring: Mirroring::OneScreenUpper,
        }
    }
}

impl Mapper for Mapper007 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        let addr = (addr & 0x7fff) as usize;
        CartridgeReadTarget::PrgRom(addr + (self.prg_bank_selector as usize) * 0x8000)
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        if addr & 0x8000 == 0x8000 {
            self.prg_bank_selector = data & 0b111;

            self.mirroring = if data & 0x10 == 0x10 {
                Mirroring::OneScreenUpper
            } else {
                Mirroring::OneScreenLower
            };
        };
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        (addr & 0x1fff) as usize
    }

    fn ppu_map_write(&self, addr: u16) -> Option<usize> {
        Some((addr & 0x1fff) as usize)
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

use super::{CartridgeReadTarget, Mapper, Mirroring};

pub struct Mapper002 {
    prg_bank_selector: u8,
    prg_banks: u8,
    mirroring: Mirroring,
}

impl Mapper002 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            prg_bank_selector: 0,
            prg_banks,
            mirroring,
        }
    }
}

impl Mapper for Mapper002 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x8000..=0xBFFF => CartridgeReadTarget::PrgRom(
                (self.prg_bank_selector as usize) * 0x4000 + (addr & 0x3FFF) as usize,
            ),
            _ => CartridgeReadTarget::PrgRom(
                (self.prg_banks as usize - 1) * 0x4000 + (addr & 0x3FFF) as usize,
            ),
        }
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        if addr >= 0x8000 {
            self.prg_bank_selector = data;
        }
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        addr as usize
    }

    fn ppu_map_write(&self, addr: u16) -> Option<usize> {
        Some(addr as usize)
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
            0x8000..=0xBFFF => Some(self.prg_bank_selector),
            0xC000..=0xFFFF => Some(self.prg_banks - 1),
            _ => None,
        }
    }
}

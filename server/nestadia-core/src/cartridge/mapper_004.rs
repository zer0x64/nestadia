use super::{Mapper, Mirroring, CartridgeReadTarget};
use std::intrinsics::{unaligned_volatile_load, unchecked_rem};

pub struct Mapper004 {
    prg_bank_selector: [u8; 4],
    chr_bank_selector: [u8; 8],
    prg_banks: u8,
    mirroring: Mirroring,
    register: [u8; 8],
    ram_data: Vec<u8>,
}

impl Mapper004 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            prg_bank_selector: [0u8, 0u8, 0u8, prg_banks - 1],
            chr_bank_selector: [0u8; 8],
            prg_banks,
            mirroring,
            register: [0u8; 8],
            ram_data: vec![0u8; 0x8000],
        }
    }
}

impl Mapper for Mapper004 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {

        match addr {
            0x6000 ..=0x7FFF => {
                // Read from RAM
                CartridgeReadTarget::PrgRam(self.ram_data[(addr & 0x1FFF) as usize])
            },
            0x8000 ..=0x9FFF => {
                CartridgeReadTarget::PrgRom((self.prg_bank_selector[0] as usize) * 0x2000 + (addr & 0x1FFF) as usize)
            },
            0xA000 ..=0xBFFF => {
                CartridgeReadTarget::PrgRom((self.prg_bank_selector[1] as usize) * 0x2000 + (addr & 0x1FFF) as usize)
            },
            0xC000 ..=0xDFFF => {
                CartridgeReadTarget::PrgRom((self.prg_bank_selector[2] as usize) * 0x2000 + (addr & 0x1FFF) as usize)
            },
            0xE000 ..=0xFFFF => {
                CartridgeReadTarget::PrgRom((self.prg_bank_selector[3] as usize) * 0x2000 + (addr & 0x1FFF) as usize)
            }
            _ => unreachable!(),
        }
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        if (0x6000 ..=0x7FFF).contains(&addr) {
            // Write to RAM
            self.ram_data[(addr & 0x1FFF) as usize] = data;
            return;
        }
    }

    fn ppu_map_read(&self, addr: u16) -> usize {
        match addr {
            0x0000 ..=0x03FF => {
                (self.chr_bank_selector[0] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            0x0400 ..=0x7FF => {
                (self.chr_bank_selector[1] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x0800 ..=0x0BFF => {
                (self.chr_bank_selector[2] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            0x0C00 ..=0xFFF => {
                (self.chr_bank_selector[3] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x1000 ..=0x13FF => {
                (self.chr_bank_selector[4] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            0x1400 ..=0x17FF => {
                (self.chr_bank_selector[5] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            0x1800 ..=0x1BFF => {
                (self.chr_bank_selector[6] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            0x1C00 ..=0x1FFF => {
                (self.chr_bank_selector[7] as usize) * 0x0400 + (addr & 0x03FF) as usize
            },
            _ => unreachable!(),
        }
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<usize> {
        None
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }
}

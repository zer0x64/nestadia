use super::Mapper;
use super::CartridgeReadTarget;

pub struct Mapper001 {
    prg_banks: u8,
    prg_bank_selector_32: u8,
    prg_bank_selector_16_lo: u8,
    prg_bank_selector_16_hi: u8,
    chr_bank_selector_8: u8,
    chr_bank_selector_4_lo: u8,
    chr_bank_selector_4_hi: u8,
    load_register: u8,
    load_register_count: u8,
    control_register: u8,
    ram_data: Vec<u8>,
}

impl Mapper001 {
    pub fn new(prg_banks: u8) -> Self {
        Self {
            prg_banks,
            prg_bank_selector_32: 0,
            prg_bank_selector_16_lo: 0,
            prg_bank_selector_16_hi: 0,
            chr_bank_selector_8: 0,
            chr_bank_selector_4_lo: 0,
            chr_bank_selector_4_hi: 0,
            load_register: 0x10000,
            load_register_count: 0,
            control_register: 0,
            ram_data: vec![0u8; 0x8000],
        }
    }
}

impl Mapper for Mapper001 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x6000 ..=0x7FFF => {
                // Read from RAM
                CartridgeReadTarget::PrgRam(self.ram_data[addr & 0x1FFF]) // TODO: windowed RAM?
            },
            _ => {
                if (self.control_register & 0b01000) == 0b01000 {
                    // 16K PRG mode
                    match addr {
                        0x8000 ..=0xBFFF => CartridgeReadTarget::PrgRom((self.prg_bank_selector_16_lo as u16) * 0x4000 + (addr & 0x3FFF)),
                        _ => CartridgeReadTarget::PrgRom((self.prg_bank_selector_16_hi as u16) * 0x4000 + (addr & 0x3FFF)),
                    }
                } else {
                    // 32K PRG mode
                    CartridgeReadTarget::PrgRom((self.prg_bank_selector_32 as u16) * 0x8000 + (addr & 0x7FFF))
                }
            },
        }
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        // TODO
        // Registers:
        //      0x8000: control, set some status bits of the mapper and mirroring
        //      0xA000: CHR LO, info to map CHR ROM
        //      0xC000: CHR HI, info to map CHR ROM
        //      0xE000: PRG ROM,
        // if bit 8 is 1 (0x80) -> reset
        // 5 writes will set the "load" register
        //      When 5th bit is set, use bit 13 and 14 of the addr to target 1 of 4 functional register
        //      Then set the register to the 5 bits that we have
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        let mut mapped = if (0 ..=0x0FFF as u16).contains(&addr) {0} else {0x1000};
        mapped + (self.chr_bank_selector as u16) * 0x1000 + (addr & 0x0FFF)
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

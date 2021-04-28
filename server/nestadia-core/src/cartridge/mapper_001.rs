use super::Mapper;
use super::CartridgeReadTarget;

const CHR_MODE_MASK: u8 = 0b10000;
const PRG_MODE_MASK: u8 = 0b01100;

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
            load_register: 0,
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
                CartridgeReadTarget::PrgRam(self.ram_data[(addr & 0x1FFF) as usize]) // TODO: windowed RAM?
            },
            _ => {
                if (self.control_register & PRG_MODE_MASK) > 1 {
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

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        // Registers:
        //      0x8000: control, set some status bits of the mapper and mirroring
        //      0xA000: CHR LO, info to map CHR ROM
        //      0xC000: CHR HI, info to map CHR ROM
        //      0xE000: PRG ROM,
        // if bit 8 is 1 (0x80) -> reset
        // 5 writes will set the "load" register
        //      When 5th bit is set, use bit 13 and 14 of the addr to target 1 of 4 functional register
        //      Then set the register to the 5 bits that we have

        if (0x6000 ..=0x7FFF).contains(&addr) {
            self.ram_data[(addr & 0x1FFF) as usize] = data;    // TODO: windowed RAM?
        }

        if (data & 0x80) == 0x80 {
            // Reset load register
            self.load_register = 0;
            self.load_register_count = 0;
            self.control_register |= 0x0C;
            return
        }

        // Add new bit to load register
        self.load_register >>= 1;
        self.load_register |= ((data & 0x01) << 4);
        self.load_register_count += 1;

        // Check if load register is full
        if self.load_register_count == 5 {
            // Check target of write using bit 14 and 13 from the address
            match (addr & 0x6000) {
                0x0000 => { // Control register
                    self.control_register = self.load_register & 0x1F;
                },
                0x2000 => { // CHR bank 0
                    if (self.control_register & CHR_MODE_MASK) != 0 {
                        self.chr_bank_selector_4_lo = self.load_register & 0x1F;
                    } else {
                        self.chr_bank_selector_8 = self.load_register & 0x1E;
                    }
                },
                0x4000 => { // CHR bank 1
                    self.chr_bank_selector_4_hi = self.load_register & 0x1F;
                },
                _ => { // PRG bank
                    match (self.control_register & PRG_MODE_MASK) >> 2 {
                        2 => { // 16K mode, fix low bank
                            self.prg_bank_selector_16_lo = 0;
                            self.prg_bank_selector_16_hi = self.load_register & 0x0F;
                        },
                        3 => { // 16K mode, fix high bank
                            self.prg_bank_selector_16_lo = self.load_register & 0x0F;
                            self.prg_bank_selector_16_hi = self.prg_banks - 1;
                        },
                        _ => { // 32K mode
                            self.prg_bank_selector_32 = (self.load_register & 0x0E) >> 1;
                        },
                    }
                },
            }

            // Reset load register
            self.load_register = 0x00;
            self.load_register_count = 0;
        }
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        if (self.control_register & CHR_MODE_MASK) != 0 {
            // 4K CHR mode
            match addr {
                0x0000 ..=0x0FFF => (self.chr_bank_selector_4_lo as u16) * 0x1000 + (addr & 0x0FFF),
                _ => (self.chr_bank_selector_4_hi as u16) * 0x1000 + (addr & 0x0FFF),
            }
        } else {
            // 8K CHR mode
            (self.chr_bank_selector_8 as u16) * 0x2000 + (addr & 0x1FFF)
        }
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

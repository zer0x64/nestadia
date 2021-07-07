use super::{CartridgeReadTarget, Mapper, Mirroring};

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
    ram_data: [u8; 0x2000],
    mirroring: Mirroring,
}

impl Mapper001 {
    pub fn new(prg_banks: u8, mirroring: Mirroring, save_data: Option<&[u8]>) -> Self {
        let mut ram_data = [0u8; 0x2000];

        // Load the save data
        if let Some(save_data) = save_data {
            ram_data
                .iter_mut()
                .zip(save_data.iter())
                .for_each(|(r, s)| *r = *s)
        };

        Self {
            prg_banks,
            prg_bank_selector_32: 0,
            prg_bank_selector_16_lo: 0,
            prg_bank_selector_16_hi: prg_banks - 1,
            chr_bank_selector_8: 0,
            chr_bank_selector_4_lo: 0,
            chr_bank_selector_4_hi: 0,
            load_register: 0,
            load_register_count: 0,
            control_register: 0x0C,
            ram_data,
            mirroring,
        }
    }
}

impl Mapper for Mapper001 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x6000..=0x7FFF => {
                // Read from RAM
                CartridgeReadTarget::PrgRam(self.ram_data[(addr & 0x1FFF) as usize])
                // TODO: windowed RAM?
            }
            _ => {
                if (self.control_register & PRG_MODE_MASK) > 1 {
                    // 16K PRG mode
                    match addr {
                        0x8000..=0xBFFF => CartridgeReadTarget::PrgRom(
                            (self.prg_bank_selector_16_lo as usize) * 0x4000
                                + (addr & 0x3FFF) as usize,
                        ),
                        0xC000..=0xFFFF => CartridgeReadTarget::PrgRom(
                            (self.prg_bank_selector_16_hi as usize) * 0x4000
                                + (addr & 0x3FFF) as usize,
                        ),
                        _ => {
                            log::warn!("Attempted to read address w/o known mapping {:#06x}", addr);
                            CartridgeReadTarget::PrgRom(0)
                        }
                    }
                } else {
                    // 32K PRG mode
                    CartridgeReadTarget::PrgRom(
                        (self.prg_bank_selector_32 as usize) * 0x8000 + (addr & 0x7FFF) as usize,
                    )
                }
            }
        }
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        if (0x6000..=0x7FFF).contains(&addr) {
            // Write to RAM
            self.ram_data[(addr & 0x1FFF) as usize] = data; // TODO: windowed RAM?
            return;
        }

        if (data & 0x80) == 0x80 {
            // Reset load register
            self.load_register = 0;
            self.load_register_count = 0;
            self.control_register |= 0x0C;
            return;
        }

        // Add new bit to load register
        self.load_register >>= 1;
        self.load_register |= (data & 0x01) << 4;
        self.load_register_count += 1;

        // Check if load register is full
        if self.load_register_count == 5 {
            // Check target of write using bit 14 and 13 from the address
            match addr & 0x6000 {
                0x0000 => {
                    // Control register
                    self.control_register = self.load_register & 0x1F;
                    match self.control_register & 0x03 {
                        0 => self.mirroring = Mirroring::OneScreenLower,
                        1 => self.mirroring = Mirroring::OneScreenUpper,
                        2 => self.mirroring = Mirroring::Vertical,
                        _ => self.mirroring = Mirroring::Horizontal,
                    }
                }
                0x2000 => {
                    // CHR bank 0
                    if (self.control_register & CHR_MODE_MASK) != 0 {
                        self.chr_bank_selector_4_lo = self.load_register & 0x1F;
                    } else {
                        self.chr_bank_selector_8 = self.load_register & 0x1E;
                    }
                }
                0x4000 => {
                    // CHR bank 1
                    self.chr_bank_selector_4_hi = self.load_register & 0x1F;
                }
                0x6000 => {
                    // PRG bank
                    match (self.control_register & PRG_MODE_MASK) >> 2 {
                        2 => {
                            // 16K mode, fix low bank
                            self.prg_bank_selector_16_lo = 0;
                            self.prg_bank_selector_16_hi = self.load_register & 0x0F;
                        }
                        3 => {
                            // 16K mode, fix high bank
                            self.prg_bank_selector_16_lo = self.load_register & 0x0F;
                            self.prg_bank_selector_16_hi = self.prg_banks - 1;
                        }
                        _ => {
                            // 32K mode
                            self.prg_bank_selector_32 = (self.load_register & 0x0E) >> 1;
                        }
                    }
                }
                _ => unreachable!(),
            }

            // Reset load register
            self.load_register = 0x00;
            self.load_register_count = 0;
        }
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        if (self.control_register & CHR_MODE_MASK) != 0 {
            // 4K CHR mode
            match addr {
                0x0000..=0x0FFF => {
                    (self.chr_bank_selector_4_lo as usize) * 0x1000 + (addr & 0x0FFF) as usize
                }
                _ => (self.chr_bank_selector_4_hi as usize) * 0x1000 + (addr & 0x0FFF) as usize,
            }
        } else {
            // 8K CHR mode
            (self.chr_bank_selector_8 as usize) * 0x2000 + (addr & 0x1FFF) as usize
        }
    }

    fn ppu_map_write(&self, addr: u16) -> Option<usize> {
        Some((self.chr_bank_selector_8 as usize) * 0x2000 + (addr & 0x1FFF) as usize)
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn get_sram(&self) -> Option<&[u8]> {
        Some(&self.ram_data)
    }

    #[cfg(feature = "debugger")]
    fn get_prg_bank(&self, addr: u16) -> Option<u8> {
        match addr {
            0x0000..=0x7FFF => None,
            _ => {
                if (self.control_register & PRG_MODE_MASK) > 1 {
                    // 16K PRG mode
                    match addr {
                        0x8000..=0xBFFF => Some(self.prg_bank_selector_16_lo),
                        0xC000..=0xFFFF => Some(self.prg_bank_selector_16_hi),
                        _ => None,
                    }
                } else {
                    // 32K PRG mode
                    Some(self.prg_bank_selector_32)
                }
            }
        }
    }
}

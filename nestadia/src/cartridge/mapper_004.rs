use alloc::vec;
use alloc::vec::Vec;

use super::{CartridgeReadTarget, Mapper, Mirroring};

pub struct Mapper004 {
    prg_banks: u8,
    prg_bank_selector: [u8; 4],
    chr_bank_selector: [u8; 8],
    mirroring: Mirroring,
    prg_mode: bool,
    chr_inverson: bool,
    register: [u8; 8],
    target_register: u8,
    ram_data: Vec<u8>,

    last_chr_bank_bit: bool, // Used to detect changed between sprites and background rendering for scanline counter

    irq_enabled: bool,
    irq_active: bool,
    irq_reload: bool,
    irq_counter: u8,
    irq_latch: u8,
}

impl Mapper004 {
    pub fn new(prg_banks: u8, mirroring: Mirroring) -> Self {
        Self {
            prg_banks,
            prg_bank_selector: [0u8, 0u8, 0u8, prg_banks * 2 - 1],
            chr_bank_selector: [0u8; 8],
            mirroring,
            prg_mode: false,
            chr_inverson: false,
            register: [0u8; 8],
            target_register: 0,
            ram_data: vec![0u8; 0x2000],

            last_chr_bank_bit: false,

            irq_active: false,
            irq_enabled: false,
            irq_reload: false,
            irq_counter: 0,
            irq_latch: 0,
        }
    }
}

impl Mapper for Mapper004 {
    fn cpu_map_read(&self, addr: u16) -> CartridgeReadTarget {
        match addr {
            0x6000..=0x7FFF => {
                // Read from RAM
                CartridgeReadTarget::PrgRam(self.ram_data[(addr & 0x1FFF) as usize])
            }
            0x8000..=0x9FFF => CartridgeReadTarget::PrgRom(
                (self.prg_bank_selector[0] as usize) * 0x2000 + (addr & 0x1FFF) as usize,
            ),
            0xA000..=0xBFFF => CartridgeReadTarget::PrgRom(
                (self.prg_bank_selector[1] as usize) * 0x2000 + (addr & 0x1FFF) as usize,
            ),
            0xC000..=0xDFFF => CartridgeReadTarget::PrgRom(
                (self.prg_bank_selector[2] as usize) * 0x2000 + (addr & 0x1FFF) as usize,
            ),
            0xE000..=0xFFFF => CartridgeReadTarget::PrgRom(
                (self.prg_bank_selector[3] as usize) * 0x2000 + (addr & 0x1FFF) as usize,
            ),
            _ => {
                log::warn!("Attempted to read address w/o known mapping {:#06x}", addr);
                CartridgeReadTarget::PrgRom(0)
            }
        }
    }

    fn cpu_map_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x6000..=0x7FFF => {
                // Write to RAM
                self.ram_data[(addr & 0x1FFF) as usize] = data;
            }
            0x8000..=0x9FFF => {
                if (addr & 0x01) == 0 {
                    // Bank select
                    self.target_register = data & 0x07;
                    self.prg_mode = (data & 0x40) == 0x40;
                    self.chr_inverson = (data & 0x80) == 0x80;
                } else {
                    // Bank data
                    self.register[self.target_register as usize] = data;

                    // Update bank selectors
                    if self.prg_mode {
                        self.prg_bank_selector[0] = self.prg_banks * 2 - 2;
                        self.prg_bank_selector[2] = self.register[6] & 0x3F;
                    } else {
                        self.prg_bank_selector[0] = self.register[6] & 0x3F;
                        self.prg_bank_selector[2] = self.prg_banks * 2 - 2;
                    }
                    self.prg_bank_selector[1] = self.register[7] & 0x3F;
                    self.prg_bank_selector[3] = self.prg_banks * 2 - 1;

                    if self.chr_inverson {
                        self.chr_bank_selector[0] = self.register[2];
                        self.chr_bank_selector[1] = self.register[3];
                        self.chr_bank_selector[2] = self.register[4];
                        self.chr_bank_selector[3] = self.register[5];
                        self.chr_bank_selector[4] = self.register[0] & 0xFE;
                        self.chr_bank_selector[5] = self.register[0] + 1;
                        self.chr_bank_selector[6] = self.register[1] & 0xFE;
                        self.chr_bank_selector[7] = self.register[1] + 1;
                    } else {
                        self.chr_bank_selector[0] = self.register[0] & 0xFE;
                        self.chr_bank_selector[1] = self.register[0] + 1;
                        self.chr_bank_selector[2] = self.register[1] & 0xFE;
                        self.chr_bank_selector[3] = self.register[1] + 1;
                        self.chr_bank_selector[4] = self.register[2];
                        self.chr_bank_selector[5] = self.register[3];
                        self.chr_bank_selector[6] = self.register[4];
                        self.chr_bank_selector[7] = self.register[5];
                    }
                }
            }
            0xA000..=0xBFFF => {
                if (addr & 0x01) == 0 {
                    // Mirroring
                    match self.mirroring {
                        Mirroring::FourScreen => {}
                        _ => {
                            self.mirroring = match data & 0x01 {
                                0 => Mirroring::Vertical,
                                1 => Mirroring::Horizontal,
                                _ => unreachable!(),
                            }
                        }
                    }
                } else {
                    // PRG RAM protect
                    // Not needed
                }
            }
            0xC000..=0xDFFF => {
                if (addr & 0x01) == 0 {
                    // IRQ latch
                    self.irq_latch = data;
                } else {
                    // IRQ reload
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                if (addr & 0x01) == 0 {
                    // IRQ disable
                    self.irq_enabled = false;
                    self.irq_active = false;
                } else {
                    // IRQ enable
                    self.irq_enabled = true;
                }
            }
            _ => log::warn!(
                "Attempted to write to address w/o known mapping: {:#06x}",
                addr
            ),
        }
    }

    fn ppu_map_read(&mut self, addr: u16) -> usize {
        let chr_bank_bit = addr & 0x1000 == 0x1000;
        if !self.last_chr_bank_bit && chr_bank_bit {
            // Rising edge of scanline counter
            if self.irq_counter == 0 || self.irq_reload {
                self.irq_counter = self.irq_latch;
                self.irq_reload = false;
            } else {
                self.irq_counter -= 1;
            };

            if self.irq_counter == 0 && self.irq_enabled {
                self.irq_active = true;
            };
        }

        self.last_chr_bank_bit = chr_bank_bit;

        match addr {
            0x0000..=0x03FF => {
                (self.chr_bank_selector[0] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x0400..=0x7FF => {
                (self.chr_bank_selector[1] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x0800..=0x0BFF => {
                (self.chr_bank_selector[2] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x0C00..=0xFFF => {
                (self.chr_bank_selector[3] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x1000..=0x13FF => {
                (self.chr_bank_selector[4] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x1400..=0x17FF => {
                (self.chr_bank_selector[5] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x1800..=0x1BFF => {
                (self.chr_bank_selector[6] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            0x1C00..=0x1FFF => {
                (self.chr_bank_selector[7] as usize) * 0x0400 + (addr & 0x03FF) as usize
            }
            _ => {
                log::warn!(
                    "Attempted to read CHR address w/o known mapping: {:#06x}",
                    addr
                );
                0_usize
            }
        }
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<usize> {
        None
    }

    fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    fn irq_state(&self) -> bool {
        self.irq_active
    }

    fn irq_clear(&mut self) {
        self.irq_active = false;
    }

    fn get_sram(&self) -> Option<&[u8]> {
        Some(&self.ram_data)
    }

    #[cfg(feature = "debugger")]
    fn get_prg_bank(&self, addr: u16) -> Option<u8> {
        match addr {
            0x0000..=0x7FFF => None,
            // The bank selector select 8KB banks, so divide by 2 to get 16KB bank
            0x8000..=0x9FFF => Some(self.prg_bank_selector[0] / 2),
            0xA000..=0xBFFF => Some(self.prg_bank_selector[1] / 2),
            0xC000..=0xDFFF => Some(self.prg_bank_selector[2] / 2),
            0xE000..=0xFFFF => Some(self.prg_bank_selector[3] / 2),
        }
    }
}

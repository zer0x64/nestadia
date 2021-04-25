use crate::bus::PpuBus;

/// Registers definitions
pub mod registers;

pub type PpuFrame = [u8; 256 * 240];

pub struct Ppu {
    // internal memory
    palette_table: [u8; 32], // For color stuff
    #[allow(dead_code)] // FIXME
    oam_data: [u8; 64 * 4], // Object Attribute Memory, internal to PPU

    // registers
    addr_reg: registers::VramAddr, // Address register pointing to name tables
    ctrl_reg: registers::ControllerReg,

    // emulation-specific internal stuff
    cycle_count: u16,
    scanline: i16,
    frame: PpuFrame,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            palette_table: [0u8; 32],
            oam_data: [0u8; 64 * 4],

            addr_reg: registers::VramAddr::new(),
            ctrl_reg: registers::ControllerReg::default(),

            cycle_count: 0,
            scanline: 0,
            frame: [0u8; 256 * 240],
        }
    }

    pub fn write_ppu_register(&mut self, bus: &mut PpuBus<'_>, addr: u16, data: u8) {
        let addr = addr & 0x07; // mirror

        match addr {
            0 => {
                // Write Control register
                self.ctrl_reg = registers::ControllerReg::from_bits_truncate(data);
            }
            1 => {
                // TODO: Mask
            }
            2 => {
                // Status - not writable
            }
            3 => {
                // TODO: OAM Address
            }
            4 => {
                // TODO: OAM Data
            }
            5 => {
                // TODO: Scroll
            }
            6 => {
                // write PPU Address
                self.addr_reg.load(data);
            }
            7 => {
                // Write PPU Data

                // Address to write data to
                #[allow(unused_variables)] // FIXME
                let write_addr = self.addr_reg.get();

                // All PPU data writes increment the nametable addr
                self.increment_vram_addr();

                // TODO: write to the right place
            }
            _ => {
                unreachable!("unexpected write to mirrored space {}", addr);
            }
        }
    }

    pub fn read_ppu_register(&mut self, bus: &mut PpuBus<'_>, addr: u16) -> u8 {
        let addr = addr & 0x07; // mirror

        match addr {
            0 => {
                // Control - not readable
                0
            }
            1 => {
                // Mask - not readable
                0
            }
            2 => {
                // TODO: read Status
                0
            }
            3 => {
                // OAM Address - not readable
                0
            }
            4 => {
                // TODO: read OAM Data
                0
            }
            5 => {
                // Scroll - not readable
                0
            }
            6 => {
                // PPU Address - not readable
                0
            }
            7 => {
                // Read PPU Data

                // Address to read data from
                let read_addr = self.addr_reg.get();

                // All PPU data reads increment the nametable addr
                self.increment_vram_addr();

                match read_addr {
                    // addresses mapped to PPU bus
                    0..=0x1FFF => bus.read_chr_mem(read_addr),
                    0x2000..=0x2FFF => bus.read_name_tables(read_addr),

                    // Unused addr space
                    0x3000..=0x3EFF => {
                        log::warn!("addr space 0x3000..0x3EFF is not expected to be used, but 0x{:#X} was requested", read_addr);
                        0
                    }

                    // Palette table is not behind bus, it can be directly returned
                    0x3F00..=0x3FFF => self.palette_table[usize::from(read_addr - 0x3F00)],

                    _ => unreachable!("unexpected access to mirrored space {}", read_addr),
                }
            }
            _ => {
                unreachable!("unexpected access to mirrored space {}", addr);
            }
        }
    }

    /// Returns frame when it's ready
    pub fn clock(&mut self, bus: &mut PpuBus<'_>) -> Option<&PpuFrame> {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            self.cycle_count = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                // Yeah! We got a frame ready
                self.scanline = -1;
                return Some(&self.frame);
            }
        }

        // <-- TODO: write to frame here :)

        // Frame is not ready yet
        None
    }

    fn increment_vram_addr(&mut self) {
        let inc_step = self.ctrl_reg.vram_addr_increment();
        self.addr_reg.inc(inc_step);
    }
}


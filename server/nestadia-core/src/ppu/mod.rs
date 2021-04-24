use crate::EmulatorContext;

pub mod registers;

pub type PpuFrame = [u8; 256 * 240];

pub struct Ppu {
    name_tables: [u8; 1024 * 2], // vram
    palette_table: [u8; 32],     // for color stuff
    oam_data: [u8; 64 * 4],      // object attribute memory, internal to PPU

    addr_reg: registers::VramAddr, // address register pointing to name tables

    cycle_count: u16,
    scanline: i16,

    frame: PpuFrame,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            name_tables: [0u8; 1024 * 2],
            palette_table: [0u8; 32],
            oam_data: [0u8; 64 * 4],
            addr_reg: registers::VramAddr::new(),
            cycle_count: 0,
            scanline: 0,
            frame: [0u8; 256 * 240],
        }
    }
}

impl dyn EmulatorContext<Ppu> {
    pub fn write_ppu_controls(&mut self, address: u16, data: u8) {
        let address = address & 0x07; // mirror

        // TODO

        match address {
            0 => { /*Control*/ }
            1 => { /*Mask*/ }
            2 => { /*Status*/ }
            3 => { /*OAM Address*/ }
            4 => { /*OAM Data*/ }
            5 => { /*Scroll*/ }
            6 => {
                // write PPU Address
                self.addr_reg.load(data);
            }
            7 => {
                // read PPU Data
                let read_addr = self.addr_reg.get();

                // TODO
            }
            _ => {
                // should never happen because of mask
                unreachable!();
            }
        }
    }

    pub fn read_ppu_controls(&mut self, address: u16, _read_only: bool) -> u8 {
        let address = address & 0x07; // mirror

        // TODO

        match address {
            0 => {
                // Control
                0
            }
            1 => {
                // Mask
                0
            }
            2 => {
                // Status
                0
            }
            3 => {
                // OAM Address
                0
            }
            4 => {
                // OAM Data
                0
            }
            5 => {
                // Scroll
                0
            }
            6 => {
                // PPU Address
                0
            }
            7 => {
                // PPU Data
                0
            }
            _ => {
                // Should never happen because of mask
                unreachable!();
            }
        }
    }

    /// Returns frame when it's ready
    pub fn clock(&mut self) -> Option<&PpuFrame> {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            self.cycle_count = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.scanline = -1;

                // TODO: write to frame

                Some(&self.frame)
            } else {
                None
            }
        } else {
            None
        }
    }
}

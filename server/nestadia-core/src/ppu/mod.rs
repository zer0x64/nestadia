use crate::EmulatorContext;

pub type PpuFrame = [u8; 256 * 240];

pub struct Ppu {
    frame: PpuFrame,
    name_tables: [[u8; 1024]; 2],
    color_pallettes: [u8; 32],
    cycle_count: u16,
    scanline: i16,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            name_tables: [[0u8; 1024]; 2],
            color_pallettes: [0u8; 32],
            cycle_count: 0,
            scanline: 0,
            frame: [0u8; 256 * 240],
        }
    }
}

impl dyn EmulatorContext<Ppu> {
    pub fn write_ppu_controls(&mut self, address: u16, data: u8) {
        let address = address & 0x07;

        // TODO

        match address {
            0 => { /*Control*/ }
            1 => { /*Mask*/ }
            2 => { /*Status*/ }
            3 => { /*OAM Address*/ }
            4 => { /*OAM Data*/ }
            5 => { /*Scroll*/ }
            6 => { /*PPU Address*/ }
            7 => { /* PPU Data*/ }
            _ => {
                // Should never happen before of mask
                unreachable!();
            }
        }
    }

    pub fn read_ppu_controls(&mut self, address: u16, _read_only: bool) -> u8 {
        // TODO

        match address {
            0 => {
                0 /*Control*/
            }
            1 => {
                0 /*Mask*/
            }
            2 => {
                0 /*Status*/
            }
            3 => {
                0 /*OAM Address*/
            }
            4 => {
                0 /*OAM Data*/
            }
            5 => {
                0 /*Scroll*/
            }
            6 => {
                0 /*PPU Address*/
            }
            7 => {
                0 /* PPU Data*/
            }
            _ => {
                // Should never happen because of mask
                unreachable!();
            }
        }
    }

    /// Returns frame when it's ready
    pub fn clock(&mut self) -> Option<PpuFrame> {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            self.cycle_count = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.scanline = -1;

                // TODO: Use actual frame data instead

                Some(self.frame)
            } else {
                None
            }
        } else {
            None
        }
    }
}

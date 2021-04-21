use crate::EmulatorContext;

pub struct Ppu {
    frame: Option<[u8; 256 * 240]>,
    name_tables: [[u8; 1024]; 2],
    color_pallettes: [u8; 32],
    cycle_count: u16,
    scanline: i32,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            name_tables: [[0u8; 1024]; 2],
            color_pallettes: [0u8; 32],
            cycle_count: 0,
            scanline: 0,
            frame: None
        }
    }

    pub fn get_frame(&mut self) -> Option<[u8; 256 * 240]> {
        let f = self.frame;
        self.frame = None;
        f
    }
}

impl dyn EmulatorContext<Ppu> {
    pub fn write_ppu_controls(&mut self, address: u16, data: u8) {
        // TODO

        match address {
            0 => {/*Control*/},
            1 => {/*Mask*/},
            2 => {/*Status*/},
            3 => {/*OAM Address*/},
            4 => {/*OAM Data*/},
            5 => {/*Scroll*/},
            6 => {/*PPU Address*/},
            7 => {/* PPU Data*/},
            _ => {
                // Should never happen before of mask
                unreachable!();
            }
        }
    }

    pub fn read_ppu_controls(&mut self, address: u16, _read_only: bool) -> u8 {
        // TODO
        
        match address {
            0 => { 0 /*Control*/},
            1 => { 0 /*Mask*/},
            2 => { 0 /*Status*/},
            3 => { 0 /*OAM Address*/},
            4 => { 0 /*OAM Data*/},
            5 => { 0 /*Scroll*/},
            6 => { 0 /*PPU Address*/},
            7 => { 0 /* PPU Data*/},
            _ => {
                // Should never happen before of mask
                unreachable!();
            }
        }
    }

    pub fn clock(&mut self) {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            self.cycle_count = 0;
            self.scanline += 1;

            if self.scanline >= 261 {
                self.scanline = -1;

                // TODO: Use actual frame data instead
                self.frame = Some([0x14u8; 256 * 240]);
            }
        }
    }
}
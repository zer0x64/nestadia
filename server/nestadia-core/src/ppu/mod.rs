use crate::EmulatorContext;

/// Registers definitions
pub mod registers;

pub type PpuFrame = [u8; 256 * 240];

pub struct Ppu {
    // memory
    name_tables: [u8; 1024 * 2], // VRAM
    palette_table: [u8; 32],     // For color stuff
    oam_data: [u8; 64 * 4],      // Object Attribute Memory, internal to PPU

    // registers
    addr_reg: registers::VramAddr, // Address register pointing to name tables
    ctrl_reg: registers::ControllerReg,

    // emulation-specific internal stuff
    last_data_on_bus: u8,
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
            ctrl_reg: registers::ControllerReg::default(),

            last_data_on_bus: 0,
            cycle_count: 0,
            scanline: 0,
            frame: [0u8; 256 * 240],
        }
    }
}

impl dyn EmulatorContext<Ppu> {
    pub fn ppu_write_register(&mut self, address: u16, data: u8) {
        let address = address & 0x07; // mirror

        match address {
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
                let write_addr = self.addr_reg.get();

                // All PPU data writes increment the nametable address
                let inc_step = self.ctrl_reg.vram_addr_increment();
                self.addr_reg.inc(inc_step);

                // TODO: write to the right place
            }
            _ => {
                unreachable!("unexpected write to mirrored space {}", address);
            }
        }
    }

    pub fn ppu_read_register(&mut self, address: u16, _read_only: bool) -> u8 {
        let address = address & 0x07; // mirror

        match address {
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

                // All PPU data reads increment the nametable address
                let inc_step = self.ctrl_reg.vram_addr_increment();
                self.addr_reg.inc(inc_step);

                // PPU has to fetch the data and keep it in internal buffer.
                // Read returns the data fetched from the previous load operation.
                let read_data = self.last_data_on_bus;

                self.last_data_on_bus = match read_addr {
                    0..=0x1FFF => todo!("read from chr_rom"),
                    0x2000..=0x2FFF => todo!("name tables mirroring for {}", read_addr),
                    0x3000..=0x3EFF => {
                        // FIXME: use log framework instead of println!
                        println!("address space 0x3000..0x3EFF is not expected to be used, requested = {} ", read_addr);
                        0
                    }
                    0x3F00..=0x3FFF => self.palette_table[usize::from(read_addr - 0x3F00)],
                    _ => unreachable!("unexpected access to mirrored space {}", read_addr),
                };

                read_data
            }
            _ => {
                unreachable!("unexpected access to mirrored space {}", address);
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
                // Yeah! We got a frame ready
                self.scanline = -1;
                return Some(&self.frame);
            }
        }

        // <-- TODO: write to frame here :)

        // Frame is not ready yet
        None
    }
}

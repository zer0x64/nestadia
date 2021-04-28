use crate::bus::PpuBus;

/// Registers definitions
pub mod registers;

pub type PpuFrame = [u8; 256 * 240];

pub struct Ppu {
    // internal memory
    palette_table: [u8; 32], // For color stuff
    oam_data: [u8; 64 * 4],  // Object Attribute Memory, internal to PPU

    // registers
    ctrl_reg: registers::ControlReg,
    mask_reg: registers::MaskReg,
    status_reg: registers::StatusReg,
    oam_addr_reg: u8,
    scroll_reg: registers::ScrollReg,
    addr_reg: registers::VramAddr, // Address register pointing to name tables

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

            ctrl_reg: registers::ControlReg::default(),
            mask_reg: registers::MaskReg::default(),
            status_reg: registers::StatusReg::default(),
            oam_addr_reg: 0,
            scroll_reg: registers::ScrollReg::default(),
            addr_reg: registers::VramAddr::default(),

            cycle_count: 0,
            scanline: 0,
            frame: [0u8; 256 * 240],
        }
    }

    pub fn write(&mut self, bus: &mut PpuBus<'_>, addr: u16, data: u8) {
        let addr = addr & 0x07; // mirror

        match addr {
            0 => {
                // Write Control register
                self.ctrl_reg.write(data);
            }
            1 => {
                // Write Mask register
                self.mask_reg.write(data);
            }
            2 => {
                // Status - not writable
                log::warn!("Attempted to write read-only PPU address: {:#X}", addr);
            }
            3 => {
                // Write OAM Adress
                self.oam_addr_reg = data;
            }
            4 => {
                // Write OAM Data
                self.oam_data[self.oam_addr_reg as usize] = data;
                // Writes increment OAM addr
                self.oam_addr_reg = self.oam_addr_reg.wrapping_add(1);
            }
            5 => {
                // Write Scroll register
                self.scroll_reg.write(data);
            }
            6 => {
                // write PPU Address
                self.addr_reg.load(data);
            }
            7 => {
                // Write PPU Data

                // Address to write data to
                let write_addr = self.addr_reg.get();

                // All PPU data writes increment the nametable addr
                self.increment_vram_addr();

                match write_addr {
                    // Addresses mapped to PPU bus
                    0..=0x1FFF => bus.write_chr_mem(write_addr, data),
                    0x2000..=0x2FFF => bus.write_name_tables(write_addr, data),

                    // Unused addresses
                    0x3000..=0x3EFF => log::warn!("address space 0x3000..0x3EFF is not expected to be used, but it was attempted to write at 0x{:#X}", write_addr),

                    // Palette table:
                    // Addresses $3F10/$3F14/$3F18/$3F1C are mirrors of $3F00/$3F04/$3F08/$3F0C
                    // (usually, used for transparency)
                    0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                        let add_mirror = addr - 0x10;
                        self.palette_table[(add_mirror - 0x3F00) as usize] = data;
                    }
                    0x3F00..=0x3FFF => self.palette_table[(addr - 0x3F00) as usize] = data,

                    _ => unreachable!("unexpected write to mirrored space {:#X}", write_addr),
                }
            }
            _ => {
                unreachable!("unexpected write to mirrored space {:#X}", addr);
            }
        }
    }

    pub fn write_oam_dma(&mut self, buffer: &[u8; 256]) {
        for data in buffer.iter() {
            self.oam_data[self.oam_addr_reg as usize] = *data;
            self.oam_addr_reg = self.oam_addr_reg.wrapping_add(1);
        }
    }

    pub fn read(&mut self, bus: &mut PpuBus<'_>, addr: u16) -> u8 {
        let addr = addr & 0x07; // mirror

        match addr {
            // Not readable addresses
            0 | 1 | 3 | 5 | 6 => {
                // Control, mask, OAM address, scroll, PPU Address
                log::warn!("Attempted to read write-only PPU address: {:#X}", addr);
                0
            }

            // Readable addresses
            2 => {
                // Read Status
                let snapshot = self.status_reg.read();
                self.status_reg.remove(registers::StatusReg::VBLANK_STARTED);
                self.addr_reg.reset_latch();
                self.scroll_reg.reset_latch();
                snapshot
            }
            4 => {
                // Read OAM Data
                // Reads do not cause increment
                self.oam_data[self.oam_addr_reg as usize]
            }
            7 => {
                // Read PPU Data

                // Address to read data from
                let read_addr = self.addr_reg.get();

                // All PPU data reads increment the nametable addr
                self.increment_vram_addr();

                match read_addr {
                    // Addresses mapped to PPU bus
                    0..=0x1FFF => bus.read_chr_mem(read_addr),
                    0x2000..=0x2FFF => bus.read_name_tables(read_addr),

                    // Unused address space
                    0x3000..=0x3EFF => {
                        log::warn!("address space 0x3000..0x3EFF is not expected to be used, but 0x{:#X} was requested", read_addr);
                        0
                    }

                    // Palette table is not behind bus, it can be directly returned.
                    // FIXME: do we need to mirror as well?
                    0x3F00..=0x3FFF => self.palette_table[usize::from(read_addr - 0x3F00)],

                    _ => unreachable!("unexpected access to mirrored space {:#X}", read_addr),
                }
            }

            _ => {
                unreachable!("unexpected access to mirrored space {:#X}", addr);
            }
        }
    }

    /// Returns frame when it's ready
    #[allow(unused_variables)] // FIXME
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

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::cartridge::Mirroring;
    use crate::Cartridge;

    const ROM_HORIZONTAL: &'static [u8] = include_bytes!("../../../test_roms/1.Branch_Basics.nes");
    const ROM_VERTICAL: &'static [u8] = include_bytes!("../../../test_roms/Alter_Ego.nes");

    struct MockEmulator {
        cartridge: Cartridge,
        ppu: Ppu,
        name_tables: [u8; 1024 * 2],
        last_data_on_ppu_bus: u8,
    }

    fn mock_emu(rom: &[u8]) -> MockEmulator {
        MockEmulator {
            cartridge: Cartridge::load(rom).unwrap(),
            ppu: Ppu::default(),
            name_tables: [0u8; 1024 * 2],
            last_data_on_ppu_bus: 0,
        }
    }

    fn mock_emu_horizontal() -> MockEmulator {
        mock_emu(ROM_HORIZONTAL)
    }

    fn mock_emu_vertical() -> MockEmulator {
        mock_emu(ROM_VERTICAL)
    }

    #[test]
    fn name_tables_writes() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        assert_eq!(emu.name_tables[0x0305], 0x66);
    }

    #[test]
    fn name_tables_reads() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        assert_ne!(emu.ppu.read(&mut bus, 0x2007), 0x66); // dummy read, returns last data loaded on the bus
        assert_eq!(emu.ppu.addr_reg.get(), 0x2306); // address is incremented
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn name_tables_reads_cross_page() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x01FF] = 0x66;
        emu.name_tables[0x0200] = 0x77;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0xFF);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    #[test]
    fn name_tables_reads_step_32() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x01FF] = 0x66;
        emu.name_tables[0x01FF + 32] = 0x77;
        emu.name_tables[0x01FF + 64] = 0x88;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b100);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0xFF);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x88);
    }

    // Horizontal
    // [0x2000 A ] [0x2400 a ]
    // [0x2800 B ] [0x2C00 b ]
    #[test]
    fn name_tables_horizontal_mirror() {
        let mut emu = mock_emu_horizontal();
        assert!(matches!(emu.cartridge.mirroring(), Mirroring::Horizontal));
        let mut bus = borrow_ppu_bus!(emu);

        // a
        emu.ppu.write(&mut bus, 0x2006, 0x24);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        // B
        emu.ppu.write(&mut bus, 0x2006, 0x28);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x77);

        // A
        emu.ppu.write(&mut bus, 0x2006, 0x20);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        // b
        emu.ppu.write(&mut bus, 0x2006, 0x2C);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    // Vertical
    // [0x2000 A ] [0x2400 B ]
    // [0x2800 a ] [0x2C00 b ]
    #[test]
    fn name_tables_vertical_mirror() {
        let mut emu = mock_emu_vertical();
        assert!(matches!(emu.cartridge.mirroring(), Mirroring::Vertical));
        let mut bus = borrow_ppu_bus!(emu);

        // A
        emu.ppu.write(&mut bus, 0x2006, 0x20);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x66);

        // b
        emu.ppu.write(&mut bus, 0x2006, 0x2C);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.write(&mut bus, 0x2007, 0x77);

        // a
        emu.ppu.write(&mut bus, 0x2006, 0x28);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        // B
        emu.ppu.write(&mut bus, 0x2006, 0x24);
        emu.ppu.write(&mut bus, 0x2006, 0x05);
        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x77);
    }

    #[test]
    fn name_tables_mirroring() {
        let mut emu = mock_emu_horizontal();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2000, 0b0);

        emu.ppu.write(&mut bus, 0x2006, 0x63); // 0x6305 -> 0x2305
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn read_status_resets_latch() {
        let mut emu = mock_emu_vertical();
        emu.name_tables[0x0305] = 0x66;
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2006, 0x21);
        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_ne!(emu.ppu.read(&mut bus, 0x2007), 0x66);

        emu.ppu.read(&mut bus, 0x2002);

        emu.ppu.write(&mut bus, 0x2006, 0x23);
        emu.ppu.write(&mut bus, 0x2006, 0x05);

        emu.ppu.read(&mut bus, 0x2007);
        assert_eq!(emu.ppu.read(&mut bus, 0x2007), 0x66);
    }

    #[test]
    fn read_status_resets_vblank() {
        let mut emu = mock_emu_horizontal();
        emu.ppu
            .status_reg
            .set(registers::StatusReg::VBLANK_STARTED, true);
        let mut bus = borrow_ppu_bus!(emu);

        assert_eq!(emu.ppu.read(&mut bus, 0x2002) >> 7, 1);
        assert_eq!(emu.ppu.status_reg.read() >> 7, 0);
    }

    #[test]
    fn oam_read_write() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        emu.ppu.write(&mut bus, 0x2004, 0x66);
        emu.ppu.write(&mut bus, 0x2004, 0x77);

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x66);

        emu.ppu.write(&mut bus, 0x2003, 0x11);
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x77);
    }

    // TODO: http://wiki.nesdev.com/w/index.php/PPU_registers#OAMDMA
    #[test]
    fn oam_dma() {
        let mut emu = mock_emu_horizontal();
        let mut bus = borrow_ppu_bus!(emu);

        let mut data = [0x66; 256];
        data[0] = 0x77;
        data[255] = 0x88;

        emu.ppu.write(&mut bus, 0x2003, 0x10);
        emu.ppu.write_oam_dma(&data);

        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x77);
        emu.ppu.write(&mut bus, 0x2003, 0x0F); // "wrap around"
        assert_eq!(emu.ppu.read(&mut bus, 0x2004), 0x88);
    }
}

use crate::bus::PpuBus;

/// Registers definitions
pub mod registers;

pub const FRAME_WIDTH: usize = 256;
pub const FRAME_HEIGHT: usize = 240;

pub type PpuFrame = [u8; FRAME_WIDTH * FRAME_HEIGHT];

// TODO: at some point, we need to set the StatusReg::SPRITE_OVERFLOW flag!
// See: https://wiki.nesdev.com/w/index.php/PPU_sprite_evaluation#Sprite_overflow_bug

pub struct Ppu {
    // Internal memory
    palette_table: [u8; 32], // For color stuff
    oam_data: [u8; 64 * 4],  // Object Attribute Memory, internal to PPU

    // Registers
    ctrl_reg: registers::ControlReg,
    mask_reg: registers::MaskReg,
    status_reg: registers::StatusReg,
    oam_addr_reg: u8,
    scroll_reg: registers::ScrollReg,
    addr_reg: registers::VramAddr, // Address register pointing to name tables

    // Emulation-specific internal stuff
    cycle_count: u16,
    scanline: i16,
    frame: PpuFrame,
    vblank_nmi_set: bool,
    last_data_on_bus: u8,
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
            vblank_nmi_set: false,
            last_data_on_bus: 0,
        }
    }

    pub fn reset(&mut self) {
        self.palette_table = [0u8; 32];
        self.oam_data = [0u8; 64 * 4];
        self.ctrl_reg = registers::ControlReg::default();
        self.mask_reg = registers::MaskReg::default();
        self.status_reg = registers::StatusReg::default();
        self.oam_addr_reg = 0;
        self.scroll_reg = registers::ScrollReg::default();
        self.addr_reg = registers::VramAddr::default();
        self.cycle_count = 0;
        self.scanline = 0;
        self.frame = [0u8; FRAME_WIDTH * FRAME_HEIGHT];
        self.vblank_nmi_set = false;
    }

    pub fn take_vblank_nmi_set_state(&mut self) -> bool {
        let state = self.vblank_nmi_set;
        self.vblank_nmi_set = false;
        state
    }

    pub fn write(&mut self, bus: &mut PpuBus<'_>, addr: u16, data: u8) {
        let addr = addr & 0x07; // mirror

        match addr {
            0 => {
                // Write Control register

                let prewrite_generate_nmi_ctrl_state =
                    self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI);

                self.ctrl_reg.write(data);

                let postwrite_generate_nmi_ctrl_state =
                    self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI);
                let is_in_vblank = self
                    .status_reg
                    .contains(registers::StatusReg::VBLANK_STARTED);

                if !prewrite_generate_nmi_ctrl_state
                    && postwrite_generate_nmi_ctrl_state
                    && is_in_vblank
                {
                    self.vblank_nmi_set = true;
                }
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
                    // Mirror some specific addresses to $3F00/$3F04/$3F08/$3F0C
                    // (usually, used for transparency)
                    0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                        let write_addr_mirror = write_addr - 0x0010;
                        self.palette_table[(write_addr_mirror % 32) as usize] = data;
                    }
                    0x3F00..=0x3FFF => self.palette_table[(write_addr % 32) as usize] = data,

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

    #[track_caller]
    pub fn read(&mut self, bus: &mut PpuBus<'_>, addr: u16) -> u8 {
        let addr = addr & 0x07; // mirror

        match addr {
            // Not readable addresses
            0 | 1 | 3 | 5 | 6 => {
                // Control, mask, OAM address, scroll, PPU Address
                log::warn!(
                    "Attempted to read write-only PPU address: {:#X} (culprit at {})",
                    addr,
                    std::panic::Location::caller()
                );
                0
            }

            // Readable addresses
            2 => {
                // Read Status

                // 3 top bits are the PPU status, least significant bits are noise from PPU bus.
                let snapshot = self.status_reg.read() | self.last_data_on_bus & 0x1F;

                // Reading the Status register clear bit 7 and also the address latch used by PPUSCROLL and PPUADDR.
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
                    0..=0x1FFF => {
                        let data = self.last_data_on_bus;
                        self.last_data_on_bus = bus.read_chr_mem(read_addr);
                        data
                    }
                    0x2000..=0x2FFF => {
                        let data = self.last_data_on_bus;
                        self.last_data_on_bus = bus.read_name_tables(read_addr);
                        data
                    }

                    // Unused address space
                    0x3000..=0x3EFF => {
                        log::warn!("address space 0x3000..0x3EFF is not expected to be used, but 0x{:#X} was requested", read_addr);
                        0
                    }

                    // Palette table is not behind bus, it can be directly returned.
                    // Mirror some specific addresses to $3F00/$3F04/$3F08/$3F0C
                    // (usually, used for transparency)
                    0x3F10 | 0x3F14 | 0x3F18 | 0x3F1C => {
                        let read_addr_mirror = read_addr - 0x10;
                        self.palette_table[(read_addr_mirror - 0x3F00) as usize]
                    }
                    0x3F00..=0x3FFF => self.palette_table[usize::from(read_addr - 0x3F00)],

                    _ => unreachable!("unexpected access to mirrored space {:#X}", read_addr),
                }
            }

            _ => {
                unreachable!("unexpected access to mirrored space {:#X}", addr);
            }
        }
    }

    pub fn ready_frame(&mut self) -> Option<&PpuFrame> {
        if self.cycle_count == 0 && self.scanline == -1 {
            // Yeah! We got a frame ready
            Some(&self.frame)
        } else {
            None
        }
    }

    /// Returns frame when it's ready
    pub fn clock(&mut self, bus: &mut PpuBus) {
        self.cycle_count += 1;

        if self.cycle_count >= 341 {
            if self.is_sprite_0_hit() {
                self.status_reg
                    .insert(registers::StatusReg::SPRITE_ZERO_HIT);
            }

            self.cycle_count = 0;
            self.scanline += 1;
            bus.irq_scanline();

            if self.scanline == 241 {
                self.status_reg.insert(registers::StatusReg::VBLANK_STARTED);
                self.status_reg
                    .remove(registers::StatusReg::SPRITE_ZERO_HIT);
                if self.ctrl_reg.contains(registers::ControlReg::GENERATE_NMI) {
                    self.vblank_nmi_set = true;
                }
            } else if self.scanline >= 261 {
                // http://wiki.nesdev.com/w/index.php/PPU_rendering#Pre-render_scanline_.28-1_or_261.29
                // scanline = -1 is the dummy scanline
                self.scanline = -1;

                // VBLANK is done
                self.status_reg.remove(registers::StatusReg::VBLANK_STARTED);

                // FIXME: temporary workaround for quick and dirty, but somewhat working, rendering
                self.dump_sprites(bus);
            }
        }

        self.render_pixel(bus);
    }

    fn render_pixel(&mut self, bus: &mut PpuBus) {
        use std::convert::TryFrom;

        if self.scanline < 0 || self.scanline > 239 || self.cycle_count > 255 {
            return;
        }

        let scroll_x = self.scroll_reg.scroll_x();
        let scroll_y = self.scroll_reg.scroll_y();

        let bank = self.ctrl_reg.background_pattern_base_addr();
        let nametable_base_addr = self.ctrl_reg.nametable_base_addr();

        let x = self.cycle_count;
        let y = u16::try_from(self.scanline).unwrap();

        let x_scrolled = x.wrapping_add(scroll_x as u16);
        let y_scrolled = y.wrapping_add(scroll_y as u16);
        
        let mut quadrant: u8 = 0; 
        let mut offset = 0;

        let tile_x = if x_scrolled < (32*8) {x_scrolled / 8} else {quadrant |= 1; offset |= 0x400; (x_scrolled - 32*8) / 8};
        let tile_y = if y_scrolled < (32*8) {y_scrolled / 8} else {quadrant |= 2; offset |= 0x800; (y_scrolled - 32*8) / 8};
        
        let tile_idx = tile_y * 32 + tile_x + offset;

        let tile = bus.read_name_tables(nametable_base_addr + tile_idx);

        let pat_x = 7 - x_scrolled % 8;
        let pat_y = y_scrolled % 8;
        let lo = (bus.read_chr_mem(bank + u16::from(tile) * 16 + pat_y) >> pat_x) & 0b1;
        let hi = (bus.read_chr_mem(bank + u16::from(tile) * 16 + pat_y + 8) >> pat_x) & 0b1;

        let pat = hi << 1 | lo;

        let palette = self.bg_palette(bus, tile_x, tile_y, quadrant);
        let color = palette[pat as usize];

        self.set_pixel(x, y, color);
    }

    fn bg_palette(&mut self, bus: &mut PpuBus, tile_x: u16, tile_y: u16, quadrant: u8) -> [u8; 4] {
        let attr_table_idx = (tile_y / 4) * 8 + (tile_x / 4);
        let nametable_base_addr = self.ctrl_reg.nametable_base_addr();

        let offset = match quadrant & 0b11 {
            0b00 => 0x3c0,
            0x01 => 0x7c0,
            0b10 => 0xbc0,
            0b11 => 0xfc0,
            _ => unreachable!(),
        };

        let attr_base_addr = nametable_base_addr + offset;
        let attr_byte = bus.read_name_tables(attr_base_addr + attr_table_idx);
        let meta_tile_x = (tile_x / 2) % 2;
        let meta_tile_y = (tile_y / 2) % 2;

        let palette_idx = match (meta_tile_x, meta_tile_y) {
            (0, 0) => attr_byte & 0b11,
            (1, 0) => (attr_byte >> 2) & 0b11,
            (0, 1) => (attr_byte >> 4) & 0b11,
            (1, 1) => (attr_byte >> 6) & 0b11,
            _ => unreachable!(),
        };

        let pallete_start = 1 + (palette_idx as usize) * 4;
        [
            self.palette_table[0],
            self.palette_table[pallete_start],
            self.palette_table[pallete_start + 1],
            self.palette_table[pallete_start + 2],
        ]
    }

    // this is mostly for quick debugging
    fn dump_sprites(&mut self, bus: &mut PpuBus) {
        for i in (0..self.oam_data.len()).step_by(4) {
            let tile_idx = u16::from(self.oam_data[i + 1]);
            let tile_x = u16::from(self.oam_data[i + 3]);
            let tile_y = u16::from(self.oam_data[i]);

            let flip_vertical = self.oam_data[i + 2] >> 7 & 1 == 1;

            let flip_horizontal = self.oam_data[i + 2] >> 6 & 1 == 1;

            // find sprite palette
            let palette_idx = self.oam_data[i + 2] & 0b11;
            let sprite_palette = self.sprite_palette(palette_idx);

            // load tile pattern
            let bank: u16 = self.ctrl_reg.sprite_pattern_base_addr();
            let mut tile = [0u8; 16];

            if self.ctrl_reg.sprite_size() == 8 {
                // 8x8 sprites
                for i in 0..16 {
                    tile[i as usize] = bus.read_chr_mem(bank + tile_idx * 16 + i);
                }
                self.dump_tile(sprite_palette, flip_horizontal, flip_vertical, tile, tile_x, tile_y);

            } else {
                // 8x16 sprites
                if !flip_vertical {
                    for i in 0..16 {
                        tile[i as usize] = bus.read_chr_mem(bank + (tile_idx & 0xFE) * 16 + i);
                    }
                    self.dump_tile(sprite_palette, flip_horizontal, flip_vertical, tile, tile_x, tile_y);

                    for i in 0..16 {
                        tile[i as usize] = bus.read_chr_mem(bank + ((tile_idx & 0xFE) + 1) * 16 + i);
                    }
                    self.dump_tile(sprite_palette, flip_horizontal, flip_vertical, tile, tile_x, tile_y + 8);
                } else {
                    for i in 0..16 {
                        tile[i as usize] = bus.read_chr_mem(bank + ((tile_idx & 0xFE) + 1) * 16 + i);
                    }
                    self.dump_tile(sprite_palette, flip_horizontal, flip_vertical, tile, tile_x, tile_y);

                    for i in 0..16 {
                        tile[i as usize] = bus.read_chr_mem(bank + (tile_idx & 0xFE) * 16 + i);
                    }
                    self.dump_tile(sprite_palette, flip_horizontal, flip_vertical, tile, tile_x, tile_y + 8);
                }

            }
        }
    }

    fn dump_tile(&mut self, sprite_palette: [u8; 4], x_flip: bool, y_flip: bool, tile: [u8; 16], tile_x: u16, tile_y: u16) {
        let flip_vertical = if y_flip {
            |x| 7 - x
        } else {
            |x| x
        };

        let flip_horizontal = if x_flip {
            |y| 7 - y
        } else {
            |y| y
        };

        for y in 0..8 {
            let byte_lo = tile[y as usize];
            let byte_hi = tile[y as usize + 8];
            for x in 0..8 {
                let shift = 7 - x;
                let pat_lo = (byte_lo >> shift) & 0b1;
                let pat_hi = (byte_hi >> shift) & 0b1;
                let pat = pat_hi << 1 | pat_lo;
                if pat != 0 {
                    // non-transparant
                    let color = sprite_palette[pat as usize];
                    self.set_pixel(
                        tile_x + flip_horizontal(x),
                        tile_y + flip_vertical(y),
                        color,
                    );
                }
            }
        }
    }

    fn sprite_palette(&mut self, palette_idx: u8) -> [u8; 4] {
        let pallete_start = usize::from(palette_idx + 4) * 4 + 1;
        [
            self.palette_table[0],
            self.palette_table[pallete_start],
            self.palette_table[pallete_start + 1],
            self.palette_table[pallete_start + 2],
        ]
    }

    fn set_pixel(&mut self, x: u16, y: u16, color: u8) {
        let idx = y as usize * FRAME_WIDTH + x as usize;
        if idx < self.frame.len() {
            self.frame[idx] = color;
        }
    }

    /// See: https://wiki.nesdev.com/w/index.php?title=PPU_OAM#Sprite_zero_hits
    fn is_sprite_0_hit(&self) -> bool {
        // Check for sprite zero hit
        // FIXME: this is an approximated simulation
        // Also, as per NESDev wiki: "Sprite 0 hit is not detected at x=255, nor is it detected at x=0 through 7 if the background or sprites are hidden in this area."
        // (more to check)
        let y = i16::from(self.oam_data[0]);
        let x = u16::from(self.oam_data[3]);
        (y == self.scanline)
            && x <= self.cycle_count
            && self.mask_reg.contains(registers::MaskReg::SHOW_SPRITES)
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

    const ROM_HORIZONTAL: &'static [u8] =
        include_bytes!("../../../default_roms/1.Branch_Basics.nes");
    const ROM_VERTICAL: &'static [u8] = include_bytes!("../../../default_roms/Alter_Ego.nes");

    struct MockEmulator {
        cartridge: Cartridge,
        ppu: Ppu,
        name_tables: [u8; 1024 * 2],
    }

    fn mock_emu(rom: &[u8]) -> MockEmulator {
        MockEmulator {
            cartridge: Cartridge::load(rom, None).unwrap(),
            ppu: Ppu::default(),
            name_tables: [0u8; 1024 * 2],
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

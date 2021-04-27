use bitflags::bitflags;

// == VRAM address register == //

/// VRAM address (0x0000..0x3FFF)
/// http://wiki.nesdev.com/w/index.php/PPU_registers#PPUADDR
pub struct VramAddr {
    value: u16,
    latch: bool,
}

impl Default for VramAddr {
    fn default() -> Self {
        Self {
            value: 0,
            latch: false,
        }
    }
}

impl VramAddr {
    pub fn load(&mut self, data: u8) {
        if self.latch {
            // reset and update lower
            self.value &= 0xFF00;
            self.value |= u16::from(data);
        } else {
            // reset and update higher
            self.value &= 0x00FF;
            self.value |= u16::from(data) << 8;
        }
        self.latch = !self.latch;
        self.mirror();
    }

    pub fn get(&self) -> u16 {
        self.value
    }

    pub fn inc(&mut self, step: u8) {
        self.value = self.value.wrapping_add(u16::from(step));
        self.mirror();
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }

    fn mirror(&mut self) {
        const MIRRORING_HIGHER_BOUND: u16 = 0x3FFF;
        self.value &= MIRRORING_HIGHER_BOUND;
    }
}

// == control register == //

bitflags! {
    /// http://wiki.nesdev.com/w/index.php/PPU_registers#PPUCTRL
    pub struct ControlReg: u8 {
        /// N: Base nametable address lower bit
        const NAMETABLE_ADDR_LO = 0b00000001;
        /// N: Base nametable address higher bit
        const NAMETABLE_ADDR_HI = 0b00000010;
        /// N: Base nametable address bits
        /// (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
        const NAMETABLE_ADDR = Self::NAMETABLE_ADDR_LO.bits | Self::NAMETABLE_ADDR_HI.bits;

        /// I: RAM address increment per CPU read/write of PPUDATA
        /// (0: add 1, going across; 1: add 32, going down)
        const VRAM_ADDR_INCREMENT = 0b00000100;

        /// S: Sprite pattern table address for 8x8 sprites
        /// (0: $0000; 1: $1000; ignored in 8x16 mode)
        const SPRITE_PATTERN_ADDR = 0b00001000;

        /// B: Background pattern table address (0: $0000; 1: $1000)
        const BACKGROUND_PATTERN_ADDR = 0b00010000;

        /// H: Sprite size (0: 8x8 pixels; 1: 8x16 pixels)
        const SPRITE_SIZE = 0b00100000;

        /// P: PPU master/slave select
        /// (0: read backdrop from EXT pins; 1: output color on EXT pins)
        const MASTER_SLAVE_SELECT = 0b01000000;

        /// V: Generate an NMI at the start of the
        /// vertical blanking interval (0: off; 1: on)
        const GENERATE_NMI = 0b10000000;
    }
}

impl Default for ControlReg {
    fn default() -> Self {
        Self::empty()
    }
}

impl ControlReg {
    pub fn write(&mut self, data: u8) {
        self.bits = data;
    }

    pub fn vram_addr_increment(&self) -> u8 {
        if self.contains(Self::VRAM_ADDR_INCREMENT) {
            // vertical mode: increment is 32 so it skips one whole nametable row.
            32
        } else {
            // horizontal mode: increment is 1, so it moves to the next column.
            1
        }
    }
}

// == scroll register == //

/// http://wiki.nesdev.com/w/index.php/PPU_registers#PPUSCROLL
pub struct ScrollReg {
    scroll_x: u8,
    scroll_y: u8,
    latch: bool,
}

impl Default for ScrollReg {
    fn default() -> Self {
        ScrollReg {
            scroll_x: 0,
            scroll_y: 0,
            latch: false,
        }
    }
}

impl ScrollReg {
    pub fn write(&mut self, data: u8) {
        if self.latch {
            self.scroll_y = data;
        } else {
            self.scroll_x = data;
        }
        self.latch = !self.latch;
    }

    pub fn reset_latch(&mut self) {
        self.latch = false;
    }

    pub fn scroll_x(&self) -> u8 {
        self.scroll_x
    }

    pub fn scroll_y(&self) -> u8 {
        self.scroll_y
    }
}

// == status register == //

bitflags! {
    /// http://wiki.nesdev.com/w/index.php/PPU_registers#PPUSTATUS
    pub struct StatusReg: u8 {
        /// O: Sprite overflow. The intent was for this flag to be set
        /// whenever more than eight sprites appear on a scanline, but a
        /// hardware bug causes the actual behavior to be more complicated
        /// and generate false positives as well as false negatives; see
        /// PPU sprite evaluation. This flag is set during sprite
        /// evaluation and cleared at dot 1 (the second dot) of the
        /// pre-render line.
        const SPRITE_OVERFLOW  = 0b00100000;

        /// S: Sprite 0 Hit.  Set when a nonzero pixel of sprite 0 overlaps
        /// a nonzero background pixel; cleared at dot 1 of the pre-render
        /// line.  Used for raster timing.
        const SPRITE_ZERO_HIT  = 0b01000000;

        /// V: Vertical blank has started (0: not in vblank; 1: in vblank).
        /// Set at dot 1 of line 241 (the line *after* the post-render
        /// line); cleared after reading $2002 and at dot 1 of the
        /// pre-render line.
        const VBLANK_STARTED   = 0b10000000;
    }
}

impl Default for StatusReg {
    fn default() -> Self {
        StatusReg::empty()
    }
}

impl StatusReg {
    pub fn read(&self) -> u8 {
        self.bits
    }
}

// == mask register == //

bitflags! {
    /// http://wiki.nesdev.com/w/index.php/PPU_registers#PPUMASK
    pub struct MaskReg: u8 {
        /// G: Greyscale (0: normal color, 1: produce a greyscale display)
        const GREYSCALE = 0b00000001;

        /// m: 1: Show background in leftmost 8 pixels of screen, 0: Hide
        const LEFTMOST_8PXL_BACKGROUND = 0b00000010;

        /// M: 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
        const LEFTMOST_8PXL_SPRITE = 0b00000100;

        /// b: 1: Show background
        const SHOW_BACKGROUND = 0b00001000;

        /// s: 1: Show sprites
        const SHOW_SPRITES = 0b00010000;

        /// R: Emphasize red
        const EMPHASISE_RED = 0b00100000;

        /// G: Emphasize green
        const EMPHASISE_GREEN = 0b01000000;

        /// B: Emphasize blue
        const EMPHASISE_BLUE = 0b10000000;
    }
}

impl Default for MaskReg {
    fn default() -> Self {
        MaskReg::empty()
    }
}

impl MaskReg {
    pub fn write(&mut self, data: u8) {
        self.bits = data;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vram_addr_mirroring() {
        let mut reg = VramAddr {
            value: 0b1001_1110_1111_1111,
            latch: true,
        };
        reg.mirror();
        assert_eq!(reg.get(), 0b0001_1110_1111_1111);
    }

    #[test]
    fn vram_addr_load() {
        let mut reg = VramAddr::default();
        reg.load(0xAC);
        assert_eq!(reg.get(), 0x2C00);
        reg.load(0x5F);
        assert_eq!(reg.get(), 0x2C5F);
        reg.load(0x06);
        assert_eq!(reg.get(), 0x065F);
        reg.load(0x00);
        assert_eq!(reg.get(), 0x0600);
    }

    #[test]
    fn vram_addr_inc() {
        let mut reg = VramAddr::default();
        reg.load(0x3F);
        reg.load(0xFF);
        assert_eq!(reg.get(), 0x3FFF);
        reg.inc(0x01);
        assert_eq!(reg.get(), 0x0000);
        reg.inc(0x0F);
        assert_eq!(reg.get(), 0x000F);
    }
}

use bitfield::bitfield;
use bitflags::bitflags;

bitfield! {
    /// A Vram address. Used to read and write on the PPU bus and during rendering
    #[derive(Clone, Copy)]
    pub struct VramAddr(u16);

    /// The X coordinate of the tile index
    pub coarse_x, set_coarse_x: 4, 0;

    /// The Y coordinate of the tile index
    pub coarse_y, set_coarse_y: 9, 5;

    /// The current nametable
    pub nametable, set_nametable: 11, 10;

    /// The Y coordinate of the pixel inside the tile
    pub fine_y, set_fine_y: 14, 12;
}

impl Default for VramAddr {
    fn default() -> Self {
        Self(0)
    }
}

impl VramAddr {
    pub fn get(&self) -> u16 {
        self.0
    }

    pub fn set(&mut self, val: u16) {
        self.0 = val;
    }

    pub fn increment_coarse_x(&mut self) {
        if self.coarse_x() == 31 {
            self.set_coarse_x(0);

            // Flip horizontal nametable
            self.set_nametable(self.nametable() ^ 0b01);
        } else {
            self.set_coarse_x(self.coarse_x() + 1);
        }
    }

    pub fn increment_fine_y(&mut self) {
        if self.fine_y() == 7 {
            self.set_fine_y(0);
            let coarse_y = self.coarse_y();

            if coarse_y == 29 {
                self.set_coarse_y(0);

                // Flip vertical nametable
                self.set_nametable(self.nametable() ^ 0b10);
            } else if coarse_y == 31 {
                // Specific edge case to emulate, nametable is not flipped
                self.set_coarse_y(0);
            } else {
                self.set_coarse_y(coarse_y + 1)
            }
        } else {
            self.set_fine_y(self.fine_y() + 1);
        }
    }

    pub fn reset_x(&mut self, other: &Self) {
        self.set_coarse_x(other.coarse_x());

        // Reset only x part of nametable
        let nametable = (self.nametable() & 0b10) | (other.nametable() & 0b01);
        self.set_nametable(nametable);
    }

    pub fn reset_y(&mut self, other: &Self) {
        self.set_coarse_y(other.coarse_y());
        self.set_fine_y(other.fine_y());

        // Reset only y part of nametable
        let nametable = (self.nametable() & 0b01) | (other.nametable() & 0b10);
        self.set_nametable(nametable);
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

    pub fn sprite_pattern_base_addr(&self) -> u16 {
        if self.contains(Self::SPRITE_PATTERN_ADDR) {
            0x1000
        } else {
            0x0000
        }
    }

    pub fn background_pattern_base_addr(&self) -> u16 {
        if self.contains(ControlReg::BACKGROUND_PATTERN_ADDR) {
            0x1000
        } else {
            0x0000
        }
    }

    #[allow(dead_code)]
    pub fn sprite_size(&self) -> u8 {
        if self.contains(ControlReg::SPRITE_SIZE) {
            16
        } else {
            8
        }
    }

    #[allow(dead_code)]
    pub fn master_slave_select(&self) -> u8 {
        if self.contains(ControlReg::SPRITE_SIZE) {
            1
        } else {
            0
        }
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

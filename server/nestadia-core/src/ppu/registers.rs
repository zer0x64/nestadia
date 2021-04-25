use bitflags::bitflags;

/// VRAM address (0x0000..0x3FFF)
pub struct VramAddr {
    value: u16,
    write_to_lower: bool,
}

impl VramAddr {
    pub fn new() -> Self {
        Self {
            value: 0,
            write_to_lower: false,
        }
    }

    pub fn load(&mut self, data: u8) {
        if self.write_to_lower {
            // reset and update lower
            self.value &= 0xFF00;
            self.value |= u16::from(data);
        } else {
            // reset and update higher
            self.value &= 0x00FF;
            self.value |= u16::from(data) << 8;
        }
        self.write_to_lower = !self.write_to_lower;
        self.mirror();
    }

    pub fn get(&self) -> u16 {
        self.value
    }

    pub fn inc(&mut self, step: u8) {
        self.value = self.value.wrapping_add(u16::from(step));
        self.mirror();
    }

    fn mirror(&mut self) {
        const MIRRORING_HIGHER_BOUND: u16 = 0x3FFF;
        self.value &= MIRRORING_HIGHER_BOUND;
    }
}

bitflags! {
    pub struct ControllerReg: u8 {
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

impl Default for ControllerReg {
    fn default() -> Self {
        Self::empty()
    }
}

impl ControllerReg {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vram_addr_mirroring() {
        let mut reg = VramAddr {
            value: 0b1001_1110_1111_1111,
            write_to_lower: true,
        };
        reg.mirror();
        assert_eq!(reg.get(), 0b0001_1110_1111_1111);
    }

    #[test]
    fn vram_addr_load() {
        let mut reg = VramAddr::new();
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
        let mut reg = VramAddr::new();
        reg.load(0x3F);
        reg.load(0xFF);
        assert_eq!(reg.get(), 0x3FFF);
        reg.inc(0x01);
        assert_eq!(reg.get(), 0x0000);
        reg.inc(0x0F);
        assert_eq!(reg.get(), 0x000F);
    }
}

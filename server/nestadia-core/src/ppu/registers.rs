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
        // can't overflow
        self.value += u16::from(step);
        self.mirror();
    }

    fn mirror(&mut self) {
        const MIRRORING_HIGHER_BOUND: u16 = 0x3FFF;
        self.value &= MIRRORING_HIGHER_BOUND;
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

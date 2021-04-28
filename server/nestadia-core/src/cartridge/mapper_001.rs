use super::Mapper;

pub struct Mapper001 {
    prg_bank_selector: u8,
    chr_bank_selector: u8,
    shift_register: u8,
}

impl Mapper001 {
    pub fn new(prg_banks: u8) -> Self {
        Self {
            prg_bank_selector: 0,
            chr_bank_selector: 0,
            shift_register: 0x10000,
        }
    }
}

impl Mapper for Mapper001 {
    fn cpu_map_read(&self, addr: u16) -> u16 {
        // TODO
        match addr {
            0x8000..=0xBFFF => (self.prg_bank_selector as u16) * 0x4000 + (addr & 0x3FFF),
            _ => (self.prg_banks as u16 - 1) * 0x4000 + (addr & 0x3FFF),
        }
    }

    fn cpu_map_write(&mut self, _addr: u16, data: u8) {
        // TODO
        self.prg_bank_selector = data;
    }

    fn ppu_map_read(&self, addr: u16) -> u16 {
        let mut mapped = if (0 ..=0x0FFF as u16).contains(&addr) {0} else {0x1000};
        mapped + (self.chr_bank_selector as u16) * 0x1000 + (addr & 0x0FFF)
    }

    fn ppu_map_write(&self, _addr: u16) -> Option<u16> {
        None
    }
}

use crate::cartridge::Cartridge;
use crate::cartridge::Mirroring;
use crate::Apu;
use crate::Ppu;
use crate::RAM_SIZE;

macro_rules! borrow_cpu_bus {
    ($owner:ident) => {{
        $crate::bus::CpuBus::borrow(
            &mut $owner.controller1,
            &mut $owner.controller2,
            &mut $owner.controller_state,
            &mut $owner.controller1_snapshot,
            &mut $owner.controller2_snapshot,
            &mut $owner.ram,
            &mut $owner.apu,
            &mut $owner.cartridge,
            &mut $owner.ppu,
            &mut $owner.name_tables,
        )
    }};
}

macro_rules! borrow_ppu_bus {
    ($owner:ident) => {{
        $crate::bus::PpuBus::borrow(&mut $owner.cartridge, &mut $owner.name_tables)
    }};
}

pub struct CpuBus<'a> {
    controller1: &'a mut u8,
    controller2: &'a mut u8,
    controller_state: &'a mut bool,
    controller1_snapshot: &'a mut u8,
    controller2_snapshot: &'a mut u8,
    ram: &'a mut [u8; RAM_SIZE as usize],
    apu: &'a mut Apu,
    cartridge: &'a mut Cartridge,
    ppu: &'a mut Ppu,
    name_tables: &'a mut [u8; 1024 * 4],
}

impl<'a> CpuBus<'a> {
    #[allow(clippy::too_many_arguments)] // it's fine, it's used by a macro
    pub fn borrow(
        controller1: &'a mut u8,
        controller2: &'a mut u8,
        controller_state: &'a mut bool,
        controller1_snapshot: &'a mut u8,
        controller2_snapshot: &'a mut u8,
        ram: &'a mut [u8; RAM_SIZE as usize],
        apu: &'a mut Apu,
        cartridge: &'a mut Cartridge,
        ppu: &'a mut Ppu,
        name_tables: &'a mut [u8; 1024 * 4],
    ) -> Self {
        Self {
            controller1,
            controller2,
            controller_state,
            controller1_snapshot,
            controller2_snapshot,
            ram,
            apu,
            cartridge,
            ppu,
            name_tables,
        }
    }
}

impl CpuBus<'_> {
    pub fn write_ram(&mut self, addr: u16, data: u8) {
        self.ram[(addr & (RAM_SIZE - 1)) as usize] = data;
    }

    pub fn read_ram(&mut self, addr: u16) -> u8 {
        self.ram[(addr & (RAM_SIZE - 1)) as usize]
    }

    pub fn write_apu_register(&mut self, addr: u16, data: u8) {
        self.apu.write(0 /*todo*/, addr, data);
    }

    #[track_caller]
    pub fn read_apu_register(&mut self, addr: u16) -> u8 {
        self.apu.read(0 /*todo*/, addr)
    }

    pub fn write_ppu_register(&mut self, addr: u16, data: u8) {
        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.write(&mut ppu_bus, addr, data);
    }

    #[track_caller]
    pub fn read_ppu_register(&mut self, addr: u16) -> u8 {
        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.read(&mut ppu_bus, addr)
    }

    pub fn controller_write(&mut self, data: u8) {
        *self.controller_state = data & 0x01 == 0x01;
        *self.controller1_snapshot = *self.controller1;
        *self.controller2_snapshot = *self.controller2;
    }

    pub fn read_controller1_snapshot(&mut self) -> u8 {
        if *self.controller_state {
            *self.controller1 & 0x80 >> 7
        } else {
            let data = (*self.controller1_snapshot & 0x80) >> 7;
            *self.controller1_snapshot <<= 1;
            data
        }
    }

    pub fn read_controller2_snapshot(&mut self) -> u8 {
        if *self.controller_state {
            *self.controller2 & 0x80 >> 7
        } else {
            let data = (*self.controller2_snapshot & 0x80) >> 7;
            *self.controller2_snapshot <<= 1;
            data
        }
    }

    pub fn write_prg_mem(&mut self, addr: u16, data: u8) {
        self.cartridge.write_prg_mem(addr, data)
    }

    pub fn read_prg_mem(&mut self, addr: u16) -> u8 {
        self.cartridge.read_prg_mem(addr)
    }

    pub fn write_ppu_oam_dma(&mut self, buffer: &[u8; 256]) {
        self.ppu.write_oam_dma(buffer);
    }
}

pub struct PpuBus<'a> {
    cartridge: &'a mut Cartridge,
    name_tables: &'a mut [u8; 1024 * 4],
}

impl<'a> PpuBus<'a> {
    pub fn borrow(cartridge: &'a mut Cartridge, name_tables: &'a mut [u8; 1024 * 4]) -> Self {
        Self {
            cartridge,
            name_tables,
        }
    }
}

impl PpuBus<'_> {
    // Read returns the data fetched from the previous load operation and internal buffer is
    // updated. Load operation must be called twice in order to get the desired data.

    /// Read CHR memory from cartridge
    pub fn read_chr_mem(&mut self, addr: u16) -> u8 {
        self.cartridge.read_chr_mem(addr)
    }

    /// Write to CHR memory on cartridge (if writable)
    pub fn write_chr_mem(&mut self, addr: u16, data: u8) {
        self.cartridge.write_chr_mem(addr, data);
    }

    pub fn read_name_tables(&mut self, addr: u16) -> u8 {
        self.name_tables[self.mirror_name_tables_addr(addr) as usize]
    }

    pub fn write_name_tables(&mut self, addr: u16, data: u8) {
        self.name_tables[self.mirror_name_tables_addr(addr) as usize] = data;
    }

    // http://wiki.nesdev.com/w/index.php/Mirroring#Nametable_Mirroring
    fn mirror_name_tables_addr(&self, addr: u16) -> u16 {
        let mirrored = addr & 0x2FFF; // mirror to $2000-$2FFF range
        let idx = mirrored - 0x2000; // project to array indexing range
        match self.cartridge.mirroring() {
            Mirroring::Horizontal => match idx {
                0..=1023 => idx,
                1024..=2047 => idx - 1024,
                2048..=3071 => idx - 1024,
                3072..=4095 => idx - 2048,
                _ => unreachable!(),
            },
            Mirroring::Vertical => match idx {
                0..=2047 => idx,
                2048..=4095 => idx - 2048,
                _ => unreachable!(),
            },
            Mirroring::FourScreen => idx,
            Mirroring::OneScreenLower => match idx {
                0..=1023 => idx,
                1024..=2047 => idx - 1024,
                2048..=3071 => idx - 2048,
                3072..=4095 => idx - 3072,
                _ => unreachable!(),
            },
            Mirroring::OneScreenUpper => match idx {
                0..=1023 => idx + 1024,
                1024..=2047 => idx,
                2048..=3071 => idx - 1024,
                3072..=4095 => idx - 2048,
                _ => unreachable!(),
            },
        }
    }
}

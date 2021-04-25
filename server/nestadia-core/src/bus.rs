use crate::Ppu;
use crate::RAM_SIZE;
use crate::cartridge::Cartridge;

macro_rules! borrow_cpu_bus {
    ($owner:ident) => {{
        $crate::bus::CpuBus::borrow(
            &mut $owner.controller1,
            &mut $owner.controller2,
            &mut $owner.controller1_snapshot,
            &mut $owner.controller2_snapshot,
            &mut $owner.ram,
            &mut $owner.cartridge,
            &mut $owner.ppu,
            &mut $owner.name_tables,
            &mut $owner.last_data_on_ppu_bus,
        )
    }}
}

macro_rules! borrow_ppu_bus {
    ($owner:ident) => {{
        $crate::bus::PpuBus::borrow(
            &mut $owner.cartridge,
            &mut $owner.name_tables,
            &mut $owner.last_data_on_ppu_bus,
        )
    }}
}

pub struct CpuBus<'a> {
    controller1: &'a mut u8,
    controller2: &'a mut u8,
    controller1_snapshot: &'a mut u8,
    controller2_snapshot: &'a mut u8,
    ram: &'a mut [u8; RAM_SIZE as usize],
    cartridge: &'a mut Cartridge,
    ppu: &'a mut Ppu,
    name_tables: &'a mut [u8; 1024 * 2],
    last_data_on_ppu_bus: &'a u8,
}

impl<'a> CpuBus<'a> {
    pub fn borrow(
        controller1: &'a mut u8,
        controller2: &'a mut u8,
        controller1_snapshot: &'a mut u8,
        controller2_snapshot: &'a mut u8,
        ram: &'a mut [u8; RAM_SIZE as usize],
        cartridge: &'a mut Cartridge,
        ppu: &'a mut Ppu,
        name_tables: &'a mut [u8; 1024 * 2],
        last_data_on_ppu_bus: &'a u8,
    ) -> Self {
        Self {
            controller1,
            controller2,
            controller1_snapshot,
            controller2_snapshot,
            ram,
            cartridge,
            ppu,
            name_tables,
            last_data_on_ppu_bus,
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

    pub fn write_ppu_register(&mut self, addr: u16, data: u8) {
        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.write_ppu_register(&mut ppu_bus, addr, data);
    }

    pub fn read_ppu_register(&mut self, addr: u16) -> u8 {
        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.read_ppu_register(&mut ppu_bus, addr)
    }

    pub fn controller1_take_snapshot(&mut self) {
        *self.controller1_snapshot = *self.controller1;
    }

    pub fn read_controller1_snapshot(&mut self) -> u8 {
        let data = *self.controller1_snapshot & 0x80 >> 7;
        *self.controller1_snapshot <<= 1;
        data
    }

    pub fn controller2_take_snapshot(&mut self) {
        *self.controller2_snapshot = *self.controller2;
    }

    pub fn read_controller2_snapshot(&mut self) -> u8 {
        let data = *self.controller2_snapshot & 0x80 >> 7;
        *self.controller2_snapshot <<= 1;
        data
    }

    pub fn write_prg_mem(&mut self, addr: u16, data: u8) {
        self.cartridge.write_prg_mem(addr, data)
    }

    pub fn read_prg_mem(&mut self, addr: u16) -> u8 {
        self.cartridge.read_prg_mem(addr)
    }
}

pub struct PpuBus<'a> {
    cartridge: &'a mut Cartridge,
    name_tables: &'a mut [u8; 1024 * 2],
    last_data_on_ppu_bus: &'a u8,
}

impl<'a> PpuBus<'a> {
    pub fn borrow(
        cartridge: &'a mut Cartridge,
        name_tables: &'a mut [u8; 1024 * 2],
        last_data_on_ppu_bus: &'a u8,
    ) -> Self {
        Self {
            cartridge,
            name_tables,
            last_data_on_ppu_bus,
        }
    }
}

impl PpuBus<'_> {
    // Read returns the data fetched from the previous load operation and internal buffer is
    // updated. Load operation must be called twice in order to get the desired data.

    pub fn read_chr_mem(&mut self, addr: u16) -> u8 {
        let data = *self.last_data_on_ppu_bus;
        *self.last_data_on_ppu_bus = todo!("read from chr_rom");
        data
    }

    pub fn read_name_tables(&mut self, addr: u16) -> u8 {
        let data = *self.last_data_on_ppu_bus;
        *self.last_data_on_ppu_bus = todo!("name tables mirroring for {}", addr);
        data
    }
}

mod cartridge;
mod cpu;
mod ppu;

use std::ops::Deref;
use std::ops::DerefMut;

use log;

use cartridge::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;
use ppu::PpuFrame;

const RAM_SIZE: u16 = 0x800;

#[derive(Debug, Clone)]
pub enum RomParserError {
    TooShort,
    InvalidMagicBytes,
    MapperNotImplemented,
}

impl std::fmt::Display for RomParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", &self)
    }
}

impl std::error::Error for RomParserError {}

pub trait BusInterface {
    fn cpu_write(&mut self, address: u16, data: u8);
    fn cpu_read(&mut self, address: u16, _read_only: bool) -> u8;
    fn ppu_write(&mut self, address: u16, data: u8);
    fn ppu_read(&self, address: u16, _read_only: bool) -> u8;
}

trait EmulatorContext<T>: BusInterface {
    fn get_mut(&mut self) -> &mut T;
    fn get_ref(&self) -> &T;
}

impl<T> Deref for dyn EmulatorContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.get_ref()
    }
}

impl<T> DerefMut for dyn EmulatorContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl EmulatorContext<Cpu> for Emulator {
    fn get_mut(&mut self) -> &mut Cpu {
        &mut self.cpu
    }

    fn get_ref(&self) -> &Cpu {
        &self.cpu
    }
}

impl EmulatorContext<Ppu> for Emulator {
    fn get_mut(&mut self) -> &mut Ppu {
        &mut self.ppu
    }

    fn get_ref(&self) -> &Ppu {
        &self.ppu
    }
}

#[derive(Clone, Debug, Copy)]
pub enum ExecutionMode {
    Ring0,
    Ring3,
}

pub struct Emulator {
    pub cpu: Cpu,
    pub ppu: Ppu,
    controller1: u8,
    controller2: u8,
    controller1_snapshot: u8,
    controller2_snapshot: u8,
    cartridge: Cartridge,
    ram: [u8; RAM_SIZE as usize],
    clock_count: u8,
}

impl Emulator {
    pub fn new(rom: &[u8], execution_mode: ExecutionMode) -> Result<Self, RomParserError> {
        let mut emulator = Self {
            cpu: Cpu::new(execution_mode),
            ppu: Ppu::new(),
            controller1: 0,
            controller2: 0,
            controller1_snapshot: 0,
            controller2_snapshot: 0,
            cartridge: Cartridge::new(rom)?,
            ram: [0u8; RAM_SIZE as usize],
            clock_count: 0,
        };

        EmulatorContext::<Cpu>::reset(&mut emulator);
        Ok(emulator)
    }

    pub fn clock(&mut self) -> Option<&PpuFrame> {
        // CPU clock is 3 times slower
        if self.clock_count % 3 == 0 {
            EmulatorContext::<Cpu>::clock(self);
            self.clock_count = 0;
        }

        self.clock_count = self.clock_count.wrapping_add(1);

        EmulatorContext::<Ppu>::clock(self)
    }

    pub fn set_controller1(&mut self, state: u8) {
        self.controller1 = state;
    }

    pub fn set_controller2(&mut self, state: u8) {
        self.controller2 = state;
    }

    pub fn reset(&mut self) {
        EmulatorContext::<Cpu>::reset(self);
        self.clock_count = 0;
    }

    #[cfg(feature = "debugger")]
    pub fn disassemble(&self, start: u16, end: u16) -> Vec<(u16, String)> {
        self.cartridge.disassemble()
    }
}

impl BusInterface for Emulator {
    fn cpu_write(&mut self, address: u16, data: u8) {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize] = data,
            0x2000..=0x3fff => EmulatorContext::<Ppu>::write_ppu_controls(self, address, data),
            0x4000..=0x4015 => { /*APU and Audio*/ }
            0x4016 => self.controller1_snapshot = self.controller1,
            0x4017 => self.controller2_snapshot = self.controller2,
            0x4018..=0x401f => { /*APU and Audio*/ }
            0x4020..=0xffff => self.cartridge.cpu_write(address, data),
        };
    }

    fn cpu_read(&mut self, address: u16, _read_only: bool) -> u8 {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize],
            0x2000..=0x3fff => EmulatorContext::<Ppu>::read_ppu_controls(self, address, _read_only),
            0x4000..=0x4015 => {
                0 /*APU and Audio*/
            }
            0x4016 => {
                let data = self.controller1_snapshot & 0x80 >> 7;
                self.controller1_snapshot <<= 1;
                data
            }
            0x4017 => {
                let data = self.controller2_snapshot & 0x80 >> 7;
                self.controller2_snapshot <<= 1;
                data
            }
            0x4018..=0x401f => {
                0 /*APU and Audio*/
            }
            0x4020..=0xffff => self.cartridge.cpu_read(address),
        }
    }

    fn ppu_write(&mut self, address: u16, data: u8) {
        match address {
            0..=0x1FFF => self.cartridge.ppu_write(address, data),
            _ => {
                log::warn!("PPU write not implemented yet")
            }
        };
    }

    fn ppu_read(&self, address: u16, _read_only: bool) -> u8 {
        match address {
            0..=0x1FFF => self.cartridge.ppu_read(address),
            _ => {
                log::warn!("PPU write not implemented yet");
                0
            }
        }
    }
}

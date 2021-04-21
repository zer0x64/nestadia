mod cartridge;
mod cpu;
mod ppu;

use std::{collections::HashMap, ops::Deref};
use std::ops::DerefMut;

use log;

use cartridge::Cartridge;
pub use cpu::Cpu;
pub use ppu::Ppu;

const RAM_SIZE: u16 = 0x800;

#[derive(Debug)]
pub enum RomParserError {
    TooShort,
    InvalidMagicBytes,
    MapperNotImplemented,
}

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

pub struct Emulator {
    pub cpu: Cpu,
    pub ppu: Ppu,
    cartridge: Cartridge,
    ram: [u8; RAM_SIZE as usize],
    clock_count: u8,
}

impl Emulator {
    pub fn new(rom: &[u8]) -> Result<Self, RomParserError> {
        let mut emulator = Self {
            cpu: Cpu::new(),
            ppu: Ppu::new(),
            cartridge: Cartridge::new(rom)?,
            ram: [0u8; RAM_SIZE as usize],
            clock_count: 0,
        };

        EmulatorContext::<Cpu>::reset(&mut emulator);
        Ok(emulator)
    }

    pub fn clock(&mut self) {
        EmulatorContext::<Ppu>::clock(self);

        // CPU clock is 3 times slower
        if self.clock_count % 3 == 0 {
            EmulatorContext::<Cpu>::clock(self);
            self.clock_count = 0;
        }

        self.clock_count += 1;
    }

    pub fn reset(&mut self) {
        EmulatorContext::<Cpu>::reset(self);
        self.clock_count = 0;
    }

    pub fn disassemble(&self, start: u16, end: u16) -> Vec<(u16, String)> {
        self.cartridge.disassemble()
    }
}

impl BusInterface for Emulator {
    fn cpu_write(&mut self, address: u16, data: u8) {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize] = data,
            0x2000..=0x3fff => EmulatorContext::<Ppu>::write_ppu_controls(self, address & 0x07, data),
            0x4000..=0x401f => { /*APU and Audio*/ }
            0x4020..=0xffff => self.cartridge.cpu_write(address, data),
        };
    }

    fn cpu_read(&mut self, address: u16, _read_only: bool) -> u8 {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize],
            0x2000..=0x3fff => EmulatorContext::<Ppu>::read_ppu_controls(self, address & 0x07, _read_only),
            0x4000..=0x401f => {
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

#[test]
fn test() {
    flexi_logger::Logger::with_str("info").start().unwrap();

    let rom = include_bytes!("../../test_roms/Donkey Kong.nes");
    //let rom = include_bytes!("../test_roms/cpu_dummy_reads.nes");
    let mut emulator = Emulator::new(rom).unwrap();
    emulator.start();
}

mod cartridge;
mod cpu;

use std::{collections::HashMap, ops::Deref};
use std::ops::DerefMut;

use log;

use cartridge::Cartridge;
pub use cpu::Cpu;

const RAM_SIZE: u16 = 0x800;

#[derive(Debug)]
pub enum RomParserError {
    TooShort,
    InvalidMagicBytes,
    MapperNotImplemented,
}

pub trait BusInterface {
    fn cpu_write(&mut self, address: u16, data: u8);
    fn cpu_read(&self, address: u16, _read_only: bool) -> u8;
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

pub struct Emulator {
    pub cpu: Cpu,
    cartridge: Cartridge,
    ram: [u8; RAM_SIZE as usize],
}

impl Emulator {
    pub fn new(rom: &[u8]) -> Result<Self, RomParserError> {
        let mut emulator = Self {
            cpu: Cpu::new(),
            cartridge: Cartridge::new(rom)?,
            ram: [0u8; RAM_SIZE as usize],
        };

        EmulatorContext::<Cpu>::reset(&mut emulator);
        Ok(emulator)
    }

    pub fn start(&mut self) {
        EmulatorContext::<Cpu>::reset(self);

        loop {
            // Clock slowly to show what's happening. Only for testing now.
            use std::{thread, time};
            thread::sleep(time::Duration::from_millis(100));
            EmulatorContext::<Cpu>::clock(self);
        }
    }

    pub fn clock(&mut self) {
        EmulatorContext::<Cpu>::clock(self);
    }

    pub fn disassemble(&self, start: u16, end: u16) -> Vec<(u16, String)> {
        self.cartridge.disassemble()
    }
}

impl BusInterface for Emulator {
    fn cpu_write(&mut self, address: u16, data: u8) {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize] = data,
            0x2000..=0x3fff => { /*PPU*/ }
            0x4000..=0x401f => { /*APU and Audio*/ }
            0x4020..=0xffff => self.cartridge.cpu_write(address, data),
        };
    }

    fn cpu_read(&self, address: u16, _read_only: bool) -> u8 {
        match address {
            0..=0x1FFF => self.ram[(address & (RAM_SIZE - 1)) as usize],
            0x2000..=0x3fff => {
                0 /*PPU*/
            }
            0x4000..=0x401f => {
                0 /*APU and Audio*/
            }
            0x4020..=0xffff => self.cartridge.cpu_read(address),
        }
    }

    fn ppu_write(&mut self, address: u16, data: u8) {
        match address {
            _ => {
                log::warn!("PPU write not implemented yet")
            }
        };
    }

    fn ppu_read(&self, address: u16, _read_only: bool) -> u8 {
        match address {
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

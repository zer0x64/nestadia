#[macro_use]
mod bus;

mod cartridge;
mod cpu;
mod ppu;

pub use cpu::Cpu;
pub use ppu::Ppu;

use crate::cartridge::Cartridge;
use crate::ppu::PpuFrame;
use crate::cartridge::RomParserError;

pub const RAM_SIZE: u16 = 0x0800;

#[derive(Clone, Debug, Copy)]
pub enum ExecutionMode {
    Ring0,
    Ring3,
}

pub struct Emulator {
    // Cartridge is shared by CPU (prg) and PPU (chr)
    cartridge: Cartridge,

    // == CPU == //
    cpu: Cpu,
    controller1: u8,
    controller2: u8,
    controller1_snapshot: u8,
    controller2_snapshot: u8,
    ram: [u8; RAM_SIZE as usize],

    // == PPU == //
    ppu: Ppu,
    name_tables: [u8; 1024 * 2], // VRAM
    last_data_on_ppu_bus: u8,

    // Emulator internal state
    clock_count: u8,
}

impl Emulator {
    pub fn new(rom: &[u8], execution_mode: ExecutionMode) -> Result<Self, RomParserError> {
        let mut emulator = Self {
            cartridge: Cartridge::new(rom)?,

            cpu: Cpu::new(execution_mode),
            controller1: 0,
            controller2: 0,
            controller1_snapshot: 0,
            controller2_snapshot: 0,
            ram: [0u8; RAM_SIZE as usize],

            ppu: Ppu::new(),
            name_tables: [0u8; 1024 * 2],
            last_data_on_ppu_bus: 0,

            clock_count: 0,
        };

        emulator.reset();

        Ok(emulator)
    }

    pub fn clock(&mut self) -> Option<&PpuFrame> {
        // CPU clock is 3 times slower
        if self.clock_count % 3 == 0 {
            let mut cpu_bus = borrow_cpu_bus!(self);
            self.cpu.clock(&mut cpu_bus);
            self.clock_count = 0;
        }

        self.clock_count = self.clock_count.wrapping_add(1);

        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.clock(&mut ppu_bus)
    }

    pub fn set_controller1(&mut self, state: u8) {
        self.controller1 = state;
    }

    pub fn set_controller2(&mut self, state: u8) {
        self.controller2 = state;
    }

    pub fn reset(&mut self) {
        let mut cpu_bus = borrow_cpu_bus!(self);
        self.cpu.reset(&mut cpu_bus);
        // TODO: PPU reset?
        self.clock_count = 0;
    }

    #[cfg(feature = "debugger")]
    #[allow(unused_variables)] // FIXME
    pub fn disassemble(&self, start: u16, end: u16) -> Vec<(u16, String)> {
        self.cartridge.disassemble()
    }

    #[cfg(feature = "debugger")]
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
}


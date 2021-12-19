#![no_std]

extern crate alloc;

#[macro_use]
mod bus;

mod apu;
mod cartridge;
mod cpu;
mod ppu;
mod rgb_palette;

pub use rgb_palette::RGB_PALETTE;

pub use apu::Apu;
pub use cartridge::RomParserError;
pub use cpu::Cpu;
pub use ppu::Ppu;

use crate::cartridge::Cartridge;
use crate::ppu::PpuFrame;

pub const RAM_SIZE: u16 = 0x0800;

pub struct Emulator {
    // == APU == //
    apu: Apu,

    // Cartridge is shared by CPU (PRG) and PPU (CHR)
    cartridge: Cartridge,

    // == CPU == //
    cpu: Cpu,
    controller1: u8,
    controller2: u8,
    controller_state: bool,
    controller1_snapshot: u8,
    controller2_snapshot: u8,
    ram: [u8; RAM_SIZE as usize],

    // == PPU == //
    ppu: Ppu,
    name_tables: [u8; 1024 * 4], // VRAM

    // Emulator internal state
    clock_count: u8,
}

impl Emulator {
    pub fn new(rom: &[u8], save_data: Option<&[u8]>) -> Result<Self, RomParserError> {
        let mut emulator = Self {
            apu: Default::default(),

            cartridge: Cartridge::load(rom, save_data)?,

            cpu: Default::default(),
            controller1: 0,
            controller2: 0,
            controller_state: false,
            controller1_snapshot: 0,
            controller2_snapshot: 0,
            ram: [0u8; RAM_SIZE as usize],

            ppu: Ppu::new(),
            name_tables: [0u8; 1024 * 4],

            clock_count: 0,
        };

        emulator.reset();

        Ok(emulator)
    }

    pub fn clock(&mut self) -> Option<&PpuFrame> {
        // Make PPU clock first
        let mut ppu_bus = borrow_ppu_bus!(self);
        self.ppu.clock(&mut ppu_bus);

        // CPU clock is 3 times slower
        if self.clock_count % 3 == 0 {
            self.clock_count = 0;

            // TODO: Cleanup if current solution is working
            /*#[cfg(feature = "audio")]*/
            self.apu.clock();

            if self.cpu.cycles == 0 && self.ppu.take_vblank_nmi_set_state() {
                // NMI interrupt
                let mut cpu_bus = borrow_cpu_bus!(self);
                self.cpu.nmi(&mut cpu_bus);
                self.cpu.clock(&mut cpu_bus);
            } else if self.cpu.cycles == 0 && self.cartridge.take_irq_set_state() {
                // IRQ interrupt
                let mut cpu_bus = borrow_cpu_bus!(self);
                self.cpu.irq(&mut cpu_bus);
                self.cpu.clock(&mut cpu_bus);
            } else {
                let mut cpu_bus = borrow_cpu_bus!(self);
                self.cpu.clock(&mut cpu_bus);
            }
        }

        self.clock_count = self.clock_count.wrapping_add(1);

        // returns PPU frame if any
        self.ppu.ready_frame()
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
        self.apu.reset();
        self.ppu.reset();
        self.clock_count = 0;
    }

    pub fn get_save_data(&self) -> Option<&[u8]> {
        self.cartridge.get_save_data()
    }

    #[cfg(feature = "audio")]
    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.apu.set_sample_rate(sample_rate);
    }

    #[cfg(feature = "audio")]
    pub fn take_audio_samples(&mut self) -> alloc::vec::Drain<i16> {
        self.apu.take_samples()
    }

    #[cfg(feature = "debugger")]
    #[allow(unused_variables)] // FIXME
    pub fn disassemble(
        &self,
        start: u16,
        end: u16,
    ) -> alloc::vec::Vec<(Option<u8>, u16, alloc::string::String)> {
        crate::cpu::disassembler::disassemble(&self.cartridge, 0x4020)
    }

    #[cfg(feature = "debugger")]
    pub fn mem_dump(&mut self, start: u16, end: u16) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::new();

        for addr in start..=end {
            let mut bus = borrow_cpu_bus!(self);
            data.push(self.cpu.mem_dump(&mut bus, addr));
        }

        data
    }

    #[cfg(feature = "debugger")]
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
}

pub fn frame_to_rgb(frame: &PpuFrame, output: &mut [u8; 256 * 240 * 3]) {
    for i in 0..frame.len() {
        let f = RGB_PALETTE[(frame[i] & 0x3f) as usize];
        output[i * 3] = f[0];
        output[i * 3 + 1] = f[1];
        output[i * 3 + 2] = f[2];
    }
}

pub fn frame_to_rgba(frame: &PpuFrame, output: &mut [u8; 256 * 240 * 4]) {
    for i in 0..frame.len() {
        let f = RGB_PALETTE[(frame[i] & 0x3f) as usize];
        output[i * 4] = f[0]; // R
        output[i * 4 + 1] = f[1]; // G
        output[i * 4 + 2] = f[2]; // B

        // Alpha is always 0xff because it's opaque
        output[i * 4 + 3] = 0xff; // A
    }
}

pub fn frame_to_argb(frame: &PpuFrame, output: &mut [u8; 256 * 240 * 4]) {
    for i in 0..frame.len() {
        let f = RGB_PALETTE[(frame[i] & 0x3f) as usize];
        output[i * 4] = f[2]; // B
        output[i * 4 + 1] = f[1]; // G
        output[i * 4 + 2] = f[0]; // R

        // Alpha is always 0xff because it's opaque
        output[i * 4 + 3] = 0xff; // A
    }
}

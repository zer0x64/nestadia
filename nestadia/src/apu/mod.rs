use bitflags::bitflags;
use alloc::vec::{Drain, Vec};
use crate::apu::pulse::PulseChannel;

mod common;
mod noise;
mod pulse;
mod triangle;

bitflags! {
    struct ChannelEnable: u8 {
        const PULSE1_ENABLE = 0b00000001;
        const PULSE2_ENABLE = 0b00000010;
        const TRIANGLE_ENABLE = 0b00000100;
        const NOISE_ENABLE = 0b00001000;
        const DMC_ENABLE = 0b00010000;
    }
}

pub struct Apu {
    // Channels
    pulse_channel_1: pulse::PulseChannel,
    //pulse_channel_2: pulse::PulseChannel,

    // Frame counter
    disable_interrupts: bool,
    sequence_mode: common::SequenceMode,
    cycle_count: u16,

    // Sample
    samples: Vec<f32>,
}

impl Default for Apu {
    fn default() -> Self {
        Self::new()
    }
}

impl Apu {
    pub fn new() -> Self {
        Self {
            pulse_channel_1: PulseChannel::new(true),

            disable_interrupts: false,
            sequence_mode: Default::default(),
            cycle_count: 0,

            samples: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        *self = Default::default();
    }

    pub fn write(&mut self, bus: /*&mut PpuBus<'_>*/u8, addr: u16, data: u8) {
        match addr {
            0x4000..=0x4003 => {
                // pulse channel 1
                self.pulse_channel_1.write(addr & 0b11, data);
            }
            0x4004..=0x4007 => {
                // pulse channel 2
                //self.pulse_channel_2.write(addr & 0b11, data);
            }
            0x4008..=0x400B => {
                // triangle channel
            }
            0x400C..=0x400F => {
                // noise channel
            },
            0x4010..=0x4013 => {
                // dmc
            }
            0x4015 => {
                // channel enable and length counter status
                self.pulse_channel_1.set_length_counter_enable((data | ChannelEnable::PULSE1_ENABLE.bits()) != 0)
            }
            0x4017 => {
                // frame counter
                self.disable_interrupts = (data & 0x40) != 0;
                self.sequence_mode = if (data & 0x80) != 0 {
                    common::SequenceMode::Step5
                } else {
                    common::SequenceMode::Step4
                }
            }
            _ => {
                unreachable!("bad apu addr {:#X}", addr);
            }
        }
    }

    pub fn read(&mut self, bus: /*&mut PpuBus<'_>*/u8, addr: u16) -> u8 {
        match addr {
            0x4000..=0x4013 | 0x4017 => {
                log::warn!(
                    "Attempted to read write-only APU address: {:#X} (culprit at {})",
                    addr,
                    core::panic::Location::caller()
                );
                0
            }
            0x4015 => {
                // channel enable and length counter status
                let mut enable = ChannelEnable::empty();
                enable.set(ChannelEnable::PULSE1_ENABLE, self.pulse_channel_1.length_counter_enable());

                enable.bits()
            }
            _ => {
                unreachable!("bad apu addr {:#X}", addr);
            }
        }
    }

    pub fn clock(&mut self) {
        self.pulse_channel_1.clock(self.sequence_mode, self.cycle_count);
        //self.pulse_channel_2.clock(self.sequence_mode, self.cycle_count);

        self.mix_samples();
        self.cycle_count = (self.cycle_count + 1) % self.sequence_mode.get_max();
    }

    fn mix_samples(&mut self) {
        const SAMPLE_RATE: f32 = 44100.0;
        const CPU_FREQUENCY: f32 = 1789733.0;
        const CPU_CYCLES_PER_SAMPLE: u16 = ((CPU_FREQUENCY / SAMPLE_RATE) + 0.5) as u16;
        const MAX_SAMPLES: usize = 1024;

        if (self.cycle_count % CPU_CYCLES_PER_SAMPLE) == 0 {
            // Linear approximation mixing
            let pulse_out = 0.00752 * (self.pulse_channel_1.sample() + 0) as f32;
            let tnd_out = 0.00851 * 0.0 + 0.00494 * 0.0 + 0.00335 * 0.0;

            if self.samples.len() == MAX_SAMPLES {
                self.samples.pop();
            }

            self.samples.push(pulse_out + tnd_out);
            log::info!("new sample: {:?}", pulse_out + tnd_out);
        }
    }

    pub fn take_samples(&mut self) -> Drain<f32> {
        self.samples.drain(..)
    }
}

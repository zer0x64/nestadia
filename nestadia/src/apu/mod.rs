use alloc::vec::Vec;
use bitflags::bitflags;

mod common;
mod dac;
mod noise;
mod pulse;
mod triangle;

use self::common::SequenceMode;
use self::dac::Dac;
use self::noise::NoiseChannel;
use self::pulse::PulseChannel;
use self::triangle::TriangleChannel;

const PULSE_MIXING_TABLE: [f32; 31] = {
    let mut table = [0f32; 31];
    let mut i = 1;
    while i < 31 {
        table[i as usize] = 95.52 / (8128.0 / i as f32 + 100.0);
        i += 1;
    }
    table
};

const TND_MIXING_TABLE: [f32; 203] = {
    let mut table = [0f32; 203];
    let mut i = 1;
    while i < 203 {
        table[i as usize] = 163.67 / (24329.0 / i as f32 + 100.0);
        i += 1;
    }
    table
};

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
    pulse_channel_1: PulseChannel,
    pulse_channel_2: PulseChannel,
    triangle_channel: TriangleChannel,
    noise_channel: NoiseChannel,

    // Frame counter
    disable_interrupts: bool,
    sequence_mode: SequenceMode,
    frame_counter: u16,

    // Sampling
    dac: Dac,

    // IRQ
    frame_irq_set: bool,
    dmc_irq_set: bool,
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
            pulse_channel_2: PulseChannel::new(false),
            triangle_channel: Default::default(),
            noise_channel: Default::default(),

            disable_interrupts: false,
            sequence_mode: Default::default(),
            frame_counter: 0,

            dac: Default::default(),

            frame_irq_set: false,
            dmc_irq_set: false,
        }
    }

    pub fn reset(&mut self) {
        let sample_rate = self.dac.get_sample_rate();
        *self = Default::default();
        self.set_sample_rate(sample_rate);
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.dac = Dac::new(sample_rate);
    }

    pub fn take_irq_set_state(&mut self) -> bool {
        let state = self.frame_irq_set || self.dmc_irq_set;
        self.frame_irq_set = false;
        self.dmc_irq_set = false;
        state
    }

    #[cfg(feature = "audio")]
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000..=0x4003 => {
                // pulse channel 1
                self.pulse_channel_1.write(addr & 0b11, data);
            }
            0x4004..=0x4007 => {
                // pulse channel 2
                self.pulse_channel_2.write(addr & 0b11, data);
            }
            0x4008..=0x400B => {
                // triangle channel
                self.triangle_channel.write(addr & 0b11, data);
            }
            0x400C..=0x400F => {
                // noise channel
                self.noise_channel.write(addr & 0b11, data);
            }
            0x4010..=0x4013 => {
                // dmc
            }
            0x4015 => {
                // channel enable and length counter status
                self.pulse_channel_1
                    .set_length_counter_enable((data & ChannelEnable::PULSE1_ENABLE.bits()) != 0);
                self.pulse_channel_2
                    .set_length_counter_enable((data & ChannelEnable::PULSE2_ENABLE.bits()) != 0);
                self.triangle_channel
                    .set_length_counter_enable((data & ChannelEnable::TRIANGLE_ENABLE.bits()) != 0);
                self.noise_channel
                    .set_length_counter_enable((data & ChannelEnable::NOISE_ENABLE.bits()) != 0);
            }
            0x4017 => {
                // frame counter
                self.disable_interrupts = (data & 0x40) != 0;
                self.sequence_mode = if (data & 0x80) != 0 {
                    SequenceMode::Step5
                } else {
                    SequenceMode::Step4
                };

                // This should be reset 2-3 cycles after the write, but for now do it immediately
                self.frame_counter = 0;

                // When step mode is 5 steps, quarter and half frames are clocked immediately
                if self.sequence_mode == SequenceMode::Step5 {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
            }
            _ => {
                unreachable!("bad apu addr {:#X}", addr);
            }
        }
    }

    #[cfg(not(feature = "audio"))]
    pub fn write(&mut self, _addr: u16, _data: u8) {
        // DO NOTHING
    }

    #[cfg(feature = "audio")]
    pub fn read(&mut self, addr: u16) -> u8 {
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
                enable.set(
                    ChannelEnable::PULSE1_ENABLE,
                    self.pulse_channel_1.length_counter_active(),
                );
                enable.set(
                    ChannelEnable::PULSE2_ENABLE,
                    self.pulse_channel_2.length_counter_active(),
                );
                enable.set(
                    ChannelEnable::TRIANGLE_ENABLE,
                    self.triangle_channel.length_counter_active(),
                );
                enable.set(
                    ChannelEnable::NOISE_ENABLE,
                    self.noise_channel.length_counter_active(),
                );

                enable.bits()
            }
            _ => {
                unreachable!("bad apu addr {:#X}", addr);
            }
        }
    }

    #[cfg(not(feature = "audio"))]
    pub fn read(&mut self, _addr: u16) -> u8 {
        0
    }

    #[cfg(feature = "audio")]
    pub fn clock(&mut self) {
        // Pulse and noise channels run every second CPU cycle, while triangle runs every cycle
        self.triangle_channel.clock();
        if (self.frame_counter % 2) == 1 {
            self.pulse_channel_1.clock();
            self.pulse_channel_2.clock();
            self.noise_channel.clock();
        }

        if self.sequence_mode.is_quarter_frame(self.frame_counter) {
            self.clock_quarter_frame();
        }

        if self.sequence_mode.is_half_frame(self.frame_counter) {
            self.clock_half_frame();
        }

        self.dac.add_sample(self.mix_samples());
        self.frame_counter = (self.frame_counter + 1) % self.sequence_mode.get_max();
    }

    #[cfg(not(feature = "audio"))]
    pub fn clock(&mut self) {
        // DO NOTHING
    }

    #[cfg(feature = "audio")]
    fn clock_quarter_frame(&mut self) {
        self.pulse_channel_1.clock_quarter_frame();
        self.pulse_channel_2.clock_quarter_frame();
        self.triangle_channel.clock_quarter_frame();
        self.noise_channel.clock_quarter_frame();
    }

    #[cfg(feature = "audio")]
    fn clock_half_frame(&mut self) {
        self.pulse_channel_1.clock_half_frame();
        self.pulse_channel_2.clock_half_frame();
        self.triangle_channel.clock_half_frame();
        self.noise_channel.clock_half_frame();
    }

    #[cfg(feature = "audio")]
    fn mix_samples(&self) -> f32 {
        let pulse1 = self.pulse_channel_1.sample() * 1;
        let pulse2 = self.pulse_channel_2.sample() * 1;
        let triangle = self.triangle_channel.sample() * 1;
        let noise = self.noise_channel.sample() * 1;
        let dmc = 0;

        // Lookup table mixing
        let pulse_out = PULSE_MIXING_TABLE[(pulse1 + pulse2) as usize];
        let tnd_out = TND_MIXING_TABLE[(3 * triangle + 2 * noise + dmc) as usize];

        pulse_out + tnd_out
    }

    pub fn take_samples(&mut self) -> Vec<i16> {
        self.dac.take_samples()
    }
}

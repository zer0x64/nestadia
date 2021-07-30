use crate::apu::common::*;

const PERIOD_TABLE: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068
];

pub struct NoiseChannel {
    enveloppe: Enveloppe,
    timer: Timer,
    length_counter: LengthCounter,

    mode: bool,
    shift_register: u16,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        Self {
            enveloppe: Default::default(),
            timer: Default::default(),
            length_counter: Default::default(),

            mode: false,
            shift_register: 1,
        }
    }
}

impl NoiseChannel {
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr & 0b11 {
            0 => {
                self.enveloppe.set_register(data);
                self.length_counter.set_halt((data & 0x20) != 0);
            }
            1 => {
                // unused
            }
            2 => {
                self.timer.set_timer(PERIOD_TABLE[(data & 0x0F) as usize]);
                self.mode = (data & 0x80) != 0;
            }
            3 => {
                self.length_counter.set_counter(data >> 3);
                self.enveloppe.set_start_flag();
            }
            _ => {}
        }
    }

    pub fn clock(&mut self, sequence_mode: SequenceMode, cycle_count: u16) {
        // APU clock
        if (cycle_count % 2) == 1 {
            self.timer.clock();
            if self.timer.done() {
                let offset = if self.mode {
                    6
                } else {
                    1
                };

                let bit1 = self.shift_register & 0b1;
                let bit2 = (self.shift_register >> offset) & 0b1;

                self.shift_register = (self.shift_register >> 1) | ((bit1 ^ bit2) << 14);
            }
        }

        // Clock the linear and length counter
        if sequence_mode.is_quarter_frame(cycle_count) {
            self.enveloppe.clock();
        }

        if sequence_mode.is_half_frame(cycle_count) {
            self.length_counter.clock();
        }
    }

    pub fn length_counter_enable(&self) -> bool {
        self.length_counter.get_enable()
    }

    pub fn set_length_counter_enable(&mut self, enable: bool) {
        self.length_counter.set_enable(enable);
    }

    #[inline]
    pub fn sample(&self) -> u8 {
        // Check if muted
        if self.is_muted() {
            0
        } else {
            self.enveloppe.volume()
        }
    }

    #[inline]
    fn is_muted(&self) -> bool {
        self.shift_register & 0b1 == 1
            || self.length_counter.counter() == 0
    }
}

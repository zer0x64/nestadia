use crate::apu::common::*;
use bitfield::bitfield;

const SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

bitfield! {
    #[derive(Clone, Copy, Default)]
    struct LinearCounterRegister(u8);
    impl Debug;

    pub reload_value, set_reload_value: 6, 0;
    pub control, set_control: 7;
}

#[derive(Default)]
struct LinearCounter {
    register: LinearCounterRegister,
    counter: u8,
    reload: bool,
}

impl LinearCounter {
    pub fn set_register(&mut self, data: u8) {
        self.register.0 = data;
    }

    pub fn set_reload(&mut self) {
        self.reload = true;
    }

    pub fn counter(&self) -> u8 {
        self.counter
    }

    pub fn clock(&mut self) {
        if self.reload {
            self.counter = self.register.reload_value();
        } else if self.counter != 0 {
            self.counter -= 1;
        }

        if !self.register.control() {
            self.reload = false;
        }
    }
}

#[derive(Default)]
pub struct TriangleChannel {
    timer: Timer,
    length_counter: LengthCounter,
    linear_counter: LinearCounter,
    sequence_index: u8,
}

impl TriangleChannel {
    pub fn write(&mut self, addr: u16, data: u8) {
        match addr & 0b11 {
            0 => {
                self.linear_counter.set_register(data);
                self.length_counter.set_halt((data & 0x80) != 0);
            }
            1 => {
                // unused
            }
            2 => {
                self.timer.set_timer_lo(data);
            }
            3 => {
                self.timer.set_timer_hi(data & 0b111);
                self.length_counter.set_counter(data >> 3);

                self.linear_counter.set_reload();
            }
            _ => {}
        }
    }

    pub fn clock(&mut self, sequence_mode: SequenceMode, cycle_count: u16) {
        // The triangle channel runs every CPU clock
        self.timer.clock();
        if self.timer.done() && !self.is_muted() {
            self.sequence_index = (self.sequence_index + 1) % 32;
        }

        // Clock the linear and length counter subunits
        if sequence_mode.is_quarter_frame(cycle_count) {
            self.linear_counter.clock();
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

    pub fn sample(&self) -> u8 {
        SEQUENCE[self.sequence_index as usize]
    }

    fn is_muted(&self) -> bool {
        self.timer.counter() < 2
            || self.linear_counter.counter() == 0
            || self.length_counter.counter() == 0
    }
}

use crate::apu::common::*;
use bitfield::bitfield;

const DUTY_SEQUENCES: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 25% negated
];

bitfield! {
    #[derive(Clone, Copy, Default)]
    struct Sweep(u8);
    impl Debug;

    pub shift_count, set_shift_count: 2, 0;
    pub negate, set_negate: 3;
    pub period, set_period: 6, 4;
    pub enable, set_enable: 7;
}

#[derive(Default)]
pub struct PulseChannel {
    envelope: Envelope,
    sweep: Sweep,
    timer: Timer,
    length_counter: LengthCounter,

    one_complement: bool,

    duty_step: u8,
    sweep_counter: u8,
    sweep_reload: bool,
}

impl PulseChannel {
    pub fn new(one_complement: bool) -> Self {
        Self {
            one_complement,
            ..Default::default()
        }
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        match addr & 0b11 {
            0 => {
                self.envelope.set_register(data);
                self.length_counter.set_halt((data & 0x20) != 0);
            }
            1 => {
                self.sweep.0 = data;
                self.sweep_reload = true;
            }
            2 => {
                self.timer.set_timer_lo(data);
            }
            3 => {
                self.timer.set_timer_hi(data & 0b111);
                self.length_counter.set_counter(data >> 3);

                self.envelope.set_start_flag();
                self.duty_step = 0;
            }
            _ => {}
        }
    }

    pub fn clock(&mut self, sequence_mode: SequenceMode, cycle_count: u16) {
        // APU clock
        if (cycle_count % 2) == 1 {
            self.timer.clock();
            if self.timer.done() {
                if self.duty_step != 7 {
                    self.duty_step += 1;
                } else {
                    self.duty_step = 0;
                }
            }
        }

        // Clock envelope, sweep and length counter subunits
        if sequence_mode.is_quarter_frame(cycle_count) {
            self.envelope.clock();
        }

        if sequence_mode.is_half_frame(cycle_count) {
            self.length_counter.clock();

            if self.sweep_counter == 0
                && self.sweep.enable()
                && self.sweep.shift_count() > 0
                && self.timer.counter() >= 8
            {
                let target_period = self.target_period();
                if target_period <= 0x07FF {
                    self.timer.set_timer(target_period);
                }
            }

            if self.sweep_counter == 0 || self.sweep_reload {
                self.sweep_counter = self.sweep.period();
                self.sweep_reload = false;
            } else {
                self.sweep_counter -= 1;
            }
        }
    }

    pub fn length_counter_enable(&self) -> bool {
        self.length_counter.get_enable()
    }

    pub fn set_length_counter_enable(&mut self, enable: bool) {
        self.length_counter.set_enable(enable);
    }

    pub fn sample(&self) -> u8 {
        if self.is_muted() {
            0
        } else {
            self.envelope.volume()
                * DUTY_SEQUENCES[self.envelope.duty() as usize][self.duty_step as usize]
        }
    }

    fn is_muted(&self) -> bool {
        self.timer.counter() < 8
            || self.target_period() > 0x07FF
            || self.length_counter.counter() == 0
    }

    fn target_period(&self) -> u16 {
        let change = self.timer.period() >> self.sweep.shift_count();
        if self.sweep.negate() {
            if self.one_complement {
                self.timer.period().wrapping_sub(change).wrapping_sub(1)
            } else {
                self.timer.period().wrapping_sub(change)
            }
        } else {
            self.timer.period().wrapping_add(change)
        }
    }
}

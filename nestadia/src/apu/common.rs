use bitfield::bitfield;

bitfield! {
    #[derive(Clone, Copy)]
    struct EnveloppeRegister(u8);
    impl Debug;

    pub volume, _: 3, 0;
    pub const_volume, _: 4;
    pub enveloppe_loop, _: 5;
    pub duty, _: 7, 6;
}

impl Default for EnveloppeRegister {
    fn default() -> Self {
        Self(0)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Enveloppe {
    register: EnveloppeRegister,
    start_flag: bool,
    decay_cycle: u8,
    divider: u8,
}

impl Enveloppe {
    pub fn set_register(&mut self, data: u8) {
        self.register.0 = data;
    }

    pub fn set_start_flag(&mut self) {
        self.start_flag = true;
    }

    pub fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_cycle = 15;
            self.divider = self.register.volume();
        } else {
            if self.divider != 0 {
                self.divider -= 1;
            } else {
                self.divider = self.register.volume();

                if self.decay_cycle != 0 {
                    self.decay_cycle -= 1;
                } else {
                    if self.register.enveloppe_loop() {
                        self.decay_cycle = 15;
                    }
                }
            }
        }
    }

    pub fn duty(&self) -> u8 {
        self.register.duty()
    }

    pub fn volume(&self) -> u8 {
        if self.register.const_volume() {
            self.register.volume()
        } else {
            self.decay_cycle
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum SequenceMode {
    Step4,
    Step5
}

impl Default for SequenceMode {
    fn default() -> Self {
        Self::Step4
    }
}

impl SequenceMode {
    pub fn is_quarter_frame(&self, cycle: u16) -> bool {
        match cycle {
            7457 | 14913 | 22371 => true,
            29829 if *self == Self::Step4 => true,
            37281 if *self == Self::Step5 => true,
            _ => false
        }
    }

    pub fn is_half_frame(&self, cycle: u16) -> bool {
        match cycle {
            14913 => true,
            29829 if *self == Self::Step4 => true,
            37281 if *self == Self::Step5 => true,
            _ => false
        }
    }

    pub fn get_max(&self) -> u16 {
        match *self {
            Self::Step4 => 29830,
            Self::Step5 => 37282
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Timer {
    timer_reload: u16,
    counter: u16,
    reloaded: bool,
}

impl Timer {
    pub fn set_timer(&mut self, val: u16) {
        self.timer_reload = val & 0x07FF;
        self.counter = self.timer_reload;
    }

    pub fn set_timer_lo(&mut self, lo: u8) {
        self.timer_reload = (self.timer_reload & 0x0700) | lo as u16;
        self.counter = self.timer_reload;
    }

    pub fn set_timer_hi(&mut self, hi: u8) {
        self.timer_reload = (self.timer_reload & 0x00FF) | ((hi as u16 & 0x07) << 8);
        self.counter = self.timer_reload;
    }

    pub fn value(&self) -> u16 {
        self.counter
    }

    pub fn clock(&mut self) {
        if self.counter != 0 {
            self.counter -= 1;
            self.reloaded = false;
        } else {
            self.counter = self.timer_reload;
            self.reloaded = true;
        }
    }

    pub fn done(&self) -> bool {
        self.reloaded
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LengthCounter {
    counter: u8,
    halt: bool,
    enable: bool,
}

impl LengthCounter {
    pub fn counter(&self) -> u8 {
        self.counter
    }

    pub fn set_counter(&mut self, index: u8) {
        const LENGTH_COUNTER_TABLE: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
            12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30
        ];

        if self.enable {
            self.counter = LENGTH_COUNTER_TABLE[index as usize];
        }
    }

    pub fn set_halt(&mut self, halt: bool) {
        self.halt = halt;
    }

    pub fn get_enable(&self) -> bool {
        self.enable
    }

    pub fn set_enable(&mut self, enable: bool) {
        self.enable = enable;
        if !enable {
            self.counter = 0;
        }
    }

    pub fn clock(&mut self) {
        if self.counter > 0 && !self.halt {
            self.counter -= 1;
        }
    }
}

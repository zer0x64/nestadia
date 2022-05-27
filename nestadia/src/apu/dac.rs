use alloc::vec::Vec;

const MAX_SAMPLES: usize = 1024;
const CPU_FREQUENCY: f32 = 1789773.0;

pub struct Dac {
    sample_rate: f32,
    cpu_cycles_per_samples: [u16; 2],
    index: usize,

    sample_sum: f32,
    sample_count: u16,
    samples: Vec<i16>,
}

impl Default for Dac {
    fn default() -> Self {
        Self::new(44100.0)
    }
}

impl Dac {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            cpu_cycles_per_samples: [
                (CPU_FREQUENCY / sample_rate).floor() as u16,
                (CPU_FREQUENCY / sample_rate).ceil() as u16
            ],
            index: 0,

            sample_sum: 0.0,
            sample_count: 0,
            samples: Vec::with_capacity(MAX_SAMPLES),
        }
    }

    pub fn get_sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn take_samples(&mut self) -> Vec<i16> {
        let mut samples = Vec::with_capacity(MAX_SAMPLES);
        core::mem::swap(&mut self.samples, &mut samples);
        samples
    }

    pub fn add_sample(&mut self, sample: f32) {
        self.sample_sum += sample;
        self.sample_count += 1;

        if self.sample_count == self.cpu_cycles_per_samples[self.index] {
            self.index = (self.index + 1) % 2;

            let sample = self.downsample();
            self.samples.push(sample);
        }
    }

    fn downsample(&mut self) -> i16 {
        let average = self.sample_sum / self.sample_count as f32;

        self.sample_sum = 0.0;
        self.sample_count = 0;

        // Remap to i16
        (average * i16::MAX as f32) as i16
    }
}

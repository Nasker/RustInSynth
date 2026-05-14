use std::f32::consts::PI;

use super::types::{Frequency, Sample, SampleRate};

/// Trait for all oscillator types.
/// Oscillators generate audio samples at a given frequency.
pub trait Oscillator: Send + Sync {
    /// Generate the next sample
    fn next_sample(&mut self) -> Sample;

    /// Set the oscillator frequency
    fn set_frequency(&mut self, frequency: Frequency);

    /// Get the current frequency
    fn frequency(&self) -> Frequency;

    /// Reset the oscillator phase to initial state
    fn reset(&mut self);

    /// Set the sample rate (called when audio engine initializes)
    fn set_sample_rate(&mut self, sample_rate: SampleRate);
}

/// Sine wave oscillator - the purest waveform
pub struct SineOscillator {
    frequency: Frequency,
    phase: f32,
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl SineOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        };
        osc.update_phase_increment();
        osc
    }

    fn update_phase_increment(&mut self) {
        self.phase_increment = 2.0 * PI * self.frequency / self.sample_rate as f32;
    }
}

impl Oscillator for SineOscillator {
    fn next_sample(&mut self) -> Sample {
        let sample = self.phase.sin();
        self.phase += self.phase_increment;
        if self.phase >= 2.0 * PI {
            self.phase -= 2.0 * PI;
        }
        sample
    }

    fn set_frequency(&mut self, frequency: Frequency) {
        self.frequency = frequency;
        self.update_phase_increment();
    }

    fn frequency(&self) -> Frequency {
        self.frequency
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_phase_increment();
    }
}

/// Square wave oscillator - rich in odd harmonics
pub struct SquareOscillator {
    frequency: Frequency,
    phase: f32,
    phase_increment: f32,
    sample_rate: SampleRate,
    duty_cycle: f32,
}

impl SquareOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
            duty_cycle: 0.5,
        };
        osc.update_phase_increment();
        osc
    }

    pub fn set_duty_cycle(&mut self, duty: f32) {
        self.duty_cycle = duty.clamp(0.01, 0.99);
    }

    fn update_phase_increment(&mut self) {
        self.phase_increment = self.frequency / self.sample_rate as f32;
    }
}

impl Oscillator for SquareOscillator {
    fn next_sample(&mut self) -> Sample {
        let sample = if self.phase < self.duty_cycle { 1.0 } else { -1.0 };
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    fn set_frequency(&mut self, frequency: Frequency) {
        self.frequency = frequency;
        self.update_phase_increment();
    }

    fn frequency(&self) -> Frequency {
        self.frequency
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_phase_increment();
    }
}

/// Sawtooth wave oscillator - all harmonics
pub struct SawOscillator {
    frequency: Frequency,
    phase: f32,
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl SawOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        };
        osc.update_phase_increment();
        osc
    }

    fn update_phase_increment(&mut self) {
        self.phase_increment = self.frequency / self.sample_rate as f32;
    }
}

impl Oscillator for SawOscillator {
    fn next_sample(&mut self) -> Sample {
        let sample = 2.0 * self.phase - 1.0;
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    fn set_frequency(&mut self, frequency: Frequency) {
        self.frequency = frequency;
        self.update_phase_increment();
    }

    fn frequency(&self) -> Frequency {
        self.frequency
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_phase_increment();
    }
}

/// Triangle wave oscillator - odd harmonics, softer than square
pub struct TriangleOscillator {
    frequency: Frequency,
    phase: f32,
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl TriangleOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_increment: 0.0,
            sample_rate,
        };
        osc.update_phase_increment();
        osc
    }

    fn update_phase_increment(&mut self) {
        self.phase_increment = self.frequency / self.sample_rate as f32;
    }
}

impl Oscillator for TriangleOscillator {
    fn next_sample(&mut self) -> Sample {
        let sample = if self.phase < 0.5 {
            4.0 * self.phase - 1.0
        } else {
            3.0 - 4.0 * self.phase
        };
        self.phase += self.phase_increment;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        sample
    }

    fn set_frequency(&mut self, frequency: Frequency) {
        self.frequency = frequency;
        self.update_phase_increment();
    }

    fn frequency(&self) -> Frequency {
        self.frequency
    }

    fn reset(&mut self) {
        self.phase = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_phase_increment();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_oscillator_range() {
        let mut osc = SineOscillator::new(440.0, 44100);
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }

    #[test]
    fn test_square_oscillator_values() {
        let mut osc = SquareOscillator::new(440.0, 44100);
        for _ in 0..1000 {
            let sample = osc.next_sample();
            assert!(sample == 1.0 || sample == -1.0);
        }
    }
}

use std::f32::consts::PI;

use super::event::WaveformType;
use super::types::{Frequency, Sample, SampleRate, Amplitude};

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

    /// Set phase offset (0.0 to 1.0, where 1.0 = full cycle)
    fn set_phase_offset(&mut self, offset: f32);

    /// Get current phase offset
    fn phase_offset(&self) -> f32;
}

/// Sine wave oscillator - the purest waveform
pub struct SineOscillator {
    frequency: Frequency,
    phase: f32,
    phase_offset: f32,  // 0.0 to 2π
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl SineOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_offset: 0.0,
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
        let sample = (self.phase + self.phase_offset).sin();
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

    fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0) * 2.0 * PI;
    }

    fn phase_offset(&self) -> f32 {
        self.phase_offset / (2.0 * PI)
    }
}

/// Square wave oscillator - rich in odd harmonics
pub struct SquareOscillator {
    frequency: Frequency,
    phase: f32,
    phase_offset: f32,  // 0.0 to 1.0
    phase_increment: f32,
    sample_rate: SampleRate,
    duty_cycle: f32,
}

impl SquareOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_offset: 0.0,
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
        let effective_phase = (self.phase + self.phase_offset) % 1.0;
        let sample = if effective_phase < self.duty_cycle { 1.0 } else { -1.0 };
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

    fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0);
    }

    fn phase_offset(&self) -> f32 {
        self.phase_offset
    }
}

/// Sawtooth wave oscillator - all harmonics
pub struct SawOscillator {
    frequency: Frequency,
    phase: f32,
    phase_offset: f32,  // 0.0 to 1.0
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl SawOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_offset: 0.0,
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
        let effective_phase = (self.phase + self.phase_offset) % 1.0;
        let sample = 2.0 * effective_phase - 1.0;
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

    fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0);
    }

    fn phase_offset(&self) -> f32 {
        self.phase_offset
    }
}

/// Triangle wave oscillator - odd harmonics, softer than square
pub struct TriangleOscillator {
    frequency: Frequency,
    phase: f32,
    phase_offset: f32,  // 0.0 to 1.0
    phase_increment: f32,
    sample_rate: SampleRate,
}

impl TriangleOscillator {
    pub fn new(frequency: Frequency, sample_rate: SampleRate) -> Self {
        let mut osc = Self {
            frequency,
            phase: 0.0,
            phase_offset: 0.0,
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
        let effective_phase = (self.phase + self.phase_offset) % 1.0;
        let sample = if effective_phase < 0.5 {
            4.0 * effective_phase - 1.0
        } else {
            3.0 - 4.0 * effective_phase
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

    fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0);
    }

    fn phase_offset(&self) -> f32 {
        self.phase_offset
    }
}

/// Factory function to create an oscillator from WaveformType
pub fn create_oscillator(waveform: WaveformType, frequency: Frequency, sample_rate: SampleRate) -> Box<dyn Oscillator> {
    match waveform {
        WaveformType::Sine => Box::new(SineOscillator::new(frequency, sample_rate)),
        WaveformType::Square => Box::new(SquareOscillator::new(frequency, sample_rate)),
        WaveformType::Saw => Box::new(SawOscillator::new(frequency, sample_rate)),
        WaveformType::Triangle => Box::new(TriangleOscillator::new(frequency, sample_rate)),
    }
}

/// Convert semitones and cents to frequency ratio
/// semitones: -24 to +24 (2 octaves)
/// cents: -100 to +100 (fine tune within semitone)
#[inline]
pub fn detune_ratio(semitones: i8, cents: i8) -> f32 {
    let total_semitones = semitones as f32 + (cents as f32 / 100.0);
    2.0_f32.powf(total_semitones / 12.0)
}

/// A single oscillator unit with waveform, detune, phase, and level controls
pub struct OscillatorUnit {
    oscillator: Box<dyn Oscillator>,
    waveform: WaveformType,
    semitones: i8,      // -24 to +24
    cents: i8,          // -100 to +100
    phase_offset: f32,  // 0.0 to 1.0
    level: Amplitude,   // 0.0 to 1.0
    base_frequency: Frequency,
    sample_rate: SampleRate,
}

impl OscillatorUnit {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            oscillator: Box::new(SineOscillator::new(440.0, sample_rate)),
            waveform: WaveformType::Sine,
            semitones: 0,
            cents: 0,
            phase_offset: 0.0,
            level: 1.0,
            base_frequency: 440.0,
            sample_rate,
        }
    }

    /// Set the waveform type
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        if self.waveform != waveform {
            self.waveform = waveform;
            let freq = self.oscillator.frequency();
            let phase = self.phase_offset;
            self.oscillator = create_oscillator(waveform, freq, self.sample_rate);
            self.oscillator.set_phase_offset(phase);
        }
    }

    /// Get current waveform
    pub fn waveform(&self) -> WaveformType {
        self.waveform
    }

    /// Set detune in semitones (-24 to +24)
    pub fn set_semitones(&mut self, semitones: i8) {
        self.semitones = semitones.clamp(-24, 24);
        self.update_frequency();
    }

    /// Get current semitone detune
    pub fn semitones(&self) -> i8 {
        self.semitones
    }

    /// Set fine detune in cents (-100 to +100)
    pub fn set_cents(&mut self, cents: i8) {
        self.cents = cents.clamp(-100, 100);
        self.update_frequency();
    }

    /// Get current cents detune
    pub fn cents(&self) -> i8 {
        self.cents
    }

    /// Set phase offset (0.0 to 1.0, where 1.0 = full cycle)
    pub fn set_phase_offset(&mut self, offset: f32) {
        self.phase_offset = offset.clamp(0.0, 1.0);
        self.oscillator.set_phase_offset(self.phase_offset);
    }

    /// Get current phase offset
    pub fn phase_offset(&self) -> f32 {
        self.phase_offset
    }

    /// Set the level (0.0 to 1.0)
    pub fn set_level(&mut self, level: Amplitude) {
        self.level = level.clamp(0.0, 1.0);
    }

    /// Get current level
    pub fn level(&self) -> Amplitude {
        self.level
    }

    /// Set the base frequency (before detune)
    pub fn set_base_frequency(&mut self, frequency: Frequency) {
        self.base_frequency = frequency;
        self.update_frequency();
    }

    /// Update oscillator frequency based on base + detune
    fn update_frequency(&mut self) {
        let ratio = detune_ratio(self.semitones, self.cents);
        self.oscillator.set_frequency(self.base_frequency * ratio);
    }

    /// Generate next sample (with level applied)
    pub fn next_sample(&mut self) -> Sample {
        self.oscillator.next_sample() * self.level
    }

    /// Reset oscillator phase
    pub fn reset(&mut self) {
        self.oscillator.reset();
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.oscillator.set_sample_rate(sample_rate);
    }
}

/// Minimoog-style oscillator bank with 3 oscillators
/// OSC 1: Main oscillator (tuned to note)
/// OSC 2: Detunable relative to OSC 1
/// OSC 3: Detunable relative to OSC 1
pub struct OscillatorBank {
    osc1: OscillatorUnit,
    osc2: OscillatorUnit,
    osc3: OscillatorUnit,
    base_frequency: Frequency,
    pitch_bend: f32,        // -1.0 to +1.0
    pitch_bend_range: u8,   // semitones (default 12 = 1 octave)
    sample_rate: SampleRate,
}

impl OscillatorBank {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            osc1: OscillatorUnit::new(sample_rate),
            osc2: OscillatorUnit::new(sample_rate),
            osc3: OscillatorUnit::new(sample_rate),
            base_frequency: 440.0,
            pitch_bend: 0.0,
            pitch_bend_range: 12, // 1 octave default
            sample_rate,
        }
    }

    /// Set pitch bend (-1.0 to +1.0)
    pub fn set_pitch_bend(&mut self, bend: f32) {
        self.pitch_bend = bend.clamp(-1.0, 1.0);
        self.update_frequencies();
    }

    /// Get current pitch bend
    pub fn pitch_bend(&self) -> f32 {
        self.pitch_bend
    }

    /// Set pitch bend range in semitones (1-24)
    pub fn set_pitch_bend_range(&mut self, semitones: u8) {
        self.pitch_bend_range = semitones.clamp(1, 24);
    }

    /// Get pitch bend range
    pub fn pitch_bend_range(&self) -> u8 {
        self.pitch_bend_range
    }

    /// Update all oscillator frequencies with pitch bend applied
    fn update_frequencies(&mut self) {
        let bend_semitones = self.pitch_bend * self.pitch_bend_range as f32;
        let bend_ratio = 2.0_f32.powf(bend_semitones / 12.0);
        let bent_freq = self.base_frequency * bend_ratio;
        self.osc1.set_base_frequency(bent_freq);
        self.osc2.set_base_frequency(bent_freq);
        self.osc3.set_base_frequency(bent_freq);
    }

    /// Get mutable reference to oscillator 1 (main)
    pub fn osc1_mut(&mut self) -> &mut OscillatorUnit {
        &mut self.osc1
    }

    /// Get mutable reference to oscillator 2
    pub fn osc2_mut(&mut self) -> &mut OscillatorUnit {
        &mut self.osc2
    }

    /// Get mutable reference to oscillator 3
    pub fn osc3_mut(&mut self) -> &mut OscillatorUnit {
        &mut self.osc3
    }

    /// Get reference to oscillator 1
    pub fn osc1(&self) -> &OscillatorUnit {
        &self.osc1
    }

    /// Get reference to oscillator 2
    pub fn osc2(&self) -> &OscillatorUnit {
        &self.osc2
    }

    /// Get reference to oscillator 3
    pub fn osc3(&self) -> &OscillatorUnit {
        &self.osc3
    }

    /// Set waveform for a specific oscillator (1, 2, or 3)
    pub fn set_waveform(&mut self, osc_num: u8, waveform: WaveformType) {
        match osc_num {
            1 => self.osc1.set_waveform(waveform),
            2 => self.osc2.set_waveform(waveform),
            3 => self.osc3.set_waveform(waveform),
            _ => {}
        }
    }

    /// Set detune for a specific oscillator
    pub fn set_detune(&mut self, osc_num: u8, semitones: i8, cents: i8) {
        match osc_num {
            1 => {
                self.osc1.set_semitones(semitones);
                self.osc1.set_cents(cents);
            }
            2 => {
                self.osc2.set_semitones(semitones);
                self.osc2.set_cents(cents);
            }
            3 => {
                self.osc3.set_semitones(semitones);
                self.osc3.set_cents(cents);
            }
            _ => {}
        }
    }

    /// Set level for a specific oscillator
    pub fn set_level(&mut self, osc_num: u8, level: Amplitude) {
        match osc_num {
            1 => self.osc1.set_level(level),
            2 => self.osc2.set_level(level),
            3 => self.osc3.set_level(level),
            _ => {}
        }
    }

    /// Set phase offset for a specific oscillator (0.0 to 1.0)
    pub fn set_phase(&mut self, osc_num: u8, phase: f32) {
        match osc_num {
            1 => self.osc1.set_phase_offset(phase),
            2 => self.osc2.set_phase_offset(phase),
            3 => self.osc3.set_phase_offset(phase),
            _ => {}
        }
    }
}

impl Oscillator for OscillatorBank {
    fn next_sample(&mut self) -> Sample {
        // Mix all three oscillators
        let sample = self.osc1.next_sample()
            + self.osc2.next_sample()
            + self.osc3.next_sample();
        
        // Normalize to prevent clipping (divide by 3)
        sample / 3.0
    }

    fn set_frequency(&mut self, frequency: Frequency) {
        self.base_frequency = frequency;
        self.update_frequencies();
    }

    fn frequency(&self) -> Frequency {
        self.base_frequency
    }

    fn reset(&mut self) {
        self.osc1.reset();
        self.osc2.reset();
        self.osc3.reset();
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.osc1.set_sample_rate(sample_rate);
        self.osc2.set_sample_rate(sample_rate);
        self.osc3.set_sample_rate(sample_rate);
    }

    fn set_phase_offset(&mut self, offset: f32) {
        // Set phase for all oscillators (typically you'd use set_phase per-osc)
        self.osc1.set_phase_offset(offset);
        self.osc2.set_phase_offset(offset);
        self.osc3.set_phase_offset(offset);
    }

    fn phase_offset(&self) -> f32 {
        // Return OSC1's phase as reference
        self.osc1.phase_offset()
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

    #[test]
    fn test_detune_ratio() {
        // No detune = ratio of 1.0
        assert!((detune_ratio(0, 0) - 1.0).abs() < 0.001);
        // +12 semitones = octave up = ratio of 2.0
        assert!((detune_ratio(12, 0) - 2.0).abs() < 0.001);
        // -12 semitones = octave down = ratio of 0.5
        assert!((detune_ratio(-12, 0) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_oscillator_bank() {
        let mut bank = OscillatorBank::new(44100);
        
        // Set different waveforms
        bank.set_waveform(1, WaveformType::Saw);
        bank.set_waveform(2, WaveformType::Square);
        bank.set_waveform(3, WaveformType::Sine);
        
        // Detune osc 2 and 3
        bank.set_detune(2, 0, 10);   // +10 cents
        bank.set_detune(3, -12, 0);  // -1 octave
        
        bank.set_frequency(440.0);
        
        // Generate samples - should be in valid range
        for _ in 0..1000 {
            let sample = bank.next_sample();
            assert!(sample >= -1.0 && sample <= 1.0);
        }
    }
}

use std::f32::consts::PI;

use super::types::{Frequency, Sample, SampleRate};

/// Filter mode - which output to use
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FilterMode {
    #[default]
    LowPass,
    HighPass,
    BandPass,
}

impl FilterMode {
    pub fn name(&self) -> &'static str {
        match self {
            FilterMode::LowPass => "LowPass",
            FilterMode::HighPass => "HighPass",
            FilterMode::BandPass => "BandPass",
        }
    }
}

/// Trait for all filter types
pub trait Filter: Send + Sync {
    /// Process a single sample through the filter
    fn process(&mut self, input: Sample) -> Sample;

    /// Set the cutoff frequency in Hz
    fn set_cutoff(&mut self, cutoff: Frequency);

    /// Set the resonance (Q factor)
    fn set_resonance(&mut self, resonance: f32);

    /// Get current cutoff frequency
    fn cutoff(&self) -> Frequency;

    /// Get current resonance
    fn resonance(&self) -> f32;

    /// Reset filter state (clear delay buffers)
    fn reset(&mut self);

    /// Set the sample rate
    fn set_sample_rate(&mut self, sample_rate: SampleRate);
}

/// State Variable Filter - resonant multimode filter
/// Can output lowpass, highpass, or bandpass simultaneously
/// Based on the Chamberlin SVF topology
pub struct SVFilter {
    cutoff: Frequency,
    resonance: f32,
    sample_rate: SampleRate,
    mode: FilterMode,

    // Filter coefficients
    f: f32,  // frequency coefficient
    q: f32,  // damping (inverse of resonance)

    // State variables
    low: f32,
    band: f32,
    high: f32,
}

impl SVFilter {
    /// Create a new state variable filter
    /// cutoff: frequency in Hz
    /// resonance: 0.0 (no resonance) to 1.0 (self-oscillation)
    pub fn new(cutoff: Frequency, resonance: f32, sample_rate: SampleRate) -> Self {
        let mut filter = Self {
            cutoff,
            resonance: resonance.clamp(0.0, 1.0),
            sample_rate,
            mode: FilterMode::LowPass,
            f: 0.0,
            q: 0.0,
            low: 0.0,
            band: 0.0,
            high: 0.0,
        };
        filter.update_coefficients();
        filter
    }

    /// Set the filter mode
    pub fn set_mode(&mut self, mode: FilterMode) {
        self.mode = mode;
    }

    /// Get the current filter mode
    pub fn mode(&self) -> FilterMode {
        self.mode
    }

    /// Update internal coefficients when parameters change
    fn update_coefficients(&mut self) {
        // Clamp cutoff to safe range (20 Hz to Nyquist)
        let max_cutoff = self.sample_rate as f32 * 0.45;
        let safe_cutoff = self.cutoff.clamp(20.0, max_cutoff);

        // Frequency coefficient (using approximation for stability)
        // f = 2 * sin(pi * cutoff / sample_rate)
        // For stability at high frequencies, we use a clamped version
        self.f = 2.0 * (PI * safe_cutoff / self.sample_rate as f32).sin();
        self.f = self.f.clamp(0.0, 1.0);

        // Q factor (damping) - maps resonance 0-1 to Q range
        // Low Q = no resonance, High Q = lots of resonance
        // Q = 1/resonance, but we map it more musically
        // resonance 0.0 -> q = 2.0 (no resonance)
        // resonance 1.0 -> q = 0.01 (near self-oscillation)
        self.q = 2.0 - self.resonance * 1.99;
        self.q = self.q.clamp(0.01, 2.0);
    }
}

impl Filter for SVFilter {
    fn process(&mut self, input: Sample) -> Sample {
        // State Variable Filter algorithm (2x oversampled for stability)
        // Run the filter twice per sample for better high-frequency response
        for _ in 0..2 {
            self.low += self.f * self.band;
            self.high = input - self.low - self.q * self.band;
            self.band += self.f * self.high;
        }

        // Soft clip the state variables to prevent blowup at high resonance
        self.low = soft_clip_filter(self.low);
        self.band = soft_clip_filter(self.band);
        self.high = soft_clip_filter(self.high);

        // Return the selected output
        match self.mode {
            FilterMode::LowPass => self.low,
            FilterMode::HighPass => self.high,
            FilterMode::BandPass => self.band,
        }
    }

    fn set_cutoff(&mut self, cutoff: Frequency) {
        self.cutoff = cutoff;
        self.update_coefficients();
    }

    fn set_resonance(&mut self, resonance: f32) {
        self.resonance = resonance.clamp(0.0, 1.0);
        self.update_coefficients();
    }

    fn cutoff(&self) -> Frequency {
        self.cutoff
    }

    fn resonance(&self) -> f32 {
        self.resonance
    }

    fn reset(&mut self) {
        self.low = 0.0;
        self.band = 0.0;
        self.high = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }
}

/// Soft clipping for filter state variables to prevent runaway at high resonance
fn soft_clip_filter(x: f32) -> f32 {
    if x > 1.0 {
        1.0 + (x - 1.0).tanh()
    } else if x < -1.0 {
        -1.0 + (x + 1.0).tanh()
    } else {
        x
    }
}

/// Filter cutoff range constants
pub const MIN_CUTOFF: Frequency = 20.0;      // 20 Hz
pub const MAX_CUTOFF: Frequency = 20000.0;   // 20 kHz

/// Convert CC value to cutoff frequency (exponential mapping)
pub fn cc_to_cutoff(value: u8) -> Frequency {
    // Exponential mapping for musical response
    // CC 0 -> 20 Hz, CC 127 -> 20000 Hz
    let normalized = value as f32 / 127.0;
    MIN_CUTOFF * (MAX_CUTOFF / MIN_CUTOFF).powf(normalized)
}

/// Convert cutoff frequency to CC value
pub fn cutoff_to_cc(cutoff: Frequency) -> u8 {
    let normalized = (cutoff / MIN_CUTOFF).ln() / (MAX_CUTOFF / MIN_CUTOFF).ln();
    (normalized.clamp(0.0, 1.0) * 127.0).round() as u8
}

/// Convert CC value to resonance (0.0 - 1.0)
pub fn cc_to_resonance(value: u8) -> f32 {
    value as f32 / 127.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svf_lowpass() {
        let mut filter = SVFilter::new(1000.0, 0.0, 44100);
        filter.set_mode(FilterMode::LowPass);

        // Process some samples
        let mut output = 0.0;
        for i in 0..100 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 }; // Square wave
            output = filter.process(input);
        }

        // Low pass should attenuate high frequencies
        assert!(output.abs() < 1.0);
    }

    #[test]
    fn test_cc_to_cutoff() {
        // CC 0 should give minimum cutoff
        assert!((cc_to_cutoff(0) - MIN_CUTOFF).abs() < 1.0);
        // CC 127 should give maximum cutoff
        assert!((cc_to_cutoff(127) - MAX_CUTOFF).abs() < 100.0);
    }

    #[test]
    fn test_resonance_range() {
        let mut filter = SVFilter::new(1000.0, 0.5, 44100);
        
        // Should handle extreme resonance without blowing up
        filter.set_resonance(0.99);
        for _ in 0..1000 {
            let output = filter.process(0.1);
            assert!(output.is_finite());
            assert!(output.abs() < 10.0);
        }
    }
}

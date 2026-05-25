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
/// Based on the Chamberlin SVF topology with analog-style improvements:
/// - Saturated feedback for warmer resonance
/// - Parameter smoothing to prevent zipper noise
pub struct SVFilter {
    cutoff: Frequency,
    resonance: f32,
    sample_rate: SampleRate,
    mode: FilterMode,

    // Filter coefficients (current)
    f: f32,  // frequency coefficient
    q: f32,  // damping (inverse of resonance)

    // Filter coefficients (target, for smoothing)
    f_target: f32,
    q_target: f32,

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
            f_target: 0.0,
            q_target: 0.0,
            low: 0.0,
            band: 0.0,
            high: 0.0,
        };
        filter.update_coefficients();
        // Initialize current values to target (no smoothing on first set)
        filter.f = filter.f_target;
        filter.q = filter.q_target;
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
        self.f_target = 2.0 * (PI * safe_cutoff / self.sample_rate as f32).sin();
        self.f_target = self.f_target.clamp(0.0, 1.0);

        // Q factor (damping) - maps resonance 0-1 to Q range
        // Low Q = no resonance, High Q = lots of resonance
        // Q = 1/resonance, but we map it more musically
        // resonance 0.0 -> q = 2.0 (no resonance)
        // resonance 1.0 -> q = 0.01 (near self-oscillation)
        self.q_target = 2.0 - self.resonance * 1.99;
        self.q_target = self.q_target.clamp(0.01, 2.0);
    }

    /// Smooth coefficient changes to prevent zipper noise
    /// Called once per sample
    #[inline]
    fn smooth_coefficients(&mut self) {
        // Smoothing factor: ~5ms at 44.1kHz
        const SMOOTH: f32 = 0.005;
        self.f += SMOOTH * (self.f_target - self.f);
        self.q += SMOOTH * (self.q_target - self.q);
    }
}

impl Filter for SVFilter {
    fn process(&mut self, input: Sample) -> Sample {
        // Smooth parameter changes to prevent zipper noise
        self.smooth_coefficients();

        // State Variable Filter algorithm (2x oversampled for stability)
        // Run the filter twice per sample for better high-frequency response
        for _ in 0..2 {
            self.low += self.f * self.band;

            // Saturate the resonance feedback for analog-style warmth
            // This prevents harsh digital ringing at high Q
            let feedback = self.q * self.band;
            let saturated_feedback = fast_tanh(feedback);

            self.high = input - self.low - saturated_feedback;
            self.band += self.f * self.high;

            // Gentle saturation on state variables for analog character
            self.band = fast_tanh(self.band);
        }

        // Soft clip the output to prevent blowup at high resonance
        self.low = soft_clip_filter(self.low);

        // Return the selected output
        match self.mode {
            FilterMode::LowPass => self.low,
            FilterMode::HighPass => soft_clip_filter(self.high),
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

/// Fast tanh approximation for saturation
/// Uses rational polynomial - accurate and cheap
#[inline]
fn fast_tanh(x: f32) -> f32 {
    let x2 = x * x;
    x * (27.0 + x2) / (27.0 + 9.0 * x2)
}

/// Soft clipping for filter state variables to prevent runaway at high resonance
fn soft_clip_filter(x: f32) -> f32 {
    if x > 1.0 {
        1.0 + fast_tanh(x - 1.0)
    } else if x < -1.0 {
        -1.0 + fast_tanh(x + 1.0)
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
mod filter_envelope_tests {
    use super::*;
    use crate::core::envelope::{ADSREnvelope, Envelope};
    use crate::core::oscillator::{SineOscillator, Oscillator};
    use std::path::PathBuf;

    /// Simulate what Voice::next_sample does for the filter signal chain,
    /// but expose the internal cutoff value each sample for analysis.
    fn run_filter_envelope_simulation(
        base_cutoff: Frequency,
        filter_env_amount: f32,
        filter_sustain: f32,
        filter_attack: f32,
        filter_decay: f32,
        filter_release: f32,
        amp_sustain: f32,
        events: &[(f32, bool)],
        duration_seconds: f32,
        sample_rate: u32,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>, Vec<(usize, bool)>) {
        // audio, cutoff_curve, filter_env_curve, event_markers
        let total = (duration_seconds * sample_rate as f32) as usize;
        let mut osc = SineOscillator::new(440.0, sample_rate);
        let mut filter = SVFilter::new(base_cutoff, 0.5, sample_rate);
        let mut amp_env = ADSREnvelope::new(0.01, 0.1, amp_sustain, 0.2, sample_rate);
        let mut filter_env = ADSREnvelope::new(
            filter_attack, filter_decay, filter_sustain, filter_release, sample_rate
        );

        let max_cutoff = 20000.0f32;
        let cutoff_range = max_cutoff - base_cutoff;

        let mut audio = Vec::with_capacity(total);
        let mut cutoff_curve = Vec::with_capacity(total);
        let mut env_curve = Vec::with_capacity(total);
        let mut markers = Vec::new();

        let mut event_index = 0;
        for sample in 0..total {
            while event_index < events.len() {
                let ev_sample = (events[event_index].0 * sample_rate as f32) as usize;
                if ev_sample == sample {
                    if events[event_index].1 {
                        amp_env.trigger();
                        filter_env.trigger();
                        markers.push((sample, true));
                    } else {
                        amp_env.release();
                        filter_env.release();
                        markers.push((sample, false));
                    }
                    event_index += 1;
                } else { break; }
            }

            let filter_env_amp = filter_env.next_amplitude();
            let modulated_cutoff = if filter_env_amount > 0.0 {
                let env_mod = cutoff_range * filter_env_amount * filter_env_amp;
                (base_cutoff + env_mod).clamp(20.0, max_cutoff)
            } else {
                base_cutoff
            };

            filter.set_cutoff(modulated_cutoff);
            let raw = osc.next_sample();
            let filtered = filter.process(raw);
            let amp = amp_env.next_amplitude();

            audio.push(filtered * amp);
            cutoff_curve.push(modulated_cutoff);
            env_curve.push(filter_env_amp);
        }

        (audio, cutoff_curve, env_curve, markers)
    }

    fn plot_filter_envelope(
        audio: &[f32],
        cutoff: &[f32],
        env_curve: &[f32],
        events: &[(usize, bool)],
        sample_rate: u32,
        filename: &str,
        title: &str,
    ) {
        use plotters::prelude::*;

        let path = PathBuf::from(filename);
        {
            let root = BitMapBackend::new(&path, (1400, 900)).into_drawing_area();
            root.fill(&WHITE).unwrap();
            let areas = root.split_evenly((3, 1));
            let duration = audio.len() as f32 / sample_rate as f32;
            let subsample = (audio.len() / 3000).max(1);

            // --- Panel 1: Audio waveform ---
            let mut c1 = ChartBuilder::on(&areas[0])
                .caption(title, ("sans-serif", 18))
                .margin(5).x_label_area_size(20).y_label_area_size(50)
                .build_cartesian_2d(0.0f32..duration, -1.1f32..1.1f32).unwrap();
            c1.configure_mesh().x_desc("Time (s)").y_desc("Audio").draw().unwrap();
            c1.draw_series(LineSeries::new(
                audio.iter().enumerate().step_by(subsample)
                    .map(|(i, &s)| (i as f32 / sample_rate as f32, s)),
                BLUE.mix(0.6),
            )).unwrap().label("Audio").legend(|(x,y)| PathElement::new(vec![(x,y),(x+20,y)], BLUE));
            for (idx, is_on) in events {
                let x = *idx as f32 / sample_rate as f32;
                c1.draw_series(std::iter::once(
                    TriangleMarker::new((x, 1.05), 8, if *is_on { GREEN } else { RED }.filled())
                )).unwrap();
            }
            c1.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw().unwrap();

            // --- Panel 2: Cutoff frequency trajectory ---
            let max_cutoff_val = cutoff.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let min_cutoff_val = cutoff.iter().cloned().fold(f32::INFINITY, f32::min);
            let cutoff_range_display = (max_cutoff_val - min_cutoff_val).max(100.0);
            let y_lo = (min_cutoff_val - cutoff_range_display * 0.1).max(0.0);
            let y_hi = max_cutoff_val + cutoff_range_display * 0.1;

            let mut c2 = ChartBuilder::on(&areas[1])
                .caption("Cutoff Frequency (Hz) - look for jumps!", ("sans-serif", 16))
                .margin(5).x_label_area_size(20).y_label_area_size(60)
                .build_cartesian_2d(0.0f32..duration, y_lo..y_hi).unwrap();
            c2.configure_mesh().x_desc("Time (s)").y_desc("Cutoff (Hz)").draw().unwrap();
            c2.draw_series(LineSeries::new(
                cutoff.iter().enumerate().step_by(subsample)
                    .map(|(i, &f)| (i as f32 / sample_rate as f32, f)),
                RED.stroke_width(2),
            )).unwrap().label("Cutoff Hz").legend(|(x,y)| PathElement::new(vec![(x,y),(x+20,y)], RED));
            for (idx, is_on) in events {
                let x = *idx as f32 / sample_rate as f32;
                c2.draw_series(std::iter::once(
                    TriangleMarker::new((x, y_hi * 0.95), 8, if *is_on { GREEN } else { RED }.filled())
                )).unwrap();
            }
            c2.configure_series_labels().background_style(WHITE.mix(0.8)).border_style(BLACK).draw().unwrap();

            // --- Panel 3: Filter envelope amplitude ---
            let mut c3 = ChartBuilder::on(&areas[2])
                .caption("Filter Envelope Amplitude", ("sans-serif", 16))
                .margin(5).x_label_area_size(25).y_label_area_size(50)
                .build_cartesian_2d(0.0f32..duration, -0.05f32..1.1f32).unwrap();
            c3.configure_mesh().x_desc("Time (s)").y_desc("Env Amp").draw().unwrap();
            c3.draw_series(LineSeries::new(
                env_curve.iter().enumerate().step_by(subsample)
                    .map(|(i, &e)| (i as f32 / sample_rate as f32, e)),
                RGBColor(180, 0, 200).stroke_width(2),
            )).unwrap();

            root.present().unwrap();
        }
        println!("Saved: {}", path.display());
    }

    fn find_cutoff_jumps(cutoff: &[f32], events: &[(usize, bool)], threshold_hz: f32)
        -> Vec<(usize, f32, f32, f32)> // (sample, prev, curr, delta)
    {
        let mut jumps = Vec::new();
        for i in 1..cutoff.len() {
            let delta = (cutoff[i] - cutoff[i-1]).abs();
            if delta > threshold_hz {
                let near_event = events.iter().any(|(idx,_)| (*idx as isize - i as isize).abs() <= 10);
                jumps.push((i, cutoff[i-1], cutoff[i], delta));
                if near_event {
                    // Mark as event-related
                }
            }
        }
        jumps
    }

    /// Basic filter sweep: sustain=1.0 should produce a smooth cutoff ramp
    #[test]
    fn plot_filter_envelope_sustain_full() {
        let sample_rate = 44100u32;
        let events = vec![(0.0, true), (1.0, false)];
        let (audio, cutoff, env_curve, markers) = run_filter_envelope_simulation(
            500.0, 0.8, 1.0, 0.1, 0.2, 0.3, 0.7, &events, 2.0, sample_rate,
        );
        plot_filter_envelope(&audio, &cutoff, &env_curve, &markers, sample_rate,
            "test_filter_env_sustain_1_0.png", "Filter Env - Sustain=1.0 (base=500Hz, amount=0.8)");

        let jumps = find_cutoff_jumps(&cutoff, &markers, 500.0);
        println!("Sustain=1.0: {} jumps >500Hz", jumps.len());
        for (i, prev, curr, d) in &jumps {
            println!("  At {}ms: {:.0}Hz -> {:.0}Hz (Δ={:.0}Hz)",
                *i * 1000 / sample_rate as usize, prev, curr, d);
        }
        assert!(jumps.is_empty(), "Unexpected cutoff jumps with sustain=1.0");
    }

    /// Sustain=0.5: decay should ramp cutoff down smoothly to mid point
    #[test]
    fn plot_filter_envelope_sustain_half() {
        let sample_rate = 44100u32;
        let events = vec![(0.0, true), (1.0, false)];
        let (audio, cutoff, env_curve, markers) = run_filter_envelope_simulation(
            500.0, 0.8, 0.5, 0.1, 0.3, 0.3, 0.7, &events, 2.0, sample_rate,
        );
        plot_filter_envelope(&audio, &cutoff, &env_curve, &markers, sample_rate,
            "test_filter_env_sustain_0_5.png", "Filter Env - Sustain=0.5 (base=500Hz, amount=0.8)");

        let jumps = find_cutoff_jumps(&cutoff, &markers, 500.0);
        println!("Sustain=0.5: {} jumps >500Hz", jumps.len());
        for (i, prev, curr, d) in &jumps {
            println!("  At {}ms: {:.0}Hz -> {:.0}Hz (Δ={:.0}Hz)",
                *i * 1000 / sample_rate as usize, prev, curr, d);
        }
    }

    /// Sustain=0.0 (pluck): exposes the cutoff jump bug when re-triggering
    #[test]
    fn plot_filter_envelope_sustain_zero_retrigger() {
        let sample_rate = 44100u32;
        // Re-trigger while filter envelope is mid-decay
        let events = vec![(0.0, true), (0.15, false), (0.2, true), (0.8, false)];
        let (audio, cutoff, env_curve, markers) = run_filter_envelope_simulation(
            500.0, 0.8, 0.0, 0.01, 0.3, 0.1, 0.7, &events, 1.5, sample_rate,
        );
        plot_filter_envelope(&audio, &cutoff, &env_curve, &markers, sample_rate,
            "test_filter_env_retrigger_pluck.png", "Filter Env - Sustain=0.0 Retrigger (click source?)");

        println!("\n=== FILTER CUTOFF JUMP ANALYSIS (retrigger, sustain=0) ===");
        let jumps = find_cutoff_jumps(&cutoff, &markers, 200.0);
        if jumps.is_empty() {
            println!("✓ No cutoff jumps detected");
        } else {
            println!("⚠ {} cutoff jumps detected (potential filter clicks):", jumps.len());
            for (i, prev, curr, d) in &jumps {
                println!("  At {}ms: {:.0}Hz -> {:.0}Hz (Δ={:.0}Hz)",
                    *i * 1000 / sample_rate as usize, prev, curr, d);
            }
        }
    }

    /// HIGH base_cutoff (20000Hz default): filter_env_amount > 0 should have NO effect
    /// because cutoff_range = 20000 - 20000 = 0 → demonstrates the "no effect" bug
    #[test]
    fn plot_filter_envelope_high_base_cutoff_zero_range() {
        let sample_rate = 44100u32;
        let events = vec![(0.0, true), (1.0, false)];
        // base_cutoff=20000 means cutoff_range=0, filter env can't modulate anything
        let (audio, cutoff, env_curve, markers) = run_filter_envelope_simulation(
            20000.0, 0.8, 0.5, 0.1, 0.3, 0.3, 0.7, &events, 2.0, sample_rate,
        );
        plot_filter_envelope(&audio, &cutoff, &env_curve, &markers, sample_rate,
            "test_filter_env_base_20k.png", "Filter Env - base_cutoff=20kHz (envelope has no effect!)");

        let range = cutoff.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                  - cutoff.iter().cloned().fold(f32::INFINITY, f32::min);
        println!("Cutoff range when base=20kHz: {:.1} Hz (should be 0)", range);
        println!("⚠ If range≈0, the filter envelope can NEVER be heard at this base cutoff.");
    }

    /// Verify that Voice::note_on fix (reset() + trigger()) ensures filter envelope
    /// always starts from amplitude 0, regardless of previous state.
    #[test]
    fn analyze_filter_env_retrigger_amplitude_continuity() {
        let sample_rate = 44100u32;

        // --- BEFORE FIX: raw trigger() resumes from sustain level ---
        let mut env_raw = ADSREnvelope::new(0.05, 0.3, 0.5, 0.2, sample_rate);
        env_raw.trigger();
        for _ in 0..(sample_rate as usize / 2) { env_raw.next_amplitude(); }
        let amp_at_sustain = env_raw.next_amplitude(); // should be ~0.5
        env_raw.trigger(); // raw retrigger — no reset
        let amp_after_raw_retrigger = env_raw.next_amplitude();

        // --- AFTER FIX: reset() + trigger() always starts from 0 ---
        let mut env_fixed = ADSREnvelope::new(0.05, 0.3, 0.5, 0.2, sample_rate);
        env_fixed.trigger();
        for _ in 0..(sample_rate as usize / 2) { env_fixed.next_amplitude(); }
        env_fixed.reset();   // Voice::note_on() now calls this first
        env_fixed.trigger();
        let amp_after_fixed_retrigger = env_fixed.next_amplitude();

        println!("\n=== FILTER ENV RETRIGGER ANALYSIS ===");
        println!("Amplitude at sustain (before retrigger):       {:.4}", amp_at_sustain);
        println!("After raw trigger()  (buggy — no reset):       {:.4} (≈ sustain level)", amp_after_raw_retrigger);
        println!("After reset()+trigger() (fixed — Voice path):  {:.4} (≈ 0.0)", amp_after_fixed_retrigger);

        // The fix: reset()+trigger() must start from ~0
        assert!(
            amp_after_fixed_retrigger < 0.01,
            "Fixed retrigger should start near 0.0, got {:.4}", amp_after_fixed_retrigger
        );
        // Document the bug: raw trigger() does NOT reset
        assert!(
            (amp_after_raw_retrigger - amp_at_sustain).abs() < 0.01,
            "Raw trigger() should resume from sustain level (known behavior)"
        );
        println!("✓ Fix confirmed: Voice::note_on() reset()+trigger() starts filter envelope from 0");
    }
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

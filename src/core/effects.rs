//! Audio effects: Delay, Reverb, Chorus

use super::types::{Sample, SampleRate};

/// Trait for audio effects
pub trait Effect: Send {
    /// Process a single sample
    fn process(&mut self, input: Sample) -> Sample;
    
    /// Set the sample rate (call when sample rate changes)
    fn set_sample_rate(&mut self, sample_rate: SampleRate);
    
    /// Reset internal state (clear buffers)
    fn reset(&mut self);
}

// ============================================================================
// DELAY
// ============================================================================

/// Simple delay line buffer
struct DelayLine {
    buffer: Vec<Sample>,
    write_pos: usize,
    sample_rate: SampleRate,
}

impl DelayLine {
    fn new(max_delay_seconds: f32, sample_rate: SampleRate) -> Self {
        let size = (max_delay_seconds * sample_rate as f32) as usize + 1;
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
            sample_rate,
        }
    }
    
    fn write(&mut self, sample: Sample) {
        self.buffer[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
    }
    
    fn read(&self, delay_samples: usize) -> Sample {
        let read_pos = (self.write_pos + self.buffer.len() - delay_samples) % self.buffer.len();
        self.buffer[read_pos]
    }
    
    fn read_interpolated(&self, delay_samples: f32) -> Sample {
        let delay_int = delay_samples as usize;
        let frac = delay_samples - delay_int as f32;
        
        let s1 = self.read(delay_int);
        let s2 = self.read(delay_int + 1);
        
        s1 + frac * (s2 - s1)
    }
    
    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

/// Stereo delay effect with feedback
pub struct Delay {
    delay_line: DelayLine,
    delay_time: f32,      // Delay time in seconds (0.0 - 2.0)
    feedback: f32,        // Feedback amount (0.0 - 0.95)
    mix: f32,             // Dry/wet mix (0.0 = dry, 1.0 = wet)
    sample_rate: SampleRate,
}

impl Delay {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            delay_line: DelayLine::new(2.0, sample_rate),
            delay_time: 0.3,
            feedback: 0.4,
            mix: 0.3,
            sample_rate,
        }
    }
    
    pub fn set_delay_time(&mut self, time: f32) {
        self.delay_time = time.clamp(0.001, 2.0);
    }
    
    pub fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.95);
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    pub fn delay_time(&self) -> f32 { self.delay_time }
    pub fn feedback(&self) -> f32 { self.feedback }
    pub fn mix(&self) -> f32 { self.mix }
}

impl Effect for Delay {
    fn process(&mut self, input: Sample) -> Sample {
        let delay_samples = (self.delay_time * self.sample_rate as f32) as usize;
        let delayed = self.delay_line.read(delay_samples.max(1));
        
        // Write input + feedback to delay line
        self.delay_line.write(input + delayed * self.feedback);
        
        // Mix dry and wet
        input * (1.0 - self.mix) + delayed * self.mix
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.delay_line = DelayLine::new(2.0, sample_rate);
    }
    
    fn reset(&mut self) {
        self.delay_line.clear();
    }
}

// ============================================================================
// REVERB (Schroeder-style with allpass + comb filters)
// ============================================================================

/// Simple comb filter for reverb
struct CombFilter {
    buffer: Vec<Sample>,
    write_pos: usize,
    feedback: f32,
    damp: f32,
    damp_state: Sample,
}

impl CombFilter {
    fn new(delay_samples: usize, feedback: f32, damp: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            write_pos: 0,
            feedback,
            damp,
            damp_state: 0.0,
        }
    }
    
    fn process(&mut self, input: Sample) -> Sample {
        let output = self.buffer[self.write_pos];
        
        // Low-pass filter on feedback (damping)
        self.damp_state = output * (1.0 - self.damp) + self.damp_state * self.damp;
        
        self.buffer[self.write_pos] = input + self.damp_state * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        
        output
    }
    
    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.damp_state = 0.0;
    }
}

/// Allpass filter for reverb diffusion
struct AllpassFilter {
    buffer: Vec<Sample>,
    write_pos: usize,
    feedback: f32,
}

impl AllpassFilter {
    fn new(delay_samples: usize, feedback: f32) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            write_pos: 0,
            feedback,
        }
    }
    
    fn process(&mut self, input: Sample) -> Sample {
        let delayed = self.buffer[self.write_pos];
        let output = -input + delayed;
        
        self.buffer[self.write_pos] = input + delayed * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        
        output
    }
    
    fn clear(&mut self) {
        self.buffer.fill(0.0);
    }
}

/// Schroeder reverb with 4 comb filters and 2 allpass filters
pub struct Reverb {
    combs: Vec<CombFilter>,
    allpasses: Vec<AllpassFilter>,
    room_size: f32,       // 0.0 - 1.0
    damping: f32,         // 0.0 - 1.0
    mix: f32,             // Dry/wet mix
    sample_rate: SampleRate,
}

impl Reverb {
    pub fn new(sample_rate: SampleRate) -> Self {
        let mut reverb = Self {
            combs: Vec::new(),
            allpasses: Vec::new(),
            room_size: 0.5,
            damping: 0.5,
            mix: 0.3,
            sample_rate,
        };
        reverb.rebuild_filters();
        reverb
    }
    
    fn rebuild_filters(&mut self) {
        // Comb filter delay times (in samples at 44100 Hz, scaled to current rate)
        let scale = self.sample_rate as f32 / 44100.0;
        let comb_delays = [1116, 1188, 1277, 1356];
        let allpass_delays = [556, 441];
        
        let feedback = 0.84 + self.room_size * 0.12; // 0.84 - 0.96
        
        self.combs = comb_delays.iter()
            .map(|&d| CombFilter::new((d as f32 * scale) as usize, feedback, self.damping))
            .collect();
        
        self.allpasses = allpass_delays.iter()
            .map(|&d| AllpassFilter::new((d as f32 * scale) as usize, 0.5))
            .collect();
    }
    
    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        // Update comb filter feedback
        let feedback = 0.84 + self.room_size * 0.12;
        for comb in &mut self.combs {
            comb.feedback = feedback;
        }
    }
    
    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
        for comb in &mut self.combs {
            comb.damp = self.damping;
        }
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    pub fn room_size(&self) -> f32 { self.room_size }
    pub fn damping(&self) -> f32 { self.damping }
    pub fn mix(&self) -> f32 { self.mix }
}

impl Effect for Reverb {
    fn process(&mut self, input: Sample) -> Sample {
        // Sum of parallel comb filters
        let mut comb_sum: Sample = 0.0;
        for comb in &mut self.combs {
            comb_sum += comb.process(input);
        }
        comb_sum *= 0.25; // Normalize
        
        // Series allpass filters for diffusion
        let mut output = comb_sum;
        for allpass in &mut self.allpasses {
            output = allpass.process(output);
        }
        
        // Mix dry and wet
        input * (1.0 - self.mix) + output * self.mix
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.rebuild_filters();
    }
    
    fn reset(&mut self) {
        for comb in &mut self.combs {
            comb.clear();
        }
        for allpass in &mut self.allpasses {
            allpass.clear();
        }
    }
}

// ============================================================================
// CHORUS
// ============================================================================

/// Chorus effect using modulated delay
pub struct Chorus {
    delay_line: DelayLine,
    lfo_phase: f32,
    rate: f32,            // LFO rate in Hz (0.1 - 5.0)
    depth: f32,           // Modulation depth in ms (0.0 - 10.0)
    mix: f32,             // Dry/wet mix
    sample_rate: SampleRate,
}

impl Chorus {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            delay_line: DelayLine::new(0.05, sample_rate), // 50ms max
            lfo_phase: 0.0,
            rate: 1.5,
            depth: 3.0,
            mix: 0.5,
            sample_rate,
        }
    }
    
    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate.clamp(0.1, 5.0);
    }
    
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 10.0);
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    pub fn rate(&self) -> f32 { self.rate }
    pub fn depth(&self) -> f32 { self.depth }
    pub fn mix(&self) -> f32 { self.mix }
}

impl Effect for Chorus {
    fn process(&mut self, input: Sample) -> Sample {
        // LFO (sine wave)
        let lfo = (self.lfo_phase * std::f32::consts::TAU).sin();
        self.lfo_phase += self.rate / self.sample_rate as f32;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        
        // Modulated delay time (center at 7ms + depth modulation)
        let center_delay_ms = 7.0;
        let delay_ms = center_delay_ms + lfo * self.depth * 0.5;
        let delay_samples = delay_ms * 0.001 * self.sample_rate as f32;
        
        // Write to delay line
        self.delay_line.write(input);
        
        // Read with interpolation
        let delayed = self.delay_line.read_interpolated(delay_samples);
        
        // Mix dry and wet
        input * (1.0 - self.mix) + delayed * self.mix
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.delay_line = DelayLine::new(0.05, sample_rate);
    }
    
    fn reset(&mut self) {
        self.delay_line.clear();
        self.lfo_phase = 0.0;
    }
}

// ============================================================================
// EFFECTS CHAIN
// ============================================================================

/// Chain of effects processed in series
pub struct EffectsChain {
    pub delay: Delay,
    pub reverb: Reverb,
    pub chorus: Chorus,
    pub delay_enabled: bool,
    pub reverb_enabled: bool,
    pub chorus_enabled: bool,
}

impl EffectsChain {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            delay: Delay::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            chorus: Chorus::new(sample_rate),
            delay_enabled: false,
            reverb_enabled: false,
            chorus_enabled: false,
        }
    }
    
    pub fn process(&mut self, input: Sample) -> Sample {
        let mut output = input;
        
        // Process in order: Chorus → Delay → Reverb
        if self.chorus_enabled {
            output = self.chorus.process(output);
        }
        if self.delay_enabled {
            output = self.delay.process(output);
        }
        if self.reverb_enabled {
            output = self.reverb.process(output);
        }
        
        output
    }
    
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.delay.set_sample_rate(sample_rate);
        self.reverb.set_sample_rate(sample_rate);
        self.chorus.set_sample_rate(sample_rate);
    }
    
    pub fn reset(&mut self) {
        self.delay.reset();
        self.reverb.reset();
        self.chorus.reset();
    }
}

//! Audio effects: Delay, Reverb, Chorus

use super::types::{Sample, SampleRate, StereoSample};

/// Trait for audio effects (mono)
pub trait Effect: Send {
    /// Process a single sample
    fn process(&mut self, input: Sample) -> Sample;
    
    /// Set the sample rate (call when sample rate changes)
    fn set_sample_rate(&mut self, sample_rate: SampleRate);
    
    /// Reset internal state (clear buffers)
    fn reset(&mut self);
}

/// Trait for stereo audio effects
pub trait StereoEffect: Send {
    /// Process a stereo sample
    fn process_stereo(&mut self, input: StereoSample) -> StereoSample;
    
    /// Set the sample rate
    fn set_sample_rate(&mut self, sample_rate: SampleRate);
    
    /// Reset internal state
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

/// Stereo ping-pong delay effect
pub struct Delay {
    delay_line_l: DelayLine,
    delay_line_r: DelayLine,
    delay_time: f32,      // Delay time in seconds (0.0 - 2.0)
    feedback: f32,        // Feedback amount (0.0 - 0.95)
    mix: f32,             // Dry/wet mix (0.0 = dry, 1.0 = wet)
    ping_pong: bool,      // Enable ping-pong mode
    sample_rate: SampleRate,
}

impl Delay {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            delay_line_l: DelayLine::new(2.0, sample_rate),
            delay_line_r: DelayLine::new(2.0, sample_rate),
            delay_time: 0.3,
            feedback: 0.4,
            mix: 0.3,
            ping_pong: true,  // Ping-pong on by default for stereo goodness
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
    
    pub fn set_ping_pong(&mut self, enabled: bool) {
        self.ping_pong = enabled;
    }
    
    pub fn delay_time(&self) -> f32 { self.delay_time }
    pub fn feedback(&self) -> f32 { self.feedback }
    pub fn mix(&self) -> f32 { self.mix }
    pub fn ping_pong(&self) -> bool { self.ping_pong }
    
    /// Process stereo with ping-pong
    pub fn process_stereo(&mut self, input: StereoSample) -> StereoSample {
        let delay_samples = (self.delay_time * self.sample_rate as f32) as usize;
        let delay_samples = delay_samples.max(1);
        
        let delayed_l = self.delay_line_l.read(delay_samples);
        let delayed_r = self.delay_line_r.read(delay_samples);
        
        if self.ping_pong {
            // Ping-pong: L feeds into R, R feeds into L
            self.delay_line_l.write(input.left + delayed_r * self.feedback);
            self.delay_line_r.write(input.right + delayed_l * self.feedback);
        } else {
            // Standard stereo delay
            self.delay_line_l.write(input.left + delayed_l * self.feedback);
            self.delay_line_r.write(input.right + delayed_r * self.feedback);
        }
        
        StereoSample::new(
            input.left * (1.0 - self.mix) + delayed_l * self.mix,
            input.right * (1.0 - self.mix) + delayed_r * self.mix,
        )
    }
}

impl Effect for Delay {
    fn process(&mut self, input: Sample) -> Sample {
        let stereo = self.process_stereo(StereoSample::from_mono(input));
        (stereo.left + stereo.right) * 0.5
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.delay_line_l = DelayLine::new(2.0, sample_rate);
        self.delay_line_r = DelayLine::new(2.0, sample_rate);
    }
    
    fn reset(&mut self) {
        self.delay_line_l.clear();
        self.delay_line_r.clear();
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

/// Stereo Schroeder reverb with different L/R comb filter delays
pub struct Reverb {
    combs_l: Vec<CombFilter>,
    combs_r: Vec<CombFilter>,
    allpasses_l: Vec<AllpassFilter>,
    allpasses_r: Vec<AllpassFilter>,
    room_size: f32,       // 0.0 - 1.0
    damping: f32,         // 0.0 - 1.0
    mix: f32,             // Dry/wet mix
    sample_rate: SampleRate,
}

impl Reverb {
    pub fn new(sample_rate: SampleRate) -> Self {
        let mut reverb = Self {
            combs_l: Vec::new(),
            combs_r: Vec::new(),
            allpasses_l: Vec::new(),
            allpasses_r: Vec::new(),
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
        // Different delays for L/R create stereo width
        let scale = self.sample_rate as f32 / 44100.0;
        let comb_delays_l = [1116, 1188, 1277, 1356];
        let comb_delays_r = [1139, 1211, 1300, 1379];  // Slightly different for stereo
        let allpass_delays_l = [556, 441];
        let allpass_delays_r = [579, 464];  // Slightly different for stereo
        
        let feedback = 0.84 + self.room_size * 0.12; // 0.84 - 0.96
        
        self.combs_l = comb_delays_l.iter()
            .map(|&d| CombFilter::new((d as f32 * scale) as usize, feedback, self.damping))
            .collect();
        self.combs_r = comb_delays_r.iter()
            .map(|&d| CombFilter::new((d as f32 * scale) as usize, feedback, self.damping))
            .collect();
        
        self.allpasses_l = allpass_delays_l.iter()
            .map(|&d| AllpassFilter::new((d as f32 * scale) as usize, 0.5))
            .collect();
        self.allpasses_r = allpass_delays_r.iter()
            .map(|&d| AllpassFilter::new((d as f32 * scale) as usize, 0.5))
            .collect();
    }
    
    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);
        // Update comb filter feedback
        let feedback = 0.84 + self.room_size * 0.12;
        for comb in &mut self.combs_l {
            comb.feedback = feedback;
        }
        for comb in &mut self.combs_r {
            comb.feedback = feedback;
        }
    }
    
    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
        for comb in &mut self.combs_l {
            comb.damp = self.damping;
        }
        for comb in &mut self.combs_r {
            comb.damp = self.damping;
        }
    }
    
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }
    
    pub fn room_size(&self) -> f32 { self.room_size }
    pub fn damping(&self) -> f32 { self.damping }
    pub fn mix(&self) -> f32 { self.mix }
    
    /// Process stereo reverb
    pub fn process_stereo(&mut self, input: StereoSample) -> StereoSample {
        // Sum of parallel comb filters (separate L/R)
        let mut comb_sum_l: Sample = 0.0;
        let mut comb_sum_r: Sample = 0.0;
        for comb in &mut self.combs_l {
            comb_sum_l += comb.process(input.left);
        }
        for comb in &mut self.combs_r {
            comb_sum_r += comb.process(input.right);
        }
        comb_sum_l *= 0.25;
        comb_sum_r *= 0.25;
        
        // Series allpass filters for diffusion
        let mut output_l = comb_sum_l;
        let mut output_r = comb_sum_r;
        for allpass in &mut self.allpasses_l {
            output_l = allpass.process(output_l);
        }
        for allpass in &mut self.allpasses_r {
            output_r = allpass.process(output_r);
        }
        
        // Mix dry and wet
        StereoSample::new(
            input.left * (1.0 - self.mix) + output_l * self.mix,
            input.right * (1.0 - self.mix) + output_r * self.mix,
        )
    }
}

impl Effect for Reverb {
    fn process(&mut self, input: Sample) -> Sample {
        let stereo = self.process_stereo(StereoSample::from_mono(input));
        (stereo.left + stereo.right) * 0.5
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.rebuild_filters();
    }
    
    fn reset(&mut self) {
        for comb in &mut self.combs_l {
            comb.clear();
        }
        for comb in &mut self.combs_r {
            comb.clear();
        }
        for allpass in &mut self.allpasses_l {
            allpass.clear();
        }
        for allpass in &mut self.allpasses_r {
            allpass.clear();
        }
    }
}

// ============================================================================
// CHORUS
// ============================================================================

/// Stereo chorus effect with LFO phase offset between L/R
pub struct Chorus {
    delay_line_l: DelayLine,
    delay_line_r: DelayLine,
    lfo_phase: f32,
    rate: f32,            // LFO rate in Hz (0.1 - 5.0)
    depth: f32,           // Modulation depth in ms (0.0 - 10.0)
    mix: f32,             // Dry/wet mix
    stereo_spread: f32,   // LFO phase offset for stereo (0.0 - 0.5)
    sample_rate: SampleRate,
}

impl Chorus {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            delay_line_l: DelayLine::new(0.05, sample_rate), // 50ms max
            delay_line_r: DelayLine::new(0.05, sample_rate),
            lfo_phase: 0.0,
            rate: 1.5,
            depth: 3.0,
            mix: 0.5,
            stereo_spread: 0.25,  // 90 degrees phase offset
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
    
    /// Process stereo chorus
    pub fn process_stereo(&mut self, input: StereoSample) -> StereoSample {
        // LFO for left channel
        let lfo_l = (self.lfo_phase * std::f32::consts::TAU).sin();
        // LFO for right channel (phase offset for stereo width)
        let lfo_r = ((self.lfo_phase + self.stereo_spread) * std::f32::consts::TAU).sin();
        
        self.lfo_phase += self.rate / self.sample_rate as f32;
        if self.lfo_phase >= 1.0 {
            self.lfo_phase -= 1.0;
        }
        
        // Modulated delay times
        let center_delay_ms = 7.0;
        let delay_ms_l = center_delay_ms + lfo_l * self.depth * 0.5;
        let delay_ms_r = center_delay_ms + lfo_r * self.depth * 0.5;
        let delay_samples_l = delay_ms_l * 0.001 * self.sample_rate as f32;
        let delay_samples_r = delay_ms_r * 0.001 * self.sample_rate as f32;
        
        // Write to delay lines
        self.delay_line_l.write(input.left);
        self.delay_line_r.write(input.right);
        
        // Read with interpolation
        let delayed_l = self.delay_line_l.read_interpolated(delay_samples_l);
        let delayed_r = self.delay_line_r.read_interpolated(delay_samples_r);
        
        // Mix dry and wet
        StereoSample::new(
            input.left * (1.0 - self.mix) + delayed_l * self.mix,
            input.right * (1.0 - self.mix) + delayed_r * self.mix,
        )
    }
}

impl Effect for Chorus {
    fn process(&mut self, input: Sample) -> Sample {
        let stereo = self.process_stereo(StereoSample::from_mono(input));
        (stereo.left + stereo.right) * 0.5
    }
    
    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.delay_line_l = DelayLine::new(0.05, sample_rate);
        self.delay_line_r = DelayLine::new(0.05, sample_rate);
    }
    
    fn reset(&mut self) {
        self.delay_line_l.clear();
        self.delay_line_r.clear();
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
    
    /// Process mono (for backwards compatibility)
    pub fn process(&mut self, input: Sample) -> Sample {
        let stereo = self.process_stereo(StereoSample::from_mono(input));
        (stereo.left + stereo.right) * 0.5
    }
    
    /// Process stereo signal through effects chain
    pub fn process_stereo(&mut self, input: StereoSample) -> StereoSample {
        let mut output = input;
        
        // Process in order: Chorus → Delay → Reverb
        if self.chorus_enabled {
            output = self.chorus.process_stereo(output);
        }
        if self.delay_enabled {
            output = self.delay.process_stereo(output);
        }
        if self.reverb_enabled {
            output = self.reverb.process_stereo(output);
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

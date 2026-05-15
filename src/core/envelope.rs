use super::types::{Amplitude, SampleRate};

/// Current state of an envelope
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnvelopeState {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Trait for all envelope types.
/// Envelopes shape the amplitude of a sound over time.
pub trait Envelope: Send + Sync {
    /// Get the next amplitude value
    fn next_amplitude(&mut self) -> Amplitude;

    /// Trigger the envelope (note on)
    fn trigger(&mut self);

    /// Release the envelope (note off)
    fn release(&mut self);

    /// Check if the envelope has finished (returned to idle)
    fn is_finished(&self) -> bool;

    /// Get the current state
    fn state(&self) -> EnvelopeState;

    /// Reset the envelope to initial state
    fn reset(&mut self);

    /// Set the sample rate
    fn set_sample_rate(&mut self, sample_rate: SampleRate);

    /// Set attack time in seconds
    fn set_attack(&mut self, _attack_time: f32) {}

    /// Set decay time in seconds
    fn set_decay(&mut self, _decay_time: f32) {}

    /// Set sustain level (0.0 to 1.0)
    fn set_sustain(&mut self, _sustain_level: f32) {}

    /// Set release time in seconds
    fn set_release(&mut self, _release_time: f32) {}

    /// Get attack time in seconds
    fn attack(&self) -> f32 { 0.0 }

    /// Get decay time in seconds
    fn decay(&self) -> f32 { 0.0 }

    /// Get sustain level
    fn sustain(&self) -> f32 { 1.0 }

    /// Get release time in seconds
    fn release_time(&self) -> f32 { 0.0 }
}

/// Simple Attack-Release envelope
/// Attack: time to reach full amplitude
/// Release: time to fade to zero after note off
pub struct AREnvelope {
    attack_time: f32,
    release_time: f32,
    sample_rate: SampleRate,
    state: EnvelopeState,
    current_amplitude: Amplitude,
    attack_increment: f32,
    release_decrement: f32,
}

impl AREnvelope {
    pub fn new(attack_time: f32, release_time: f32, sample_rate: SampleRate) -> Self {
        let mut env = Self {
            attack_time,
            release_time,
            sample_rate,
            state: EnvelopeState::Idle,
            current_amplitude: 0.0,
            attack_increment: 0.0,
            release_decrement: 0.0,
        };
        env.update_increments();
        env
    }

    pub fn set_attack(&mut self, attack_time: f32) {
        self.attack_time = attack_time.max(0.001);
        self.update_increments();
    }

    pub fn set_release(&mut self, release_time: f32) {
        self.release_time = release_time.max(0.001);
        self.update_increments();
    }

    fn update_increments(&mut self) {
        let samples_per_second = self.sample_rate as f32;
        self.attack_increment = 1.0 / (self.attack_time * samples_per_second);
        self.release_decrement = 1.0 / (self.release_time * samples_per_second);
    }
}

impl Envelope for AREnvelope {
    fn next_amplitude(&mut self) -> Amplitude {
        match self.state {
            EnvelopeState::Idle => {
                self.current_amplitude = 0.0;
            }
            EnvelopeState::Attack => {
                self.current_amplitude += self.attack_increment;
                if self.current_amplitude >= 1.0 {
                    self.current_amplitude = 1.0;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                self.current_amplitude = 1.0;
            }
            EnvelopeState::Release => {
                self.current_amplitude -= self.release_decrement;
                if self.current_amplitude <= 0.0 {
                    self.current_amplitude = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
            EnvelopeState::Decay => {
                // AR envelope doesn't use decay, but included for trait completeness
                self.state = EnvelopeState::Sustain;
            }
        }
        self.current_amplitude
    }

    fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    fn release(&mut self) {
        if self.state != EnvelopeState::Idle {
            self.state = EnvelopeState::Release;
        }
    }

    fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Idle
    }

    fn state(&self) -> EnvelopeState {
        self.state
    }

    fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.current_amplitude = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_increments();
    }

    fn set_attack(&mut self, attack_time: f32) {
        self.attack_time = attack_time.max(0.001);
        self.update_increments();
    }

    fn set_release(&mut self, release_time: f32) {
        self.release_time = release_time.max(0.001);
        self.update_increments();
    }

    fn attack(&self) -> f32 {
        self.attack_time
    }

    fn release_time(&self) -> f32 {
        self.release_time
    }
}

/// Full ADSR envelope for future use
pub struct ADSREnvelope {
    attack_time: f32,
    decay_time: f32,
    sustain_level: Amplitude,
    release_time: f32,
    sample_rate: SampleRate,
    state: EnvelopeState,
    current_amplitude: Amplitude,
    attack_increment: f32,
    decay_decrement: f32,
    release_decrement: f32,
}

impl ADSREnvelope {
    pub fn new(
        attack_time: f32,
        decay_time: f32,
        sustain_level: Amplitude,
        release_time: f32,
        sample_rate: SampleRate,
    ) -> Self {
        let mut env = Self {
            attack_time: attack_time.max(0.001),
            decay_time: decay_time.max(0.001),
            sustain_level: sustain_level.clamp(0.0, 1.0),
            release_time: release_time.max(0.001),
            sample_rate,
            state: EnvelopeState::Idle,
            current_amplitude: 0.0,
            attack_increment: 0.0,
            decay_decrement: 0.0,
            release_decrement: 0.0,
        };
        env.update_increments();
        env
    }

    /// Create with default parameters (quick attack, medium decay, 70% sustain, medium release)
    pub fn default_adsr(sample_rate: SampleRate) -> Self {
        Self::new(0.01, 0.1, 0.7, 0.2, sample_rate)
    }

    fn update_increments(&mut self) {
        let samples_per_second = self.sample_rate as f32;
        self.attack_increment = 1.0 / (self.attack_time * samples_per_second);
        self.decay_decrement =
            (1.0 - self.sustain_level) / (self.decay_time * samples_per_second);
        // Release decrement is recalculated in release() based on current amplitude
        self.release_decrement =
            self.sustain_level / (self.release_time * samples_per_second);
    }
}

impl Envelope for ADSREnvelope {
    fn next_amplitude(&mut self) -> Amplitude {
        match self.state {
            EnvelopeState::Idle => {
                self.current_amplitude = 0.0;
            }
            EnvelopeState::Attack => {
                self.current_amplitude += self.attack_increment;
                if self.current_amplitude >= 1.0 {
                    self.current_amplitude = 1.0;
                    self.state = EnvelopeState::Decay;
                }
            }
            EnvelopeState::Decay => {
                self.current_amplitude -= self.decay_decrement;
                if self.current_amplitude <= self.sustain_level {
                    self.current_amplitude = self.sustain_level;
                    self.state = EnvelopeState::Sustain;
                }
            }
            EnvelopeState::Sustain => {
                self.current_amplitude = self.sustain_level;
            }
            EnvelopeState::Release => {
                self.current_amplitude -= self.release_decrement;
                if self.current_amplitude <= 0.0 {
                    self.current_amplitude = 0.0;
                    self.state = EnvelopeState::Idle;
                }
            }
        }
        self.current_amplitude
    }

    fn trigger(&mut self) {
        self.state = EnvelopeState::Attack;
    }

    fn release(&mut self) {
        if self.state != EnvelopeState::Idle {
            // Recalculate release decrement based on current amplitude
            let samples_per_second = self.sample_rate as f32;
            self.release_decrement =
                self.current_amplitude / (self.release_time.max(0.001) * samples_per_second);
            self.state = EnvelopeState::Release;
        }
    }

    fn is_finished(&self) -> bool {
        self.state == EnvelopeState::Idle
    }

    fn state(&self) -> EnvelopeState {
        self.state
    }

    fn reset(&mut self) {
        self.state = EnvelopeState::Idle;
        self.current_amplitude = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.update_increments();
    }

    fn set_attack(&mut self, attack_time: f32) {
        self.attack_time = attack_time.max(0.001);
        self.update_increments();
    }

    fn set_decay(&mut self, decay_time: f32) {
        self.decay_time = decay_time.max(0.001);
        self.update_increments();
    }

    fn set_sustain(&mut self, sustain_level: f32) {
        self.sustain_level = sustain_level.clamp(0.0, 1.0);
        self.update_increments();
    }

    fn set_release(&mut self, release_time: f32) {
        self.release_time = release_time.max(0.001);
        self.update_increments();
    }

    fn attack(&self) -> f32 {
        self.attack_time
    }

    fn decay(&self) -> f32 {
        self.decay_time
    }

    fn sustain(&self) -> f32 {
        self.sustain_level
    }

    fn release_time(&self) -> f32 {
        self.release_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ar_envelope_lifecycle() {
        let mut env = AREnvelope::new(0.01, 0.01, 44100);

        assert_eq!(env.state(), EnvelopeState::Idle);

        env.trigger();
        assert_eq!(env.state(), EnvelopeState::Attack);

        // Run through attack
        for _ in 0..500 {
            env.next_amplitude();
        }

        env.release();
        assert_eq!(env.state(), EnvelopeState::Release);

        // Run through release
        for _ in 0..500 {
            env.next_amplitude();
        }

        assert!(env.is_finished());
    }

    #[test]
    fn test_adsr_envelope_lifecycle() {
        let mut env = ADSREnvelope::new(0.01, 0.05, 0.5, 0.01, 44100);

        assert_eq!(env.state(), EnvelopeState::Idle);
        assert_eq!(env.next_amplitude(), 0.0);

        // Trigger attack
        env.trigger();
        assert_eq!(env.state(), EnvelopeState::Attack);

        // Run through attack phase
        for _ in 0..500 {
            env.next_amplitude();
        }
        assert_eq!(env.state(), EnvelopeState::Decay);

        // Run through decay phase
        for _ in 0..3000 {
            env.next_amplitude();
        }
        assert_eq!(env.state(), EnvelopeState::Sustain);
        
        // Sustain level should be ~0.5
        let amp = env.next_amplitude();
        assert!((amp - 0.5).abs() < 0.01);

        // Release
        env.release();
        assert_eq!(env.state(), EnvelopeState::Release);

        // Run through release
        for _ in 0..500 {
            env.next_amplitude();
        }

        assert!(env.is_finished());
    }

    #[test]
    fn test_adsr_setters() {
        let mut env = ADSREnvelope::default_adsr(44100);
        
        env.set_attack(0.5);
        assert_eq!(env.attack(), 0.5);
        
        env.set_decay(0.3);
        assert_eq!(env.decay(), 0.3);
        
        env.set_sustain(0.6);
        assert_eq!(env.sustain(), 0.6);
        
        env.set_release(1.0);
        assert_eq!(env.release_time(), 1.0);
    }
}

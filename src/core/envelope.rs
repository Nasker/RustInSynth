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

    /// Set attack time in seconds (if supported)
    fn set_attack(&mut self, _attack_time: f32) {}

    /// Set release time in seconds (if supported)
    fn set_release(&mut self, _release_time: f32) {}

    /// Get attack time in seconds
    fn attack(&self) -> f32 { 0.0 }

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
            attack_time,
            decay_time,
            sustain_level: sustain_level.clamp(0.0, 1.0),
            release_time,
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

    fn update_increments(&mut self) {
        let samples_per_second = self.sample_rate as f32;
        self.attack_increment = 1.0 / (self.attack_time.max(0.001) * samples_per_second);
        self.decay_decrement =
            (1.0 - self.sustain_level) / (self.decay_time.max(0.001) * samples_per_second);
        self.release_decrement =
            self.sustain_level / (self.release_time.max(0.001) * samples_per_second);
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
}

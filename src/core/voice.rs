use super::envelope::{AREnvelope, Envelope, EnvelopeState};
use super::event::{NoteEvent, SynthEventKind, SynthEventReceiver, WaveformType};
use super::filter::{Filter, SVFilter, cc_to_cutoff, cc_to_resonance};
use super::oscillator::{Oscillator, SineOscillator, SquareOscillator, SawOscillator, TriangleOscillator};
use super::params::{CCMapping, SynthParam, cc_to_time};
use super::types::{midi_to_frequency, Amplitude, Frequency, MidiNote, Sample, SampleRate};

/// Envelope time range constants
pub const MIN_ATTACK_TIME: f32 = 0.001;  // 1ms
pub const MAX_ATTACK_TIME: f32 = 2.0;    // 2 seconds
pub const MIN_RELEASE_TIME: f32 = 0.001; // 1ms  
pub const MAX_RELEASE_TIME: f32 = 5.0;   // 5 seconds

/// A single synthesizer voice containing an oscillator, filter, and envelope
pub struct Voice {
    oscillator: Box<dyn Oscillator>,
    filter: Box<dyn Filter>,
    envelope: Box<dyn Envelope>,
    current_note: Option<MidiNote>,
    velocity: Amplitude,
    sample_rate: SampleRate,
    waveform: WaveformType,
}

impl Voice {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            oscillator: Box::new(SineOscillator::new(440.0, sample_rate)),
            filter: Box::new(SVFilter::new(20000.0, 0.0, sample_rate)),
            envelope: Box::new(AREnvelope::new(0.01, 0.1, sample_rate)),
            current_note: None,
            velocity: 1.0,
            sample_rate,
            waveform: WaveformType::Sine,
        }
    }

    /// Set the waveform type for this voice
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.waveform = waveform;
        let freq = self.oscillator.frequency();
        self.oscillator = match waveform {
            WaveformType::Sine => Box::new(SineOscillator::new(freq, self.sample_rate)),
            WaveformType::Square => Box::new(SquareOscillator::new(freq, self.sample_rate)),
            WaveformType::Saw => Box::new(SawOscillator::new(freq, self.sample_rate)),
            WaveformType::Triangle => Box::new(TriangleOscillator::new(freq, self.sample_rate)),
        };
    }

    /// Get the current waveform type
    pub fn waveform(&self) -> WaveformType {
        self.waveform
    }

    pub fn with_oscillator<O: Oscillator + 'static>(mut self, oscillator: O) -> Self {
        self.oscillator = Box::new(oscillator);
        self
    }

    pub fn with_envelope<E: Envelope + 'static>(mut self, envelope: E) -> Self {
        self.envelope = Box::new(envelope);
        self
    }

    /// Generate the next sample from this voice
    pub fn next_sample(&mut self) -> Sample {
        if self.envelope.is_finished() {
            return 0.0;
        }

        let osc_sample = self.oscillator.next_sample();
        let filtered_sample = self.filter.process(osc_sample);
        let env_amplitude = self.envelope.next_amplitude();

        filtered_sample * env_amplitude * self.velocity
    }

    /// Trigger a note on this voice
    pub fn note_on(&mut self, note: MidiNote, velocity: Amplitude) {
        self.current_note = Some(note);
        self.velocity = velocity;
        self.oscillator.set_frequency(midi_to_frequency(note));
        self.oscillator.reset();
        self.envelope.trigger();
    }

    /// Release the current note
    pub fn note_off(&mut self) {
        self.envelope.release();
    }

    /// Check if this voice is currently playing
    pub fn is_active(&self) -> bool {
        !self.envelope.is_finished()
    }

    /// Check if this voice is in release phase
    pub fn is_releasing(&self) -> bool {
        self.envelope.state() == EnvelopeState::Release
    }

    /// Get the note currently assigned to this voice
    pub fn current_note(&self) -> Option<MidiNote> {
        self.current_note
    }

    /// Reset the voice to initial state
    pub fn reset(&mut self) {
        self.oscillator.reset();
        self.filter.reset();
        self.envelope.reset();
        self.current_note = None;
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.oscillator.set_sample_rate(sample_rate);
        self.filter.set_sample_rate(sample_rate);
        self.envelope.set_sample_rate(sample_rate);
    }

    /// Set the attack time
    pub fn set_attack(&mut self, attack_time: f32) {
        self.envelope.set_attack(attack_time);
    }

    /// Set the release time
    pub fn set_release(&mut self, release_time: f32) {
        self.envelope.set_release(release_time);
    }

    /// Set the filter cutoff frequency
    pub fn set_filter_cutoff(&mut self, cutoff: Frequency) {
        self.filter.set_cutoff(cutoff);
    }

    /// Set the filter resonance
    pub fn set_filter_resonance(&mut self, resonance: f32) {
        self.filter.set_resonance(resonance);
    }
}

/// Manages multiple voices for polyphonic playback
/// Currently configured for monophonic operation but ready for polyphony
pub struct VoiceManager {
    voices: Vec<Voice>,
    max_voices: usize,
    sample_rate: SampleRate,
    master_volume: Amplitude,
    current_waveform: WaveformType,
    cc_mapping: CCMapping,
    attack_time: f32,
    release_time: f32,
    filter_cutoff: Frequency,
    filter_resonance: f32,
}

impl VoiceManager {
    pub fn new(max_voices: usize, sample_rate: SampleRate) -> Self {
        let voices = (0..max_voices)
            .map(|_| Voice::new(sample_rate))
            .collect();

        Self {
            voices,
            max_voices,
            sample_rate,
            master_volume: 0.5,
            current_waveform: WaveformType::Sine,
            cc_mapping: CCMapping::default_mappings(),
            attack_time: 0.01,
            release_time: 0.1,
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
        }
    }

    /// Create a monophonic voice manager (single voice)
    pub fn monophonic(sample_rate: SampleRate) -> Self {
        Self::new(1, sample_rate)
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_master_volume(&mut self, volume: Amplitude) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Generate the next mixed sample from all active voices
    pub fn next_sample(&mut self) -> Sample {
        let mut mixed_sample: Sample = 0.0;

        for voice in &mut self.voices {
            if voice.is_active() {
                mixed_sample += voice.next_sample();
            }
        }

        // Apply master volume and soft clipping
        let output = mixed_sample * self.master_volume;
        soft_clip(output)
    }

    /// Find a free voice or steal the oldest releasing voice
    fn allocate_voice_index(&self) -> Option<usize> {
        // First, try to find an inactive voice
        if let Some(idx) = self.voices.iter().position(|v| !v.is_active()) {
            return Some(idx);
        }

        // Then, try to find a releasing voice
        if let Some(idx) = self.voices.iter().position(|v| v.is_releasing()) {
            return Some(idx);
        }

        // Finally, steal the first voice (simple voice stealing)
        if !self.voices.is_empty() {
            Some(0)
        } else {
            None
        }
    }

    /// Find the voice playing a specific note
    fn find_voice_with_note(&mut self, note: MidiNote) -> Option<&mut Voice> {
        self.voices
            .iter_mut()
            .find(|v| v.current_note() == Some(note) && v.is_active())
    }

    /// Set the sample rate for all voices
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices {
            voice.set_sample_rate(sample_rate);
        }
    }

    /// Configure all voices with a specific oscillator type
    pub fn configure_voices<F>(&mut self, mut voice_factory: F)
    where
        F: FnMut(SampleRate) -> Voice,
    {
        self.voices = (0..self.max_voices)
            .map(|_| voice_factory(self.sample_rate))
            .collect();
    }

    /// Set the waveform for all voices
    pub fn set_waveform(&mut self, waveform: WaveformType) {
        self.current_waveform = waveform;
        for voice in &mut self.voices {
            voice.set_waveform(waveform);
        }
    }

    /// Get the current waveform type
    pub fn waveform(&self) -> WaveformType {
        self.current_waveform
    }

    /// Set attack time for all voices
    pub fn set_attack(&mut self, attack_time: f32) {
        self.attack_time = attack_time;
        for voice in &mut self.voices {
            voice.set_attack(attack_time);
        }
    }

    /// Set release time for all voices
    pub fn set_release(&mut self, release_time: f32) {
        self.release_time = release_time;
        for voice in &mut self.voices {
            voice.set_release(release_time);
        }
    }

    /// Get current attack time
    pub fn attack(&self) -> f32 {
        self.attack_time
    }

    /// Get current release time
    pub fn release_time(&self) -> f32 {
        self.release_time
    }

    /// Set filter cutoff for all voices
    pub fn set_filter_cutoff(&mut self, cutoff: Frequency) {
        self.filter_cutoff = cutoff;
        for voice in &mut self.voices {
            voice.set_filter_cutoff(cutoff);
        }
    }

    /// Set filter resonance for all voices
    pub fn set_filter_resonance(&mut self, resonance: f32) {
        self.filter_resonance = resonance;
        for voice in &mut self.voices {
            voice.set_filter_resonance(resonance);
        }
    }

    /// Get current filter cutoff
    pub fn filter_cutoff(&self) -> Frequency {
        self.filter_cutoff
    }

    /// Get current filter resonance
    pub fn filter_resonance(&self) -> f32 {
        self.filter_resonance
    }

    /// Get a reference to the CC mapping
    pub fn cc_mapping(&self) -> &CCMapping {
        &self.cc_mapping
    }

    /// Get a mutable reference to the CC mapping
    pub fn cc_mapping_mut(&mut self) -> &mut CCMapping {
        &mut self.cc_mapping
    }

    /// Handle a parameter change from CC
    fn handle_param_change(&mut self, param: SynthParam, value: u8) {
        match param {
            SynthParam::Waveform => {
                let waveform = WaveformType::from_index(value / 32);
                self.set_waveform(waveform);
            }
            SynthParam::Attack => {
                let time = cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME);
                self.set_attack(time);
            }
            SynthParam::Release => {
                let time = cc_to_time(value, MIN_RELEASE_TIME, MAX_RELEASE_TIME);
                self.set_release(time);
            }
            SynthParam::FilterCutoff => {
                let cutoff = cc_to_cutoff(value);
                self.set_filter_cutoff(cutoff);
            }
            SynthParam::FilterResonance => {
                let resonance = cc_to_resonance(value);
                self.set_filter_resonance(resonance);
            }
        }
    }
}

impl SynthEventReceiver for VoiceManager {
    fn receive_event(&mut self, event: NoteEvent) {
        match event.kind {
            SynthEventKind::NoteOn => {
                if let Some(idx) = self.allocate_voice_index() {
                    // Ensure voice has current waveform before playing
                    self.voices[idx].set_waveform(self.current_waveform);
                    self.voices[idx].note_on(event.note, event.velocity);
                }
            }
            SynthEventKind::NoteOff => {
                if let Some(voice) = self.find_voice_with_note(event.note) {
                    voice.note_off();
                }
            }
            SynthEventKind::WaveformChange(waveform) => {
                self.set_waveform(waveform);
            }
            SynthEventKind::ControlChange { cc, value } => {
                if let Some(param) = self.cc_mapping.get_param(cc) {
                    self.handle_param_change(param, value);
                }
            }
        }
    }
}

/// Soft clipping function to prevent harsh digital distortion
fn soft_clip(sample: Sample) -> Sample {
    if sample > 1.0 {
        1.0 - (-sample + 1.0).exp() * 0.5
    } else if sample < -1.0 {
        -1.0 + (sample + 1.0).exp() * 0.5
    } else {
        sample
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_lifecycle() {
        let mut voice = Voice::new(44100);

        assert!(!voice.is_active());

        voice.note_on(69, 1.0); // A4
        assert!(voice.is_active());

        // Generate some samples
        for _ in 0..100 {
            let _ = voice.next_sample();
        }

        voice.note_off();
        assert!(voice.is_releasing());
    }

    #[test]
    fn test_voice_manager_monophonic() {
        let mut vm = VoiceManager::monophonic(44100);

        vm.receive_event(NoteEvent::note_on(69, 1.0));

        // Generate several samples to get past initial attack
        let mut has_nonzero = false;
        for _ in 0..100 {
            let sample = vm.next_sample();
            if sample != 0.0 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero, "Voice manager should produce non-zero samples");
    }
}

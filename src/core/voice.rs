use super::envelope::{AREnvelope, Envelope, EnvelopeState};
use super::event::{NoteEvent, SynthEventKind, SynthEventReceiver, WaveformType};
use super::filter::{Filter, SVFilter, cc_to_cutoff, cc_to_resonance};
use super::oscillator::{Oscillator, OscillatorBank};
use super::params::{CCMapping, SynthParam, cc_to_time, cc_to_level, cc_to_semitones, cc_to_cents, cc_to_waveform, cc_to_phase};
use super::types::{midi_to_frequency, Amplitude, Frequency, MidiNote, Sample, SampleRate};

/// Envelope time range constants
pub const MIN_ATTACK_TIME: f32 = 0.001;  // 1ms
pub const MAX_ATTACK_TIME: f32 = 2.0;    // 2 seconds
pub const MIN_RELEASE_TIME: f32 = 0.001; // 1ms  
pub const MAX_RELEASE_TIME: f32 = 5.0;   // 5 seconds

/// A single synthesizer voice containing an oscillator bank, filter, and envelope
pub struct Voice {
    osc_bank: OscillatorBank,
    filter: Box<dyn Filter>,
    envelope: Box<dyn Envelope>,
    current_note: Option<MidiNote>,
    velocity: Amplitude,
    sample_rate: SampleRate,
}

impl Voice {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            osc_bank: OscillatorBank::new(sample_rate),
            filter: Box::new(SVFilter::new(20000.0, 0.0, sample_rate)),
            envelope: Box::new(AREnvelope::new(0.01, 0.1, sample_rate)),
            current_note: None,
            velocity: 1.0,
            sample_rate,
        }
    }

    /// Get mutable reference to the oscillator bank
    pub fn osc_bank_mut(&mut self) -> &mut OscillatorBank {
        &mut self.osc_bank
    }

    /// Get reference to the oscillator bank
    pub fn osc_bank(&self) -> &OscillatorBank {
        &self.osc_bank
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

        let osc_sample = self.osc_bank.next_sample();
        let filtered_sample = self.filter.process(osc_sample);
        let env_amplitude = self.envelope.next_amplitude();

        filtered_sample * env_amplitude * self.velocity
    }

    /// Trigger a note on this voice
    pub fn note_on(&mut self, note: MidiNote, velocity: Amplitude) {
        self.current_note = Some(note);
        self.velocity = velocity;
        self.osc_bank.set_frequency(midi_to_frequency(note));
        self.osc_bank.reset();
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
        self.osc_bank.reset();
        self.filter.reset();
        self.envelope.reset();
        self.current_note = None;
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.osc_bank.set_sample_rate(sample_rate);
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

/// Oscillator bank state for VoiceManager
#[derive(Clone)]
pub struct OscBankState {
    pub osc1_waveform: WaveformType,
    pub osc1_level: f32,
    pub osc1_phase: f32,
    pub osc2_waveform: WaveformType,
    pub osc2_level: f32,
    pub osc2_semitones: i8,
    pub osc2_cents: i8,
    pub osc2_phase: f32,
    pub osc3_waveform: WaveformType,
    pub osc3_level: f32,
    pub osc3_semitones: i8,
    pub osc3_cents: i8,
    pub osc3_phase: f32,
}

impl Default for OscBankState {
    fn default() -> Self {
        Self {
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Saw,
            osc2_level: 0.8,
            osc2_semitones: 0,
            osc2_cents: 7,  // Slight detune for fatness
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Square,
            osc3_level: 0.5,
            osc3_semitones: -12,  // Sub oscillator
            osc3_cents: 0,
            osc3_phase: 0.0,
        }
    }
}

/// Manages multiple voices for polyphonic playback
/// Currently configured for monophonic operation but ready for polyphony
pub struct VoiceManager {
    voices: Vec<Voice>,
    max_voices: usize,
    sample_rate: SampleRate,
    master_volume: Amplitude,
    cc_mapping: CCMapping,
    attack_time: f32,
    release_time: f32,
    filter_cutoff: Frequency,
    filter_resonance: f32,
    osc_state: OscBankState,
}

impl VoiceManager {
    pub fn new(max_voices: usize, sample_rate: SampleRate) -> Self {
        let osc_state = OscBankState::default();
        let mut voices: Vec<Voice> = (0..max_voices)
            .map(|_| Voice::new(sample_rate))
            .collect();
        
        // Apply default oscillator bank state to all voices
        for voice in &mut voices {
            Self::apply_osc_state_to_voice(voice, &osc_state);
        }

        Self {
            voices,
            max_voices,
            sample_rate,
            master_volume: 0.5,
            cc_mapping: CCMapping::default_mappings(),
            attack_time: 0.01,
            release_time: 0.1,
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
            osc_state,
        }
    }

    /// Apply oscillator state to a voice
    fn apply_osc_state_to_voice(voice: &mut Voice, state: &OscBankState) {
        let bank = voice.osc_bank_mut();
        bank.set_waveform(1, state.osc1_waveform);
        bank.set_level(1, state.osc1_level);
        bank.set_phase(1, state.osc1_phase);
        bank.set_waveform(2, state.osc2_waveform);
        bank.set_level(2, state.osc2_level);
        bank.set_detune(2, state.osc2_semitones, state.osc2_cents);
        bank.set_phase(2, state.osc2_phase);
        bank.set_waveform(3, state.osc3_waveform);
        bank.set_level(3, state.osc3_level);
        bank.set_detune(3, state.osc3_semitones, state.osc3_cents);
        bank.set_phase(3, state.osc3_phase);
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
        // Re-apply oscillator state to new voices
        for voice in &mut self.voices {
            Self::apply_osc_state_to_voice(voice, &self.osc_state);
        }
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

    /// Get current oscillator bank state
    pub fn osc_state(&self) -> &OscBankState {
        &self.osc_state
    }

    /// Get a reference to the CC mapping
    pub fn cc_mapping(&self) -> &CCMapping {
        &self.cc_mapping
    }

    /// Get a mutable reference to the CC mapping
    pub fn cc_mapping_mut(&mut self) -> &mut CCMapping {
        &mut self.cc_mapping
    }

    /// Set oscillator waveform
    pub fn set_osc_waveform(&mut self, osc_num: u8, waveform: WaveformType) {
        match osc_num {
            1 => self.osc_state.osc1_waveform = waveform,
            2 => self.osc_state.osc2_waveform = waveform,
            3 => self.osc_state.osc3_waveform = waveform,
            _ => return,
        }
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_waveform(osc_num, waveform);
        }
    }

    /// Set oscillator level
    pub fn set_osc_level(&mut self, osc_num: u8, level: f32) {
        match osc_num {
            1 => self.osc_state.osc1_level = level,
            2 => self.osc_state.osc2_level = level,
            3 => self.osc_state.osc3_level = level,
            _ => return,
        }
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_level(osc_num, level);
        }
    }

    /// Set oscillator detune (semitones and cents)
    pub fn set_osc_detune(&mut self, osc_num: u8, semitones: i8, cents: i8) {
        match osc_num {
            2 => {
                self.osc_state.osc2_semitones = semitones;
                self.osc_state.osc2_cents = cents;
            }
            3 => {
                self.osc_state.osc3_semitones = semitones;
                self.osc_state.osc3_cents = cents;
            }
            _ => return,
        }
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_detune(osc_num, semitones, cents);
        }
    }

    /// Set oscillator semitones only
    pub fn set_osc_semitones(&mut self, osc_num: u8, semitones: i8) {
        let cents = match osc_num {
            2 => self.osc_state.osc2_cents,
            3 => self.osc_state.osc3_cents,
            _ => return,
        };
        self.set_osc_detune(osc_num, semitones, cents);
    }

    /// Set oscillator cents only
    pub fn set_osc_cents(&mut self, osc_num: u8, cents: i8) {
        let semitones = match osc_num {
            2 => self.osc_state.osc2_semitones,
            3 => self.osc_state.osc3_semitones,
            _ => return,
        };
        self.set_osc_detune(osc_num, semitones, cents);
    }

    /// Set oscillator phase offset (0.0 to 1.0)
    pub fn set_osc_phase(&mut self, osc_num: u8, phase: f32) {
        match osc_num {
            1 => self.osc_state.osc1_phase = phase,
            2 => self.osc_state.osc2_phase = phase,
            3 => self.osc_state.osc3_phase = phase,
            _ => return,
        }
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_phase(osc_num, phase);
        }
    }

    /// Handle a parameter change from CC
    fn handle_param_change(&mut self, param: SynthParam, value: u8) {
        match param {
            // Envelope
            SynthParam::Attack => {
                let time = cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME);
                self.set_attack(time);
            }
            SynthParam::Release => {
                let time = cc_to_time(value, MIN_RELEASE_TIME, MAX_RELEASE_TIME);
                self.set_release(time);
            }
            
            // Filter
            SynthParam::FilterCutoff => {
                let cutoff = cc_to_cutoff(value);
                self.set_filter_cutoff(cutoff);
            }
            SynthParam::FilterResonance => {
                let resonance = cc_to_resonance(value);
                self.set_filter_resonance(resonance);
            }
            
            // Oscillator 1
            SynthParam::Osc1Waveform => {
                let waveform = WaveformType::from_index(cc_to_waveform(value));
                self.set_osc_waveform(1, waveform);
            }
            SynthParam::Osc1Level => {
                self.set_osc_level(1, cc_to_level(value));
            }
            SynthParam::Osc1Phase => {
                self.set_osc_phase(1, cc_to_phase(value));
            }
            
            // Oscillator 2
            SynthParam::Osc2Waveform => {
                let waveform = WaveformType::from_index(cc_to_waveform(value));
                self.set_osc_waveform(2, waveform);
            }
            SynthParam::Osc2Level => {
                self.set_osc_level(2, cc_to_level(value));
            }
            SynthParam::Osc2Semitones => {
                self.set_osc_semitones(2, cc_to_semitones(value));
            }
            SynthParam::Osc2Cents => {
                self.set_osc_cents(2, cc_to_cents(value));
            }
            SynthParam::Osc2Phase => {
                self.set_osc_phase(2, cc_to_phase(value));
            }
            
            // Oscillator 3
            SynthParam::Osc3Waveform => {
                let waveform = WaveformType::from_index(cc_to_waveform(value));
                self.set_osc_waveform(3, waveform);
            }
            SynthParam::Osc3Level => {
                self.set_osc_level(3, cc_to_level(value));
            }
            SynthParam::Osc3Semitones => {
                self.set_osc_semitones(3, cc_to_semitones(value));
            }
            SynthParam::Osc3Cents => {
                self.set_osc_cents(3, cc_to_cents(value));
            }
            SynthParam::Osc3Phase => {
                self.set_osc_phase(3, cc_to_phase(value));
            }
        }
    }
}

impl SynthEventReceiver for VoiceManager {
    fn receive_event(&mut self, event: NoteEvent) {
        match event.kind {
            SynthEventKind::NoteOn => {
                if let Some(idx) = self.allocate_voice_index() {
                    self.voices[idx].note_on(event.note, event.velocity);
                }
            }
            SynthEventKind::NoteOff => {
                if let Some(voice) = self.find_voice_with_note(event.note) {
                    voice.note_off();
                }
            }
            SynthEventKind::WaveformChange(waveform) => {
                // Legacy: set all oscillators to the same waveform
                self.set_osc_waveform(1, waveform);
                self.set_osc_waveform(2, waveform);
                self.set_osc_waveform(3, waveform);
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

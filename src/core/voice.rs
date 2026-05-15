use super::envelope::{ADSREnvelope, Envelope, EnvelopeState};
use super::event::{NoteEvent, SynthEventKind, SynthEventReceiver, WaveformType};
use super::filter::{Filter, SVFilter, cc_to_cutoff, cc_to_resonance};
use super::lfo::{LFO, LfoDestination, LfoWaveform};
use super::oscillator::{Oscillator, OscillatorBank};
use super::params::{CCMapping, SynthParam, cc_to_time, cc_to_level, cc_to_semitones, cc_to_cents, cc_to_waveform, cc_to_phase, cc_to_sustain, cc_to_pitch_bend_range, cc_to_filter_env_amount, cc_to_lfo_rate, cc_to_lfo_depth, cc_to_lfo_waveform, cc_to_lfo_destination};
use super::types::{midi_to_frequency, Amplitude, Frequency, MidiNote, Sample, SampleRate};

/// Envelope time range constants
pub const MIN_ATTACK_TIME: f32 = 0.001;  // 1ms
pub const MAX_ATTACK_TIME: f32 = 2.0;    // 2 seconds
pub const MIN_DECAY_TIME: f32 = 0.001;   // 1ms
pub const MAX_DECAY_TIME: f32 = 5.0;     // 5 seconds
pub const MIN_RELEASE_TIME: f32 = 0.001; // 1ms  
pub const MAX_RELEASE_TIME: f32 = 5.0;   // 5 seconds

/// A single synthesizer voice containing an oscillator bank, filter, envelopes, and LFO
pub struct Voice {
    osc_bank: OscillatorBank,
    filter: Box<dyn Filter>,
    envelope: Box<dyn Envelope>,       // Amplitude envelope (VCA)
    filter_envelope: Box<dyn Envelope>, // Filter envelope (VCF)
    lfo: LFO,                          // Low frequency oscillator for modulation
    current_note: Option<MidiNote>,
    velocity: Amplitude,
    sample_rate: SampleRate,
    // Filter envelope modulation
    base_cutoff: Frequency,
    filter_env_amount: f32,            // 0.0 to 1.0 (amount of envelope applied)
    // LFO modulation
    base_frequency: Frequency,         // Base note frequency for pitch modulation
}

impl Voice {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            osc_bank: OscillatorBank::new(sample_rate),
            filter: Box::new(SVFilter::new(20000.0, 0.0, sample_rate)),
            envelope: Box::new(ADSREnvelope::default_adsr(sample_rate)),
            filter_envelope: Box::new(ADSREnvelope::new(0.01, 0.3, 0.0, 0.3, sample_rate)), // Quick decay for pluck
            lfo: LFO::new(sample_rate),
            current_note: None,
            velocity: 1.0,
            sample_rate,
            base_cutoff: 20000.0,
            filter_env_amount: 0.0, // Off by default
            base_frequency: 440.0,
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

        // Get LFO value for this sample
        let lfo_value = self.lfo.next_value();

        // Apply LFO pitch modulation (vibrato) if enabled
        if self.lfo.destination() == LfoDestination::Pitch && lfo_value != 0.0 {
            // Vibrato: modulate pitch by up to ±1 semitone at full depth
            let vibrato_semitones = lfo_value; // -1.0 to +1.0 semitones
            let vibrato_ratio = 2.0_f32.powf(vibrato_semitones / 12.0);
            let modulated_freq = self.base_frequency * vibrato_ratio;
            self.osc_bank.set_frequency(modulated_freq);
        }

        // Calculate filter envelope and LFO modulation
        let filter_env_amp = self.filter_envelope.next_amplitude();
        let filter_lfo = if self.lfo.destination() == LfoDestination::FilterCutoff {
            lfo_value // -1.0 to +1.0
        } else {
            0.0
        };

        let modulated_cutoff = if self.filter_env_amount > 0.0 || filter_lfo != 0.0 {
            // Combine envelope and LFO modulation
            let max_cutoff = 20000.0f32;
            let cutoff_range = max_cutoff - self.base_cutoff;

            // Envelope opens filter upward
            let env_modulation = cutoff_range * self.filter_env_amount * filter_env_amp;

            // LFO can modulate up or down (±50% of remaining range at full depth)
            let lfo_depth = self.lfo.depth();
            let lfo_modulation = cutoff_range * lfo_depth * filter_lfo * 0.5;

            let target_cutoff = self.base_cutoff + env_modulation + lfo_modulation;
            target_cutoff.min(max_cutoff).max(20.0)
        } else {
            self.base_cutoff
        };
        self.filter.set_cutoff(modulated_cutoff);

        let osc_sample = self.osc_bank.next_sample();
        let filtered_sample = self.filter.process(osc_sample);
        let env_amplitude = self.envelope.next_amplitude();

        // Apply LFO amplitude modulation (tremolo) if enabled
        let lfo_amp = if self.lfo.destination() == LfoDestination::Amplitude {
            // Tremolo: 1.0 ± depth (never goes negative)
            1.0 + lfo_value * 0.5 // 0.5 to 1.5 range at full depth
        } else {
            1.0
        };

        filtered_sample * env_amplitude * self.velocity * lfo_amp
    }

    /// Trigger a note on this voice
    pub fn note_on(&mut self, note: MidiNote, velocity: Amplitude) {
        self.current_note = Some(note);
        self.velocity = velocity;
        let freq = midi_to_frequency(note);
        self.base_frequency = freq;
        self.osc_bank.set_frequency(freq);
        self.osc_bank.reset();
        self.envelope.trigger();
        self.filter_envelope.trigger();
        self.lfo.reset();
    }

    /// Release the current note
    pub fn note_off(&mut self) {
        self.envelope.release();
        self.filter_envelope.release();
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
        self.filter_envelope.reset();
        self.lfo.reset();
        self.current_note = None;
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.osc_bank.set_sample_rate(sample_rate);
        self.filter.set_sample_rate(sample_rate);
        self.envelope.set_sample_rate(sample_rate);
        self.filter_envelope.set_sample_rate(sample_rate);
        self.lfo.set_sample_rate(sample_rate);
    }

    /// Set LFO rate (0.1 to 20 Hz)
    pub fn set_lfo_rate(&mut self, rate: f32) {
        self.lfo.set_rate(rate);
    }

    /// Set LFO depth (0.0 to 1.0)
    pub fn set_lfo_depth(&mut self, depth: f32) {
        self.lfo.set_depth(depth);
    }

    /// Set LFO waveform
    pub fn set_lfo_waveform(&mut self, waveform: LfoWaveform) {
        self.lfo.set_waveform(waveform);
    }

    /// Set LFO destination
    pub fn set_lfo_destination(&mut self, destination: LfoDestination) {
        self.lfo.set_destination(destination);
    }

    /// Get LFO rate
    pub fn lfo_rate(&self) -> f32 {
        self.lfo.rate()
    }

    /// Get LFO depth
    pub fn lfo_depth(&self) -> f32 {
        self.lfo.depth()
    }

    /// Get LFO waveform
    pub fn lfo_waveform(&self) -> LfoWaveform {
        self.lfo.waveform()
    }

    /// Get LFO destination
    pub fn lfo_destination(&self) -> LfoDestination {
        self.lfo.destination()
    }

    /// Set the filter envelope attack time
    pub fn set_filter_attack(&mut self, attack_time: f32) {
        self.filter_envelope.set_attack(attack_time);
    }

    /// Set the filter envelope decay time
    pub fn set_filter_decay(&mut self, decay_time: f32) {
        self.filter_envelope.set_decay(decay_time);
    }

    /// Set the filter envelope sustain level
    pub fn set_filter_sustain(&mut self, sustain_level: f32) {
        self.filter_envelope.set_sustain(sustain_level);
    }

    /// Set the filter envelope release time
    pub fn set_filter_release(&mut self, release_time: f32) {
        self.filter_envelope.set_release(release_time);
    }

    /// Set the filter envelope amount (0.0 to 1.0)
    pub fn set_filter_env_amount(&mut self, amount: f32) {
        self.filter_env_amount = amount.clamp(0.0, 1.0);
    }

    /// Get the filter envelope amount
    pub fn filter_env_amount(&self) -> f32 {
        self.filter_env_amount
    }

    /// Set the base filter cutoff (also updates base_cutoff for envelope modulation)
    pub fn set_filter_cutoff(&mut self, cutoff: Frequency) {
        self.base_cutoff = cutoff;
        self.filter.set_cutoff(cutoff);
    }

    /// Set the attack time
    pub fn set_attack(&mut self, attack_time: f32) {
        self.envelope.set_attack(attack_time);
    }

    /// Set the decay time
    pub fn set_decay(&mut self, decay_time: f32) {
        self.envelope.set_decay(decay_time);
    }

    /// Set the sustain level (0.0 to 1.0)
    pub fn set_sustain(&mut self, sustain_level: f32) {
        self.envelope.set_sustain(sustain_level);
    }

    /// Set the release time
    pub fn set_release(&mut self, release_time: f32) {
        self.envelope.set_release(release_time);
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
    // Amplitude ADSR envelope state
    attack_time: f32,
    decay_time: f32,
    sustain_level: f32,
    release_time: f32,
    // Filter envelope state
    filter_attack_time: f32,
    filter_decay_time: f32,
    filter_sustain_level: f32,
    filter_release_time: f32,
    filter_env_amount: f32,
    // Filter state
    filter_cutoff: Frequency,
    filter_resonance: f32,
    // LFO state
    lfo_rate: f32,
    lfo_depth: f32,
    lfo_waveform: LfoWaveform,
    lfo_destination: LfoDestination,
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
            // Amp envelope defaults
            attack_time: 0.01,
            decay_time: 0.1,
            sustain_level: 0.7,
            release_time: 0.2,
            // Filter envelope defaults (quick pluck by default)
            filter_attack_time: 0.01,
            filter_decay_time: 0.3,
            filter_sustain_level: 0.0,
            filter_release_time: 0.3,
            filter_env_amount: 0.0, // Off by default
            // Filter state
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,
            // LFO defaults (off by default)
            lfo_rate: 6.0,
            lfo_depth: 0.0,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Off,
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

    /// Set decay time for all voices
    pub fn set_decay(&mut self, decay_time: f32) {
        self.decay_time = decay_time;
        for voice in &mut self.voices {
            voice.set_decay(decay_time);
        }
    }

    /// Set sustain level for all voices (0.0 to 1.0)
    pub fn set_sustain(&mut self, sustain_level: f32) {
        self.sustain_level = sustain_level;
        for voice in &mut self.voices {
            voice.set_sustain(sustain_level);
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

    /// Get current decay time
    pub fn decay(&self) -> f32 {
        self.decay_time
    }

    /// Get current sustain level
    pub fn sustain(&self) -> f32 {
        self.sustain_level
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

    /// Set filter envelope attack time for all voices
    pub fn set_filter_attack(&mut self, attack_time: f32) {
        self.filter_attack_time = attack_time;
        for voice in &mut self.voices {
            voice.set_filter_attack(attack_time);
        }
    }

    /// Set filter envelope decay time for all voices
    pub fn set_filter_decay(&mut self, decay_time: f32) {
        self.filter_decay_time = decay_time;
        for voice in &mut self.voices {
            voice.set_filter_decay(decay_time);
        }
    }

    /// Set filter envelope sustain level for all voices
    pub fn set_filter_sustain(&mut self, sustain_level: f32) {
        self.filter_sustain_level = sustain_level;
        for voice in &mut self.voices {
            voice.set_filter_sustain(sustain_level);
        }
    }

    /// Set filter envelope release time for all voices
    pub fn set_filter_release(&mut self, release_time: f32) {
        self.filter_release_time = release_time;
        for voice in &mut self.voices {
            voice.set_filter_release(release_time);
        }
    }

    /// Set filter envelope amount for all voices (0.0 to 1.0)
    pub fn set_filter_env_amount(&mut self, amount: f32) {
        self.filter_env_amount = amount.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            voice.set_filter_env_amount(self.filter_env_amount);
        }
    }

    /// Get current filter envelope attack time
    pub fn filter_attack(&self) -> f32 {
        self.filter_attack_time
    }

    /// Get current filter envelope decay time
    pub fn filter_decay(&self) -> f32 {
        self.filter_decay_time
    }

    /// Get current filter envelope sustain level
    pub fn filter_sustain(&self) -> f32 {
        self.filter_sustain_level
    }

    /// Get current filter envelope release time
    pub fn filter_release_time(&self) -> f32 {
        self.filter_release_time
    }

    /// Get current filter envelope amount
    pub fn filter_env_amount(&self) -> f32 {
        self.filter_env_amount
    }

    /// Set LFO rate for all voices (0.1 to 20 Hz)
    pub fn set_lfo_rate(&mut self, rate: f32) {
        self.lfo_rate = rate.clamp(0.1, 20.0);
        for voice in &mut self.voices {
            voice.set_lfo_rate(self.lfo_rate);
        }
    }

    /// Set LFO depth for all voices (0.0 to 1.0)
    pub fn set_lfo_depth(&mut self, depth: f32) {
        self.lfo_depth = depth.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            voice.set_lfo_depth(self.lfo_depth);
        }
    }

    /// Set LFO waveform for all voices
    pub fn set_lfo_waveform(&mut self, waveform: LfoWaveform) {
        self.lfo_waveform = waveform;
        for voice in &mut self.voices {
            voice.set_lfo_waveform(waveform);
        }
    }

    /// Set LFO destination for all voices
    pub fn set_lfo_destination(&mut self, destination: LfoDestination) {
        self.lfo_destination = destination;
        for voice in &mut self.voices {
            voice.set_lfo_destination(destination);
        }
    }

    /// Get current LFO rate
    pub fn lfo_rate(&self) -> f32 {
        self.lfo_rate
    }

    /// Get current LFO depth
    pub fn lfo_depth(&self) -> f32 {
        self.lfo_depth
    }

    /// Get current LFO waveform
    pub fn lfo_waveform(&self) -> LfoWaveform {
        self.lfo_waveform
    }

    /// Get current LFO destination
    pub fn lfo_destination(&self) -> LfoDestination {
        self.lfo_destination
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

    /// Set pitch bend for all voices (-1.0 to +1.0)
    pub fn set_pitch_bend(&mut self, bend: f32) {
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_pitch_bend(bend);
        }
    }

    /// Set pitch bend range in semitones (1-24)
    pub fn set_pitch_bend_range(&mut self, semitones: u8) {
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_pitch_bend_range(semitones);
        }
    }

    /// Handle a parameter change from CC
    fn handle_param_change(&mut self, param: SynthParam, value: u8) {
        match param {
            // ADSR Envelope
            SynthParam::Attack => {
                let time = cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME);
                self.set_attack(time);
            }
            SynthParam::Decay => {
                let time = cc_to_time(value, MIN_DECAY_TIME, MAX_DECAY_TIME);
                self.set_decay(time);
            }
            SynthParam::Sustain => {
                self.set_sustain(cc_to_sustain(value));
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
            // Filter Envelope
            SynthParam::FilterAttack => {
                let time = cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME);
                self.set_filter_attack(time);
            }
            SynthParam::FilterDecay => {
                let time = cc_to_time(value, MIN_DECAY_TIME, MAX_DECAY_TIME);
                self.set_filter_decay(time);
            }
            SynthParam::FilterSustain => {
                self.set_filter_sustain(cc_to_sustain(value));
            }
            SynthParam::FilterRelease => {
                let time = cc_to_time(value, MIN_RELEASE_TIME, MAX_RELEASE_TIME);
                self.set_filter_release(time);
            }
            SynthParam::FilterEnvAmount => {
                self.set_filter_env_amount(cc_to_filter_env_amount(value));
            }

            // LFO
            SynthParam::LfoRate => {
                self.set_lfo_rate(cc_to_lfo_rate(value));
            }
            SynthParam::LfoDepth => {
                self.set_lfo_depth(cc_to_lfo_depth(value));
            }
            SynthParam::LfoWaveform => {
                let waveform_idx = cc_to_lfo_waveform(value);
                let waveform = match waveform_idx {
                    0 => LfoWaveform::Sine,
                    1 => LfoWaveform::Triangle,
                    2 => LfoWaveform::Square,
                    3 => LfoWaveform::Saw,
                    _ => LfoWaveform::Random,
                };
                self.set_lfo_waveform(waveform);
            }
            SynthParam::LfoDestination => {
                let dest_idx = cc_to_lfo_destination(value);
                let destination = match dest_idx {
                    0 => LfoDestination::Off,
                    1 => LfoDestination::Pitch,
                    2 => LfoDestination::FilterCutoff,
                    _ => LfoDestination::Amplitude,
                };
                self.set_lfo_destination(destination);
            }

            // Pitch
            SynthParam::PitchBendRange => {
                self.set_pitch_bend_range(cc_to_pitch_bend_range(value));
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
            SynthEventKind::PitchBend(bend) => {
                self.set_pitch_bend(bend);
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

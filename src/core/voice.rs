use serde::{Deserialize, Serialize};

use super::envelope::{ADSREnvelope, Envelope, EnvelopeState};
use super::event::{NoteEvent, SynthEventKind, SynthEventReceiver, WaveformType};
use super::filter::{make_filter, Filter, FilterType};
use super::lfo::{LFO, LfoDestination, LfoWaveform};
use super::mod_matrix::{ModDest, ModMatrix, ModSlot, ModSource, ModSourceValues};
use super::oscillator::{Oscillator, OscillatorBank};
use super::param_spec::cc_to_plain;
use super::params::{CCMapping, SynthParam};
use super::presets::Preset;
use super::types::{midi_to_frequency, Amplitude, Frequency, MidiNote, Sample, SampleRate, StereoSample};

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
    filter_left: Box<dyn Filter>,
    filter_right: Box<dyn Filter>,
    filter_type: FilterType,           // Current filter topology
    envelope: Box<dyn Envelope>,       // Amplitude envelope (VCA)
    filter_envelope: Box<dyn Envelope>, // Filter envelope (VCF)
    env3: Box<dyn Envelope>,           // Free modulation envelope
    env4: Box<dyn Envelope>,           // Second free modulation envelope
    lfos: [LFO; 3],                    // Per-voice low frequency oscillators
    mod_matrix: ModMatrix,             // Modulation routing matrix
    mod_wheel: f32,                    // 0.0 to 1.0, fed from CC1
    pitch_bend_mod: f32,               // -1.0 to 1.0, pitch wheel as a mod source
    aftertouch: f32,                   // 0.0 to 1.0, channel pressure
    lfo_base: [(f32, f32); 3],         // Unmodulated (rate Hz, depth) per LFO
    last_values: ModSourceValues,      // Previous sample's sources (mod-on-mod)
    used_dests: u32,                   // Bitmask of ModDest targeted by the matrix
    current_note: Option<MidiNote>,
    velocity: Amplitude,
    sample_rate: SampleRate,
    // Filter envelope modulation
    base_cutoff: Frequency,
    base_resonance: f32,
    filter_env_amount: f32,            // 0.0 to 1.0 (amount of envelope applied)
    // LFO modulation
    base_frequency: Frequency,         // Base note frequency for pitch modulation
    // Portamento (glide)
    portamento_time: f32,              // Glide time in seconds (0.0 = off)
    glide_target_freq: Frequency,       // Target frequency during glide
    glide_active: bool,                 // Whether portamento glide is in progress
}

impl Voice {
    pub fn new(sample_rate: SampleRate) -> Self {
        Self {
            osc_bank: OscillatorBank::new(sample_rate),
            filter_left: make_filter(FilterType::Svf, 20000.0, 0.0, sample_rate),
            filter_right: make_filter(FilterType::Svf, 20000.0, 0.0, sample_rate),
            filter_type: FilterType::Svf,
            envelope: Box::new(ADSREnvelope::default_adsr(sample_rate)),
            filter_envelope: Box::new(ADSREnvelope::new(0.01, 0.3, 0.0, 0.3, sample_rate)), // Quick decay for pluck
            env3: Box::new(ADSREnvelope::new(0.01, 0.3, 0.0, 0.3, sample_rate)),
            env4: Box::new(ADSREnvelope::new(0.01, 0.3, 0.0, 0.3, sample_rate)),
            lfos: [LFO::new(sample_rate), LFO::new(sample_rate), LFO::new(sample_rate)],
            mod_matrix: ModMatrix::default(),
            mod_wheel: 0.0,
            pitch_bend_mod: 0.0,
            aftertouch: 0.0,
            lfo_base: [(1.0, 0.0); 3],
            last_values: ModSourceValues::default(),
            used_dests: 0,
            current_note: None,
            velocity: 1.0,
            sample_rate,
            base_cutoff: 20000.0,
            base_resonance: 0.0,
            filter_env_amount: 0.0, // Off by default
            base_frequency: 440.0,
            portamento_time: 0.0,
            glide_target_freq: 440.0,
            glide_active: false,
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

        // Update portamento glide
        if self.glide_active {
            let step = (self.glide_target_freq - self.base_frequency)
                / (self.portamento_time * self.sample_rate as f32);
            self.base_frequency += step;
            if (self.glide_target_freq - self.base_frequency).abs() <= step.abs() {
                self.base_frequency = self.glide_target_freq;
                self.glide_active = false;
            }
        }

        let mods = self.compute_modulation();
        self.apply_resonance_offset(mods.resonance_offset);

        self.osc_bank.set_frequency(self.base_frequency * 2.0f32.powf(mods.pitch_semitones / 12.0));

        let modulated_cutoff = self.modulated_cutoff(mods.filter_env_amp, mods.cutoff_semitones);
        // For mono output, use left filter (both filters have same parameters)
        self.filter_left.set_cutoff(modulated_cutoff);
        self.filter_right.set_cutoff(modulated_cutoff);

        let (s1, s2, s3) = self.osc_bank.next_samples_individual();
        let osc_sample = s1 * mods.osc_gain[0] + s2 * mods.osc_gain[1] + s3 * mods.osc_gain[2];
        let filtered_sample = self.filter_left.process(osc_sample);

        filtered_sample * mods.amp_env * self.velocity * mods.amplitude_gain
    }
    
    /// Generate the next stereo sample from this voice with per-oscillator panning
    /// pans: (osc1_pan, osc2_pan, osc3_pan) where -1.0 = left, 0.0 = center, 1.0 = right
    pub fn next_sample_stereo(&mut self, pans: (f32, f32, f32)) -> StereoSample {
        if self.envelope.is_finished() {
            return StereoSample::ZERO;
        }

        // Update portamento glide
        if self.glide_active {
            let step = (self.glide_target_freq - self.base_frequency)
                / (self.portamento_time * self.sample_rate as f32);
            self.base_frequency += step;
            if (self.glide_target_freq - self.base_frequency).abs() <= step.abs() {
                self.base_frequency = self.glide_target_freq;
                self.glide_active = false;
            }
        }

        let mods = self.compute_modulation();
        self.apply_resonance_offset(mods.resonance_offset);

        self.osc_bank.set_frequency(self.base_frequency * 2.0f32.powf(mods.pitch_semitones / 12.0));

        let modulated_cutoff = self.modulated_cutoff(mods.filter_env_amp, mods.cutoff_semitones);
        // Set cutoff on both filters for stereo processing
        self.filter_left.set_cutoff(modulated_cutoff);
        self.filter_right.set_cutoff(modulated_cutoff);

        // Get individual oscillator samples and pan them
        let (s1, s2, s3) = self.osc_bank.next_samples_individual();

        let pan = |base: f32, off: f32| (base + off).clamp(-1.0, 1.0);
        let stereo_osc = StereoSample::from_mono_panned(s1 * mods.osc_gain[0], pan(pans.0, mods.pan_offset[0]))
            + StereoSample::from_mono_panned(s2 * mods.osc_gain[1], pan(pans.1, mods.pan_offset[1]))
            + StereoSample::from_mono_panned(s3 * mods.osc_gain[2], pan(pans.2, mods.pan_offset[2]));
        
        // Filter the stereo signal (process L and R separately with independent filters)
        let filtered_left = self.filter_left.process(stereo_osc.left);
        let filtered_right = self.filter_right.process(stereo_osc.right);
        let mut filtered_stereo = StereoSample::new(filtered_left, filtered_right);
        if mods.width != 1.0 {
            filtered_stereo = filtered_stereo.with_width(mods.width);
        }

        filtered_stereo * (mods.amp_env * self.velocity * mods.amplitude_gain)
    }

    /// Resolved modulation offsets for the current sample (destination units).
    fn compute_modulation(&mut self) -> VoiceMods {
        let used = self.used_dests;
        let has = |d: ModDest| used & (1u32 << d as u32) != 0;

        // Mod-on-mod: LFO rate/depth follow the previous sample's sources
        // (one-sample lag, inaudible) so LFOs can modulate each other.
        if used & LFO_MOD_MASK != 0 {
            let prev = self.last_values;
            let prev_get = |s: ModSource| prev.get(s);
            for i in 0..3 {
                let (base_rate, base_depth) = self.lfo_base[i];
                let rate_off = if has(LFO_RATE_DESTS[i]) {
                    self.mod_matrix.offset_for(LFO_RATE_DESTS[i], prev_get)
                } else {
                    0.0
                };
                let depth_off = if has(LFO_DEPTH_DESTS[i]) {
                    self.mod_matrix.offset_for(LFO_DEPTH_DESTS[i], prev_get)
                } else {
                    0.0
                };
                // Rate: ±3 octaves at full amount; depth: additive.
                self.lfos[i].set_rate(base_rate * 2.0f32.powf(rate_off * 3.0));
                self.lfos[i].set_depth((base_depth + depth_off).clamp(0.0, 1.0));
            }
        }

        let lfo_values = [
            self.lfos[0].next_value(),
            self.lfos[1].next_value(),
            self.lfos[2].next_value(),
        ];
        let filter_env_amp = self.filter_envelope.next_amplitude();
        let amp_env = self.envelope.next_amplitude();
        let env3_amp = self.env3.next_amplitude();
        let env4_amp = self.env4.next_amplitude();

        let key_track = self
            .current_note
            .map(|n| (n as f32 - 60.0) / 60.0)
            .unwrap_or(0.0);

        let values = ModSourceValues {
            lfo: lfo_values,
            amp_env,
            filter_env: filter_env_amp,
            env3: env3_amp,
            env4: env4_amp,
            velocity: self.velocity,
            key_track,
            mod_wheel: self.mod_wheel,
            pitch_bend: self.pitch_bend_mod,
            aftertouch: self.aftertouch,
            ..Default::default()
        };
        self.last_values = values;
        let get = |s: ModSource| values.get(s);
        let off = |d: ModDest| if has(d) { self.mod_matrix.offset_for(d, get) } else { 0.0 };

        let pitch_semitones = off(ModDest::AllPitch);
        let cutoff_semitones = off(ModDest::FilterCutoff);
        let resonance_offset = off(ModDest::FilterResonance);
        let amplitude_offset = off(ModDest::Amplitude);
        let gain = |d: ModDest| (1.0 + off(d)).clamp(0.0, 2.0);

        VoiceMods {
            pitch_semitones,
            cutoff_semitones,
            resonance_offset,
            amplitude_gain: (1.0 + amplitude_offset).max(0.0),
            amp_env,
            filter_env_amp,
            osc_gain: [gain(ModDest::Osc1Level), gain(ModDest::Osc2Level), gain(ModDest::Osc3Level)],
            pan_offset: [off(ModDest::Osc1Pan), off(ModDest::Osc2Pan), off(ModDest::Osc3Pan)],
            width: (1.0 + off(ModDest::StereoWidth)).clamp(0.0, 2.0),
        }
    }

    /// Apply filter envelope + matrix cutoff offset (semitones) to the base
    /// cutoff, clamped to the audible range.
    fn modulated_cutoff(&self, filter_env_amp: f32, cutoff_semitones: f32) -> Frequency {
        let max_cutoff = 20000.0f32;
        if self.filter_env_amount <= 0.0 && cutoff_semitones == 0.0 {
            return self.base_cutoff;
        }
        // Octave-based envelope: 6 octaves of sweep at full amount.
        let octave_sweep = 6.0 * self.filter_env_amount * filter_env_amp;
        let env_cutoff = (self.base_cutoff * 2.0f32.powf(octave_sweep)).min(max_cutoff);
        let cutoff_factor = 2.0f32.powf(cutoff_semitones / 12.0);
        (env_cutoff * cutoff_factor).clamp(20.0, max_cutoff)
    }

    /// Swap the filter topology for both channels, preserving cutoff/resonance.
    pub fn set_filter_type(&mut self, ty: FilterType) {
        if ty == self.filter_type {
            return;
        }
        self.filter_type = ty;
        let cutoff = self.filter_left.cutoff();
        let res = self.filter_left.resonance();
        self.filter_left = make_filter(ty, cutoff, res, self.sample_rate);
        self.filter_right = make_filter(ty, cutoff, res, self.sample_rate);
    }

    /// Current filter topology.
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    /// Apply a matrix resonance offset on top of the base resonance.
    fn apply_resonance_offset(&mut self, offset: f32) {
        if offset != 0.0 {
            let res = (self.base_resonance + offset).clamp(0.0, 1.0);
            self.filter_left.set_resonance(res);
            self.filter_right.set_resonance(res);
        }
    }

    /// Trigger a note on this voice
    pub fn note_on(&mut self, note: MidiNote, velocity: Amplitude) {
        self.current_note = Some(note);
        self.velocity = velocity;
        let target_freq = midi_to_frequency(note);

        if self.portamento_time > 0.0 && self.is_active() {
            // Start glide from current frequency
            self.glide_target_freq = target_freq;
            self.glide_active = true;
            // Don't reset oscillator phase during glide
        } else {
            self.base_frequency = target_freq;
            self.glide_target_freq = target_freq;
            self.glide_active = false;
            self.osc_bank.set_frequency(self.base_frequency);
            self.osc_bank.reset();
        }

        self.filter_left.reset();
        self.filter_right.reset();
        self.filter_envelope.reset();
        self.envelope.trigger();
        self.filter_envelope.trigger();
        self.env3.reset();
        self.env3.trigger();
        self.env4.reset();
        self.env4.trigger();
        for lfo in &mut self.lfos {
            lfo.reset();
        }
    }

    /// Release the current note
    pub fn note_off(&mut self) {
        self.envelope.release();
        self.filter_envelope.release();
        self.env3.release();
        self.env4.release();
    }

    /// Check if this voice is currently playing
    pub fn is_active(&self) -> bool {
        !self.envelope.is_finished()
    }

    /// Debug: amp envelope state and configured sustain level.
    pub fn debug_env(&self) -> (EnvelopeState, f32) {
        (self.envelope.state(), self.envelope.sustain())
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
        self.filter_left.reset();
        self.filter_right.reset();
        self.envelope.reset();
        self.filter_envelope.reset();
        self.env3.reset();
        self.env4.reset();
        for lfo in &mut self.lfos {
            lfo.reset();
        }
        self.current_note = None;
    }

    /// Set the sample rate
    pub fn set_sample_rate(&mut self, sample_rate: SampleRate) {
        self.sample_rate = sample_rate;
        self.osc_bank.set_sample_rate(sample_rate);
        self.filter_left.set_sample_rate(sample_rate);
        self.filter_right.set_sample_rate(sample_rate);
        self.envelope.set_sample_rate(sample_rate);
        self.filter_envelope.set_sample_rate(sample_rate);
        self.env3.set_sample_rate(sample_rate);
        self.env4.set_sample_rate(sample_rate);
        for lfo in &mut self.lfos {
            lfo.set_sample_rate(sample_rate);
        }
    }

    /// Set LFO rate (0.1 to 20 Hz) for one of the 3 LFOs
    pub fn set_lfo_rate_at(&mut self, index: usize, rate: f32) {
        if let Some(lfo) = self.lfos.get_mut(index) {
            lfo.set_rate(rate);
            self.lfo_base[index].0 = lfo.rate();
        }
    }

    /// Set LFO depth (0.0 to 1.0) for one of the 3 LFOs
    pub fn set_lfo_depth_at(&mut self, index: usize, depth: f32) {
        if let Some(lfo) = self.lfos.get_mut(index) {
            lfo.set_depth(depth);
            self.lfo_base[index].1 = lfo.depth();
        }
    }

    /// Set LFO waveform for one of the 3 LFOs
    pub fn set_lfo_waveform_at(&mut self, index: usize, waveform: LfoWaveform) {
        if let Some(lfo) = self.lfos.get_mut(index) {
            lfo.set_waveform(waveform);
        }
    }

    /// Set the free envelope (Env3) ADSR times
    pub fn set_env3_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.env3.set_attack(attack);
        self.env3.set_decay(decay);
        self.env3.set_sustain(sustain);
        self.env3.set_release(release);
    }

    /// Set the second free envelope (Env4) ADSR times
    pub fn set_env4_adsr(&mut self, adsr: [f32; 4]) {
        self.env4.set_attack(adsr[0]);
        self.env4.set_decay(adsr[1]);
        self.env4.set_sustain(adsr[2]);
        self.env4.set_release(adsr[3]);
    }

    /// Set LFO rate (0.1 to 20 Hz)
    pub fn set_lfo_rate(&mut self, rate: f32) {
        self.set_lfo_rate_at(0, rate);
    }

    /// Set LFO depth (0.0 to 1.0)
    pub fn set_lfo_depth(&mut self, depth: f32) {
        self.set_lfo_depth_at(0, depth);
    }

    /// Set LFO waveform
    pub fn set_lfo_waveform(&mut self, waveform: LfoWaveform) {
        self.set_lfo_waveform_at(0, waveform);
    }

    /// Set LFO destination (legacy; routing is handled by the mod matrix)
    pub fn set_lfo_destination(&mut self, destination: LfoDestination) {
        self.lfos[0].set_destination(destination);
    }

    /// Get LFO rate
    pub fn lfo_rate(&self) -> f32 {
        self.lfos[0].rate()
    }

    /// Get LFO depth
    pub fn lfo_depth(&self) -> f32 {
        self.lfos[0].depth()
    }

    /// Get LFO waveform
    pub fn lfo_waveform(&self) -> LfoWaveform {
        self.lfos[0].waveform()
    }

    /// Get LFO destination
    pub fn lfo_destination(&self) -> LfoDestination {
        self.lfos[0].destination()
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
        self.filter_left.set_cutoff(cutoff);
        self.filter_right.set_cutoff(cutoff);
    }

    /// Set the modulation routing matrix
    pub fn set_mod_matrix(&mut self, matrix: ModMatrix) {
        let had_lfo_mods = self.used_dests & LFO_MOD_MASK != 0;
        self.mod_matrix = matrix;
        self.used_dests = matrix
            .slots
            .iter()
            .filter(|s| s.enabled && s.source != ModSource::Off)
            .fold(0u32, |m, s| m | (1u32 << s.dest as u32));
        // Restore unmodulated LFO settings when mod-on-mod routes go away.
        if had_lfo_mods && self.used_dests & LFO_MOD_MASK == 0 {
            for i in 0..3 {
                let (rate, depth) = self.lfo_base[i];
                self.lfos[i].set_rate(rate);
                self.lfos[i].set_depth(depth);
            }
        }
    }

    /// Set the pitch-wheel position (-1.0 to 1.0) used as a modulation source
    pub fn set_pitch_bend_mod(&mut self, bend: f32) {
        self.pitch_bend_mod = bend.clamp(-1.0, 1.0);
    }

    /// Set the aftertouch pressure (0.0 to 1.0) used as a modulation source
    pub fn set_aftertouch(&mut self, pressure: f32) {
        self.aftertouch = pressure.clamp(0.0, 1.0);
    }

    /// Set the mod wheel value (0.0 to 1.0) used as a modulation source
    pub fn set_mod_wheel(&mut self, value: f32) {
        self.mod_wheel = value.clamp(0.0, 1.0);
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
        self.base_resonance = resonance;
        self.filter_left.set_resonance(resonance);
        self.filter_right.set_resonance(resonance);
    }

    /// Get the modulation matrix
    pub fn mod_matrix(&self) -> &ModMatrix {
        &self.mod_matrix
    }
}

/// Per-sample resolved modulation for a voice (destination units).
struct VoiceMods {
    pitch_semitones: f32,
    cutoff_semitones: f32,
    resonance_offset: f32,
    amplitude_gain: f32,
    amp_env: f32,
    filter_env_amp: f32,
    /// Per-oscillator level multipliers (1.0 = unmodulated).
    osc_gain: [f32; 3],
    /// Per-oscillator pan offsets added to the base pans.
    pan_offset: [f32; 3],
    /// Per-voice stereo width factor (1.0 = unmodulated).
    width: f32,
}

/// Matrix destinations for LFO rate / depth (mod-on-mod), indexed by LFO.
const LFO_RATE_DESTS: [ModDest; 3] = [ModDest::Lfo1Rate, ModDest::Lfo2Rate, ModDest::Lfo3Rate];
const LFO_DEPTH_DESTS: [ModDest; 3] = [ModDest::Lfo1Depth, ModDest::Lfo2Depth, ModDest::Lfo3Depth];
/// Bitmask of all mod-on-mod destinations.
const LFO_MOD_MASK: u32 = (1 << ModDest::Lfo1Rate as u32)
    | (1 << ModDest::Lfo2Rate as u32)
    | (1 << ModDest::Lfo3Rate as u32)
    | (1 << ModDest::Lfo1Depth as u32)
    | (1 << ModDest::Lfo2Depth as u32)
    | (1 << ModDest::Lfo3Depth as u32);

/// Settings for one of the 3 LFOs
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoSettings {
    pub rate: f32,
    pub depth: f32,
    pub waveform: LfoWaveform,
    /// When true, `rate` is ignored and the LFO period is `rate_beats`
    /// quarter-notes at the current tempo.
    pub sync: bool,
    /// Synced LFO period in beats (e.g. 1.0 = quarter note, 0.5 = eighth).
    pub rate_beats: f32,
}

impl Default for LfoSettings {
    fn default() -> Self {
        Self {
            rate: 6.0,
            depth: 0.0,
            waveform: LfoWaveform::Sine,
            sync: false,
            rate_beats: 1.0,
        }
    }
}

/// Effective LFO rate in Hz for a settings block at a given tempo.
pub fn lfo_rate_hz(s: &LfoSettings, bpm: f32) -> f32 {
    if s.sync {
        (bpm / 60.0) / s.rate_beats.max(0.01)
    } else {
        s.rate
    }
}

/// Complete modulation state shared between GUI and audio thread:
/// the routing matrix plus the extra modulator settings that are not
/// part of the SynthParam bank.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModState {
    pub matrix: ModMatrix,
    /// LFO 1 settings used only for sync fields (rate/depth/waveform come
    /// from SynthParam); kept here so the whole tempo state lives together.
    pub lfo1: LfoSettings,
    pub lfo2: LfoSettings,
    pub lfo3: LfoSettings,
    /// Env3 ADSR: [attack, decay, sustain, release]
    pub env3: [f32; 4],
    /// Env4 ADSR: [attack, decay, sustain, release]
    pub env4: [f32; 4],
    /// Tempo in BPM. Standalone: user-set; plugin: host transport.
    pub bpm: f32,
    /// Delay effect tempo sync.
    pub delay_sync: bool,
    /// Synced delay time in beats.
    pub delay_beats: f32,
    /// Filter topology (SVF / Ladder / Comb).
    pub filter_type: FilterType,
}

impl Default for ModState {
    fn default() -> Self {
        Self {
            matrix: ModMatrix::default(),
            lfo1: LfoSettings::default(),
            lfo2: LfoSettings::default(),
            lfo3: LfoSettings::default(),
            env3: [0.01, 0.3, 0.0, 0.3],
            env4: [0.01, 0.3, 0.0, 0.3],
            bpm: 120.0,
            delay_sync: false,
            delay_beats: 1.0,
            filter_type: FilterType::Svf,
        }
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

/// Polyphony mode for the voice manager
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolyphonyMode {
    /// Single voice with key stacking (returns to previous note)
    Mono,
    /// Multiple independent voices for chords
    Poly,
}

impl Default for PolyphonyMode {
    fn default() -> Self {
        PolyphonyMode::Mono
    }
}

/// Manages multiple voices for polyphonic playback
pub struct VoiceManager {
    voices: Vec<Voice>,
    max_voices: usize,
    sample_rate: SampleRate,
    master_volume: Amplitude,
    cc_mapping: CCMapping,
    // Polyphony mode
    polyphony_mode: PolyphonyMode,
    // Stereo panning (-1.0 = left, 0.0 = center, 1.0 = right)
    osc1_pan: f32,
    osc2_pan: f32,
    osc3_pan: f32,
    stereo_width: f32,  // 0.0 = mono, 1.0 = normal, 2.0 = extra wide
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
    lfo2: LfoSettings,
    lfo3: LfoSettings,
    // Free modulation envelope (Env3) ADSR
    env3_attack: f32,
    env3_decay: f32,
    env3_sustain: f32,
    env3_release: f32,
    // Second free modulation envelope (Env4) ADSR
    env4: [f32; 4],
    // LFO1 tempo sync (rate/depth/waveform come from SynthParam fields)
    lfo1_sync: bool,
    lfo1_beats: f32,
    // Tempo
    bpm: f32,
    // Modulation
    mod_matrix: ModMatrix,
    mod_wheel: f32,
    // Delay tempo sync (applied by the engine to the effects chain)
    delay_sync: bool,
    delay_beats: f32,
    // Filter topology for all voices
    filter_type: FilterType,
    osc_state: OscBankState,
    // Portamento
    portamento_time: f32,
    // Key stack for monophonic note priority (newest on top)
    key_stack: Vec<(MidiNote, Amplitude)>,
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
            polyphony_mode: PolyphonyMode::Mono,
            // Stereo defaults (slight spread for fatness)
            osc1_pan: 0.0,    // Center
            osc2_pan: -0.3,   // Slightly left
            osc3_pan: 0.3,    // Slightly right
            stereo_width: 1.0,
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
            lfo2: LfoSettings::default(),
            lfo3: LfoSettings::default(),
            lfo1_sync: false,
            lfo1_beats: 1.0,
            bpm: 120.0,
            env3_attack: 0.01,
            env3_decay: 0.3,
            env3_sustain: 0.0,
            env3_release: 0.3,
            env4: [0.01, 0.3, 0.0, 0.3],
            mod_matrix: ModMatrix::default(),
            mod_wheel: 0.0,
            delay_sync: false,
            delay_beats: 1.0,
            filter_type: FilterType::Svf,
            osc_state,
            // Portamento
            portamento_time: 0.0,
            // Key stack
            key_stack: Vec::new(),
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

    /// Create a monophonic voice manager (8 voices but mono mode)
    pub fn monophonic(sample_rate: SampleRate) -> Self {
        let mut vm = Self::new(8, sample_rate);
        vm.polyphony_mode = PolyphonyMode::Mono;
        vm
    }
    
    /// Create a polyphonic voice manager (8 voices)
    pub fn polyphonic(sample_rate: SampleRate) -> Self {
        let mut vm = Self::new(8, sample_rate);
        vm.polyphony_mode = PolyphonyMode::Poly;
        vm
    }

    /// Reset all voices and key stack to initial state
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.reset();
        }
        self.key_stack.clear();
    }
    
    /// Set polyphony mode. No-op when the mode is unchanged, so it is safe
    /// to call every buffer (releasing voices only makes sense on an actual
    /// mode switch).
    pub fn set_polyphony_mode(&mut self, mode: PolyphonyMode) {
        if self.polyphony_mode == mode {
            return;
        }
        self.polyphony_mode = mode;
        // Clear key stack when switching modes
        self.key_stack.clear();
        // Release all voices when switching
        for voice in &mut self.voices {
            voice.note_off();
        }
    }
    
    /// Get current polyphony mode
    pub fn polyphony_mode(&self) -> PolyphonyMode {
        self.polyphony_mode
    }
    
    /// Get number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }
    
    /// Get max voices
    pub fn max_voices(&self) -> usize {
        self.max_voices
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_master_volume(&mut self, volume: Amplitude) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Set portamento time in seconds (0.0 = off)
    pub fn set_portamento_time(&mut self, time: f32) {
        self.portamento_time = time.max(0.0);
        for voice in &mut self.voices {
            voice.portamento_time = self.portamento_time;
        }
    }

    /// Generate the next mixed sample from all active voices (mono, for backwards compat)
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
    
    /// Debug: snapshot of voice 0's amp envelope (state, env sustain) plus
    /// the manager-level sustain and filter parameters.
    pub fn debug_voice0(&self) -> (EnvelopeState, f32, f32, f32, f32) {
        let (state, env_sustain) = self.voices[0].debug_env();
        (
            state,
            env_sustain,
            self.sustain_level,
            self.filter_cutoff,
            self.filter_env_amount,
        )
    }

    /// Generate the next stereo sample from all active voices
    pub fn next_sample_stereo(&mut self) -> StereoSample {
        let mut mixed = StereoSample::ZERO;
        let pans = (self.osc1_pan, self.osc2_pan, self.osc3_pan);

        for voice in &mut self.voices {
            if voice.is_active() {
                mixed += voice.next_sample_stereo(pans);
            }
        }

        // Apply master volume, stereo width, and soft clipping
        let output = mixed * self.master_volume;
        let widened = output.with_width(self.stereo_width);
        StereoSample::new(soft_clip(widened.left), soft_clip(widened.right))
    }
    
    /// Set oscillator pan (-1.0 = left, 0.0 = center, 1.0 = right)
    pub fn set_osc_pan(&mut self, osc_num: u8, pan: f32) {
        let pan = pan.clamp(-1.0, 1.0);
        match osc_num {
            1 => self.osc1_pan = pan,
            2 => self.osc2_pan = pan,
            3 => self.osc3_pan = pan,
            _ => {}
        }
    }
    
    /// Get oscillator pan
    pub fn osc_pan(&self, osc_num: u8) -> f32 {
        match osc_num {
            1 => self.osc1_pan,
            2 => self.osc2_pan,
            3 => self.osc3_pan,
            _ => 0.0,
        }
    }
    
    /// Set stereo width (0.0 = mono, 1.0 = normal, 2.0 = extra wide)
    pub fn set_stereo_width(&mut self, width: f32) {
        self.stereo_width = width.clamp(0.0, 2.0);
    }
    
    /// Get stereo width
    pub fn stereo_width(&self) -> f32 {
        self.stereo_width
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
        let rate = self.effective_lfo_rate(0);
        for voice in &mut self.voices {
            voice.set_lfo_rate(rate);
        }
    }

    /// Effective rate in Hz for LFO `index` given current sync/tempo.
    fn effective_lfo_rate(&self, index: usize) -> f32 {
        match index {
            0 if self.lfo1_sync => (self.bpm / 60.0) / self.lfo1_beats.max(0.01),
            0 => self.lfo_rate,
            1 => lfo_rate_hz(&self.lfo2, self.bpm),
            2 => lfo_rate_hz(&self.lfo3, self.bpm),
            _ => self.lfo_rate,
        }
    }

    /// Swap the filter topology on every voice, preserving cutoff/resonance.
    pub fn set_filter_type(&mut self, ty: FilterType) {
        if ty == self.filter_type {
            return;
        }
        self.filter_type = ty;
        for voice in &mut self.voices {
            voice.set_filter_type(ty);
        }
    }

    /// Current filter topology.
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    /// Set the tempo in BPM; recomputes rates of synced LFOs.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.bpm = bpm.clamp(20.0, 300.0);
        let rates = [
            self.effective_lfo_rate(0),
            self.effective_lfo_rate(1),
            self.effective_lfo_rate(2),
        ];
        for voice in &mut self.voices {
            for i in 0..3 {
                voice.set_lfo_rate_at(i, rates[i]);
            }
        }
    }

    /// Current tempo in BPM.
    pub fn bpm(&self) -> f32 {
        self.bpm
    }

    /// Enable/disable tempo sync for LFO1 and set its beat period.
    pub fn set_lfo1_sync(&mut self, sync: bool, beats: f32) {
        self.lfo1_sync = sync;
        self.lfo1_beats = beats.max(0.01);
        let rate = self.effective_lfo_rate(0);
        for voice in &mut self.voices {
            voice.set_lfo_rate_at(0, rate);
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

    /// Set rate/depth/waveform for LFO 2 or 3 (index 1 or 2)
    pub fn set_lfo_settings(&mut self, index: usize, s: LfoSettings) {
        let slot = match index {
            1 => &mut self.lfo2,
            2 => &mut self.lfo3,
            _ => return,
        };
        *slot = s;
        let rate = self.effective_lfo_rate(index);
        for voice in &mut self.voices {
            voice.set_lfo_rate_at(index, rate);
            voice.set_lfo_depth_at(index, s.depth);
            voice.set_lfo_waveform_at(index, s.waveform);
        }
    }

    /// Get settings for LFO 2 or 3 (index 1 or 2)
    pub fn lfo_settings(&self, index: usize) -> Option<LfoSettings> {
        match index {
            1 => Some(self.lfo2),
            2 => Some(self.lfo3),
            _ => None,
        }
    }

    /// Set the free envelope (Env3) ADSR times on all voices
    pub fn set_env3_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        self.env3_attack = attack;
        self.env3_decay = decay;
        self.env3_sustain = sustain;
        self.env3_release = release;
        for voice in &mut self.voices {
            voice.set_env3_adsr(attack, decay, sustain, release);
        }
    }

    /// Set the Env4 ADSR times on all voices
    pub fn set_env4_adsr(&mut self, adsr: [f32; 4]) {
        self.env4 = adsr;
        for voice in &mut self.voices {
            voice.set_env4_adsr(adsr);
        }
    }

    /// Get the Env3 ADSR times
    pub fn env3_adsr(&self) -> (f32, f32, f32, f32) {
        (
            self.env3_attack,
            self.env3_decay,
            self.env3_sustain,
            self.env3_release,
        )
    }

    /// Set the modulation routing matrix on the manager and all voices
    pub fn set_mod_matrix(&mut self, matrix: ModMatrix) {
        self.mod_matrix = matrix;
        for voice in &mut self.voices {
            voice.set_mod_matrix(matrix);
        }
    }

    /// Get the current modulation routing matrix
    pub fn mod_matrix(&self) -> &ModMatrix {
        &self.mod_matrix
    }

    /// Apply a full modulation state (matrix + LFOs + Env3 + tempo)
    pub fn apply_mod_state(&mut self, s: &ModState) {
        self.set_bpm(s.bpm);
        self.set_mod_matrix(s.matrix);
        self.set_lfo1_sync(s.lfo1.sync, s.lfo1.rate_beats);
        self.set_lfo_settings(1, s.lfo2);
        self.set_lfo_settings(2, s.lfo3);
        self.set_env3_adsr(s.env3[0], s.env3[1], s.env3[2], s.env3[3]);
        if s.env4 != self.env4 {
            self.set_env4_adsr(s.env4);
        }
        self.delay_sync = s.delay_sync;
        self.delay_beats = s.delay_beats;
        self.set_filter_type(s.filter_type);
    }

    /// Delay tempo-sync state for the engine to apply to the effects chain:
    /// (synced, beats, bpm).
    pub fn delay_sync_state(&self) -> (bool, f32, f32) {
        (self.delay_sync, self.delay_beats, self.bpm)
    }

    /// Set delay tempo-sync state (stored on the manager; applied by the
    /// engine to the effects chain).
    pub fn set_delay_sync(&mut self, sync: bool, beats: f32) {
        self.delay_sync = sync;
        self.delay_beats = beats.max(0.05);
    }

    /// Snapshot the current modulation state
    pub fn mod_state(&self) -> ModState {
        ModState {
            matrix: self.mod_matrix,
            lfo1: LfoSettings {
                rate: self.lfo_rate,
                depth: self.lfo_depth,
                waveform: self.lfo_waveform,
                sync: self.lfo1_sync,
                rate_beats: self.lfo1_beats,
            },
            lfo2: self.lfo2,
            lfo3: self.lfo3,
            env3: [
                self.env3_attack,
                self.env3_decay,
                self.env3_sustain,
                self.env3_release,
            ],
            env4: self.env4,
            bpm: self.bpm,
            delay_sync: self.delay_sync,
            delay_beats: self.delay_beats,
            filter_type: self.filter_type,
        }
    }

    /// Set the mod wheel value (0.0 to 1.0) on all voices
    pub fn set_mod_wheel(&mut self, value: f32) {
        self.mod_wheel = value.clamp(0.0, 1.0);
        for voice in &mut self.voices {
            voice.set_mod_wheel(self.mod_wheel);
        }
    }

    /// Set aftertouch pressure (0.0 to 1.0) on all voices
    pub fn set_aftertouch(&mut self, pressure: f32) {
        for voice in &mut self.voices {
            voice.set_aftertouch(pressure);
        }
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
            voice.set_pitch_bend_mod(bend);
        }
    }

    /// Set pitch bend range in semitones (1-24)
    pub fn set_pitch_bend_range(&mut self, semitones: u8) {
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_pitch_bend_range(semitones);
        }
    }

    /// Handle a parameter change from CC. Scaling goes through the shared
    /// parameter spec (`cc_to_plain`), so a CC produces exactly the same value
    /// as the equivalent GUI slider position in both the standalone and the
    /// plugin.
    fn handle_param_change(&mut self, param: SynthParam, value: u8) {
        let plain = cc_to_plain(param, value);
        self.apply_plain_param(param, plain);
    }

    /// Apply an already-scaled (plain) parameter value to all voices.
    fn apply_plain_param(&mut self, param: SynthParam, plain: f32) {
        match param {
            // ADSR Envelope
            SynthParam::Attack => self.set_attack(plain),
            SynthParam::Decay => self.set_decay(plain),
            SynthParam::Sustain => self.set_sustain(plain),
            SynthParam::Release => self.set_release(plain),

            // Filter
            SynthParam::FilterCutoff => self.set_filter_cutoff(plain),
            SynthParam::FilterResonance => self.set_filter_resonance(plain),

            // Filter Envelope
            SynthParam::FilterAttack => self.set_filter_attack(plain),
            SynthParam::FilterDecay => self.set_filter_decay(plain),
            SynthParam::FilterSustain => self.set_filter_sustain(plain),
            SynthParam::FilterRelease => self.set_filter_release(plain),
            SynthParam::FilterEnvAmount => self.set_filter_env_amount(plain),

            // LFO
            SynthParam::LfoRate => self.set_lfo_rate(plain),
            SynthParam::LfoDepth => self.set_lfo_depth(plain),
            SynthParam::LfoWaveform => {
                let waveform = match plain as i32 {
                    0 => LfoWaveform::Sine,
                    1 => LfoWaveform::Triangle,
                    2 => LfoWaveform::Square,
                    3 => LfoWaveform::Saw,
                    _ => LfoWaveform::Random,
                };
                self.set_lfo_waveform(waveform);
            }
            SynthParam::LfoDestination => {
                let destination = match plain as i32 {
                    0 => LfoDestination::Off,
                    1 => LfoDestination::Pitch,
                    2 => LfoDestination::FilterCutoff,
                    _ => LfoDestination::Amplitude,
                };
                self.set_lfo_destination(destination);
            }

            // Pitch
            SynthParam::PitchBendRange => self.set_pitch_bend_range(plain as u8),
            SynthParam::PortamentoTime => self.set_portamento_time(plain),

            // Oscillators
            SynthParam::Osc1Waveform => {
                self.set_osc_waveform(1, WaveformType::from_index(plain as u8))
            }
            SynthParam::Osc1Level => self.set_osc_level(1, plain),
            SynthParam::Osc1Phase => self.set_osc_phase(1, plain),
            SynthParam::Osc2Waveform => {
                self.set_osc_waveform(2, WaveformType::from_index(plain as u8))
            }
            SynthParam::Osc2Level => self.set_osc_level(2, plain),
            SynthParam::Osc2Semitones => self.set_osc_semitones(2, plain as i8),
            SynthParam::Osc2Cents => self.set_osc_cents(2, plain as i8),
            SynthParam::Osc2Phase => self.set_osc_phase(2, plain),
            SynthParam::Osc3Waveform => {
                self.set_osc_waveform(3, WaveformType::from_index(plain as u8))
            }
            SynthParam::Osc3Level => self.set_osc_level(3, plain),
            SynthParam::Osc3Semitones => self.set_osc_semitones(3, plain as i8),
            SynthParam::Osc3Cents => self.set_osc_cents(3, plain as i8),
            SynthParam::Osc3Phase => self.set_osc_phase(3, plain),

            // Stereo
            SynthParam::Osc1Pan => self.set_osc_pan(1, plain),
            SynthParam::Osc2Pan => self.set_osc_pan(2, plain),
            SynthParam::Osc3Pan => self.set_osc_pan(3, plain),
            SynthParam::StereoWidth => self.set_stereo_width(plain),

            // Master
            SynthParam::MasterVolume => self.set_master_volume(plain),
        }
    }
}

impl SynthEventReceiver for VoiceManager {
    fn receive_event(&mut self, event: NoteEvent) {
        match event.kind {
            SynthEventKind::NoteOn => {
                self.handle_note_on(event.note, event.velocity);
            }
            SynthEventKind::NoteOff => {
                self.handle_note_off(event.note);
            }
            SynthEventKind::WaveformChange(waveform) => {
                // Legacy: set all oscillators to the same waveform
                self.set_osc_waveform(1, waveform);
                self.set_osc_waveform(2, waveform);
                self.set_osc_waveform(3, waveform);
            }
            SynthEventKind::ControlChange { cc, value } => {
                // CC1 is the mod wheel: always available as a mod matrix source,
                // in addition to any explicit CC mapping.
                if cc == 1 {
                    self.set_mod_wheel(value as f32 / 127.0);
                }
                if let Some(param) = self.cc_mapping.get_param(cc) {
                    self.handle_param_change(param, value);
                }
            }
            SynthEventKind::PitchBend(bend) => {
                self.set_pitch_bend(bend);
            }
            SynthEventKind::Aftertouch(pressure) => {
                self.set_aftertouch(pressure);
            }
        }
    }
}

impl VoiceManager {
    /// Handle note on - dispatches to mono or poly handler
    fn handle_note_on(&mut self, note: MidiNote, velocity: Amplitude) {
        match self.polyphony_mode {
            PolyphonyMode::Mono => self.handle_note_on_mono(note, velocity),
            PolyphonyMode::Poly => self.handle_note_on_poly(note, velocity),
        }
    }
    
    /// Handle note off - dispatches to mono or poly handler
    fn handle_note_off(&mut self, note: MidiNote) {
        match self.polyphony_mode {
            PolyphonyMode::Mono => self.handle_note_off_mono(note),
            PolyphonyMode::Poly => self.handle_note_off_poly(note),
        }
    }
    
    /// Monophonic note on with key stacking
    fn handle_note_on_mono(&mut self, note: MidiNote, velocity: Amplitude) {
        // Remove note if already pressed (avoid duplicates)
        self.key_stack.retain(|&(n, _)| n != note);
        // Push to top of stack (newest priority)
        self.key_stack.push((note, velocity));
        
        // Trigger the note on the first voice
        if let Some(voice) = self.voices.first_mut() {
            voice.note_on(note, velocity);
        }
    }
    
    /// Monophonic note off with key stacking - return to previous note if any
    fn handle_note_off_mono(&mut self, note: MidiNote) {
        // Remove the note from stack
        self.key_stack.retain(|&(n, _)| n != note);
        
        if let Some(voice) = self.voices.first_mut() {
            // If this was the current note, switch to previous if available
            if voice.current_note() == Some(note) {
                if let Some(&(prev_note, prev_velocity)) = self.key_stack.last() {
                    // Switch to previous note with its original velocity
                    voice.note_on(prev_note, prev_velocity);
                } else {
                    // No more notes in stack, release the voice
                    voice.note_off();
                }
            }
        }
    }
    
    /// Polyphonic note on - allocate a voice for the note
    fn handle_note_on_poly(&mut self, note: MidiNote, velocity: Amplitude) {
        // First check if this note is already playing - retrigger it
        if let Some(voice) = self.find_voice_with_note(note) {
            voice.note_on(note, velocity);
            return;
        }
        
        // Allocate a new voice
        if let Some(idx) = self.allocate_voice_index() {
            // Apply current envelope/filter/lfo settings to the voice
            let voice = &mut self.voices[idx];
            voice.set_attack(self.attack_time);
            voice.set_decay(self.decay_time);
            voice.set_sustain(self.sustain_level);
            voice.set_release(self.release_time);
            voice.set_filter_cutoff(self.filter_cutoff);
            voice.set_filter_resonance(self.filter_resonance);
            voice.set_filter_attack(self.filter_attack_time);
            voice.set_filter_decay(self.filter_decay_time);
            voice.set_filter_sustain(self.filter_sustain_level);
            voice.set_filter_release(self.filter_release_time);
            voice.set_filter_env_amount(self.filter_env_amount);
            voice.set_lfo_rate(self.lfo_rate);
            voice.set_lfo_depth(self.lfo_depth);
            voice.set_lfo_waveform(self.lfo_waveform);
            voice.set_lfo_destination(self.lfo_destination);
            voice.set_lfo_rate_at(1, self.lfo2.rate);
            voice.set_lfo_depth_at(1, self.lfo2.depth);
            voice.set_lfo_waveform_at(1, self.lfo2.waveform);
            voice.set_lfo_rate_at(2, self.lfo3.rate);
            voice.set_lfo_depth_at(2, self.lfo3.depth);
            voice.set_lfo_waveform_at(2, self.lfo3.waveform);
            voice.set_env3_adsr(
                self.env3_attack,
                self.env3_decay,
                self.env3_sustain,
                self.env3_release,
            );
            voice.set_env4_adsr(self.env4);
            voice.set_mod_matrix(self.mod_matrix);
            voice.set_mod_wheel(self.mod_wheel);
            // Portamento doesn't make as much sense in poly, but keep it
            voice.portamento_time = self.portamento_time;
            
            voice.note_on(note, velocity);
        }
    }
    
    /// Polyphonic note off - release the voice playing this note
    fn handle_note_off_poly(&mut self, note: MidiNote) {
        if let Some(voice) = self.find_voice_with_note(note) {
            voice.note_off();
        }
    }
}

/// Convert a legacy `lfo_destination` preset field into an equivalent
/// matrix slot, preserving the old modulation depth semantics.
pub(crate) fn legacy_lfo_slot(preset: &Preset) -> Option<ModSlot> {
    if preset.lfo_depth <= 0.0 {
        return None;
    }
    let (dest, amount) = match preset.lfo_destination {
        // Old vibrato: ±1 semitone at depth 1.0 -> depth already scales the
        // LFO output, so amount maps to the 12-semitone destination range.
        LfoDestination::Pitch => (ModDest::AllPitch, preset.lfo_depth / 12.0),
        // Old filter LFO: ±2 octaves at full depth (depth was applied twice).
        LfoDestination::FilterCutoff => (ModDest::FilterCutoff, preset.lfo_depth * 0.4),
        // Old tremolo: ±50% amplitude at depth 1.0.
        LfoDestination::Amplitude => (ModDest::Amplitude, preset.lfo_depth * 0.5),
        LfoDestination::Off => return None,
    };
    Some(ModSlot {
        source: ModSource::Lfo1,
        dest,
        amount,
        enabled: true,
    })
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

impl VoiceManager {
    /// Apply a preset to all voices
    pub fn apply_preset(&mut self, preset: &Preset) {
        // Oscillator settings
        self.set_osc_waveform(1, preset.osc1_waveform);
        self.set_osc_level(1, preset.osc1_level);
        self.set_osc_phase(1, preset.osc1_phase);

        self.set_osc_waveform(2, preset.osc2_waveform);
        self.set_osc_level(2, preset.osc2_level);
        self.set_osc_detune(2, preset.osc2_semitones, preset.osc2_cents);
        self.set_osc_phase(2, preset.osc2_phase);

        self.set_osc_waveform(3, preset.osc3_waveform);
        self.set_osc_level(3, preset.osc3_level);
        self.set_osc_detune(3, preset.osc3_semitones, preset.osc3_cents);
        self.set_osc_phase(3, preset.osc3_phase);

        // Filter settings
        self.set_filter_cutoff(preset.filter_cutoff);
        self.set_filter_resonance(preset.filter_resonance);

        // Amplitude envelope
        self.set_attack(preset.amp_attack);
        self.set_decay(preset.amp_decay);
        self.set_sustain(preset.amp_sustain);
        self.set_release(preset.amp_release);

        // Filter envelope
        self.set_filter_attack(preset.filter_attack);
        self.set_filter_decay(preset.filter_decay);
        self.set_filter_sustain(preset.filter_sustain);
        self.set_filter_release(preset.filter_release);
        self.set_filter_env_amount(preset.filter_env_amount);

        // LFO settings
        self.set_lfo_rate(preset.lfo_rate);
        self.set_lfo_depth(preset.lfo_depth);
        self.set_lfo_waveform(preset.lfo_waveform);
        self.set_lfo_destination(preset.lfo_destination);

        // Tempo + sync (must run before LFO rate so synced rates resolve)
        self.set_bpm(preset.tempo_bpm);
        self.set_lfo1_sync(preset.lfo1_sync, preset.lfo1_beats);
        self.lfo2.sync = preset.lfo2_sync;
        self.lfo2.rate_beats = preset.lfo2_beats;
        self.lfo3.sync = preset.lfo3_sync;
        self.lfo3.rate_beats = preset.lfo3_beats;
        self.set_delay_sync(preset.delay_sync, preset.delay_beats);
        self.set_filter_type(preset.filter_type);
        // lfo2/lfo3 sync fields were written directly above; refresh all
        // effective rates so the voices pick them up.
        let rates = [
            self.effective_lfo_rate(0),
            self.effective_lfo_rate(1),
            self.effective_lfo_rate(2),
        ];
        for voice in &mut self.voices {
            for i in 0..3 {
                voice.set_lfo_rate_at(i, rates[i]);
            }
        }

        // Mod matrix: use the stored matrix, or migrate the legacy
        // lfo_destination into slot 0 for old presets.
        let mut matrix = preset.mod_matrix;
        if matrix.is_empty() {
            if let Some(slot) = legacy_lfo_slot(preset) {
                matrix.slots[0] = slot;
            }
        }
        self.set_mod_matrix(matrix);

        // Pitch bend
        for voice in &mut self.voices {
            voice.osc_bank_mut().set_pitch_bend_range(preset.pitch_bend_range);
        }

        // Portamento
        self.set_portamento_time(preset.portamento_time);

        // Master volume
        self.master_volume = preset.master_volume.clamp(0.0, 1.0);

        // Stereo / Pan
        self.set_osc_pan(1, preset.osc1_pan);
        self.set_osc_pan(2, preset.osc2_pan);
        self.set_osc_pan(3, preset.osc3_pan);
        self.set_stereo_width(preset.stereo_width);

        // Polyphony mode
        self.set_polyphony_mode(preset.polyphony_mode);

        // Note: Effects are handled by the AudioEngine, not VoiceManager
    }

    /// Create a preset from current settings
    pub fn create_preset(&self, name: &str) -> Preset {
        Preset {
            name: name.to_string(),
            version: "1.0".to_string(),

            osc1_waveform: self.osc_state.osc1_waveform,
            osc1_level: self.osc_state.osc1_level,
            osc1_phase: self.osc_state.osc1_phase,

            osc2_waveform: self.osc_state.osc2_waveform,
            osc2_level: self.osc_state.osc2_level,
            osc2_semitones: self.osc_state.osc2_semitones,
            osc2_cents: self.osc_state.osc2_cents,
            osc2_phase: self.osc_state.osc2_phase,

            osc3_waveform: self.osc_state.osc3_waveform,
            osc3_level: self.osc_state.osc3_level,
            osc3_semitones: self.osc_state.osc3_semitones,
            osc3_cents: self.osc_state.osc3_cents,
            osc3_phase: self.osc_state.osc3_phase,

            filter_cutoff: self.filter_cutoff,
            filter_resonance: self.filter_resonance,

            amp_attack: self.attack_time,
            amp_decay: self.decay_time,
            amp_sustain: self.sustain_level,
            amp_release: self.release_time,

            filter_attack: self.filter_attack_time,
            filter_decay: self.filter_decay_time,
            filter_sustain: self.filter_sustain_level,
            filter_release: self.filter_release_time,
            filter_env_amount: self.filter_env_amount,

            lfo_rate: self.lfo_rate,
            lfo_depth: self.lfo_depth,
            lfo_waveform: self.lfo_waveform,
            lfo_destination: self.lfo_destination,

            lfo2_rate: self.lfo2.rate,
            lfo2_depth: self.lfo2.depth,
            lfo2_waveform: self.lfo2.waveform,
            lfo3_rate: self.lfo3.rate,
            lfo3_depth: self.lfo3.depth,
            lfo3_waveform: self.lfo3.waveform,
            env3_attack: self.env3_attack,
            env3_decay: self.env3_decay,
            env3_sustain: self.env3_sustain,
            env3_release: self.env3_release,
            env4: self.env4,

            mod_matrix: self.mod_matrix,

            tempo_bpm: self.bpm,
            lfo1_sync: self.lfo1_sync,
            lfo1_beats: self.lfo1_beats,
            lfo2_sync: self.lfo2.sync,
            lfo2_beats: self.lfo2.rate_beats,
            lfo3_sync: self.lfo3.sync,
            lfo3_beats: self.lfo3.rate_beats,
            delay_sync: self.delay_sync,
            delay_beats: self.delay_beats,
            filter_type: self.filter_type,

            pitch_bend_range: if let Some(voice) = self.voices.first() {
                voice.osc_bank().pitch_bend_range()
            } else {
                12
            },

            portamento_time: self.portamento_time,

            master_volume: self.master_volume,

            // Stereo / Pan
            osc1_pan: self.osc1_pan,
            osc2_pan: self.osc2_pan,
            osc3_pan: self.osc3_pan,
            stereo_width: self.stereo_width,

            // Polyphony mode
            polyphony_mode: self.polyphony_mode,

            // Effects are not stored in VoiceManager, use defaults
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: the plugin calls set_polyphony_mode every buffer. When the
    /// mode is unchanged this must NOT release held voices (it used to
    /// note_off all voices unconditionally, killing sustain in the VST).
    #[test]
    fn test_set_polyphony_mode_same_mode_does_not_release_voices() {
        let mut vm = VoiceManager::monophonic(44100);
        let mode = vm.polyphony_mode();

        vm.receive_event(NoteEvent::note_on(69, 1.0));

        // Simulate many buffers, each re-applying the same mode like the
        // plugin's per-buffer sync does
        for _ in 0..100 {
            vm.set_polyphony_mode(mode);
            for _ in 0..128 {
                let _ = vm.next_sample_stereo();
            }
        }

        let (state, _, _, _, _) = vm.debug_voice0();
        assert_ne!(state, EnvelopeState::Release, "held note must not be releasing");
        assert_ne!(state, EnvelopeState::Idle, "held note must still be active");
    }

    /// Test that sustain level is correctly applied to voices
    #[test]
    fn test_voice_manager_sustain_behavior() {
        let mut vm = VoiceManager::monophonic(44100);

        // Set sustain to 50%
        vm.set_sustain(0.5);

        // Trigger note
        vm.receive_event(NoteEvent::note_on(69, 1.0));

        // Run through attack (0.01s = 441 samples at 44.1kHz)
        for _ in 0..500 {
            let _ = vm.next_sample_stereo();
        }

        // Run through decay (0.1s = 4410 samples)
        for _ in 0..5000 {
            let _ = vm.next_sample_stereo();
        }

        // Now should be in sustain phase - get samples
        let mut samples_at_sustain = Vec::new();
        for _ in 0..1000 {
            samples_at_sustain.push(vm.next_sample_stereo());
        }

        // All samples should have similar amplitude (sustaining)
        // With sustain at 0.5 and velocity 1.0, we should see significant amplitude
        let avg_left: f32 = samples_at_sustain.iter().map(|s| s.left.abs()).sum::<f32>() / samples_at_sustain.len() as f32;
        let avg_right: f32 = samples_at_sustain.iter().map(|s| s.right.abs()).sum::<f32>() / samples_at_sustain.len() as f32;

        // Should have significant output (not near zero)
        assert!(avg_left > 0.01 || avg_right > 0.01,
            "Sustain should produce audible output, got avg_left={}, avg_right={}", avg_left, avg_right);
    }

    /// Test that oscillator panning creates stereo separation
    #[test]
    fn test_oscillator_panning_stereo_separation() {
        let mut vm = VoiceManager::monophonic(44100);

        // Set up: osc1 hard left, osc2 hard right, osc3 center
        // Only enable osc1 and osc2 for clearer test
        vm.set_osc_level(1, 1.0);
        vm.set_osc_level(2, 1.0);
        vm.set_osc_level(3, 0.0);  // Disable osc3

        vm.set_osc_pan(1, -1.0);  // Hard left
        vm.set_osc_pan(2, 1.0);   // Hard right
        vm.set_osc_pan(3, 0.0);   // Center (but disabled)

        // Trigger note
        vm.receive_event(NoteEvent::note_on(69, 1.0));

        // Collect stereo samples
        let mut left_channel_sum = 0.0f32;
        let mut right_channel_sum = 0.0f32;
        let sample_count = 1000;

        for _ in 0..sample_count {
            let sample = vm.next_sample_stereo();
            left_channel_sum += sample.left.abs();
            right_channel_sum += sample.right.abs();
        }

        let left_avg = left_channel_sum / sample_count as f32;
        let right_avg = right_channel_sum / sample_count as f32;

        // With osc1 panned left and osc2 panned right, we should have
        // significant energy in both channels
        assert!(left_avg > 0.001, "Left channel should have energy with osc1 panned left, got {}", left_avg);
        assert!(right_avg > 0.001, "Right channel should have energy with osc2 panned right, got {}", right_avg);

        // The channels should be somewhat balanced since we have one osc on each side
        let ratio = if left_avg > right_avg { left_avg / right_avg } else { right_avg / left_avg };
        assert!(ratio < 10.0, "Left and right should be relatively balanced (ratio < 10), got ratio={}", ratio);
    }

    /// Test that extreme panning works correctly
    #[test]
    fn test_extreme_panning() {
        let mut vm = VoiceManager::monophonic(44100);

        // Only osc1 enabled, panned hard left
        vm.set_osc_level(1, 1.0);
        vm.set_osc_level(2, 0.0);
        vm.set_osc_level(3, 0.0);
        vm.set_osc_pan(1, -1.0);

        vm.receive_event(NoteEvent::note_on(69, 1.0));

        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;

        for _ in 0..1000 {
            let sample = vm.next_sample_stereo();
            left_sum += sample.left.abs();
            right_sum += sample.right.abs();
        }

        // With osc1 hard panned left, left should be significantly louder than right
        assert!(left_sum > right_sum * 2.0,
            "Hard left pan should have left > 2x right, got left={}, right={}", left_sum, right_sum);
    }

    /// Test that sustain can be changed dynamically during a playing note (like in a DAW)
    #[test]
    fn test_sustain_dynamic_change() {
        let mut vm = VoiceManager::monophonic(44100);

        // Set initial sustain to 0.0
        vm.set_sustain(0.0);

        // Trigger note
        vm.receive_event(NoteEvent::note_on(69, 1.0));

        // Run through attack + decay to get to sustain phase
        for _ in 0..6000 {
            let _ = vm.next_sample_stereo();
        }

        // Now change sustain to 0.8 (like user moving the knob in DAW)
        vm.set_sustain(0.8);

        // Collect samples after sustain change
        let mut samples_after_change = Vec::new();
        for _ in 0..1000 {
            samples_after_change.push(vm.next_sample_stereo());
        }

        // Calculate average amplitude after sustain change
        let avg_amp: f32 = samples_after_change.iter()
            .map(|s| (s.left.abs() + s.right.abs()) / 2.0)
            .sum::<f32>() / samples_after_change.len() as f32;

        // With sustain at 0.8, we should have significant output
        assert!(avg_amp > 0.05,
            "After changing sustain to 0.8, output should be audible, got avg_amp={}", avg_amp);

        // Now test starting with high sustain, then lowering it
        let mut vm2 = VoiceManager::monophonic(44100);
        vm2.set_sustain(0.9);
        vm2.receive_event(NoteEvent::note_on(69, 1.0));

        // Run to sustain phase
        for _ in 0..6000 {
            let _ = vm2.next_sample_stereo();
        }

        // Lower sustain to 0.1
        vm2.set_sustain(0.1);

        let mut samples_after_lower = Vec::new();
        for _ in 0..1000 {
            samples_after_lower.push(vm2.next_sample_stereo());
        }

        let avg_amp_lower: f32 = samples_after_lower.iter()
            .map(|s| (s.left.abs() + s.right.abs()) / 2.0)
            .sum::<f32>() / samples_after_lower.len() as f32;

        // With sustain lowered to 0.1, output should be much lower
        assert!(avg_amp_lower < avg_amp * 0.5,
            "Lowering sustain should reduce output, got avg_amp_lower={} vs previous {}"
            , avg_amp_lower, avg_amp);
    }

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

    /// Regression: mod-on-mod routes (LFO rate/depth destinations) are applied
    /// around the stored base values, and removing the routes restores them.
    #[test]
    fn test_mod_on_mod_uses_base_values() {
        let mut voice = Voice::new(44100);
        voice.set_lfo_rate(2.0);
        voice.set_lfo_depth(0.5);
        voice.note_on(69, 1.0);

        let mut m = ModMatrix::default();
        m.slots[0] = ModSlot {
            source: ModSource::Constant,
            dest: ModDest::Lfo1Rate,
            amount: 1.0,
            enabled: true,
        };
        m.slots[1] = ModSlot {
            source: ModSource::Constant,
            dest: ModDest::Lfo1Depth,
            amount: -1.0,
            enabled: true,
        };
        voice.set_mod_matrix(m);
        for _ in 0..64 {
            let _ = voice.next_sample();
        }
        // Rate: +3 octaves at full amount → 2 Hz * 2^3 = 16 Hz; depth clamps to 0.
        assert!(
            (voice.lfos[0].rate() - 16.0).abs() < 1e-3,
            "expected modulated rate 16 Hz, got {}",
            voice.lfos[0].rate()
        );
        assert_eq!(voice.lfos[0].depth(), 0.0);

        // Routes removed → base rate/depth restored.
        voice.set_mod_matrix(ModMatrix::default());
        assert!((voice.lfos[0].rate() - 2.0).abs() < 1e-3);
        assert_eq!(voice.lfos[0].depth(), 0.5);
    }

    /// Regression: routing all oscillator levels down to zero silences the
    /// voice even while the amplitude envelope is open.
    #[test]
    fn test_osc_level_routes_silence_voice() {
        let mut voice = Voice::new(44100);
        let mut m = ModMatrix::default();
        for (i, d) in [ModDest::Osc1Level, ModDest::Osc2Level, ModDest::Osc3Level]
            .iter()
            .enumerate()
        {
            m.slots[i] = ModSlot {
                source: ModSource::Constant,
                dest: *d,
                amount: -1.0,
                enabled: true,
            };
        }
        voice.set_mod_matrix(m);
        voice.note_on(69, 1.0);
        for _ in 0..2000 {
            assert_eq!(voice.next_sample(), 0.0);
        }
    }

    /// Regression: the StereoWidth destination collapses a panned voice to
    /// mono at full negative amount.
    #[test]
    fn test_stereo_width_route_collapses_to_mono() {
        let mut voice = Voice::new(44100);
        let mut m = ModMatrix::default();
        m.slots[0] = ModSlot {
            source: ModSource::Constant,
            dest: ModDest::StereoWidth,
            amount: -1.0,
            enabled: true,
        };
        voice.set_mod_matrix(m);
        voice.note_on(69, 1.0);
        let mut audible = false;
        for _ in 0..2000 {
            let s = voice.next_sample_stereo((-0.8, 0.0, 0.8));
            assert_eq!(s.left, s.right, "width 0 must collapse to mono");
            if s.left != 0.0 {
                audible = true;
            }
        }
        assert!(audible);
    }

    /// Regression: per-oscillator pan routes shift the stereo image.
    #[test]
    fn test_pan_route_shifts_image() {
        let max_diff = |m: ModMatrix| {
            let mut v = Voice::new(44100);
            v.set_mod_matrix(m);
            v.note_on(69, 1.0);
            let mut d = 0.0f32;
            for _ in 0..2000 {
                let s = v.next_sample_stereo((0.0, 0.0, 0.0));
                d = d.max((s.left - s.right).abs());
            }
            d
        };
        let centered = max_diff(ModMatrix::default());
        let mut m = ModMatrix::default();
        m.slots[0] = ModSlot {
            source: ModSource::Constant,
            dest: ModDest::Osc1Pan,
            amount: -1.0,
            enabled: true,
        };
        let panned = max_diff(m);
        assert_eq!(centered, 0.0, "centered oscillators must stay centered");
        assert!(panned > 0.01, "pan route must create stereo separation");
    }

    /// Regression: aftertouch and pitch bend are live modulation sources.
    #[test]
    fn test_performance_sources_modulate() {
        let mk = |src: ModSource| {
            let mut m = ModMatrix::default();
            m.slots[0] = ModSlot {
                source: src,
                dest: ModDest::Amplitude,
                amount: -1.0,
                enabled: true,
            };
            m
        };
        // Full pressure pushes the amplitude route to zero.
        let mut v = Voice::new(44100);
        v.set_mod_matrix(mk(ModSource::Aftertouch));
        v.set_aftertouch(1.0);
        v.note_on(69, 1.0);
        for _ in 0..1000 {
            assert_eq!(v.next_sample(), 0.0);
        }

        // Same for pitch bend fully up.
        let mut v = Voice::new(44100);
        v.set_mod_matrix(mk(ModSource::PitchBend));
        v.set_pitch_bend_mod(1.0);
        v.note_on(69, 1.0);
        for _ in 0..1000 {
            assert_eq!(v.next_sample(), 0.0);
        }
    }

    /// Regression: Env4 is a live modulation source (same wiring as Env3).
    #[test]
    fn test_env4_source_modulates() {
        let mut voice = Voice::new(44100);
        // Long attack so Env4 is mid-rise while samples are produced.
        voice.set_env4_adsr([0.5, 0.1, 1.0, 0.1]);
        let mut m = ModMatrix::default();
        m.slots[0] = ModSlot {
            source: ModSource::Env4,
            dest: ModDest::Amplitude,
            amount: -1.0,
            enabled: true,
        };
        voice.set_mod_matrix(m);
        voice.note_on(69, 1.0);

        // Sample 1: Env4 ≈ 0 → gain ≈ 1 (audible). Later: Env4 ≈ high →
        // amplitude_gain = 1 - env4 < 1 (quieter).
        let first = voice.next_sample().abs();
        for _ in 0..22050 {
            let _ = voice.next_sample();
        }
        let later = (0..200).map(|_| voice.next_sample().abs()).fold(0.0f32, f32::max);
        assert!(later <= first || first == 0.0, "env4 should attenuate over time");
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

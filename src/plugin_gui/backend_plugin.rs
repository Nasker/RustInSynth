//! Plugin backend: implements `SynthBackend` over `Arc<RustInSynthParams>` + `ParamSetter`.
//!
//! The plugin backend has no MIDI ports (the host handles that), no audio engine
//! ownership, and no CC mappings.  Parameter reads go straight to the NIH-plug
//! atomic param storage; writes go through `ParamSetter` so the host is notified.

use std::sync::Arc;

use nih_plug::prelude::ParamSetter;

use crate::core::params::SynthParam;
use crate::core::voice::PolyphonyMode;
use crate::gui::backend::{CcMapping, MidiLearnState, SynthBackend};
use crate::plugin::RustInSynthParams;
use crate::plugin_gui::shared_state::PluginSharedState;

/// Backend implementation for the NIH-plug plugin.
///
/// Reads come from `Arc<RustInSynthParams>` (atomic, lock-free).
/// Writes go through `ParamSetter<'_>` so the DAW records automation.
///
/// Because `ParamSetter` holds a lifetime-bound reference to the plugin
/// context, we cannot store it directly — instead `PluginBackend` is
/// re-created on every GUI frame (it's cheap: just two Arc clones).
pub struct PluginBackend<'a> {
    params: Arc<RustInSynthParams>,
    setter: &'a ParamSetter<'a>,
    shared: PluginSharedState,
}

impl<'a> PluginBackend<'a> {
    pub fn new(
        params: Arc<RustInSynthParams>,
        setter: &'a ParamSetter<'a>,
        shared: PluginSharedState,
    ) -> Self {
        Self { params, setter, shared }
    }
}

// ============================================================================
// SynthBackend implementation
// ============================================================================

impl<'a> SynthBackend for PluginBackend<'a> {
    // ── Parameters ───────────────────────────────────────────────────────────

    fn get_param(&self, param: SynthParam) -> f32 {
        match param {
            SynthParam::Attack => self.params.attack.value(),
            SynthParam::Decay => self.params.decay.value(),
            SynthParam::Sustain => self.params.sustain.value(),
            SynthParam::Release => self.params.release.value(),
            SynthParam::FilterCutoff => self.params.cutoff.value(),
            SynthParam::FilterResonance => self.params.resonance.value(),
            SynthParam::FilterAttack => self.params.f_attack.value(),
            SynthParam::FilterDecay => self.params.f_decay.value(),
            SynthParam::FilterSustain => self.params.f_sustain.value(),
            SynthParam::FilterRelease => self.params.f_release.value(),
            SynthParam::FilterEnvAmount => self.params.f_amount.value(),
            SynthParam::LfoRate => self.params.lfo_rate.value(),
            SynthParam::LfoDepth => self.params.lfo_depth.value(),
            SynthParam::LfoWaveform => self.params.lfo_waveform.value() as f32,
            SynthParam::LfoDestination => self.params.lfo_destination.value() as f32,
            SynthParam::Osc1Waveform => self.params.osc1_waveform.value() as f32,
            SynthParam::Osc1Level => self.params.osc1_level.value(),
            SynthParam::Osc1Phase => self.params.osc1_phase.value(),
            SynthParam::Osc2Waveform => self.params.osc2_waveform.value() as f32,
            SynthParam::Osc2Level => self.params.osc2_level.value(),
            SynthParam::Osc2Semitones => self.params.osc2_semitones.value() as f32,
            SynthParam::Osc2Cents => self.params.osc2_cents.value() as f32,
            SynthParam::Osc2Phase => self.params.osc2_phase.value(),
            SynthParam::Osc3Waveform => self.params.osc3_waveform.value() as f32,
            SynthParam::Osc3Level => self.params.osc3_level.value(),
            SynthParam::Osc3Semitones => self.params.osc3_semitones.value() as f32,
            SynthParam::Osc3Cents => self.params.osc3_cents.value() as f32,
            SynthParam::Osc3Phase => self.params.osc3_phase.value(),
            SynthParam::PortamentoTime => self.params.portamento.value(),
            SynthParam::PitchBendRange => self.params.pitch_bend_range.value() as f32,
            SynthParam::MasterVolume => self.params.master_volume.value(),
        }
    }

    fn set_param(&mut self, param: SynthParam, value: f32) {
        match param {
            SynthParam::Attack => self.setter.set_parameter(&self.params.attack, value),
            SynthParam::Decay => self.setter.set_parameter(&self.params.decay, value),
            SynthParam::Sustain => self.setter.set_parameter(&self.params.sustain, value),
            SynthParam::Release => self.setter.set_parameter(&self.params.release, value),
            SynthParam::FilterCutoff => self.setter.set_parameter(&self.params.cutoff, value),
            SynthParam::FilterResonance => self.setter.set_parameter(&self.params.resonance, value),
            SynthParam::FilterAttack => self.setter.set_parameter(&self.params.f_attack, value),
            SynthParam::FilterDecay => self.setter.set_parameter(&self.params.f_decay, value),
            SynthParam::FilterSustain => self.setter.set_parameter(&self.params.f_sustain, value),
            SynthParam::FilterRelease => self.setter.set_parameter(&self.params.f_release, value),
            SynthParam::FilterEnvAmount => self.setter.set_parameter(&self.params.f_amount, value),
            SynthParam::LfoRate => self.setter.set_parameter(&self.params.lfo_rate, value),
            SynthParam::LfoDepth => self.setter.set_parameter(&self.params.lfo_depth, value),
            SynthParam::LfoWaveform => {
                self.setter.set_parameter(&self.params.lfo_waveform, value as i32)
            }
            SynthParam::LfoDestination => {
                self.setter.set_parameter(&self.params.lfo_destination, value as i32)
            }
            SynthParam::Osc1Waveform => {
                self.setter.set_parameter(&self.params.osc1_waveform, value as i32)
            }
            SynthParam::Osc1Level => self.setter.set_parameter(&self.params.osc1_level, value),
            SynthParam::Osc1Phase => self.setter.set_parameter(&self.params.osc1_phase, value),
            SynthParam::Osc2Waveform => {
                self.setter.set_parameter(&self.params.osc2_waveform, value as i32)
            }
            SynthParam::Osc2Level => self.setter.set_parameter(&self.params.osc2_level, value),
            SynthParam::Osc2Semitones => {
                self.setter.set_parameter(&self.params.osc2_semitones, value as i32)
            }
            SynthParam::Osc2Cents => {
                self.setter.set_parameter(&self.params.osc2_cents, value as i32)
            }
            SynthParam::Osc2Phase => self.setter.set_parameter(&self.params.osc2_phase, value),
            SynthParam::Osc3Waveform => {
                self.setter.set_parameter(&self.params.osc3_waveform, value as i32)
            }
            SynthParam::Osc3Level => self.setter.set_parameter(&self.params.osc3_level, value),
            SynthParam::Osc3Semitones => {
                self.setter.set_parameter(&self.params.osc3_semitones, value as i32)
            }
            SynthParam::Osc3Cents => {
                self.setter.set_parameter(&self.params.osc3_cents, value as i32)
            }
            SynthParam::Osc3Phase => self.setter.set_parameter(&self.params.osc3_phase, value),
            SynthParam::PortamentoTime => {
                self.setter.set_parameter(&self.params.portamento, value)
            }
            SynthParam::PitchBendRange => {
                self.setter.set_parameter(&self.params.pitch_bend_range, value as i32)
            }
            SynthParam::MasterVolume => {
                self.setter.set_parameter(&self.params.master_volume, value)
            }
        }
    }

    // ── Effects: Delay ───────────────────────────────────────────────────────

    fn delay_enabled(&self) -> bool {
        self.params.delay_enabled.value()
    }
    fn set_delay_enabled(&mut self, enabled: bool) {
        self.setter.set_parameter(&self.params.delay_enabled, enabled);
    }
    fn delay_time(&self) -> f32 {
        self.params.delay_time.value()
    }
    fn set_delay_time(&mut self, time: f32) {
        self.setter.set_parameter(&self.params.delay_time, time);
    }
    fn delay_feedback(&self) -> f32 {
        self.params.delay_feedback.value()
    }
    fn set_delay_feedback(&mut self, feedback: f32) {
        self.setter.set_parameter(&self.params.delay_feedback, feedback);
    }
    fn delay_mix(&self) -> f32 {
        self.params.delay_mix.value()
    }
    fn set_delay_mix(&mut self, mix: f32) {
        self.setter.set_parameter(&self.params.delay_mix, mix);
    }

    // ── Effects: Reverb ──────────────────────────────────────────────────────

    fn reverb_enabled(&self) -> bool {
        self.params.reverb_enabled.value()
    }
    fn set_reverb_enabled(&mut self, enabled: bool) {
        self.setter.set_parameter(&self.params.reverb_enabled, enabled);
    }
    fn reverb_room_size(&self) -> f32 {
        self.params.reverb_room_size.value()
    }
    fn set_reverb_room_size(&mut self, size: f32) {
        self.setter.set_parameter(&self.params.reverb_room_size, size);
    }
    fn reverb_damping(&self) -> f32 {
        self.params.reverb_damping.value()
    }
    fn set_reverb_damping(&mut self, damping: f32) {
        self.setter.set_parameter(&self.params.reverb_damping, damping);
    }
    fn reverb_mix(&self) -> f32 {
        self.params.reverb_mix.value()
    }
    fn set_reverb_mix(&mut self, mix: f32) {
        self.setter.set_parameter(&self.params.reverb_mix, mix);
    }

    // ── Effects: Chorus ──────────────────────────────────────────────────────

    fn chorus_enabled(&self) -> bool {
        self.params.chorus_enabled.value()
    }
    fn set_chorus_enabled(&mut self, enabled: bool) {
        self.setter.set_parameter(&self.params.chorus_enabled, enabled);
    }
    fn chorus_rate(&self) -> f32 {
        self.params.chorus_rate.value()
    }
    fn set_chorus_rate(&mut self, rate: f32) {
        self.setter.set_parameter(&self.params.chorus_rate, rate);
    }
    fn chorus_depth(&self) -> f32 {
        self.params.chorus_depth.value()
    }
    fn set_chorus_depth(&mut self, depth: f32) {
        self.setter.set_parameter(&self.params.chorus_depth, depth);
    }
    fn chorus_mix(&self) -> f32 {
        self.params.chorus_mix.value()
    }
    fn set_chorus_mix(&mut self, mix: f32) {
        self.setter.set_parameter(&self.params.chorus_mix, mix);
    }

    // ── Voice management ─────────────────────────────────────────────────────

    fn polyphony_mode(&self) -> PolyphonyMode {
        if self.params.polyphony_mode.value() == 0 {
            PolyphonyMode::Mono
        } else {
            PolyphonyMode::Poly
        }
    }
    fn set_polyphony_mode(&mut self, mode: PolyphonyMode) {
        let val = match mode {
            PolyphonyMode::Mono => 0,
            PolyphonyMode::Poly => 1,
        };
        self.setter.set_parameter(&self.params.polyphony_mode, val);
    }
    fn active_voice_count(&self) -> usize {
        self.shared.voice_count()
    }
    fn max_voices(&self) -> usize {
        self.shared.max_voices()
    }

    // ── Stereo ───────────────────────────────────────────────────────────────

    fn stereo_width(&self) -> f32 {
        self.params.stereo_width.value()
    }
    fn set_stereo_width(&mut self, width: f32) {
        self.setter.set_parameter(&self.params.stereo_width, width);
    }
    fn osc_pan(&self, osc_num: u8) -> f32 {
        match osc_num {
            1 => self.params.osc1_pan.value(),
            2 => self.params.osc2_pan.value(),
            3 => self.params.osc3_pan.value(),
            _ => 0.0,
        }
    }
    fn set_osc_pan(&mut self, osc_num: u8, pan: f32) {
        match osc_num {
            1 => self.setter.set_parameter(&self.params.osc1_pan, pan),
            2 => self.setter.set_parameter(&self.params.osc2_pan, pan),
            3 => self.setter.set_parameter(&self.params.osc3_pan, pan),
            _ => {}
        }
    }

    // ── MIDI (not applicable in plugin mode) ─────────────────────────────────

    fn midi_connected(&self) -> bool { false }
    fn midi_ports(&self) -> Vec<String> { Vec::new() }
    fn selected_midi_port(&self) -> Option<usize> { None }
    fn connect_midi_port(&mut self, _port_idx: usize) {}
    fn refresh_midi_ports(&mut self) {}
    fn midi_channel_display(&self) -> u8 { 0 }
    fn set_midi_channel(&mut self, _display_idx: u8) {}
    fn midi_cc_activity(&self) -> Vec<(u8, u8)> { Vec::new() }
    fn midi_learn_state(&self) -> MidiLearnState { MidiLearnState::Idle }
    fn start_midi_learn(&mut self, _param: SynthParam) {}
    fn cancel_midi_learn(&mut self) {}
    fn custom_cc_mappings(&self) -> Vec<CcMapping> { Vec::new() }
    fn clear_cc_mappings(&mut self) {}

    // ── Status ───────────────────────────────────────────────────────────────

    fn get_cpu_load(&self) -> f32 {
        self.shared.get_cpu_load()
    }

    // update() default (no-op) is sufficient for the plugin backend
}

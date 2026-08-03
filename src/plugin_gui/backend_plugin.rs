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

/// Bind `$p` to the nih parameter field matching `$param` and run the
/// float- or int-specific body. Single source for the SynthParam → field
/// mapping used by `get_param`, `set_param` and the gesture hooks.
macro_rules! with_nih_param {
    ($self:ident, $param:expr, $p:ident, $float_body:expr, $int_body:expr) => {
        match $param {
            SynthParam::Attack => { let $p = &$self.params.attack; $float_body }
            SynthParam::Decay => { let $p = &$self.params.decay; $float_body }
            SynthParam::Sustain => { let $p = &$self.params.sustain; $float_body }
            SynthParam::Release => { let $p = &$self.params.release; $float_body }
            SynthParam::FilterCutoff => { let $p = &$self.params.cutoff; $float_body }
            SynthParam::FilterResonance => { let $p = &$self.params.resonance; $float_body }
            SynthParam::FilterAttack => { let $p = &$self.params.f_attack; $float_body }
            SynthParam::FilterDecay => { let $p = &$self.params.f_decay; $float_body }
            SynthParam::FilterSustain => { let $p = &$self.params.f_sustain; $float_body }
            SynthParam::FilterRelease => { let $p = &$self.params.f_release; $float_body }
            SynthParam::FilterEnvAmount => { let $p = &$self.params.f_amount; $float_body }
            SynthParam::LfoRate => { let $p = &$self.params.lfo_rate; $float_body }
            SynthParam::LfoDepth => { let $p = &$self.params.lfo_depth; $float_body }
            SynthParam::LfoWaveform => { let $p = &$self.params.lfo_waveform; $int_body }
            SynthParam::LfoDestination => { let $p = &$self.params.lfo_destination; $int_body }
            SynthParam::PitchBendRange => { let $p = &$self.params.pitch_bend_range; $int_body }
            SynthParam::PortamentoTime => { let $p = &$self.params.portamento; $float_body }
            SynthParam::Osc1Waveform => { let $p = &$self.params.osc1_waveform; $int_body }
            SynthParam::Osc1Level => { let $p = &$self.params.osc1_level; $float_body }
            SynthParam::Osc1Phase => { let $p = &$self.params.osc1_phase; $float_body }
            SynthParam::Osc1Pan => { let $p = &$self.params.osc1_pan; $float_body }
            SynthParam::Osc2Waveform => { let $p = &$self.params.osc2_waveform; $int_body }
            SynthParam::Osc2Level => { let $p = &$self.params.osc2_level; $float_body }
            SynthParam::Osc2Semitones => { let $p = &$self.params.osc2_semitones; $int_body }
            SynthParam::Osc2Cents => { let $p = &$self.params.osc2_cents; $int_body }
            SynthParam::Osc2Phase => { let $p = &$self.params.osc2_phase; $float_body }
            SynthParam::Osc2Pan => { let $p = &$self.params.osc2_pan; $float_body }
            SynthParam::Osc3Waveform => { let $p = &$self.params.osc3_waveform; $int_body }
            SynthParam::Osc3Level => { let $p = &$self.params.osc3_level; $float_body }
            SynthParam::Osc3Semitones => { let $p = &$self.params.osc3_semitones; $int_body }
            SynthParam::Osc3Cents => { let $p = &$self.params.osc3_cents; $int_body }
            SynthParam::Osc3Phase => { let $p = &$self.params.osc3_phase; $float_body }
            SynthParam::Osc3Pan => { let $p = &$self.params.osc3_pan; $float_body }
            SynthParam::StereoWidth => { let $p = &$self.params.stereo_width; $float_body }
            SynthParam::MasterVolume => { let $p = &$self.params.master_volume; $float_body }
        }
    };
}

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
        with_nih_param!(self, param, p, p.value(), p.value() as f32)
    }

    fn set_param(&mut self, param: SynthParam, value: f32) {
        with_nih_param!(
            self,
            param,
            p,
            self.setter.set_parameter(p, value),
            self.setter.set_parameter(p, value as i32)
        )
    }

    fn begin_param_change(&mut self, param: SynthParam) {
        with_nih_param!(
            self,
            param,
            p,
            self.setter.begin_set_parameter(p),
            self.setter.begin_set_parameter(p)
        )
    }

    fn end_param_change(&mut self, param: SynthParam) {
        with_nih_param!(
            self,
            param,
            p,
            self.setter.end_set_parameter(p),
            self.setter.end_set_parameter(p)
        )
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

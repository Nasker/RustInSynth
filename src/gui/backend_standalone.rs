//! Standalone backend: wraps `AudioEngine`, `SharedState`, and `MidiInputHandler`.

use std::collections::HashMap;

use crate::audio::AudioEngine;
use crate::core::event::NoteEventKind;
use crate::core::param_spec::cc_to_plain;
use crate::core::params::{
    load_custom_cc_mappings, save_custom_cc_mappings, CCMapping, SynthParam,
};
use crate::core::voice::PolyphonyMode;
use crate::gui::SharedState;
use crate::gui::backend::{CcMapping, MidiLearnState, SynthBackend};
use crate::input::midi::MidiInputHandler;

// ============================================================================
// StandaloneBackend
// ============================================================================

/// Backend implementation for the standalone (non-plugin) application.
///
/// Owns the `AudioEngine`, `SharedState`, and optional `MidiInputHandler`.
pub struct StandaloneBackend {
    pub shared: SharedState,
    pub audio_engine: AudioEngine,

    midi_handler: Option<MidiInputHandler>,
    midi_ports: Vec<String>,
    selected_midi_port: Option<usize>,

    midi_learn_state: MidiLearnState,
    custom_cc_map: HashMap<u8, SynthParam>,
    /// Factory default CC → parameter mapping (custom map takes precedence)
    default_cc_mapping: CCMapping,
}

impl StandaloneBackend {
    /// Create and start the standalone backend (starts audio, auto-connects MIDI).
    pub fn new(shared: SharedState) -> Self {
        let cpu_load = std::sync::Arc::clone(&shared.cpu_load);
        let audio_engine = match AudioEngine::new(cpu_load) {
            Ok(mut engine) => {
                engine.set_master_volume(0.5);
                if let Err(e) = engine.start() {
                    eprintln!("Failed to start audio: {}", e);
                }
                engine
            }
            Err(e) => {
                panic!("Audio engine required: {}", e);
            }
        };

        let midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
        let selected_midi_port = if !midi_ports.is_empty() { Some(0) } else { None };
        let midi_handler = MidiInputHandler::connect_auto().ok();
        if midi_handler.is_some() {
            println!("MIDI connected!");
        }

        let custom_cc_map = load_custom_cc_mappings();

        Self {
            shared,
            audio_engine,
            midi_handler,
            midi_ports,
            selected_midi_port,
            midi_learn_state: MidiLearnState::Idle,
            custom_cc_map,
            default_cc_mapping: CCMapping::default_mappings(),
        }
    }

    // ========================================================================
    // Internal helpers (mirror the logic previously in SynthApp)
    // ========================================================================

    /// Scale a CC value through the parameter's spec (range + curve) — the
    /// exact same conversion the plugin and the DSP-side CC handler use.
    fn apply_cc_to_param(&mut self, param: SynthParam, value: u8) {
        self.shared.params.set(param, cc_to_plain(param, value));
    }

    fn update_param_from_cc(&mut self, cc: u8, value: u8) {
        let param = self
            .custom_cc_map
            .get(&cc)
            .copied()
            .or_else(|| self.default_cc_mapping.get_param(cc));
        if let Some(param) = param {
            self.apply_cc_to_param(param, value);
        }
    }
}

// ============================================================================
// SynthBackend implementation
// ============================================================================

impl SynthBackend for StandaloneBackend {
    // ── Parameters ───────────────────────────────────────────────────────────

    fn get_param(&self, param: SynthParam) -> f32 {
        self.shared.params.get(param)
    }

    fn set_param(&mut self, param: SynthParam, value: f32) {
        self.shared.params.set(param, value);
    }

    // ── Effects: Delay ───────────────────────────────────────────────────────

    fn delay_enabled(&self) -> bool {
        self.audio_engine.delay_enabled()
    }
    fn set_delay_enabled(&mut self, enabled: bool) {
        self.audio_engine.set_delay_enabled(enabled);
    }
    fn delay_time(&self) -> f32 {
        self.audio_engine.delay_time()
    }
    fn set_delay_time(&mut self, time: f32) {
        self.audio_engine.set_delay_time(time);
    }
    fn delay_feedback(&self) -> f32 {
        self.audio_engine.delay_feedback()
    }
    fn set_delay_feedback(&mut self, feedback: f32) {
        self.audio_engine.set_delay_feedback(feedback);
    }
    fn delay_mix(&self) -> f32 {
        self.audio_engine.delay_mix()
    }
    fn set_delay_mix(&mut self, mix: f32) {
        self.audio_engine.set_delay_mix(mix);
    }

    // ── Effects: Reverb ──────────────────────────────────────────────────────

    fn reverb_enabled(&self) -> bool {
        self.audio_engine.reverb_enabled()
    }
    fn set_reverb_enabled(&mut self, enabled: bool) {
        self.audio_engine.set_reverb_enabled(enabled);
    }
    fn reverb_room_size(&self) -> f32 {
        self.audio_engine.reverb_room_size()
    }
    fn set_reverb_room_size(&mut self, size: f32) {
        self.audio_engine.set_reverb_room_size(size);
    }
    fn reverb_damping(&self) -> f32 {
        self.audio_engine.reverb_damping()
    }
    fn set_reverb_damping(&mut self, damping: f32) {
        self.audio_engine.set_reverb_damping(damping);
    }
    fn reverb_mix(&self) -> f32 {
        self.audio_engine.reverb_mix()
    }
    fn set_reverb_mix(&mut self, mix: f32) {
        self.audio_engine.set_reverb_mix(mix);
    }

    // ── Effects: Chorus ──────────────────────────────────────────────────────

    fn chorus_enabled(&self) -> bool {
        self.audio_engine.chorus_enabled()
    }
    fn set_chorus_enabled(&mut self, enabled: bool) {
        self.audio_engine.set_chorus_enabled(enabled);
    }
    fn chorus_rate(&self) -> f32 {
        self.audio_engine.chorus_rate()
    }
    fn set_chorus_rate(&mut self, rate: f32) {
        self.audio_engine.set_chorus_rate(rate);
    }
    fn chorus_depth(&self) -> f32 {
        self.audio_engine.chorus_depth()
    }
    fn set_chorus_depth(&mut self, depth: f32) {
        self.audio_engine.set_chorus_depth(depth);
    }
    fn chorus_mix(&self) -> f32 {
        self.audio_engine.chorus_mix()
    }
    fn set_chorus_mix(&mut self, mix: f32) {
        self.audio_engine.set_chorus_mix(mix);
    }

    // ── Voice management ─────────────────────────────────────────────────────

    fn polyphony_mode(&self) -> PolyphonyMode {
        self.audio_engine.polyphony_mode()
    }
    fn set_polyphony_mode(&mut self, mode: PolyphonyMode) {
        self.audio_engine.set_polyphony_mode(mode);
    }
    fn active_voice_count(&self) -> usize {
        self.audio_engine.active_voice_count()
    }
    fn max_voices(&self) -> usize {
        self.audio_engine.max_voices()
    }

    // ── Stereo ───────────────────────────────────────────────────────────────

    fn stereo_width(&self) -> f32 {
        self.shared.params.get(SynthParam::StereoWidth)
    }
    fn set_stereo_width(&mut self, width: f32) {
        self.shared.params.set(SynthParam::StereoWidth, width);
    }
    fn osc_pan(&self, osc_num: u8) -> f32 {
        match osc_num {
            1 => self.shared.params.get(SynthParam::Osc1Pan),
            2 => self.shared.params.get(SynthParam::Osc2Pan),
            3 => self.shared.params.get(SynthParam::Osc3Pan),
            _ => 0.0,
        }
    }
    fn set_osc_pan(&mut self, osc_num: u8, pan: f32) {
        match osc_num {
            1 => self.shared.params.set(SynthParam::Osc1Pan, pan),
            2 => self.shared.params.set(SynthParam::Osc2Pan, pan),
            3 => self.shared.params.set(SynthParam::Osc3Pan, pan),
            _ => {}
        }
    }

    // ── MIDI ─────────────────────────────────────────────────────────────────

    fn midi_connected(&self) -> bool {
        self.midi_handler.is_some()
    }

    fn midi_ports(&self) -> Vec<String> {
        self.midi_ports.clone()
    }

    fn selected_midi_port(&self) -> Option<usize> {
        self.selected_midi_port
    }

    fn connect_midi_port(&mut self, port_idx: usize) {
        match MidiInputHandler::connect(port_idx, None) {
            Ok(handler) => {
                self.midi_handler = Some(handler);
                self.selected_midi_port = Some(port_idx);
                println!("Connected to MIDI port {}", port_idx);
            }
            Err(e) => {
                eprintln!("Failed to connect MIDI: {}", e);
            }
        }
    }

    fn refresh_midi_ports(&mut self) {
        self.midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
        if self.midi_ports.is_empty() {
            self.selected_midi_port = None;
        }
    }

    fn midi_channel_display(&self) -> u8 {
        self.midi_handler
            .as_ref()
            .map(|h| h.channel_display())
            .unwrap_or(0)
    }

    fn set_midi_channel(&mut self, display_idx: u8) {
        if let Some(ref handler) = self.midi_handler {
            handler.set_channel_from_display(display_idx);
        }
    }

    fn midi_cc_activity(&self) -> Vec<(u8, u8)> {
        let mut list: Vec<_> = self
            .shared
            .midi_feedback
            .iter()
            .map(|e| (*e.key(), *e.value()))
            .collect();
        list.sort_by_key(|(cc, _)| *cc);
        list
    }

    fn midi_learn_state(&self) -> MidiLearnState {
        self.midi_learn_state.clone()
    }

    fn start_midi_learn(&mut self, param: SynthParam) {
        self.midi_learn_state = MidiLearnState::Waiting(param);
    }

    fn cancel_midi_learn(&mut self) {
        self.midi_learn_state = MidiLearnState::Idle;
    }

    fn custom_cc_mappings(&self) -> Vec<CcMapping> {
        self.custom_cc_map
            .iter()
            .map(|(&cc, &param)| CcMapping { cc, param })
            .collect()
    }

    fn clear_cc_mappings(&mut self) {
        self.custom_cc_map.clear();
        let _ = save_custom_cc_mappings(&self.custom_cc_map);
    }

    // ── Status ───────────────────────────────────────────────────────────────

    fn get_cpu_load(&self) -> f32 {
        self.shared.get_cpu_load()
    }

    // ── Per-frame update ─────────────────────────────────────────────────────

    fn update(&mut self) {
        // Drain MIDI events
        let events: Vec<_> = if let Some(ref midi) = self.midi_handler {
            let mut evts = Vec::new();
            for _ in 0..64 {
                match midi.poll() {
                    Some(event) => evts.push(event),
                    None => break,
                }
            }
            evts
        } else {
            Vec::new()
        };

        for event in events {
            self.audio_engine.send_event(event.clone());

            if let NoteEventKind::ControlChange { cc, value } = event.kind {
                self.shared.midi_feedback.insert(cc, value);

                if let MidiLearnState::Waiting(target_param) =
                    std::mem::replace(&mut self.midi_learn_state, MidiLearnState::Idle)
                {
                    self.custom_cc_map.insert(cc, target_param);
                    if let Err(e) = save_custom_cc_mappings(&self.custom_cc_map) {
                        eprintln!("Failed to save CC mappings: {}", e);
                    }
                    println!("Mapped CC {} → {}", cc, target_param.name());
                }

                self.update_param_from_cc(cc, value);
            }
        }

        // Sync params to audio engine every frame
        self.audio_engine.sync_params(&self.shared.params);
    }
}

# Unified GUI Architecture: One Codebase for Standalone & Plugin

## Goal
**Write the GUI once, run it in both standalone and plugin contexts.**

Abstract away the differences in parameter access, audio control, and MIDI handling so the actual UI code is 100% shared.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│           Shared GUI Code (src/gui/)                    │
│  ┌───────────────────────────────────────────────────┐  │
│  │  SynthApp (UI logic, widgets, layout)             │  │
│  │  - Oscillators, Filter, Envelopes, LFO, Effects   │  │
│  │  - Presets, MIDI Learn, Status displays           │  │
│  └───────────────────────────────────────────────────┘  │
│                        ↓ uses                            │
│  ┌───────────────────────────────────────────────────┐  │
│  │  SynthBackend trait (abstraction layer)           │  │
│  │  - get_param() / set_param()                      │  │
│  │  - get_effect_state() / set_effect_state()        │  │
│  │  - get_midi_ports() / connect_midi()              │  │
│  │  - get_cpu_load() / get_voice_count()             │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                           ↓ implemented by
        ┌──────────────────────────────────────────┐
        │                                          │
┌───────▼────────┐                      ┌─────────▼────────┐
│  Standalone    │                      │  Plugin          │
│  Backend       │                      │  Backend         │
│                │                      │                  │
│ - ParamBank    │                      │ - NIH-plug       │
│ - AudioEngine  │                      │   Params         │
│ - MidiHandler  │                      │ - ParamSetter    │
│ - Direct       │                      │ - Atomic state   │
│   control      │                      │ - DAW MIDI       │
└────────────────┘                      └──────────────────┘
```

---

## Core Abstraction: `SynthBackend` Trait

```rust
// src/gui/backend.rs

use crate::core::params::SynthParam;
use crate::core::voice::PolyphonyMode;

/// Abstraction over standalone vs plugin parameter/audio access
pub trait SynthBackend: Send + Sync {
    // ============ PARAMETERS ============
    
    /// Get a parameter value
    fn get_param(&self, param: SynthParam) -> f32;
    
    /// Set a parameter value
    fn set_param(&self, param: SynthParam, value: f32);
    
    // ============ EFFECTS ============
    
    /// Get delay enabled state
    fn delay_enabled(&self) -> bool;
    fn set_delay_enabled(&self, enabled: bool);
    fn delay_time(&self) -> f32;
    fn set_delay_time(&self, time: f32);
    fn delay_feedback(&self) -> f32;
    fn set_delay_feedback(&self, feedback: f32);
    fn delay_mix(&self) -> f32;
    fn set_delay_mix(&self, mix: f32);
    
    /// Get reverb state
    fn reverb_enabled(&self) -> bool;
    fn set_reverb_enabled(&self, enabled: bool);
    fn reverb_room_size(&self) -> f32;
    fn set_reverb_room_size(&self, size: f32);
    fn reverb_damping(&self) -> f32;
    fn set_reverb_damping(&self, damping: f32);
    fn reverb_mix(&self) -> f32;
    fn set_reverb_mix(&self, mix: f32);
    
    /// Get chorus state
    fn chorus_enabled(&self) -> bool;
    fn set_chorus_enabled(&self, enabled: bool);
    fn chorus_rate(&self) -> f32;
    fn set_chorus_rate(&self, rate: f32);
    fn chorus_depth(&self) -> f32;
    fn set_chorus_depth(&self, depth: f32);
    fn chorus_mix(&self) -> f32;
    fn set_chorus_mix(&self, mix: f32);
    
    // ============ VOICE & POLYPHONY ============
    
    fn polyphony_mode(&self) -> PolyphonyMode;
    fn set_polyphony_mode(&self, mode: PolyphonyMode);
    fn active_voice_count(&self) -> usize;
    fn max_voices(&self) -> usize;
    
    // ============ STEREO ============
    
    fn stereo_width(&self) -> f32;
    fn set_stereo_width(&self, width: f32);
    
    // ============ OSCILLATOR PAN (not in params) ============
    
    fn osc_pan(&self, osc_num: u8) -> f32;
    fn set_osc_pan(&self, osc_num: u8, pan: f32);
    
    // ============ STATUS ============
    
    fn get_cpu_load(&self) -> f32;
    
    // ============ MIDI (optional - only standalone) ============
    
    /// Get available MIDI ports (empty for plugin)
    fn midi_ports(&self) -> Vec<String> {
        Vec::new()
    }
    
    /// Connect to MIDI port (no-op for plugin)
    fn connect_midi(&mut self, _port_idx: usize) -> Result<(), String> {
        Ok(())
    }
    
    /// Get MIDI connection status
    fn midi_connected(&self) -> bool {
        false
    }
    
    // ============ MIDI LEARN ============
    
    /// Get last received CC values (for display)
    fn get_midi_cc_values(&self) -> Vec<(u8, u8)>;
    
    /// Set MIDI learn target
    fn set_midi_learn_target(&mut self, param: Option<SynthParam>);
    
    /// Get current MIDI learn target
    fn midi_learn_target(&self) -> Option<SynthParam>;
    
    /// Get custom CC mappings
    fn get_cc_mappings(&self) -> Vec<(u8, SynthParam)>;
    
    /// Clear all CC mappings
    fn clear_cc_mappings(&mut self);
}
```

---

## Implementation 1: Standalone Backend

```rust
// src/gui/backend_standalone.rs

use super::backend::SynthBackend;
use crate::audio::AudioEngine;
use crate::core::params::SynthParam;
use crate::core::voice::PolyphonyMode;
use crate::gui::SharedState;
use crate::input::midi::MidiInputHandler;
use std::collections::HashMap;
use std::sync::Arc;

pub struct StandaloneBackend {
    shared: SharedState,
    audio_engine: AudioEngine,
    midi_handler: Option<MidiInputHandler>,
    midi_ports: Vec<String>,
    midi_learn_target: Option<SynthParam>,
    custom_cc_map: HashMap<u8, SynthParam>,
}

impl StandaloneBackend {
    pub fn new(shared: SharedState, audio_engine: AudioEngine) -> Self {
        let midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
        let midi_handler = MidiInputHandler::connect_auto().ok();
        
        Self {
            shared,
            audio_engine,
            midi_handler,
            midi_ports,
            midi_learn_target: None,
            custom_cc_map: HashMap::new(),
        }
    }
    
    /// Poll MIDI events and update state (call from GUI update loop)
    pub fn poll_midi(&mut self) {
        if let Some(ref midi) = self.midi_handler {
            // Process MIDI events, update feedback, handle learn mode
            // (existing logic from SynthApp::update)
        }
    }
    
    /// Sync parameters to audio engine (call from GUI update loop)
    pub fn sync_to_audio(&mut self) {
        self.audio_engine.sync_params(&self.shared.params);
    }
}

impl SynthBackend for StandaloneBackend {
    fn get_param(&self, param: SynthParam) -> f32 {
        self.shared.params.get(param)
    }
    
    fn set_param(&self, param: SynthParam, value: f32) {
        self.shared.params.set(param, value);
    }
    
    fn delay_enabled(&self) -> bool {
        self.audio_engine.delay_enabled()
    }
    
    fn set_delay_enabled(&self, enabled: bool) {
        self.audio_engine.set_delay_enabled(enabled);
    }
    
    fn delay_time(&self) -> f32 {
        self.audio_engine.delay_time()
    }
    
    fn set_delay_time(&self, time: f32) {
        self.audio_engine.set_delay_time(time);
    }
    
    // ... implement all other methods by delegating to audio_engine
    
    fn polyphony_mode(&self) -> PolyphonyMode {
        self.audio_engine.polyphony_mode()
    }
    
    fn set_polyphony_mode(&self, mode: PolyphonyMode) {
        self.audio_engine.set_polyphony_mode(mode);
    }
    
    fn active_voice_count(&self) -> usize {
        self.audio_engine.active_voice_count()
    }
    
    fn max_voices(&self) -> usize {
        self.audio_engine.max_voices()
    }
    
    fn get_cpu_load(&self) -> f32 {
        self.shared.get_cpu_load()
    }
    
    fn midi_ports(&self) -> Vec<String> {
        self.midi_ports.clone()
    }
    
    fn connect_midi(&mut self, port_idx: usize) -> Result<(), String> {
        match MidiInputHandler::connect(port_idx, None) {
            Ok(handler) => {
                self.midi_handler = Some(handler);
                Ok(())
            }
            Err(e) => Err(format!("MIDI connect failed: {}", e))
        }
    }
    
    fn midi_connected(&self) -> bool {
        self.midi_handler.is_some()
    }
    
    fn get_midi_cc_values(&self) -> Vec<(u8, u8)> {
        self.shared.midi_feedback.iter()
            .map(|e| (*e.key(), *e.value()))
            .collect()
    }
    
    fn set_midi_learn_target(&mut self, param: Option<SynthParam>) {
        self.midi_learn_target = param;
    }
    
    fn midi_learn_target(&self) -> Option<SynthParam> {
        self.midi_learn_target
    }
    
    fn get_cc_mappings(&self) -> Vec<(u8, SynthParam)> {
        self.custom_cc_map.iter()
            .map(|(cc, param)| (*cc, *param))
            .collect()
    }
    
    fn clear_cc_mappings(&mut self) {
        self.custom_cc_map.clear();
    }
}
```

---

## Implementation 2: Plugin Backend

```rust
// src/plugin_gui/backend_plugin.rs

use crate::gui::backend::SynthBackend;
use crate::core::params::SynthParam;
use crate::core::voice::PolyphonyMode;
use crate::plugin::RustInSynthParams;
use nih_plug::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use parking_lot::RwLock;

/// Shared state between plugin audio thread and GUI
pub struct PluginSharedState {
    pub cpu_load: Arc<AtomicU32>,
    pub voice_count: Arc<AtomicUsize>,
    pub max_voices: Arc<AtomicUsize>,
    pub midi_cc_values: Arc<RwLock<Vec<(u8, u8)>>>,
    pub midi_learn_target: Arc<RwLock<Option<SynthParam>>>,
    pub cc_mappings: Arc<RwLock<Vec<(u8, SynthParam)>>>,
}

pub struct PluginBackend<'a> {
    params: Arc<RustInSynthParams>,
    setter: &'a ParamSetter<'a>,
    shared: Arc<PluginSharedState>,
}

impl<'a> PluginBackend<'a> {
    pub fn new(
        params: Arc<RustInSynthParams>,
        setter: &'a ParamSetter<'a>,
        shared: Arc<PluginSharedState>,
    ) -> Self {
        Self { params, setter, shared }
    }
}

impl<'a> SynthBackend for PluginBackend<'a> {
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
    
    fn set_param(&self, param: SynthParam, value: f32) {
        match param {
            SynthParam::Attack => self.setter.set_parameter(&self.params.attack, value),
            SynthParam::Decay => self.setter.set_parameter(&self.params.decay, value),
            SynthParam::Sustain => self.setter.set_parameter(&self.params.sustain, value),
            SynthParam::Release => self.setter.set_parameter(&self.params.release, value),
            SynthParam::FilterCutoff => self.setter.set_parameter(&self.params.cutoff, value),
            SynthParam::FilterResonance => self.setter.set_parameter(&self.params.resonance, value),
            // ... all other parameters
            SynthParam::MasterVolume => self.setter.set_parameter(&self.params.master_volume, value),
        }
    }
    
    // Effects - read from parameters added in Phase 1
    fn delay_enabled(&self) -> bool {
        self.params.delay_enabled.value() > 0
    }
    
    fn set_delay_enabled(&self, enabled: bool) {
        self.setter.set_parameter(&self.params.delay_enabled, if enabled { 1 } else { 0 });
    }
    
    fn delay_time(&self) -> f32 {
        self.params.delay_time.value()
    }
    
    fn set_delay_time(&self, time: f32) {
        self.setter.set_parameter(&self.params.delay_time, time);
    }
    
    // ... implement all effect methods similarly
    
    fn polyphony_mode(&self) -> PolyphonyMode {
        if self.params.polyphony_mode.value() == 0 {
            PolyphonyMode::Mono
        } else {
            PolyphonyMode::Poly
        }
    }
    
    fn set_polyphony_mode(&self, mode: PolyphonyMode) {
        let value = match mode {
            PolyphonyMode::Mono => 0,
            PolyphonyMode::Poly => 1,
        };
        self.setter.set_parameter(&self.params.polyphony_mode, value);
    }
    
    fn active_voice_count(&self) -> usize {
        self.shared.voice_count.load(Ordering::Relaxed)
    }
    
    fn max_voices(&self) -> usize {
        self.shared.max_voices.load(Ordering::Relaxed)
    }
    
    fn stereo_width(&self) -> f32 {
        self.params.stereo_width.value()
    }
    
    fn set_stereo_width(&self, width: f32) {
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
    
    fn set_osc_pan(&self, osc_num: u8, pan: f32) {
        match osc_num {
            1 => self.setter.set_parameter(&self.params.osc1_pan, pan),
            2 => self.setter.set_parameter(&self.params.osc2_pan, pan),
            3 => self.setter.set_parameter(&self.params.osc3_pan, pan),
            _ => {}
        }
    }
    
    fn get_cpu_load(&self) -> f32 {
        f32::from_bits(self.shared.cpu_load.load(Ordering::Relaxed)) * 100.0
    }
    
    // MIDI - no port management for plugin
    fn midi_ports(&self) -> Vec<String> {
        Vec::new()
    }
    
    fn midi_connected(&self) -> bool {
        true // Always "connected" via DAW
    }
    
    fn get_midi_cc_values(&self) -> Vec<(u8, u8)> {
        self.shared.midi_cc_values.read().clone()
    }
    
    fn set_midi_learn_target(&mut self, param: Option<SynthParam>) {
        *self.shared.midi_learn_target.write() = param;
    }
    
    fn midi_learn_target(&self) -> Option<SynthParam> {
        *self.shared.midi_learn_target.read()
    }
    
    fn get_cc_mappings(&self) -> Vec<(u8, SynthParam)> {
        self.shared.cc_mappings.read().clone()
    }
    
    fn clear_cc_mappings(&mut self) {
        self.shared.cc_mappings.write().clear();
    }
}
```

---

## Unified GUI Code

```rust
// src/gui/app.rs (REFACTORED)

use super::backend::SynthBackend;
use super::widgets::*;
use super::theme::{THEME, panel_background, section_header};
use crate::core::params::SynthParam;
use crate::core::presets::{list_presets, load_preset, save_preset, Preset};
use egui::*;

/// Universal synth GUI - works with any backend
pub struct SynthApp {
    backend: Box<dyn SynthBackend>,
    
    // UI state (no audio/MIDI dependencies)
    preset_name: String,
    available_presets: Vec<String>,
    selected_preset: Option<usize>,
}

impl SynthApp {
    pub fn new(backend: Box<dyn SynthBackend>) -> Self {
        let available_presets = list_presets().unwrap_or_default();
        
        Self {
            backend,
            preset_name: "New Preset".to_string(),
            available_presets,
            selected_preset: None,
        }
    }
    
    /// Main UI rendering - IDENTICAL for standalone and plugin!
    pub fn ui(&mut self, ctx: &egui::Context) {
        // Set theme
        ctx.set_visuals(egui::Visuals {
            window_fill: THEME.bg_blue,
            panel_fill: THEME.panel_bg,
            ..egui::Visuals::dark()
        });
        
        // Top bar
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🔊 Rust In Synth")
                        .size(18.0)
                        .strong()
                        .color(THEME.gold)
                );
                
                // CPU Load
                let cpu_load = self.backend.get_cpu_load();
                let cpu_color = if cpu_load < 50.0 {
                    Color32::from_rgb(100, 200, 100)
                } else if cpu_load < 80.0 {
                    Color32::from_rgb(200, 200, 100)
                } else {
                    Color32::from_rgb(200, 100, 100)
                };
                ui.label(RichText::new("CPU:").size(10.0));
                ui.add(egui::ProgressBar::new(cpu_load / 100.0)
                    .desired_width(60.0)
                    .fill(cpu_color));
                ui.label(RichText::new(format!("{:.1}%", cpu_load)).size(10.0));
            });
        });
        
        // Main panel
        egui::CentralPanel::default()
            .frame(Frame::none().fill(THEME.bg_blue))
            .show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        // Column 1: Oscillators
                        ui.vertical(|ui| {
                            ui.set_width(185.0);
                            self.ui_oscillators(ui);
                        });
                        
                        ui.separator();
                        
                        // Column 2: Filter
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            self.ui_filter(ui);
                        });
                        
                        ui.separator();
                        
                        // Column 3: Envelopes
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            self.ui_envelopes(ui);
                        });
                        
                        ui.separator();
                        
                        // Column 4: LFO
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            self.ui_lfo(ui);
                        });
                        
                        ui.separator();
                        
                        // Column 5: Effects
                        ui.vertical(|ui| {
                            ui.set_width(155.0);
                            self.ui_effects(ui);
                        });
                        
                        ui.separator();
                        
                        // Column 6: Presets & MIDI
                        ui.vertical(|ui| {
                            ui.set_width(185.0);
                            self.ui_presets(ui);
                            ui.add_space(16.0);
                            self.ui_midi(ui);
                        });
                    });
                });
            });
    }
    
    fn ui_oscillators(&mut self, ui: &mut Ui) {
        section_header(ui, "OSCILLATORS");
        
        let wf_labels = ["Sin", "Tri", "Saw", "Sqr", "Nse"];
        
        // OSC 1
        ui.group(|ui| {
            ui.label(RichText::new("OSC 1").size(10.0).strong());
            
            let mut wf1 = self.backend.get_param(SynthParam::Osc1Waveform) as usize;
            egui::ComboBox::from_id_salt("osc1_wf")
                .selected_text(wf_labels[wf1.min(4)])
                .show_ui(ui, |ui| {
                    for (i, label) in wf_labels.iter().enumerate() {
                        if ui.selectable_label(wf1 == i, *label).clicked() {
                            self.backend.set_param(SynthParam::Osc1Waveform, i as f32);
                        }
                    }
                });
            
            let mut lvl1 = self.backend.get_param(SynthParam::Osc1Level);
            if knob(ui, &mut lvl1, 0.0..=1.0, "Level", "").changed() {
                self.backend.set_param(SynthParam::Osc1Level, lvl1);
            }
            
            // ... more controls
        });
        
        // OSC 2, OSC 3 similar...
    }
    
    fn ui_effects(&mut self, ui: &mut Ui) {
        section_header(ui, "EFFECTS");
        
        // DELAY
        ui.group(|ui| {
            let mut delay_on = self.backend.delay_enabled();
            if ui.checkbox(&mut delay_on, "DELAY").changed() {
                self.backend.set_delay_enabled(delay_on);
            }
            
            if delay_on {
                let mut time = self.backend.delay_time();
                if knob(ui, &mut time, 0.05..=1.0, "Time", "s").changed() {
                    self.backend.set_delay_time(time);
                }
                
                // ... more controls
            }
        });
        
        // REVERB, CHORUS similar...
    }
    
    fn ui_midi(&mut self, ui: &mut Ui) {
        section_header(ui, "MIDI");
        
        // Connection status
        let status = if self.backend.midi_connected() {
            "✅ Connected"
        } else {
            "❌ Disconnected"
        };
        ui.label(status);
        
        // Port selector (only shown if ports available - i.e., standalone)
        let ports = self.backend.midi_ports();
        if !ports.is_empty() {
            // Show port selector UI
        }
        
        // MIDI Learn (works for both!)
        if let Some(target) = self.backend.midi_learn_target() {
            ui.label(format!("Learning: {}", target.name()));
            if ui.button("Cancel").clicked() {
                self.backend.set_midi_learn_target(None);
            }
        } else {
            // Show learn button
        }
        
        // CC Activity (works for both!)
        let cc_values = self.backend.get_midi_cc_values();
        for (cc, val) in cc_values.iter().take(10) {
            ui.horizontal(|ui| {
                ui.label(format!("CC {:>3}:", cc));
                ui.add(egui::ProgressBar::new(*val as f32 / 127.0));
            });
        }
    }
    
    // ... other UI methods
}
```

---

## Integration

### Standalone Main
```rust
// src/main.rs

use rust_in_synth::gui::{SynthApp, SharedState};
use rust_in_synth::gui::backend_standalone::StandaloneBackend;
use rust_in_synth::audio::AudioEngine;

fn main() {
    let shared = SharedState::new();
    let audio_engine = AudioEngine::new(shared.cpu_load.clone()).unwrap();
    
    let backend = Box::new(StandaloneBackend::new(shared, audio_engine));
    let app = SynthApp::new(backend);
    
    // Run eframe
    eframe::run_native(
        "Rust In Synth",
        options,
        Box::new(|_cc| Ok(Box::new(app))),
    ).unwrap();
}
```

### Plugin Editor
```rust
// src/plugin_gui/editor.rs

use crate::gui::SynthApp;
use crate::plugin_gui::backend_plugin::{PluginBackend, PluginSharedState};

pub fn create_editor(
    egui_state: Arc<EguiState>,
    params: Arc<RustInSynthParams>,
    shared: Arc<PluginSharedState>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        egui_state,
        (),
        EguiSettings::default(),
        |_state, _ctx, _queue| {},
        move |ctx, setter, _state, _ui| {
            let backend = Box::new(PluginBackend::new(
                params.clone(),
                setter,
                shared.clone(),
            ));
            let mut app = SynthApp::new(backend);
            app.ui(ctx);
        },
    )
}
```

---

## Benefits

✅ **Zero code duplication** - GUI code written once
✅ **Identical behavior** - Same UI logic for both
✅ **Type-safe** - Trait ensures all methods implemented
✅ **Maintainable** - Changes to UI automatically apply to both
✅ **Testable** - Can mock backend for testing
✅ **Extensible** - Easy to add new backends (e.g., CLAP, LV2)

---

## Implementation Plan

### Phase 1: Create Abstraction Layer
1. Create `src/gui/backend.rs` with `SynthBackend` trait
2. Create `src/gui/backend_standalone.rs` implementing trait
3. Test standalone still works

### Phase 2: Refactor Standalone GUI
1. Modify `SynthApp` to use `Box<dyn SynthBackend>`
2. Replace all direct `audio_engine` calls with `backend` calls
3. Replace all `shared.params` calls with `backend.get/set_param`
4. Test standalone still works

### Phase 3: Add Effects Parameters to Plugin
1. Add ~12 effect parameters to `RustInSynthParams`
2. Sync them in `sync_plugin_params_safe()`
3. Test plugin audio still works

### Phase 4: Create Plugin Backend
1. Create `src/plugin_gui/backend_plugin.rs`
2. Implement `SynthBackend` trait
3. Add `PluginSharedState` for atomic state sharing

### Phase 5: Use Unified GUI in Plugin
1. Modify plugin editor to use `SynthApp`
2. Create `PluginBackend` instance
3. Test in DAW

**Estimated time: 10-14 hours** (slightly more upfront, but perfect result)

---

## Result

**One GUI codebase, two contexts, identical behavior.**

The abstraction is clean, the code is maintainable, and you never have to worry about the standalone and plugin UIs diverging.

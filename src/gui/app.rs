//! Main egui application

use crate::audio::AudioEngine;
use crate::core::event::WaveformType;
use crate::core::filter::{cc_to_cutoff, cc_to_resonance};
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::{
    cc_to_filter_env_amount, cc_to_lfo_depth, cc_to_lfo_rate, cc_to_sustain, cc_to_time,
    SynthParam,
};
use crate::core::voice::{
    MIN_ATTACK_TIME, MAX_ATTACK_TIME, MIN_DECAY_TIME, MAX_DECAY_TIME,
    MIN_RELEASE_TIME, MAX_RELEASE_TIME,
};
use crate::core::presets::{list_presets, load_preset, save_preset, Preset};
use crate::gui::widgets::*;
use crate::gui::SharedState;
use crate::input::midi::MidiInputHandler;
use egui::*;

/// Main synth application
pub struct SynthApp {
    /// Shared state with audio thread
    shared: SharedState,
    
    /// Audio engine
    audio_engine: AudioEngine,
    
    /// MIDI input handler (optional)
    midi_handler: Option<MidiInputHandler>,
    
    /// Available MIDI ports
    midi_ports: Vec<String>,
    
    /// Selected MIDI port index
    selected_midi_port: Option<usize>,
    
    /// Preset name input
    preset_name: String,
    
    /// Available presets list
    available_presets: Vec<String>,
    
    /// Selected preset index
    selected_preset: Option<usize>,
}

impl SynthApp {
    pub fn new(shared: SharedState) -> Self {
        let available_presets = list_presets().unwrap_or_default();

        // Get available MIDI ports
        let midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
        let selected_midi_port = if !midi_ports.is_empty() { Some(0) } else { None };

        // Initialize audio engine
        let mut audio_engine = match AudioEngine::new() {
            Ok(mut engine) => {
                engine.set_master_volume(0.5);
                if let Err(e) = engine.start() {
                    eprintln!("Failed to start audio: {}", e);
                }
                engine
            }
            Err(e) => {
                eprintln!("Failed to create audio engine: {}", e);
                panic!("Audio engine required: {}", e);
            }
        };

        // Try to connect MIDI (auto-connect to first port)
        let midi_handler = MidiInputHandler::connect_auto().ok();
        if midi_handler.is_some() {
            println!("MIDI connected!");
        }

        Self {
            shared,
            audio_engine,
            midi_handler,
            midi_ports,
            selected_midi_port,
            preset_name: "New Preset".to_string(),
            available_presets,
            selected_preset: None,
        }
    }

    /// Update parameter from UI
    fn set_param(&self, param: SynthParam, value: f32) {
        self.shared.params.set(param, value);
        println!("Param {} set to {}", param, value);
    }

    /// Get parameter value
    fn get_param(&self, param: SynthParam) -> f32 {
        self.shared.params.get(param)
    }

    /// Apply a loaded preset to the synth
    fn apply_preset(&mut self, preset: &Preset) {
        // Apply all parameters
        self.set_param(SynthParam::Osc1Waveform, preset.osc1_waveform as u8 as f32);
        self.set_param(SynthParam::Osc1Level, preset.osc1_level);
        self.set_param(SynthParam::Osc1Phase, preset.osc1_phase);

        self.set_param(SynthParam::Osc2Waveform, preset.osc2_waveform as u8 as f32);
        self.set_param(SynthParam::Osc2Level, preset.osc2_level);
        self.set_param(SynthParam::Osc2Semitones, preset.osc2_semitones as f32);
        self.set_param(SynthParam::Osc2Cents, preset.osc2_cents as f32);
        self.set_param(SynthParam::Osc2Phase, preset.osc2_phase);

        self.set_param(SynthParam::Osc3Waveform, preset.osc3_waveform as u8 as f32);
        self.set_param(SynthParam::Osc3Level, preset.osc3_level);
        self.set_param(SynthParam::Osc3Semitones, preset.osc3_semitones as f32);
        self.set_param(SynthParam::Osc3Cents, preset.osc3_cents as f32);
        self.set_param(SynthParam::Osc3Phase, preset.osc3_phase);

        self.set_param(SynthParam::FilterCutoff, preset.filter_cutoff);
        self.set_param(SynthParam::FilterResonance, preset.filter_resonance);

        self.set_param(SynthParam::Attack, preset.amp_attack);
        self.set_param(SynthParam::Decay, preset.amp_decay);
        self.set_param(SynthParam::Sustain, preset.amp_sustain);
        self.set_param(SynthParam::Release, preset.amp_release);

        self.set_param(SynthParam::FilterAttack, preset.filter_attack);
        self.set_param(SynthParam::FilterDecay, preset.filter_decay);
        self.set_param(SynthParam::FilterSustain, preset.filter_sustain);
        self.set_param(SynthParam::FilterRelease, preset.filter_release);
        self.set_param(SynthParam::FilterEnvAmount, preset.filter_env_amount);

        self.set_param(SynthParam::LfoRate, preset.lfo_rate);
        self.set_param(SynthParam::LfoDepth, preset.lfo_depth);
        self.set_param(SynthParam::LfoWaveform, preset.lfo_waveform as u8 as f32);
        self.set_param(SynthParam::LfoDestination, preset.lfo_destination as u8 as f32);

        self.set_param(SynthParam::PitchBendRange, preset.pitch_bend_range as f32);
        self.set_param(SynthParam::MasterVolume, preset.master_volume);
    }

    /// Create a preset from current settings
    fn create_preset(&self) -> Preset {
        Preset {
            name: self.preset_name.clone(),
            version: "1.0".to_string(),

            osc1_waveform: WaveformType::from_index(self.get_param(SynthParam::Osc1Waveform) as u8),
            osc1_level: self.get_param(SynthParam::Osc1Level),
            osc1_phase: self.get_param(SynthParam::Osc1Phase),

            osc2_waveform: WaveformType::from_index(self.get_param(SynthParam::Osc2Waveform) as u8),
            osc2_level: self.get_param(SynthParam::Osc2Level),
            osc2_semitones: self.get_param(SynthParam::Osc2Semitones) as i8,
            osc2_cents: self.get_param(SynthParam::Osc2Cents) as i8,
            osc2_phase: self.get_param(SynthParam::Osc2Phase),

            osc3_waveform: WaveformType::from_index(self.get_param(SynthParam::Osc3Waveform) as u8),
            osc3_level: self.get_param(SynthParam::Osc3Level),
            osc3_semitones: self.get_param(SynthParam::Osc3Semitones) as i8,
            osc3_cents: self.get_param(SynthParam::Osc3Cents) as i8,
            osc3_phase: self.get_param(SynthParam::Osc3Phase),

            filter_cutoff: self.get_param(SynthParam::FilterCutoff),
            filter_resonance: self.get_param(SynthParam::FilterResonance),

            amp_attack: self.get_param(SynthParam::Attack),
            amp_decay: self.get_param(SynthParam::Decay),
            amp_sustain: self.get_param(SynthParam::Sustain),
            amp_release: self.get_param(SynthParam::Release),

            filter_attack: self.get_param(SynthParam::FilterAttack),
            filter_decay: self.get_param(SynthParam::FilterDecay),
            filter_sustain: self.get_param(SynthParam::FilterSustain),
            filter_release: self.get_param(SynthParam::FilterRelease),
            filter_env_amount: self.get_param(SynthParam::FilterEnvAmount),

            lfo_rate: self.get_param(SynthParam::LfoRate),
            lfo_depth: self.get_param(SynthParam::LfoDepth),
            lfo_waveform: match self.get_param(SynthParam::LfoWaveform) as u8 {
                0 => LfoWaveform::Sine,
                1 => LfoWaveform::Triangle,
                2 => LfoWaveform::Square,
                3 => LfoWaveform::Saw,
                _ => LfoWaveform::Random,
            },
            lfo_destination: match self.get_param(SynthParam::LfoDestination) as u8 {
                0 => LfoDestination::Off,
                1 => LfoDestination::Pitch,
                2 => LfoDestination::FilterCutoff,
                _ => LfoDestination::Amplitude,
            },

            pitch_bend_range: self.get_param(SynthParam::PitchBendRange) as u8,

            master_volume: self.get_param(SynthParam::MasterVolume),
        }
    }

    /// Update ParamBank from incoming MIDI CC to keep GUI in sync
    fn update_param_from_cc(&mut self, cc: u8, value: u8) {
        match cc {
            // ADSR Envelope
            73 => self.set_param(SynthParam::Attack, cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME)),
            83 => self.set_param(SynthParam::Decay, cc_to_time(value, MIN_DECAY_TIME, MAX_DECAY_TIME)),
            84 => self.set_param(SynthParam::Sustain, cc_to_sustain(value)),
            72 => self.set_param(SynthParam::Release, cc_to_time(value, MIN_RELEASE_TIME, MAX_RELEASE_TIME)),
            
            // Filter
            74 => self.set_param(SynthParam::FilterCutoff, cc_to_cutoff(value)),
            71 => self.set_param(SynthParam::FilterResonance, cc_to_resonance(value)),
            
            // Filter Envelope
            103 => self.set_param(SynthParam::FilterAttack, cc_to_time(value, MIN_ATTACK_TIME, MAX_ATTACK_TIME)),
            104 => self.set_param(SynthParam::FilterDecay, cc_to_time(value, MIN_DECAY_TIME, MAX_DECAY_TIME)),
            105 => self.set_param(SynthParam::FilterSustain, cc_to_sustain(value)),
            106 => self.set_param(SynthParam::FilterRelease, cc_to_time(value, MIN_RELEASE_TIME, MAX_RELEASE_TIME)),
            107 => self.set_param(SynthParam::FilterEnvAmount, cc_to_filter_env_amount(value)),
            
            // LFO
            108 => self.set_param(SynthParam::LfoRate, cc_to_lfo_rate(value)),
            109 => self.set_param(SynthParam::LfoDepth, cc_to_lfo_depth(value)),
            110 => self.set_param(SynthParam::LfoWaveform, (value / 26).min(4) as f32), // 0-4
            111 => self.set_param(SynthParam::LfoDestination, (value / 32).min(3) as f32), // 0-3
            
            // Oscillator levels
            80 => self.set_param(SynthParam::Osc1Level, value as f32 / 127.0),
            81 => self.set_param(SynthParam::Osc2Level, value as f32 / 127.0),
            82 => self.set_param(SynthParam::Osc3Level, value as f32 / 127.0),
            
            // Oscillator waveforms
            75 => self.set_param(SynthParam::Osc1Waveform, (value / 26).min(4) as f32),
            76 => self.set_param(SynthParam::Osc2Waveform, (value / 26).min(4) as f32),
            77 => self.set_param(SynthParam::Osc3Waveform, (value / 26).min(4) as f32),
            
            _ => {} // Ignore unmapped CCs
        }
    }

    fn ui_oscillators(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "OSCILLATORS");

            // OSC 1
            ui.horizontal(|ui| {
                ui.label(RichText::new("OSC 1").size(12.0).strong());
            });
            ui.add_space(8.0);

            let mut osc1_wave = self.get_param(SynthParam::Osc1Waveform) as u8;
            if selector_switch(
                ui,
                &mut (osc1_wave as usize),
                &["Sine", "Square", "Saw", "Tri", "Noise"],
                "Waveform",
            ).changed() {
                self.set_param(SynthParam::Osc1Waveform, osc1_wave as f32);
            }

            ui.horizontal(|ui| {
                let mut level = self.get_param(SynthParam::Osc1Level);
                if knob(ui, &mut level, 0.0..=1.0, "Level", "").changed() {
                    self.set_param(SynthParam::Osc1Level, level);
                }

                let mut phase = self.get_param(SynthParam::Osc1Phase);
                if knob(ui, &mut phase, 0.0..=1.0, "Phase", "°").changed() {
                    self.set_param(SynthParam::Osc1Phase, phase);
                }
            });

            ui.add_space(16.0);

            // OSC 2
            ui.horizontal(|ui| {
                ui.label(RichText::new("OSC 2").size(12.0).strong());
            });
            ui.add_space(8.0);

            let mut osc2_wave = self.get_param(SynthParam::Osc2Waveform) as u8;
            if selector_switch(
                ui,
                &mut (osc2_wave as usize),
                &["Sine", "Square", "Saw", "Tri", "Noise"],
                "Waveform",
            ).changed() {
                self.set_param(SynthParam::Osc2Waveform, osc2_wave as f32);
            }

            ui.horizontal(|ui| {
                let mut level = self.get_param(SynthParam::Osc2Level);
                if knob(ui, &mut level, 0.0..=1.0, "Level", "").changed() {
                    self.set_param(SynthParam::Osc2Level, level);
                }

                let mut semi = self.get_param(SynthParam::Osc2Semitones);
                if knob(ui, &mut semi, -24.0..=24.0, "Semitones", "st").changed() {
                    self.set_param(SynthParam::Osc2Semitones, semi);
                }

                let mut cents = self.get_param(SynthParam::Osc2Cents);
                if knob(ui, &mut cents, -100.0..=100.0, "Detune", "¢").changed() {
                    self.set_param(SynthParam::Osc2Cents, cents);
                }

                let mut phase = self.get_param(SynthParam::Osc2Phase);
                if knob(ui, &mut phase, 0.0..=1.0, "Phase", "°").changed() {
                    self.set_param(SynthParam::Osc2Phase, phase);
                }
            });

            ui.add_space(16.0);

            // OSC 3
            ui.horizontal(|ui| {
                ui.label(RichText::new("OSC 3").size(12.0).strong());
            });
            ui.add_space(8.0);

            let mut osc3_wave = self.get_param(SynthParam::Osc3Waveform) as u8;
            if selector_switch(
                ui,
                &mut (osc3_wave as usize),
                &["Sine", "Square", "Saw", "Tri", "Noise"],
                "Waveform",
            ).changed() {
                self.set_param(SynthParam::Osc3Waveform, osc3_wave as f32);
            }

            ui.horizontal(|ui| {
                let mut level = self.get_param(SynthParam::Osc3Level);
                if knob(ui, &mut level, 0.0..=1.0, "Level", "").changed() {
                    self.set_param(SynthParam::Osc3Level, level);
                }

                let mut semi = self.get_param(SynthParam::Osc3Semitones);
                if knob(ui, &mut semi, -24.0..=24.0, "Semitones", "st").changed() {
                    self.set_param(SynthParam::Osc3Semitones, semi);
                }

                let mut cents = self.get_param(SynthParam::Osc3Cents);
                if knob(ui, &mut cents, -100.0..=100.0, "Detune", "¢").changed() {
                    self.set_param(SynthParam::Osc3Cents, cents);
                }

                let mut phase = self.get_param(SynthParam::Osc3Phase);
                if knob(ui, &mut phase, 0.0..=1.0, "Phase", "°").changed() {
                    self.set_param(SynthParam::Osc3Phase, phase);
                }
            });
        });
    }

    fn ui_filter(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "FILTER");

            ui.horizontal(|ui| {
                let mut cutoff = self.get_param(SynthParam::FilterCutoff);
                if knob(ui, &mut cutoff, 20.0..=20000.0, "Cutoff", "Hz").changed() {
                    self.set_param(SynthParam::FilterCutoff, cutoff);
                }

                let mut res = self.get_param(SynthParam::FilterResonance);
                if knob(ui, &mut res, 0.0..=1.0, "Resonance", "").changed() {
                    self.set_param(SynthParam::FilterResonance, res);
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(RichText::new("FILTER ENVELOPE").size(11.0).strong());

            ui.horizontal(|ui| {
                let mut attack = self.get_param(SynthParam::FilterAttack);
                if knob(ui, &mut attack, 0.001..=2.0, "Attack", "s").changed() {
                    self.set_param(SynthParam::FilterAttack, attack);
                }

                let mut decay = self.get_param(SynthParam::FilterDecay);
                if knob(ui, &mut decay, 0.001..=5.0, "Decay", "s").changed() {
                    self.set_param(SynthParam::FilterDecay, decay);
                }

                let mut sustain = self.get_param(SynthParam::FilterSustain);
                if knob(ui, &mut sustain, 0.0..=1.0, "Sustain", "").changed() {
                    self.set_param(SynthParam::FilterSustain, sustain);
                }

                let mut release = self.get_param(SynthParam::FilterRelease);
                if knob(ui, &mut release, 0.001..=5.0, "Release", "s").changed() {
                    self.set_param(SynthParam::FilterRelease, release);
                }

                let mut amount = self.get_param(SynthParam::FilterEnvAmount);
                if knob(ui, &mut amount, 0.0..=1.0, "Amount", "").changed() {
                    self.set_param(SynthParam::FilterEnvAmount, amount);
                }
            });
        });
    }

    fn ui_envelopes(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "AMPLITUDE ENVELOPE");

            ui.horizontal(|ui| {
                let mut attack = self.get_param(SynthParam::Attack);
                if knob(ui, &mut attack, 0.001..=2.0, "Attack", "s").changed() {
                    self.set_param(SynthParam::Attack, attack);
                }

                let mut decay = self.get_param(SynthParam::Decay);
                if knob(ui, &mut decay, 0.001..=5.0, "Decay", "s").changed() {
                    self.set_param(SynthParam::Decay, decay);
                }

                let mut sustain = self.get_param(SynthParam::Sustain);
                if knob(ui, &mut sustain, 0.0..=1.0, "Sustain", "").changed() {
                    self.set_param(SynthParam::Sustain, sustain);
                }

                let mut release = self.get_param(SynthParam::Release);
                if knob(ui, &mut release, 0.001..=5.0, "Release", "s").changed() {
                    self.set_param(SynthParam::Release, release);
                }
            });
        });
    }

    fn ui_lfo(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "LFO");

            let mut wave = self.get_param(SynthParam::LfoWaveform) as u8;
            if selector_switch(
                ui,
                &mut (wave as usize),
                &["Sine", "Triangle", "Square", "Saw", "Random"],
                "Waveform",
            ).changed() {
                self.set_param(SynthParam::LfoWaveform, wave as f32);
            }

            let mut dest = self.get_param(SynthParam::LfoDestination) as u8;
            if selector_switch(
                ui,
                &mut (dest as usize),
                &["Off", "Pitch", "Filter", "Amp"],
                "Destination",
            ).changed() {
                self.set_param(SynthParam::LfoDestination, dest as f32);
            }

            ui.horizontal(|ui| {
                let mut rate = self.get_param(SynthParam::LfoRate);
                if knob(ui, &mut rate, 0.1..=20.0, "Rate", "Hz").changed() {
                    self.set_param(SynthParam::LfoRate, rate);
                }

                let mut depth = self.get_param(SynthParam::LfoDepth);
                if knob(ui, &mut depth, 0.0..=1.0, "Depth", "").changed() {
                    self.set_param(SynthParam::LfoDepth, depth);
                }
            });
        });
    }

    fn ui_midi(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "MIDI INPUT");

            // MIDI Port Selection
            ui.label(RichText::new("MIDI Port:").size(11.0).strong());
            ui.add_space(4.0);

            egui::ComboBox::from_label("Select Port")
                .selected_text(
                    self.selected_midi_port
                        .and_then(|i| self.midi_ports.get(i))
                        .map(|s| s.as_str())
                        .unwrap_or("No MIDI ports available")
                )
                .show_ui(ui, |ui| {
                    for (i, port_name) in self.midi_ports.iter().enumerate() {
                        let is_selected = self.selected_midi_port == Some(i);
                        if ui.selectable_label(is_selected, port_name).clicked() {
                            self.selected_midi_port = Some(i);
                        }
                    }
                });

            ui.add_space(8.0);

            // Connect/Refresh buttons
            ui.horizontal(|ui| {
                if ui.button("🔄 Refresh Ports").clicked() {
                    self.midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
                    if self.midi_ports.is_empty() {
                        self.selected_midi_port = None;
                    }
                }

                if ui.button("🔌 Connect").clicked() {
                    if let Some(port_idx) = self.selected_midi_port {
                        match MidiInputHandler::connect(port_idx, None) {
                            Ok(handler) => {
                                self.midi_handler = Some(handler);
                                println!("Connected to MIDI port {}", port_idx);
                            }
                            Err(e) => {
                                eprintln!("Failed to connect MIDI: {}", e);
                            }
                        }
                    }
                }
            });

            // Show connection status
            ui.add_space(8.0);
            let status_text = if self.midi_handler.is_some() {
                "✅ Connected"
            } else {
                "❌ Not connected"
            };
            ui.label(RichText::new(status_text).size(12.0));

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            ui.label(RichText::new("Recent CC Messages:").size(11.0));
            ui.add_space(4.0);

            // Show recent MIDI CC feedback
            let cc_mappings = [
                (73, "Attack"),
                (83, "Decay"),
                (84, "Sustain"),
                (72, "Release"),
                (74, "Cutoff"),
                (71, "Resonance"),
            ];

            for (cc, name) in &cc_mappings {
                let value = self.shared.midi_feedback.get(cc).map(|e| *e.value());
                midi_indicator(ui, *cc, value, name);
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            let mut bend_range = self.get_param(SynthParam::PitchBendRange);
            if knob(ui, &mut bend_range, 1.0..=24.0, "Pitch Bend Range", "st").changed() {
                self.set_param(SynthParam::PitchBendRange, bend_range);
            }
        });
    }

    fn ui_presets(&mut self, ui: &mut Ui) {
        panel_background(ui, |ui| {
            section_header(ui, "PRESETS");

            // Preset name input
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.preset_name);
            });

            ui.add_space(8.0);

            // Save button
            if ui.button(RichText::new("💾 Save").size(12.0)).clicked() {
                let preset = self.create_preset();
                if let Err(e) = save_preset(&preset) {
                    eprintln!("Failed to save preset: {}", e);
                } else {
                    // Refresh list
                    self.available_presets = list_presets().unwrap_or_default();
                }
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Preset list
            ui.label(RichText::new("Load Preset:").size(11.0));

            // Collect clicked preset info first to avoid borrow issues
            let mut clicked_preset: Option<(usize, String)> = None;

            egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                for (i, name) in self.available_presets.iter().enumerate() {
                    let is_selected = self.selected_preset == Some(i);
                    let response = ui.selectable_label(
                        is_selected,
                        RichText::new(name).size(11.0),
                    );

                    if response.clicked() {
                        clicked_preset = Some((i, name.clone()));
                    }
                }
            });

            // Apply preset outside of the closure
            if let Some((i, name)) = clicked_preset {
                self.selected_preset = Some(i);
                if let Ok(preset) = load_preset(&name) {
                    self.apply_preset(&preset);
                }
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Master volume
            ui.label(RichText::new("MASTER").size(12.0).strong());
            let mut vol = self.get_param(SynthParam::MasterVolume);
            if knob(ui, &mut vol, 0.0..=1.0, "Volume", "").changed() {
                self.set_param(SynthParam::MasterVolume, vol);
            }
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Request continuous repaints for responsive MIDI handling
        ctx.request_repaint();
        
        // Poll MIDI input and process ALL pending events
        // Collect events first to avoid borrow issues
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
        
        // Process collected events
        for event in events {
            // Send note events to audio engine immediately
            self.audio_engine.send_event(event.clone());
            
            // Update MIDI feedback map and ParamBank for CC messages
            if let crate::core::event::NoteEventKind::ControlChange { cc, value } = event.kind {
                // Always update feedback map for ALL CCs
                self.shared.midi_feedback.insert(cc, value);
                // Update ParamBank for mapped CCs so sync_params doesn't overwrite
                self.update_param_from_cc(cc, value);
            }
        }

        // Sync GUI parameters to audio engine (every frame for responsiveness)
        self.audio_engine.sync_params(&self.shared.params);

        // Set vintage dark theme
        let mut visuals = ctx.style().visuals.clone();
        visuals.dark_mode = true;
        visuals.panel_fill = Color32::from_rgb(25, 20, 15);
        ctx.set_visuals(visuals);

        // Top bar with title
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🔊 Rust In Synth")
                        .size(18.0)
                        .strong()
                        .color(Color32::from_rgb(255, 180, 60))
                );
                ui.label(
                    RichText::new("v0.4.0")
                        .size(10.0)
                        .color(Color32::from_gray(120))
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Monophonic Synthesizer")
                            .size(11.0)
                            .color(Color32::from_gray(180))
                    );
                });
            });
        });

        // Main panel - Single window layout with all sections
        egui::CentralPanel::default()
            .frame(Frame::none().fill(Color32::from_rgb(25, 20, 15)))
            .show(ctx, |ui| {
                // Fill available space with scroll area
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_height(ui.available_height());
                        
                        ui.columns(5, |columns| {
                            // Column 1: Oscillators
                            columns[0].vertical(|ui| {
                                ui.set_min_width(180.0);
                                self.ui_oscillators_compact(ui);
                            });
                            
                            // Column 2: Filter
                            columns[1].vertical(|ui| {
                                ui.set_min_width(140.0);
                                self.ui_filter_compact(ui);
                            });
                            
                            // Column 3: Envelopes
                            columns[2].vertical(|ui| {
                                ui.set_min_width(140.0);
                                self.ui_envelopes_compact(ui);
                            });
                            
                            // Column 4: LFO
                            columns[3].vertical(|ui| {
                                ui.set_min_width(140.0);
                                self.ui_lfo_compact(ui);
                            });
                            
                            // Column 5: Presets & MIDI
                            columns[4].vertical(|ui| {
                                ui.set_min_width(180.0);
                                self.ui_presets_compact(ui);
                                ui.add_space(16.0);
                                ui.separator();
                                ui.add_space(8.0);
                                self.ui_midi_compact(ui);
                            });
                        });
                    });
            });
    }

    /// Compact oscillators panel
    fn ui_oscillators_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "OSCILLATORS");
        
        let wf_labels = ["Sin", "Tri", "Saw", "Sqr", "Nse"];
        
        // OSC 1
        ui.group(|ui| {
            ui.label(RichText::new("OSC 1").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
            
            let mut wf1 = self.get_param(SynthParam::Osc1Waveform) as usize;
            egui::ComboBox::from_id_source("osc1_wf")
                .width(60.0)
                .selected_text(wf_labels[wf1.min(4)])
                .show_ui(ui, |ui| {
                    for (i, label) in wf_labels.iter().enumerate() {
                        if ui.selectable_label(wf1 == i, *label).clicked() {
                            self.set_param(SynthParam::Osc1Waveform, i as f32);
                        }
                    }
                });
            
            let mut lvl1 = self.get_param(SynthParam::Osc1Level);
            if ui.add(egui::Slider::new(&mut lvl1, 0.0..=1.0).text("Level")).changed() {
                self.set_param(SynthParam::Osc1Level, lvl1);
            }
            
            let mut phase1 = self.get_param(SynthParam::Osc1Phase);
            if ui.add(egui::Slider::new(&mut phase1, 0.0..=1.0).text("Phase")).changed() {
                self.set_param(SynthParam::Osc1Phase, phase1);
            }
        });
        
        ui.add_space(4.0);
        
        // OSC 2
        ui.group(|ui| {
            ui.label(RichText::new("OSC 2").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
            
            let mut wf2 = self.get_param(SynthParam::Osc2Waveform) as usize;
            egui::ComboBox::from_id_source("osc2_wf")
                .width(60.0)
                .selected_text(wf_labels[wf2.min(4)])
                .show_ui(ui, |ui| {
                    for (i, label) in wf_labels.iter().enumerate() {
                        if ui.selectable_label(wf2 == i, *label).clicked() {
                            self.set_param(SynthParam::Osc2Waveform, i as f32);
                        }
                    }
                });
            
            let mut lvl2 = self.get_param(SynthParam::Osc2Level);
            if ui.add(egui::Slider::new(&mut lvl2, 0.0..=1.0).text("Level")).changed() {
                self.set_param(SynthParam::Osc2Level, lvl2);
            }
            
            let mut semi2 = self.get_param(SynthParam::Osc2Semitones);
            if ui.add(egui::Slider::new(&mut semi2, -24.0..=24.0).text("Semi")).changed() {
                self.set_param(SynthParam::Osc2Semitones, semi2);
            }
            
            let mut cents2 = self.get_param(SynthParam::Osc2Cents);
            if ui.add(egui::Slider::new(&mut cents2, -100.0..=100.0).text("Cents")).changed() {
                self.set_param(SynthParam::Osc2Cents, cents2);
            }
            
            let mut phase2 = self.get_param(SynthParam::Osc2Phase);
            if ui.add(egui::Slider::new(&mut phase2, 0.0..=1.0).text("Phase")).changed() {
                self.set_param(SynthParam::Osc2Phase, phase2);
            }
        });
        
        ui.add_space(4.0);
        
        // OSC 3
        ui.group(|ui| {
            ui.label(RichText::new("OSC 3").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
            
            let mut wf3 = self.get_param(SynthParam::Osc3Waveform) as usize;
            egui::ComboBox::from_id_source("osc3_wf")
                .width(60.0)
                .selected_text(wf_labels[wf3.min(4)])
                .show_ui(ui, |ui| {
                    for (i, label) in wf_labels.iter().enumerate() {
                        if ui.selectable_label(wf3 == i, *label).clicked() {
                            self.set_param(SynthParam::Osc3Waveform, i as f32);
                        }
                    }
                });
            
            let mut lvl3 = self.get_param(SynthParam::Osc3Level);
            if ui.add(egui::Slider::new(&mut lvl3, 0.0..=1.0).text("Level")).changed() {
                self.set_param(SynthParam::Osc3Level, lvl3);
            }
            
            let mut semi3 = self.get_param(SynthParam::Osc3Semitones);
            if ui.add(egui::Slider::new(&mut semi3, -24.0..=24.0).text("Semi")).changed() {
                self.set_param(SynthParam::Osc3Semitones, semi3);
            }
            
            let mut cents3 = self.get_param(SynthParam::Osc3Cents);
            if ui.add(egui::Slider::new(&mut cents3, -100.0..=100.0).text("Cents")).changed() {
                self.set_param(SynthParam::Osc3Cents, cents3);
            }
            
            let mut phase3 = self.get_param(SynthParam::Osc3Phase);
            if ui.add(egui::Slider::new(&mut phase3, 0.0..=1.0).text("Phase")).changed() {
                self.set_param(SynthParam::Osc3Phase, phase3);
            }
        });
    }
    
    /// Compact filter panel
    fn ui_filter_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "FILTER");
        
        let mut cutoff = self.get_param(SynthParam::FilterCutoff);
        if ui.add(egui::Slider::new(&mut cutoff, 20.0..=20000.0).text("Cutoff").logarithmic(true)).changed() {
            self.set_param(SynthParam::FilterCutoff, cutoff);
        }
        
        let mut res = self.get_param(SynthParam::FilterResonance);
        if ui.add(egui::Slider::new(&mut res, 0.0..=1.0).text("Res")).changed() {
            self.set_param(SynthParam::FilterResonance, res);
        }
        
        ui.add_space(8.0);
        ui.label(RichText::new("Env Amount").size(10.0));
        let mut env_amt = self.get_param(SynthParam::FilterEnvAmount);
        if ui.add(egui::Slider::new(&mut env_amt, -1.0..=1.0).text("Amt")).changed() {
            self.set_param(SynthParam::FilterEnvAmount, env_amt);
        }
    }
    
    /// Compact envelopes panel
    fn ui_envelopes_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "AMP ENV");
        
        let mut atk = self.get_param(SynthParam::Attack);
        if ui.add(egui::Slider::new(&mut atk, 0.001..=5.0).text("Attack").logarithmic(true)).changed() {
            self.set_param(SynthParam::Attack, atk);
        }
        
        let mut dec = self.get_param(SynthParam::Decay);
        if ui.add(egui::Slider::new(&mut dec, 0.001..=5.0).text("Decay").logarithmic(true)).changed() {
            self.set_param(SynthParam::Decay, dec);
        }
        
        let mut sus = self.get_param(SynthParam::Sustain);
        if ui.add(egui::Slider::new(&mut sus, 0.0..=1.0).text("Sustain")).changed() {
            self.set_param(SynthParam::Sustain, sus);
        }
        
        let mut rel = self.get_param(SynthParam::Release);
        if ui.add(egui::Slider::new(&mut rel, 0.001..=5.0).text("Release").logarithmic(true)).changed() {
            self.set_param(SynthParam::Release, rel);
        }
        
        ui.add_space(12.0);
        section_header(ui, "FILTER ENV");
        
        let mut fatk = self.get_param(SynthParam::FilterAttack);
        if ui.add(egui::Slider::new(&mut fatk, 0.001..=5.0).text("Attack").logarithmic(true)).changed() {
            self.set_param(SynthParam::FilterAttack, fatk);
        }
        
        let mut fdec = self.get_param(SynthParam::FilterDecay);
        if ui.add(egui::Slider::new(&mut fdec, 0.001..=5.0).text("Decay").logarithmic(true)).changed() {
            self.set_param(SynthParam::FilterDecay, fdec);
        }
        
        let mut fsus = self.get_param(SynthParam::FilterSustain);
        if ui.add(egui::Slider::new(&mut fsus, 0.0..=1.0).text("Sustain")).changed() {
            self.set_param(SynthParam::FilterSustain, fsus);
        }
        
        let mut frel = self.get_param(SynthParam::FilterRelease);
        if ui.add(egui::Slider::new(&mut frel, 0.001..=5.0).text("Release").logarithmic(true)).changed() {
            self.set_param(SynthParam::FilterRelease, frel);
        }
        
        ui.add_space(12.0);
        section_header(ui, "MASTER");
        
        let mut vol = self.get_param(SynthParam::MasterVolume);
        if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Volume")).changed() {
            self.set_param(SynthParam::MasterVolume, vol);
        }
    }
    
    /// Compact LFO panel
    fn ui_lfo_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "LFO");
        
        let mut rate = self.get_param(SynthParam::LfoRate);
        if ui.add(egui::Slider::new(&mut rate, 0.1..=20.0).text("Rate")).changed() {
            self.set_param(SynthParam::LfoRate, rate);
        }
        
        let mut depth = self.get_param(SynthParam::LfoDepth);
        if ui.add(egui::Slider::new(&mut depth, 0.0..=1.0).text("Depth")).changed() {
            self.set_param(SynthParam::LfoDepth, depth);
        }
        
        ui.label(RichText::new("Waveform").size(10.0));
        let lfo_wf_labels = ["Sin", "Tri", "Sqr", "Saw", "Rnd"];
        let mut lfo_wf = self.get_param(SynthParam::LfoWaveform) as usize;
        egui::ComboBox::from_id_source("lfo_wf")
            .width(70.0)
            .selected_text(lfo_wf_labels[lfo_wf.min(4)])
            .show_ui(ui, |ui| {
                for (i, label) in lfo_wf_labels.iter().enumerate() {
                    if ui.selectable_label(lfo_wf == i, *label).clicked() {
                        self.set_param(SynthParam::LfoWaveform, i as f32);
                    }
                }
            });
        
        ui.label(RichText::new("Destination").size(10.0));
        let lfo_dest_labels = ["Off", "Pitch", "Filter", "Amp"];
        let mut lfo_dest = self.get_param(SynthParam::LfoDestination) as usize;
        egui::ComboBox::from_id_source("lfo_dest")
            .width(70.0)
            .selected_text(lfo_dest_labels[lfo_dest.min(3)])
            .show_ui(ui, |ui| {
                for (i, label) in lfo_dest_labels.iter().enumerate() {
                    if ui.selectable_label(lfo_dest == i, *label).clicked() {
                        self.set_param(SynthParam::LfoDestination, i as f32);
                    }
                }
            });
        
        ui.add_space(12.0);
        section_header(ui, "PITCH");
        
        let mut bend = self.get_param(SynthParam::PitchBendRange);
        if ui.add(egui::Slider::new(&mut bend, 1.0..=24.0).text("Bend Range")).changed() {
            self.set_param(SynthParam::PitchBendRange, bend);
        }
    }
    
    /// Compact presets panel
    fn ui_presets_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "PRESETS");
        
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.preset_name);
        });
        
        if ui.button("💾 Save").clicked() {
            let preset = self.create_preset();
            if let Err(e) = save_preset(&preset) {
                eprintln!("Failed to save preset: {}", e);
            } else {
                self.available_presets = list_presets().unwrap_or_default();
            }
        }
        
        ui.add_space(8.0);
        ui.label("Load:");
        
        let mut clicked_name: Option<String> = None;
        egui::ScrollArea::vertical()
            .id_source("preset_list")
            .max_height(80.0)
            .show(ui, |ui| {
                for (i, name) in self.available_presets.iter().enumerate() {
                    let is_selected = self.selected_preset == Some(i);
                    if ui.selectable_label(is_selected, name).clicked() {
                        self.selected_preset = Some(i);
                        clicked_name = Some(name.clone());
                    }
                }
            });
        if let Some(ref name) = clicked_name {
            if let Ok(preset) = load_preset(name) {
                self.preset_name = preset.name.clone();
                self.apply_preset(&preset);
            }
        }
    }
    
    /// Compact MIDI panel
    fn ui_midi_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "MIDI");
        
        // Connection status
        let status = if self.midi_handler.is_some() { "✅ Connected" } else { "❌ Disconnected" };
        ui.label(RichText::new(status).size(11.0));
        
        // Port selector
        egui::ComboBox::from_id_source("midi_port")
            .selected_text(
                self.selected_midi_port
                    .and_then(|i| self.midi_ports.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("No ports")
            )
            .show_ui(ui, |ui| {
                for (i, port_name) in self.midi_ports.iter().enumerate() {
                    let is_selected = self.selected_midi_port == Some(i);
                    if ui.selectable_label(is_selected, port_name).clicked() {
                        self.selected_midi_port = Some(i);
                    }
                }
            });
        
        ui.horizontal(|ui| {
            if ui.button("🔄").on_hover_text("Refresh ports").clicked() {
                self.midi_ports = MidiInputHandler::list_ports().unwrap_or_default();
            }
            if ui.button("🔌 Connect").clicked() {
                if let Some(port_idx) = self.selected_midi_port {
                    match MidiInputHandler::connect(port_idx, None) {
                        Ok(handler) => {
                            self.midi_handler = Some(handler);
                        }
                        Err(e) => {
                            eprintln!("MIDI connect failed: {}", e);
                        }
                    }
                }
            }
        });
        
        // Recent CCs - show all received CCs
        ui.add_space(8.0);
        ui.label(RichText::new("CC Activity:").size(10.0));
        
        // Collect and sort CCs for consistent display
        let mut cc_list: Vec<_> = self.shared.midi_feedback.iter().map(|e| (*e.key(), *e.value())).collect();
        cc_list.sort_by_key(|(cc, _)| *cc);
        
        egui::ScrollArea::vertical()
            .id_source("midi_cc_activity")
            .max_height(100.0)
            .show(ui, |ui| {
                for (cc, val) in cc_list.iter().take(12) {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("CC {:>3}:", cc)).size(10.0).monospace());
                        ui.add(egui::ProgressBar::new(*val as f32 / 127.0).desired_width(60.0));
                        ui.label(RichText::new(format!("{:>3}", val)).size(10.0).monospace());
                    });
                }
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop audio engine
        self.audio_engine.stop();
        println!("Audio engine stopped.");
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Delegate to the SynthApp implementation
        SynthApp::update(self, ctx, frame);
    }

    fn on_exit(&mut self, gl: Option<&eframe::glow::Context>) {
        // Delegate to the SynthApp implementation
        SynthApp::on_exit(self, gl);
    }
}

/// Run the GUI application
pub fn run_gui(shared: SharedState) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1050.0, 480.0])
            .with_min_inner_size([1000.0, 450.0])
            .with_resizable(true),
        ..Default::default()
    };

    eframe::run_native(
        "Rust In Synth",
        options,
        Box::new(|_cc| Ok(Box::new(SynthApp::new(shared)))),
    )
}

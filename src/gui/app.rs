//! Main egui application

use crate::audio::AudioEngine;
use crate::core::event::WaveformType;
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::SynthParam;
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
    
    /// Currently active tab
    active_tab: Tab,
    
    /// Master volume (local copy for UI)
    master_volume: f32,
}

/// Docking tabs
#[derive(Debug, Clone)]
enum Tab {
    Oscillators,
    Filter,
    Envelopes,
    Lfo,
    Midi,
    Presets,
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
            active_tab: Tab::Oscillators,
            master_volume: 0.5,
        }
    }

    /// Update parameter from UI
    fn set_param(&self, param: SynthParam, value: f32) {
        self.shared.params.set(param, value);
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

        self.master_volume = preset.master_volume;
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

            master_volume: self.master_volume,
        }
    }

    fn ui_tab(&mut self, ui: &mut Ui, tab: &Tab) {
        match tab {
            Tab::Oscillators => self.ui_oscillators(ui),
            Tab::Filter => self.ui_filter(ui),
            Tab::Envelopes => self.ui_envelopes(ui),
            Tab::Lfo => self.ui_lfo(ui),
            Tab::Midi => self.ui_midi(ui),
            Tab::Presets => self.ui_presets(ui),
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
            if knob(ui, &mut self.master_volume, 0.0..=1.0, "Volume", "").changed() {
                // Master volume is handled separately
            }
        });
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll MIDI input and process events
        if let Some(ref midi) = self.midi_handler {
            while let Some(event) = midi.poll() {
                // Send note events to audio engine
                self.audio_engine.send_event(event.clone());
                
                // Update MIDI feedback map for CC messages
                match event.kind {
                    crate::core::event::NoteEventKind::ControlChange { cc, value } => {
                        self.shared.midi_feedback.insert(cc, value);
                    }
                    _ => {}
                }
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
                    RichText::new("🔊 RustSynth")
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

        // Main panel area
        egui::CentralPanel::default()
            .frame(Frame::none().fill(Color32::from_rgb(25, 20, 15)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Tab buttons
                    for tab in [Tab::Oscillators, Tab::Filter, Tab::Envelopes, Tab::Lfo, Tab::Presets, Tab::Midi] {
                        let is_selected = std::mem::discriminant(&self.active_tab) == std::mem::discriminant(&tab);
                        
                        let tab_name = format!("{:?}", tab);
                        let response = ui.selectable_label(
                            is_selected,
                            RichText::new(&tab_name).size(12.0).color(Color32::from_rgb(255, 180, 60))
                        );
                        
                        if response.clicked() {
                            self.active_tab = tab.clone();
                        }
                    }
                });
                
                ui.separator();
                
                // Show current tab content
                let current_tab = self.active_tab.clone();
                self.ui_tab(ui, &current_tab);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Stop audio engine
        self.audio_engine.stop();
        println!("Audio engine stopped.");
    }
}

/// Run the GUI application
pub fn run_gui(shared: SharedState) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RustSynth",
        options,
        Box::new(|_cc| Ok(Box::new(SynthApp::new(shared)))),
    )
}

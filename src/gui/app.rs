//! Main egui application — backend-agnostic version.
//!
//! All audio/MIDI access goes through `Box<dyn SynthBackend>`.

use crate::core::event::WaveformType;
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::SynthParam;
use crate::core::presets::{install_factory_presets, list_presets, load_preset, save_preset, Preset};
use crate::core::voice::PolyphonyMode;
use crate::gui::backend::{MidiLearnState, SynthBackend};
use crate::gui::theme::{panel_background, section_header, THEME};
use egui::*;

/// Main synth application — owns the backend abstraction
pub struct SynthApp {
    /// Backend abstraction (standalone or plugin)
    backend: Box<dyn SynthBackend + 'static>,

    /// UI-only state
    preset_name: String,
    available_presets: Vec<String>,
    selected_preset: Option<usize>,
}

impl SynthApp {
    pub fn new(backend: Box<dyn SynthBackend + 'static>) -> Self {
        match install_factory_presets() {
            Ok(n) if n > 0 => println!("Installed {} factory presets", n),
            Err(e) => eprintln!("Failed to install factory presets: {}", e),
            _ => {}
        }

        let available_presets = list_presets().unwrap_or_default();

        Self {
            backend,
            preset_name: "New Preset".to_string(),
            available_presets,
            selected_preset: None,
        }
    }

    // ========================================================================
    // Parameter helpers
    // ========================================================================

    fn set_param(&mut self, param: SynthParam, value: f32) {
        self.backend.set_param(param, value);
    }

    fn get_param(&self, param: SynthParam) -> f32 {
        self.backend.get_param(param)
    }

    // ========================================================================
    // Preset helpers
    // ========================================================================

    fn apply_preset(&mut self, preset: &Preset) {
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
        self.set_param(SynthParam::PortamentoTime, preset.portamento_time);
        self.set_param(SynthParam::MasterVolume, preset.master_volume);
    }

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
            portamento_time: self.get_param(SynthParam::PortamentoTime),
            master_volume: self.get_param(SynthParam::MasterVolume),
        }
    }

    // ========================================================================
    // UI panels
    // ========================================================================

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(egui::Visuals {
            window_fill: THEME.bg_blue,
            panel_fill: THEME.panel_bg,
            ..egui::Visuals::dark()
        });

        ctx.request_repaint();

        // Let the backend poll MIDI and sync params every frame
        self.backend.update();

        // ── Top bar ──────────────────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("🔊 Rust In Synth")
                        .size(18.0)
                        .strong()
                        .color(THEME.gold),
                );
                ui.label(
                    RichText::new("v0.5.0")
                        .size(10.0)
                        .color(Color32::from_gray(120)),
                );

                ui.separator();

                let cpu_load = self.backend.get_cpu_load();
                let cpu_color = if cpu_load < 50.0 {
                    Color32::from_rgb(100, 200, 100)
                } else if cpu_load < 80.0 {
                    Color32::from_rgb(200, 200, 100)
                } else {
                    Color32::from_rgb(200, 100, 100)
                };
                ui.label(RichText::new("CPU:").size(10.0).color(Color32::from_gray(150)));
                ui.add(
                    egui::ProgressBar::new(cpu_load / 100.0)
                        .desired_width(60.0)
                        .fill(cpu_color),
                );
                ui.label(
                    RichText::new(format!("{:.1}%", cpu_load))
                        .size(10.0)
                        .monospace(),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("Analog-Modeled Subtractive Synthesizer")
                            .size(11.0)
                            .color(Color32::from_gray(180)),
                    );
                });
            });
        });

        // ── Main panel ───────────────────────────────────────────────────────
        egui::CentralPanel::default()
            .frame(Frame::none().fill(THEME.bg_blue))
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_height(ui.available_height());

                        ui.horizontal_top(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(185.0);
                                self.ui_oscillators_compact(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(145.0);
                                self.ui_filter_compact(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(145.0);
                                self.ui_envelopes_compact(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(145.0);
                                self.ui_lfo_compact(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(155.0);
                                self.ui_effects_compact(ui);
                            });
                            ui.separator();
                            ui.vertical(|ui| {
                                ui.set_width(185.0);
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

    // ── Oscillators ──────────────────────────────────────────────────────────

    fn ui_oscillators_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "OSCILLATORS");

        let wf_labels = ["Sin", "Tri", "Saw", "Sqr", "Nse"];

        // OSC 1
        ui.group(|ui| {
            ui.label(
                RichText::new("OSC 1")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(255, 180, 60)),
            );

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
            if ui
                .add(egui::Slider::new(&mut lvl1, 0.0..=1.0).text("Level"))
                .changed()
            {
                self.set_param(SynthParam::Osc1Level, lvl1);
            }

            let mut phase1 = self.get_param(SynthParam::Osc1Phase);
            if ui
                .add(egui::Slider::new(&mut phase1, 0.0..=1.0).text("Phase"))
                .changed()
            {
                self.set_param(SynthParam::Osc1Phase, phase1);
            }

            let mut pan1 = self.backend.osc_pan(1);
            if ui
                .add(egui::Slider::new(&mut pan1, -1.0..=1.0).text("Pan"))
                .changed()
            {
                self.backend.set_osc_pan(1, pan1);
            }
        });

        ui.add_space(4.0);

        // OSC 2
        ui.group(|ui| {
            ui.label(
                RichText::new("OSC 2")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(255, 180, 60)),
            );

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
            if ui
                .add(egui::Slider::new(&mut lvl2, 0.0..=1.0).text("Level"))
                .changed()
            {
                self.set_param(SynthParam::Osc2Level, lvl2);
            }

            let mut semi2 = self.get_param(SynthParam::Osc2Semitones);
            if ui
                .add(egui::Slider::new(&mut semi2, -24.0..=24.0).text("Semi"))
                .changed()
            {
                self.set_param(SynthParam::Osc2Semitones, semi2);
            }

            let mut cents2 = self.get_param(SynthParam::Osc2Cents);
            if ui
                .add(egui::Slider::new(&mut cents2, -100.0..=100.0).text("Cents"))
                .changed()
            {
                self.set_param(SynthParam::Osc2Cents, cents2);
            }

            let mut phase2 = self.get_param(SynthParam::Osc2Phase);
            if ui
                .add(egui::Slider::new(&mut phase2, 0.0..=1.0).text("Phase"))
                .changed()
            {
                self.set_param(SynthParam::Osc2Phase, phase2);
            }

            let mut pan2 = self.backend.osc_pan(2);
            if ui
                .add(egui::Slider::new(&mut pan2, -1.0..=1.0).text("Pan"))
                .changed()
            {
                self.backend.set_osc_pan(2, pan2);
            }
        });

        ui.add_space(4.0);

        // OSC 3
        ui.group(|ui| {
            ui.label(
                RichText::new("OSC 3")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(255, 180, 60)),
            );

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
            if ui
                .add(egui::Slider::new(&mut lvl3, 0.0..=1.0).text("Level"))
                .changed()
            {
                self.set_param(SynthParam::Osc3Level, lvl3);
            }

            let mut semi3 = self.get_param(SynthParam::Osc3Semitones);
            if ui
                .add(egui::Slider::new(&mut semi3, -24.0..=24.0).text("Semi"))
                .changed()
            {
                self.set_param(SynthParam::Osc3Semitones, semi3);
            }

            let mut cents3 = self.get_param(SynthParam::Osc3Cents);
            if ui
                .add(egui::Slider::new(&mut cents3, -100.0..=100.0).text("Cents"))
                .changed()
            {
                self.set_param(SynthParam::Osc3Cents, cents3);
            }

            let mut phase3 = self.get_param(SynthParam::Osc3Phase);
            if ui
                .add(egui::Slider::new(&mut phase3, 0.0..=1.0).text("Phase"))
                .changed()
            {
                self.set_param(SynthParam::Osc3Phase, phase3);
            }

            let mut pan3 = self.backend.osc_pan(3);
            if ui
                .add(egui::Slider::new(&mut pan3, -1.0..=1.0).text("Pan"))
                .changed()
            {
                self.backend.set_osc_pan(3, pan3);
            }
        });

        ui.add_space(4.0);

        // Stereo Width
        ui.group(|ui| {
            ui.label(
                RichText::new("STEREO")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(100, 200, 255)),
            );
            let mut width = self.backend.stereo_width();
            if ui
                .add(egui::Slider::new(&mut width, 0.0..=2.0).text("Width"))
                .changed()
            {
                self.backend.set_stereo_width(width);
            }
        });
    }

    // ── Filter ───────────────────────────────────────────────────────────────

    fn ui_filter_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "FILTER");

        let mut cutoff = self.get_param(SynthParam::FilterCutoff);
        if ui
            .add(
                egui::Slider::new(&mut cutoff, 20.0..=20000.0)
                    .text("Cutoff")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::FilterCutoff, cutoff);
        }

        let mut res = self.get_param(SynthParam::FilterResonance);
        if ui
            .add(egui::Slider::new(&mut res, 0.0..=1.0).text("Res"))
            .changed()
        {
            self.set_param(SynthParam::FilterResonance, res);
        }

        ui.add_space(8.0);
        ui.label(RichText::new("Env Amount").size(10.0));
        let mut env_amt = self.get_param(SynthParam::FilterEnvAmount);
        if ui
            .add(egui::Slider::new(&mut env_amt, -1.0..=1.0).text("Amt"))
            .changed()
        {
            self.set_param(SynthParam::FilterEnvAmount, env_amt);
        }
    }

    // ── Envelopes ────────────────────────────────────────────────────────────

    fn ui_envelopes_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "AMP ENV");

        let mut atk = self.get_param(SynthParam::Attack);
        if ui
            .add(
                egui::Slider::new(&mut atk, 0.001..=5.0)
                    .text("Attack")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::Attack, atk);
        }

        let mut dec = self.get_param(SynthParam::Decay);
        if ui
            .add(
                egui::Slider::new(&mut dec, 0.001..=5.0)
                    .text("Decay")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::Decay, dec);
        }

        let mut sus = self.get_param(SynthParam::Sustain);
        if ui
            .add(egui::Slider::new(&mut sus, 0.0..=1.0).text("Sustain"))
            .changed()
        {
            self.set_param(SynthParam::Sustain, sus);
        }

        let mut rel = self.get_param(SynthParam::Release);
        if ui
            .add(
                egui::Slider::new(&mut rel, 0.001..=5.0)
                    .text("Release")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::Release, rel);
        }

        ui.add_space(12.0);
        section_header(ui, "FILTER ENV");

        let mut fatk = self.get_param(SynthParam::FilterAttack);
        if ui
            .add(
                egui::Slider::new(&mut fatk, 0.001..=5.0)
                    .text("Attack")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::FilterAttack, fatk);
        }

        let mut fdec = self.get_param(SynthParam::FilterDecay);
        if ui
            .add(
                egui::Slider::new(&mut fdec, 0.001..=5.0)
                    .text("Decay")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::FilterDecay, fdec);
        }

        let mut fsus = self.get_param(SynthParam::FilterSustain);
        if ui
            .add(egui::Slider::new(&mut fsus, 0.0..=1.0).text("Sustain"))
            .changed()
        {
            self.set_param(SynthParam::FilterSustain, fsus);
        }

        let mut frel = self.get_param(SynthParam::FilterRelease);
        if ui
            .add(
                egui::Slider::new(&mut frel, 0.001..=5.0)
                    .text("Release")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::FilterRelease, frel);
        }

        ui.add_space(12.0);
        section_header(ui, "MASTER");

        let mut port_time = self.get_param(SynthParam::PortamentoTime);
        if ui
            .add(
                egui::Slider::new(&mut port_time, 0.0..=3.0)
                    .text("Portamento")
                    .logarithmic(true),
            )
            .changed()
        {
            self.set_param(SynthParam::PortamentoTime, port_time);
        }

        let mut vol = self.get_param(SynthParam::MasterVolume);
        if ui
            .add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Volume"))
            .changed()
        {
            self.set_param(SynthParam::MasterVolume, vol);
        }
    }

    // ── LFO ──────────────────────────────────────────────────────────────────

    fn ui_lfo_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "LFO");

        let mut rate = self.get_param(SynthParam::LfoRate);
        if ui
            .add(egui::Slider::new(&mut rate, 0.1..=20.0).text("Rate"))
            .changed()
        {
            self.set_param(SynthParam::LfoRate, rate);
        }

        let mut depth = self.get_param(SynthParam::LfoDepth);
        if ui
            .add(egui::Slider::new(&mut depth, 0.0..=1.0).text("Depth"))
            .changed()
        {
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
        section_header(ui, "PITCH / VOICE");

        let current_mode = self.backend.polyphony_mode();
        ui.horizontal(|ui| {
            ui.label("Mode:");
            let mono_selected = current_mode == PolyphonyMode::Mono;
            if ui.selectable_label(mono_selected, "MONO").clicked() && !mono_selected {
                self.backend.set_polyphony_mode(PolyphonyMode::Mono);
            }
            let poly_selected = current_mode == PolyphonyMode::Poly;
            if ui.selectable_label(poly_selected, "POLY").clicked() && !poly_selected {
                self.backend.set_polyphony_mode(PolyphonyMode::Poly);
            }
        });

        let active = self.backend.active_voice_count();
        let max = self.backend.max_voices();
        ui.label(
            RichText::new(format!("Voices: {}/{}", active, max))
                .size(10.0)
                .color(Color32::from_gray(150)),
        );

        let mut bend = self.get_param(SynthParam::PitchBendRange);
        if ui
            .add(egui::Slider::new(&mut bend, 1.0..=24.0).text("Bend Range"))
            .changed()
        {
            self.set_param(SynthParam::PitchBendRange, bend);
        }
    }

    // ── Effects ──────────────────────────────────────────────────────────────

    fn ui_effects_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "EFFECTS");

        // DELAY
        ui.group(|ui| {
            let mut delay_on = self.backend.delay_enabled();
            if ui
                .checkbox(&mut delay_on, RichText::new("DELAY").size(10.0).strong())
                .changed()
            {
                self.backend.set_delay_enabled(delay_on);
            }

            if delay_on {
                let mut time = self.backend.delay_time();
                if ui
                    .add(
                        egui::Slider::new(&mut time, 0.05..=1.0)
                            .text("Time")
                            .suffix("s"),
                    )
                    .changed()
                {
                    self.backend.set_delay_time(time);
                }

                let mut feedback = self.backend.delay_feedback();
                if ui
                    .add(egui::Slider::new(&mut feedback, 0.0..=0.9).text("Feedback"))
                    .changed()
                {
                    self.backend.set_delay_feedback(feedback);
                }

                let mut mix = self.backend.delay_mix();
                if ui
                    .add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix"))
                    .changed()
                {
                    self.backend.set_delay_mix(mix);
                }
            }
        });

        ui.add_space(4.0);

        // REVERB
        ui.group(|ui| {
            let mut reverb_on = self.backend.reverb_enabled();
            if ui
                .checkbox(&mut reverb_on, RichText::new("REVERB").size(10.0).strong())
                .changed()
            {
                self.backend.set_reverb_enabled(reverb_on);
            }

            if reverb_on {
                let mut room = self.backend.reverb_room_size();
                if ui
                    .add(egui::Slider::new(&mut room, 0.0..=1.0).text("Room"))
                    .changed()
                {
                    self.backend.set_reverb_room_size(room);
                }

                let mut damp = self.backend.reverb_damping();
                if ui
                    .add(egui::Slider::new(&mut damp, 0.0..=1.0).text("Damp"))
                    .changed()
                {
                    self.backend.set_reverb_damping(damp);
                }

                let mut mix = self.backend.reverb_mix();
                if ui
                    .add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix"))
                    .changed()
                {
                    self.backend.set_reverb_mix(mix);
                }
            }
        });

        ui.add_space(4.0);

        // CHORUS
        ui.group(|ui| {
            let mut chorus_on = self.backend.chorus_enabled();
            if ui
                .checkbox(&mut chorus_on, RichText::new("CHORUS").size(10.0).strong())
                .changed()
            {
                self.backend.set_chorus_enabled(chorus_on);
            }

            if chorus_on {
                let mut rate = self.backend.chorus_rate();
                if ui
                    .add(
                        egui::Slider::new(&mut rate, 0.1..=5.0)
                            .text("Rate")
                            .suffix("Hz"),
                    )
                    .changed()
                {
                    self.backend.set_chorus_rate(rate);
                }

                let mut depth = self.backend.chorus_depth();
                if ui
                    .add(
                        egui::Slider::new(&mut depth, 0.0..=10.0)
                            .text("Depth")
                            .suffix("ms"),
                    )
                    .changed()
                {
                    self.backend.set_chorus_depth(depth);
                }

                let mut mix = self.backend.chorus_mix();
                if ui
                    .add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix"))
                    .changed()
                {
                    self.backend.set_chorus_mix(mix);
                }
            }
        });
    }

    // ── Presets ───────────────────────────────────────────────────────────────

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

    // ── MIDI ─────────────────────────────────────────────────────────────────

    fn ui_midi_compact(&mut self, ui: &mut Ui) {
        section_header(ui, "MIDI");

        let status = if self.backend.midi_connected() {
            "✅ Connected"
        } else {
            "❌ Disconnected"
        };
        ui.label(RichText::new(status).size(11.0));

        // Port selector (not relevant for plugin, but rendered anyway for unified GUI)
        let ports = self.backend.midi_ports();
        let selected_port = self.backend.selected_midi_port();
        egui::ComboBox::from_id_source("midi_port")
            .selected_text(
                selected_port
                    .and_then(|i| ports.get(i))
                    .map(|s| s.as_str())
                    .unwrap_or("No ports"),
            )
            .show_ui(ui, |ui| {
                for (i, port_name) in ports.iter().enumerate() {
                    let is_selected = selected_port == Some(i);
                    if ui.selectable_label(is_selected, port_name).clicked() {
                        self.backend.connect_midi_port(i);
                    }
                }
            });

        ui.horizontal(|ui| {
            if ui.button("🔄").on_hover_text("Refresh ports").clicked() {
                self.backend.refresh_midi_ports();
            }
            if ui.button("🔌 Connect").clicked() {
                if let Some(port_idx) = self.backend.selected_midi_port() {
                    self.backend.connect_midi_port(port_idx);
                }
            }
        });

        // MIDI Channel
        ui.add_space(4.0);
        let channel_labels = [
            "All", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14",
            "15", "16",
        ];
        let current_channel = self.backend.midi_channel_display() as usize;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Channel:").size(10.0));
            egui::ComboBox::from_id_source("midi_channel")
                .width(50.0)
                .selected_text(channel_labels[current_channel.min(16)])
                .show_ui(ui, |ui| {
                    for (i, label) in channel_labels.iter().enumerate() {
                        if ui
                            .selectable_label(current_channel == i, *label)
                            .clicked()
                        {
                            self.backend.set_midi_channel(i as u8);
                        }
                    }
                });
        });

        // MIDI Learn
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        match self.backend.midi_learn_state() {
            MidiLearnState::Waiting(param) => {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🎯 Learning:")
                            .size(10.0)
                            .color(Color32::YELLOW),
                    );
                    ui.label(RichText::new(param.name()).size(10.0).strong());
                });
                if ui.button("Cancel").clicked() {
                    self.backend.cancel_midi_learn();
                }
            }
            MidiLearnState::Idle => {
                ui.label(RichText::new("CC Learn:").size(10.0));
                egui::ComboBox::from_id_source("midi_learn_param")
                    .selected_text("Select param...")
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for param in SynthParam::all() {
                            if ui.selectable_label(false, param.name()).clicked() {
                                self.backend.start_midi_learn(*param);
                            }
                        }
                    });
            }
        }

        // Custom mappings
        let mappings = self.backend.custom_cc_mappings();
        if !mappings.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("{} custom mappings", mappings.len()))
                    .size(9.0)
                    .color(Color32::from_gray(120)),
            );
            if ui.small_button("Clear all").clicked() {
                self.backend.clear_cc_mappings();
            }
        }

        // CC Activity
        ui.add_space(8.0);
        ui.label(RichText::new("CC Activity:").size(10.0));

        let cc_list = self.backend.midi_cc_activity();
        egui::ScrollArea::vertical()
            .id_source("midi_cc_activity")
            .max_height(80.0)
            .show(ui, |ui| {
                for (cc, val) in cc_list.iter().take(10) {
                    let mapped = mappings.iter().find(|m| m.cc == *cc).map(|m| m.param.short_name());
                    let cc_text = if let Some(param_name) = mapped {
                        format!("CC {:>3} → {}", cc, param_name)
                    } else {
                        format!("CC {:>3}:", cc)
                    };
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(cc_text).size(9.0).monospace());
                        ui.add(
                            egui::ProgressBar::new(*val as f32 / 127.0).desired_width(40.0),
                        );
                    });
                }
            });
    }

    fn on_exit(&mut self) {
        // The backend owns the audio engine; when it drops, audio stops automatically.
        println!("Goodbye!");
    }
}

impl eframe::App for SynthApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        SynthApp::update(self, ctx, frame);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        SynthApp::on_exit(self);
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

/// Run the standalone GUI application.
///
/// Creates a `StandaloneBackend`, wraps it in `SynthApp`, and runs eframe.
#[cfg(not(feature = "plugin"))]
pub fn run_gui(shared: crate::gui::SharedState) -> Result<(), eframe::Error> {
    use crate::gui::StandaloneBackend;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 580.0])
            .with_min_inner_size([1350.0, 550.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Rust In Synth",
        options,
        Box::new(|_cc| {
            let backend = Box::new(StandaloneBackend::new(shared));
            Ok(Box::new(SynthApp::new(backend)))
        }),
    )
}

//! Shared GUI panels — rendered identically for standalone and plugin.
//!
//! Every panel is a free function taking `&mut dyn SynthBackend`, so the exact
//! same widget code drives both `SynthApp` (standalone) and the plugin editor.
//! This is the single source of truth for the synth UI — there is no second copy.

use egui::*;

use crate::core::event::WaveformType;
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::SynthParam;
use crate::core::presets::{list_presets, load_preset, save_preset, Preset};
use crate::core::voice::PolyphonyMode;
use crate::gui::backend::{MidiLearnState, SynthBackend};
use crate::gui::theme::{section_header, THEME};

/// GUI-only preset state shared by both the standalone app and the plugin editor.
pub struct PresetState {
    pub preset_name: String,
    pub available_presets: Vec<String>,
    pub selected_preset: Option<usize>,
}

impl PresetState {
    pub fn new() -> Self {
        Self {
            preset_name: "New Preset".to_string(),
            available_presets: list_presets().unwrap_or_default(),
            selected_preset: None,
        }
    }
}

impl Default for PresetState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Window shell (top bar + panel layout)
// ============================================================================

/// How the shared shell should differ between the standalone app and the
/// plugin editor. The layout itself is identical; only these knobs change.
pub struct ShellConfig {
    /// Version string shown next to the title (e.g. "v1.0.1" or "v1.0.1 (Plugin)").
    pub version: String,
    /// Show the CPU-load meter in the top bar (standalone only).
    pub show_cpu: bool,
    /// Include the MIDI panel in the rightmost column (standalone only).
    pub show_midi_panel: bool,
}

/// Render the complete synth UI (top bar + all panels) onto `ctx`.
///
/// Single source of truth for the window shell, shared by the standalone app
/// and the plugin editor. Per-host differences are expressed via `ShellConfig`
/// rather than duplicated layout code.
// egui 0.34 deprecates the top-level `Panel::show(ctx)` / `CentralPanel::show(ctx)`
// in favour of `show_inside(ui)`, which needs a `&mut Ui` we don't have at the
// eframe/baseview entry points — `show(ctx)` is still the supported top-level
// pattern. Allow it for these calls only.
#[allow(deprecated)]
pub fn shell(
    ctx: &egui::Context,
    b: &mut dyn SynthBackend,
    preset_state: &mut PresetState,
    cfg: &ShellConfig,
) {
    ctx.set_visuals(egui::Visuals {
        window_fill: THEME.bg_blue,
        panel_fill: THEME.panel_bg,
        ..egui::Visuals::dark()
    });

    // ── Top bar ─────────────────────────────────────────────────────────────
    egui::Panel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("🔊 Rust In Synth")
                    .size(18.0)
                    .strong()
                    .color(THEME.gold),
            );
            ui.label(
                RichText::new(&cfg.version)
                    .size(10.0)
                    .color(Color32::from_gray(120)),
            );

            if cfg.show_cpu {
                ui.separator();
                let cpu_load = b.get_cpu_load();
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
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new("Analog-Modeled Subtractive Synthesizer")
                        .size(11.0)
                        .color(Color32::from_gray(180)),
                );
            });
        });
    });

    // ── Main panel ──────────────────────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(Frame::new().fill(THEME.bg_blue))
        .show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(185.0);
                            oscillators(ui, b);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            filter(ui, b);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            envelopes(ui, b);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(145.0);
                            lfo(ui, b);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(155.0);
                            effects(ui, b);
                        });
                        ui.separator();
                        ui.vertical(|ui| {
                            ui.set_width(185.0);
                            presets(ui, b, preset_state);
                            if cfg.show_midi_panel {
                                ui.add_space(16.0);
                                ui.separator();
                                ui.add_space(8.0);
                                midi(ui, b);
                            }
                        });
                    });
                });
        });
}

// ============================================================================
// Oscillators
// ============================================================================

pub fn oscillators(ui: &mut Ui, b: &mut dyn SynthBackend) {
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

        let wf1 = b.get_param(SynthParam::Osc1Waveform) as usize;
        egui::ComboBox::from_id_salt("osc1_wf")
            .width(60.0)
            .selected_text(wf_labels[wf1.min(4)])
            .show_ui(ui, |ui| {
                for (i, label) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf1 == i, *label).clicked() {
                        b.set_param(SynthParam::Osc1Waveform, i as f32);
                    }
                }
            });

        let mut lvl1 = b.get_param(SynthParam::Osc1Level);
        if ui.add(egui::Slider::new(&mut lvl1, 0.0..=1.0).text("Level")).changed() {
            b.set_param(SynthParam::Osc1Level, lvl1);
        }

        let mut phase1 = b.get_param(SynthParam::Osc1Phase);
        if ui.add(egui::Slider::new(&mut phase1, 0.0..=1.0).text("Phase")).changed() {
            b.set_param(SynthParam::Osc1Phase, phase1);
        }

        let mut pan1 = b.osc_pan(1);
        if ui.add(egui::Slider::new(&mut pan1, -1.0..=1.0).text("Pan")).changed() {
            b.set_osc_pan(1, pan1);
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

        let wf2 = b.get_param(SynthParam::Osc2Waveform) as usize;
        egui::ComboBox::from_id_salt("osc2_wf")
            .width(60.0)
            .selected_text(wf_labels[wf2.min(4)])
            .show_ui(ui, |ui| {
                for (i, label) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf2 == i, *label).clicked() {
                        b.set_param(SynthParam::Osc2Waveform, i as f32);
                    }
                }
            });

        let mut lvl2 = b.get_param(SynthParam::Osc2Level);
        if ui.add(egui::Slider::new(&mut lvl2, 0.0..=1.0).text("Level")).changed() {
            b.set_param(SynthParam::Osc2Level, lvl2);
        }

        let mut semi2 = b.get_param(SynthParam::Osc2Semitones);
        if ui.add(egui::Slider::new(&mut semi2, -24.0..=24.0).text("Semi")).changed() {
            b.set_param(SynthParam::Osc2Semitones, semi2);
        }

        let mut cents2 = b.get_param(SynthParam::Osc2Cents);
        if ui.add(egui::Slider::new(&mut cents2, -100.0..=100.0).text("Cents")).changed() {
            b.set_param(SynthParam::Osc2Cents, cents2);
        }

        let mut phase2 = b.get_param(SynthParam::Osc2Phase);
        if ui.add(egui::Slider::new(&mut phase2, 0.0..=1.0).text("Phase")).changed() {
            b.set_param(SynthParam::Osc2Phase, phase2);
        }

        let mut pan2 = b.osc_pan(2);
        if ui.add(egui::Slider::new(&mut pan2, -1.0..=1.0).text("Pan")).changed() {
            b.set_osc_pan(2, pan2);
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

        let wf3 = b.get_param(SynthParam::Osc3Waveform) as usize;
        egui::ComboBox::from_id_salt("osc3_wf")
            .width(60.0)
            .selected_text(wf_labels[wf3.min(4)])
            .show_ui(ui, |ui| {
                for (i, label) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf3 == i, *label).clicked() {
                        b.set_param(SynthParam::Osc3Waveform, i as f32);
                    }
                }
            });

        let mut lvl3 = b.get_param(SynthParam::Osc3Level);
        if ui.add(egui::Slider::new(&mut lvl3, 0.0..=1.0).text("Level")).changed() {
            b.set_param(SynthParam::Osc3Level, lvl3);
        }

        let mut semi3 = b.get_param(SynthParam::Osc3Semitones);
        if ui.add(egui::Slider::new(&mut semi3, -24.0..=24.0).text("Semi")).changed() {
            b.set_param(SynthParam::Osc3Semitones, semi3);
        }

        let mut cents3 = b.get_param(SynthParam::Osc3Cents);
        if ui.add(egui::Slider::new(&mut cents3, -100.0..=100.0).text("Cents")).changed() {
            b.set_param(SynthParam::Osc3Cents, cents3);
        }

        let mut phase3 = b.get_param(SynthParam::Osc3Phase);
        if ui.add(egui::Slider::new(&mut phase3, 0.0..=1.0).text("Phase")).changed() {
            b.set_param(SynthParam::Osc3Phase, phase3);
        }

        let mut pan3 = b.osc_pan(3);
        if ui.add(egui::Slider::new(&mut pan3, -1.0..=1.0).text("Pan")).changed() {
            b.set_osc_pan(3, pan3);
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
        let mut width = b.stereo_width();
        if ui.add(egui::Slider::new(&mut width, 0.0..=2.0).text("Width")).changed() {
            b.set_stereo_width(width);
        }
    });
}

// ============================================================================
// Filter
// ============================================================================

pub fn filter(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "FILTER");

    let mut cutoff = b.get_param(SynthParam::FilterCutoff);
    if ui
        .add(egui::Slider::new(&mut cutoff, 20.0..=20000.0).text("Cutoff").logarithmic(true))
        .changed()
    {
        b.set_param(SynthParam::FilterCutoff, cutoff);
    }

    let mut res = b.get_param(SynthParam::FilterResonance);
    if ui.add(egui::Slider::new(&mut res, 0.0..=1.0).text("Res")).changed() {
        b.set_param(SynthParam::FilterResonance, res);
    }

    ui.add_space(8.0);
    ui.label(RichText::new("Env Amount").size(10.0));
    let mut env_amt = b.get_param(SynthParam::FilterEnvAmount);
    if ui.add(egui::Slider::new(&mut env_amt, -1.0..=1.0).text("Amt")).changed() {
        b.set_param(SynthParam::FilterEnvAmount, env_amt);
    }
}

// ============================================================================
// Envelopes + Master
// ============================================================================

pub fn envelopes(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "AMP ENV");

    let mut atk = b.get_param(SynthParam::Attack);
    if ui.add(egui::Slider::new(&mut atk, 0.001..=5.0).text("Attack").logarithmic(true)).changed() {
        b.set_param(SynthParam::Attack, atk);
    }

    let mut dec = b.get_param(SynthParam::Decay);
    if ui.add(egui::Slider::new(&mut dec, 0.001..=5.0).text("Decay").logarithmic(true)).changed() {
        b.set_param(SynthParam::Decay, dec);
    }

    let mut sus = b.get_param(SynthParam::Sustain);
    if ui.add(egui::Slider::new(&mut sus, 0.0..=1.0).text("Sustain")).changed() {
        b.set_param(SynthParam::Sustain, sus);
    }

    let mut rel = b.get_param(SynthParam::Release);
    if ui.add(egui::Slider::new(&mut rel, 0.001..=5.0).text("Release").logarithmic(true)).changed() {
        b.set_param(SynthParam::Release, rel);
    }

    ui.add_space(12.0);
    section_header(ui, "FILTER ENV");

    let mut fatk = b.get_param(SynthParam::FilterAttack);
    if ui.add(egui::Slider::new(&mut fatk, 0.001..=5.0).text("Attack").logarithmic(true)).changed() {
        b.set_param(SynthParam::FilterAttack, fatk);
    }

    let mut fdec = b.get_param(SynthParam::FilterDecay);
    if ui.add(egui::Slider::new(&mut fdec, 0.001..=5.0).text("Decay").logarithmic(true)).changed() {
        b.set_param(SynthParam::FilterDecay, fdec);
    }

    let mut fsus = b.get_param(SynthParam::FilterSustain);
    if ui.add(egui::Slider::new(&mut fsus, 0.0..=1.0).text("Sustain")).changed() {
        b.set_param(SynthParam::FilterSustain, fsus);
    }

    let mut frel = b.get_param(SynthParam::FilterRelease);
    if ui.add(egui::Slider::new(&mut frel, 0.001..=5.0).text("Release").logarithmic(true)).changed() {
        b.set_param(SynthParam::FilterRelease, frel);
    }

    ui.add_space(12.0);
    section_header(ui, "MASTER");

    let mut port_time = b.get_param(SynthParam::PortamentoTime);
    if ui
        .add(egui::Slider::new(&mut port_time, 0.0..=3.0).text("Portamento").logarithmic(true))
        .changed()
    {
        b.set_param(SynthParam::PortamentoTime, port_time);
    }

    let mut vol = b.get_param(SynthParam::MasterVolume);
    if ui.add(egui::Slider::new(&mut vol, 0.0..=1.0).text("Volume")).changed() {
        b.set_param(SynthParam::MasterVolume, vol);
    }
}

// ============================================================================
// LFO + Pitch / Voice
// ============================================================================

pub fn lfo(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "LFO");

    let mut rate = b.get_param(SynthParam::LfoRate);
    if ui.add(egui::Slider::new(&mut rate, 0.1..=20.0).text("Rate")).changed() {
        b.set_param(SynthParam::LfoRate, rate);
    }

    let mut depth = b.get_param(SynthParam::LfoDepth);
    if ui.add(egui::Slider::new(&mut depth, 0.0..=1.0).text("Depth")).changed() {
        b.set_param(SynthParam::LfoDepth, depth);
    }

    ui.label(RichText::new("Waveform").size(10.0));
    let lfo_wf_labels = ["Sin", "Tri", "Sqr", "Saw", "Rnd"];
    let lfo_wf = b.get_param(SynthParam::LfoWaveform) as usize;
    egui::ComboBox::from_id_salt("lfo_wf")
        .width(70.0)
        .selected_text(lfo_wf_labels[lfo_wf.min(4)])
        .show_ui(ui, |ui| {
            for (i, label) in lfo_wf_labels.iter().enumerate() {
                if ui.selectable_label(lfo_wf == i, *label).clicked() {
                    b.set_param(SynthParam::LfoWaveform, i as f32);
                }
            }
        });

    ui.label(RichText::new("Destination").size(10.0));
    let lfo_dest_labels = ["Off", "Pitch", "Filter", "Amp"];
    let lfo_dest = b.get_param(SynthParam::LfoDestination) as usize;
    egui::ComboBox::from_id_salt("lfo_dest")
        .width(70.0)
        .selected_text(lfo_dest_labels[lfo_dest.min(3)])
        .show_ui(ui, |ui| {
            for (i, label) in lfo_dest_labels.iter().enumerate() {
                if ui.selectable_label(lfo_dest == i, *label).clicked() {
                    b.set_param(SynthParam::LfoDestination, i as f32);
                }
            }
        });

    ui.add_space(12.0);
    section_header(ui, "PITCH / VOICE");

    let current_mode = b.polyphony_mode();
    ui.horizontal(|ui| {
        ui.label("Mode:");
        let mono_selected = current_mode == PolyphonyMode::Mono;
        if ui.selectable_label(mono_selected, "MONO").clicked() && !mono_selected {
            b.set_polyphony_mode(PolyphonyMode::Mono);
        }
        let poly_selected = current_mode == PolyphonyMode::Poly;
        if ui.selectable_label(poly_selected, "POLY").clicked() && !poly_selected {
            b.set_polyphony_mode(PolyphonyMode::Poly);
        }
    });

    // Voice count is only meaningful when the backend exposes it (standalone).
    let max = b.max_voices();
    if max > 0 {
        let active = b.active_voice_count();
        ui.label(
            RichText::new(format!("Voices: {}/{}", active, max))
                .size(10.0)
                .color(Color32::from_gray(150)),
        );
    }

    let mut bend = b.get_param(SynthParam::PitchBendRange);
    if ui.add(egui::Slider::new(&mut bend, 1.0..=24.0).text("Bend Range")).changed() {
        b.set_param(SynthParam::PitchBendRange, bend);
    }
}

// ============================================================================
// Effects
// ============================================================================

pub fn effects(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "EFFECTS");

    // DELAY
    ui.group(|ui| {
        let mut delay_on = b.delay_enabled();
        if ui.checkbox(&mut delay_on, RichText::new("DELAY").size(10.0).strong()).changed() {
            b.set_delay_enabled(delay_on);
        }

        if delay_on {
            let mut time = b.delay_time();
            if ui.add(egui::Slider::new(&mut time, 0.05..=1.0).text("Time").suffix("s")).changed() {
                b.set_delay_time(time);
            }
            let mut feedback = b.delay_feedback();
            if ui.add(egui::Slider::new(&mut feedback, 0.0..=0.9).text("Feedback")).changed() {
                b.set_delay_feedback(feedback);
            }
            let mut mix = b.delay_mix();
            if ui.add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix")).changed() {
                b.set_delay_mix(mix);
            }
        }
    });

    ui.add_space(4.0);

    // REVERB
    ui.group(|ui| {
        let mut reverb_on = b.reverb_enabled();
        if ui.checkbox(&mut reverb_on, RichText::new("REVERB").size(10.0).strong()).changed() {
            b.set_reverb_enabled(reverb_on);
        }

        if reverb_on {
            let mut room = b.reverb_room_size();
            if ui.add(egui::Slider::new(&mut room, 0.0..=1.0).text("Room")).changed() {
                b.set_reverb_room_size(room);
            }
            let mut damp = b.reverb_damping();
            if ui.add(egui::Slider::new(&mut damp, 0.0..=1.0).text("Damp")).changed() {
                b.set_reverb_damping(damp);
            }
            let mut mix = b.reverb_mix();
            if ui.add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix")).changed() {
                b.set_reverb_mix(mix);
            }
        }
    });

    ui.add_space(4.0);

    // CHORUS
    ui.group(|ui| {
        let mut chorus_on = b.chorus_enabled();
        if ui.checkbox(&mut chorus_on, RichText::new("CHORUS").size(10.0).strong()).changed() {
            b.set_chorus_enabled(chorus_on);
        }

        if chorus_on {
            let mut rate = b.chorus_rate();
            if ui.add(egui::Slider::new(&mut rate, 0.1..=5.0).text("Rate").suffix("Hz")).changed() {
                b.set_chorus_rate(rate);
            }
            let mut depth = b.chorus_depth();
            if ui.add(egui::Slider::new(&mut depth, 0.0..=10.0).text("Depth").suffix("ms")).changed() {
                b.set_chorus_depth(depth);
            }
            let mut mix = b.chorus_mix();
            if ui.add(egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix")).changed() {
                b.set_chorus_mix(mix);
            }
        }
    });
}

// ============================================================================
// Presets
// ============================================================================

pub fn presets(ui: &mut Ui, b: &mut dyn SynthBackend, state: &mut PresetState) {
    section_header(ui, "PRESETS");

    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut state.preset_name);
    });

    if ui.button("💾 Save").clicked() {
        let preset = create_preset(b, &state.preset_name);
        if let Err(e) = save_preset(&preset) {
            eprintln!("Failed to save preset: {}", e);
        } else {
            state.available_presets = list_presets().unwrap_or_default();
        }
    }

    ui.add_space(8.0);
    ui.label("Load:");

    let mut clicked_name: Option<String> = None;
    egui::ScrollArea::vertical()
        .id_salt("preset_list")
        .max_height(80.0)
        .show(ui, |ui| {
            for (i, name) in state.available_presets.iter().enumerate() {
                let is_selected = state.selected_preset == Some(i);
                if ui.selectable_label(is_selected, name).clicked() {
                    state.selected_preset = Some(i);
                    clicked_name = Some(name.clone());
                }
            }
        });
    if let Some(ref name) = clicked_name {
        if let Ok(preset) = load_preset(name) {
            state.preset_name = preset.name.clone();
            apply_preset(b, &preset);
        }
    }
}

// ============================================================================
// MIDI
// ============================================================================

pub fn midi(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "MIDI");

    let status = if b.midi_connected() {
        "✅ Connected"
    } else {
        "❌ Disconnected"
    };
    ui.label(RichText::new(status).size(11.0));

    // Port selector (only meaningful for standalone; plugin reports no ports)
    let ports = b.midi_ports();
    let selected_port = b.selected_midi_port();
    egui::ComboBox::from_id_salt("midi_port")
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
                    b.connect_midi_port(i);
                }
            }
        });

    ui.horizontal(|ui| {
        if ui.button("🔄").on_hover_text("Refresh ports").clicked() {
            b.refresh_midi_ports();
        }
        if ui.button("🔌 Connect").clicked() {
            if let Some(port_idx) = b.selected_midi_port() {
                b.connect_midi_port(port_idx);
            }
        }
    });

    // MIDI Channel
    ui.add_space(4.0);
    let channel_labels = [
        "All", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
        "16",
    ];
    let current_channel = b.midi_channel_display() as usize;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Channel:").size(10.0));
        egui::ComboBox::from_id_salt("midi_channel")
            .width(50.0)
            .selected_text(channel_labels[current_channel.min(16)])
            .show_ui(ui, |ui| {
                for (i, label) in channel_labels.iter().enumerate() {
                    if ui.selectable_label(current_channel == i, *label).clicked() {
                        b.set_midi_channel(i as u8);
                    }
                }
            });
    });

    // MIDI Learn
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(4.0);

    match b.midi_learn_state() {
        MidiLearnState::Waiting(param) => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 Learning:").size(10.0).color(Color32::YELLOW));
                ui.label(RichText::new(param.name()).size(10.0).strong());
            });
            if ui.button("Cancel").clicked() {
                b.cancel_midi_learn();
            }
        }
        MidiLearnState::Idle => {
            ui.label(RichText::new("CC Learn:").size(10.0));
            egui::ComboBox::from_id_salt("midi_learn_param")
                .selected_text("Select param...")
                .width(120.0)
                .show_ui(ui, |ui| {
                    for param in SynthParam::all() {
                        if ui.selectable_label(false, param.name()).clicked() {
                            b.start_midi_learn(*param);
                        }
                    }
                });
        }
    }

    // Custom mappings
    let mappings = b.custom_cc_mappings();
    if !mappings.is_empty() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!("{} custom mappings", mappings.len()))
                .size(9.0)
                .color(Color32::from_gray(120)),
        );
        if ui.small_button("Clear all").clicked() {
            b.clear_cc_mappings();
        }
    }

    // CC Activity
    ui.add_space(8.0);
    ui.label(RichText::new("CC Activity:").size(10.0));

    let cc_list = b.midi_cc_activity();
    egui::ScrollArea::vertical()
        .id_salt("midi_cc_activity")
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
                    ui.add(egui::ProgressBar::new(*val as f32 / 127.0).desired_width(40.0));
                });
            }
        });
}

// ============================================================================
// Preset (de)serialisation helpers
// ============================================================================

pub fn apply_preset(b: &mut dyn SynthBackend, preset: &Preset) {
    b.set_param(SynthParam::Osc1Waveform, preset.osc1_waveform as u8 as f32);
    b.set_param(SynthParam::Osc1Level, preset.osc1_level);
    b.set_param(SynthParam::Osc1Phase, preset.osc1_phase);

    b.set_param(SynthParam::Osc2Waveform, preset.osc2_waveform as u8 as f32);
    b.set_param(SynthParam::Osc2Level, preset.osc2_level);
    b.set_param(SynthParam::Osc2Semitones, preset.osc2_semitones as f32);
    b.set_param(SynthParam::Osc2Cents, preset.osc2_cents as f32);
    b.set_param(SynthParam::Osc2Phase, preset.osc2_phase);

    b.set_param(SynthParam::Osc3Waveform, preset.osc3_waveform as u8 as f32);
    b.set_param(SynthParam::Osc3Level, preset.osc3_level);
    b.set_param(SynthParam::Osc3Semitones, preset.osc3_semitones as f32);
    b.set_param(SynthParam::Osc3Cents, preset.osc3_cents as f32);
    b.set_param(SynthParam::Osc3Phase, preset.osc3_phase);

    b.set_param(SynthParam::FilterCutoff, preset.filter_cutoff);
    b.set_param(SynthParam::FilterResonance, preset.filter_resonance);

    b.set_param(SynthParam::Attack, preset.amp_attack);
    b.set_param(SynthParam::Decay, preset.amp_decay);
    b.set_param(SynthParam::Sustain, preset.amp_sustain);
    b.set_param(SynthParam::Release, preset.amp_release);

    b.set_param(SynthParam::FilterAttack, preset.filter_attack);
    b.set_param(SynthParam::FilterDecay, preset.filter_decay);
    b.set_param(SynthParam::FilterSustain, preset.filter_sustain);
    b.set_param(SynthParam::FilterRelease, preset.filter_release);
    b.set_param(SynthParam::FilterEnvAmount, preset.filter_env_amount);

    b.set_param(SynthParam::LfoRate, preset.lfo_rate);
    b.set_param(SynthParam::LfoDepth, preset.lfo_depth);
    b.set_param(SynthParam::LfoWaveform, preset.lfo_waveform as u8 as f32);
    b.set_param(SynthParam::LfoDestination, preset.lfo_destination as u8 as f32);

    b.set_param(SynthParam::PitchBendRange, preset.pitch_bend_range as f32);
    b.set_param(SynthParam::PortamentoTime, preset.portamento_time);
    b.set_param(SynthParam::MasterVolume, preset.master_volume);
}

pub fn create_preset(b: &dyn SynthBackend, name: &str) -> Preset {
    Preset {
        name: name.to_string(),
        version: "1.0".to_string(),

        osc1_waveform: WaveformType::from_index(b.get_param(SynthParam::Osc1Waveform) as u8),
        osc1_level: b.get_param(SynthParam::Osc1Level),
        osc1_phase: b.get_param(SynthParam::Osc1Phase),

        osc2_waveform: WaveformType::from_index(b.get_param(SynthParam::Osc2Waveform) as u8),
        osc2_level: b.get_param(SynthParam::Osc2Level),
        osc2_semitones: b.get_param(SynthParam::Osc2Semitones) as i8,
        osc2_cents: b.get_param(SynthParam::Osc2Cents) as i8,
        osc2_phase: b.get_param(SynthParam::Osc2Phase),

        osc3_waveform: WaveformType::from_index(b.get_param(SynthParam::Osc3Waveform) as u8),
        osc3_level: b.get_param(SynthParam::Osc3Level),
        osc3_semitones: b.get_param(SynthParam::Osc3Semitones) as i8,
        osc3_cents: b.get_param(SynthParam::Osc3Cents) as i8,
        osc3_phase: b.get_param(SynthParam::Osc3Phase),

        filter_cutoff: b.get_param(SynthParam::FilterCutoff),
        filter_resonance: b.get_param(SynthParam::FilterResonance),

        amp_attack: b.get_param(SynthParam::Attack),
        amp_decay: b.get_param(SynthParam::Decay),
        amp_sustain: b.get_param(SynthParam::Sustain),
        amp_release: b.get_param(SynthParam::Release),

        filter_attack: b.get_param(SynthParam::FilterAttack),
        filter_decay: b.get_param(SynthParam::FilterDecay),
        filter_sustain: b.get_param(SynthParam::FilterSustain),
        filter_release: b.get_param(SynthParam::FilterRelease),
        filter_env_amount: b.get_param(SynthParam::FilterEnvAmount),

        lfo_rate: b.get_param(SynthParam::LfoRate),
        lfo_depth: b.get_param(SynthParam::LfoDepth),
        lfo_waveform: match b.get_param(SynthParam::LfoWaveform) as u8 {
            0 => LfoWaveform::Sine,
            1 => LfoWaveform::Triangle,
            2 => LfoWaveform::Square,
            3 => LfoWaveform::Saw,
            _ => LfoWaveform::Random,
        },
        lfo_destination: match b.get_param(SynthParam::LfoDestination) as u8 {
            0 => LfoDestination::Off,
            1 => LfoDestination::Pitch,
            2 => LfoDestination::FilterCutoff,
            _ => LfoDestination::Amplitude,
        },

        pitch_bend_range: b.get_param(SynthParam::PitchBendRange) as u8,
        portamento_time: b.get_param(SynthParam::PortamentoTime),
        master_volume: b.get_param(SynthParam::MasterVolume),
    }
}

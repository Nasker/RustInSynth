//! Plugin GUI editor — unified with the standalone GUI via `SynthApp`.
//!
//! Every frame a fresh `PluginBackend` is constructed (cheap: just Arc clones)
//! and handed to `SynthApp::draw_frame`, which renders all panels without
//! owning the backend across frames.

use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiSettings, EguiState};

use crate::core::event::WaveformType;
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::SynthParam;
use crate::core::presets::{install_factory_presets, list_presets, load_preset, save_preset, Preset};
use crate::core::voice::PolyphonyMode;
use crate::gui::backend::{MidiLearnState, SynthBackend};
use crate::gui::theme::{panel_background, section_header, THEME};
use crate::plugin::RustInSynthParams;
use crate::plugin_gui::backend_plugin::PluginBackend;
use crate::plugin_gui::shared_state::PluginSharedState;
use egui::*;

/// Size of the editor window
pub const EDITOR_WIDTH: u32 = 1100;
pub const EDITOR_HEIGHT: u32 = 680;

/// Persistent GUI-only state that lives across frames.
pub struct EditorState {
    preset_name: String,
    available_presets: Vec<String>,
    selected_preset: Option<usize>,
    shared: PluginSharedState,
}

impl Default for EditorState {
    fn default() -> Self {
        let _ = install_factory_presets();
        Self {
            preset_name: "New Preset".to_string(),
            available_presets: list_presets().unwrap_or_default(),
            selected_preset: None,
            shared: PluginSharedState::new(),
        }
    }
}

/// Create the plugin editor using the unified GUI panels.
pub fn create_editor(
    egui_state: Arc<EguiState>,
    params: Arc<RustInSynthParams>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        egui_state,
        EditorState::default(),
        EguiSettings::default(),
        |_ctx, _queue, _state| {},
        move |ui, setter, _queue, state| {
            // Build a fresh backend for this frame (just Arc clones + a ref)
            let mut backend = PluginBackend::new(
                Arc::clone(&params),
                setter,
                state.shared.clone(),
            );

            let ctx = ui.ctx().clone();

            // Apply theme
            ctx.set_visuals(egui::Visuals {
                window_fill: THEME.bg_blue,
                panel_fill: THEME.panel_bg,
                ..egui::Visuals::dark()
            });

            // ── Top bar ──────────────────────────────────────────────────────
            egui::TopBottomPanel::top("top_bar").show(&ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("🔊 Rust In Synth")
                            .size(18.0)
                            .strong()
                            .color(THEME.gold),
                    );
                    ui.label(
                        RichText::new("v1.0.1 (Plugin)")
                            .size(10.0)
                            .color(Color32::from_gray(120)),
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

            // ── Main panel ───────────────────────────────────────────────────
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(THEME.bg_blue))
                .show(&ctx, |ui| {
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_height(ui.available_height());
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(185.0);
                                    ui_oscillators_compact(ui, &mut backend);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(145.0);
                                    ui_filter_compact(ui, &mut backend);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(145.0);
                                    ui_envelopes_compact(ui, &mut backend);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(145.0);
                                    ui_lfo_compact(ui, &mut backend);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(155.0);
                                    ui_effects_compact(ui, &mut backend);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(185.0);
                                    ui_presets_compact(ui, &mut backend, state);
                                });
                            });
                        });
                });
        },
    )
}

// ============================================================================
// Panel helpers — identical logic to SynthApp, but take &mut dyn SynthBackend
// ============================================================================

fn ui_oscillators_compact(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "OSCILLATORS");
    let wf_labels = ["Sin", "Tri", "Saw", "Sqr", "Nse"];

    ui.group(|ui| {
        ui.label(RichText::new("OSC 1").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
        let mut wf = b.get_param(SynthParam::Osc1Waveform) as usize;
        egui::ComboBox::from_id_source("osc1_wf").width(60.0)
            .selected_text(wf_labels[wf.min(4)])
            .show_ui(ui, |ui| {
                for (i, l) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf == i, *l).clicked() { b.set_param(SynthParam::Osc1Waveform, i as f32); }
                }
            });
        let mut v = b.get_param(SynthParam::Osc1Level);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Level")).changed() { b.set_param(SynthParam::Osc1Level, v); }
        let mut v = b.get_param(SynthParam::Osc1Phase);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Phase")).changed() { b.set_param(SynthParam::Osc1Phase, v); }
        let mut v = b.osc_pan(1);
        if ui.add(egui::Slider::new(&mut v, -1.0..=1.0).text("Pan")).changed() { b.set_osc_pan(1, v); }
    });
    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(RichText::new("OSC 2").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
        let mut wf = b.get_param(SynthParam::Osc2Waveform) as usize;
        egui::ComboBox::from_id_source("osc2_wf").width(60.0)
            .selected_text(wf_labels[wf.min(4)])
            .show_ui(ui, |ui| {
                for (i, l) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf == i, *l).clicked() { b.set_param(SynthParam::Osc2Waveform, i as f32); }
                }
            });
        let mut v = b.get_param(SynthParam::Osc2Level);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Level")).changed() { b.set_param(SynthParam::Osc2Level, v); }
        let mut v = b.get_param(SynthParam::Osc2Semitones);
        if ui.add(egui::Slider::new(&mut v, -24.0..=24.0).text("Semi")).changed() { b.set_param(SynthParam::Osc2Semitones, v); }
        let mut v = b.get_param(SynthParam::Osc2Cents);
        if ui.add(egui::Slider::new(&mut v, -100.0..=100.0).text("Cents")).changed() { b.set_param(SynthParam::Osc2Cents, v); }
        let mut v = b.get_param(SynthParam::Osc2Phase);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Phase")).changed() { b.set_param(SynthParam::Osc2Phase, v); }
        let mut v = b.osc_pan(2);
        if ui.add(egui::Slider::new(&mut v, -1.0..=1.0).text("Pan")).changed() { b.set_osc_pan(2, v); }
    });
    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(RichText::new("OSC 3").size(10.0).strong().color(Color32::from_rgb(255, 180, 60)));
        let mut wf = b.get_param(SynthParam::Osc3Waveform) as usize;
        egui::ComboBox::from_id_source("osc3_wf").width(60.0)
            .selected_text(wf_labels[wf.min(4)])
            .show_ui(ui, |ui| {
                for (i, l) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf == i, *l).clicked() { b.set_param(SynthParam::Osc3Waveform, i as f32); }
                }
            });
        let mut v = b.get_param(SynthParam::Osc3Level);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Level")).changed() { b.set_param(SynthParam::Osc3Level, v); }
        let mut v = b.get_param(SynthParam::Osc3Semitones);
        if ui.add(egui::Slider::new(&mut v, -24.0..=24.0).text("Semi")).changed() { b.set_param(SynthParam::Osc3Semitones, v); }
        let mut v = b.get_param(SynthParam::Osc3Cents);
        if ui.add(egui::Slider::new(&mut v, -100.0..=100.0).text("Cents")).changed() { b.set_param(SynthParam::Osc3Cents, v); }
        let mut v = b.get_param(SynthParam::Osc3Phase);
        if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Phase")).changed() { b.set_param(SynthParam::Osc3Phase, v); }
        let mut v = b.osc_pan(3);
        if ui.add(egui::Slider::new(&mut v, -1.0..=1.0).text("Pan")).changed() { b.set_osc_pan(3, v); }
    });
    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(RichText::new("STEREO").size(10.0).strong().color(Color32::from_rgb(100, 200, 255)));
        let mut v = b.stereo_width();
        if ui.add(egui::Slider::new(&mut v, 0.0..=2.0).text("Width")).changed() { b.set_stereo_width(v); }
    });
}

fn ui_filter_compact(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "FILTER");
    let mut v = b.get_param(SynthParam::FilterCutoff);
    if ui.add(egui::Slider::new(&mut v, 20.0..=20000.0).text("Cutoff").logarithmic(true)).changed() { b.set_param(SynthParam::FilterCutoff, v); }
    let mut v = b.get_param(SynthParam::FilterResonance);
    if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Res")).changed() { b.set_param(SynthParam::FilterResonance, v); }
    ui.add_space(8.0);
    ui.label(RichText::new("Env Amount").size(10.0));
    let mut v = b.get_param(SynthParam::FilterEnvAmount);
    if ui.add(egui::Slider::new(&mut v, -1.0..=1.0).text("Amt")).changed() { b.set_param(SynthParam::FilterEnvAmount, v); }
}

fn ui_envelopes_compact(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "AMP ENV");
    let mut v = b.get_param(SynthParam::Attack);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Attack").logarithmic(true)).changed() { b.set_param(SynthParam::Attack, v); }
    let mut v = b.get_param(SynthParam::Decay);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Decay").logarithmic(true)).changed() { b.set_param(SynthParam::Decay, v); }
    let mut v = b.get_param(SynthParam::Sustain);
    if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Sustain")).changed() { b.set_param(SynthParam::Sustain, v); }
    let mut v = b.get_param(SynthParam::Release);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Release").logarithmic(true)).changed() { b.set_param(SynthParam::Release, v); }

    ui.add_space(12.0);
    section_header(ui, "FILTER ENV");
    let mut v = b.get_param(SynthParam::FilterAttack);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Attack").logarithmic(true)).changed() { b.set_param(SynthParam::FilterAttack, v); }
    let mut v = b.get_param(SynthParam::FilterDecay);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Decay").logarithmic(true)).changed() { b.set_param(SynthParam::FilterDecay, v); }
    let mut v = b.get_param(SynthParam::FilterSustain);
    if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Sustain")).changed() { b.set_param(SynthParam::FilterSustain, v); }
    let mut v = b.get_param(SynthParam::FilterRelease);
    if ui.add(egui::Slider::new(&mut v, 0.001..=5.0).text("Release").logarithmic(true)).changed() { b.set_param(SynthParam::FilterRelease, v); }

    ui.add_space(12.0);
    section_header(ui, "MASTER");
    let mut v = b.get_param(SynthParam::PortamentoTime);
    if ui.add(egui::Slider::new(&mut v, 0.0..=3.0).text("Portamento").logarithmic(true)).changed() { b.set_param(SynthParam::PortamentoTime, v); }
    let mut v = b.get_param(SynthParam::MasterVolume);
    if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Volume")).changed() { b.set_param(SynthParam::MasterVolume, v); }
}

fn ui_lfo_compact(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "LFO");
    let mut v = b.get_param(SynthParam::LfoRate);
    if ui.add(egui::Slider::new(&mut v, 0.1..=20.0).text("Rate")).changed() { b.set_param(SynthParam::LfoRate, v); }
    let mut v = b.get_param(SynthParam::LfoDepth);
    if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Depth")).changed() { b.set_param(SynthParam::LfoDepth, v); }

    let lfo_wf_labels = ["Sin", "Tri", "Sqr", "Saw", "Rnd"];
    let mut wf = b.get_param(SynthParam::LfoWaveform) as usize;
    egui::ComboBox::from_id_source("lfo_wf").width(70.0)
        .selected_text(lfo_wf_labels[wf.min(4)])
        .show_ui(ui, |ui| {
            for (i, l) in lfo_wf_labels.iter().enumerate() {
                if ui.selectable_label(wf == i, *l).clicked() { b.set_param(SynthParam::LfoWaveform, i as f32); }
            }
        });

    let lfo_dest_labels = ["Off", "Pitch", "Filter", "Amp"];
    let mut dest = b.get_param(SynthParam::LfoDestination) as usize;
    egui::ComboBox::from_id_source("lfo_dest").width(70.0)
        .selected_text(lfo_dest_labels[dest.min(3)])
        .show_ui(ui, |ui| {
            for (i, l) in lfo_dest_labels.iter().enumerate() {
                if ui.selectable_label(dest == i, *l).clicked() { b.set_param(SynthParam::LfoDestination, i as f32); }
            }
        });

    ui.add_space(12.0);
    section_header(ui, "PITCH / VOICE");
    let mode = b.polyphony_mode();
    ui.horizontal(|ui| {
        ui.label("Mode:");
        if ui.selectable_label(mode == PolyphonyMode::Mono, "MONO").clicked() && mode != PolyphonyMode::Mono {
            b.set_polyphony_mode(PolyphonyMode::Mono);
        }
        if ui.selectable_label(mode == PolyphonyMode::Poly, "POLY").clicked() && mode != PolyphonyMode::Poly {
            b.set_polyphony_mode(PolyphonyMode::Poly);
        }
    });

    let mut v = b.get_param(SynthParam::PitchBendRange);
    if ui.add(egui::Slider::new(&mut v, 1.0..=24.0).text("Bend Range")).changed() { b.set_param(SynthParam::PitchBendRange, v); }
}

fn ui_effects_compact(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "EFFECTS");

    ui.group(|ui| {
        let mut on = b.delay_enabled();
        if ui.checkbox(&mut on, RichText::new("DELAY").size(10.0).strong()).changed() { b.set_delay_enabled(on); }
        if on {
            let mut v = b.delay_time();
            if ui.add(egui::Slider::new(&mut v, 0.05..=1.0).text("Time").suffix("s")).changed() { b.set_delay_time(v); }
            let mut v = b.delay_feedback();
            if ui.add(egui::Slider::new(&mut v, 0.0..=0.9).text("Feedback")).changed() { b.set_delay_feedback(v); }
            let mut v = b.delay_mix();
            if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Mix")).changed() { b.set_delay_mix(v); }
        }
    });
    ui.add_space(4.0);

    ui.group(|ui| {
        let mut on = b.reverb_enabled();
        if ui.checkbox(&mut on, RichText::new("REVERB").size(10.0).strong()).changed() { b.set_reverb_enabled(on); }
        if on {
            let mut v = b.reverb_room_size();
            if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Room")).changed() { b.set_reverb_room_size(v); }
            let mut v = b.reverb_damping();
            if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Damp")).changed() { b.set_reverb_damping(v); }
            let mut v = b.reverb_mix();
            if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Mix")).changed() { b.set_reverb_mix(v); }
        }
    });
    ui.add_space(4.0);

    ui.group(|ui| {
        let mut on = b.chorus_enabled();
        if ui.checkbox(&mut on, RichText::new("CHORUS").size(10.0).strong()).changed() { b.set_chorus_enabled(on); }
        if on {
            let mut v = b.chorus_rate();
            if ui.add(egui::Slider::new(&mut v, 0.1..=5.0).text("Rate").suffix("Hz")).changed() { b.set_chorus_rate(v); }
            let mut v = b.chorus_depth();
            if ui.add(egui::Slider::new(&mut v, 0.0..=10.0).text("Depth").suffix("ms")).changed() { b.set_chorus_depth(v); }
            let mut v = b.chorus_mix();
            if ui.add(egui::Slider::new(&mut v, 0.0..=1.0).text("Mix")).changed() { b.set_chorus_mix(v); }
        }
    });
}

fn apply_preset(b: &mut dyn SynthBackend, preset: &Preset) {
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

fn create_preset(b: &dyn SynthBackend, name: &str) -> Preset {
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

fn ui_presets_compact(ui: &mut Ui, b: &mut dyn SynthBackend, state: &mut EditorState) {
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
        .id_source("preset_list")
        .max_height(80.0)
        .show(ui, |ui| {
            for (i, name) in state.available_presets.iter().enumerate() {
                let sel = state.selected_preset == Some(i);
                if ui.selectable_label(sel, name).clicked() {
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

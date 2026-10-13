//! Shared GUI panels — rendered identically for standalone and plugin.
//!
//! Every panel is a free function taking `&mut dyn SynthBackend`, so the exact
//! same widget code drives both `SynthApp` (standalone) and the plugin editor.
//! This is the single source of truth for the synth UI — there is no second copy.

use egui::*;

use crate::core::event::WaveformType;
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::params::SynthParam;
use crate::core::mod_matrix::{ModDest, ModMatrix, ModSlot, ModSource, MOD_SLOTS};
use crate::core::presets::{
    list_library, load_preset, save_preset, Preset, PresetEntry, PRESET_BANKS,
};
use crate::core::voice::{LfoSettings, ModState, PolyphonyMode};
use crate::gui::backend::{MidiLearnState, SynthBackend};
use crate::gui::theme::{hangar_background, paint_chrome_title, section_header, spaced_label, THEME};
use crate::gui::widgets::{gold_value_slider, param_slider};

/// Which page is shown in the main panel (Massive-style tabbed layers).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightTab {
    /// Main synth controls (oscillators, filter, FX, …).
    #[default]
    Synth,
    /// Modulation routing matrix.
    Matrix,
    /// Preset library / manager.
    Presets,
}

/// GUI-only preset state shared by both the standalone app and the plugin editor.
pub struct PresetState {
    pub preset_name: String,
    /// Library entries: factory presets + user presets on disk.
    pub available_presets: Vec<PresetEntry>,
    pub selected_preset: Option<usize>,
    /// Active page (Synth / Matrix / Presets).
    pub right_tab: RightTab,
    /// Category written into presets saved from the GUI.
    pub category: String,
    /// Search filter for the preset list.
    pub search: String,
}

impl PresetState {
    pub fn new() -> Self {
        Self {
            preset_name: "New Preset".to_string(),
            available_presets: list_library(),
            selected_preset: None,
            right_tab: RightTab::default(),
            category: "User".to_string(),
            search: String::new(),
        }
    }

    /// Re-scan the library (e.g. after saving a new preset).
    pub fn refresh(&mut self) {
        self.available_presets = list_library();
    }
}

impl Default for PresetState {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a slider value to 2 decimal places with fixed width to prevent layout shifts
/// Uses "k" notation for values >= 1000 to keep width consistent
fn format_slider_value(value: f64, _range: std::ops::RangeInclusive<usize>) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.2}k", value / 1000.0)
    } else {
        format!("{:.2}", value)
    }
}

// ============================================================================
// Window shell (top bar + panel layout)
// ============================================================================

/// Window size shared by the standalone app and the plugin editor, sized so
/// the SYNTH page fits without scrolling (the LFO/FX column is the tallest).
pub const WINDOW_SIZE: [f32; 2] = [1120.0, 720.0];

/// How the shared shell should differ between the standalone app and the
/// plugin editor. The layout itself is identical; only these knobs change.
pub struct ShellConfig {
    /// Version string shown next to the title (e.g. "v1.2.0" or "v1.2.0 (Plugin)").
    pub version: String,
    /// Show the CPU-load meter in the top bar (standalone only).
    pub show_cpu: bool,
    /// Include the MIDI panel in the rightmost column (standalone only).
    pub show_midi_panel: bool,
    /// Tempo comes from the host transport (plugin) instead of the BPM slider.
    pub tempo_follows_host: bool,
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
    ctx.set_visuals({
        let mut v = egui::Visuals::dark();
        v.window_fill = THEME.bg_blue;
        v.panel_fill = THEME.panel_bg;
        // Radioactive controls: handles glow from a deep green interior,
        // like the stone Vic Rattlehead holds up on the cover
        v.widgets.inactive.bg_fill = THEME.toxic_green_deep;
        v.widgets.inactive.weak_bg_fill = THEME.bg_blue_light;
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, THEME.toxic_green_dark);
        v.widgets.hovered.bg_fill = THEME.toxic_green_dark;
        v.widgets.hovered.weak_bg_fill = THEME.bg_blue_light;
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, THEME.toxic_green);
        v.widgets.hovered.expansion = 1.5;
        v.widgets.active.bg_fill = THEME.toxic_green;
        v.widgets.active.weak_bg_fill = THEME.bg_blue_light;
        v.widgets.active.fg_stroke = Stroke::new(1.0, THEME.toxic_green_light);
        v.selection.bg_fill = THEME.toxic_green_dark;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, THEME.panel_border);
        v
    });

    // Monospace value numbers — fixed digit width keeps the layout stable
    ctx.style_mut(|style| style.drag_value_text_style = TextStyle::Monospace);

    // ── Top bar ─────────────────────────────────────────────────────────────
    egui::Panel::top("top_bar").show(ctx, |ui| {
        let bar_rect = ui.max_rect();
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.set_min_height(30.0);
            ui.label(
                RichText::new(&cfg.version)
                    .size(9.0)
                    .color(THEME.gold_dark),
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
                spaced_label(
                    ui,
                    "ANALOG-MODELED SUBTRACTIVE SYNTHESIZER",
                    9.0,
                    THEME.gold_dark,
                );
            });
        });

        // Centered chrome logo, painted over the reserved bar height
        paint_chrome_title(ctx, ui.painter().clone(), bar_rect.center(), "RUST IN SYNTH", 18.0);
    });

    // ── Main panel ──────────────────────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(Frame::new().fill(THEME.bg_blue))
        .show(ctx, |ui| {
            hangar_background(ui.painter(), ui.max_rect());
            // ── Tab strip: SYNTH | MATRIX | PRESETS ────────────────────────
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                for (tab, label) in [
                    (RightTab::Synth, "SYNTH"),
                    (RightTab::Matrix, "ROUTING MATRIX"),
                    (RightTab::Presets, "PRESETS"),
                ] {
                    let selected = preset_state.right_tab == tab;
                    if ui
                        .selectable_label(
                            selected,
                            RichText::new(label).size(13.0).strong(),
                        )
                        .clicked()
                    {
                        preset_state.right_tab = tab;
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_min_height(ui.available_height());
                    match preset_state.right_tab {
                        // ── SYNTH: the classic three-column layout ───────────
                        RightTab::Synth => {
                            ui.horizontal_top(|ui| {
                                // Left gutter so the panels don't hug the edge.
                                ui.add_space(16.0);
                                ui.vertical(|ui| {
                                    ui.set_width(185.0);
                                    oscillators(ui, b);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(160.0);
                                    filter2(ui, b);
                                    ui.add_space(8.0);
                                    filter(ui, b);
                                    ui.add_space(8.0);
                                    envelopes(ui, b);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(160.0);
                                    lfo(ui, b);
                                    ui.add_space(8.0);
                                    effects(ui, b);
                                });
                                ui.separator();
                                ui.vertical(|ui| {
                                    ui.set_width(200.0);
                                    pitch_voice(ui, b);
                                    ui.add_space(8.0);
                                    tempo(ui, b, cfg.tempo_follows_host);
                                    ui.add_space(8.0);
                                    master(ui, b);
                                    if cfg.show_midi_panel {
                                        ui.add_space(8.0);
                                        midi(ui, b);
                                    }
                                });
                                ui.add_space(8.0);
                            });
                        }

                        // ── MATRIX: routing matrix fills the same area ───────
                        RightTab::Matrix => {
                            // Center the page content horizontally.
                            let pad = ((ui.available_width() - 980.0) / 2.0).max(12.0);
                            ui.horizontal(|ui| {
                                ui.add_space(pad);
                                ui.vertical(|ui| {
                                    ui.set_width(980.0);
                                    mod_matrix_panel(ui, b);
                                });
                            });
                        }

                        // ── PRESETS: library + (standalone) MIDI ─────────────
                        RightTab::Presets => {
                            presets(ui, b, preset_state);
                        }
                    }
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

        param_slider(ui, b, SynthParam::Osc1Level, "Level");
        param_slider(ui, b, SynthParam::Osc1Phase, "Phase");
        param_slider(ui, b, SynthParam::Osc1Pan, "Pan");
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

        param_slider(ui, b, SynthParam::Osc2Level, "Level");
        param_slider(ui, b, SynthParam::Osc2Semitones, "Semi");
        param_slider(ui, b, SynthParam::Osc2Cents, "Cents");
        param_slider(ui, b, SynthParam::Osc2Phase, "Phase");
        param_slider(ui, b, SynthParam::Osc2Pan, "Pan");
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

        param_slider(ui, b, SynthParam::Osc3Level, "Level");
        param_slider(ui, b, SynthParam::Osc3Semitones, "Semi");
        param_slider(ui, b, SynthParam::Osc3Cents, "Cents");
        param_slider(ui, b, SynthParam::Osc3Phase, "Phase");
        param_slider(ui, b, SynthParam::Osc3Pan, "Pan");
    });

    ui.add_space(4.0);

    // OSC 4 — also feeds the routing matrix as an FM/RM source
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("OSC 4")
                    .size(10.0)
                    .strong()
                    .color(Color32::from_rgb(255, 180, 60)),
            );
            ui.label(
                RichText::new("FM/RM")
                    .size(8.0)
                    .color(THEME.text_dim),
            );
        });

        let wf4 = b.get_param(SynthParam::Osc4Waveform) as usize;
        egui::ComboBox::from_id_salt("osc4_wf")
            .width(60.0)
            .selected_text(wf_labels[wf4.min(4)])
            .show_ui(ui, |ui| {
                for (i, label) in wf_labels.iter().enumerate() {
                    if ui.selectable_label(wf4 == i, *label).clicked() {
                        b.set_param(SynthParam::Osc4Waveform, i as f32);
                    }
                }
            });

        param_slider(ui, b, SynthParam::Osc4Level, "Level");
        param_slider(ui, b, SynthParam::Osc4Semitones, "Semi");
        param_slider(ui, b, SynthParam::Osc4Cents, "Cents");
        param_slider(ui, b, SynthParam::Osc4Phase, "Phase");
        param_slider(ui, b, SynthParam::Osc4Pan, "Pan");
    });
}

// ============================================================================
// Filter
// ============================================================================

/// Second serial filter — runs after FILTER 1 in the voice chain and is
/// modulatable via the routing matrix (Cutoff 2 / Resonance 2).
pub fn filter2(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "FILTER 2");

    let ft = b.get_param(SynthParam::Filter2Type) as usize;
    egui::ComboBox::from_id_salt("filter2_type")
        .selected_text(crate::core::filter::FilterType::ALL[ft.min(2)].name())
        .show_ui(ui, |ui| {
            for (i, ty) in crate::core::filter::FilterType::ALL.iter().enumerate() {
                if ui.selectable_label(ft == i, ty.name()).clicked() {
                    b.set_param(SynthParam::Filter2Type, i as f32);
                }
            }
        });

    param_slider(ui, b, SynthParam::Filter2Cutoff, "Cutoff");
    param_slider(ui, b, SynthParam::Filter2Resonance, "Res");
}

pub fn filter(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "FILTER 1");

    // Filter topology selector (stored in ModState so it reaches the DSP)
    let mut ms = b.mod_state();
    let before = ms;
    egui::ComboBox::from_id_salt("filter_type")
        .selected_text(ms.filter_type.name())
        .show_ui(ui, |ui| {
            for ty in crate::core::filter::FilterType::ALL {
                ui.selectable_value(&mut ms.filter_type, ty, ty.name());
            }
        });
    if ms != before {
        b.set_mod_state(ms);
    }

    param_slider(ui, b, SynthParam::FilterCutoff, "Cutoff");
    param_slider(ui, b, SynthParam::FilterResonance, "Res");

    ui.add_space(8.0);
    ui.label(RichText::new("Env Amount").size(10.0));
    param_slider(ui, b, SynthParam::FilterEnvAmount, "Amt");
}

// ============================================================================
// Envelopes + Master
// ============================================================================

pub fn envelopes(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "AMP ENV");

    param_slider(ui, b, SynthParam::Attack, "Attack");
    param_slider(ui, b, SynthParam::Decay, "Decay");
    param_slider(ui, b, SynthParam::Sustain, "Sustain");
    param_slider(ui, b, SynthParam::Release, "Release");

    ui.add_space(12.0);
    section_header(ui, "FILTER ENV");

    param_slider(ui, b, SynthParam::FilterAttack, "Attack");
    param_slider(ui, b, SynthParam::FilterDecay, "Decay");
    param_slider(ui, b, SynthParam::FilterSustain, "Sustain");
    param_slider(ui, b, SynthParam::FilterRelease, "Release");
}

pub fn master(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "MASTER");

    param_slider(ui, b, SynthParam::PortamentoTime, "Portamento");
    param_slider(ui, b, SynthParam::MasterVolume, "Volume");

    ui.add_space(8.0);
    // Stereo width lives with the master controls.
    ui.group(|ui| {
        ui.label(
            RichText::new("STEREO")
                .size(10.0)
                .strong()
                .color(Color32::from_rgb(100, 200, 255)),
        );
        param_slider(ui, b, SynthParam::StereoWidth, "Width");
    });
}

/// Tempo block: BPM drives synced LFOs and the synced delay.
pub fn tempo(ui: &mut Ui, b: &mut dyn SynthBackend, follows_host: bool) {
    section_header(ui, "TEMPO");

    let mut ms = b.mod_state();
    let before = ms;
    ui.add_enabled(
        !follows_host,
        egui::Slider::new(&mut ms.bpm, 20.0..=300.0)
            .text("BPM")
            .fixed_decimals(1),
    );
    if follows_host {
        ui.label(
            RichText::new("Follows host transport")
                .size(9.0)
                .color(THEME.text_dim),
        );
    }
    if ms != before {
        b.set_mod_state(ms);
    }
}

// ============================================================================
// LFO + Pitch / Voice
// ============================================================================

pub fn lfo(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "LFO");

    // Tempo sync lives with LFO 1; when synced the Rate slider is bypassed.
    let mut ms = b.mod_state();
    let before = ms;
    ui.add_enabled_ui(!ms.lfo1.sync, |ui| {
        param_slider(ui, b, SynthParam::LfoRate, "Rate");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut ms.lfo1.sync, RichText::new("Sync").size(10.0));
        if ms.lfo1.sync {
            division_combo(ui, "lfo1_div".into(), &mut ms.lfo1.rate_beats);
        }
    });
    if ms != before {
        b.set_mod_state(ms);
    }
    param_slider(ui, b, SynthParam::LfoDepth, "Depth");

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
    // Routing lives on the ROUTING MATRIX page now; the legacy LFO1
    // destination only survives for old-preset migration, so no combo here.
}

pub fn pitch_voice(ui: &mut Ui, b: &mut dyn SynthBackend) {
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

    param_slider(ui, b, SynthParam::PitchBendRange, "Bend Range");
}

// ============================================================================
// Effects
// ============================================================================

pub fn effects(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "EFFECTS");

    // DISTORTION
    ui.group(|ui| {
        let mut dist_on = b.distortion_enabled();
        if ui.checkbox(&mut dist_on, RichText::new("DIST").size(10.0).strong()).changed() {
            b.set_distortion_enabled(dist_on);
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("Type").size(9.0).color(THEME.text_dim));
            let dt = b.get_param(SynthParam::DistortionType) as usize;
            egui::ComboBox::from_id_salt("dist_type")
                .width(74.0)
                .selected_text(crate::core::effects::DistortionType::ALL[dt.min(3)].name())
                .show_ui(ui, |ui| {
                    for (i, t) in crate::core::effects::DistortionType::ALL.iter().enumerate() {
                        if ui.selectable_label(dt == i, t.name()).clicked() {
                            b.set_param(SynthParam::DistortionType, i as f32);
                        }
                    }
                });
        });

        let mut drive = b.distortion_drive();
        if gold_value_slider(ui, egui::Slider::new(&mut drive, 0.0..=1.0).text("Drive").custom_formatter(format_slider_value)).changed() {
            b.set_distortion_drive(drive);
        }
        let mut mix = b.distortion_mix();
        if gold_value_slider(ui, egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix").custom_formatter(format_slider_value)).changed() {
            b.set_distortion_mix(mix);
        }
    });

    ui.add_space(4.0);

    // DELAY
    ui.group(|ui| {
        let mut delay_on = b.delay_enabled();
        if ui.checkbox(&mut delay_on, RichText::new("DELAY").size(10.0).strong()).changed() {
            b.set_delay_enabled(delay_on);
        }

        // Tempo sync lives with the delay; when synced Time is bypassed.
        let mut ms = b.mod_state();
        let before = ms;
        let mut time = b.delay_time();
        let time_resp = ui.add_enabled_ui(!ms.delay_sync, |ui| {
            gold_value_slider(ui, egui::Slider::new(&mut time, 0.05..=1.0).text("Time").suffix("s").custom_formatter(format_slider_value))
        });
        if time_resp.inner.changed() {
            b.set_delay_time(time);
        }
        ui.horizontal(|ui| {
            ui.checkbox(&mut ms.delay_sync, RichText::new("Sync").size(10.0));
            if ms.delay_sync {
                division_combo(ui, "delay_div".into(), &mut ms.delay_beats);
            }
        });
        if ms != before {
            b.set_mod_state(ms);
        }
        let mut feedback = b.delay_feedback();
        if gold_value_slider(ui, egui::Slider::new(&mut feedback, 0.0..=0.9).text("Fdbk").custom_formatter(format_slider_value)).changed() {
            b.set_delay_feedback(feedback);
        }
        let mut mix = b.delay_mix();
        if gold_value_slider(ui, egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix").custom_formatter(format_slider_value)).changed() {
            b.set_delay_mix(mix);
        }
    });

    ui.add_space(4.0);

    // REVERB
    ui.group(|ui| {
        let mut reverb_on = b.reverb_enabled();
        if ui.checkbox(&mut reverb_on, RichText::new("REVERB").size(10.0).strong()).changed() {
            b.set_reverb_enabled(reverb_on);
        }

        let mut room = b.reverb_room_size();
        if gold_value_slider(ui, egui::Slider::new(&mut room, 0.0..=1.0).text("Room").custom_formatter(format_slider_value)).changed() {
            b.set_reverb_room_size(room);
        }
        let mut damp = b.reverb_damping();
        if gold_value_slider(ui, egui::Slider::new(&mut damp, 0.0..=1.0).text("Damp").custom_formatter(format_slider_value)).changed() {
            b.set_reverb_damping(damp);
        }
        let mut mix = b.reverb_mix();
        if gold_value_slider(ui, egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix").custom_formatter(format_slider_value)).changed() {
            b.set_reverb_mix(mix);
        }
    });

    ui.add_space(4.0);

    // CHORUS
    ui.group(|ui| {
        let mut chorus_on = b.chorus_enabled();
        if ui.checkbox(&mut chorus_on, RichText::new("CHORUS").size(10.0).strong()).changed() {
            b.set_chorus_enabled(chorus_on);
        }

        let mut rate = b.chorus_rate();
        if gold_value_slider(ui, egui::Slider::new(&mut rate, 0.1..=5.0).text("Rate").suffix("Hz").custom_formatter(format_slider_value)).changed() {
            b.set_chorus_rate(rate);
        }
        let mut depth = b.chorus_depth();
        if gold_value_slider(ui, egui::Slider::new(&mut depth, 0.0..=10.0).text("Depth").suffix("ms").custom_formatter(format_slider_value)).changed() {
            b.set_chorus_depth(depth);
        }
        let mut mix = b.chorus_mix();
        if gold_value_slider(ui, egui::Slider::new(&mut mix, 0.0..=1.0).text("Mix").custom_formatter(format_slider_value)).changed() {
            b.set_chorus_mix(mix);
        }
    });
}

// ============================================================================
// Presets
// ============================================================================

/// Save the current sound into the selected bank. Names that collide with a
/// factory preset get a " (User)" suffix so factory refreshes never clobber them.
fn save_current(b: &mut dyn SynthBackend, state: &mut PresetState) {
    let mut name = state.preset_name.trim().to_string();
    if name.is_empty() {
        name = "New Preset".to_string();
    }
    if state
        .available_presets
        .iter()
        .any(|e| e.is_factory && e.name.eq_ignore_ascii_case(&name))
    {
        name = format!("{} (User)", name);
    }
    let preset = create_preset(b, &name, &state.category);
    match save_preset(&preset) {
        Ok(()) => {
            state.preset_name = name.clone();
            state.refresh();
            state.selected_preset = state
                .available_presets
                .iter()
                .position(|e| !e.is_factory && e.name == name);
        }
        Err(e) => eprintln!("Failed to save preset: {}", e),
    }
}

/// One themed shelf: bank title + count, then a scrollable preset list.
fn preset_shelf(
    ui: &mut Ui,
    title: &str,
    idxs: &[usize],
    state: &PresetState,
    size: Vec2,
    clicked: &mut Option<usize>,
) {
    Frame::new()
        .fill(Color32::from_black_alpha(70))
        .stroke(Stroke::new(1.0, THEME.panel_border))
        .corner_radius(4)
        .inner_margin(Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(size.x - 14.0);
            ui.set_height(size.y - 14.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(title.to_uppercase()).size(11.0).strong().color(THEME.gold));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new(idxs.len().to_string()).size(9.0).color(THEME.text_dim));
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt(("preset_shelf", title))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                        if idxs.is_empty() {
                            ui.label(RichText::new("— empty —").size(10.0).color(THEME.text_dim));
                        }
                        for &i in idxs {
                            let e = &state.available_presets[i];
                            let text = if e.is_factory {
                                RichText::new(&e.name).size(11.0)
                            } else {
                                RichText::new(format!("✎ {}", e.name))
                                    .size(11.0)
                                    .italics()
                                    .color(THEME.gold_light)
                            };
                            if ui.selectable_label(state.selected_preset == Some(i), text).clicked() {
                                *clicked = Some(i);
                            }
                        }
                    });
                });
        });
}

/// The PRESETS page: save/search toolbar above the library, which is
/// arranged as themed bank shelves (Bass, Lead, Pad, …) in four columns.
pub fn presets(ui: &mut Ui, b: &mut dyn SynthBackend, state: &mut PresetState) {
    section_header(ui, "PRESET LIBRARY");

    // ── Toolbar ─────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(RichText::new("Name").size(10.0));
        ui.add(egui::TextEdit::singleline(&mut state.preset_name).desired_width(150.0));
        ui.label(RichText::new("Bank").size(10.0));
        egui::ComboBox::from_id_salt("save_bank")
            .width(70.0)
            .selected_text(state.category.clone())
            .show_ui(ui, |ui| {
                for bank in PRESET_BANKS {
                    ui.selectable_value(&mut state.category, bank.to_string(), bank);
                }
            });
        if ui.button("💾 Save").clicked() {
            save_current(b, state);
        }
        ui.separator();
        ui.label("🔍");
        ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .hint_text("Search name or bank…")
                .desired_width(170.0),
        );
        if !state.search.is_empty() && ui.small_button("✕").clicked() {
            state.search.clear();
        }
        ui.separator();
        let factory = state.available_presets.iter().filter(|e| e.is_factory).count();
        let user = state.available_presets.len() - factory;
        ui.label(
            RichText::new(format!("{} factory · {} user", factory, user))
                .size(9.0)
                .color(THEME.text_dim),
        );
    });
    ui.add_space(6.0);

    // ── Bucket (filtered) entries into bank shelves ─────────────────────────
    let needle = state.search.to_lowercase();
    let mut shelves: Vec<(String, Vec<usize>)> =
        PRESET_BANKS.iter().map(|b| (b.to_string(), Vec::new())).collect();
    for (i, e) in state.available_presets.iter().enumerate() {
        if !needle.is_empty()
            && !e.name.to_lowercase().contains(&needle)
            && !e.category.to_lowercase().contains(&needle)
        {
            continue;
        }
        match shelves.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(&e.category)) {
            Some((_, v)) => v.push(i),
            None => shelves.push((e.category.clone(), vec![i])),
        }
    }
    for (_, v) in &mut shelves {
        v.sort_by_key(|&i| state.available_presets[i].name.to_lowercase());
    }

    // ── Shelves: 4 columns, rows sized to fill the page ─────────────────────
    const COLS: usize = 4;
    let gap = 8.0;
    let col_w = ((ui.available_width() - gap * (COLS as f32 - 1.0)) / COLS as f32).floor();
    let rows = shelves.len().div_ceil(COLS).max(1);
    let shelf_h = ((ui.available_height() - gap * (rows as f32 - 1.0)) / rows as f32).max(140.0);

    let mut clicked: Option<usize> = None;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for col in 0..COLS {
            ui.vertical(|ui| {
                ui.set_width(col_w);
                ui.spacing_mut().item_spacing.y = gap;
                for (title, idxs) in shelves.iter().skip(col).step_by(COLS) {
                    preset_shelf(ui, title, idxs, state, vec2(col_w, shelf_h), &mut clicked);
                }
            });
        }
    });

    if let Some(i) = clicked {
        let entry = state.available_presets[i].clone();
        if let Ok(preset) = load_preset(&entry.name) {
            state.selected_preset = Some(i);
            state.preset_name = preset.name.clone();
            state.category = entry.category.clone();
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

    b.set_param(SynthParam::Osc4Waveform, preset.osc4_waveform as u8 as f32);
    b.set_param(SynthParam::Osc4Level, preset.osc4_level);
    b.set_param(SynthParam::Osc4Semitones, preset.osc4_semitones as f32);
    b.set_param(SynthParam::Osc4Cents, preset.osc4_cents as f32);
    b.set_param(SynthParam::Osc4Phase, preset.osc4_phase);

    b.set_param(SynthParam::FilterCutoff, preset.filter_cutoff);
    b.set_param(SynthParam::FilterResonance, preset.filter_resonance);
    b.set_param(SynthParam::Filter2Type, preset.filter2_type.index() as f32);
    b.set_param(SynthParam::Filter2Cutoff, preset.filter2_cutoff);
    b.set_param(SynthParam::Filter2Resonance, preset.filter2_resonance);

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

    // Stereo / Pan
    b.set_osc_pan(1, preset.osc1_pan);
    b.set_osc_pan(2, preset.osc2_pan);
    b.set_osc_pan(3, preset.osc3_pan);
    b.set_osc_pan(4, preset.osc4_pan);
    b.set_stereo_width(preset.stereo_width);

    // Polyphony mode
    b.set_polyphony_mode(preset.polyphony_mode);

    // Effects - Distortion
    b.set_distortion_enabled(preset.distortion_enabled);
    b.set_distortion_drive(preset.distortion_drive);
    b.set_distortion_mix(preset.distortion_mix);
    b.set_param(SynthParam::DistortionType, preset.distortion_type.index() as f32);

    // Effects - Delay
    b.set_delay_enabled(preset.delay_enabled);
    b.set_delay_time(preset.delay_time);
    b.set_delay_feedback(preset.delay_feedback);
    b.set_delay_mix(preset.delay_mix);

    // Effects - Reverb
    b.set_reverb_enabled(preset.reverb_enabled);
    b.set_reverb_room_size(preset.reverb_room_size);
    b.set_reverb_damping(preset.reverb_damping);
    b.set_reverb_mix(preset.reverb_mix);

    // Effects - Chorus
    b.set_chorus_enabled(preset.chorus_enabled);
    b.set_chorus_rate(preset.chorus_rate);
    b.set_chorus_depth(preset.chorus_depth);
    b.set_chorus_mix(preset.chorus_mix);

    // Modulation state: migrate legacy lfo_destination into the matrix when
    // the preset has no routing stored.
    let mut matrix = preset.mod_matrix;
    if matrix.is_empty() {
        if let Some(slot) = crate::core::voice::legacy_lfo_slot(preset) {
            matrix.slots[0] = slot;
        }
    }
    b.set_mod_state(ModState {
        matrix,
        lfo1: LfoSettings {
            rate: preset.lfo_rate,
            depth: preset.lfo_depth,
            waveform: preset.lfo_waveform,
            sync: preset.lfo1_sync,
            rate_beats: preset.lfo1_beats,
        },
        lfo2: LfoSettings {
            rate: preset.lfo2_rate,
            depth: preset.lfo2_depth,
            waveform: preset.lfo2_waveform,
            sync: preset.lfo2_sync,
            rate_beats: preset.lfo2_beats,
        },
        lfo3: LfoSettings {
            rate: preset.lfo3_rate,
            depth: preset.lfo3_depth,
            waveform: preset.lfo3_waveform,
            sync: preset.lfo3_sync,
            rate_beats: preset.lfo3_beats,
        },
        env3: [
            preset.env3_attack,
            preset.env3_decay,
            preset.env3_sustain,
            preset.env3_release,
        ],
        env4: preset.env4,
        bpm: preset.tempo_bpm,
        delay_sync: preset.delay_sync,
        delay_beats: preset.delay_beats,
        filter_type: preset.filter_type,
    });
}

pub fn create_preset(b: &dyn SynthBackend, name: &str, category: &str) -> Preset {
    let ms = b.mod_state();
    Preset {
        name: name.to_string(),
        version: "1.0".to_string(),
        category: if category.trim().is_empty() {
            "User".to_string()
        } else {
            category.trim().to_string()
        },

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

        osc4_waveform: WaveformType::from_index(b.get_param(SynthParam::Osc4Waveform) as u8),
        osc4_level: b.get_param(SynthParam::Osc4Level),
        osc4_semitones: b.get_param(SynthParam::Osc4Semitones) as i8,
        osc4_cents: b.get_param(SynthParam::Osc4Cents) as i8,
        osc4_phase: b.get_param(SynthParam::Osc4Phase),

        filter_cutoff: b.get_param(SynthParam::FilterCutoff),
        filter_resonance: b.get_param(SynthParam::FilterResonance),
        filter2_cutoff: b.get_param(SynthParam::Filter2Cutoff),
        filter2_resonance: b.get_param(SynthParam::Filter2Resonance),
        filter2_type: crate::core::filter::FilterType::from_index(
            b.get_param(SynthParam::Filter2Type) as u8,
        ),

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
        lfo2_rate: ms.lfo2.rate,
        lfo2_depth: ms.lfo2.depth,
        lfo2_waveform: ms.lfo2.waveform,
        lfo3_rate: ms.lfo3.rate,
        lfo3_depth: ms.lfo3.depth,
        lfo3_waveform: ms.lfo3.waveform,
        env3_attack: ms.env3[0],
        env3_decay: ms.env3[1],
        env3_sustain: ms.env3[2],
        env3_release: ms.env3[3],
        env4: ms.env4,
        mod_matrix: ms.matrix,

        tempo_bpm: ms.bpm,
        lfo1_sync: ms.lfo1.sync,
        lfo1_beats: ms.lfo1.rate_beats,
        lfo2_sync: ms.lfo2.sync,
        lfo2_beats: ms.lfo2.rate_beats,
        lfo3_sync: ms.lfo3.sync,
        lfo3_beats: ms.lfo3.rate_beats,
        delay_sync: ms.delay_sync,
        delay_beats: ms.delay_beats,
        filter_type: ms.filter_type,

        pitch_bend_range: b.get_param(SynthParam::PitchBendRange) as u8,
        portamento_time: b.get_param(SynthParam::PortamentoTime),
        master_volume: b.get_param(SynthParam::MasterVolume),

        // Stereo / Pan
        osc1_pan: b.osc_pan(1),
        osc2_pan: b.osc_pan(2),
        osc3_pan: b.osc_pan(3),
        osc4_pan: b.osc_pan(4),
        stereo_width: b.stereo_width(),

        // Polyphony mode
        polyphony_mode: b.polyphony_mode(),

        // Effects - Distortion
        distortion_enabled: b.distortion_enabled(),
        distortion_drive: b.distortion_drive(),
        distortion_mix: b.distortion_mix(),
        distortion_type: crate::core::effects::DistortionType::from_index(
            b.get_param(SynthParam::DistortionType) as u8,
        ),

        delay_enabled: b.delay_enabled(),
        delay_time: b.delay_time(),
        delay_feedback: b.delay_feedback(),
        delay_mix: b.delay_mix(),

        // Effects - Reverb
        reverb_enabled: b.reverb_enabled(),
        reverb_room_size: b.reverb_room_size(),
        reverb_damping: b.reverb_damping(),
        reverb_mix: b.reverb_mix(),

        // Effects - Chorus
        chorus_enabled: b.chorus_enabled(),
        chorus_rate: b.chorus_rate(),
        chorus_depth: b.chorus_depth(),
        chorus_mix: b.chorus_mix(),
    }
}

// ============================================================================
// Mod Matrix + extra modulators
// ============================================================================

/// LFO waveform labels shared by all three LFO controls.
const LFO_WF_LABELS: [&str; 5] = ["Sin", "Tri", "Sqr", "Saw", "Rnd"];

/// Width of one modulator block.
const MOD_BLOCK_W: f32 = 210.0;

fn lfo_wf_index(wf: LfoWaveform) -> usize {
    match wf {
        LfoWaveform::Sine => 0,
        LfoWaveform::Triangle => 1,
        LfoWaveform::Square => 2,
        LfoWaveform::Saw => 3,
        LfoWaveform::Random => 4,
    }
}

fn lfo_wf_from_index(i: usize) -> LfoWaveform {
    match i {
        0 => LfoWaveform::Sine,
        1 => LfoWaveform::Triangle,
        2 => LfoWaveform::Square,
        3 => LfoWaveform::Saw,
        _ => LfoWaveform::Random,
    }
}

fn mod_block_title(ui: &mut Ui, title: &str) {
    ui.label(
        RichText::new(title)
            .size(10.0)
            .strong()
            .color(Color32::from_rgb(255, 180, 60)),
    );
}

/// One extra LFO block (LFO 2/3). Explicit vertical layout keeps the title
/// on its own row; controls are split into two parameter rows below it.
fn extra_lfo(ui: &mut Ui, title: &str, s: &mut LfoSettings) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.set_width(MOD_BLOCK_W);
            mod_block_title(ui, title);

            let mut wf_sel = lfo_wf_index(s.waveform);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt(format!("{}_wf", title))
                    .width(56.0)
                    .selected_text(LFO_WF_LABELS[wf_sel])
                    .show_ui(ui, |ui| {
                        for (i, label) in LFO_WF_LABELS.iter().enumerate() {
                            ui.selectable_value(&mut wf_sel, i, *label);
                        }
                    });
                ui.checkbox(&mut s.sync, RichText::new("Sync").size(10.0));
                if s.sync {
                    division_combo(ui, format!("{}_div", title), &mut s.rate_beats);
                }
            });
            s.waveform = lfo_wf_from_index(wf_sel);

            ui.horizontal(|ui| {
                ui.spacing_mut().slider_width = 55.0;
                ui.add(egui::Slider::new(&mut s.depth, 0.0..=1.0).text("Depth").fixed_decimals(2));
                ui.add_enabled(
                    !s.sync,
                    egui::Slider::new(&mut s.rate, 0.1..=20.0)
                        .text("Rate")
                        .logarithmic(true)
                        .fixed_decimals(2),
                );
            });
        });
    });
}

/// One free envelope block (ENV 3/4): title row, then A/D and S/R sliders
/// side by side on two rows.
fn extra_env(ui: &mut Ui, title: &str, adsr: &mut [f32; 4]) {
    let env_slider = |ui: &mut Ui, v: &mut f32, label: &str, level: bool| {
        let slider = if level {
            egui::Slider::new(v, 0.0..=1.0)
        } else {
            egui::Slider::new(v, 0.001..=5.0).logarithmic(true).suffix(" s")
        };
        ui.add(slider.text(label).fixed_decimals(2));
    };
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.set_width(MOD_BLOCK_W);
            ui.spacing_mut().slider_width = 60.0;
            mod_block_title(ui, title);
            ui.horizontal(|ui| {
                env_slider(ui, &mut adsr[0], "A", false);
                env_slider(ui, &mut adsr[1], "D", false);
            });
            ui.horizontal(|ui| {
                env_slider(ui, &mut adsr[2], "S", true);
                env_slider(ui, &mut adsr[3], "R", false);
            });
        });
    });
}

/// Common note divisions as (label, beats-per-cycle) pairs.
const DIVISIONS: [(&str, f32); 9] = [
    ("4 Bars", 16.0),
    ("2 Bars", 8.0),
    ("1 Bar", 4.0),
    ("1/2", 2.0),
    ("1/4", 1.0),
    ("1/8", 0.5),
    ("1/16", 0.25),
    ("1/32", 0.125),
    ("1/64", 0.0625),
];

/// Combo box selecting a note division (period in quarter-note beats).
fn division_combo(ui: &mut Ui, id_salt: String, beats: &mut f32) {
    let current = DIVISIONS
        .iter()
        .find(|(_, b)| (*b - *beats).abs() < 0.001)
        .map(|(l, _)| *l)
        .unwrap_or("1/4");
    egui::ComboBox::from_id_salt(id_salt)
        .width(58.0)
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (label, b) in DIVISIONS {
                ui.selectable_value(beats, b, label);
            }
        });
}

/// Routing grid cell size, row-label width and rotated column-footer height.
const GRID_CELL: f32 = 24.0;
const GRID_LABEL_W: f32 = 78.0;
const GRID_FOOTER_H: f32 = 84.0;

/// Thicker separator after these rows (LFOs | envelopes | performance |
/// audio-rate sources).
fn src_group_break(s: ModSource) -> bool {
    matches!(s, ModSource::Lfo3 | ModSource::Env4 | ModSource::PitchBend)
}

/// Thicker separator after these columns (pitch | filters | amp/stereo |
/// osc levels | osc pans | LFO rates | LFO depths).
fn dst_group_break(d: ModDest) -> bool {
    matches!(
        d,
        ModDest::Osc4Pitch
            | ModDest::Filter2Resonance
            | ModDest::StereoWidth
            | ModDest::Osc4Level
            | ModDest::Osc4Pan
            | ModDest::Lfo3Rate
    )
}

/// Index of the enabled slot routing `s` → `d`, if any.
fn find_route(m: &ModMatrix, s: ModSource, d: ModDest) -> Option<usize> {
    m.slots
        .iter()
        .position(|x| x.enabled && x.source == s && x.dest == d)
}

/// Source (rows) × destination (columns) routing grid. Clicking a cell
/// toggles a route (backed by a free matrix slot); hovering a lit cell pops
/// up an amount slider.
fn routing_grid(ui: &mut Ui, ms: &mut ModState) {
    let srcs: Vec<ModSource> = ModSource::all().iter().copied().filter(|s| s.is_wired()).collect();
    let dsts: Vec<ModDest> = ModDest::all().iter().copied().filter(|d| d.is_wired()).collect();
    let (rows, cols) = (srcs.len(), dsts.len());

    let size = vec2(
        GRID_LABEL_W + cols as f32 * GRID_CELL,
        GRID_FOOTER_H + rows as f32 * GRID_CELL,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    // Destinations are labelled below the grid now.
    let origin = rect.min + vec2(GRID_LABEL_W, 0.0);
    let cell_rect = |r: usize, c: usize| {
        Rect::from_min_size(
            origin + vec2(c as f32 * GRID_CELL, r as f32 * GRID_CELL),
            Vec2::splat(GRID_CELL),
        )
    };
    let grid_rect = Rect::from_min_size(origin, vec2(cols as f32 * GRID_CELL, rows as f32 * GRID_CELL));

    let grid_id = ui.id().with("routing_grid");
    let popup_rect_id = grid_id.with("popup_rect");
    let full_id = grid_id.with("full_at");
    let now = ui.input(|i| i.time);
    let mut active: Option<(usize, usize)> = ui
        .data(|d| d.get_temp::<Option<(usize, usize)>>(grid_id))
        .flatten();
    let prev_popup: Option<Rect> = ui.data(|d| d.get_temp(popup_rect_id));

    // ── Interaction ─────────────────────────────────────────────────────────
    let mut hovered: Option<(usize, usize)> = None;
    for r in 0..rows {
        for c in 0..cols {
            let resp = ui.interact(cell_rect(r, c), grid_id.with((r, c)), Sense::click());
            if resp.hovered() {
                hovered = Some((r, c));
            }
            if resp.clicked() {
                let (s, d) = (srcs[r], dsts[c]);
                match find_route(&ms.matrix, s, d) {
                    Some(i) => ms.matrix.slots[i] = ModSlot::default(),
                    None => match ms.matrix.slots.iter().position(|x| !x.enabled) {
                        Some(i) => {
                            ms.matrix.slots[i] = ModSlot {
                                source: s,
                                dest: d,
                                amount: 0.5,
                                enabled: true,
                            };
                        }
                        None => {
                            ui.data_mut(|dm| dm.insert_temp(full_id, now));
                        }
                    },
                }
            }
        }
    }

    // The amount popup stays open while the pointer is on its cell or on it.
    let pointer = ui.input(|i| i.pointer.hover_pos());
    match hovered {
        Some((r, c)) if find_route(&ms.matrix, srcs[r], dsts[c]).is_some() => active = Some((r, c)),
        Some(_) => active = None,
        None => {
            if let Some((r, c)) = active {
                let keep = ui.ctx().egui_is_using_pointer()
                    || pointer.is_some_and(|p| {
                        cell_rect(r, c).expand(2.0).contains(p)
                            || prev_popup.is_some_and(|pr| pr.expand(6.0).contains(p))
                    });
                if !keep {
                    active = None;
                }
            }
        }
    }
    if let Some((r, c)) = active {
        if find_route(&ms.matrix, srcs[r], dsts[c]).is_none() {
            active = None;
        }
    }

    // ── Painting ────────────────────────────────────────────────────────────
    {
        let painter = ui.painter();
        let focus = active.or(hovered);
        painter.rect_filled(grid_rect, 3.0, Color32::from_black_alpha(90));

        // Crosshair on the focused row/column
        if let Some((r, c)) = focus {
            let row = cell_rect(r, 0);
            let col = cell_rect(0, c);
            let tint = THEME.toxic_green.gamma_multiply(0.10);
            painter.rect_filled(
                Rect::from_min_max(pos2(rect.min.x, row.min.y), pos2(grid_rect.max.x, row.max.y)),
                0.0,
                tint,
            );
            painter.rect_filled(
                Rect::from_min_max(pos2(col.min.x, grid_rect.min.y), pos2(col.max.x, grid_rect.max.y)),
                0.0,
                tint,
            );
        }

        // Grid lines, thicker between thematic groups
        let thin = Stroke::new(1.0, Color32::from_white_alpha(14));
        let thick = Stroke::new(1.0, THEME.gold_dark.gamma_multiply(0.7));
        for c in 0..=cols {
            let x = origin.x + c as f32 * GRID_CELL;
            let stroke = if c == 0 || c == cols || dst_group_break(dsts[c - 1]) { thick } else { thin };
            painter.line_segment([pos2(x, grid_rect.min.y), pos2(x, grid_rect.max.y)], stroke);
        }
        for r in 0..=rows {
            let y = origin.y + r as f32 * GRID_CELL;
            let stroke = if r == 0 || r == rows || src_group_break(srcs[r - 1]) { thick } else { thin };
            painter.line_segment([pos2(grid_rect.min.x, y), pos2(grid_rect.max.x, y)], stroke);
        }

        // Row labels (sources)
        for (r, s) in srcs.iter().enumerate() {
            let color = if focus.map(|f| f.0) == Some(r) { THEME.gold_light } else { THEME.text_secondary };
            painter.text(
                pos2(origin.x - 6.0, cell_rect(r, 0).center().y),
                Align2::RIGHT_CENTER,
                s.name(),
                FontId::proportional(10.0),
                color,
            );
        }

        // Rotated column labels (destinations) below the grid, reading
        // bottom-to-top so they anchor at the column's bottom edge.
        for (c, d) in dsts.iter().enumerate() {
            let color = if focus.map(|f| f.1) == Some(c) { THEME.gold_light } else { THEME.text_secondary };
            let galley = painter.layout_no_wrap(d.name().to_string(), FontId::proportional(10.0), color);
            let x = cell_rect(0, c).center().x - galley.size().y / 2.0;
            painter.add(
                epaint::TextShape::new(
                    pos2(x, grid_rect.max.y + 6.0 + galley.size().x),
                    galley,
                    color,
                )
                .with_angle(-std::f32::consts::FRAC_PI_2),
            );
        }

        // Cells: lit node sized/colored by amount; faint dot when unrouted
        for r in 0..rows {
            for c in 0..cols {
                let center = cell_rect(r, c).center();
                if let Some(i) = find_route(&ms.matrix, srcs[r], dsts[c]) {
                    let amt = ms.matrix.slots[i].amount.clamp(-1.0, 1.0);
                    let color = if amt >= 0.0 { THEME.toxic_green } else { THEME.rust };
                    painter.circle_filled(center, 3.5 + 5.5 * amt.abs(), color.gamma_multiply(0.35 + 0.65 * amt.abs()));
                    painter.circle_stroke(center, 9.0, Stroke::new(1.0, color));
                } else if hovered == Some((r, c)) {
                    painter.circle_stroke(center, 9.0, Stroke::new(1.0, Color32::from_white_alpha(70)));
                } else {
                    painter.circle_filled(center, 1.5, Color32::from_white_alpha(28));
                }
            }
        }
    }

    // ── Hover amount popup ──────────────────────────────────────────────────
    let mut popup_rect = None;
    if let Some((r, c)) = active {
        if let Some(i) = find_route(&ms.matrix, srcs[r], dsts[c]) {
            let cr = cell_rect(r, c);
            // Open towards the side with room so it never covers its own cell.
            let (pivot, pos) = if c + 7 < cols {
                (Align2::LEFT_TOP, cr.right_top() + vec2(-1.0, -1.0))
            } else {
                (Align2::RIGHT_TOP, cr.left_top() + vec2(1.0, -1.0))
            };
            let area = egui::Area::new(grid_id.with("amount_popup"))
                .order(Order::Foreground)
                .pivot(pivot)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    Frame::popup(ui.style()).show(ui, |ui| {
                        ui.label(
                            RichText::new(format!("{} → {}", srcs[r].name(), dsts[c].name()))
                                .size(10.0)
                                .color(THEME.gold),
                        );
                        ui.spacing_mut().slider_width = 120.0;
                        ui.add(
                            egui::Slider::new(&mut ms.matrix.slots[i].amount, -1.0..=1.0)
                                .text("Amt")
                                .fixed_decimals(2),
                        );
                        ui.horizontal(|ui| {
                            if ui.small_button("Invert").clicked() {
                                ms.matrix.slots[i].amount = -ms.matrix.slots[i].amount;
                            }
                            if ui.small_button("✕ Remove").clicked() {
                                ms.matrix.slots[i] = ModSlot::default();
                            }
                        });
                    });
                });
            popup_rect = Some(area.response.rect);
        }
    }
    ui.data_mut(|d| {
        d.insert_temp(grid_id, active);
        match popup_rect {
            Some(r) => {
                d.insert_temp(popup_rect_id, r);
            }
            None => d.remove::<Rect>(popup_rect_id),
        }
    });

    // ── Status line ─────────────────────────────────────────────────────────
    let used = ms.matrix.slots.iter().filter(|s| s.enabled).count();
    let hidden = ms
        .matrix
        .slots
        .iter()
        .filter(|s| s.enabled && !(s.source.is_wired() && s.dest.is_wired()))
        .count();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let color = if used == MOD_SLOTS { THEME.logo_red_light } else { THEME.gold };
        ui.label(RichText::new(format!("Routes {}/{}", used, MOD_SLOTS)).size(10.0).strong().color(color));
        if hidden > 0 {
            ui.label(
                RichText::new(format!("({} legacy route(s) not shown)", hidden))
                    .size(9.0)
                    .color(THEME.text_dim),
            );
        }
        if ui.small_button("Clear all").clicked() {
            ms.matrix = ModMatrix::default();
        }
    });
    ui.label(
        RichText::new("Click a cell to route a source to a destination · hover a lit cell to set its amount")
            .size(9.0)
            .color(THEME.text_dim),
    );
    if let Some(t) = ui.data(|d| d.get_temp::<f64>(full_id)) {
        if now - t < 2.0 {
            ui.label(
                RichText::new("All routes are in use — remove one first")
                    .size(10.0)
                    .color(THEME.logo_red_light),
            );
            ui.ctx().request_repaint();
        }
    }
}

/// The ROUTING MATRIX page: the extra modulators (LFO 2/3, ENV 3/4) in a
/// 2x2 block on top, with the source × destination grid below it.
pub fn mod_matrix_panel(ui: &mut Ui, b: &mut dyn SynthBackend) {
    section_header(ui, "ROUTING MATRIX");

    let before = b.mod_state();
    let mut ms = before;

    ui.label(RichText::new("MODULATORS").size(10.0).color(THEME.gold_dark));
    egui::Grid::new("modulators_2x2")
        .num_columns(2)
        .spacing([6.0, 6.0])
        .show(ui, |ui| {
            extra_lfo(ui, "LFO 2", &mut ms.lfo2);
            extra_lfo(ui, "LFO 3", &mut ms.lfo3);
            ui.end_row();
            extra_env(ui, "ENV 3", &mut ms.env3);
            extra_env(ui, "ENV 4", &mut ms.env4);
            ui.end_row();
        });
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
    routing_grid(ui, &mut ms);

    if ms != before {
        b.set_mod_state(ms);
    }
}


//! Plugin GUI Editor Implementation
//! Uses nih_plug_egui to create the GUI window
//! Features a fixed-size 5-column layout matching the "Rust In Peace" theme

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState, widgets::ParamSlider};
use std::sync::Arc;

use crate::plugin::RustInSynthParams;
use crate::gui::theme::{THEME, panel_background, section_header};

/// Size of the editor window - fixed size to prevent layout overlap
pub const EDITOR_WIDTH: u32 = 1100;
pub const EDITOR_HEIGHT: u32 = 680;

/// Create the plugin editor
pub fn create_editor(
    egui_state: Arc<EguiState>,
    params: Arc<RustInSynthParams>,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        egui_state,
        (), // No custom state needed for simple version
        nih_plug_egui::EguiSettings::default(),
        |_state, _ctx, _queue| {}, // Update function takes 3 args
        move |ctx, setter, _state, _ui| { // Draw function takes 4 args
            // Fixed-size UI that doesn't stretch or overlap on window resize
            let fixed_width = EDITOR_WIDTH as f32;
            let fixed_height = EDITOR_HEIGHT as f32;

            // Fill the entire window with the theme's background blue
            let available = ctx.available_rect();
            let painter = ctx.layer_painter(egui::LayerId::background());
            painter.rect_filled(
                available,
                egui::CornerRadius::ZERO,
                THEME.bg_blue,
            );

            // Create a fixed-size centered container for the actual UI
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(THEME.bg_blue))
                .show(ctx, |ui| {
                    // Allocate exactly our fixed size, centered
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(fixed_width, fixed_height),
                        egui::Sense::hover(),
                    );

                    // Create a child UI with the fixed rect
                    let mut child_ui = ui.child_ui(rect, egui::Layout::default(), None);
                    let ui = &mut child_ui;

                    // Set visual style to match the standalone app
                    let mut visuals = ui.style().visuals.clone();
                    visuals.dark_mode = true;
                    visuals.window_fill = THEME.bg_blue;
                    visuals.panel_fill = THEME.panel_bg;
                    ui.ctx().set_visuals(visuals);

                    // 1. Title / Top Bar
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("🔊 Rust In Synth")
                                .size(18.0)
                                .strong()
                                .color(THEME.gold)
                        );
                        ui.label(
                            egui::RichText::new("v1.0.1 (Plugin)")
                                .size(10.0)
                                .color(egui::Color32::from_gray(120))
                        );
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new("Analog-Modeled Subtractive Synthesizer")
                                    .size(11.0)
                                    .color(egui::Color32::from_gray(180))
                            );
                        });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    // 2. 5-Column Core Interface Layout
                    ui.horizontal_top(|ui| {
                        // COLUMN 1: OSCILLATORS
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            panel_background(ui, |ui| {
                                section_header(ui, "OSCILLATORS");
                                ui.add_space(4.0);

                                // OSC 1
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("OSC 1").size(10.0).strong().color(THEME.toxic_green));
                                    ui.add(ParamSlider::for_param(&params.osc1_waveform, setter));
                                    ui.add(ParamSlider::for_param(&params.osc1_level, setter));
                                    ui.add(ParamSlider::for_param(&params.osc1_phase, setter));
                                    ui.add(ParamSlider::for_param(&params.osc1_pan, setter));
                                });

                                ui.add_space(4.0);

                                // OSC 2
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("OSC 2").size(10.0).strong().color(THEME.toxic_green));
                                    ui.add(ParamSlider::for_param(&params.osc2_waveform, setter));
                                    ui.add(ParamSlider::for_param(&params.osc2_level, setter));
                                    ui.add(ParamSlider::for_param(&params.osc2_semitones, setter));
                                    ui.add(ParamSlider::for_param(&params.osc2_cents, setter));
                                    ui.add(ParamSlider::for_param(&params.osc2_phase, setter));
                                    ui.add(ParamSlider::for_param(&params.osc2_pan, setter));
                                });

                                ui.add_space(4.0);

                                // OSC 3
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("OSC 3").size(10.0).strong().color(THEME.toxic_green));
                                    ui.add(ParamSlider::for_param(&params.osc3_waveform, setter));
                                    ui.add(ParamSlider::for_param(&params.osc3_level, setter));
                                    ui.add(ParamSlider::for_param(&params.osc3_semitones, setter));
                                    ui.add(ParamSlider::for_param(&params.osc3_cents, setter));
                                    ui.add(ParamSlider::for_param(&params.osc3_phase, setter));
                                    ui.add(ParamSlider::for_param(&params.osc3_pan, setter));
                                });
                            });
                        });

                        ui.separator();

                        // COLUMN 2: FILTER
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            panel_background(ui, |ui| {
                                section_header(ui, "FILTER");
                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("State Variable Filter").size(10.0).strong().color(THEME.logo_red_light));
                                    ui.add_space(4.0);
                                    ui.add(ParamSlider::for_param(&params.cutoff, setter));
                                    ui.add_space(2.0);
                                    ui.add(ParamSlider::for_param(&params.resonance, setter));
                                });
                            });
                        });

                        ui.separator();

                        // COLUMN 3: ENVELOPES
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            panel_background(ui, |ui| {
                                section_header(ui, "ENVELOPES");
                                ui.add_space(4.0);

                                // Amp Env
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Amplifier Envelope").size(10.0).strong().color(THEME.logo_red));
                                    ui.add(ParamSlider::for_param(&params.attack, setter));
                                    ui.add(ParamSlider::for_param(&params.decay, setter));
                                    ui.add(ParamSlider::for_param(&params.sustain, setter));
                                    ui.add(ParamSlider::for_param(&params.release, setter));
                                });

                                ui.add_space(4.0);

                                // Filter Env
                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Filter Envelope").size(10.0).strong().color(THEME.logo_red));
                                    ui.add(ParamSlider::for_param(&params.f_attack, setter));
                                    ui.add(ParamSlider::for_param(&params.f_decay, setter));
                                    ui.add(ParamSlider::for_param(&params.f_sustain, setter));
                                    ui.add(ParamSlider::for_param(&params.f_release, setter));
                                    ui.add(ParamSlider::for_param(&params.f_amount, setter));
                                });
                            });
                        });

                        ui.separator();

                        // COLUMN 4: LFO
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            panel_background(ui, |ui| {
                                section_header(ui, "LFO");
                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Low Frequency Oscillator").size(10.0).strong().color(THEME.gold));
                                    ui.add_space(4.0);
                                    ui.add(ParamSlider::for_param(&params.lfo_waveform, setter));
                                    ui.add(ParamSlider::for_param(&params.lfo_destination, setter));
                                    ui.add(ParamSlider::for_param(&params.lfo_rate, setter));
                                    ui.add(ParamSlider::for_param(&params.lfo_depth, setter));
                                });
                            });
                        });

                        ui.separator();

                        // COLUMN 5: MASTER CONTROLS
                        ui.vertical(|ui| {
                            ui.set_width(210.0);
                            panel_background(ui, |ui| {
                                section_header(ui, "MASTER");
                                ui.add_space(4.0);

                                ui.group(|ui| {
                                    ui.label(egui::RichText::new("Voice & Output").size(10.0).strong().color(THEME.steel_light));
                                    ui.add_space(4.0);
                                    ui.add(ParamSlider::for_param(&params.polyphony_mode, setter));
                                    ui.add(ParamSlider::for_param(&params.portamento, setter));
                                    ui.add(ParamSlider::for_param(&params.pitch_bend_range, setter));
                                    ui.add(ParamSlider::for_param(&params.stereo_width, setter));
                                    ui.add(ParamSlider::for_param(&params.master_volume, setter));
                                });
                            });
                        });
                    });
                });
        },
    )
}

/// Editor state that persists between editor open/close
#[derive(Default)]
pub struct EditorState {
    // Track which sections are expanded
    pub oscillators_expanded: bool,
    pub filter_expanded: bool,
    pub envelopes_expanded: bool,
    pub lfo_expanded: bool,
    pub master_expanded: bool,
}

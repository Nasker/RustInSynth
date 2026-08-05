//! Plugin GUI editor — unified with the standalone GUI via `crate::gui::panels`.
//!
//! Every frame a fresh `PluginBackend` is constructed (cheap: just Arc clones)
//! and handed to the shared `panels::*` functions — the exact same widget code
//! the standalone `SynthApp` uses. There is no duplicated panel logic here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiSettings, EguiState};

use crate::core::param_spec::cc_to_plain;
use crate::core::params::{cc_mapping_with_user_overrides, CCMapping};
use crate::core::presets::install_factory_presets;
use crate::gui::backend::SynthBackend;
use crate::gui::panels::{self, PresetState};
use crate::plugin::RustInSynthParams;
use crate::plugin_gui::backend_plugin::PluginBackend;
use crate::plugin_gui::shared_state::PluginSharedState;

/// Size of the editor window (matches standalone dimensions)
pub const EDITOR_WIDTH: u32 = 1050;
pub const EDITOR_HEIGHT: u32 = 540;

/// Persistent GUI-only state that lives across frames.
pub struct EditorState {
    presets: PresetState,
    shared: PluginSharedState,
    /// Raw MIDI CC events captured by the audio thread while the editor is
    /// open; applied here through `ParamSetter` so the host records them as
    /// automation and the GUI reflects them — exactly like the standalone,
    /// where CCs update the shared ParamBank that drives the GUI.
    cc_rx: Arc<Mutex<Receiver<(u8, u8)>>>,
    /// CC → parameter mapping (defaults + user's learned mappings)
    cc_mapping: CCMapping,
    /// Flag telling the audio thread the editor is alive (selects CC path)
    editor_open: Arc<AtomicBool>,
}

impl Drop for EditorState {
    fn drop(&mut self) {
        self.editor_open.store(false, Ordering::Relaxed);
    }
}

/// Create the plugin editor using the shared GUI panels.
///
/// `shared` is owned by the plugin and written to from the audio thread, so the
/// CPU meter (and any future shared metrics) stay live in the editor.
pub fn create_editor(
    egui_state: Arc<EguiState>,
    params: Arc<RustInSynthParams>,
    shared: PluginSharedState,
    cc_rx: Arc<Mutex<Receiver<(u8, u8)>>>,
    editor_open: Arc<AtomicBool>,
) -> Option<Box<dyn Editor>> {
    let _ = install_factory_presets();
    editor_open.store(true, Ordering::Relaxed);
    let initial_state = EditorState {
        presets: PresetState::new(),
        shared,
        cc_rx,
        cc_mapping: cc_mapping_with_user_overrides(),
        editor_open,
    };

    create_egui_editor(
        egui_state,
        initial_state,
        EguiSettings::default(),
        |_ctx, _queue, _state| {},
        move |ui, setter, _queue, state| {
            // Build a fresh backend for this frame (just Arc clones + a ref)
            let mut backend =
                PluginBackend::new(Arc::clone(&params), setter, state.shared.clone());

            // Drain queued MIDI CCs: apply each through the backend (ParamSetter)
            // exactly like a GUI edit — host automation, GUI feedback and DSP
            // sync all follow from the parameter value itself.
            if let Ok(rx) = state.cc_rx.lock() {
                let events: Vec<(u8, u8)> = rx.try_iter().take(128).collect();
                for (cc, value) in events {
                    if let Some(param) = state.cc_mapping.get_param(cc) {
                        backend.set_param(param, cc_to_plain(param, value));
                    }
                }
            }

            // Render the shared window shell. The plugin host provides its own
            // MIDI routing, so the MIDI panel is omitted here.
            let cfg = panels::ShellConfig {
                version: format!("v{} (Plugin)", env!("CARGO_PKG_VERSION")),
                show_cpu: true,
                show_midi_panel: false,
            };
            panels::shell(ui.ctx(), &mut backend, &mut state.presets, &cfg);
        },
    )
}

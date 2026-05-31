//! Plugin GUI editor — unified with the standalone GUI via `crate::gui::panels`.
//!
//! Every frame a fresh `PluginBackend` is constructed (cheap: just Arc clones)
//! and handed to the shared `panels::*` functions — the exact same widget code
//! the standalone `SynthApp` uses. There is no duplicated panel logic here.

use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiSettings, EguiState};

use crate::core::presets::install_factory_presets;
use crate::gui::panels::{self, PresetState};
use crate::plugin::RustInSynthParams;
use crate::plugin_gui::backend_plugin::PluginBackend;
use crate::plugin_gui::shared_state::PluginSharedState;

/// Size of the editor window
pub const EDITOR_WIDTH: u32 = 1100;
pub const EDITOR_HEIGHT: u32 = 680;

/// Persistent GUI-only state that lives across frames.
pub struct EditorState {
    presets: PresetState,
    shared: PluginSharedState,
}

/// Create the plugin editor using the shared GUI panels.
///
/// `shared` is owned by the plugin and written to from the audio thread, so the
/// CPU meter (and any future shared metrics) stay live in the editor.
pub fn create_editor(
    egui_state: Arc<EguiState>,
    params: Arc<RustInSynthParams>,
    shared: PluginSharedState,
) -> Option<Box<dyn Editor>> {
    let _ = install_factory_presets();
    let initial_state = EditorState {
        presets: PresetState::new(),
        shared,
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

            // Render the shared window shell. The plugin host provides its own
            // CPU/voice metering and MIDI routing, so both are omitted here.
            let cfg = panels::ShellConfig {
                version: format!("v{} (Plugin)", env!("CARGO_PKG_VERSION")),
                show_cpu: false,
                show_midi_panel: false,
            };
            panels::shell(ui.ctx(), &mut backend, &mut state.presets, &cfg);
        },
    )
}

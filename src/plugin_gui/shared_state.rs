//! Shared state for the plugin GUI ↔ audio thread communication.
//!
//! Unlike the standalone backend (which owns the audio engine directly),
//! the plugin backend communicates with its DSP through NIH-plug's
//! `ParamSetter` / `Arc<RustInSynthParams>`.  The only extra information
//! the GUI needs that NIH-plug does not provide is:
//!   - CPU load (approximated from a simple frame counter)
//!   - No MIDI feedback map needed (plugin host handles MIDI routing)

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

/// Lightweight state shared between the plugin GUI and the audio thread.
#[derive(Clone)]
pub struct PluginSharedState {
    /// CPU load as a percentage (0–100), stored as f32 bits in an atomic u32.
    cpu_load: Arc<AtomicU32>,
}

impl PluginSharedState {
    pub fn new() -> Self {
        Self {
            cpu_load: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn get_cpu_load(&self) -> f32 {
        f32::from_bits(self.cpu_load.load(Ordering::Relaxed))
    }

    pub fn set_cpu_load(&self, load: f32) {
        self.cpu_load.store(load.to_bits(), Ordering::Relaxed);
    }

    /// Returns a clone of the inner `Arc` so the audio thread can write to it.
    pub fn cpu_load_arc(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.cpu_load)
    }
}

impl Default for PluginSharedState {
    fn default() -> Self {
        Self::new()
    }
}

//! GUI module for RustSynth
//! 
//! Provides a Minimoog-style interface using egui with docking panels.
//! Uses lock-free parameter sharing for real-time audio synchronization.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub mod app;
pub mod backend;
pub mod panels;
pub mod widgets;
pub mod theme;

#[cfg(not(feature = "plugin"))]
pub mod backend_standalone;

pub use backend::SynthBackend;
#[cfg(not(feature = "plugin"))]
pub use backend_standalone::StandaloneBackend;

#[cfg(not(feature = "plugin"))]
pub use app::run_gui;

use crate::core::param_spec::{self, PARAM_SPECS};
use crate::core::params::SynthParam;

/// Number of parameters in the synth
pub const NUM_PARAMS: usize = PARAM_SPECS.len();

/// Lock-free parameter bank for audio thread communication
/// Each parameter is stored as an atomic u32 (f32 bits) for lock-free reads/writes
pub struct ParamBank {
    values: [AtomicU32; NUM_PARAMS],
}

impl ParamBank {
    /// Create a new parameter bank with the default values from
    /// `core::param_spec::PARAM_SPECS` (the single source of truth shared
    /// with the plugin parameter definitions).
    pub fn new() -> Self {
        Self {
            values: std::array::from_fn(|i| {
                let default = param_spec::param_at_index(i)
                    .map(param_spec::spec)
                    .map(|s| s.default)
                    .unwrap_or(0.0);
                AtomicU32::new(default.to_bits())
            }),
        }
    }

    /// Get a parameter value (lock-free, for audio thread)
    pub fn get(&self, param: SynthParam) -> f32 {
        f32::from_bits(self.values[param_index(param)].load(Ordering::Relaxed))
    }

    /// Set a parameter value (lock-free, for GUI thread)
    pub fn set(&self, param: SynthParam, value: f32) {
        self.values[param_index(param)].store(value.to_bits(), Ordering::Relaxed);
    }

    /// Get all parameter values as a Vec (for saving presets)
    pub fn get_all(&self) -> Vec<f32> {
        self.values.iter()
            .map(|v| f32::from_bits(v.load(Ordering::Relaxed)))
            .collect()
    }

    /// Set all parameter values from a Vec (for loading presets)
    pub fn set_all(&self, values: &[f32]) {
        for (i, &value) in values.iter().take(NUM_PARAMS).enumerate() {
            self.values[i].store(value.to_bits(), Ordering::Relaxed);
        }
    }
}

impl Default for ParamBank {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert SynthParam to array index
fn param_index(param: SynthParam) -> usize {
    param_spec::index(param)
}

/// Convert index back to SynthParam (for MIDI feedback display)
pub fn index_to_param(index: usize) -> Option<SynthParam> {
    param_spec::param_at_index(index)
}

/// Shared state between GUI and audio threads
pub struct SharedState {
    pub params: Arc<ParamBank>,
    // MIDI feedback: (CC number, value) from external controllers
    pub midi_feedback: Arc<dashmap::DashMap<u8, u8>>, // CC -> value
    // CPU load from audio thread (0.0 - 1.0, stored as u32 bits)
    pub cpu_load: Arc<std::sync::atomic::AtomicU32>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            params: Arc::new(ParamBank::new()),
            midi_feedback: Arc::new(dashmap::DashMap::new()),
            cpu_load: Arc::new(std::sync::atomic::AtomicU32::new(0)),
        }
    }
    
    /// Get CPU load as percentage (0.0 - 100.0)
    pub fn get_cpu_load(&self) -> f32 {
        let bits = self.cpu_load.load(std::sync::atomic::Ordering::Relaxed);
        f32::from_bits(bits) * 100.0
    }
    
    /// Set CPU load (0.0 - 1.0)
    pub fn set_cpu_load(&self, load: f32) {
        let bits = load.to_bits();
        self.cpu_load.store(bits, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

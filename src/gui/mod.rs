//! GUI module for RustSynth
//! 
//! Provides a Minimoog-style interface using egui with docking panels.
//! Uses lock-free parameter sharing for real-time audio synchronization.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub mod app;
pub mod widgets;
pub mod theme;

pub use app::run_gui;

use crate::core::params::SynthParam;

/// Number of parameters in the synth
pub const NUM_PARAMS: usize = 31;

/// Lock-free parameter bank for audio thread communication
/// Each parameter is stored as an atomic u32 (f32 bits) for lock-free reads/writes
pub struct ParamBank {
    values: [AtomicU32; NUM_PARAMS],
}

impl ParamBank {
    /// Create a new parameter bank with default values
    pub fn new() -> Self {
        // Initialize with defaults matching the synth's init patch
        let defaults: [f32; NUM_PARAMS] = [
            0.01_f32,  // Attack
            0.1_f32,   // Decay
            0.7_f32,   // Sustain
            0.2_f32,   // Release
            20000.0_f32, // FilterCutoff
            0.0_f32,   // FilterResonance
            0.01_f32,  // FilterAttack
            0.3_f32,   // FilterDecay
            0.0_f32,   // FilterSustain
            0.3_f32,   // FilterRelease
            0.0_f32,   // FilterEnvAmount
            6.0_f32,   // LfoRate
            0.0_f32,   // LfoDepth
            0.0_f32,   // LfoWaveform (Sine)
            0.0_f32,   // LfoDestination (Off)
            12.0_f32,  // PitchBendRange
            2.0_f32,   // Osc1Waveform (Saw)
            1.0_f32,   // Osc1Level
            0.0_f32,   // Osc1Phase
            2.0_f32,   // Osc2Waveform (Saw)
            0.8_f32,   // Osc2Level
            0.0_f32,   // Osc2Semitones
            7.0_f32,   // Osc2Cents
            0.0_f32,   // Osc2Phase
            1.0_f32,   // Osc3Waveform (Square)
            0.5_f32,   // Osc3Level
            -12.0_f32, // Osc3Semitones
            0.0_f32,   // Osc3Cents
            0.0_f32,   // Osc3Phase
            0.5_f32,   // MasterVolume
            0.0_f32,   // PortamentoTime
        ];
        
        Self {
            values: std::array::from_fn(|i| AtomicU32::new(defaults[i].to_bits())),
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
    match param {
        SynthParam::Attack => 0,
        SynthParam::Decay => 1,
        SynthParam::Sustain => 2,
        SynthParam::Release => 3,
        SynthParam::FilterCutoff => 4,
        SynthParam::FilterResonance => 5,
        SynthParam::FilterAttack => 6,
        SynthParam::FilterDecay => 7,
        SynthParam::FilterSustain => 8,
        SynthParam::FilterRelease => 9,
        SynthParam::FilterEnvAmount => 10,
        SynthParam::LfoRate => 11,
        SynthParam::LfoDepth => 12,
        SynthParam::LfoWaveform => 13,
        SynthParam::LfoDestination => 14,
        SynthParam::PitchBendRange => 15,
        SynthParam::Osc1Waveform => 16,
        SynthParam::Osc1Level => 17,
        SynthParam::Osc1Phase => 18,
        SynthParam::Osc2Waveform => 19,
        SynthParam::Osc2Level => 20,
        SynthParam::Osc2Semitones => 21,
        SynthParam::Osc2Cents => 22,
        SynthParam::Osc2Phase => 23,
        SynthParam::Osc3Waveform => 24,
        SynthParam::Osc3Level => 25,
        SynthParam::Osc3Semitones => 26,
        SynthParam::Osc3Cents => 27,
        SynthParam::Osc3Phase => 28,
        SynthParam::MasterVolume => 29,
        SynthParam::PortamentoTime => 30,
    }
}

/// Convert index back to SynthParam (for MIDI feedback display)
pub fn index_to_param(index: usize) -> Option<SynthParam> {
    match index {
        0 => Some(SynthParam::Attack),
        1 => Some(SynthParam::Decay),
        2 => Some(SynthParam::Sustain),
        3 => Some(SynthParam::Release),
        4 => Some(SynthParam::FilterCutoff),
        5 => Some(SynthParam::FilterResonance),
        6 => Some(SynthParam::FilterAttack),
        7 => Some(SynthParam::FilterDecay),
        8 => Some(SynthParam::FilterSustain),
        9 => Some(SynthParam::FilterRelease),
        10 => Some(SynthParam::FilterEnvAmount),
        11 => Some(SynthParam::LfoRate),
        12 => Some(SynthParam::LfoDepth),
        13 => Some(SynthParam::LfoWaveform),
        14 => Some(SynthParam::LfoDestination),
        15 => Some(SynthParam::PitchBendRange),
        16 => Some(SynthParam::Osc1Waveform),
        17 => Some(SynthParam::Osc1Level),
        18 => Some(SynthParam::Osc1Phase),
        19 => Some(SynthParam::Osc2Waveform),
        20 => Some(SynthParam::Osc2Level),
        21 => Some(SynthParam::Osc2Semitones),
        22 => Some(SynthParam::Osc2Cents),
        23 => Some(SynthParam::Osc2Phase),
        24 => Some(SynthParam::Osc3Waveform),
        25 => Some(SynthParam::Osc3Level),
        26 => Some(SynthParam::Osc3Semitones),
        27 => Some(SynthParam::Osc3Cents),
        28 => Some(SynthParam::Osc3Phase),
        29 => Some(SynthParam::MasterVolume),
        30 => Some(SynthParam::PortamentoTime),
        _ => None,
    }
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

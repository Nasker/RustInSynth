use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use super::event::WaveformType;
use super::lfo::LfoDestination;
use super::lfo::LfoWaveform;

/// A complete synthesizer preset/patch
/// Contains all parameters needed to recreate a sound
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub name: String,
    pub version: String, // For migration if format changes

    // Oscillator settings
    pub osc1_waveform: WaveformType,
    pub osc1_level: f32,
    pub osc1_phase: f32,

    pub osc2_waveform: WaveformType,
    pub osc2_level: f32,
    pub osc2_semitones: i8,
    pub osc2_cents: i8,
    pub osc2_phase: f32,

    pub osc3_waveform: WaveformType,
    pub osc3_level: f32,
    pub osc3_semitones: i8,
    pub osc3_cents: i8,
    pub osc3_phase: f32,

    // Filter settings
    pub filter_cutoff: f32,
    pub filter_resonance: f32,

    // Amplitude envelope
    pub amp_attack: f32,
    pub amp_decay: f32,
    pub amp_sustain: f32,
    pub amp_release: f32,

    // Filter envelope
    pub filter_attack: f32,
    pub filter_decay: f32,
    pub filter_sustain: f32,
    pub filter_release: f32,
    pub filter_env_amount: f32,

    // LFO settings
    pub lfo_rate: f32,
    pub lfo_depth: f32,
    pub lfo_waveform: LfoWaveform,
    pub lfo_destination: LfoDestination,

    // Pitch bend
    pub pitch_bend_range: u8,

    // Portamento
    #[serde(default)]
    pub portamento_time: f32,

    // Master
    pub master_volume: f32,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: "Init".to_string(),
            version: "1.0".to_string(),

            // Default: Saw lead with slight detune
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,

            osc2_waveform: WaveformType::Saw,
            osc2_level: 0.8,
            osc2_semitones: 0,
            osc2_cents: 7,
            osc2_phase: 0.0,

            osc3_waveform: WaveformType::Square,
            osc3_level: 0.5,
            osc3_semitones: -12,
            osc3_cents: 0,
            osc3_phase: 0.0,

            // Filter wide open
            filter_cutoff: 20000.0,
            filter_resonance: 0.0,

            // Medium envelope
            amp_attack: 0.01,
            amp_decay: 0.1,
            amp_sustain: 0.7,
            amp_release: 0.2,

            // Filter envelope off
            filter_attack: 0.01,
            filter_decay: 0.3,
            filter_sustain: 0.0,
            filter_release: 0.3,
            filter_env_amount: 0.0,

            // LFO off
            lfo_rate: 6.0,
            lfo_depth: 0.0,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Off,

            // Pitch bend
            pitch_bend_range: 12,

            // Portamento
            portamento_time: 0.0,

            // Master
            master_volume: 0.5,
        }
    }
}

impl Preset {
    /// Create a new preset with the given name
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Save preset to a JSON file
    pub fn save_to_file(&self, path: &Path) -> Result<(), PresetError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| PresetError::Serialization(e.to_string()))?;
        fs::write(path, json)
            .map_err(|e| PresetError::Io(e.to_string()))?;
        Ok(())
    }

    /// Load preset from a JSON file
    pub fn load_from_file(path: &Path) -> Result<Self, PresetError> {
        let json = fs::read_to_string(path)
            .map_err(|e| PresetError::Io(e.to_string()))?;
        let preset: Preset = serde_json::from_str(&json)
            .map_err(|e| PresetError::Deserialization(e.to_string()))?;
        Ok(preset)
    }

    /// Load preset from JSON string
    pub fn from_json(json: &str) -> Result<Self, PresetError> {
        serde_json::from_str(json)
            .map_err(|e| PresetError::Deserialization(e.to_string()))
    }

    /// Export preset to JSON string
    pub fn to_json(&self) -> Result<String, PresetError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| PresetError::Serialization(e.to_string()))
    }
}

/// Error type for preset operations
#[derive(Debug, Clone, PartialEq)]
pub enum PresetError {
    Io(String),
    Serialization(String),
    Deserialization(String),
}

impl std::fmt::Display for PresetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresetError::Io(msg) => write!(f, "IO error: {}", msg),
            PresetError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            PresetError::Deserialization(msg) => write!(f, "Deserialization error: {}", msg),
        }
    }
}

impl std::error::Error for PresetError {}

/// Get the default presets directory path
pub fn default_presets_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
        .join(".rustsynth")
        .join("presets")
}

/// Ensure the presets directory exists
pub fn ensure_presets_dir() -> Result<std::path::PathBuf, PresetError> {
    let dir = default_presets_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| PresetError::Io(format!("Failed to create presets dir: {}", e)))?;
    Ok(dir)
}

/// List all preset files in the presets directory
pub fn list_presets() -> Result<Vec<String>, PresetError> {
    let dir = ensure_presets_dir()?;
    let mut presets = Vec::new();

    for entry in fs::read_dir(&dir)
        .map_err(|e| PresetError::Io(format!("Failed to read presets dir: {}", e)))? {
        let entry = entry
            .map_err(|e| PresetError::Io(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "json") {
            if let Some(stem) = path.file_stem() {
                presets.push(stem.to_string_lossy().to_string());
            }
        }
    }

    presets.sort();
    Ok(presets)
}

/// Load a preset by name (from default location)
pub fn load_preset(name: &str) -> Result<Preset, PresetError> {
    let dir = default_presets_dir();
    let path = dir.join(format!("{}.json", name));
    Preset::load_from_file(&path)
}

/// Save a preset by name (to default location)
pub fn save_preset(preset: &Preset) -> Result<(), PresetError> {
    let dir = ensure_presets_dir()?;
    let path = dir.join(format!("{}.json", preset.name));
    preset.save_to_file(&path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_preset_default() {
        let preset = Preset::default();
        assert_eq!(preset.name, "Init");
        assert_eq!(preset.osc1_waveform, WaveformType::Saw);
        assert_eq!(preset.amp_attack, 0.01);
    }

    #[test]
    fn test_preset_serialize_deserialize() {
        let preset = Preset::new("Test Patch");
        let json = preset.to_json().unwrap();
        let loaded = Preset::from_json(&json).unwrap();
        assert_eq!(preset, loaded);
    }

    #[test]
    fn test_preset_save_load_file() {
        let preset = Preset::new("File Test");

        // Create a temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Save
        preset.save_to_file(&path).unwrap();

        // Load
        let loaded = Preset::load_from_file(&path).unwrap();
        assert_eq!(preset, loaded);
    }

    #[test]
    fn test_preset_version() {
        let preset = Preset::default();
        assert_eq!(preset.version, "1.0");
    }
}

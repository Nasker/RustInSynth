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

/// Create factory presets
pub fn factory_presets() -> Vec<Preset> {
    vec![
        // Classic sub bass - deep and powerful
        Preset {
            name: "Sub Bass".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Sine,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Square,
            osc2_level: 0.3,
            osc2_semitones: 0,
            osc2_cents: 0,
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Sine,
            osc3_level: 0.8,
            osc3_semitones: -12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 800.0,
            filter_resonance: 0.2,
            amp_attack: 0.005,
            amp_decay: 0.2,
            amp_sustain: 0.9,
            amp_release: 0.15,
            filter_attack: 0.01,
            filter_decay: 0.3,
            filter_sustain: 0.3,
            filter_release: 0.2,
            filter_env_amount: 0.4,
            lfo_rate: 0.5,
            lfo_depth: 0.0,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Off,
            pitch_bend_range: 2,
            portamento_time: 0.05,
            master_volume: 0.6,
        },
        
        // Bright lead with portamento
        Preset {
            name: "Glide Lead".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Saw,
            osc2_level: 0.9,
            osc2_semitones: 0,
            osc2_cents: 12,
            osc2_phase: 0.25,
            osc3_waveform: WaveformType::Square,
            osc3_level: 0.4,
            osc3_semitones: 12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 3500.0,
            filter_resonance: 0.35,
            amp_attack: 0.01,
            amp_decay: 0.15,
            amp_sustain: 0.8,
            amp_release: 0.25,
            filter_attack: 0.01,
            filter_decay: 0.4,
            filter_sustain: 0.4,
            filter_release: 0.3,
            filter_env_amount: 0.6,
            lfo_rate: 5.5,
            lfo_depth: 0.15,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Pitch,
            pitch_bend_range: 12,
            portamento_time: 0.12,
            master_volume: 0.5,
        },
        
        // Lush pad with slow attack
        Preset {
            name: "Warm Pad".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Saw,
            osc1_level: 0.7,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Saw,
            osc2_level: 0.7,
            osc2_semitones: 0,
            osc2_cents: -8,
            osc2_phase: 0.33,
            osc3_waveform: WaveformType::Triangle,
            osc3_level: 0.5,
            osc3_semitones: -12,
            osc3_cents: 5,
            osc3_phase: 0.66,
            filter_cutoff: 2000.0,
            filter_resonance: 0.15,
            amp_attack: 0.8,
            amp_decay: 0.5,
            amp_sustain: 0.7,
            amp_release: 1.2,
            filter_attack: 1.0,
            filter_decay: 0.8,
            filter_sustain: 0.5,
            filter_release: 1.5,
            filter_env_amount: 0.3,
            lfo_rate: 0.8,
            lfo_depth: 0.1,
            lfo_waveform: LfoWaveform::Triangle,
            lfo_destination: LfoDestination::FilterCutoff,
            pitch_bend_range: 7,
            portamento_time: 0.0,
            master_volume: 0.45,
        },
        
        // Punchy pluck
        Preset {
            name: "Pluck".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Square,
            osc2_level: 0.6,
            osc2_semitones: 0,
            osc2_cents: 3,
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Triangle,
            osc3_level: 0.3,
            osc3_semitones: 12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 5000.0,
            filter_resonance: 0.4,
            amp_attack: 0.001,
            amp_decay: 0.3,
            amp_sustain: 0.0,
            amp_release: 0.2,
            filter_attack: 0.001,
            filter_decay: 0.25,
            filter_sustain: 0.1,
            filter_release: 0.15,
            filter_env_amount: 0.8,
            lfo_rate: 6.0,
            lfo_depth: 0.0,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Off,
            pitch_bend_range: 12,
            portamento_time: 0.0,
            master_volume: 0.5,
        },
        
        // Noise sweep FX
        Preset {
            name: "Sweep FX".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Noise,
            osc1_level: 0.8,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Saw,
            osc2_level: 0.4,
            osc2_semitones: 0,
            osc2_cents: 0,
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Square,
            osc3_level: 0.3,
            osc3_semitones: 7,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 500.0,
            filter_resonance: 0.7,
            amp_attack: 0.5,
            amp_decay: 1.0,
            amp_sustain: 0.3,
            amp_release: 2.0,
            filter_attack: 0.8,
            filter_decay: 1.5,
            filter_sustain: 0.2,
            filter_release: 2.5,
            filter_env_amount: 0.9,
            lfo_rate: 0.3,
            lfo_depth: 0.5,
            lfo_waveform: LfoWaveform::Triangle,
            lfo_destination: LfoDestination::FilterCutoff,
            pitch_bend_range: 24,
            portamento_time: 0.0,
            master_volume: 0.4,
        },
        
        // Classic Minimoog-style bass
        Preset {
            name: "Moog Bass".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Square,
            osc2_level: 0.7,
            osc2_semitones: 0,
            osc2_cents: -5,
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Saw,
            osc3_level: 0.6,
            osc3_semitones: -12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 600.0,
            filter_resonance: 0.5,
            amp_attack: 0.002,
            amp_decay: 0.4,
            amp_sustain: 0.6,
            amp_release: 0.1,
            filter_attack: 0.001,
            filter_decay: 0.35,
            filter_sustain: 0.2,
            filter_release: 0.1,
            filter_env_amount: 0.7,
            lfo_rate: 6.0,
            lfo_depth: 0.0,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Off,
            pitch_bend_range: 2,
            portamento_time: 0.03,
            master_volume: 0.55,
        },
        
        // Wobble bass (dubstep style)
        Preset {
            name: "Wobble".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Saw,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Square,
            osc2_level: 0.8,
            osc2_semitones: 0,
            osc2_cents: 0,
            osc2_phase: 0.5,
            osc3_waveform: WaveformType::Saw,
            osc3_level: 0.7,
            osc3_semitones: -12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 400.0,
            filter_resonance: 0.6,
            amp_attack: 0.01,
            amp_decay: 0.1,
            amp_sustain: 0.9,
            amp_release: 0.15,
            filter_attack: 0.01,
            filter_decay: 0.1,
            filter_sustain: 0.8,
            filter_release: 0.1,
            filter_env_amount: 0.2,
            lfo_rate: 4.0,
            lfo_depth: 0.9,
            lfo_waveform: LfoWaveform::Square,
            lfo_destination: LfoDestination::FilterCutoff,
            pitch_bend_range: 2,
            portamento_time: 0.0,
            master_volume: 0.5,
        },
        
        // Soft keys / electric piano style
        Preset {
            name: "Soft Keys".to_string(),
            version: "1.0".to_string(),
            osc1_waveform: WaveformType::Sine,
            osc1_level: 1.0,
            osc1_phase: 0.0,
            osc2_waveform: WaveformType::Triangle,
            osc2_level: 0.5,
            osc2_semitones: 0,
            osc2_cents: 2,
            osc2_phase: 0.0,
            osc3_waveform: WaveformType::Sine,
            osc3_level: 0.3,
            osc3_semitones: 12,
            osc3_cents: 0,
            osc3_phase: 0.0,
            filter_cutoff: 4000.0,
            filter_resonance: 0.1,
            amp_attack: 0.005,
            amp_decay: 0.8,
            amp_sustain: 0.4,
            amp_release: 0.5,
            filter_attack: 0.001,
            filter_decay: 0.5,
            filter_sustain: 0.3,
            filter_release: 0.4,
            filter_env_amount: 0.4,
            lfo_rate: 5.0,
            lfo_depth: 0.05,
            lfo_waveform: LfoWaveform::Sine,
            lfo_destination: LfoDestination::Amplitude,
            pitch_bend_range: 2,
            portamento_time: 0.0,
            master_volume: 0.5,
        },
    ]
}

/// Install factory presets if they don't exist
pub fn install_factory_presets() -> Result<usize, PresetError> {
    let dir = ensure_presets_dir()?;
    let mut installed = 0;
    
    for preset in factory_presets() {
        let path = dir.join(format!("{}.json", preset.name));
        if !path.exists() {
            preset.save_to_file(&path)?;
            installed += 1;
        }
    }
    
    Ok(installed)
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

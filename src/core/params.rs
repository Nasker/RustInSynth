use std::collections::HashMap;

/// Synth parameters that can be controlled via MIDI CC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SynthParam {
    Waveform,
    Attack,
    Release,
    FilterCutoff,
    FilterResonance,
}

impl SynthParam {
    /// Get display name for the parameter
    pub fn name(&self) -> &'static str {
        match self {
            SynthParam::Waveform => "Waveform",
            SynthParam::Attack => "Attack",
            SynthParam::Release => "Release",
            SynthParam::FilterCutoff => "Filter Cutoff",
            SynthParam::FilterResonance => "Filter Resonance",
        }
    }

    /// Get all available parameters
    pub fn all() -> &'static [SynthParam] {
        &[
            SynthParam::Waveform,
            SynthParam::Attack,
            SynthParam::Release,
            SynthParam::FilterCutoff,
            SynthParam::FilterResonance,
        ]
    }
}

/// Maps MIDI CC numbers to synth parameters
#[derive(Debug, Clone)]
pub struct CCMapping {
    cc_to_param: HashMap<u8, SynthParam>,
    param_to_cc: HashMap<SynthParam, u8>,
}

impl CCMapping {
    /// Create a new empty CC mapping
    pub fn new() -> Self {
        Self {
            cc_to_param: HashMap::new(),
            param_to_cc: HashMap::new(),
        }
    }

    /// Create default CC mappings based on MIDI standard Sound Controllers
    pub fn default_mappings() -> Self {
        let mut mapping = Self::new();
        // MIDI Sound Controller conventions:
        // CC 75 = Sound Controller 1 (often waveform/variation)
        // CC 71 = Sound Controller 2 (often resonance)
        // CC 72 = Sound Controller 3 (often release time)
        // CC 73 = Sound Controller 4 (often attack time)
        // CC 74 = Sound Controller 5 (often cutoff/brightness)
        mapping.map(75, SynthParam::Waveform);
        mapping.map(73, SynthParam::Attack);
        mapping.map(72, SynthParam::Release);
        mapping.map(74, SynthParam::FilterCutoff);
        mapping.map(71, SynthParam::FilterResonance);
        mapping
    }

    /// Map a CC number to a parameter
    pub fn map(&mut self, cc: u8, param: SynthParam) {
        // Remove old mapping for this CC if exists
        if let Some(old_param) = self.cc_to_param.remove(&cc) {
            self.param_to_cc.remove(&old_param);
        }
        // Remove old CC for this param if exists
        if let Some(old_cc) = self.param_to_cc.remove(&param) {
            self.cc_to_param.remove(&old_cc);
        }
        // Add new mapping
        self.cc_to_param.insert(cc, param);
        self.param_to_cc.insert(param, cc);
    }

    /// Unmap a CC number
    pub fn unmap_cc(&mut self, cc: u8) {
        if let Some(param) = self.cc_to_param.remove(&cc) {
            self.param_to_cc.remove(&param);
        }
    }

    /// Get the parameter mapped to a CC number
    pub fn get_param(&self, cc: u8) -> Option<SynthParam> {
        self.cc_to_param.get(&cc).copied()
    }

    /// Get the CC number mapped to a parameter
    pub fn get_cc(&self, param: SynthParam) -> Option<u8> {
        self.param_to_cc.get(&param).copied()
    }

    /// List all current mappings
    pub fn list_mappings(&self) -> Vec<(u8, SynthParam)> {
        let mut mappings: Vec<_> = self.cc_to_param.iter()
            .map(|(&cc, &param)| (cc, param))
            .collect();
        mappings.sort_by_key(|(cc, _)| *cc);
        mappings
    }
}

impl Default for CCMapping {
    fn default() -> Self {
        Self::default_mappings()
    }
}

/// Converts a MIDI CC value (0-127) to a time value in seconds
/// Uses an exponential curve for more musical feel
pub fn cc_to_time(value: u8, min_time: f32, max_time: f32) -> f32 {
    let normalized = value as f32 / 127.0;
    // Exponential curve: more resolution at lower values
    let curved = normalized * normalized;
    min_time + curved * (max_time - min_time)
}

/// Converts a time value back to CC (0-127)
pub fn time_to_cc(time: f32, min_time: f32, max_time: f32) -> u8 {
    let normalized = ((time - min_time) / (max_time - min_time)).clamp(0.0, 1.0);
    let curved = normalized.sqrt();
    (curved * 127.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cc_mapping() {
        let mut mapping = CCMapping::new();
        mapping.map(73, SynthParam::Attack);
        
        assert_eq!(mapping.get_param(73), Some(SynthParam::Attack));
        assert_eq!(mapping.get_cc(SynthParam::Attack), Some(73));
    }

    #[test]
    fn test_cc_to_time() {
        // CC 0 should give min time
        assert!((cc_to_time(0, 0.001, 2.0) - 0.001).abs() < 0.001);
        // CC 127 should give max time
        assert!((cc_to_time(127, 0.001, 2.0) - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_default_mappings() {
        let mapping = CCMapping::default_mappings();
        assert_eq!(mapping.get_param(75), Some(SynthParam::Waveform));
        assert_eq!(mapping.get_param(72), Some(SynthParam::Release));
        assert_eq!(mapping.get_param(73), Some(SynthParam::Attack));
        assert_eq!(mapping.get_param(74), Some(SynthParam::FilterCutoff));
        assert_eq!(mapping.get_param(71), Some(SynthParam::FilterResonance));
    }
}

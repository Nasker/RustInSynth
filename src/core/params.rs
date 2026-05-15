use std::collections::HashMap;

/// Synth parameters that can be controlled via MIDI CC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SynthParam {
    // Envelope (ADSR)
    Attack,
    Decay,
    Sustain,
    Release,
    
    // Filter
    FilterCutoff,
    FilterResonance,
    
    // Pitch
    PitchBendRange,
    
    // Oscillator 1 (Main)
    Osc1Waveform,
    Osc1Level,
    Osc1Phase,
    
    // Oscillator 2
    Osc2Waveform,
    Osc2Level,
    Osc2Semitones,
    Osc2Cents,
    Osc2Phase,
    
    // Oscillator 3
    Osc3Waveform,
    Osc3Level,
    Osc3Semitones,
    Osc3Cents,
    Osc3Phase,
}

impl SynthParam {
    /// Get display name for the parameter
    pub fn name(&self) -> &'static str {
        match self {
            SynthParam::Attack => "Attack",
            SynthParam::Decay => "Decay",
            SynthParam::Sustain => "Sustain",
            SynthParam::Release => "Release",
            SynthParam::FilterCutoff => "Filter Cutoff",
            SynthParam::FilterResonance => "Filter Resonance",
            SynthParam::PitchBendRange => "Pitch Bend Range",
            SynthParam::Osc1Waveform => "OSC1 Waveform",
            SynthParam::Osc1Level => "OSC1 Level",
            SynthParam::Osc1Phase => "OSC1 Phase",
            SynthParam::Osc2Waveform => "OSC2 Waveform",
            SynthParam::Osc2Level => "OSC2 Level",
            SynthParam::Osc2Semitones => "OSC2 Semitones",
            SynthParam::Osc2Cents => "OSC2 Cents",
            SynthParam::Osc2Phase => "OSC2 Phase",
            SynthParam::Osc3Waveform => "OSC3 Waveform",
            SynthParam::Osc3Level => "OSC3 Level",
            SynthParam::Osc3Semitones => "OSC3 Semitones",
            SynthParam::Osc3Cents => "OSC3 Cents",
            SynthParam::Osc3Phase => "OSC3 Phase",
        }
    }

    /// Get short name for compact display
    pub fn short_name(&self) -> &'static str {
        match self {
            SynthParam::Attack => "ATK",
            SynthParam::Decay => "DEC",
            SynthParam::Sustain => "SUS",
            SynthParam::Release => "REL",
            SynthParam::FilterCutoff => "CUT",
            SynthParam::FilterResonance => "RES",
            SynthParam::PitchBendRange => "PBR",
            SynthParam::Osc1Waveform => "O1W",
            SynthParam::Osc1Level => "O1L",
            SynthParam::Osc1Phase => "O1P",
            SynthParam::Osc2Waveform => "O2W",
            SynthParam::Osc2Level => "O2L",
            SynthParam::Osc2Semitones => "O2S",
            SynthParam::Osc2Cents => "O2C",
            SynthParam::Osc2Phase => "O2P",
            SynthParam::Osc3Waveform => "O3W",
            SynthParam::Osc3Level => "O3L",
            SynthParam::Osc3Semitones => "O3S",
            SynthParam::Osc3Cents => "O3C",
            SynthParam::Osc3Phase => "O3P",
        }
    }

    /// Get all available parameters
    pub fn all() -> &'static [SynthParam] {
        &[
            SynthParam::Attack,
            SynthParam::Decay,
            SynthParam::Sustain,
            SynthParam::Release,
            SynthParam::FilterCutoff,
            SynthParam::FilterResonance,
            SynthParam::PitchBendRange,
            SynthParam::Osc1Waveform,
            SynthParam::Osc1Level,
            SynthParam::Osc1Phase,
            SynthParam::Osc2Waveform,
            SynthParam::Osc2Level,
            SynthParam::Osc2Semitones,
            SynthParam::Osc2Cents,
            SynthParam::Osc2Phase,
            SynthParam::Osc3Waveform,
            SynthParam::Osc3Level,
            SynthParam::Osc3Semitones,
            SynthParam::Osc3Cents,
            SynthParam::Osc3Phase,
        ]
    }
}

// ============================================================================
// CC Registry - Standard MIDI CC definitions and default assignments
// ============================================================================

/// Standard MIDI CC numbers with their conventional meanings
/// This serves as documentation and a reference for CC assignments
pub mod cc {
    // Standard MIDI CCs (0-31 are MSB, 32-63 are LSB for 14-bit)
    pub const MODULATION: u8 = 1;
    pub const BREATH: u8 = 2;
    pub const FOOT: u8 = 4;
    pub const PORTAMENTO_TIME: u8 = 5;
    pub const VOLUME: u8 = 7;
    pub const BALANCE: u8 = 8;
    pub const PAN: u8 = 10;
    pub const EXPRESSION: u8 = 11;
    
    // Sound Controllers (70-79) - commonly used for synth parameters
    pub const SOUND_CTRL_1: u8 = 70;   // Sound Variation
    pub const SOUND_CTRL_2: u8 = 71;   // Timbre/Resonance
    pub const SOUND_CTRL_3: u8 = 72;   // Release Time
    pub const SOUND_CTRL_4: u8 = 73;   // Attack Time
    pub const SOUND_CTRL_5: u8 = 74;   // Brightness/Cutoff
    pub const SOUND_CTRL_6: u8 = 75;   // Decay Time
    pub const SOUND_CTRL_7: u8 = 76;   // Vibrato Rate
    pub const SOUND_CTRL_8: u8 = 77;   // Vibrato Depth
    pub const SOUND_CTRL_9: u8 = 78;   // Vibrato Delay
    pub const SOUND_CTRL_10: u8 = 79;  // Undefined
    
    // General Purpose Controllers (80-83)
    pub const GP_CTRL_5: u8 = 80;
    pub const GP_CTRL_6: u8 = 81;
    pub const GP_CTRL_7: u8 = 82;
    pub const GP_CTRL_8: u8 = 83;
    
    // Portamento Control (84)
    pub const PORTAMENTO_CTRL: u8 = 84;
    
    // Undefined CCs good for custom use (85-87, 89-90, 102-119)
    pub const UNDEFINED_85: u8 = 85;
    pub const UNDEFINED_86: u8 = 86;
    pub const UNDEFINED_87: u8 = 87;
    pub const UNDEFINED_89: u8 = 89;
    pub const UNDEFINED_90: u8 = 90;
    pub const UNDEFINED_102: u8 = 102;
    
    // Effects (91-95)
    pub const REVERB: u8 = 91;
    pub const TREMOLO: u8 = 92;
    pub const CHORUS: u8 = 93;
    pub const DETUNE: u8 = 94;
    pub const PHASER: u8 = 95;
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
    /// 
    /// Default layout:
    /// - CC 71: Filter Resonance (standard)
    /// - CC 72: Release Time (standard)
    /// - CC 73: Attack Time (standard)
    /// - CC 74: Filter Cutoff (standard)
    /// - CC 75-77: OSC1/2/3 Waveform
    /// - CC 78-79: OSC2/3 Semitones
    /// - CC 80-82: OSC1/2/3 Level
    /// - CC 83: Decay Time
    /// - CC 84: Sustain Level
    /// - CC 85-86: OSC2/3 Cents (fine detune)
    /// - CC 87,89,90: OSC1/2/3 Phase
    pub fn default_mappings() -> Self {
        let mut mapping = Self::new();
        
        // ADSR Envelope - using standard Sound Controllers
        mapping.map(cc::SOUND_CTRL_4, SynthParam::Attack);       // CC 73
        mapping.map(cc::GP_CTRL_8, SynthParam::Decay);           // CC 83
        mapping.map(cc::PORTAMENTO_CTRL, SynthParam::Sustain);   // CC 84
        mapping.map(cc::SOUND_CTRL_3, SynthParam::Release);      // CC 72
        
        // Filter - using standard Sound Controllers
        mapping.map(cc::SOUND_CTRL_5, SynthParam::FilterCutoff);     // CC 74 (Brightness)
        mapping.map(cc::SOUND_CTRL_2, SynthParam::FilterResonance);  // CC 71 (Timbre)
        
        // Oscillator Waveforms - Sound Controllers 6-8
        mapping.map(cc::SOUND_CTRL_6, SynthParam::Osc1Waveform);  // CC 75
        mapping.map(cc::SOUND_CTRL_7, SynthParam::Osc2Waveform);  // CC 76
        mapping.map(cc::SOUND_CTRL_8, SynthParam::Osc3Waveform);  // CC 77
        
        // Oscillator Semitones - Sound Controllers 9-10
        mapping.map(cc::SOUND_CTRL_9, SynthParam::Osc2Semitones);  // CC 78
        mapping.map(cc::SOUND_CTRL_10, SynthParam::Osc3Semitones); // CC 79
        
        // Oscillator Levels - General Purpose Controllers
        mapping.map(cc::GP_CTRL_5, SynthParam::Osc1Level);  // CC 80
        mapping.map(cc::GP_CTRL_6, SynthParam::Osc2Level);  // CC 81
        mapping.map(cc::GP_CTRL_7, SynthParam::Osc3Level);  // CC 82
        
        // Oscillator Fine Detune (Cents) - Undefined CCs
        mapping.map(cc::UNDEFINED_85, SynthParam::Osc2Cents);  // CC 85
        mapping.map(cc::UNDEFINED_86, SynthParam::Osc3Cents);  // CC 86
        
        // Oscillator Phase - Undefined CCs
        mapping.map(cc::UNDEFINED_87, SynthParam::Osc1Phase);  // CC 87
        mapping.map(cc::UNDEFINED_89, SynthParam::Osc2Phase);  // CC 89
        mapping.map(cc::UNDEFINED_90, SynthParam::Osc3Phase);  // CC 90
        
        // Pitch Bend Range - RPN would be standard but we use CC for simplicity
        mapping.map(cc::UNDEFINED_102, SynthParam::PitchBendRange);  // CC 102
        
        mapping
    }
    
    /// Print a formatted table of all CC mappings
    pub fn print_mappings(&self) {
        println!("┌─────┬────────────────────┐");
        println!("│ CC  │ Parameter          │");
        println!("├─────┼────────────────────┤");
        for (cc, param) in self.list_mappings() {
            println!("│ {:>3} │ {:<18} │", cc, param.name());
        }
        println!("└─────┴────────────────────┘");
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

// ============================================================================
// CC Value Conversion Functions
// ============================================================================

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

/// Converts CC (0-127) to level (0.0-1.0)
pub fn cc_to_level(value: u8) -> f32 {
    value as f32 / 127.0
}

/// Converts CC (0-127) to semitones (-24 to +24)
/// CC 0 = -24, CC 64 = 0, CC 127 = +24
pub fn cc_to_semitones(value: u8) -> i8 {
    ((value as i16 - 64) * 24 / 63).clamp(-24, 24) as i8
}

/// Converts CC (0-127) to cents (-100 to +100)
/// CC 0 = -100, CC 64 = 0, CC 127 = +100
pub fn cc_to_cents(value: u8) -> i8 {
    ((value as i16 - 64) * 100 / 63).clamp(-100, 100) as i8
}

/// Converts CC (0-127) to waveform index (0-3)
/// Divides the range into 4 equal parts
pub fn cc_to_waveform(value: u8) -> u8 {
    (value / 32).min(3)
}

/// Converts CC (0-127) to phase offset (0.0-1.0)
/// Full range: 0 = 0°, 127 = 360°
pub fn cc_to_phase(value: u8) -> f32 {
    value as f32 / 127.0
}

/// Converts CC (0-127) to sustain level (0.0-1.0)
pub fn cc_to_sustain(value: u8) -> f32 {
    value as f32 / 127.0
}

/// Converts CC (0-127) to pitch bend range in semitones (1-24)
/// CC 0-5 = 1, CC 6-10 = 2, ..., CC 122-127 = 24
pub fn cc_to_pitch_bend_range(value: u8) -> u8 {
    ((value as u16 * 24 / 127) + 1).min(24) as u8
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
        // Envelope
        assert_eq!(mapping.get_param(72), Some(SynthParam::Release));
        assert_eq!(mapping.get_param(73), Some(SynthParam::Attack));
        // Filter
        assert_eq!(mapping.get_param(74), Some(SynthParam::FilterCutoff));
        assert_eq!(mapping.get_param(71), Some(SynthParam::FilterResonance));
        // Oscillator waveforms
        assert_eq!(mapping.get_param(75), Some(SynthParam::Osc1Waveform));
        assert_eq!(mapping.get_param(76), Some(SynthParam::Osc2Waveform));
        assert_eq!(mapping.get_param(77), Some(SynthParam::Osc3Waveform));
        // Oscillator levels
        assert_eq!(mapping.get_param(80), Some(SynthParam::Osc1Level));
        assert_eq!(mapping.get_param(81), Some(SynthParam::Osc2Level));
        assert_eq!(mapping.get_param(82), Some(SynthParam::Osc3Level));
    }

    #[test]
    fn test_cc_to_semitones() {
        assert_eq!(cc_to_semitones(0), -24);
        assert_eq!(cc_to_semitones(64), 0);
        assert_eq!(cc_to_semitones(127), 24);
    }

    #[test]
    fn test_cc_to_cents() {
        assert_eq!(cc_to_cents(0), -100);
        assert_eq!(cc_to_cents(64), 0);
        assert_eq!(cc_to_cents(127), 100);
    }

    #[test]
    fn test_cc_to_waveform() {
        assert_eq!(cc_to_waveform(0), 0);    // Sine
        assert_eq!(cc_to_waveform(31), 0);   // Sine
        assert_eq!(cc_to_waveform(32), 1);   // Square
        assert_eq!(cc_to_waveform(64), 2);   // Saw
        assert_eq!(cc_to_waveform(96), 3);   // Triangle
        assert_eq!(cc_to_waveform(127), 3);  // Triangle
    }
}

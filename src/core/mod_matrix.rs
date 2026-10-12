//! Modulation routing matrix.
//!
//! A [`ModMatrix`] holds [`MOD_SLOTS`] routing slots. Each slot connects a
//! [`ModSource`] (LFO, envelope, performance controller) to a [`ModDest`]
//! (synth/effect parameter) with a bipolar amount. Per sample, each active
//! voice computes the current value of every source, then accumulates
//! `source * amount` into per-destination offset buffers which are applied
//! on top of the base parameter values.

use serde::{Deserialize, Serialize};

/// Number of routing slots in the matrix.
pub const MOD_SLOTS: usize = 16;

/// Modulation sources.
///
/// Bipolar sources output -1.0..=+1.0; unipolar sources output 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModSource {
    #[default]
    Off,
    Lfo1,
    Lfo2,
    Lfo3,
    AmpEnv,
    FilterEnv,
    Env3,
    Env4,
    Velocity,
    KeyTrack,
    ModWheel,
    Aftertouch,
    PitchBend,
    Constant,
}

impl ModSource {
    pub fn name(&self) -> &'static str {
        match self {
            ModSource::Off => "Off",
            ModSource::Lfo1 => "LFO 1",
            ModSource::Lfo2 => "LFO 2",
            ModSource::Lfo3 => "LFO 3",
            ModSource::AmpEnv => "Amp Env",
            ModSource::FilterEnv => "Filter Env",
            ModSource::Env3 => "Env 3",
            ModSource::Env4 => "Env 4",
            ModSource::Velocity => "Velocity",
            ModSource::KeyTrack => "Key Track",
            ModSource::ModWheel => "Mod Wheel",
            ModSource::Aftertouch => "Aftertouch",
            ModSource::PitchBend => "Pitch Bend",
            ModSource::Constant => "Constant",
        }
    }

    /// All selectable sources (excludes `Off`; `Off` is implied by `enabled`).
    pub fn all() -> &'static [ModSource] {
        &[
            ModSource::Lfo1,
            ModSource::Lfo2,
            ModSource::Lfo3,
            ModSource::AmpEnv,
            ModSource::FilterEnv,
            ModSource::Env3,
            ModSource::Env4,
            ModSource::Velocity,
            ModSource::KeyTrack,
            ModSource::ModWheel,
            ModSource::Aftertouch,
            ModSource::PitchBend,
            ModSource::Constant,
        ]
    }

    /// Whether the engine currently feeds a live value for this source.
    pub fn is_wired(&self) -> bool {
        !matches!(self, ModSource::Off)
    }

    /// Whether this source is bipolar (-1..1) rather than unipolar (0..1).
    pub fn is_bipolar(&self) -> bool {
        matches!(
            self,
            ModSource::Lfo1
                | ModSource::Lfo2
                | ModSource::Lfo3
                | ModSource::KeyTrack
                | ModSource::PitchBend
        )
    }
}

/// Modulation destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModDest {
    #[default]
    Off,
    // Oscillator pitch
    AllPitch,
    // Filter
    FilterCutoff,
    FilterResonance,
    // Amplitude / stereo
    Amplitude,
    StereoWidth,
    Osc1Level,
    Osc2Level,
    Osc3Level,
    Osc1Pan,
    Osc2Pan,
    Osc3Pan,
    // Effects (global, applied by the engine rather than per-voice)
    DelayTime,
    DelayFeedback,
    DelayMix,
    ReverbMix,
    ChorusRate,
    ChorusDepth,
    ChorusMix,
    // Mod-on-mod
    Lfo1Rate,
    Lfo2Rate,
    Lfo3Rate,
    Lfo1Depth,
    Lfo2Depth,
    Lfo3Depth,
}

impl ModDest {
    pub fn name(&self) -> &'static str {
        match self {
            ModDest::Off => "Off",
            ModDest::AllPitch => "Pitch (all)",
            ModDest::FilterCutoff => "Cutoff",
            ModDest::FilterResonance => "Resonance",
            ModDest::Amplitude => "Amplitude",
            ModDest::StereoWidth => "Stereo Width",
            ModDest::Osc1Level => "Osc 1 Level",
            ModDest::Osc2Level => "Osc 2 Level",
            ModDest::Osc3Level => "Osc 3 Level",
            ModDest::Osc1Pan => "Osc 1 Pan",
            ModDest::Osc2Pan => "Osc 2 Pan",
            ModDest::Osc3Pan => "Osc 3 Pan",
            ModDest::DelayTime => "Delay Time",
            ModDest::DelayFeedback => "Delay Fdbk",
            ModDest::DelayMix => "Delay Mix",
            ModDest::ReverbMix => "Reverb Mix",
            ModDest::ChorusRate => "Chorus Rate",
            ModDest::ChorusDepth => "Chorus Depth",
            ModDest::ChorusMix => "Chorus Mix",
            ModDest::Lfo1Rate => "LFO 1 Rate",
            ModDest::Lfo2Rate => "LFO 2 Rate",
            ModDest::Lfo3Rate => "LFO 3 Rate",
            ModDest::Lfo1Depth => "LFO 1 Depth",
            ModDest::Lfo2Depth => "LFO 2 Depth",
            ModDest::Lfo3Depth => "LFO 3 Depth",
        }
    }

    /// All selectable destinations (excludes `Off`).
    pub fn all() -> &'static [ModDest] {
        &[
            ModDest::AllPitch,
            ModDest::FilterCutoff,
            ModDest::FilterResonance,
            ModDest::Amplitude,
            ModDest::StereoWidth,
            ModDest::Osc1Level,
            ModDest::Osc2Level,
            ModDest::Osc3Level,
            ModDest::Osc1Pan,
            ModDest::Osc2Pan,
            ModDest::Osc3Pan,
            ModDest::DelayTime,
            ModDest::DelayFeedback,
            ModDest::DelayMix,
            ModDest::ReverbMix,
            ModDest::ChorusRate,
            ModDest::ChorusDepth,
            ModDest::ChorusMix,
            ModDest::Lfo1Rate,
            ModDest::Lfo2Rate,
            ModDest::Lfo3Rate,
            ModDest::Lfo1Depth,
            ModDest::Lfo2Depth,
            ModDest::Lfo3Depth,
        ]
    }

    /// Whether the DSP currently applies this destination. Global effect
    /// destinations are not yet wired into the effects chain.
    pub fn is_wired(&self) -> bool {
        self.is_per_voice()
            || matches!(
                self,
                ModDest::Lfo1Rate
                    | ModDest::Lfo2Rate
                    | ModDest::Lfo3Rate
                    | ModDest::Lfo1Depth
                    | ModDest::Lfo2Depth
                    | ModDest::Lfo3Depth
            )
    }

    /// Whether this destination is evaluated per-voice (as opposed to global
    /// engine-level destinations such as effect parameters).
    pub fn is_per_voice(&self) -> bool {
        matches!(
            self,
            ModDest::AllPitch
                | ModDest::FilterCutoff
                | ModDest::FilterResonance
                | ModDest::Amplitude
                | ModDest::StereoWidth
                | ModDest::Osc1Level
                | ModDest::Osc2Level
                | ModDest::Osc3Level
                | ModDest::Osc1Pan
                | ModDest::Osc2Pan
                | ModDest::Osc3Pan
        )
    }
}

/// Physical modulation range in destination units for `amount = ±1.0`.
///
/// Pitch-type destinations are in semitones; cutoff is in semitones applied
/// multiplicatively (`cutoff * 2^(semi/12)`); the rest are normalized units
/// the caller interprets.
pub fn dest_range(dest: ModDest) -> f32 {
    match dest {
        ModDest::Off => 0.0,
        ModDest::AllPitch => 12.0,         // ±12 semitones
        ModDest::FilterCutoff => 60.0,     // ±5 octaves
        ModDest::FilterResonance => 1.0,
        ModDest::Amplitude => 1.0,
        ModDest::StereoWidth => 1.0,
        ModDest::Osc1Level | ModDest::Osc2Level | ModDest::Osc3Level => 1.0,
        ModDest::Osc1Pan | ModDest::Osc2Pan | ModDest::Osc3Pan => 1.0,
        ModDest::DelayTime
        | ModDest::DelayFeedback
        | ModDest::DelayMix
        | ModDest::ReverbMix
        | ModDest::ChorusRate
        | ModDest::ChorusDepth
        | ModDest::ChorusMix => 1.0,
        ModDest::Lfo1Rate | ModDest::Lfo2Rate | ModDest::Lfo3Rate => 1.0,
        ModDest::Lfo1Depth | ModDest::Lfo2Depth | ModDest::Lfo3Depth => 1.0,
    }
}

/// One routing slot: `dest += source_value * amount * dest_range(dest)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModSlot {
    pub source: ModSource,
    pub dest: ModDest,
    /// Bipolar amount, -1.0..=1.0.
    pub amount: f32,
    pub enabled: bool,
}

impl Default for ModSlot {
    fn default() -> Self {
        Self {
            source: ModSource::Off,
            dest: ModDest::Off,
            amount: 0.0,
            enabled: false,
        }
    }
}

/// The full routing matrix: a fixed array of slots.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModMatrix {
    pub slots: [ModSlot; MOD_SLOTS],
}

impl Default for ModMatrix {
    fn default() -> Self {
        Self {
            slots: [ModSlot::default(); MOD_SLOTS],
        }
    }
}

impl ModMatrix {
    /// True when no slot is enabled.
    pub fn is_empty(&self) -> bool {
        self.slots.iter().all(|s| !s.enabled)
    }

    /// Accumulated normalized offset for a destination given a lookup of
    /// source values. The returned value is already scaled by
    /// [`dest_range`] (i.e. in destination units).
    pub fn offset_for<F>(&self, dest: ModDest, source_value: F) -> f32
    where
        F: Fn(ModSource) -> f32,
    {
        let mut acc = 0.0;
        for slot in &self.slots {
            if slot.enabled && slot.dest == dest && slot.source != ModSource::Off {
                acc += source_value(slot.source) * slot.amount;
            }
        }
        acc * dest_range(dest)
    }
}

/// Snapshot of per-source values for one sample, as seen by a single voice.
/// Sources that are not yet implemented default to 0.
#[derive(Debug, Clone, Copy, Default)]
pub struct ModSourceValues {
    pub lfo: [f32; 3],
    pub amp_env: f32,
    pub filter_env: f32,
    pub env3: f32,
    pub env4: f32,
    pub velocity: f32,
    pub key_track: f32,
    pub mod_wheel: f32,
    pub aftertouch: f32,
    pub pitch_bend: f32,
}

impl ModSourceValues {
    pub fn get(&self, source: ModSource) -> f32 {
        match source {
            ModSource::Off => 0.0,
            ModSource::Lfo1 => self.lfo[0],
            ModSource::Lfo2 => self.lfo[1],
            ModSource::Lfo3 => self.lfo[2],
            ModSource::AmpEnv => self.amp_env,
            ModSource::FilterEnv => self.filter_env,
            ModSource::Env3 => self.env3,
            ModSource::Env4 => self.env4,
            ModSource::Velocity => self.velocity,
            ModSource::KeyTrack => self.key_track,
            ModSource::ModWheel => self.mod_wheel,
            ModSource::Aftertouch => self.aftertouch,
            ModSource::PitchBend => self.pitch_bend,
            ModSource::Constant => 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_matrix_no_offset() {
        let m = ModMatrix::default();
        assert!(m.is_empty());
        assert_eq!(
            m.offset_for(ModDest::FilterCutoff, |_| 1.0),
            0.0
        );
    }

    #[test]
    fn test_slot_accumulation() {
        let mut m = ModMatrix::default();
        m.slots[0] = ModSlot {
            source: ModSource::Lfo1,
            dest: ModDest::AllPitch,
            amount: 1.0,
            enabled: true,
        };
        let values = ModSourceValues {
            lfo: [0.5, 0.0, 0.0],
            ..Default::default()
        };
        let off = m.offset_for(ModDest::AllPitch, |s| values.get(s));
        assert!((off - 6.0).abs() < 1e-6, "expected +6 semitones, got {}", off);
    }
}

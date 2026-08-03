//! SynthBackend trait abstraction
//!
//! Abstracts all differences between standalone and plugin backends so that
//! a single GUI codebase (`SynthApp`) can drive both.

use crate::core::params::SynthParam;
use crate::core::voice::PolyphonyMode;

/// A CC-to-parameter mapping entry
#[derive(Debug, Clone)]
pub struct CcMapping {
    pub cc: u8,
    pub param: SynthParam,
}

/// MIDI learn state
#[derive(Debug, Clone, PartialEq)]
pub enum MidiLearnState {
    Idle,
    Waiting(SynthParam),
}

/// The single trait that both `StandaloneBackend` and `PluginBackend` implement.
///
/// The GUI only calls methods on this trait — never touches `AudioEngine`,
/// `SharedState`, `ParamSetter`, or any backend-specific type directly.
pub trait SynthBackend {
    // ========================================================================
    // Parameter access
    // ========================================================================

    fn get_param(&self, param: SynthParam) -> f32;
    fn set_param(&mut self, param: SynthParam, value: f32);

    /// Gesture hooks bracketing a user edit (e.g. a slider drag).
    /// The plugin backend forwards these to the host so DAW automation
    /// recording works; the standalone backend leaves them as no-ops.
    fn begin_param_change(&mut self, _param: SynthParam) {}
    fn end_param_change(&mut self, _param: SynthParam) {}

    // ========================================================================
    // Effects – Delay
    // ========================================================================

    fn delay_enabled(&self) -> bool;
    fn set_delay_enabled(&mut self, enabled: bool);

    fn delay_time(&self) -> f32;
    fn set_delay_time(&mut self, time: f32);

    fn delay_feedback(&self) -> f32;
    fn set_delay_feedback(&mut self, feedback: f32);

    fn delay_mix(&self) -> f32;
    fn set_delay_mix(&mut self, mix: f32);

    // ========================================================================
    // Effects – Reverb
    // ========================================================================

    fn reverb_enabled(&self) -> bool;
    fn set_reverb_enabled(&mut self, enabled: bool);

    fn reverb_room_size(&self) -> f32;
    fn set_reverb_room_size(&mut self, size: f32);

    fn reverb_damping(&self) -> f32;
    fn set_reverb_damping(&mut self, damping: f32);

    fn reverb_mix(&self) -> f32;
    fn set_reverb_mix(&mut self, mix: f32);

    // ========================================================================
    // Effects – Chorus
    // ========================================================================

    fn chorus_enabled(&self) -> bool;
    fn set_chorus_enabled(&mut self, enabled: bool);

    fn chorus_rate(&self) -> f32;
    fn set_chorus_rate(&mut self, rate: f32);

    fn chorus_depth(&self) -> f32;
    fn set_chorus_depth(&mut self, depth: f32);

    fn chorus_mix(&self) -> f32;
    fn set_chorus_mix(&mut self, mix: f32);

    // ========================================================================
    // Voice management
    // ========================================================================

    fn polyphony_mode(&self) -> PolyphonyMode;
    fn set_polyphony_mode(&mut self, mode: PolyphonyMode);

    fn active_voice_count(&self) -> usize;
    fn max_voices(&self) -> usize;

    // ========================================================================
    // Stereo
    // ========================================================================

    fn stereo_width(&self) -> f32;
    fn set_stereo_width(&mut self, width: f32);

    fn osc_pan(&self, osc_num: u8) -> f32;
    fn set_osc_pan(&mut self, osc_num: u8, pan: f32);

    // ========================================================================
    // MIDI
    // ========================================================================

    /// Whether a MIDI input is currently connected
    fn midi_connected(&self) -> bool;

    /// List available MIDI port names (empty for plugin)
    fn midi_ports(&self) -> Vec<String>;

    /// Currently selected MIDI port index (None if not connected)
    fn selected_midi_port(&self) -> Option<usize>;

    /// Connect to the given port index
    fn connect_midi_port(&mut self, port_idx: usize);

    /// Refresh the list of available MIDI ports
    fn refresh_midi_ports(&mut self);

    /// Current MIDI channel display index (0 = All, 1-16 = specific)
    fn midi_channel_display(&self) -> u8;

    /// Set MIDI channel filter by display index
    fn set_midi_channel(&mut self, display_idx: u8);

    /// Recent MIDI CC feedback: sorted list of (cc, value) pairs
    fn midi_cc_activity(&self) -> Vec<(u8, u8)>;

    /// Current MIDI learn state
    fn midi_learn_state(&self) -> MidiLearnState;

    /// Start learning a CC for the given parameter
    fn start_midi_learn(&mut self, param: SynthParam);

    /// Cancel current MIDI learn
    fn cancel_midi_learn(&mut self);

    /// Custom (user) CC mappings
    fn custom_cc_mappings(&self) -> Vec<CcMapping>;

    /// Clear all custom CC mappings
    fn clear_cc_mappings(&mut self);

    // ========================================================================
    // Status
    // ========================================================================

    /// CPU load as a percentage (0.0 – 100.0)
    fn get_cpu_load(&self) -> f32;

    // ========================================================================
    // Per-frame update hook
    //
    // Called once per GUI frame so the backend can:
    //  - poll MIDI events and forward them to audio
    //  - sync the ParamBank → VoiceManager
    // The default implementation does nothing (suitable for plugin backend).
    // ========================================================================

    fn update(&mut self) {}
}

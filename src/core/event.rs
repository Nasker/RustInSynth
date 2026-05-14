use super::types::{Amplitude, MidiNote};

/// Available waveform types
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WaveformType {
    #[default]
    Sine,
    Square,
    Saw,
    Triangle,
}

impl WaveformType {
    /// Get waveform from index (0-3)
    pub fn from_index(index: u8) -> Self {
        match index % 4 {
            0 => WaveformType::Sine,
            1 => WaveformType::Square,
            2 => WaveformType::Saw,
            3 => WaveformType::Triangle,
            _ => WaveformType::Sine,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            WaveformType::Sine => "Sine",
            WaveformType::Square => "Square",
            WaveformType::Saw => "Sawtooth",
            WaveformType::Triangle => "Triangle",
        }
    }
}

/// Type of synth event - designed to be compatible with MIDI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthEventKind {
    NoteOn,
    NoteOff,
    WaveformChange(WaveformType),
    ControlChange { cc: u8, value: u8 },
}

/// Type of note event - for backwards compatibility
pub type NoteEventKind = SynthEventKind;

/// A synth event, compatible with MIDI messages
#[derive(Debug, Clone, Copy)]
pub struct SynthEvent {
    pub kind: SynthEventKind,
    pub note: MidiNote,
    pub velocity: Amplitude,
}

/// Alias for backwards compatibility
pub type NoteEvent = SynthEvent;

impl SynthEvent {
    pub fn note_on(note: MidiNote, velocity: Amplitude) -> Self {
        Self {
            kind: SynthEventKind::NoteOn,
            note,
            velocity: velocity.clamp(0.0, 1.0),
        }
    }

    pub fn note_off(note: MidiNote) -> Self {
        Self {
            kind: SynthEventKind::NoteOff,
            note,
            velocity: 0.0,
        }
    }

    /// Create a note on event from MIDI velocity (0-127)
    pub fn note_on_midi(note: MidiNote, midi_velocity: u8) -> Self {
        Self::note_on(note, midi_velocity as f32 / 127.0)
    }

    /// Create a waveform change event
    pub fn waveform_change(waveform: WaveformType) -> Self {
        Self {
            kind: SynthEventKind::WaveformChange(waveform),
            note: 0,
            velocity: 0.0,
        }
    }

    /// Create a control change event
    pub fn control_change(cc: u8, value: u8) -> Self {
        Self {
            kind: SynthEventKind::ControlChange { cc, value },
            note: 0,
            velocity: 0.0,
        }
    }
}

/// Trait for anything that can receive synth events (synth, voice manager, etc.)
pub trait SynthEventReceiver: Send {
    fn receive_event(&mut self, event: SynthEvent);
}

/// Alias for backwards compatibility
pub trait NoteEventReceiver: SynthEventReceiver {}

pub mod envelope;
pub mod event;
pub mod filter;
pub mod oscillator;
pub mod params;
pub mod types;
pub mod voice;

pub use envelope::{Envelope, EnvelopeState, AREnvelope, ADSREnvelope};
pub use event::{NoteEvent, NoteEventKind, SynthEvent, SynthEventKind, SynthEventReceiver, WaveformType};
pub use filter::{Filter, SVFilter, FilterMode, cc_to_cutoff, cc_to_resonance};
pub use oscillator::{Oscillator, SineOscillator, SquareOscillator, SawOscillator, TriangleOscillator, OscillatorBank, OscillatorUnit, create_oscillator};
pub use params::{SynthParam, CCMapping, cc_to_time, cc_to_level, cc_to_semitones, cc_to_cents, cc_to_waveform, cc_to_phase, cc_to_sustain, cc};
pub use types::{Sample, SampleRate, Frequency, Amplitude};
pub use voice::{Voice, VoiceManager};

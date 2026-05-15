pub mod envelope;
pub mod event;
pub mod filter;
pub mod lfo;
pub mod oscillator;
pub mod params;
pub mod presets;
pub mod types;
pub mod voice;

pub use envelope::{Envelope, EnvelopeState, AREnvelope, ADSREnvelope};
pub use event::{NoteEvent, NoteEventKind, SynthEvent, SynthEventKind, SynthEventReceiver, WaveformType};
pub use filter::{Filter, SVFilter, FilterMode, cc_to_cutoff, cc_to_resonance};
pub use lfo::{LFO, LfoDestination, LfoWaveform};
pub use oscillator::{Oscillator, SineOscillator, SquareOscillator, SawOscillator, TriangleOscillator, NoiseOscillator, OscillatorBank, OscillatorUnit, create_oscillator};
pub use params::{SynthParam, CCMapping, cc_to_time, cc_to_level, cc_to_semitones, cc_to_cents, cc_to_waveform, cc_to_phase, cc_to_sustain, cc_to_pitch_bend_range, cc_to_filter_env_amount, cc_to_lfo_rate, cc_to_lfo_depth, cc_to_lfo_waveform, cc_to_lfo_destination, cc};
pub use presets::{Preset, PresetError, default_presets_dir, ensure_presets_dir, list_presets, load_preset, save_preset};
pub use types::{Sample, SampleRate, Frequency, Amplitude};
pub use voice::{Voice, VoiceManager};

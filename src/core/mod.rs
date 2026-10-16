pub mod effects;
pub mod envelope;
pub mod event;
pub mod factory_bank;
pub mod filter;
pub mod lfo;
pub mod mod_matrix;
pub mod oscillator;
pub mod param_spec;
pub mod params;
pub mod presets;
pub mod types;
pub mod voice;

pub use effects::{Chorus, Delay, Effect, EffectsChain, Reverb};
pub use envelope::{ADSREnvelope, AREnvelope, Envelope, EnvelopeState};
pub use event::{
    NoteEvent, NoteEventKind, SynthEvent, SynthEventKind, SynthEventReceiver, WaveformType,
};
pub use filter::{cc_to_cutoff, cc_to_resonance, Filter, FilterMode, SVFilter};
pub use lfo::{LfoDestination, LfoWaveform, LFO};
pub use mod_matrix::{ModDest, ModMatrix, ModSlot, ModSource, ModSourceValues, MOD_SLOTS};
pub use oscillator::{
    create_oscillator, NoiseOscillator, Oscillator, OscillatorBank, OscillatorUnit, SawOscillator,
    SineOscillator, SquareOscillator, TriangleOscillator,
};
pub use params::{
    cc, cc_to_cents, cc_to_filter_env_amount, cc_to_level, cc_to_lfo_depth, cc_to_lfo_destination,
    cc_to_lfo_rate, cc_to_lfo_waveform, cc_to_phase, cc_to_pitch_bend_range, cc_to_semitones,
    cc_to_sustain, cc_to_time, cc_to_waveform, CCMapping, SynthParam,
};
pub use presets::{
    default_presets_dir, ensure_presets_dir, list_presets, load_preset, save_preset, Preset,
    PresetError,
};
pub use types::{Amplitude, Frequency, Sample, SampleRate, StereoSample};
pub use voice::{LfoSettings, ModState, PolyphonyMode, Voice, VoiceManager};

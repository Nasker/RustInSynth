pub mod envelope;
pub mod event;
pub mod filter;
pub mod oscillator;
pub mod params;
pub mod types;
pub mod voice;

pub use envelope::{Envelope, EnvelopeState, AREnvelope};
pub use event::{NoteEvent, NoteEventKind, SynthEvent, SynthEventKind, SynthEventReceiver, WaveformType};
pub use filter::{Filter, SVFilter, FilterMode, cc_to_cutoff, cc_to_resonance};
pub use oscillator::{Oscillator, SineOscillator, SquareOscillator, SawOscillator, TriangleOscillator};
pub use params::{SynthParam, CCMapping, cc_to_time};
pub use types::{Sample, SampleRate, Frequency, Amplitude};
pub use voice::{Voice, VoiceManager};

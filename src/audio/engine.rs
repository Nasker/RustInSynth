use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use parking_lot::Mutex;

use crate::core::event::{NoteEvent, SynthEventReceiver, WaveformType};
use crate::core::params::SynthParam;
use crate::core::types::SampleRate;
use crate::core::voice::VoiceManager;
use crate::gui::ParamBank;

/// Error type for audio engine operations
#[derive(Debug)]
pub enum AudioError {
    NoDevice,
    NoConfig,
    StreamError(String),
}

impl std::fmt::Display for AudioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AudioError::NoDevice => write!(f, "No audio output device found"),
            AudioError::NoConfig => write!(f, "No supported audio config found"),
            AudioError::StreamError(e) => write!(f, "Audio stream error: {}", e),
        }
    }
}

impl std::error::Error for AudioError {}

/// The main audio engine that manages audio output and synthesis
pub struct AudioEngine {
    _host: Host,
    _device: Device,
    _config: StreamConfig,
    stream: Option<Stream>,
    voice_manager: Arc<Mutex<VoiceManager>>,
    sample_rate: SampleRate,
}

impl AudioEngine {
    /// Create a new audio engine with default settings
    pub fn new() -> Result<Self, AudioError> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or(AudioError::NoDevice)?;

        let config = device
            .default_output_config()
            .map_err(|_| AudioError::NoConfig)?;

        let sample_rate = config.sample_rate().0;
        let stream_config: StreamConfig = config.into();

        let voice_manager = Arc::new(Mutex::new(VoiceManager::monophonic(sample_rate)));

        Ok(Self {
            _host: host,
            _device: device,
            _config: stream_config,
            stream: None,
            voice_manager,
            sample_rate,
        })
    }

    /// Get the sample rate of the audio engine
    pub fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    /// Get a reference to the voice manager for configuration
    pub fn voice_manager(&self) -> Arc<Mutex<VoiceManager>> {
        Arc::clone(&self.voice_manager)
    }

    /// Start the audio stream
    pub fn start(&mut self) -> Result<(), AudioError> {
        let voice_manager = Arc::clone(&self.voice_manager);
        let channels = self._config.channels as usize;

        let stream = self
            ._device
            .build_output_stream(
                &self._config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut vm = voice_manager.lock();
                    for frame in data.chunks_mut(channels) {
                        let sample = vm.next_sample();
                        for channel_sample in frame.iter_mut() {
                            *channel_sample = sample;
                        }
                    }
                },
                move |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        stream
            .play()
            .map_err(|e| AudioError::StreamError(e.to_string()))?;

        self.stream = Some(stream);
        Ok(())
    }

    /// Stop the audio stream
    pub fn stop(&mut self) {
        self.stream = None;
    }

    /// Send a note event to the synth
    pub fn send_event(&self, event: NoteEvent) {
        self.voice_manager.lock().receive_event(event);
    }

    /// Set the master volume (0.0 to 1.0)
    pub fn set_master_volume(&self, volume: f32) {
        self.voice_manager.lock().set_master_volume(volume);
    }

    /// Set the waveform for a specific oscillator (1, 2, or 3)
    pub fn set_osc_waveform(&self, osc_num: u8, waveform: WaveformType) {
        self.voice_manager.lock().set_osc_waveform(osc_num, waveform);
    }

    /// Set waveform for all oscillators (convenience method)
    pub fn set_waveform(&self, waveform: WaveformType) {
        let mut vm = self.voice_manager.lock();
        vm.set_osc_waveform(1, waveform);
        vm.set_osc_waveform(2, waveform);
        vm.set_osc_waveform(3, waveform);
    }

    /// Get the current waveform type for oscillator 1
    pub fn waveform(&self) -> WaveformType {
        self.voice_manager.lock().osc_state().osc1_waveform
    }

    /// Sync parameters from ParamBank to VoiceManager
    /// Call this periodically from the GUI thread (e.g., every frame)
    pub fn sync_params(&self, params: &ParamBank) {
        let mut vm = self.voice_manager.lock();

        // Filter parameters
        vm.set_filter_cutoff(params.get(SynthParam::FilterCutoff));
        vm.set_filter_resonance(params.get(SynthParam::FilterResonance));

        // Amplitude envelope
        vm.set_attack(params.get(SynthParam::Attack));
        vm.set_decay(params.get(SynthParam::Decay));
        vm.set_sustain(params.get(SynthParam::Sustain));
        vm.set_release(params.get(SynthParam::Release));

        // Filter envelope
        vm.set_filter_attack(params.get(SynthParam::FilterAttack));
        vm.set_filter_decay(params.get(SynthParam::FilterDecay));
        vm.set_filter_sustain(params.get(SynthParam::FilterSustain));
        vm.set_filter_release(params.get(SynthParam::FilterRelease));
        vm.set_filter_env_amount(params.get(SynthParam::FilterEnvAmount));

        // LFO parameters
        use crate::core::lfo::{LfoDestination, LfoWaveform};
        vm.set_lfo_rate(params.get(SynthParam::LfoRate));
        vm.set_lfo_depth(params.get(SynthParam::LfoDepth));
        let waveform_idx = params.get(SynthParam::LfoWaveform) as u8;
        let lfo_waveform = match waveform_idx {
            0 => LfoWaveform::Sine,
            1 => LfoWaveform::Triangle,
            2 => LfoWaveform::Square,
            3 => LfoWaveform::Saw,
            _ => LfoWaveform::Random,
        };
        vm.set_lfo_waveform(lfo_waveform);
        let dest_idx = params.get(SynthParam::LfoDestination) as u8;
        let lfo_dest = match dest_idx {
            0 => LfoDestination::Off,
            1 => LfoDestination::Pitch,
            2 => LfoDestination::FilterCutoff,
            _ => LfoDestination::Amplitude,
        };
        vm.set_lfo_destination(lfo_dest);

        // Oscillator parameters (using conversion functions)
        use crate::core::event::WaveformType;
        let waveform1 = WaveformType::from_index(params.get(SynthParam::Osc1Waveform) as u8);
        let waveform2 = WaveformType::from_index(params.get(SynthParam::Osc2Waveform) as u8);
        let waveform3 = WaveformType::from_index(params.get(SynthParam::Osc3Waveform) as u8);
        vm.set_osc_waveform(1, waveform1);
        vm.set_osc_waveform(2, waveform2);
        vm.set_osc_waveform(3, waveform3);

        vm.set_osc_level(1, params.get(SynthParam::Osc1Level));
        vm.set_osc_level(2, params.get(SynthParam::Osc2Level));
        vm.set_osc_level(3, params.get(SynthParam::Osc3Level));

        // Oscillator detune
        vm.set_osc_semitones(2, params.get(SynthParam::Osc2Semitones) as i8);
        vm.set_osc_cents(2, params.get(SynthParam::Osc2Cents) as i8);
        vm.set_osc_semitones(3, params.get(SynthParam::Osc3Semitones) as i8);
        vm.set_osc_cents(3, params.get(SynthParam::Osc3Cents) as i8);

        // Oscillator phase
        vm.set_osc_phase(1, params.get(SynthParam::Osc1Phase));
        vm.set_osc_phase(2, params.get(SynthParam::Osc2Phase));
        vm.set_osc_phase(3, params.get(SynthParam::Osc3Phase));

        // Pitch bend range
        vm.set_pitch_bend_range(params.get(SynthParam::PitchBendRange) as u8);

        // Portamento
        vm.set_portamento_time(params.get(SynthParam::PortamentoTime));

        // Master volume
        vm.set_master_volume(params.get(SynthParam::MasterVolume));
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

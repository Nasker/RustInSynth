use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use parking_lot::Mutex;

use crate::core::event::{NoteEvent, SynthEventReceiver, WaveformType};
use crate::core::types::SampleRate;
use crate::core::voice::VoiceManager;

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

    /// Set the waveform type
    pub fn set_waveform(&self, waveform: WaveformType) {
        self.voice_manager.lock().set_waveform(waveform);
    }

    /// Get the current waveform type
    pub fn waveform(&self) -> WaveformType {
        self.voice_manager.lock().waveform()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

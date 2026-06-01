use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use parking_lot::Mutex;

use crate::core::event::{NoteEvent, SynthEventReceiver, WaveformType};
use crate::core::params::SynthParam;
use crate::core::types::SampleRate;
use crate::core::voice::{VoiceManager, PolyphonyMode};
use crate::core::effects::EffectsChain;
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
    effects_chain: Arc<Mutex<EffectsChain>>,
    sample_rate: SampleRate,
    cpu_load: Arc<AtomicU32>,
}

impl AudioEngine {
    /// Create a new audio engine with default settings
    pub fn new(cpu_load: Arc<AtomicU32>) -> Result<Self, AudioError> {
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
        let effects_chain = Arc::new(Mutex::new(EffectsChain::new(sample_rate)));

        Ok(Self {
            _host: host,
            _device: device,
            _config: stream_config,
            stream: None,
            voice_manager,
            effects_chain,
            sample_rate,
            cpu_load,
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
        let effects_chain = Arc::clone(&self.effects_chain);
        let channels = self._config.channels as usize;
        let cpu_load = Arc::clone(&self.cpu_load);
        let sample_rate = self.sample_rate as f64;

        let stream = self
            ._device
            .build_output_stream(
                &self._config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let start = Instant::now();
                    
                    let mut vm = voice_manager.lock();
                    let mut fx = effects_chain.lock();
                    for frame in data.chunks_mut(channels) {
                        // Generate stereo sample from voices
                        let stereo = vm.next_sample_stereo();
                        // Process through stereo effects chain
                        let processed = fx.process_stereo(stereo);
                        
                        // Output to channels (stereo or mono)
                        if channels >= 2 {
                            frame[0] = processed.left;
                            frame[1] = processed.right;
                            // Fill remaining channels with silence if > 2
                            for ch in frame.iter_mut().skip(2) {
                                *ch = 0.0;
                            }
                        } else {
                            // Mono output: mix L+R
                            frame[0] = (processed.left + processed.right) * 0.5;
                        }
                    }
                    drop(vm);
                    drop(fx);
                    
                    // Calculate CPU load: process_time / available_time
                    let elapsed = start.elapsed().as_secs_f64();
                    let buffer_samples = data.len() / channels;
                    let available_time = buffer_samples as f64 / sample_rate;
                    let load = (elapsed / available_time) as f32;
                    
                    // Store as atomic (smoothed with previous value)
                    let prev = f32::from_bits(cpu_load.load(Ordering::Relaxed));
                    let smoothed = prev * 0.9 + load * 0.1; // Simple low-pass filter
                    cpu_load.store(smoothed.to_bits(), Ordering::Relaxed);
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

        // Stereo imaging
        vm.set_osc_pan(1, params.get(SynthParam::Osc1Pan));
        vm.set_osc_pan(2, params.get(SynthParam::Osc2Pan));
        vm.set_osc_pan(3, params.get(SynthParam::Osc3Pan));
        vm.set_stereo_width(params.get(SynthParam::StereoWidth));

        // Pitch bend range
        vm.set_pitch_bend_range(params.get(SynthParam::PitchBendRange) as u8);

        // Portamento
        vm.set_portamento_time(params.get(SynthParam::PortamentoTime));

        // Master volume
        vm.set_master_volume(params.get(SynthParam::MasterVolume));
    }
    
    /// Set polyphony mode (Mono or Poly)
    pub fn set_polyphony_mode(&self, mode: PolyphonyMode) {
        self.voice_manager.lock().set_polyphony_mode(mode);
    }
    
    /// Get current polyphony mode
    pub fn polyphony_mode(&self) -> PolyphonyMode {
        self.voice_manager.lock().polyphony_mode()
    }
    
    /// Get number of active voices
    pub fn active_voice_count(&self) -> usize {
        self.voice_manager.lock().active_voice_count()
    }
    
    /// Get max voices
    pub fn max_voices(&self) -> usize {
        self.voice_manager.lock().max_voices()
    }
    
    // ========================================================================
    // Stereo Control
    // ========================================================================
    
    /// Set oscillator pan (-1.0 = left, 0.0 = center, 1.0 = right)
    pub fn set_osc_pan(&self, osc_num: u8, pan: f32) {
        self.voice_manager.lock().set_osc_pan(osc_num, pan);
    }
    
    /// Get oscillator pan
    pub fn osc_pan(&self, osc_num: u8) -> f32 {
        self.voice_manager.lock().osc_pan(osc_num)
    }
    
    /// Set stereo width (0.0 = mono, 1.0 = normal, 2.0 = extra wide)
    pub fn set_stereo_width(&self, width: f32) {
        self.voice_manager.lock().set_stereo_width(width);
    }
    
    /// Get stereo width
    pub fn stereo_width(&self) -> f32 {
        self.voice_manager.lock().stereo_width()
    }
    
    // ========================================================================
    // Effects Control
    // ========================================================================
    
    /// Get a reference to the effects chain
    pub fn effects_chain(&self) -> Arc<Mutex<EffectsChain>> {
        Arc::clone(&self.effects_chain)
    }
    
    // Delay
    pub fn set_delay_enabled(&self, enabled: bool) {
        self.effects_chain.lock().delay_enabled = enabled;
    }
    pub fn delay_enabled(&self) -> bool {
        self.effects_chain.lock().delay_enabled
    }
    pub fn set_delay_time(&self, time: f32) {
        self.effects_chain.lock().delay.set_delay_time(time);
    }
    pub fn delay_time(&self) -> f32 {
        self.effects_chain.lock().delay.delay_time()
    }
    pub fn set_delay_feedback(&self, feedback: f32) {
        self.effects_chain.lock().delay.set_feedback(feedback);
    }
    pub fn delay_feedback(&self) -> f32 {
        self.effects_chain.lock().delay.feedback()
    }
    pub fn set_delay_mix(&self, mix: f32) {
        self.effects_chain.lock().delay.set_mix(mix);
    }
    pub fn delay_mix(&self) -> f32 {
        self.effects_chain.lock().delay.mix()
    }
    
    // Reverb
    pub fn set_reverb_enabled(&self, enabled: bool) {
        self.effects_chain.lock().reverb_enabled = enabled;
    }
    pub fn reverb_enabled(&self) -> bool {
        self.effects_chain.lock().reverb_enabled
    }
    pub fn set_reverb_room_size(&self, size: f32) {
        self.effects_chain.lock().reverb.set_room_size(size);
    }
    pub fn reverb_room_size(&self) -> f32 {
        self.effects_chain.lock().reverb.room_size()
    }
    pub fn set_reverb_damping(&self, damping: f32) {
        self.effects_chain.lock().reverb.set_damping(damping);
    }
    pub fn reverb_damping(&self) -> f32 {
        self.effects_chain.lock().reverb.damping()
    }
    pub fn set_reverb_mix(&self, mix: f32) {
        self.effects_chain.lock().reverb.set_mix(mix);
    }
    pub fn reverb_mix(&self) -> f32 {
        self.effects_chain.lock().reverb.mix()
    }
    
    // Chorus
    pub fn set_chorus_enabled(&self, enabled: bool) {
        self.effects_chain.lock().chorus_enabled = enabled;
    }
    pub fn chorus_enabled(&self) -> bool {
        self.effects_chain.lock().chorus_enabled
    }
    pub fn set_chorus_rate(&self, rate: f32) {
        self.effects_chain.lock().chorus.set_rate(rate);
    }
    pub fn chorus_rate(&self) -> f32 {
        self.effects_chain.lock().chorus.rate()
    }
    pub fn set_chorus_depth(&self, depth: f32) {
        self.effects_chain.lock().chorus.set_depth(depth);
    }
    pub fn chorus_depth(&self) -> f32 {
        self.effects_chain.lock().chorus.depth()
    }
    pub fn set_chorus_mix(&self, mix: f32) {
        self.effects_chain.lock().chorus.set_mix(mix);
    }
    pub fn chorus_mix(&self) -> f32 {
        self.effects_chain.lock().chorus.mix()
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop();
    }
}

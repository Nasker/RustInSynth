use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

use crate::core::effects::EffectsChain;
use crate::core::event::{SynthEvent, WaveformType, SynthEventReceiver};
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::voice::{PolyphonyMode, VoiceManager};
use crate::plugin_gui::editor::{EDITOR_HEIGHT, EDITOR_WIDTH};
use crate::plugin_gui::shared_state::PluginSharedState;

/// The NIH-plug implementation of RustInSynth
pub struct RustInSynthPlugin {
    params: Arc<RustInSynthParams>,
    voice_manager: VoiceManager,
    effects_chain: EffectsChain,
    // Track last synced waveforms to avoid allocations in audio thread
    last_osc1_waveform: i32,
    last_osc2_waveform: i32,
    last_osc3_waveform: i32,
    // GUI editor state
    editor_state: Arc<EguiState>,
    // Shared GUI metrics (CPU load) written from the audio thread
    gui_shared: PluginSharedState,
    // Sample rate captured at initialize() for CPU-load estimation
    sample_rate: f32,
}

#[derive(Params)]
pub struct RustInSynthParams {
    // Envelope
    #[id = "attack"]
    pub attack: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "sustain"]
    pub sustain: FloatParam,
    #[id = "release"]
    pub release: FloatParam,

    // Filter
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "resonance"]
    pub resonance: FloatParam,

    // Filter Env
    #[id = "f_attack"]
    pub f_attack: FloatParam,
    #[id = "f_decay"]
    pub f_decay: FloatParam,
    #[id = "f_sustain"]
    pub f_sustain: FloatParam,
    #[id = "f_release"]
    pub f_release: FloatParam,
    #[id = "f_amount"]
    pub f_amount: FloatParam,

    // LFO
    #[id = "lfo_rate"]
    pub lfo_rate: FloatParam,
    #[id = "lfo_depth"]
    pub lfo_depth: FloatParam,
    #[id = "lfo_waveform"]
    pub lfo_waveform: IntParam,
    #[id = "lfo_destination"]
    pub lfo_destination: IntParam,

    // Osc 1
    #[id = "osc1_waveform"]
    pub osc1_waveform: IntParam,
    #[id = "osc1_level"]
    pub osc1_level: FloatParam,
    #[id = "osc1_phase"]
    pub osc1_phase: FloatParam,
    #[id = "osc1_pan"]
    pub osc1_pan: FloatParam,

    // Osc 2
    #[id = "osc2_waveform"]
    pub osc2_waveform: IntParam,
    #[id = "osc2_level"]
    pub osc2_level: FloatParam,
    #[id = "osc2_semitones"]
    pub osc2_semitones: IntParam,
    #[id = "osc2_cents"]
    pub osc2_cents: IntParam,
    #[id = "osc2_phase"]
    pub osc2_phase: FloatParam,
    #[id = "osc2_pan"]
    pub osc2_pan: FloatParam,

    // Osc 3
    #[id = "osc3_waveform"]
    pub osc3_waveform: IntParam,
    #[id = "osc3_level"]
    pub osc3_level: FloatParam,
    #[id = "osc3_semitones"]
    pub osc3_semitones: IntParam,
    #[id = "osc3_cents"]
    pub osc3_cents: IntParam,
    #[id = "osc3_phase"]
    pub osc3_phase: FloatParam,
    #[id = "osc3_pan"]
    pub osc3_pan: FloatParam,

    // Master
    #[id = "portamento"]
    pub portamento: FloatParam,
    #[id = "pitch_bend_range"]
    pub pitch_bend_range: IntParam,
    #[id = "stereo_width"]
    pub stereo_width: FloatParam,
    #[id = "master_volume"]
    pub master_volume: FloatParam,

    // Polyphony mode: 0 = Mono, 1 = Poly
    #[id = "polyphony_mode"]
    pub polyphony_mode: IntParam,

    // Delay
    #[id = "delay_enabled"]
    pub delay_enabled: BoolParam,
    #[id = "delay_time"]
    pub delay_time: FloatParam,
    #[id = "delay_feedback"]
    pub delay_feedback: FloatParam,
    #[id = "delay_mix"]
    pub delay_mix: FloatParam,

    // Reverb
    #[id = "reverb_enabled"]
    pub reverb_enabled: BoolParam,
    #[id = "reverb_room_size"]
    pub reverb_room_size: FloatParam,
    #[id = "reverb_damping"]
    pub reverb_damping: FloatParam,
    #[id = "reverb_mix"]
    pub reverb_mix: FloatParam,

    // Chorus
    #[id = "chorus_enabled"]
    pub chorus_enabled: BoolParam,
    #[id = "chorus_rate"]
    pub chorus_rate: FloatParam,
    #[id = "chorus_depth"]
    pub chorus_depth: FloatParam,
    #[id = "chorus_mix"]
    pub chorus_mix: FloatParam,
}

impl Default for RustInSynthParams {
    fn default() -> Self {
        Self {
            attack: FloatParam::new(
                "Amp Attack",
                0.01,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),
            decay: FloatParam::new(
                "Amp Decay",
                0.1,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),
            sustain: FloatParam::new(
                "Amp Sustain",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            release: FloatParam::new(
                "Amp Release",
                0.2,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),

            cutoff: FloatParam::new(
                "Filter Cutoff",
                20000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20000.0,
                    factor: 0.2,
                },
            )
            .with_unit(" Hz"),
            resonance: FloatParam::new(
                "Filter Resonance",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            f_attack: FloatParam::new(
                "Filter Attack",
                0.01,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),
            f_decay: FloatParam::new(
                "Filter Decay",
                0.1,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),
            f_sustain: FloatParam::new(
                "Filter Sustain",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            f_release: FloatParam::new(
                "Filter Release",
                0.2,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: 0.25,
                },
            )
            .with_unit(" s"),
            f_amount: FloatParam::new(
                "Filter Env Amount",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            lfo_rate: FloatParam::new(
                "LFO Rate",
                1.0,
                FloatRange::Skewed {
                    min: 0.1,
                    max: 20.0,
                    factor: 0.3,
                },
            )
            .with_unit(" Hz"),
            lfo_depth: FloatParam::new(
                "LFO Depth",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            lfo_waveform: IntParam::new(
                "LFO Waveform",
                0, // 0 = Sine, 1 = Triangle, 2 = Square, 3 = Saw, 4 = Random
                IntRange::Linear { min: 0, max: 4 },
            ),
            lfo_destination: IntParam::new(
                "LFO Destination",
                0, // 0 = Off, 1 = Pitch, 2 = Filter, 3 = Amplitude
                IntRange::Linear { min: 0, max: 3 },
            ),

            osc1_waveform: IntParam::new(
                "OSC1 Waveform",
                2, // Default: Saw
                IntRange::Linear { min: 0, max: 4 },
            ),
            osc1_level: FloatParam::new(
                "OSC1 Level",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc1_phase: FloatParam::new(
                "OSC1 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc1_pan: FloatParam::new(
                "OSC1 Pan",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            ),

            osc2_waveform: IntParam::new(
                "OSC2 Waveform",
                2,
                IntRange::Linear { min: 0, max: 4 },
            ),
            osc2_level: FloatParam::new(
                "OSC2 Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc2_semitones: IntParam::new(
                "OSC2 Semitones",
                0,
                IntRange::Linear { min: -24, max: 24 },
            ),
            osc2_cents: IntParam::new(
                "OSC2 Cents",
                0,
                IntRange::Linear { min: -100, max: 100 },
            ),
            osc2_phase: FloatParam::new(
                "OSC2 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc2_pan: FloatParam::new(
                "OSC2 Pan",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            ),

            osc3_waveform: IntParam::new(
                "OSC3 Waveform",
                2,
                IntRange::Linear { min: 0, max: 4 },
            ),
            osc3_level: FloatParam::new(
                "OSC3 Level",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc3_semitones: IntParam::new(
                "OSC3 Semitones",
                0,
                IntRange::Linear { min: -24, max: 24 },
            ),
            osc3_cents: IntParam::new(
                "OSC3 Cents",
                0,
                IntRange::Linear { min: -100, max: 100 },
            ),
            osc3_phase: FloatParam::new(
                "OSC3 Phase",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            osc3_pan: FloatParam::new(
                "OSC3 Pan",
                0.0,
                FloatRange::Linear { min: -1.0, max: 1.0 },
            ),

            portamento: FloatParam::new(
                "Portamento",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 2.0,
                    factor: 0.3,
                },
            )
            .with_unit(" s"),
            pitch_bend_range: IntParam::new(
                "Pitch Bend Range",
                2,
                IntRange::Linear { min: 1, max: 24 },
            ),
            stereo_width: FloatParam::new(
                "Stereo Width",
                1.0,
                FloatRange::Linear { min: 0.0, max: 2.0 },
            ),
            master_volume: FloatParam::new(
                "Master Volume",
                0.7,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            polyphony_mode: IntParam::new(
                "Polyphony Mode",
                1, // 0 = Mono, 1 = Poly
                IntRange::Linear { min: 0, max: 1 },
            ),

            delay_enabled: BoolParam::new("Delay Enabled", false),
            delay_time: FloatParam::new(
                "Delay Time",
                0.375,
                FloatRange::Linear { min: 0.05, max: 1.0 },
            )
            .with_unit(" s"),
            delay_feedback: FloatParam::new(
                "Delay Feedback",
                0.4,
                FloatRange::Linear { min: 0.0, max: 0.9 },
            ),
            delay_mix: FloatParam::new(
                "Delay Mix",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            reverb_enabled: BoolParam::new("Reverb Enabled", false),
            reverb_room_size: FloatParam::new(
                "Reverb Room Size",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_damping: FloatParam::new(
                "Reverb Damping",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            reverb_mix: FloatParam::new(
                "Reverb Mix",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

            chorus_enabled: BoolParam::new("Chorus Enabled", false),
            chorus_rate: FloatParam::new(
                "Chorus Rate",
                1.0,
                FloatRange::Linear { min: 0.1, max: 5.0 },
            )
            .with_unit(" Hz"),
            chorus_depth: FloatParam::new(
                "Chorus Depth",
                3.0,
                FloatRange::Linear { min: 0.0, max: 10.0 },
            )
            .with_unit(" ms"),
            chorus_mix: FloatParam::new(
                "Chorus Mix",
                0.3,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
        }
    }
}

impl Default for RustInSynthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin for RustInSynthPlugin {
    const NAME: &'static str = "RustInSynth";
    const VENDOR: &'static str = "Nasker";
    const URL: &'static str = "https://github.com/oscarmartinez/RustInSynth";
    const EMAIL: &'static str = "oscar@rtp.com";

    const VERSION: &'static str = "1.0.1";

    // Standard stereo input/output
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: std::num::NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames {
            main_input: None,
            main_output: Some("Stereo Output"),
            aux_inputs: &[],
            aux_outputs: &[],
            layout: None,
        },
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        crate::plugin_gui::editor::create_editor(
            self.editor_state.clone(),
            self.params.clone(),
            self.gui_shared.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sample_rate = buffer_config.sample_rate as u32;
        self.sample_rate = buffer_config.sample_rate;
        self.voice_manager.set_sample_rate(sample_rate);
        self.effects_chain.set_sample_rate(sample_rate);

        // Sync all parameters to ensure envelope sustain and other values are applied
        // before the first audio callback. This fixes the issue where the plugin
        // envelope would never sustain because parameters weren't synced at init.
        self.sync_plugin_params_safe();

        // Initialize oscillator waveforms here (safe to allocate outside audio thread)
        let w1 = WaveformType::from_index(self.params.osc1_waveform.value() as u8);
        let w2 = WaveformType::from_index(self.params.osc2_waveform.value() as u8);
        let w3 = WaveformType::from_index(self.params.osc3_waveform.value() as u8);
        self.voice_manager.set_osc_waveform(1, w1);
        self.voice_manager.set_osc_waveform(2, w2);
        self.voice_manager.set_osc_waveform(3, w3);

        // Track current waveforms
        self.last_osc1_waveform = self.params.osc1_waveform.value();
        self.last_osc2_waveform = self.params.osc2_waveform.value();
        self.last_osc3_waveform = self.params.osc3_waveform.value();

        true
    }

    fn reset(&mut self) {
        self.voice_manager.reset();
        self.effects_chain.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // 1. Sync non-allocating parameter values from host controls
        self.sync_plugin_params_safe();

        // 2. Process incoming MIDI notes and events
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    self.voice_manager.receive_event(SynthEvent::note_on(note, velocity));
                }
                NoteEvent::NoteOff { note, .. } => {
                    self.voice_manager.receive_event(SynthEvent::note_off(note));
                }
                NoteEvent::MidiPitchBend { value, .. } => {
                    // NIH-plug provides pitch bend in [0.0, 1.0], center = 0.5
                    let scaled_bend = (value - 0.5) * 2.0; // [-1.0, 1.0]
                    self.voice_manager.receive_event(SynthEvent::pitch_bend(scaled_bend));
                }
                NoteEvent::MidiCC { cc, value, .. } => {
                    // value is 0.0 to 1.0 float in NIH-plug
                    let cc_val = (value * 127.0).round() as u8;
                    self.voice_manager.receive_event(SynthEvent::control_change(cc, cc_val));
                }
                _ => {}
            }
        }

        // 3. Synthesize and process audio block (timed for CPU-load estimate)
        let num_samples = buffer.samples();
        let block_start = std::time::Instant::now();
        {
            let outputs = buffer.as_slice();
            for i in 0..num_samples {
                // Generate raw stereo sample from the active voices
                let stereo_raw = self.voice_manager.next_sample_stereo();
                // Process it through our stereo effects chain (delay, reverb, chorus)
                let processed = self.effects_chain.process_stereo(stereo_raw);

                // Output to the DAW buffer
                outputs[0][i] = processed.left;
                outputs[1][i] = processed.right;
            }
        }

        // 4. Estimate CPU load = (time spent) / (audio time available) * 100
        if self.sample_rate > 0.0 && num_samples > 0 {
            let elapsed = block_start.elapsed().as_secs_f32();
            let budget = num_samples as f32 / self.sample_rate;
            if budget > 0.0 {
                let load = (elapsed / budget * 100.0).min(100.0);
                self.gui_shared.set_cpu_load(load);
            }
        }

        // 5. Publish voice counts for the GUI meter
        self.gui_shared.set_voice_count(self.voice_manager.active_voice_count());
        self.gui_shared.set_max_voices(self.voice_manager.max_voices());

        ProcessStatus::Normal
    }
}

impl RustInSynthPlugin {
    /// Create a new instance of the NIH-plug wrapper
    pub fn new() -> Self {
        let sample_rate = 44100u32;
        Self {
            params: Arc::new(RustInSynthParams::default()),
            voice_manager: VoiceManager::polyphonic(sample_rate),
            effects_chain: EffectsChain::new(sample_rate),
            last_osc1_waveform: 0,
            last_osc2_waveform: 0,
            last_osc3_waveform: 0,
            editor_state: EguiState::from_size(EDITOR_WIDTH, EDITOR_HEIGHT),
            gui_shared: PluginSharedState::new(),
            sample_rate: sample_rate as f32,
        }
    }

    /// Sync parameters that are safe to change in the audio thread (no allocations)
    fn sync_plugin_params_safe(&mut self) {
        // Envelopes
        self.voice_manager.set_attack(self.params.attack.value());
        self.voice_manager.set_decay(self.params.decay.value());
        self.voice_manager.set_sustain(self.params.sustain.value());
        self.voice_manager.set_release(self.params.release.value());

        self.voice_manager.set_filter_attack(self.params.f_attack.value());
        self.voice_manager.set_filter_decay(self.params.f_decay.value());
        self.voice_manager.set_filter_sustain(self.params.f_sustain.value());
        self.voice_manager.set_filter_release(self.params.f_release.value());
        self.voice_manager.set_filter_env_amount(self.params.f_amount.value());

        // Filter
        self.voice_manager.set_filter_cutoff(self.params.cutoff.value());
        self.voice_manager.set_filter_resonance(self.params.resonance.value());

        // LFO
        self.voice_manager.set_lfo_rate(self.params.lfo_rate.value());
        self.voice_manager.set_lfo_depth(self.params.lfo_depth.value());

        let lfo_waveform = match self.params.lfo_waveform.value() {
            0 => LfoWaveform::Sine,
            1 => LfoWaveform::Triangle,
            2 => LfoWaveform::Square,
            3 => LfoWaveform::Saw,
            _ => LfoWaveform::Random,
        };
        self.voice_manager.set_lfo_waveform(lfo_waveform);

        let lfo_dest = match self.params.lfo_destination.value() {
            1 => LfoDestination::Pitch,
            2 => LfoDestination::FilterCutoff,
            3 => LfoDestination::Amplitude,
            _ => LfoDestination::Off,
        };
        self.voice_manager.set_lfo_destination(lfo_dest);

        // Oscillator levels, detune, phase, pan (no allocations)
        self.voice_manager.set_osc_level(1, self.params.osc1_level.value());
        self.voice_manager.set_osc_level(2, self.params.osc2_level.value());
        self.voice_manager.set_osc_level(3, self.params.osc3_level.value());

        self.voice_manager.set_osc_semitones(2, self.params.osc2_semitones.value() as i8);
        self.voice_manager.set_osc_cents(2, self.params.osc2_cents.value() as i8);
        self.voice_manager.set_osc_semitones(3, self.params.osc3_semitones.value() as i8);
        self.voice_manager.set_osc_cents(3, self.params.osc3_cents.value() as i8);

        self.voice_manager.set_osc_phase(1, self.params.osc1_phase.value());
        self.voice_manager.set_osc_phase(2, self.params.osc2_phase.value());
        self.voice_manager.set_osc_phase(3, self.params.osc3_phase.value());

        self.voice_manager.set_osc_pan(1, self.params.osc1_pan.value());
        self.voice_manager.set_osc_pan(2, self.params.osc2_pan.value());
        self.voice_manager.set_osc_pan(3, self.params.osc3_pan.value());

        // Master controls
        self.voice_manager.set_portamento_time(self.params.portamento.value());
        self.voice_manager.set_pitch_bend_range(self.params.pitch_bend_range.value() as u8);
        self.voice_manager.set_stereo_width(self.params.stereo_width.value());
        self.voice_manager.set_master_volume(self.params.master_volume.value());

        let mode = if self.params.polyphony_mode.value() == 0 {
            PolyphonyMode::Mono
        } else {
            PolyphonyMode::Poly
        };
        self.voice_manager.set_polyphony_mode(mode);

        // Effects
        self.effects_chain.set_delay_enabled(self.params.delay_enabled.value());
        self.effects_chain.set_delay_time(self.params.delay_time.value());
        self.effects_chain.set_delay_feedback(self.params.delay_feedback.value());
        self.effects_chain.set_delay_mix(self.params.delay_mix.value());

        self.effects_chain.set_reverb_enabled(self.params.reverb_enabled.value());
        self.effects_chain.set_reverb_room_size(self.params.reverb_room_size.value());
        self.effects_chain.set_reverb_damping(self.params.reverb_damping.value());
        self.effects_chain.set_reverb_mix(self.params.reverb_mix.value());

        self.effects_chain.set_chorus_enabled(self.params.chorus_enabled.value());
        self.effects_chain.set_chorus_rate(self.params.chorus_rate.value());
        self.effects_chain.set_chorus_depth(self.params.chorus_depth.value());
        self.effects_chain.set_chorus_mix(self.params.chorus_mix.value());
    }

    /// Sync the host's active parameter values into the DSP engine
    /// WARNING: This allocates memory and should NOT be called from the audio thread!
    #[allow(dead_code)]
    fn sync_plugin_params(&mut self) {
        // Envelopes
        self.voice_manager.set_attack(self.params.attack.value());
        self.voice_manager.set_decay(self.params.decay.value());
        self.voice_manager.set_sustain(self.params.sustain.value());
        self.voice_manager.set_release(self.params.release.value());

        self.voice_manager.set_filter_attack(self.params.f_attack.value());
        self.voice_manager.set_filter_decay(self.params.f_decay.value());
        self.voice_manager.set_filter_sustain(self.params.f_sustain.value());
        self.voice_manager.set_filter_release(self.params.f_release.value());
        self.voice_manager.set_filter_env_amount(self.params.f_amount.value());

        // Filter
        self.voice_manager.set_filter_cutoff(self.params.cutoff.value());
        self.voice_manager.set_filter_resonance(self.params.resonance.value());

        // LFO
        self.voice_manager.set_lfo_rate(self.params.lfo_rate.value());
        self.voice_manager.set_lfo_depth(self.params.lfo_depth.value());

        let lfo_waveform = match self.params.lfo_waveform.value() {
            0 => LfoWaveform::Sine,
            1 => LfoWaveform::Triangle,
            2 => LfoWaveform::Square,
            3 => LfoWaveform::Saw,
            _ => LfoWaveform::Random,
        };
        self.voice_manager.set_lfo_waveform(lfo_waveform);

        let lfo_dest = match self.params.lfo_destination.value() {
            1 => LfoDestination::Pitch,
            2 => LfoDestination::FilterCutoff,
            3 => LfoDestination::Amplitude,
            _ => LfoDestination::Off,
        };
        self.voice_manager.set_lfo_destination(lfo_dest);

        // Oscillators
        let w1 = WaveformType::from_index(self.params.osc1_waveform.value() as u8);
        let w2 = WaveformType::from_index(self.params.osc2_waveform.value() as u8);
        let w3 = WaveformType::from_index(self.params.osc3_waveform.value() as u8);
        self.voice_manager.set_osc_waveform(1, w1);
        self.voice_manager.set_osc_waveform(2, w2);
        self.voice_manager.set_osc_waveform(3, w3);

        self.voice_manager.set_osc_level(1, self.params.osc1_level.value());
        self.voice_manager.set_osc_level(2, self.params.osc2_level.value());
        self.voice_manager.set_osc_level(3, self.params.osc3_level.value());

        self.voice_manager.set_osc_semitones(2, self.params.osc2_semitones.value() as i8);
        self.voice_manager.set_osc_cents(2, self.params.osc2_cents.value() as i8);
        self.voice_manager.set_osc_semitones(3, self.params.osc3_semitones.value() as i8);
        self.voice_manager.set_osc_cents(3, self.params.osc3_cents.value() as i8);

        self.voice_manager.set_osc_phase(1, self.params.osc1_phase.value());
        self.voice_manager.set_osc_phase(2, self.params.osc2_phase.value());
        self.voice_manager.set_osc_phase(3, self.params.osc3_phase.value());

        self.voice_manager.set_osc_pan(1, self.params.osc1_pan.value());
        self.voice_manager.set_osc_pan(2, self.params.osc2_pan.value());
        self.voice_manager.set_osc_pan(3, self.params.osc3_pan.value());

        // Master controls
        self.voice_manager.set_portamento_time(self.params.portamento.value());
        self.voice_manager.set_pitch_bend_range(self.params.pitch_bend_range.value() as u8);
        self.voice_manager.set_stereo_width(self.params.stereo_width.value());
        self.voice_manager.set_master_volume(self.params.master_volume.value());

        let mode = if self.params.polyphony_mode.value() == 0 {
            PolyphonyMode::Mono
        } else {
            PolyphonyMode::Poly
        };
        self.voice_manager.set_polyphony_mode(mode);
    }
}

// Generate CLAP and VST3 entry points for DAWs to load
impl ClapPlugin for RustInSynthPlugin {
    const CLAP_ID: &'static str = "com.nasker.rustinsynth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A warm analog-modeled subtractive synthesizer in Rust");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Synthesizer,
        ClapFeature::Instrument,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for RustInSynthPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"RustInSynthNsker";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Synth,
        Vst3SubCategory::Instrument,
    ];
}

nih_export_clap!(RustInSynthPlugin);
nih_export_vst3!(RustInSynthPlugin);

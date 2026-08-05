use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};

use crate::core::effects::EffectsChain;
use crate::core::event::{SynthEvent, WaveformType, SynthEventReceiver};
use crate::core::lfo::{LfoDestination, LfoWaveform};
use crate::core::param_spec::{self, ParamCurve, ParamSpec};
use crate::core::params::{cc_mapping_with_user_overrides, SynthParam};
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
    // Raw MIDI CC events queued for the editor, which applies them through
    // ParamSetter (host sees automation, GUI reflects the change)
    cc_tx: SyncSender<(u8, u8)>,
    cc_rx: Arc<Mutex<Receiver<(u8, u8)>>>,
    // Whether the editor is open; selects how CCs are applied (see process())
    editor_open: Arc<AtomicBool>,
    // Last parameter values synced into the DSP. Change detection matters:
    // CCs applied directly to the VoiceManager (editor closed) must not be
    // overwritten by a blind params → DSP sync on the next buffer.
    // NaN forces a full sync once.
    last_synced: [f32; param_spec::PARAM_SPECS.len()],
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

/// Build a `FloatParam` from the shared parameter spec. The nih range mirrors
/// the spec's curve (exact for linear, skew-approximated for logarithmic) so
/// host-drawn controls and automation lanes feel like the GUI.
fn float_param(spec: &ParamSpec) -> FloatParam {
    let range = match spec.curve {
        ParamCurve::Linear => FloatRange::Linear {
            min: spec.min,
            max: spec.max,
        },
        _ => FloatRange::Skewed {
            min: spec.min,
            max: spec.max,
            factor: spec.nih_skew_factor(),
        },
    };
    let param = FloatParam::new(spec.param.name(), spec.default, range);
    if spec.unit.is_empty() {
        param
    } else {
        param.with_unit(spec.unit)
    }
}

/// Build an `IntParam` from the shared parameter spec.
fn int_param(spec: &ParamSpec) -> IntParam {
    IntParam::new(
        spec.param.name(),
        spec.default as i32,
        IntRange::Linear {
            min: spec.min as i32,
            max: spec.max as i32,
        },
    )
}

impl Default for RustInSynthParams {
    fn default() -> Self {
        // Every SynthParam-backed field comes straight from the shared spec
        // table — no hand-copied defaults or ranges here.
        Self {
            attack: float_param(param_spec::spec(SynthParam::Attack)),
            decay: float_param(param_spec::spec(SynthParam::Decay)),
            sustain: float_param(param_spec::spec(SynthParam::Sustain)),
            release: float_param(param_spec::spec(SynthParam::Release)),

            cutoff: float_param(param_spec::spec(SynthParam::FilterCutoff)),
            resonance: float_param(param_spec::spec(SynthParam::FilterResonance)),

            f_attack: float_param(param_spec::spec(SynthParam::FilterAttack)),
            f_decay: float_param(param_spec::spec(SynthParam::FilterDecay)),
            f_sustain: float_param(param_spec::spec(SynthParam::FilterSustain)),
            f_release: float_param(param_spec::spec(SynthParam::FilterRelease)),
            f_amount: float_param(param_spec::spec(SynthParam::FilterEnvAmount)),

            lfo_rate: float_param(param_spec::spec(SynthParam::LfoRate)),
            lfo_depth: float_param(param_spec::spec(SynthParam::LfoDepth)),
            lfo_waveform: int_param(param_spec::spec(SynthParam::LfoWaveform)),
            lfo_destination: int_param(param_spec::spec(SynthParam::LfoDestination)),

            osc1_waveform: int_param(param_spec::spec(SynthParam::Osc1Waveform)),
            osc1_level: float_param(param_spec::spec(SynthParam::Osc1Level)),
            osc1_phase: float_param(param_spec::spec(SynthParam::Osc1Phase)),
            osc1_pan: float_param(param_spec::spec(SynthParam::Osc1Pan)),

            osc2_waveform: int_param(param_spec::spec(SynthParam::Osc2Waveform)),
            osc2_level: float_param(param_spec::spec(SynthParam::Osc2Level)),
            osc2_semitones: int_param(param_spec::spec(SynthParam::Osc2Semitones)),
            osc2_cents: int_param(param_spec::spec(SynthParam::Osc2Cents)),
            osc2_phase: float_param(param_spec::spec(SynthParam::Osc2Phase)),
            osc2_pan: float_param(param_spec::spec(SynthParam::Osc2Pan)),

            osc3_waveform: int_param(param_spec::spec(SynthParam::Osc3Waveform)),
            osc3_level: float_param(param_spec::spec(SynthParam::Osc3Level)),
            osc3_semitones: int_param(param_spec::spec(SynthParam::Osc3Semitones)),
            osc3_cents: int_param(param_spec::spec(SynthParam::Osc3Cents)),
            osc3_phase: float_param(param_spec::spec(SynthParam::Osc3Phase)),
            osc3_pan: float_param(param_spec::spec(SynthParam::Osc3Pan)),

            portamento: float_param(param_spec::spec(SynthParam::PortamentoTime)),
            pitch_bend_range: int_param(param_spec::spec(SynthParam::PitchBendRange)),
            stereo_width: float_param(param_spec::spec(SynthParam::StereoWidth)),
            master_volume: float_param(param_spec::spec(SynthParam::MasterVolume)),

            polyphony_mode: IntParam::new(
                "Polyphony Mode",
                0, // 0 = Mono, 1 = Poly (match standalone default)
                IntRange::Linear { min: 0, max: 1 },
            ),

            delay_enabled: BoolParam::new("Delay Enabled", false),
            delay_time: FloatParam::new(
                "Delay Time",
                0.3, // Match EffectsChain default
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
                1.5, // Match Chorus default
                FloatRange::Linear { min: 0.1, max: 5.0 },
            )
            .with_unit(" Hz"),
            chorus_depth: FloatParam::new(
                "Chorus Depth",
                3.0, // Match Chorus default
                FloatRange::Linear { min: 0.0, max: 10.0 },
            )
            .with_unit(" ms"),
            chorus_mix: FloatParam::new(
                "Chorus Mix",
                0.5, // Match Chorus default
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

/// Temporary debug aid: with `RUSTINSYNTH_DEBUG=1` set, log every MIDI event
/// (plus the current sustain param value) to /tmp/rustinsynth_debug.log so
/// plugin-host event streams can be inspected. Event-rate only; the env var
/// check is cached. TODO: remove once the sustain investigation is done.
fn debug_log(msg: &str) {
    use std::sync::OnceLock;
    static ENABLED: OnceLock<bool> = OnceLock::new();
    if !*ENABLED.get_or_init(|| std::env::var_os("RUSTINSYNTH_DEBUG").is_some()) {
        return;
    }
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/rustinsynth_debug.log")
    {
        let _ = writeln!(f, "{}", msg);
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
            Arc::clone(&self.cc_rx),
            Arc::clone(&self.editor_open),
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

        // Force a full params → DSP sync: envelope sustain and other values must
        // be applied before set_sample_rate is called. This is critical because
        // set_sample_rate calls update_increments() on envelopes, which uses the
        // current sustain level. NaN slots in last_synced never compare equal,
        // so the next sync applies every parameter.
        self.last_synced = [f32::NAN; param_spec::PARAM_SPECS.len()];
        self.sync_plugin_params_safe();

        // Now set sample rate after parameters are synced
        self.voice_manager.set_sample_rate(sample_rate);
        self.effects_chain.set_sample_rate(sample_rate);

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
                    debug_log(&format!(
                        "NoteOn  note={:>3} vel={:.3} sustain_param={:.3}",
                        note,
                        velocity,
                        self.params.sustain.value()
                    ));
                    self.voice_manager.receive_event(SynthEvent::note_on(note, velocity));
                }
                NoteEvent::NoteOff { note, .. } => {
                    debug_log(&format!("NoteOff note={:>3}", note));
                    self.voice_manager.receive_event(SynthEvent::note_off(note));
                }
                NoteEvent::MidiPitchBend { value, .. } => {
                    // NIH-plug provides pitch bend in [0.0, 1.0], center = 0.5
                    let scaled_bend = (value - 0.5) * 2.0; // [-1.0, 1.0]
                    self.voice_manager.receive_event(SynthEvent::pitch_bend(scaled_bend));
                }
                NoteEvent::MidiCC { cc, value, .. } => {
                    // value is 0.0 to 1.0 float in NIH-plug
                    debug_log(&format!("CC      cc={:>3} raw={:.4}", cc, value));
                    let cc_val = (value * 127.0).round() as u8;
                    if self.editor_open.load(Ordering::Relaxed) {
                        // Editor open: route through the GUI → ParamSetter so the
                        // host records automation, the GUI knob moves, and the
                        // (change-detecting) sync applies it to the DSP.
                        // Drop on full queue rather than blocking the audio thread.
                        let _ = self.cc_tx.try_send((cc, cc_val));
                    } else {
                        // Editor closed: apply directly to the DSP. The
                        // change-detecting sync will not overwrite it because
                        // the nih parameter value itself did not change.
                        self.voice_manager.receive_event(SynthEvent::control_change(cc, cc_val));
                    }
                }
                other => {
                    debug_log(&format!("Other   {:?}", std::mem::discriminant(&other)));
                }
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
        let params = Arc::new(RustInSynthParams::default());
        let mut voice_manager = VoiceManager::monophonic(sample_rate);
        // CC → parameter resolution identical to the standalone: factory
        // defaults overlaid with the user's learned mappings
        *voice_manager.cc_mapping_mut() = cc_mapping_with_user_overrides();
        let effects_chain = EffectsChain::new(sample_rate);

        let (cc_tx, cc_rx) = sync_channel(256);

        // Create plugin instance
        let mut plugin = Self {
            params: Arc::clone(&params),
            voice_manager,
            effects_chain,
            last_osc1_waveform: 0,
            last_osc2_waveform: 0,
            last_osc3_waveform: 0,
            editor_state: EguiState::from_size(EDITOR_WIDTH, EDITOR_HEIGHT),
            gui_shared: PluginSharedState::new(),
            sample_rate: sample_rate as f32,
            cc_tx,
            cc_rx: Arc::new(Mutex::new(cc_rx)),
            editor_open: Arc::new(AtomicBool::new(false)),
            last_synced: [f32::NAN; param_spec::PARAM_SPECS.len()],
        };

        // Sync parameters immediately to ensure voice_manager has correct initial values
        // This is critical for sustain and other envelope parameters
        plugin.sync_plugin_params_safe();

        plugin
    }

    /// Sync parameters that are safe to change in the audio thread (no allocations).
    ///
    /// Change-detecting: a parameter is only applied to the DSP when its value
    /// actually changed since the last sync. This is what allows MIDI CCs to be
    /// applied directly to the VoiceManager while the editor is closed without
    /// the next buffer's sync stomping them — the nih parameter value did not
    /// change in that case, so nothing is re-applied.
    fn sync_plugin_params_safe(&mut self) {
        /// Apply a FloatParam to the DSP only when its value changed.
        macro_rules! sync_f32 {
            ($param:expr, $field:ident, $apply:expr) => {{
                let v = self.params.$field.value();
                let i = param_spec::index($param);
                if v != self.last_synced[i] {
                    let apply: fn(&mut VoiceManager, f32) = $apply;
                    apply(&mut self.voice_manager, v);
                    self.last_synced[i] = v;
                }
            }};
        }
        /// Same for IntParams (stored as f32 in last_synced).
        macro_rules! sync_int {
            ($param:expr, $field:ident, $apply:expr) => {{
                let v = self.params.$field.value();
                let i = param_spec::index($param);
                if v as f32 != self.last_synced[i] {
                    let apply: fn(&mut VoiceManager, i32) = $apply;
                    apply(&mut self.voice_manager, v);
                    self.last_synced[i] = v as f32;
                }
            }};
        }

        // Amp envelope
        sync_f32!(SynthParam::Attack, attack, |vm, v| vm.set_attack(v));
        sync_f32!(SynthParam::Decay, decay, |vm, v| vm.set_decay(v));
        sync_f32!(SynthParam::Sustain, sustain, |vm, v| vm.set_sustain(v));
        sync_f32!(SynthParam::Release, release, |vm, v| vm.set_release(v));

        // Filter envelope
        sync_f32!(SynthParam::FilterAttack, f_attack, |vm, v| vm.set_filter_attack(v));
        sync_f32!(SynthParam::FilterDecay, f_decay, |vm, v| vm.set_filter_decay(v));
        sync_f32!(SynthParam::FilterSustain, f_sustain, |vm, v| vm.set_filter_sustain(v));
        sync_f32!(SynthParam::FilterRelease, f_release, |vm, v| vm.set_filter_release(v));
        sync_f32!(SynthParam::FilterEnvAmount, f_amount, |vm, v| vm.set_filter_env_amount(v));

        // Filter
        sync_f32!(SynthParam::FilterCutoff, cutoff, |vm, v| vm.set_filter_cutoff(v));
        sync_f32!(SynthParam::FilterResonance, resonance, |vm, v| vm.set_filter_resonance(v));

        // LFO
        sync_f32!(SynthParam::LfoRate, lfo_rate, |vm, v| vm.set_lfo_rate(v));
        sync_f32!(SynthParam::LfoDepth, lfo_depth, |vm, v| vm.set_lfo_depth(v));
        sync_int!(SynthParam::LfoWaveform, lfo_waveform, |vm, v| {
            vm.set_lfo_waveform(match v {
                0 => LfoWaveform::Sine,
                1 => LfoWaveform::Triangle,
                2 => LfoWaveform::Square,
                3 => LfoWaveform::Saw,
                _ => LfoWaveform::Random,
            })
        });
        sync_int!(SynthParam::LfoDestination, lfo_destination, |vm, v| {
            vm.set_lfo_destination(match v {
                1 => LfoDestination::Pitch,
                2 => LfoDestination::FilterCutoff,
                3 => LfoDestination::Amplitude,
                _ => LfoDestination::Off,
            })
        });

        // Oscillators
        sync_int!(SynthParam::Osc1Waveform, osc1_waveform, |vm, v| {
            vm.set_osc_waveform(1, WaveformType::from_index(v as u8))
        });
        sync_int!(SynthParam::Osc2Waveform, osc2_waveform, |vm, v| {
            vm.set_osc_waveform(2, WaveformType::from_index(v as u8))
        });
        sync_int!(SynthParam::Osc3Waveform, osc3_waveform, |vm, v| {
            vm.set_osc_waveform(3, WaveformType::from_index(v as u8))
        });

        sync_f32!(SynthParam::Osc1Level, osc1_level, |vm, v| vm.set_osc_level(1, v));
        sync_f32!(SynthParam::Osc2Level, osc2_level, |vm, v| vm.set_osc_level(2, v));
        sync_f32!(SynthParam::Osc3Level, osc3_level, |vm, v| vm.set_osc_level(3, v));

        sync_int!(SynthParam::Osc2Semitones, osc2_semitones, |vm, v| vm.set_osc_semitones(2, v as i8));
        sync_int!(SynthParam::Osc2Cents, osc2_cents, |vm, v| vm.set_osc_cents(2, v as i8));
        sync_int!(SynthParam::Osc3Semitones, osc3_semitones, |vm, v| vm.set_osc_semitones(3, v as i8));
        sync_int!(SynthParam::Osc3Cents, osc3_cents, |vm, v| vm.set_osc_cents(3, v as i8));

        sync_f32!(SynthParam::Osc1Phase, osc1_phase, |vm, v| vm.set_osc_phase(1, v));
        sync_f32!(SynthParam::Osc2Phase, osc2_phase, |vm, v| vm.set_osc_phase(2, v));
        sync_f32!(SynthParam::Osc3Phase, osc3_phase, |vm, v| vm.set_osc_phase(3, v));

        sync_f32!(SynthParam::Osc1Pan, osc1_pan, |vm, v| vm.set_osc_pan(1, v));
        sync_f32!(SynthParam::Osc2Pan, osc2_pan, |vm, v| vm.set_osc_pan(2, v));
        sync_f32!(SynthParam::Osc3Pan, osc3_pan, |vm, v| vm.set_osc_pan(3, v));

        // Master controls
        sync_f32!(SynthParam::PortamentoTime, portamento, |vm, v| vm.set_portamento_time(v));
        sync_int!(SynthParam::PitchBendRange, pitch_bend_range, |vm, v| vm.set_pitch_bend_range(v as u8));
        sync_f32!(SynthParam::StereoWidth, stereo_width, |vm, v| vm.set_stereo_width(v));
        sync_f32!(SynthParam::MasterVolume, master_volume, |vm, v| vm.set_master_volume(v));

        // Not CC-mapped and not directly applied elsewhere: unconditional is
        // fine (and cheap) here.
        let mode = if self.params.polyphony_mode.value() == 0 {
            PolyphonyMode::Mono
        } else {
            PolyphonyMode::Poly
        };
        self.voice_manager.set_polyphony_mode(mode);

        // Effects (not CC-mapped; always applied)
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

        // Keep the legacy waveform trackers in sync for initialize()
        self.last_osc1_waveform = self.params.osc1_waveform.value();
        self.last_osc2_waveform = self.params.osc2_waveform.value();
        self.last_osc3_waveform = self.params.osc3_waveform.value();
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

#[cfg(test)]
mod parity_tests {
    use super::*;
    use crate::core::param_spec;
    use crate::gui::ParamBank;

    /// Every SynthParam-backed nih parameter id, paired with its SynthParam.
    /// If a field is added/renamed in `RustInSynthParams`, this list (and the
    /// spec table) must be updated — the tests below enforce consistency.
    const EXPECTED: &[(SynthParam, &str)] = &[
        (SynthParam::Attack, "attack"),
        (SynthParam::Decay, "decay"),
        (SynthParam::Sustain, "sustain"),
        (SynthParam::Release, "release"),
        (SynthParam::FilterCutoff, "cutoff"),
        (SynthParam::FilterResonance, "resonance"),
        (SynthParam::FilterAttack, "f_attack"),
        (SynthParam::FilterDecay, "f_decay"),
        (SynthParam::FilterSustain, "f_sustain"),
        (SynthParam::FilterRelease, "f_release"),
        (SynthParam::FilterEnvAmount, "f_amount"),
        (SynthParam::LfoRate, "lfo_rate"),
        (SynthParam::LfoDepth, "lfo_depth"),
        (SynthParam::LfoWaveform, "lfo_waveform"),
        (SynthParam::LfoDestination, "lfo_destination"),
        (SynthParam::PitchBendRange, "pitch_bend_range"),
        (SynthParam::PortamentoTime, "portamento"),
        (SynthParam::Osc1Waveform, "osc1_waveform"),
        (SynthParam::Osc1Level, "osc1_level"),
        (SynthParam::Osc1Phase, "osc1_phase"),
        (SynthParam::Osc1Pan, "osc1_pan"),
        (SynthParam::Osc2Waveform, "osc2_waveform"),
        (SynthParam::Osc2Level, "osc2_level"),
        (SynthParam::Osc2Semitones, "osc2_semitones"),
        (SynthParam::Osc2Cents, "osc2_cents"),
        (SynthParam::Osc2Phase, "osc2_phase"),
        (SynthParam::Osc2Pan, "osc2_pan"),
        (SynthParam::Osc3Waveform, "osc3_waveform"),
        (SynthParam::Osc3Level, "osc3_level"),
        (SynthParam::Osc3Semitones, "osc3_semitones"),
        (SynthParam::Osc3Cents, "osc3_cents"),
        (SynthParam::Osc3Phase, "osc3_phase"),
        (SynthParam::Osc3Pan, "osc3_pan"),
        (SynthParam::StereoWidth, "stereo_width"),
        (SynthParam::MasterVolume, "master_volume"),
    ];

    #[test]
    fn every_spec_param_has_plugin_param() {
        assert_eq!(EXPECTED.len(), param_spec::PARAM_SPECS.len());
        let params = RustInSynthParams::default();
        let map = params.param_map();
        for (sp, id) in EXPECTED {
            assert!(
                map.iter().any(|(pid, _, _)| pid == id),
                "missing plugin param {:?} (id {})",
                sp,
                id
            );
        }
    }

    #[test]
    fn plugin_defaults_match_spec() {
        let params = RustInSynthParams::default();
        let map = params.param_map();
        for (sp, id) in EXPECTED {
            let (_, ptr, _) = map
                .iter()
                .find(|(pid, _, _)| pid == id)
                .expect("param must exist");
            let plugin_default = unsafe { ptr.default_plain_value() };
            let spec_default = param_spec::spec(*sp).default;
            assert!(
                (plugin_default - spec_default).abs() < 1e-6,
                "{:?}: plugin default {} != spec default {}",
                sp,
                plugin_default,
                spec_default
            );
        }
    }

    #[test]
    fn plugin_ranges_match_spec() {
        let params = RustInSynthParams::default();
        let map = params.param_map();
        for (sp, id) in EXPECTED {
            let s = param_spec::spec(*sp);
            let (_, ptr, _) = map
                .iter()
                .find(|(pid, _, _)| pid == id)
                .expect("param must exist");
            let lo = unsafe { ptr.preview_plain(0.0) };
            let hi = unsafe { ptr.preview_plain(1.0) };
            assert!(
                (lo - s.min).abs() < 0.51,
                "{:?}: plugin range min {} != spec min {}",
                sp,
                lo,
                s.min
            );
            assert!(
                (hi - s.max).abs() < 0.51,
                "{:?}: plugin range max {} != spec max {}",
                sp,
                hi,
                s.max
            );
        }
    }

    #[test]
    fn param_bank_defaults_match_spec() {
        let bank = ParamBank::new();
        for &p in SynthParam::all() {
            let expected = param_spec::spec(p).default;
            assert_eq!(
                bank.get(p),
                expected,
                "ParamBank default for {:?} differs from spec",
                p
            );
        }
    }
}

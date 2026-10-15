# RustInSynth Architecture

**Version: post-1.2.0 (unreleased; `Cargo.toml` still reads 1.2.0)**

## Overview

RustInSynth is a real-time polyphonic synthesizer with a GUI, designed around lock-free communication between the audio thread and the UI thread. It supports both standalone and plugin (VST3/CLAP) operation through a unified `SynthBackend` trait. Sound shaping is built around a 4-oscillator voice, two serial filters and a 16-slot modulation matrix.

```
┌─────────────────────────────────────────────────────────────────┐
│                         GUI Thread                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   egui      │───▶│  panels::*  │───▶│  ParamBank          │  │
│  │  (render)   │    │  (shared UI)│    │  (AtomicU32 array)  │  │
│  └─────────────┘    └──────┬──────┘    │  + Mutex<ModState>  │  │
│                            │           └──────────┬──────────┘  │
└────────────────────────────┼──────────────────────┼─────────────┘
                             │                      │ lock-free / short lock
┌────────────────────────────┼──────────────────────┼─────────────┐
│                       Audio Thread                │              │
│  ┌─────────────┐    ┌──────▼──────┐    ┌─────────▼──────────┐   │
│  │   cpal      │◀───│AudioEngine  │◀───│  VoiceManager      │   │
│  │  (output)   │    │ + Effects   │    │  (mono/poly DSP)   │   │
│  └─────────────┘    └─────────────┘    └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Signal Flow (per voice)

```
OSC1 ─┐
OSC2 ─┼─ level + pan ─▶ Filter 1 ─▶ Filter 2 (optional) ─▶ Amp env ─┐
OSC3 ─┤   (L/R pairs)   SVF/Ladder/Comb  SVF/Ladder/Comb            │
OSC4 ─┘                                                              ▼
 │                                              sum voices ─▶ width ─▶ EffectsChain
 └─ previous-sample output feeds ModSource::Osc4                      Dist → Chorus → Delay → Reverb
```

Modulation offsets from the matrix are applied on top of the base parameter values at the points above (pitch, level, pan, cutoff, resonance, amplitude, width).

## Core Components

### 1. GUI Layer (`src/gui/`)

#### `mod.rs` - Shared State
- **`ParamBank`**: Array of `AtomicU32` storing all synth parameters (as f32 bits); size and defaults come from `param_spec::PARAM_SPECS`
- **`SharedState`**: `ParamBank` + `DashMap` for MIDI CC feedback + CPU load atomic + `Arc<Mutex<ModState>>` for the modulation state (matrix, LFO 1-3 sync, Env 3/4, BPM, delay sync, filter type)
- Lock-free reads/writes of parameters using `Ordering::Relaxed`

#### `app.rs` - Main Application (Standalone)
- **`SynthApp`**: Owns `Box<dyn SynthBackend>` for backend-agnostic GUI
- Delegates to shared `panels::*` functions for all UI rendering
- `run_gui()`: Entry point for standalone application with X11/Wayland handling

#### `backend.rs` - Unified Backend Trait
- **`SynthBackend`**: Core trait abstracting all parameter access
- Methods for: parameters, effects (distortion/delay/reverb/chorus), voice management, stereo, MIDI, CPU load, **`mod_state()` / `set_mod_state()`**, and `begin_param_change`/`end_param_change` host gestures
- Implemented by both `StandaloneBackend` and `PluginBackend<'a>`

#### `backend_standalone.rs` - Standalone Implementation
- **`StandaloneBackend`**: Owns `AudioEngine`, `SharedState`, `MidiInputHandler`
- MIDI CC learn with persistence to `~/.rustinsynth/cc_mappings.json`
- Polls MIDI and syncs `ParamBank` and `ModState` → `VoiceManager` every frame

#### `panels.rs` - Shared UI
- `shell()` lays out the window as a tab strip: **SYNTH | ROUTING MATRIX | PRESETS** (`RightTab`)
- SYNTH page: oscillators (4), filter, filter 2, envelopes (amp/filter + Env 3/4), LFOs 1-3, tempo, pitch/voice, effects (incl. distortion), master, MIDI
- ROUTING MATRIX page: `routing_grid` over the 16 slots (source/destination/amount/enable)
- PRESETS page: category shelves, preset cards, save/load (`PresetState`)
- `apply_preset()` / `create_preset()` translate between `Preset` and the backend
- Used unchanged by standalone and plugin editors

#### `widgets.rs` - Custom Controls
- Curve-aware `param_slider` (log/linear/discrete from the param spec; fires begin/end gestures)
- Rotary knobs, toggle/selector switches, VU meter, MIDI indicator

#### `theme.rs` - Centralized Styling
- `SynthTheme` struct with all UI colors; global `THEME` constant

### 2. Audio Layer (`src/audio/`)

#### `engine.rs` - Audio Engine
- Creates `cpal` output stream
- Owns `Arc<Mutex<VoiceManager>>` and the `EffectsChain` shared with the audio callback
- `sync_params()`: Reads `ParamBank` and applies to `VoiceManager`
- `set_mod_state()`: Pushes the `ModState` (matrix, LFO sync, Env 3/4, BPM, delay sync) to the voices and the effects chain
- `send_event()`: Forwards MIDI events (notes, CC, pitch bend, aftertouch) to `VoiceManager`
- **CPU load measurement**: `process_time / available_time` per buffer, smoothed (0.9/0.1)

### 3. Core DSP (`src/core/`)

#### `voice.rs` - Voice Manager
- **Polyphonic mode**: 8 voices with voice stealing, or monophonic with key stacking
- Per voice: `OscillatorBank` (4 oscs), two L/R filter pairs (`Box<dyn Filter>`), amp + filter `ADSREnvelope`, Env 3/4, 3 LFOs
- **`ModState`**: matrix + `LfoSettings` ×3 (rate/depth/waveform/sync/beats) + Env 3/4 ADSR + BPM + delay sync + filter type
- **`VoiceMods`**: per-sample accumulation of matrix offsets per destination, built from `ModSourceValues`
- **Legacy LFO destination** (`LfoDestination`) is mapped into a matrix slot (`legacy_lfo_slot`) so old presets keep working
- **Key Stack**, **Portamento** (exponential 0.005-2.0 s), `SynthEventReceiver` for note/CC/aftertouch handling
- `next_sample_stereo()`: stereo output with per-oscillator panning

#### `mod_matrix.rs` - Modulation Matrix
- `ModSource`: Lfo1-3, AmpEnv, FilterEnv, Env3, Env4, Velocity, KeyTrack, ModWheel, Aftertouch, PitchBend, Osc4 (audio rate), Constant; each flagged unipolar/bipolar
- `ModDest`: pitch (all + per-oscillator), filter 1/2 cutoff and resonance, amplitude, stereo width, osc levels and pans, effect parameters, LFO rate/depth (mod-on-mod), Osc4 level
- `ModSlot { source, dest, amount (-1..1), enabled }`; `ModMatrix` holds `MOD_SLOTS = 16`
- `dest_range()` defines the physical range of `amount = ±1` per destination (e.g. cutoff ±5 octaves, pitch ±12 semitones)
- `is_wired()` / `is_per_voice()` describe what the DSP applies. **Global effect destinations (delay/reverb/chorus targets) are not yet applied.**
- Matrix state lives in `ModState`/`Preset`, not in `SynthParam` (16 slots × 3 fields would bloat the flat param list), so it is not host-automatable

#### `oscillator.rs` - Oscillator Bank
- 4 independent oscillators: 5 waveforms, level, phase, detune (semitones + cents), pan
- PolyBLEP band-limiting
- OSC 4 defaults to silent; its raw output (previous sample) is exposed as `ModSource::Osc4` for FM (→ pitch dests) and ring mod (→ level dests)
- Per-sample pitch modulation input on `OscillatorUnit`

#### `filter.rs` - Filters
- `Filter` trait; implementations: `SVFilter` (LP/HP/BP/Notch via `FilterMode`), `LadderFilter` (4-pole, 24 dB/oct, saturating), `CombFilter`
- `FilterType { Svf, Ladder, Comb }` + `make_filter()` factory; changing type rebuilds the filter and clears state
- Separate L/R instances per voice; Filter 2 is a second serial instance with its own type/cutoff/resonance
- Cutoff 20 Hz - 20 kHz (exponential); filter envelope uses a 6-octave musical range

#### `envelope.rs` - ADSR Envelope
- Attack/Decay/Sustain/Release with exponential curves; click-free retrigger
- Used for amp, filter, Env 3 and Env 4

#### `lfo.rs` - Low Frequency Oscillator
- 5 waveforms (Sine, Triangle, Square, Saw, Random); 3 instances per voice
- Rate in Hz or tempo-synced (`LfoSettings::sync` + `rate_beats`, resolved by `lfo_rate_hz(settings, bpm)`)

#### `effects.rs` - Effects
- `StereoEffect` trait; `Distortion`, `Chorus`, `Delay`, `Reverb`
- **Distortion**: `DistortionType { Tube, Fuzz, Bitcrush, Fold }`, drive + mix, level-compensated
- **Delay**: ping-pong, optional tempo sync (`set_bpm` + `set_tempo_sync(enabled, beats)`)
- **Reverb**: Schroeder (4 comb + 2 allpass per channel, different L/R times)
- **Chorus**: LFO-modulated delay with L/R phase offset
- `EffectsChain` processes in fixed order **Distortion → Chorus → Delay → Reverb**, each with an enable flag and setter/getter wrappers

#### `param_spec.rs` - Parameter Spec (single source of truth)
- `PARAM_SPECS`: 45 entries with min, max, default, curve (Linear / Log / Discrete), unit, stepped
- Drives `ParamBank` defaults, GUI slider ranges/curves, CC → value scaling (`cc_to_plain`) and the plugin's `FloatParam` ranges/skews
- Parity tests in `plugin.rs` assert the plugin and the bank agree on defaults and ranges

#### `params.rs` - CC Mapping
- `SynthParam` enum; `CCMapping`: bidirectional CC ↔ `SynthParam` map (defaults follow MIDI Sound Controller conventions)
- Custom mappings persist to `~/.rustinsynth/cc_mappings.json` (shared by both modes)

#### `presets.rs` - Preset System
- `Preset` (flat serde struct with `#[serde(default)]` on newer fields): oscillators 1-4, filters 1/2 + type, all envelopes, 3 LFOs + sync, Env 3/4, `mod_matrix`, tempo, distortion, delay (sync), reverb, chorus, stereo, polyphony, master, `category`
- **Format v2**: `PresetFile { format_version: 2, preset }`; bare v1 files still parse (`parse_preset_json`)
- Library: `list_library()` merges built-in factory presets with user files in `~/.rustsynth/presets/` (`PresetEntry { name, category, is_factory }`); `PRESET_BANKS` = Bass, Lead, Pad, Keys, Pluck, Seq, FX, User
- `load_preset()` resolves factory names to the built-in version first, so stale on-disk copies never shadow them
- Original factory presets live here; `factory_bank.rs` refreshes them (`renew`) and adds themed ones (`extended`) built on the matrix and tempo sync — ~75 total

#### `factory_bank.rs` - Extended Factory Bank
- Per-bank builders (`bass()`, `lead()`, `pad()`, `keys()`, `pluck()`, `seq()`, `fx()`, `showcase()`) using a compact `matrix(&[(source, dest, amount)])` syntax

### 4. Input Layer (`src/input/`)

#### `midi.rs` - MIDI Handler
- `midir` input; callback on the MIDI thread sends events via `mpsc::channel`
- `poll()`: non-blocking receive for the GUI thread
- Parses Note On/Off, CC, Pitch Bend and channel pressure (aftertouch)

### 5. Plugin Layer (`src/plugin*.rs`)

#### `plugin.rs` - NIH-plug Integration
- **`RustInSynthPlugin`** implementing `Plugin`; **`RustInSynthParams`** with every spec parameter exposed to the DAW
- `sync_plugin_params_safe()`: audio-thread-safe, change-detecting parameter sync
- **Tempo**: reads `context.transport().tempo` each buffer and writes it into `ModState.bpm`, falling back to the stored BPM when the host reports none
- **CC path** mirrors standalone: editor open → CC queue → `ParamSetter` (host sees automation, knobs move); editor closed → direct apply
- VST3 and CLAP export via `nih_export_vst3!` / `nih_export_clap!`

#### `plugin_gui/` - Plugin Editor
- **`PluginBackend<'a>`**: implements `SynthBackend` using `Arc<RustInSynthParams>` + `ParamSetter`; gestures map to `begin/end_set_parameter`
- **`PluginSharedState`**: CPU load and `ModState` shared between GUI and audio threads
- `create_editor()`: builds the editor from the same `panels::*` functions as standalone

## Data Flow

### Parameter Change (GUI → Audio)
```
1. User drags slider in egui
2. SynthApp/panels call backend.set_param() → ParamBank (atomic)
3. Next frame: sync_params() reads ParamBank
4. AudioEngine applies to VoiceManager
5. Audio callback uses new value
```

### Modulation Change (GUI → Audio)
```
1. User edits a matrix slot / LFO sync / Env 3-4 / BPM
2. panels call backend.set_mod_state(ModState)
3. State stored behind Mutex in SharedState / PluginSharedState
4. AudioEngine (or plugin process()) applies it via VoiceManager::apply_mod_state
   and EffectsChain::set_bpm / set_delay_tempo_sync
```

### Parameter Change (MIDI CC → Audio + GUI)
```
1. MIDI callback receives CC message
2. Sends event via channel
3. GUI polls channel, gets event
4. CC → value via param_spec::cc_to_plain; applied to engine and ParamBank
5. GUI sliders reflect new value
```

### Note Event (MIDI → Audio)
```
1. MIDI callback parses Note On/Off
2. Sends event via channel
3. GUI polls and forwards to AudioEngine
4. VoiceManager handles voice allocation / key stacking:
   - Poly: allocate or steal a voice
   - Mono: push (note, velocity) to stack; Note Off returns to previous note
5. Portamento: exponential glide between frequency changes
6. Velocity / key track / aftertouch become matrix source values
```

## Thread Safety

| Component | Thread | Synchronization |
|-----------|--------|-----------------|
| `ParamBank` | GUI + Audio | `AtomicU32` (lock-free) |
| `ModState` | GUI → Audio | `parking_lot::Mutex` (copied out per buffer) |
| `cpu_load` | Audio → GUI | `AtomicU32` (lock-free) |
| `midi_feedback` | GUI + MIDI | `DashMap` |
| `VoiceManager` | Audio | `parking_lot::Mutex` |
| MIDI events | MIDI → GUI | `mpsc::channel` |

## Performance Considerations

1. **Audio thread never blocks**: uses `Mutex::try_lock()` pattern
2. **GUI updates at 60fps**: `ctx.request_repaint()` for continuous polling
3. **MIDI processed in batches**: up to 64 events per frame
4. **Atomic parameters**: no locks for parameter reads in the audio callback
5. **Plugin builds use `assert_process_allocs`**: the audio path must not allocate; parameter sync avoids allocations

## File Summary

| File | Lines | Purpose |
|------|-------|---------|
| `core/voice.rs` | ~2800 | Voice manager, DSP, polyphony, ModState |
| `core/presets.rs` | ~2250 | Preset format v2, library, original factory presets |
| `gui/panels.rs` | ~1760 | Shared UI: tabs, matrix grid, preset browser |
| `core/oscillator.rs` | ~1100 | 4-oscillator bank |
| `core/factory_bank.rs` | ~1090 | Themed factory presets |
| `plugin.rs` | ~930 | NIH-plug plugin definition |
| `core/effects.rs` | ~920 | Distortion, Chorus, Delay, Reverb, chain |
| `core/envelope.rs` | ~850 | ADSR envelope |
| `core/filter.rs` | ~790 | SVF, Ladder, Comb filters |
| `core/params.rs` | ~690 | CC mapping system |
| `input/midi.rs` | ~490 | MIDI input handling |
| `audio/engine.rs` | ~455 | Audio stream management |
| `gui/widgets.rs` | ~455 | Custom egui widgets |
| `core/mod_matrix.rs` | ~445 | Modulation matrix |
| `core/param_spec.rs` | ~440 | Parameter spec table |
| `gui/backend_standalone.rs` | ~395 | Standalone backend |
| `gui/theme.rs` | ~365 | Color theme |
| `plugin_gui/backend_plugin.rs` | ~335 | Plugin backend |
| `core/lfo.rs` | ~315 | LFO |
| `gui/backend.rs` | ~190 | `SynthBackend` trait |
| `gui/app.rs` | ~105 | Main GUI app |
| `plugin_gui/editor.rs` | ~100 | Plugin editor |

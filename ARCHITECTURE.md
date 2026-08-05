# RustInSynth Architecture

**Version: v1.2.0**

## Overview

RustInSynth is a real-time polyphonic synthesizer with a GUI, designed around lock-free communication between the audio thread and the UI thread. It supports both standalone and plugin (VST3/CLAP) operation through a unified `SynthBackend` trait.

```
┌─────────────────────────────────────────────────────────────────┐
│                         GUI Thread                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   egui      │───▶│  panels::*  │───▶│  ParamBank          │  │
│  │  (render)   │    │  (shared UI)│    │  (AtomicU32 array)  │  │
│  └─────────────┘    └─────────────┘    └──────────┬──────────┘  │
└───────────────────────────────────────────────────┼─────────────┘
                                                    │ lock-free
┌───────────────────────────────────────────────────┼─────────────┐
│                       Audio Thread                │              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────▼──────────┐   │
│  │   cpal      │◀───│AudioEngine  │◀───│  VoiceManager      │   │
│  │  (output)   │    │  (stream)   │    │  (mono/poly DSP)   │   │
│  └─────────────┘    └─────────────┘    └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. GUI Layer (`src/gui/`)

#### `mod.rs` - Shared State
- **`ParamBank`**: Array of `AtomicU32` storing all synth parameters as f32 bits
- **`SharedState`**: Contains `ParamBank` + `DashMap` for MIDI CC feedback + CPU load atomic
- Lock-free reads/writes using `Ordering::Relaxed`
- `get_cpu_load()` / `set_cpu_load()` for real-time CPU measurement

#### `app.rs` - Main Application (Standalone)
- **`SynthApp`**: Owns `Box<dyn SynthBackend>` for backend-agnostic GUI
- Delegates to shared `panels::*` functions for all UI rendering
- `run_gui()`: Entry point for standalone application with X11/Wayland handling

#### `backend.rs` - Unified Backend Trait
- **`SynthBackend`**: Core trait abstracting all parameter access
- Methods for: parameters, effects (delay/reverb/chorus), voice management, stereo, MIDI, CPU load
- Implemented by both `StandaloneBackend` and `PluginBackend<'a>`

#### `backend_standalone.rs` - Standalone Implementation
- **`StandaloneBackend`**: Owns `AudioEngine`, `SharedState`, `MidiInputHandler`
- MIDI CC learn with persistence to `~/.rustinsynth/cc_mappings.json`
- Polls MIDI and syncs `ParamBank` → `VoiceManager` every frame

#### `widgets.rs` - Custom Controls
- Rotary knobs with drag interaction
- Toggle switches, selector switches
- VU meter, MIDI indicator
- Minimoog-style panel backgrounds

#### `theme.rs` - Centralized Styling
- `SynthTheme` struct with all UI colors
- Global `THEME` constant for consistent styling
- Easy to swap color schemes

### 2. Audio Layer (`src/audio/`)

#### `engine.rs` - Audio Engine
- Creates `cpal` output stream
- Owns `Arc<Mutex<VoiceManager>>` shared with audio callback
- `sync_params()`: Reads `ParamBank` and applies to `VoiceManager`
- `send_event()`: Forwards MIDI events to `VoiceManager`
- **CPU load measurement**: Measures `process_time / available_time` per buffer
- Smoothed with low-pass filter (0.9/0.1) for stable display

### 3. Core DSP (`src/core/`)

#### `voice.rs` - Voice Manager
- **Polyphonic mode**: 8 voices with voice stealing, or monophonic with key stacking
- Contains per-voice: `OscillatorBank`, `SVFilter` (L/R), `ADSREnvelope` (×2), `LFO`
- **Key Stack**: `Vec<(MidiNote, Amplitude)>` for proper note priority (mono mode)
- **Portamento**: Exponential glide between notes (0.005-2.0 seconds)
- Implements `SynthEventReceiver` trait for note/CC handling
- `next_sample_stereo()`: Generates stereo output with per-oscillator panning

#### `oscillator.rs` - Oscillator Bank
- 3 independent oscillators with:
  - 5 waveforms (Sine, Triangle, Saw, Square, Noise)
  - Level, phase, detune (semitones + cents)
- Band-limited waveforms using PolyBLEP anti-aliasing

#### `filter.rs` - State Variable Filter
- 12dB/oct lowpass with resonance (per-channel filters for stereo)
- Analog-style saturation (tanh)
- Cutoff range: 20Hz - 20kHz (exponential)
- **Filter envelope**: 6-octave range, musical (not linear Hz)

#### `envelope.rs` - ADSR Envelope
- Attack, Decay, Sustain, Release stages
- Exponential curves for natural response
- Used for both amplitude and filter modulation

#### `lfo.rs` - Low Frequency Oscillator
- 5 waveforms (Sine, Triangle, Square, Saw, Random)
- 4 destinations (Off, Pitch, Filter, Amplitude)
- Rate: 0.1 - 20 Hz

#### `params.rs` - CC Mapping
- `CCMapping`: Bidirectional map between MIDI CC numbers and `SynthParam`
- Default mappings follow MIDI Sound Controller conventions
- **Portamento**: CC 5 → `PortamentoTime` (0.0-3.0s exponential)
- Conversion functions: `cc_to_cutoff()`, `cc_to_time()`, `cc_to_portamento_time()`, etc.

#### `presets.rs` - Preset System
- JSON serialization via `serde`
- Stored in `~/.rustsynth/presets/`
- Contains all oscillator, filter, envelope, LFO, stereo, effects, and polyphony settings
- 15+ factory presets included (bass, lead, pad, FX, keys)

### 4. Input Layer (`src/input/`)

#### `midi.rs` - MIDI Handler
- Uses `midir` for cross-platform MIDI input
- Runs callback on MIDI thread, sends events via `mpsc::channel`
- `poll()`: Non-blocking receive for GUI thread
- Parses Note On/Off, CC, Pitch Bend

### 5. Plugin Layer (`src/plugin*.rs`)

#### `plugin.rs` - NIH-plug Integration
- **`RustInSynthPlugin`**: Main plugin struct implementing `Plugin` trait
- **`RustInSynthParams`**: All parameters exposed to DAW (32+ automatable params)
- **`sync_plugin_params_safe()`**: Audio-thread-safe parameter sync
- VST3 and CLAP export via `nih_export_vst3!` / `nih_export_clap!`

#### `plugin_gui/` - Plugin Editor
- **`PluginBackend<'a>`**: Implements `SynthBackend` using `Arc<RustInSynthParams>` + `ParamSetter`
- **`create_editor()`**: Builds plugin UI using shared `panels::*` functions
- **`PluginSharedState`**: Lock-free CPU load sharing from audio to GUI thread
- Identical UI to standalone (same `panels.rs` code)

## Data Flow

### Parameter Change (GUI → Audio)
```
1. User drags slider in egui
2. SynthApp::set_param() writes to ParamBank (atomic)
3. Next frame: sync_params() reads ParamBank
4. AudioEngine applies to VoiceManager
5. Audio callback uses new value
```

### Parameter Change (MIDI CC → Audio + GUI)
```
1. MIDI callback receives CC message
2. Sends NoteEvent via channel
3. GUI polls channel, gets event
4. Sends event to AudioEngine (immediate effect)
5. Updates ParamBank (keeps GUI in sync)
6. GUI sliders reflect new value
```

### Note Event (MIDI → Audio)
```
1. MIDI callback parses Note On/Off
2. Sends NoteEvent via channel
3. GUI polls and forwards to AudioEngine
4. VoiceManager handles key stacking:
   - Note On: Push (note, velocity) to stack, trigger voice
   - Note Off: Remove from stack, return to previous note if any
5. Portamento: Linear glide between frequency changes
```

## Thread Safety

| Component | Thread | Synchronization |
|-----------|--------|-----------------|
| `ParamBank` | GUI + Audio | `AtomicU32` (lock-free) |
| `cpu_load` | Audio → GUI | `AtomicU32` (lock-free) |
| `midi_feedback` | GUI + MIDI | `DashMap` (lock-free) |
| `VoiceManager` | Audio | `parking_lot::Mutex` |
| MIDI events | MIDI → GUI | `mpsc::channel` |

## Performance Considerations

1. **Audio thread never blocks**: Uses `Mutex::try_lock()` pattern
2. **GUI updates at 60fps**: `ctx.request_repaint()` for continuous polling
3. **MIDI processed in batches**: Up to 64 events per frame
4. **Atomic parameters**: No locks for parameter reads in audio callback

## File Summary

| File | Lines | Purpose |
|------|-------|---------|
| `gui/app.rs` | ~110 | Main GUI app (backend-agnostic) |
| `gui/backend.rs` | ~165 | `SynthBackend` trait definition |
| `gui/backend_standalone.rs` | ~525 | Standalone backend implementation |
| `gui/panels.rs` | ~930 | Shared UI panels (standalone + plugin) |
| `gui/widgets.rs` | ~385 | Custom egui widgets |
| `gui/theme.rs` | ~210 | Centralized color theme |
| `gui/mod.rs` | ~235 | SharedState, ParamBank |
| `core/voice.rs` | ~1650 | Voice manager + DSP + polyphony |
| `core/oscillator.rs` | ~500 | Oscillator bank |
| `core/filter.rs` | ~200 | SVF implementation |
| `core/envelope.rs` | ~285 | ADSR envelope |
| `core/effects.rs` | ~200 | Delay, Reverb, Chorus |
| `core/params.rs` | ~540 | CC mapping system |
| `core/presets.rs` | ~1055 | Preset system + factory presets |
| `audio/engine.rs` | ~230 | Audio stream management |
| `input/midi.rs` | ~480 | MIDI input handling |
| `plugin.rs` | ~825 | NIH-plug plugin definition |
| `plugin_gui/backend_plugin.rs` | ~305 | Plugin backend implementation |
| `plugin_gui/editor.rs` | ~65 | Plugin editor (shared UI) |

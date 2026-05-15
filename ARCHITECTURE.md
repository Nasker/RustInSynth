# RustInSynth Architecture

## Overview

RustInSynth is a real-time monophonic synthesizer with a GUI, designed around lock-free communication between the audio thread and the UI thread.

```
┌─────────────────────────────────────────────────────────────────┐
│                         GUI Thread                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐  │
│  │   egui      │───▶│  SynthApp   │───▶│  ParamBank          │  │
│  │  (render)   │    │  (logic)    │    │  (AtomicU32 array)  │  │
│  └─────────────┘    └─────────────┘    └──────────┬──────────┘  │
└───────────────────────────────────────────────────┼─────────────┘
                                                    │ lock-free
┌───────────────────────────────────────────────────┼─────────────┐
│                       Audio Thread                │              │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────▼──────────┐   │
│  │   cpal      │◀───│AudioEngine  │◀───│  VoiceManager      │   │
│  │  (output)   │    │  (stream)   │    │  (synthesis)       │   │
│  └─────────────┘    └─────────────┘    └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. GUI Layer (`src/gui/`)

#### `mod.rs` - Shared State
- **`ParamBank`**: Array of `AtomicU32` storing all synth parameters as f32 bits
- **`SharedState`**: Contains `ParamBank` + `DashMap` for MIDI CC feedback
- Lock-free reads/writes using `Ordering::Relaxed`

#### `app.rs` - Main Application
- **`SynthApp`**: Owns `AudioEngine`, `MidiInputHandler`, and `SharedState`
- Polls MIDI events every frame (up to 64 per frame)
- Syncs `ParamBank` → `VoiceManager` every frame
- Updates `ParamBank` from incoming MIDI CCs to prevent overwrite

#### `widgets.rs` - Custom Controls
- Rotary knobs with drag interaction
- Toggle switches, selector switches
- VU meter, MIDI indicator
- Minimoog-style panel backgrounds

### 2. Audio Layer (`src/audio/`)

#### `engine.rs` - Audio Engine
- Creates `cpal` output stream
- Owns `Arc<Mutex<VoiceManager>>` shared with audio callback
- `sync_params()`: Reads `ParamBank` and applies to `VoiceManager`
- `send_event()`: Forwards MIDI events to `VoiceManager`

### 3. Core DSP (`src/core/`)

#### `voice.rs` - Voice Manager
- Manages monophonic voice allocation with key stacking
- Contains: `OscillatorBank`, `SVFilter`, `ADSREnvelope` (×2), `LFO`
- **Key Stack**: `Vec<(MidiNote, Amplitude)>` for proper note priority
- **Portamento**: Linear glide between notes (0-3 seconds)
- Implements `SynthEventReceiver` trait for note/CC handling
- `next_sample()`: Called by audio thread ~44100×/sec

#### `oscillator.rs` - Oscillator Bank
- 3 independent oscillators with:
  - 5 waveforms (Sine, Triangle, Saw, Square, Noise)
  - Level, phase, detune (semitones + cents)
- Band-limited waveforms using PolyBLEP anti-aliasing

#### `filter.rs` - State Variable Filter
- 12dB/oct lowpass with resonance
- Analog-style saturation (tanh)
- Cutoff range: 20Hz - 20kHz (exponential)

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
- Contains all oscillator, filter, envelope, LFO settings

### 4. Input Layer (`src/input/`)

#### `midi.rs` - MIDI Handler
- Uses `midir` for cross-platform MIDI input
- Runs callback on MIDI thread, sends events via `mpsc::channel`
- `poll()`: Non-blocking receive for GUI thread
- Parses Note On/Off, CC, Pitch Bend

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
| `gui/app.rs` | ~1100 | Main GUI application |
| `gui/widgets.rs` | ~400 | Custom egui widgets |
| `gui/mod.rs` | ~200 | SharedState, ParamBank |
| `core/voice.rs` | ~1150 | Voice manager + DSP + key stacking |
| `core/oscillator.rs` | ~500 | Oscillator bank |
| `core/filter.rs` | ~200 | SVF implementation |
| `core/envelope.rs` | ~200 | ADSR envelope |
| `core/params.rs` | ~540 | CC mapping system + portamento |
| `audio/engine.rs` | ~230 | Audio stream management |
| `input/midi.rs` | ~440 | MIDI input handling |

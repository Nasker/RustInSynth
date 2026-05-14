# RustSynth

A modular monophonic synthesizer written in pure Rust.

## Features

- **Real-time audio synthesis** using `cpal`
- **Multiple waveforms**: Sine, Square, Saw, Triangle
- **Resonant low-pass filter** (State Variable Filter)
- **AR envelope** with configurable attack/release
- **Dual input modes**:
  - QWERTY keyboard (piano-style layout)
  - USB MIDI controller support via `midir`
- **MIDI CC control** for real-time parameter tweaking
- **MIDI debug mode** for inspecting raw messages

## Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Oscillator │───▶│   Filter    │───▶│  Envelope   │───▶ Output
│ (Waveform)  │    │ (SVF LP/HP) │    │    (AR)     │
└─────────────┘    └─────────────┘    └─────────────┘
```

The synth follows a classic subtractive synthesis signal chain:
1. **Oscillator** generates the raw waveform
2. **Filter** shapes the harmonic content (resonant SVF)
3. **Envelope** controls amplitude over time

## MIDI CC Mappings

| CC  | Parameter        | Range                              |
|-----|------------------|------------------------------------|
| 71  | Filter Resonance | 0% → 100%                          |
| 72  | Release Time     | 1ms → 5s                           |
| 73  | Attack Time      | 1ms → 2s                           |
| 74  | Filter Cutoff    | 20Hz → 20kHz (exponential)         |
| 75  | Waveform         | 0-31=Sine, 32-63=Sq, 64-95=Saw, 96+=Tri |

CC mappings follow MIDI Sound Controller conventions and are fully configurable at runtime.

## Keyboard Layout

```
Upper row: Q W E R T Y U I  (C5 to C6)
Black keys: 2 3   5 6 7     (sharps/flats)

Lower row: Z X C V B N M ,  (C4 to C5)
Black keys: S D   G H J     (sharps/flats)

Controls:
  ↑/↓   - Octave up/down
  F1-F4 - Waveform select
  ESC   - Quit
```

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

On startup, choose between keyboard or MIDI input mode.

## Dependencies

- `cpal` - Cross-platform audio I/O
- `midir` - MIDI input handling
- `crossterm` - Terminal keyboard input
- `parking_lot` - Fast synchronization primitives

## Project Structure

```
src/
├── main.rs              # Entry point, input mode selection
├── lib.rs               # Library exports
├── audio/
│   └── engine.rs        # Audio stream management
├── core/
│   ├── envelope.rs      # AR/ADSR envelopes
│   ├── event.rs         # Note/CC event system
│   ├── filter.rs        # State Variable Filter
│   ├── oscillator.rs    # Waveform generators
│   ├── params.rs        # CC mapping system
│   ├── types.rs         # Core type definitions
│   └── voice.rs         # Voice allocation
└── input/
    ├── keyboard.rs      # QWERTY input handling
    └── midi.rs          # MIDI input + debug parser
```

## Current Status

**v0.1.0** - Core synthesis engine complete:
- [x] Monophonic voice management
- [x] 4 oscillator types
- [x] Resonant SVF filter
- [x] AR envelope
- [x] MIDI input with channel filtering
- [x] Configurable CC mappings
- [x] MIDI message debug output

## Roadmap

- [ ] Polyphonic voice allocation
- [ ] ADSR envelope
- [ ] LFO modulation
- [ ] Filter envelope
- [ ] Preset save/load
- [ ] GUI (egui or iced)

## License

MIT

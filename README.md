# RustSynth

A Minimoog-style monophonic synthesizer written in pure Rust.

## Features

- **3-oscillator bank** with per-oscillator waveform, level, phase, and detune
- **Resonant low-pass filter** (State Variable Filter with analog-style saturation)
- **Full ADSR envelope** with attack, decay, sustain, and release
- **Real-time audio synthesis** using `cpal`
- **Dual input modes**:
  - QWERTY keyboard (piano-style layout)
  - USB MIDI controller support via `midir`
- **Full MIDI CC control** for all 21 parameters
- **MIDI debug mode** for inspecting raw messages

## Architecture

```
┌─────────────────────────────────────┐
│          OSCILLATOR BANK            │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐│
│  │  OSC 1  │ │  OSC 2  │ │  OSC 3  ││
│  │ (main)  │ │(detune) │ │ (sub)   ││
│  └────┬────┘ └────┬────┘ └────┬────┘│
│       └──────┬────┴───────────┘     │
└──────────────┼──────────────────────┘
               ▼
        ┌─────────────┐    ┌─────────────┐
        │   Filter    │───▶│  Envelope   │───▶ Output
        │ (SVF + sat) │    │   (ADSR)    │
        └─────────────┘    └─────────────┘
```

**Default patch**: OSC1=Saw, OSC2=Saw+7¢ (slight detune), OSC3=Square-12st (sub bass)

## MIDI CC Mappings

All parameters follow MIDI Sound Controller conventions:

### ADSR Envelope
| CC  | Parameter     | Range      |
|-----|---------------|------------|
| 73  | Attack Time   | 1ms → 2s   |
| 83  | Decay Time    | 1ms → 5s   |
| 84  | Sustain Level | 0% → 100%  |
| 72  | Release Time  | 1ms → 5s   |

### Filter
| CC  | Parameter  | Range                |
|-----|------------|----------------------|
| 74  | Cutoff     | 20Hz → 20kHz (exp)   |
| 71  | Resonance  | 0% → 100%            |

### Oscillators
| CC  | Parameter      | Range                              |
|-----|----------------|------------------------------------|
| 75  | OSC1 Waveform  | 0-31=Sine, 32-63=Sq, 64-95=Saw, 96+=Tri |
| 76  | OSC2 Waveform  | (same)                             |
| 77  | OSC3 Waveform  | (same)                             |
| 80  | OSC1 Level     | 0% → 100%                          |
| 81  | OSC2 Level     | 0% → 100%                          |
| 82  | OSC3 Level     | 0% → 100%                          |
| 78  | OSC2 Semitones | -24 → +24 (CC 64 = 0)              |
| 79  | OSC3 Semitones | -24 → +24 (CC 64 = 0)              |
| 85  | OSC2 Cents     | -100 → +100 (CC 64 = 0)            |
| 86  | OSC3 Cents     | -100 → +100 (CC 64 = 0)            |
| 87  | OSC1 Phase     | 0° → 360°                          |
| 89  | OSC2 Phase     | 0° → 360°                          |
| 90  | OSC3 Phase     | 0° → 360°                          |

CC mappings are fully configurable at runtime via the `CCMapping` API.

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

**v0.2.0** - Full ADSR envelope:
- [x] 3-oscillator bank (Minimoog-style)
- [x] Per-oscillator waveform, level, phase, detune
- [x] Resonant SVF filter with analog saturation
- [x] Full ADSR envelope
- [x] MIDI input with channel filtering
- [x] Full CC mapping system (21 parameters)
- [x] MIDI message debug output

## Roadmap

- [ ] Polyphonic voice allocation
- [ ] LFO modulation
- [ ] Filter envelope
- [ ] Noise oscillator
- [ ] Preset save/load
- [ ] GUI (egui or iced)

## License

MIT

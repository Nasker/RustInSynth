# RustSynth

A Minimoog-style monophonic synthesizer written in pure Rust.

## Features

- **3-oscillator bank** with per-oscillator waveform, level, phase, and detune
- **Resonant low-pass filter** (State Variable Filter with analog-style saturation)
- **Dual ADSR envelopes** - amplitude (VCA) and filter (VCF)
- **LFO modulation** - vibrato, filter wah, or tremolo (5 waveforms)
- **Real-time audio synthesis** using `cpal`
- **Dual input modes**:
  - QWERTY keyboard (piano-style layout)
  - USB MIDI controller support via `midir`
- **Full MIDI CC control** for all 31 parameters
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

### Amplitude Envelope (VCA)
| CC  | Parameter     | Range      |
|-----|---------------|------------|
| 73  | Attack Time   | 1ms → 2s   |
| 83  | Decay Time    | 1ms → 5s   |
| 84  | Sustain Level | 0% → 100%  |
| 72  | Release Time  | 1ms → 5s   |

### Filter Envelope (VCF)
| CC  | Parameter       | Range      |
|-----|-----------------|------------|
| 103 | Filter Attack   | 1ms → 2s   |
| 104 | Filter Decay    | 1ms → 5s   |
| 105 | Filter Sustain  | 0% → 100%  |
| 106 | Filter Release  | 1ms → 5s   |
| 107 | Filter Env Amt  | 0% → 100%  |

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

### LFO
| CC  | Parameter       | Range                           |
|-----|-----------------|---------------------------------|
| 108 | LFO Rate        | 0.1 → 20 Hz                     |
| 109 | LFO Depth       | 0% → 100%                       |
| 110 | LFO Waveform    | 0=Sine, 1=Tri, 2=Sq, 3=Saw, 4=Rand |
| 111 | LFO Destination | 0=Off, 1=Pitch, 2=Filter, 3=Amp |

### Pitch Bend
| Control | Parameter       | Range                    |
|---------|-----------------|--------------------------|
| Wheel   | Pitch Bend      | ±range (default ±12 st)  |
| CC 102  | Pitch Bend Range| 1 → 24 semitones         |

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
│   ├── lfo.rs           # Low Frequency Oscillator
│   ├── oscillator.rs    # Waveform generators
│   ├── params.rs        # CC mapping system
│   ├── types.rs         # Core type definitions
│   └── voice.rs         # Voice allocation
└── input/
    ├── keyboard.rs      # QWERTY input handling
    └── midi.rs          # MIDI input + debug parser
```

## Current Status

**v0.3.0** - Filter envelope + LFO:
- [x] 3-oscillator bank (Minimoog-style)
- [x] Per-oscillator waveform, level, phase, detune
- [x] Resonant SVF filter with analog saturation
- [x] Dual ADSR envelopes (amplitude + filter)
- [x] LFO with 5 waveforms, 3 destinations
- [x] Pitch bend with configurable range (1-24 semitones)
- [x] MIDI input with channel filtering
- [x] Full CC mapping system (31 parameters)
- [x] MIDI message debug output

## Roadmap

### Near-term (Complete Monophonic Synth)
- [x] **Filter envelope** - Dedicated ADSR for cutoff modulation
- [x] **LFO modulation** - Low-frequency oscillator for pitch/filter
- [ ] **Noise oscillator** - White/pink noise source
- [ ] **Preset save/load** - JSON-based patch storage

### Long-term
- [ ] Polyphonic voice allocation
- [ ] GUI (egui or iced)
- [ ] Effects (reverb, delay, chorus)
- [ ] Arpeggiator/sequencer

## License

MIT

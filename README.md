# RustInSynth

A Minimoog-style monophonic synthesizer written in pure Rust with a real-time GUI.

![RustInSynth GUI](docs/screenshot.png)

## Features

- **3-oscillator bank** with 5 waveforms (Sine, Triangle, Saw, Square, Noise)
- **Resonant low-pass filter** (State Variable Filter with analog-style saturation)
- **Dual ADSR envelopes** - amplitude (VCA) and filter (VCF)
- **LFO modulation** - vibrato, filter wah, or tremolo (5 waveforms)
- **Real-time GUI** built with `egui` - single-window Minimoog-style layout
- **Real-time audio synthesis** using `cpal`
- **USB MIDI controller support** via `midir` with live CC feedback
- **Full MIDI CC control** for all 31+ parameters
- **Preset system** - JSON-based save/load
- **Lock-free parameter sharing** between GUI and audio threads

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
| 75  | OSC1 Waveform  | 0-25=Sine, 26-51=Sq, 52-77=Saw, 78-103=Tri, 104+=Noise |
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

## Presets

Presets are saved as JSON files in `~/.rustsynth/presets/`. Each preset contains all synth parameters including oscillators, filter, envelopes, LFO, and master volume.

**Programmatic usage:**
```rust
use rustsynth::core::{VoiceManager, save_preset, load_preset};

// Create and save a preset
let mut vm = VoiceManager::monophonic(44100);
// ... configure the sound ...
let preset = vm.create_preset("My Bass");
save_preset(&preset).expect("Failed to save preset");

// Load and apply a preset
let loaded = load_preset("My Bass").expect("Failed to load preset");
vm.apply_preset(&loaded);
```

**CLI preset management** (future enhancement): Load presets by name from command line arguments.

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
- `egui` / `eframe` - Immediate-mode GUI
- `parking_lot` - Fast synchronization primitives
- `dashmap` - Lock-free concurrent hashmap
- `serde` / `serde_json` - Preset serialization

## Project Structure

```
src/
├── main.rs              # Entry point (GUI mode)
├── lib.rs               # Library exports
├── audio/
│   └── engine.rs        # Audio stream + param sync
├── core/
│   ├── envelope.rs      # AR/ADSR envelopes
│   ├── event.rs         # Note/CC event system
│   ├── filter.rs        # State Variable Filter
│   ├── lfo.rs           # Low Frequency Oscillator
│   ├── oscillator.rs    # Waveform generators + bank
│   ├── params.rs        # CC mapping system
│   ├── presets.rs       # JSON preset save/load
│   ├── types.rs         # Core type definitions
│   └── voice.rs         # Voice manager
├── gui/
│   ├── mod.rs           # SharedState + ParamBank
│   ├── app.rs           # Main egui application
│   └── widgets.rs       # Custom knobs, toggles, meters
└── input/
    └── midi.rs          # MIDI input handler
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed design documentation.

## Current Status

**v0.5.0** - Full GUI:
- [x] Real-time egui GUI with Minimoog-style layout
- [x] 3-oscillator bank with 5 waveforms
- [x] Per-oscillator waveform, level, phase, detune (semi + cents)
- [x] Resonant SVF filter with analog saturation
- [x] Dual ADSR envelopes (amplitude + filter)
- [x] LFO with 5 waveforms, 4 destinations
- [x] Pitch bend with configurable range
- [x] JSON-based preset save/load
- [x] MIDI input with live CC feedback display
- [x] Lock-free GUI ↔ Audio parameter sync
- [x] Full CC mapping (31+ parameters)

## Roadmap

- [ ] Runtime MIDI CC learn/mapping
- [ ] Polyphonic voice allocation
- [ ] Effects (reverb, delay, chorus)
- [ ] Arpeggiator/sequencer
- [ ] VST3/CLAP plugin export

## License

MIT

# RustInSynth

A Minimoog-style synthesizer written in pure Rust with a real-time GUI, **true stereo output**, and both **standalone and plugin (VST3/CLAP)** operation.

![RustInSynth GUI](docs/screenshot.png)

## Features

- **3-oscillator bank** with 5 waveforms (Sine, Triangle, Saw, Square, Noise) + PolyBLEP anti-aliasing
- **True stereo output** with per-oscillator panning, stereo width, and independent L/R filters
- **Resonant low-pass filter** (State Variable Filter with analog-style saturation)
- **Dual ADSR envelopes** - amplitude (VCA) and filter (VCF) with 6-octave filter envelope range
- **LFO modulation** - vibrato, filter wah, or tremolo (5 waveforms)
- **Stereo effects chain** - Ping-pong Delay, Stereo Reverb, Wide Chorus
- **Portamento (glide)** with exponential curve (0.005-2.0 seconds)
- **Mono/Poly modes** - monophonic with key stacking OR 8-voice polyphony with voice stealing
- **Dual operation modes** - Standalone app AND VST3/CLAP plugin via NIH-plug
- **Unified GUI** - single codebase drives both standalone and plugin via `SynthBackend` trait
- **Real-time GUI** built with `egui` - single-window Minimoog-style layout
- **Real-time audio synthesis** using `cpal` (standalone)
- **USB MIDI controller support** via `midir` with live CC feedback and runtime CC learn
- **Full MIDI CC control** for all 35+ parameters with customizable mappings
- **Preset system** - JSON-based save/load with 15+ factory presets
- **Lock-free parameter sharing** between GUI and audio threads
- **Real-time CPU meter** - actual DSP load measurement

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

### Portamento
| CC  | Parameter        | Range                      |
|-----|------------------|----------------------------|
| 5   | Portamento Time  | 0.0 → 3.0s (exponential)  |

CC mappings are fully configurable at runtime via the `CCMapping` API. Custom mappings are persisted to `~/.rustinsynth/cc_mappings.json`.

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

### Standalone Application

```bash
cargo build --release
```

### Plugin (VST3/CLAP)

```bash
cargo build --release --features plugin
```

The plugin binaries will be in `target/release/` with `.so` (Linux), `.dll` (Windows), or `.dylib` (macOS) extensions. Use a plugin scanner or copy to your DAW's plugin folder.

## Running

### Standalone

```bash
cargo run
```

Connect a USB MIDI controller or use the keyboard shortcuts (see below).

## Dependencies

- `cpal` - Cross-platform audio I/O
- `midir` - MIDI input handling  
- `egui` / `eframe` - Immediate-mode GUI
- `parking_lot` - Fast synchronization primitives
- `dashmap` - Lock-free concurrent hashmap
- `serde` / `serde_json` - Preset serialization
- `nih_plug` / `nih_plug_egui` - VST3/CLAP plugin framework

## Project Structure

```
src/
├── main.rs                       # Entry point (standalone GUI mode)
├── lib.rs                        # Library exports
├── audio/
│   └── engine.rs                 # Audio stream + param sync (standalone)
├── core/
│   ├── effects.rs                # Delay, Reverb, Chorus + EffectsChain
│   ├── envelope.rs               # AR/ADSR envelopes
│   ├── event.rs                  # Note/CC event system
│   ├── filter.rs                 # State Variable Filter (L/R for stereo)
│   ├── lfo.rs                    # Low Frequency Oscillator
│   ├── oscillator.rs             # Waveform generators + bank (PolyBLEP)
│   ├── params.rs                 # CC mapping system + SynthParam enum
│   ├── presets.rs                # JSON preset save/load + factory presets
│   ├── types.rs                  # Core type definitions (StereoSample)
│   └── voice.rs                  # Voice manager (mono/poly, 8 voices)
├── gui/
│   ├── mod.rs                    # SharedState, ParamBank, exports
│   ├── app.rs                    # SynthApp — single GUI, backend-agnostic
│   ├── backend.rs                # SynthBackend trait (unified abstraction)
│   ├── backend_standalone.rs     # StandaloneBackend (AudioEngine + MIDI)
│   ├── panels.rs                 # Shared UI panels (standalone + plugin)
│   ├── theme.rs                  # Rust In Peace color theme
│   └── widgets.rs                # Custom knobs, toggles, VU meters
├── input/
│   ├── midi.rs                   # MIDI input handler (standalone)
│   └── keyboard.rs               # QWERTY keyboard input
├── plugin.rs                     # NIH-plug plugin definition
└── plugin_gui/
    ├── mod.rs                    # Module exports
    ├── backend_plugin.rs         # PluginBackend (NIH-plug integration)
    ├── editor.rs                 # Plugin editor (shared panels)
    ├── shared_state.rs           # PluginSharedState (CPU load)
    └── widgets.rs                # Plugin widget helpers
```

## Unified GUI Architecture

Both the **standalone application** and the **NIH-plug plugin** share a single GUI codebase through the `SynthBackend` trait abstraction.

```
              ┌──────────────────────────────┐
              │        SynthApp              │
              │  (one GUI, backend-agnostic) │
              └──────────────┬───────────────┘
                             │ Box<dyn SynthBackend>
              ┌──────────────┴───────────────┐
              │                              │
   ┌──────────▼──────────┐      ┌────────────▼────────────┐
   │  StandaloneBackend  │      │     PluginBackend<'a>    │
   │  (owns AudioEngine, │      │  (wraps Arc<Params> +   │
   │   SharedState, MIDI)│      │   &'a ParamSetter)       │
   └─────────────────────┘      └─────────────────────────┘
```

### Key design decisions

- **`SynthBackend` trait** (`src/gui/backend.rs`) — defines `get_param()`, `set_param()`, effects, voice management, stereo, MIDI, and CPU load. No `'static` bound on the trait itself.
- **`StandaloneBackend`** — owns `AudioEngine` + `SharedState`. Implements MIDI polling and CC-learn. `'static` because all owned types are `'static`.
- **`PluginBackend<'a>`** — holds `Arc<RustInSynthParams>` + `&'a ParamSetter`. Recreated cheaply each frame. MIDI methods are no-ops (host handles routing).
- **`SynthApp`** stores `Box<dyn SynthBackend + 'static>`, satisfying `eframe::App`'s `'static` requirement for standalone. For the plugin, `PluginBackend` is used as `&mut dyn SynthBackend` without boxing.
- **Adding a new feature**: implement it in `SynthBackend`, add to `StandaloneBackend` and `PluginBackend`, add UI in `app.rs` — one change covers both modes.

See [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for the phased implementation history.

## Current Status

**v1.0.1** - Stereo Fix & Unified GUI Architecture:
- [x] Fixed stereo separation with independent left/right filters per voice
- [x] `SynthBackend` trait abstraction — single GUI codebase for standalone and plugin
- [x] `StandaloneBackend` wrapping `AudioEngine`, `SharedState`, and MIDI handler
- [x] `PluginBackend<'a>` wrapping NIH-plug `RustInSynthParams` + `ParamSetter`
- [x] Effects parameters (delay/reverb/chorus) exposed via NIH-plug automation
- [x] `EffectsChain` unified setter/getter API
- [x] Plugin editor using identical panel functions as standalone GUI
- [x] `PluginSharedState` for lock-free CPU load sharing
- [x] Unified stereo width and oscillator pan across standalone and plugin

**v1.0.0** - True Stereo Output & VST3/CLAP:
- [x] VST3 and CLAP plugin export via nih-plug
- [x] 32+ automatable parameters in DAWs
- [x] Per-oscillator panning (L/R positioning)
- [x] Stereo width control (mono to extra-wide)
- [x] Ping-pong delay with stereo feedback
- [x] True stereo reverb (different L/R reflections)
- [x] Wide chorus with LFO phase offset
- [x] GUI controls for pan and width

**v0.9.0** - Effects Chain:
- [x] Delay effect (time, feedback, mix)
- [x] Reverb (Schroeder-style: 4 comb + 2 allpass filters)
- [x] Chorus (LFO-modulated delay)
- [x] GUI with enable toggles + parameter sliders
- [x] 8-voice polyphony with voice stealing
- [x] 15+ factory presets (bass, lead, pad, FX, keys)

**v0.6.0** - Portamento + Key Stacking:
- [x] Portamento (glide) with linear interpolation (0-3s)
- [x] Monophonic key stacking with proper note priority
- [x] Portamento works between stacked notes
- [x] MIDI CC 5 control for portamento time
- [x] GUI controls for portamento in both panels
- [x] Preset support for portamento settings

**v0.5.0** - Full GUI:
- [x] Real-time egui GUI with Minimoog-style layout
- [x] 3-oscillator bank with 5 waveforms (PolyBLEP anti-aliasing)
- [x] Per-oscillator waveform, level, phase, detune (semi + cents), pan
- [x] Resonant SVF filter with analog saturation
- [x] Dual ADSR envelopes (amplitude + filter)
- [x] LFO with 5 waveforms, 4 destinations
- [x] Pitch bend with configurable range
- [x] JSON-based preset save/load
- [x] MIDI input with live CC feedback display and CC learn
- [x] Lock-free GUI ↔ Audio parameter sync
- [x] Full CC mapping (35+ parameters)

## Roadmap

- [x] ~~Polyphonic voice allocation~~ ✅ Done (8 voices)
- [x] ~~Effects chain~~ ✅ Done (Delay, Reverb, Chorus)
- [x] ~~Stereo output~~ ✅ Done (per-osc pan, width, stereo FX)
- [x] ~~VST3/CLAP plugin export~~ ✅ Done (via nih-plug)
- [ ] Arpeggiator
- [ ] Additional filter types (HP, BP, ladder)
- [ ] Oscilloscope / waveform display

## License

MIT

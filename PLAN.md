# RustInSynth Development Plan

## Current Version: v1.2.0

## Roadmap (Ordered by Priority)

### Phase 0: Standalone ↔ Plugin Parity (1:1) ⬅️ ACTIVE

> Goal: the VST3/CLAP version must behave identically to the standalone.
> Root causes found in audit (2026-08-03): divergent MIDI CC paths, three
> different curves for the same parameters, duplicated defaults/scaling code,
> and missing host gestures in the plugin GUI.

- [x] **0.1 Single parameter spec (single source of truth)** ✅ DONE
  - New `core/param_spec.rs`: one table with id, name, min, max, default,
    curve (Linear/Log/Discrete), unit, and default CC for every `SynthParam`
  - Generate `ParamBank` defaults AND `RustInSynthParams` (nih `FloatParam`
    ranges/skews) from this table — deletes all "Match ParamBank default" drift
  - Fix range inconsistency: GUI sliders allow 0.001–5.0 s attack but CC
    mapping caps attack at `MAX_ATTACK_TIME = 2.0 s` — one range everywhere

- [x] **0.2 One curve per parameter** ✅ DONE
  - Today envelope times use THREE curves: egui `logarithmic(true)` slider,
    `cc_to_time()` quadratic, nih `FloatRange::Skewed { factor: 0.25 }`
  - Pick one (log for times); shared slider widget in `gui/widgets.rs`
    implements it; nih skew factor tuned to match; CC mapping routes through
    the spec's curve instead of `cc_to_time()`

- [x] **0.3 Fix plugin MIDI CC path (envelope controls broken)** ✅ DONE
  - Plugin forwards CC straight to `VoiceManager`, but
    `sync_plugin_params_safe()` overwrites it with param values on the NEXT
    buffer → envelope CCs in the plugin last ~1 buffer then snap back
  - Mirror the standalone: CC → atomic param store → GUI thread applies via
    `ParamSetter` (host sees automation, GUI knobs move) → DSP sync
  - Fallback when editor is closed: make `sync_plugin_params_safe`
    change-detecting (like the waveform tracking already does)

- [x] **0.4 Deduplicate CC scaling** ✅ DONE
  - `StandaloneBackend::apply_cc_to_param` and `VoiceManager::handle_param_change`
    are two separate, drifting implementations of CC→value scaling
  - Merge into one mapping in `core/params.rs` driven by the spec (0.1)

- [x] **0.5 Host automation gestures** ✅ DONE
  - Plugin GUI sliders never call `begin_set_parameter`/`end_set_parameter`
  - Add `begin_param_change`/`end_param_change` to `SynthBackend` (no-op in
    standalone, `ParamSetter` gestures in plugin); wire into shared slider
    widget via egui drag start/stop

- [x] **0.6 Cleanup & UI shell parity** ✅ DONE
  - Delete dead `sync_plugin_params()` (duplicate of `_safe` version)
  - Show CPU meter in plugin editor (data already in `PluginSharedState`)
  - Same default window size for both versions

- [x] **0.7 Parity verification** ✅ DONE (unit tests; manual checklist below)
  - Unit test: per `SynthParam`, assert default + range match between
    `ParamBank` and `RustInSynthParams`, and curve round-trips
  - Manual checklist (for the user):
    - [ ] Standalone: sliders show correct values/units, semitones snap to
      integers, MIDI CC moves GUI sliders
    - [ ] Plugin in DAW: cutoff & envelope times feel logarithmic in GUI and
      in host automation lanes; host automation updates the plugin GUI
    - [ ] Plugin MIDI CC with editor open: knob moves + host records automation
    - [ ] Plugin MIDI CC with editor closed: parameter changes audibly and
      persists (not overwritten on the next buffer)
    - [ ] Same preset file sounds identical in standalone and plugin
    - [ ] DAW automation recording works for drags (gestures)
    - [ ] Note: `bundle_vst3.sh` is macOS-only; on Linux bundle the
      CLAP/VST3 artifacts from `target/release` manually

### Phase 1: Usability Polish
- [x] **1.1 Runtime MIDI CC Learn** ✅ DONE
  - Select parameter from dropdown → move CC → mapping saved
  - Persist custom mappings to `~/.rustinsynth/cc_mappings.json`
  - Shows custom mappings in CC activity list
  - "Clear all" button to reset

- [x] **1.2 Factory Presets** ✅ DONE
  - Sub Bass, Moog Bass (deep, filtered)
  - Glide Lead (bright, with portamento + vibrato)
  - Warm Pad (slow attack, lush, LFO filter)
  - Pluck (punchy, short decay)
  - Sweep FX (noise, resonant filter sweep)
  - Wobble (dubstep LFO bass)
  - Soft Keys (electric piano style)

- [x] **1.3 Envelope & Control Curves Polish** ✅ DONE
  - Fixed retrigger clicks by clearing filter state on `note_on`
  - Converted filter envelope modulation from linear Hz to musical octaves (6-octave range)
  - Updated portamento mapping to true exponential curve (0.005s to 2.0s) with decade-based CC resolution

### Phase 2: Sound Design
- [x] **2.1 Polyphonic Mode** ✅ DONE
  - 8-voice polyphony with voice stealing
  - MONO/POLY toggle in GUI
  - Live voice count display
  - Mono mode retains key stacking

- [x] **2.2 Effects Chain** ✅ DONE
  - Delay (time, feedback, mix)
  - Reverb (Schroeder-style: 4 comb + 2 allpass)
  - Chorus (LFO-modulated delay)
  - GUI with enable toggles + parameters

- [x] **2.3 Stereo Output** ✅ DONE
  - StereoSample type with equal-power panning
  - Pan control per oscillator (-1.0 to +1.0)
  - Stereo width control (0.0 mono to 2.0 extra-wide)
  - Ping-pong delay with cross-feedback
  - True stereo reverb (different L/R comb delays)
  - Wide stereo chorus (90° LFO phase offset)
  - GUI sliders for pan and width

### Phase 2.0: GUI Aesthetics Refresh (backlog, right after parity)
- [ ] **2.0.1 Adopt the custom widgets**
  - `gui/widgets.rs` already has Minimoog-style `knob()`/toggle/selector
    widgets, but `panels.rs` uses plain egui sliders everywhere — swap them in
  - Wire the new curve-aware slider/knob from Phase 0.2
- [ ] **2.0.2 Visual polish**
  - Refresh `THEME` palette, panel backgrounds, spacing and typography
  - Consistent knob sizes/labels per section (env, filter, osc, fx)

- [ ] **2.4 Arpeggiator**
  - Up/Down/Random patterns
  - Tempo sync
  - Octave range

- [ ] **2.5 Additional Filter Types**
  - Highpass, Bandpass modes
  - 24dB/oct Ladder filter option
  - Filter drive/saturation control

### Phase 3: Visual Feedback (Nice to Have)
- [ ] **3.1 Oscilloscope / Waveform Display**
  - Real-time waveform visualization
  - Ring buffer from audio thread
  - Simple line plot in egui

- [ ] **3.2 Envelope Visualizer**
  - ADSR curve display
  - Current position indicator
  - For both amp and filter envelopes

- [ ] **3.3 Spectrum Analyzer**
  - FFT of output signal
  - Bar graph display
  - Optional (CPU intensive)

### Phase 4: Distribution
- [x] **4.1 VST3/CLAP Plugin Export** ✅ DONE
  - Integrate nih-plug
  - Wrap existing DSP
  - Cross-platform builds
  - ~200 lines glue code

- [ ] **4.2 Standalone Installer**
  - Linux: AppImage/Flatpak
  - macOS: .app bundle
  - Windows: .exe installer

- [ ] **4.3 Documentation**
  - User manual
  - Sound design guide
  - Video demo

- [ ] **4.4 Virtual Keyboard** (low priority)
  - Clickable piano keys in GUI
  - Mouse + computer keyboard input
  - Visual feedback on note press

---

## Implementation Notes

### 1.1 MIDI CC Learn - Design

**State to add:**
```rust
struct SynthApp {
    // ... existing fields
    midi_learn_target: Option<SynthParam>,  // Which param is learning
    custom_cc_map: HashMap<u8, SynthParam>, // User overrides
}
```

**UI Flow:**
1. User clicks "Learn" next to a control (or global learn button)
2. `midi_learn_target` set to that parameter
3. Next CC received → store in `custom_cc_map`
4. Clear `midi_learn_target`
5. Save to `~/.rustinsynth/cc_mappings.json`

**Files to modify:**
- `gui/app.rs` - Learn state + UI
- `gui/mod.rs` - Persist mappings
- `core/params.rs` - Load custom mappings

---

## Progress Log

| Date | Version | Changes |
|------|---------|---------|
| 2026-08-03 | — | Audited standalone↔plugin parity; identified broken plugin envelope CC path, triple curve mismatch, duplicated defaults/scaling; wrote Phase 0 parity plan + GUI aesthetics backlog |
| 2026-08-03 | — | Implemented Phase 0: `core/param_spec.rs` single spec table driving `ParamBank` + `RustInSynthParams`; shared curve-aware `param_slider` widget used by both GUIs; plugin CC path mirrors standalone (editor-open → CC queue → `ParamSetter`; editor-closed → direct apply + change-detecting `sync_plugin_params_safe`); CC scaling deduped into `param_spec::cc_to_plain` (used by DSP, standalone GUI feedback, and plugin editor); CC mapping persistence shared via `core::params`; automation gestures (`begin/end_param_change`) wired through the shared slider; dead `sync_plugin_params` deleted; editor 1500×550 + CPU meter + voice counts; parity tests in `plugin.rs` — 65 tests pass, standalone and plugin both compile |
| 2026-06-04 | v1.0.1 | Fixed stereo separation with independent L/R filters, added comprehensive panning tests, fixed plugin sustain initialization, unified stereo parameters across standalone/plugin |
| 2026-05-25 | v1.0.1 | Fixed filter envelope clicks/discontinuities on retrigger, mapped filter envelope & LFO to octaves, true exponential portamento resolution |
| 2026-05-21 | v1.0.0 | True stereo output (per-osc pan, width, stereo FX) |
| 2026-05-21 | v0.9.0 | Effects chain (Delay, Reverb, Chorus) |
| 2026-05-21 | v0.8.0 | Polyphonic mode (8 voices, MONO/POLY toggle) |
| 2026-05-21 | v0.7.2 | 8 factory presets (bass, lead, pad, FX) |
| 2026-05-21 | v0.7.1 | MIDI CC Learn with persistence |
| 2026-05-21 | v0.7.0 | CPU meter, theme system |
| 2026-05-15 | v0.6.0 | Portamento, key stacking |
| 2026-05-15 | v0.5.0 | Full GUI, MIDI CC sync |

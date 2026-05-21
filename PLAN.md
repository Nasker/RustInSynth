# RustInSynth Development Plan

## Current Version: v0.7.0

## Roadmap (Ordered by Priority)

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

- [ ] **2.4 Additional Filter Types** ⬅️ NEXT
  - Highpass, Bandpass modes
  - 24dB/oct Ladder filter option
  - Filter drive/saturation control

- [ ] **2.5 Arpeggiator**
  - Up/Down/Random patterns
  - Tempo sync
  - Octave range

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
- [ ] **4.1 VST3/CLAP Plugin Export**
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
| 2026-05-21 | v1.0.0 | True stereo output (per-osc pan, width, stereo FX) |
| 2026-05-21 | v0.9.0 | Effects chain (Delay, Reverb, Chorus) |
| 2026-05-21 | v0.8.0 | Polyphonic mode (8 voices, MONO/POLY toggle) |
| 2026-05-21 | v0.7.2 | 8 factory presets (bass, lead, pad, FX) |
| 2026-05-21 | v0.7.1 | MIDI CC Learn with persistence |
| 2026-05-21 | v0.7.0 | CPU meter, theme system |
| 2026-05-15 | v0.6.0 | Portamento, key stacking |
| 2026-05-15 | v0.5.0 | Full GUI, MIDI CC sync |

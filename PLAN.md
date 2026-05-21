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
- [ ] **2.1 Polyphonic Mode** ⬅️ NEXT
  - Voice allocation (4-8 voices)
  - Voice stealing strategies
  - Unison/detune mode
  - Major refactor ~300+ lines

- [ ] **2.2 Effects Chain**
  - Delay (tempo-sync optional)
  - Reverb (simple algorithmic)
  - Chorus/Flanger
  - Each ~200 lines

- [ ] **2.3 Additional Filter Types**
  - Highpass, Bandpass modes
  - 24dB/oct Ladder filter option
  - Filter drive/saturation control

- [ ] **2.4 Arpeggiator**
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
| 2026-05-21 | v0.7.2 | 8 factory presets (bass, lead, pad, FX) |
| 2026-05-21 | v0.7.1 | MIDI CC Learn with persistence |
| 2026-05-21 | v0.7.0 | CPU meter, theme system |
| 2026-05-15 | v0.6.0 | Portamento, key stacking |
| 2026-05-15 | v0.5.0 | Full GUI, MIDI CC sync |

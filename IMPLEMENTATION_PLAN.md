# Implementation Plan: Unified GUI Architecture

**Goal**: Refactor to use a single GUI codebase for both standalone and plugin via the `SynthBackend` trait abstraction.

**Reference**: See `UNIFIED_GUI_ARCHITECTURE.md` for the complete design.

---

## Overview

We will:
1. Create the `SynthBackend` trait abstraction
2. Implement it for standalone (wrapping existing `AudioEngine`)
3. Refactor existing standalone GUI to use the trait
4. Add missing parameters to plugin (effects, etc.)
5. Implement the trait for plugin (wrapping NIH-plug params)
6. Use the unified GUI in plugin editor

**Result**: One GUI codebase, zero duplication, guaranteed consistency.

---

## Phase 1: Create Backend Abstraction Layer

**Goal**: Define the trait that abstracts all backend differences.

### Tasks

- [x] **1.1** Create `src/gui/backend.rs`
  - Define `SynthBackend` trait with all methods
  - Parameter access: `get_param()`, `set_param()`
  - Effects control: delay, reverb, chorus (enabled, time, mix, etc.)
  - Voice management: polyphony mode, voice count
  - Stereo: width, osc pan
  - MIDI: ports (optional), CC values, learn mode, mappings
  - Status: CPU load
  - **Lines of code**: ~150

- [x] **1.2** Add `backend` module to `src/gui/mod.rs`
  ```rust
  pub mod backend;
  pub use backend::SynthBackend;
  ```

**Estimated time**: 1-2 hours

**Test**: Trait compiles, no implementation yet.

---

## Phase 2: Implement Standalone Backend

**Goal**: Wrap existing `AudioEngine` and `SharedState` in the trait.

### Tasks

- [x] **2.1** Create `src/gui/backend_standalone.rs`
  - Define `StandaloneBackend` struct
  - Fields: `shared: SharedState`, `audio_engine: AudioEngine`, `midi_handler`, etc.
  - **Lines of code**: ~400

- [x] **2.2** Implement `SynthBackend` for `StandaloneBackend`
  - Delegate `get_param()` → `self.shared.params.get()`
  - Delegate `set_param()` → `self.shared.params.set()`
  - Delegate effects → `self.audio_engine.set_delay_enabled()`, etc.
  - Delegate voice management → `self.audio_engine.polyphony_mode()`, etc.
  - Implement MIDI port management
  - Implement MIDI learn state
  - **Lines of code**: ~300

- [x] **2.3** Add helper methods to `StandaloneBackend`
  - `poll_midi()` - process MIDI events, update feedback
  - `sync_to_audio()` - sync params to audio engine
  - Move MIDI polling logic from `SynthApp::update()`
  - **Lines of code**: ~100

- [x] **2.4** Export from `src/gui/mod.rs`
  ```rust
  #[cfg(not(feature = "plugin"))]
  pub mod backend_standalone;
  ```

**Estimated time**: 3-4 hours

**Test**: Standalone backend compiles, all trait methods implemented.

---

## Phase 3: Refactor Standalone GUI to Use Backend

**Goal**: Modify `SynthApp` to use `Box<dyn SynthBackend>` instead of direct dependencies.

### Tasks

- [x] **3.1** Modify `SynthApp` struct in `src/gui/app.rs`
  - **Remove**: `shared: SharedState`, `audio_engine: AudioEngine`, `midi_handler`, `midi_ports`
  - **Add**: `backend: Box<dyn SynthBackend + 'static>`
  - **Keep**: UI state only (`preset_name`, `available_presets`, `selected_preset`)
  - **Lines changed**: ~20

- [x] **3.2** Update `SynthApp::new()`
  - Change signature: `pub fn new(backend: Box<dyn SynthBackend + 'static>) -> Self`
  - Remove audio/MIDI initialization (now in backend)
  - **Lines changed**: ~50

- [x] **3.3** Replace all parameter access
  - Find/replace: `self.get_param(` → `self.backend.get_param(`
  - Find/replace: `self.set_param(` → `self.backend.set_param(`
  - **Lines changed**: ~200

- [x] **3.4** Replace all audio engine calls
  - `self.audio_engine.set_delay_enabled(` → `self.backend.set_delay_enabled(`
  - `self.audio_engine.delay_time()` → `self.backend.delay_time()`
  - Same for reverb, chorus, polyphony, voice count, stereo width, osc pan
  - **Lines changed**: ~100

- [x] **3.5** Replace all MIDI calls
  - `self.midi_handler.is_some()` → `self.backend.midi_connected()`
  - `MidiInputHandler::list_ports()` → `self.backend.midi_ports()`
  - `self.midi_learn_target` → `self.backend.midi_learn_target()`
  - `self.custom_cc_map` → `self.backend.get_cc_mappings()`
  - **Lines changed**: ~80

- [x] **3.6** Replace CPU load access
  - `self.shared.get_cpu_load()` → `self.backend.get_cpu_load()`
  - **Lines changed**: ~5

- [x] **3.7** Update `SynthApp::update()` method
  - Remove MIDI polling (now in backend)
  - Remove param syncing (now in backend)
  - Add calls to `backend.poll_midi()` and `backend.sync_to_audio()` if standalone
  - **Lines changed**: ~50

- [x] **3.8** Update standalone `main.rs`
  - Create `StandaloneBackend`
  - Pass to `SynthApp::new()`
  - **Lines changed**: ~10

**Estimated time**: 4-5 hours

**Test**: Standalone builds and runs, all features work identically.

---

## Phase 4: Add Effects Parameters to Plugin

**Goal**: Expose the existing effects chain via NIH-plug parameters.

### Tasks

- [x] **4.1** Add effect parameters to `RustInSynthParams` in `src/plugin.rs`
  - Delay: `delay_enabled: BoolParam`, `delay_time: FloatParam`, `delay_feedback: FloatParam`, `delay_mix: FloatParam`
  - Reverb: `reverb_enabled: BoolParam`, `reverb_room_size: FloatParam`, `reverb_damping: FloatParam`, `reverb_mix: FloatParam`
  - Chorus: `chorus_enabled: BoolParam`, `chorus_rate: FloatParam`, `chorus_depth: FloatParam`, `chorus_mix: FloatParam`
  - **Lines added**: ~120

- [x] **4.2** Initialize effect parameters in `RustInSynthParams::default()`
  - Set sensible defaults matching `EffectsChain::new()`
  - Add proper ranges and units
  - **Lines added**: ~150

- [x] **4.3** Sync effect parameters in `sync_plugin_params_safe()`
  - Read parameter values
  - Update via `self.effects_chain.set_delay_enabled()`, `self.effects_chain.set_delay_time()`, etc.
  - **Lines added**: ~40

**Estimated time**: 2-3 hours

**Test**: Plugin builds, effects can be controlled via parameters (test with ParamSlider in current GUI).

---

## Phase 5: Add Shared State for Plugin

**Goal**: Create atomic state sharing between plugin audio thread and GUI.

### Tasks

- [x] **5.1** Create `src/plugin_gui/shared_state.rs`
  - Define `PluginSharedState` struct with `cpu_load: Arc<AtomicU32>`
  - Lightweight lock-free CPU load sharing between GUI and audio thread
  - **Lines added**: ~50

- [ ] **5.2** Add `PluginSharedState` to `RustInSynthPlugin`
  - **GAP**: editor currently builds its *own* `PluginSharedState` in
    `EditorState::default()`, which the audio thread never writes to → CPU
    meter is always 0 in the plugin. Addressed in Phase 10.

- [ ] **5.3** Update shared state in `process()` (write CPU load) — Phase 10

- [ ] **5.4** Pass shared state to editor via `create_editor(...)` — Phase 10

**Estimated time**: 2-3 hours

**Test**: Plugin builds, shared state updates correctly.

---

## Phase 6: Implement Plugin Backend

**Goal**: Wrap NIH-plug parameters in the `SynthBackend` trait.

### Tasks

- [x] **6.1** Create `src/plugin_gui/backend_plugin.rs`
  - Define `PluginBackend<'a>` struct
  - Fields: `params: Arc<RustInSynthParams>`, `setter: &'a ParamSetter<'a>`, `shared: PluginSharedState`
  - Lifetime `'a` avoids `'static` constraint — recreated cheaply each frame
  - **Lines added**: ~30

- [x] **6.2** Implement `SynthBackend` for `PluginBackend<'a>`
  - `get_param()`: Match on `SynthParam`, return `self.params.attack.value()`, etc.
  - `set_param()`: Match on `SynthParam`, call `self.setter.set_parameter(&self.params.attack, value)`, etc.
  - Effects: Read/write effect parameters via NIH-plug BoolParam/FloatParam
  - Voice management: Read/write polyphony mode param
  - Stereo: Read/write `stereo_width` param
  - Osc pan: Read/write osc pan params
  - CPU load: Read from `shared.cpu_load`
  - MIDI: no-ops (host handles MIDI routing)
  - **Lines added**: ~297

- [x] **6.3** Export from `src/plugin_gui/mod.rs`
  ```rust
  pub mod backend_plugin;
  pub mod shared_state;
  ```

**Estimated time**: 4-5 hours

**Test**: Plugin backend compiles, all trait methods implemented.

---

## Phase 7: Use Unified GUI in Plugin

**Goal**: Replace current plugin GUI with the unified `SynthApp`.

### Tasks

- [x] **7.1** Rewrite `src/plugin_gui/editor.rs`
  - Defined `EditorState` for persistent GUI-only state (preset name, list, selection)
  - `create_editor()` creates a fresh `PluginBackend` per frame
  - All UI panels are free functions taking `&mut dyn SynthBackend`
  - Correct 4-arg `create_egui_editor` signature: `(egui_state, user_state, build, update)`
  - **Lines**: ~472

- [⚠️] **7.2** Panel helpers "identical to standalone logic"
  - `ui_oscillators_compact`, `ui_filter_compact`, `ui_envelopes_compact`
  - `ui_lfo_compact`, `ui_effects_compact`, `ui_presets_compact`
  - **KNOWN GAP (see Phase 10)**: these were *copied* into `editor.rs` as free
    functions while `app.rs` keeps its own copies as `SynthApp` methods. This is
    duplication, not unification — the two can drift (already do: voice-count
    display differs). The "zero duplication" claim in 7.1 was premature.

- [x] **7.3** Window size set to 1100×680 for plugin

**Estimated time**: 1-2 hours

**Test**: Plugin builds, loads in DAW, GUI appears and works.

---

## Phase 8: Testing & Polish

**Goal**: Ensure both standalone and plugin work perfectly with the unified GUI.

### Tasks

- [ ] **8.1** Test standalone
  - All parameters work
  - Effects work (delay, reverb, chorus)
  - MIDI ports can be selected
  - MIDI learn works
  - Presets save/load
  - CPU meter works
  - Voice count displays correctly

- [ ] **8.2** Test plugin in DAW
  - All parameters work
  - DAW automation works
  - Effects work
  - MIDI from DAW works
  - Presets save/load (via plugin state)
  - CPU meter works
  - GUI can be closed/reopened without issues

- [x] **8.3** Code cleanup
  - Added unified `EffectsChain` wrapper API (set/get methods on chain itself)
  - Fixed `create_egui_editor` call to match actual 4-arg signature
  - `SynthBackend` trait has no `'static` bound; `Box<dyn SynthBackend + 'static>` used only in `SynthApp`
  - Zero project-source compiler errors or warnings

- [ ] **8.4** Fix any bugs found during DAW testing

**Estimated time**: 3-4 hours

**Test**: Both standalone and plugin work flawlessly, look identical.

---

## Phase 9: Documentation & Finalization

**Goal**: Document the new architecture for future maintenance.

### Tasks

- [x] **9.1** Code comments / doc-comments
  - `SynthBackend` trait methods all have inline section headers
  - `PluginBackend` struct and `EditorState` have module-level doc comments
  - `PluginSharedState`, `EffectsChain` wrappers are self-documenting

- [ ] **9.2** Update README with unified GUI architecture overview

- [ ] **9.3** Create architecture diagram (optional)

- [ ] **9.4** Archive obsolete planning docs (optional)

**Estimated time**: 1-2 hours

---

## Phase 10: Close the Unification Gaps (Audit Remediation)

**Context**: An adherence audit against `UNIFIED_GUI_ARCHITECTURE.md` found that the
abstraction layer (Phases 1–6) is solid, but the doc's core promise — *write the GUI
once* — is only half met:

1. **Panel rendering is duplicated.** The 6 panels live twice: as `SynthApp` methods
   in `src/gui/app.rs` (standalone) and as free functions in
   `src/plugin_gui/editor.rs` (plugin). They already diverge (voice-count display).
2. **`PluginSharedState` is orphaned.** The editor builds its own instance that the
   audio thread never writes → CPU meter stuck at 0; voice counts hardcoded to 0.

### Tasks

- [x] **10.1** Extract shared panels into `src/gui/panels.rs`
  - Free functions taking `&mut dyn SynthBackend`: `oscillators`, `filter`,
    `envelopes`, `lfo`, `effects`, `presets`, `midi`
  - Moved `apply_preset` / `create_preset` here
  - Added `PresetState { preset_name, available_presets, selected_preset }` for
    GUI-only preset state shared by both consumers
  - Voice-count line hidden when `max_voices() == 0` (clean for plugin)

- [x] **10.2** Refactor `SynthApp` (`src/gui/app.rs`)
  - Panel methods + preset helpers removed; now calls `panels::*`
  - Holds a `panels::PresetState`; `app.rs` shrank from ~1065 to ~188 lines

- [x] **10.3** Refactor `src/plugin_gui/editor.rs`
  - Deleted the duplicated panel free functions + `apply_preset`/`create_preset`
  - Calls `panels::*`; `EditorState { presets: PresetState, shared }`

- [x] **10.4** Wire `PluginSharedState` end-to-end
  - Stored on `RustInSynthPlugin`; CPU load measured + written in `process()`
  - Passed into `create_editor(egui_state, params, shared)` so the meter is live
  - `sample_rate` captured in `initialize()` for the load estimate
  - Added `voice_count` / `max_voices` `AtomicUsize`s to `PluginSharedState`,
    written from `process()` and read by `PluginBackend` → live voice meter

- [x] **10.5** Tidy build + finish the `plugin` feature
  - Declared `[features] plugin = []` in `Cargo.toml` (was cfg-gated but never
    defined → silenced 3 `unexpected cfg` warnings)
  - Gated `run_gui` re-export and `main.rs` behind `not(feature = "plugin")`;
    `cargo check --features plugin` now compiles cleanly
  - Removed unused imports/vars (`widgets.rs`, `theme.rs`,
    `backend_standalone.rs`) and migrated `from_id_source`/`id_source`/
    `Frame::none`/`TopBottomPanel` to current egui names in `panels.rs`/`app.rs`/
    `editor.rs`
  - Removed dead struct fields (`DelayLine::sample_rate`,
    `KeyboardInput::enhanced_keyboard`, `MidiInputHandler::debug_mode`)
  - Scoped `#[allow(deprecated)]` (with rationale) on the eframe `update` /
    `create_editor` entry points for the top-level `Panel::show(ctx)` /
    `CentralPanel::show(ctx)` calls, whose egui-0.34 `show_inside(ui)`
    replacement needs a `&mut Ui` unavailable at the top level
  - **Warnings: 30 → 0** (both default and `--features plugin`)

**Estimated time**: 3-4 hours — **DONE**. Default + `--features plugin` both
compile; `cargo test` green (48 passed).

**Test**: Both standalone and plugin build; a change to any panel is reflected in
both UIs with no second edit.

---

## Summary

### Total Estimated Time
- Phase 1: 1-2 hours (trait definition)
- Phase 2: 3-4 hours (standalone backend)
- Phase 3: 4-5 hours (refactor GUI)
- Phase 4: 2-3 hours (plugin effects params)
- Phase 5: 2-3 hours (plugin shared state)
- Phase 6: 4-5 hours (plugin backend)
- Phase 7: 1-2 hours (integrate unified GUI)
- Phase 8: 3-4 hours (testing)
- Phase 9: 1-2 hours (documentation)

**Total: 22-30 hours** of focused development

### Lines of Code
- New code: ~1,500 lines (backends, trait)
- Modified code: ~500 lines (refactoring)
- Deleted code: ~200 lines (old plugin GUI)
- **Net change**: +1,800 lines

### Risk Assessment
- **Low risk**: Trait abstraction is well-defined
- **Low risk**: Standalone backend is straightforward wrapper
- **Medium risk**: Plugin backend needs careful parameter mapping
- **Low risk**: GUI refactoring is mechanical find/replace
- **Medium risk**: Testing in multiple DAWs for compatibility

### Success Criteria
- ✅ Standalone builds and runs with no regressions
- ✅ Plugin builds and loads in DAW
- ✅ Both GUIs look identical
- ✅ All features work in both contexts
- ✅ No code duplication between standalone and plugin GUI
- ✅ Easy to add new features to both simultaneously

---

## Development Workflow

### Recommended Order
1. **Phase 1-2**: Create trait and standalone backend (can test immediately)
2. **Phase 3**: Refactor standalone GUI (test after each major change)
3. **Phase 4-5**: Add plugin infrastructure (test with current plugin GUI)
4. **Phase 6-7**: Implement plugin backend and switch to unified GUI
5. **Phase 8-9**: Test, polish, document

### Testing Strategy
- Test after each phase
- Keep both standalone and plugin building at all times
- Use git branches for major refactors
- Test in DAW frequently during Phase 6-8

### Rollback Plan
If issues arise:
- Phase 1-2: No impact on existing code
- Phase 3: Can revert standalone GUI changes
- Phase 4-5: Can disable new parameters
- Phase 6-7: Can keep old plugin GUI temporarily

---

## Next Steps

1. Review this plan
2. Confirm approach is acceptable
3. Start Phase 1: Create `SynthBackend` trait
4. Proceed incrementally, testing at each phase

**Ready to begin implementation!**

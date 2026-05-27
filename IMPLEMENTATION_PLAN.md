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

- [ ] **1.1** Create `src/gui/backend.rs`
  - Define `SynthBackend` trait with all methods
  - Parameter access: `get_param()`, `set_param()`
  - Effects control: delay, reverb, chorus (enabled, time, mix, etc.)
  - Voice management: polyphony mode, voice count
  - Stereo: width, osc pan
  - MIDI: ports (optional), CC values, learn mode, mappings
  - Status: CPU load
  - **Lines of code**: ~150

- [ ] **1.2** Add `backend` module to `src/gui/mod.rs`
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

- [ ] **2.1** Create `src/gui/backend_standalone.rs`
  - Define `StandaloneBackend` struct
  - Fields: `shared: SharedState`, `audio_engine: AudioEngine`, `midi_handler`, etc.
  - **Lines of code**: ~400

- [ ] **2.2** Implement `SynthBackend` for `StandaloneBackend`
  - Delegate `get_param()` → `self.shared.params.get()`
  - Delegate `set_param()` → `self.shared.params.set()`
  - Delegate effects → `self.audio_engine.set_delay_enabled()`, etc.
  - Delegate voice management → `self.audio_engine.polyphony_mode()`, etc.
  - Implement MIDI port management
  - Implement MIDI learn state
  - **Lines of code**: ~300

- [ ] **2.3** Add helper methods to `StandaloneBackend`
  - `poll_midi()` - process MIDI events, update feedback
  - `sync_to_audio()` - sync params to audio engine
  - Move MIDI polling logic from `SynthApp::update()`
  - **Lines of code**: ~100

- [ ] **2.4** Export from `src/gui/mod.rs`
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

- [ ] **3.1** Modify `SynthApp` struct in `src/gui/app.rs`
  - **Remove**: `shared: SharedState`, `audio_engine: AudioEngine`, `midi_handler`, `midi_ports`
  - **Add**: `backend: Box<dyn SynthBackend>`
  - **Keep**: UI state only (`preset_name`, `available_presets`, `selected_preset`)
  - **Lines changed**: ~20

- [ ] **3.2** Update `SynthApp::new()`
  - Change signature: `pub fn new(backend: Box<dyn SynthBackend>) -> Self`
  - Remove audio/MIDI initialization (now in backend)
  - **Lines changed**: ~50

- [ ] **3.3** Replace all parameter access
  - Find/replace: `self.get_param(` → `self.backend.get_param(`
  - Find/replace: `self.set_param(` → `self.backend.set_param(`
  - **Lines changed**: ~200

- [ ] **3.4** Replace all audio engine calls
  - `self.audio_engine.set_delay_enabled(` → `self.backend.set_delay_enabled(`
  - `self.audio_engine.delay_time()` → `self.backend.delay_time()`
  - Same for reverb, chorus, polyphony, voice count, stereo width, osc pan
  - **Lines changed**: ~100

- [ ] **3.5** Replace all MIDI calls
  - `self.midi_handler.is_some()` → `self.backend.midi_connected()`
  - `MidiInputHandler::list_ports()` → `self.backend.midi_ports()`
  - `self.midi_learn_target` → `self.backend.midi_learn_target()`
  - `self.custom_cc_map` → `self.backend.get_cc_mappings()`
  - **Lines changed**: ~80

- [ ] **3.6** Replace CPU load access
  - `self.shared.get_cpu_load()` → `self.backend.get_cpu_load()`
  - **Lines changed**: ~5

- [ ] **3.7** Update `SynthApp::update()` method
  - Remove MIDI polling (now in backend)
  - Remove param syncing (now in backend)
  - Add calls to `backend.poll_midi()` and `backend.sync_to_audio()` if standalone
  - **Lines changed**: ~50

- [ ] **3.8** Update standalone `main.rs`
  - Create `StandaloneBackend`
  - Pass to `SynthApp::new()`
  - **Lines changed**: ~10

**Estimated time**: 4-5 hours

**Test**: Standalone builds and runs, all features work identically.

---

## Phase 4: Add Effects Parameters to Plugin

**Goal**: Expose the existing effects chain via NIH-plug parameters.

### Tasks

- [ ] **4.1** Add effect parameters to `RustInSynthParams` in `src/plugin.rs`
  - Delay: `delay_enabled: IntParam`, `delay_time: FloatParam`, `delay_feedback: FloatParam`, `delay_mix: FloatParam`
  - Reverb: `reverb_enabled: IntParam`, `reverb_room_size: FloatParam`, `reverb_damping: FloatParam`, `reverb_mix: FloatParam`
  - Chorus: `chorus_enabled: IntParam`, `chorus_rate: FloatParam`, `chorus_depth: FloatParam`, `chorus_mix: FloatParam`
  - **Lines added**: ~120

- [ ] **4.2** Initialize effect parameters in `RustInSynthParams::default()`
  - Set sensible defaults matching `EffectsChain::new()`
  - Add proper ranges and units
  - **Lines added**: ~150

- [ ] **4.3** Sync effect parameters in `sync_plugin_params_safe()`
  - Read parameter values
  - Update `self.effects_chain.delay_enabled`, `self.effects_chain.delay.set_delay_time()`, etc.
  - **Lines added**: ~40

**Estimated time**: 2-3 hours

**Test**: Plugin builds, effects can be controlled via parameters (test with ParamSlider in current GUI).

---

## Phase 5: Add Shared State for Plugin

**Goal**: Create atomic state sharing between plugin audio thread and GUI.

### Tasks

- [ ] **5.1** Create `src/plugin_gui/shared_state.rs`
  - Define `PluginSharedState` struct
  - Fields: `cpu_load: Arc<AtomicU32>`, `voice_count: Arc<AtomicUsize>`, `max_voices: Arc<AtomicUsize>`
  - Fields: `midi_cc_values: Arc<RwLock<Vec<(u8, u8)>>>`, `midi_learn_target: Arc<RwLock<Option<SynthParam>>>`
  - Fields: `cc_mappings: Arc<RwLock<Vec<(u8, SynthParam)>>>`
  - **Lines added**: ~30

- [ ] **5.2** Add `PluginSharedState` to `RustInSynthPlugin`
  - Add field: `shared_state: Arc<PluginSharedState>`
  - Initialize in `Default::default()`
  - **Lines changed**: ~10

- [ ] **5.3** Update shared state in `process()`
  - Update CPU load (measure process time)
  - Update voice count: `self.shared_state.voice_count.store(self.voice_manager.active_voice_count())`
  - Capture MIDI CC values: `self.shared_state.midi_cc_values.write().push((cc, value))`
  - Handle MIDI learn: check `midi_learn_target`, update `cc_mappings`
  - **Lines added**: ~50

- [ ] **5.4** Pass shared state to editor
  - Modify `editor()` to pass `self.shared_state.clone()`
  - **Lines changed**: ~5

**Estimated time**: 2-3 hours

**Test**: Plugin builds, shared state updates correctly.

---

## Phase 6: Implement Plugin Backend

**Goal**: Wrap NIH-plug parameters in the `SynthBackend` trait.

### Tasks

- [ ] **6.1** Create `src/plugin_gui/backend_plugin.rs`
  - Define `PluginBackend<'a>` struct
  - Fields: `params: Arc<RustInSynthParams>`, `setter: &'a ParamSetter<'a>`, `shared: Arc<PluginSharedState>`
  - **Lines added**: ~30

- [ ] **6.2** Implement `SynthBackend` for `PluginBackend<'a>`
  - `get_param()`: Match on `SynthParam`, return `self.params.attack.value()`, etc.
  - `set_param()`: Match on `SynthParam`, call `self.setter.set_parameter(&self.params.attack, value)`, etc.
  - Effects: Read/write effect parameters
  - Voice management: Read from `shared.voice_count`, write polyphony mode param
  - Stereo: Read/write `stereo_width` param
  - Osc pan: Read/write osc pan params
  - CPU load: Read from `shared.cpu_load`
  - MIDI: Return empty ports, read CC values from `shared.midi_cc_values`, etc.
  - **Lines added**: ~500

- [ ] **6.3** Export from `src/plugin_gui/mod.rs`
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

- [ ] **7.1** Modify `src/plugin_gui/editor.rs`
  - Import `SynthApp` from `crate::gui::SynthApp`
  - In `create_editor()`, create `PluginBackend` instance
  - Create `SynthApp::new(Box::new(backend))`
  - Call `app.ui(ctx)` in the draw closure
  - **Lines changed**: ~50 (major simplification!)

- [ ] **7.2** Remove old plugin GUI code
  - Delete custom widget code in `editor.rs` (keep only `create_editor()`)
  - `src/plugin_gui/widgets.rs` can be deleted (was just a placeholder)
  - **Lines removed**: ~200

- [ ] **7.3** Update window size if needed
  - Match standalone window size (currently 1400x580)
  - Update `EDITOR_WIDTH` and `EDITOR_HEIGHT` constants
  - **Lines changed**: ~2

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
  - MIDI learn works (via DAW MIDI)
  - Presets save/load (via plugin state)
  - CPU meter works
  - Voice count displays correctly
  - GUI can be closed/reopened without issues

- [ ] **8.3** Visual consistency check
  - Both GUIs look identical
  - Same theme colors
  - Same layout
  - Same widget styles
  - Same spacing

- [ ] **8.4** Fix any bugs found
  - Parameter ranges
  - Default values
  - Edge cases
  - Performance issues

- [ ] **8.5** Code cleanup
  - Remove unused imports
  - Fix compiler warnings
  - Add documentation comments
  - Format code

**Estimated time**: 3-4 hours

**Test**: Both standalone and plugin work flawlessly, look identical.

---

## Phase 9: Documentation & Finalization

**Goal**: Document the new architecture for future maintenance.

### Tasks

- [ ] **9.1** Update README
  - Explain unified GUI architecture
  - Document the `SynthBackend` trait
  - Explain how to add new features to both

- [ ] **9.2** Add code comments
  - Document `SynthBackend` trait methods
  - Explain backend implementations
  - Comment any non-obvious code

- [ ] **9.3** Create architecture diagram
  - Visual representation of the abstraction
  - Show how standalone and plugin share GUI code

- [ ] **9.4** Clean up old documentation
  - Archive or remove obsolete planning docs
  - Keep only relevant documentation

**Estimated time**: 1-2 hours

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

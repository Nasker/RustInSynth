# GUI Implementation Plan - Hybrid Approach

## ⚠️ Important: Framework Migration
**nice-plug** is the actively maintained community fork of nih-plug. The original nih-plug is in maintenance mode. We should migrate to nice-plug before implementing the GUI.

### Migration Steps (Phase 0):
1. Update `Cargo.toml` dependencies from `nih_plug` → `nice_plug`
2. Update imports in `src/plugin.rs` from `nih_plug` → `nice_plug`
3. Update export macros: `nih_export_*!` → `nice_export_*!`
4. Test that plugin still builds and works

**Repository**: https://codeberg.org/RustAudio/nice-plug

## Overview
Port the standalone GUI to the plugin using `nice_plug_egui` widgets while preserving the visual design and layout from the existing standalone application.

## Architecture

### Current State
- **Standalone GUI**: `src/gui/` with custom widgets, theme, and `ParamBank` for parameters
- **Plugin**: No GUI, uses NIH-plug's parameter system

### Target State
- **Plugin GUI Module**: `src/plugin_gui/` with adapted widgets using nice-plug parameters
- **Shared Resources**: Reuse theme and visual styling from `src/gui/theme.rs`

## Implementation Steps

### Phase 0: Migrate to nice-plug (REQUIRED FIRST)
**Goal**: Update from nih-plug to nice-plug before GUI work

1. **Update Cargo.toml**
   ```toml
   # Change from:
   nih_plug = { git = "https://github.com/robbert-vdh/nih-plug.git", features = ["assert_process_allocs"] }
   nih_plug_egui = { git = "https://github.com/robbert-vdh/nih-plug.git" }
   
   # To:
   nice_plug = { git = "https://codeberg.org/RustAudio/nice-plug.git", features = ["assert_process_allocs"] }
   nice_plug_egui = { git = "https://codeberg.org/RustAudio/nice-plug.git" }
   ```

2. **Update plugin.rs imports**
   ```rust
   // Change from:
   use nih_plug::prelude::*;
   
   // To:
   use nice_plug::prelude::*;
   ```

3. **Update export macros at end of plugin.rs**
   ```rust
   // Change from:
   nih_export_clap!(RustInSynthPlugin);
   nih_export_vst3!(RustInSynthPlugin);
   
   // To:
   nice_export_clap!(RustInSynthPlugin);
   nice_export_vst3!(RustInSynthPlugin);
   ```

4. **Test the migration**
   - Build: `cargo build --release`
   - Bundle: `./bundle_vst3.sh`
   - Test in Ableton Live

### Phase 1: Setup & Structure
**Goal**: Create the foundation for the plugin GUI

1. **Create plugin GUI module**
   - Create `src/plugin_gui/mod.rs`
   - Create `src/plugin_gui/editor.rs` - main editor implementation
   - Create `src/plugin_gui/widgets.rs` - custom widgets adapted for NIH-plug
   - Add module declaration in `src/lib.rs`

2. **Add editor state to plugin**
   - Import `nice_plug_egui` types in `plugin.rs`
   - Add `editor_state: Arc<EguiState>` field to `RustInSynthPlugin`
   - Initialize with appropriate window size (e.g., 1000x700 to match standalone)

3. **Implement basic editor() method**
   - Return `create_egui_editor()` with empty UI
   - Test that plugin shows a blank window in Ableton

### Phase 2: Port Layout Structure
**Goal**: Recreate the visual layout from standalone

4. **Copy layout structure from `src/gui/app.rs`**
   - Analyze the standalone's panel layout (oscillators, filter, envelope, etc.)
   - Create similar structure using egui panels in plugin editor
   - Use `egui::TopBottomPanel`, `egui::SidePanel`, or `egui::CentralPanel` as needed

5. **Apply theme/styling**
   - Copy relevant styling from `src/gui/theme.rs`
   - Apply colors, fonts, spacing to match standalone aesthetic
   - Create helper function for consistent styling

### Phase 3: Port Widgets Section by Section
**Goal**: Implement interactive controls using NIH-plug widgets

#### 3.1 Oscillators Section
6. **OSC 1 Controls**
   - Waveform selector using `nih_plug_egui::widgets::ParamSlider` for int param
   - Level knob/slider using `ParamSlider` for float param
   - Phase offset control
   - Pan control

7. **OSC 2 Controls**
   - Same as OSC 1 plus:
   - Semitones detune (int param)
   - Cents detune (int param)

8. **OSC 3 Controls**
   - Same as OSC 2

**Key Pattern**:
```rust
use nice_plug_egui::widgets;

// In editor closure:
ui.add(widgets::ParamSlider::for_param(&params.osc1_level, setter));
```

#### 3.2 Filter Section
9. **Filter Controls**
   - Cutoff frequency (logarithmic slider)
   - Resonance
   - Filter envelope amount
   - Filter ADSR (attack, decay, sustain, release)

#### 3.3 Envelope Section
10. **Amplitude ADSR**
    - Attack slider
    - Decay slider
    - Sustain slider
    - Release slider
    - Visual envelope curve display (optional, can be added later)

#### 3.4 LFO Section
11. **LFO Controls**
    - Rate slider
    - Depth slider
    - Waveform selector (int param: 0-4)
    - Destination selector (int param: 0-3)

#### 3.5 Master Section
12. **Master Controls**
    - Master volume
    - Polyphony mode toggle (mono/poly)
    - Portamento time
    - Pitch bend range
    - Stereo width

### Phase 4: Custom Widgets (Optional Enhancement)
**Goal**: Recreate custom widgets from standalone if desired

13. **Adapt custom knob widget**
    - Copy logic from `src/gui/widgets.rs` knob implementation
    - Modify to use `ParamSetter` instead of direct value mutation
    - Ensure proper parameter automation support

14. **Adapt waveform selector**
    - Visual waveform display buttons
    - Use `ParamSetter` to update int parameter

15. **Adapt other custom widgets**
    - Any other specialized controls from standalone

### Phase 5: Polish & Testing
**Goal**: Ensure everything works smoothly

16. **Visual polish**
    - Match colors and spacing to standalone
    - Add icons/emojis if used in standalone
    - Ensure responsive layout

17. **Parameter automation testing**
    - Test that DAW automation works for all parameters
    - Verify parameter changes update GUI in real-time
    - Test preset loading/saving

18. **Performance optimization**
    - Ensure GUI doesn't cause audio dropouts
    - Optimize redraw frequency if needed

## Technical Details

### Using nice-plug Widgets

**Basic slider:**
```rust
use nice_plug_egui::widgets;

ui.add(widgets::ParamSlider::for_param(&params.cutoff, setter)
    .with_width(200.0));
```

**Custom styling:**
```rust
ui.add(widgets::ParamSlider::for_param(&params.resonance, setter)
    .with_width(150.0)
    .with_label("Res"));
```

**Integer parameters (for waveform selection):**
```rust
// Will show as discrete steps
ui.add(widgets::ParamSlider::for_param(&params.osc1_waveform, setter));
```

### Accessing Parameters in Editor

The editor closure receives:
- `egui_ctx: &egui::Context` - egui context
- `setter: &ParamSetter` - for updating parameters
- `state: &mut YourState` - custom editor state if needed

Parameters are accessed via the `params` Arc:
```rust
let params = self.params.clone();

create_egui_editor(
    self.editor_state.clone(),
    (), // or custom state
    |_, _| {}, // update function
    move |egui_ctx, setter, _state| {
        // Use params and setter here
        ui.add(widgets::ParamSlider::for_param(&params.attack, setter));
    },
)
```

### Handling Custom State

If you need editor-specific state (e.g., selected tab, expanded sections):
```rust
#[derive(Default)]
struct EditorState {
    selected_tab: usize,
    oscillator_expanded: bool,
}

create_egui_editor(
    self.editor_state.clone(),
    EditorState::default(),
    |_, _| {},
    move |egui_ctx, setter, state| {
        // Access state.selected_tab, etc.
    },
)
```

## File Structure After Implementation

```
src/
├── plugin.rs              # Main plugin, includes editor() method
├── plugin_gui/
│   ├── mod.rs            # Module exports
│   ├── editor.rs         # Main editor implementation
│   └── widgets.rs        # Custom widgets adapted for NIH-plug
├── gui/                  # Standalone GUI (unchanged)
│   ├── app.rs
│   ├── theme.rs          # Can be shared with plugin_gui
│   ├── widgets.rs
│   └── mod.rs
└── core/                 # Audio engine (unchanged)
```

## Benefits of This Approach

1. **Active Development**: nice-plug is actively maintained (nih-plug is in maintenance mode)
2. **Proper DAW Integration**: Parameters work with automation, presets, undo/redo
3. **Reuse Visual Design**: Keep the look and feel of standalone
4. **Maintainability**: Separate plugin GUI from standalone GUI
5. **Performance**: nice-plug widgets are optimized for plugin use
6. **Future-proof**: Easy to add features like preset browser, visualizations

## Estimated Effort

- **Phase 0 (Migration)**: 30 minutes
- **Phase 1 (Setup)**: 30 minutes
- **Phase 2 (Layout)**: 1 hour
- **Phase 3 (Widgets)**: 3-4 hours (incremental, can test each section)
- **Phase 4 (Custom Widgets)**: 2-3 hours (optional)
- **Phase 5 (Polish)**: 1-2 hours

**Total**: 8-12 hours of focused work, can be done incrementally

## Testing Strategy

After each phase:
1. Build: `cargo build --release`
2. Bundle: `./bundle_vst3.sh`
3. Test in Ableton Live
4. Verify parameter changes work
5. Check for audio dropouts or crashes

## Next Steps

1. **FIRST**: Migrate to nice-plug (Phase 0) - this is essential
2. Review this plan
3. Confirm approach is acceptable
4. Start with Phase 1 (setup & structure)
5. Implement incrementally, testing after each section

## Why nice-plug?

From the original nih-plug README:
> "NOTE: NIH-plug the plugin framework is currently in maintenance mode. If you are interested in the framework rather than the plugin, please check out this community fork instead."

nice-plug is:
- ✅ Actively maintained by the RustAudio community
- ✅ API-compatible with nih-plug (minimal migration effort)
- ✅ Receives bug fixes and new features
- ✅ Better long-term support

The migration is straightforward - mainly updating import paths and macro names.

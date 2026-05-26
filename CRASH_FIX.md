# Ableton Live Crash Fix

## Problem
The plugin crashed Ableton Live immediately upon loading with the error:
```
abort() called
assert_no_alloc::AllocDisabler::check
```

## Root Cause
The crash occurred because **memory allocation was happening in the audio processing thread**, which is strictly forbidden in real-time audio contexts. Specifically:

1. In `process()` (line 412 of plugin.rs), we called `sync_plugin_params()`
2. This function called `set_osc_waveform()` for each oscillator
3. `set_osc_waveform()` called `create_oscillator()` which allocates a `Box<dyn Oscillator>`
4. The `assert_no_alloc` guard (enabled by the `assert_process_allocs` feature) detected this allocation and aborted

**Stack trace excerpt:**
```
Thread 19 Crashed:: com.apple.audio.IOThread.client
...
30  RustInSynth  create_oscillator::h9a682b7758138c75 (oscillator.rs:389)
31  RustInSynth  OscillatorUnit::set_waveform::h38a2c3f7d20b0e88 (oscillator.rs:436)
32  RustInSynth  OscillatorBank::set_waveform::h406b6a3c861da2d4 (oscillator.rs:611)
33  RustInSynth  VoiceManager::set_osc_waveform::h04be608dd1be98d3 (voice.rs:940)
34  RustInSynth  sync_plugin_params::hfc4f8cac69bfdd02 (plugin.rs:512)
35  RustInSynth  process::h5da62078383c7e4a (plugin.rs:412)
```

## Solution
The fix involved three key changes:

### 1. Split Parameter Synchronization
Created two functions:
- **`sync_plugin_params_safe()`**: Syncs only non-allocating parameters (envelopes, filter, LFO, oscillator levels/detune/phase/pan, master controls). Safe to call in audio thread.
- **`sync_plugin_params()`**: Original function marked with `#[allow(dead_code)]` and warning comment. Contains allocating operations.

### 2. Initialize Waveforms Outside Audio Thread
Moved oscillator waveform initialization to the `initialize()` function, which runs outside the audio thread:

```rust
fn initialize(...) -> bool {
    // ... sample rate setup ...
    
    // Initialize oscillator waveforms here (safe to allocate outside audio thread)
    let w1 = WaveformType::from_index(self.params.osc1_waveform.value() as u8);
    let w2 = WaveformType::from_index(self.params.osc2_waveform.value() as u8);
    let w3 = WaveformType::from_index(self.params.osc3_waveform.value() as u8);
    self.voice_manager.set_osc_waveform(1, w1);
    self.voice_manager.set_osc_waveform(2, w2);
    self.voice_manager.set_osc_waveform(3, w3);
    
    // Track current waveforms
    self.last_osc1_waveform = self.params.osc1_waveform.value();
    self.last_osc2_waveform = self.params.osc2_waveform.value();
    self.last_osc3_waveform = self.params.osc3_waveform.value();
    
    true
}
```

### 3. Added Waveform Tracking Fields
Added fields to track the last synced waveform values:
```rust
pub struct RustInSynthPlugin {
    // ... existing fields ...
    last_osc1_waveform: i32,
    last_osc2_waveform: i32,
    last_osc3_waveform: i32,
}
```

## Current Behavior
- ✅ Plugin loads successfully in Ableton Live without crashing
- ✅ All non-waveform parameters can be changed in real-time
- ✅ GUI displays current parameter values (read-only display)
- ⚠️ Oscillator waveform changes are initialized at plugin load time
- ⚠️ To change waveforms, users need to reload the plugin or restart the DAW
- 💡 Parameters are controlled via DAW automation, not the GUI sliders

## Why It Worked Standalone But Not in Ableton
Standalone mode may:
- Not enforce strict real-time constraints
- Have more lenient memory allocation policies
- Not use the `assert_no_alloc` guard

Professional DAWs like Ableton Live enforce strict real-time audio requirements and will crash or glitch if allocations occur in the audio thread.

## Future Improvements
To support real-time waveform changes, consider:

1. **Pre-allocate all oscillator types**: Keep instances of all waveform types and switch between them
2. **Use a lock-free queue**: Send waveform change messages from a background thread
3. **Implement a custom allocator**: Use a pre-allocated memory pool for oscillator changes
4. **Use NIH-plug's background task system**: Handle waveform changes asynchronously

## Testing
After applying this fix:
1. Build: `cargo build --release`
2. Bundle: `./bundle_vst3.sh`
3. Load in Ableton Live - should work without crashes
4. Test MIDI input, parameter changes, and audio output

## Related Files
- `src/plugin.rs`: Main plugin implementation
- `src/core/oscillator.rs`: Oscillator creation and management
- `Cargo.toml`: Dependencies (note `assert_process_allocs` feature)

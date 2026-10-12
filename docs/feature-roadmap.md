# Feature Roadmap — RustInSynth

Scope: preset management overhaul, tempo sync, modulation routing matrix (extra
LFOs/envelopes), new filter types, and a multi-mode distortion unit.

Current state (relevant):
- `src/core/presets.rs` — flat `Preset` struct, one JSON file per preset in
  `default_presets_dir()`, `#[serde(default)]` for forward compat.
- `src/core/lfo.rs` — single LFO with `LfoDestination` enum (Off/Pitch/Cutoff/Amp).
- `src/core/envelope.rs` — `Envelope` trait + ADSR impls (amp + filter env in `voice.rs`).
- `src/core/effects.rs` — `StereoEffect` trait; Delay, Reverb, Chorus.
- `src/core/filter.rs` — `Filter` trait; SVF with `FilterMode::{LowPass,HighPass,BandPass}`.
- `src/core/params.rs` / `param_spec.rs` — `SynthParam` enum + `ParamSpec`
  registry; all GUI/CC/plugin params flow through it.
- `src/gui/panels.rs` — egui panels, one per module section.
- `src/plugin.rs` — nih-plug wrapper (CLAP/VST3); `plugin_gui/` hosts egui editor.

---

## 1. Modulation Routing Matrix (foundation — implement first)

Everything else (extra LFOs, extra envelopes, tempo sync) plugs into this.
Do this **before** adding new modulators so we don't build routing twice.

### 1.1 Core data model — new file `src/core/mod_matrix.rs`

```rust
pub enum ModSource {
    Lfo1, Lfo2, Lfo3,          // existing LFO becomes Lfo1
    FilterEnv, AmpEnv, Env3,   // Env3 = new aux ADSR
    Velocity, Aftertouch, ModWheel, PitchBend, KeyTrack,
    Constant,                  // for "offset" rows
}

pub enum ModDest {
    // oscillator
    Osc1Pitch, Osc2Pitch, Osc3Pitch, AllPitch,
    Osc1Level, Osc2Level, Osc3Level,
    Osc1Phase, Osc2Phase, Osc3Phase,      // phase/PM
    Osc1Pan, Osc2Pan, Osc3Pan,
    // filter
    FilterCutoff, FilterResonance,
    // amp
    Amplitude, StereoWidth,
    // effects
    DelayTime, DelayFeedback, DelayMix,
    ReverbMix, ReverbSize, ChorusRate, ChorusDepth, ChorusMix,
    DistortionDrive, DistortionMix,
    // meta: modulate another row's amount (mod-on-mod)
    Lfo1Rate, Lfo2Rate, Lfo3Rate,
    Lfo1Depth, Lfo2Depth, Lfo3Depth,
    Env3Attack, Env3Decay,
}

pub struct ModSlot {
    pub source: ModSource,
    pub dest: ModDest,
    pub amount: f32,       // -1.0..=1.0 bipolar
    pub curve: ModCurve,   // Linear | Exponential | S-curve
    pub enabled: bool,
}

pub const MOD_SLOTS: usize = 16;   // matrix rows

pub struct ModMatrix {
    pub slots: [ModSlot; MOD_SLOTS],
}
```

### 1.2 Runtime evaluation

- Per-voice sources (envelopes, velocity, keytrack, per-voice LFOs) are
  evaluated in `Voice`; global sources (mod wheel, global LFOs if LFO is
  monophonic-retriggered) evaluated once per block.
- Each `Voice` gets a `mod_offsets: EnumMap<ModDest, f32>` scratch buffer;
  per sample (or per control-block of 32/64 samples for perf):
  1. zero the buffer
  2. accumulate `source_value * amount` into each active slot's dest
  3. apply offsets on top of base parameter values
- Define each dest's **modulation range** (e.g. cutoff ±5 octaves, pitch ±12
  semitones, pan ±1.0) — a `fn dest_range(dest) -> f32` table.
- Keep the existing hardwired `lfo_destination` path working for one release
  via a default matrix row, then remove it once the matrix ships.

### 1.3 Parameter plumbing

- `ModSlot` fields need `SynthParam` entries? — No: 16 slots × 3 fields = 48
  params is too many for the flat enum. Instead expose slots to the plugin as
  dynamic params or keep them **outside** the nih-plug param list (stored in
  the preset + shared state only). Decision: keep matrix out of `SynthParam`,
  store in `Preset` and `SharedState`; automation of matrix amounts is a
  later stretch goal.
- CC mapping: allow CC-learn on matrix amounts later.

### 1.4 GUI — Matrix canvas panel

- New `src/gui/matrix_panel.rs`: egui grid — rows = slots.
  - Source dropdown, dest dropdown, bipolar amount slider, enable toggle.
  - Drag-drop not needed for v1.
- **Thumbnails**: each row shows a 48×24 `egui::Painter` waveform thumbnail —
  LFO shape + rate for LFO sources, ADSR curve for env sources. Same painter
  code reused in the LFO/env panels.
- Destination side gets a small icon/color per category (osc/filter/fx).

### 1.5 Preset serialization

`Preset` gains `#[serde(default)] pub mod_matrix: ModMatrix` — old presets
load with an empty matrix; `Preset::version` bumped to `"2"` with a migration
that converts legacy `lfo_destination` into slot 0.

---

## 2. Extra LFOs and Envelopes

### 2.1 LFOs (3 total)

- Generalize `src/core/lfo.rs` `Lfo` struct into an instance array
  `lfo_bank: [Lfo; 3]` in `Voice` (or engine, depending on retrig mode).
- Per-LFO params: waveform, rate, depth, sync mode, retrigger
  (free-run | per-note), phase offset, unipolar/bipolar switch.
- Remove `LfoDestination` entirely — routing is the matrix's job.

### 2.2 Envelopes (Env3 = aux ADSR)

- Reuse the existing `Envelope` trait + `AdsrEnvelope` impl; instantiate a
  third in `voice.rs`. No new DSP needed.
- Env3 params: ADSR + optional loop mode (A→D→A→D…) so it can act as a
  crude LFO, and a velocity-sensitivity scalar.

### 2.3 GUI

- LFO panel gets a tab strip (`LFO 1 | 2 | 3`); same for a new `ENV` section
  (`AMP | FILTER | ENV3`). Reuse existing widgets — no new controls needed
  except the sync selector and waveform thumbnail.

---

## 3. Tempo Sync

### 3.1 Tempo source

- **Plugin**: nih-plug exposes host transport via
  `context.transport()` → `tempo` (BPM) and `pos_beats`. Read per block in
  `plugin.rs` `process()`, push into engine state.
- **Standalone**: internal BPM clock — a `bpm: f32` param (default 120) +
  optional tap-tempo button in the GUI top bar.
- Store current BPM + beats position in the engine's shared state so DSP and
  GUI both see it.

### 3.2 Synced rate model

- `enum SyncRate { Hz, Div(TempoDivision) }` where
  `TempoDivision` covers `1/1, 1/2, 1/4, 1/8, 1/16, 1/32` ×
  `straight | dotted | triplet`.
- `fn rate_hz(div, bpm) -> f32` — LFO rate params switch to `SyncRate` display.
  Keep raw Hz available when sync is off.
- Apply to: LFO 1–3 rate, Env3 loop rate, **Delay time**, Chorus rate
  (optionally), Distortion bit-crusher sample-rate reduction (no — stays Hz),
  and any future time-based FX.
- LFO phase alignment: when synced, reset/advance LFO phase from host
  `pos_beats` so modulation lands on the beat in plugin mode.

### 3.3 ParamSpec

- `ParamCurve` gains nothing; instead synced params get a dual-mode `ParamSpec`
  (normalized over an indexable division list when synced). Simplest v1:
  separate bool param `lfoN_sync` + float `lfoN_rate_div` (index into division
  table), keeping `lfoN_rate` for Hz mode.

---

## 4. Preset Management (standards-aligned)

### 4.1 File format

- Keep JSON (already versioned) but wrap in a header object:
  `{ "format": "rustinsynth-preset", "version": 2, "name": ..., "category": ..., "author": ..., "tags": [...], "data": { ...existing fields... } }`
- Migration path: `version` field on old flat files → `migrate_v1_to_v2()`;
  already-established `#[serde(default)]` pattern keeps this cheap.
- New fields: `category`, `author`, `tags`, `mod_matrix`, LFO 2/3, Env3,
  distortion, filter model, sync flags.

### 4.2 Bank / library model — `src/core/preset_library.rs`

- `PresetLibrary` scans `presets_dir` + a read-only `factory/` subdir:
  - `Bank { name, presets: Vec<PresetMeta> }` — meta = name, category, tags,
    path, thumbnail hash.
- Factory bank ships embedded via `include_dir!` or generated on first run.
- User presets stay writable; factory presets save-as-copy.

### 4.3 Browser GUI — dedicated canvas

- New `src/gui/preset_browser.rs` rendered as a **separate egui
  `CentralPanel` mode** or `egui::Window` toggled from the top bar (preset
  name button opens it).
- Layout: left column = bank/category tree + tag filter + search box;
  main grid = preset cards with **thumbnail** (rendered mini
  waveform/ADSR/LFO snapshot via `egui::Painter`, same thumbnail code as the
  matrix rows), name, category badge.
- Actions: Load, Save, Save-as, Rename, Delete, Init, and
  prev/next preset buttons in the top bar for fast auditioning.
- A/B compare: optional stretch — keep a loaded snapshot to toggle.

### 4.4 Program lists in plugin

- nih-plug: expose presets as CLAP preset discovery / VST3 program list if the
  version supports it; otherwise the browser is the sole UI.

---

## 5. New Filter Types

- `src/core/filter.rs`: extend `FilterMode` into a `FilterType` enum selecting
  the algorithm, not just the SVF tap:
  - `Svf(Mode)` — existing (LP/HP/BP; add Notch + Peak taps, nearly free).
  - `Ladder` — 4-pole Moog-style ladder (24 dB/oct LP), `LadderFilter` struct,
    with drive into tanh saturation. ~150 lines DSP.
  - `Comb` — feedback/feedforward comb (cheap, great for FX-ey sounds).
  - `Formant`/`Vocal` — optional stretch, 3 parallel bandpasses.
- `Voice` holds `Box<dyn Filter>` or an enum-dispatched filter; filter type is
  a `SynthParam` (discrete). Changing type resets filter state.
- Keep cutoff/resonance params shared; ladder gains a `filter_drive` param.

## 6. Distortion Unit (effects zone)

- New `src/core/distortion.rs`, `StereoEffect` impl, chained **first** in the
  FX chain (dist → chorus → delay → reverb), with `dist_enabled`,
  `dist_type`, `dist_drive`, `dist_tone` (post-EQ), `dist_mix`, `dist_output`
  params.
- `enum DistortionType`:
  - `HardClip` — `x.clamp(-1,1)`
  - `SoftClip` — `tanh(drive*x)`
  - `Tube` — asymmetric `x/(1+|x|)` or `exp`-curve waveshaper + DC blocker
  - `Fold` — wavefolder (`sin(drive*x)` / `asinh` fold)
  - `Bitcrush` — bit-depth quantize + sample-rate decimator
  - `Fuzz` — squared/clipped + octave-ish asymmetric curve
- Bitcrush exposes `crush_bits` (1–16) and `crush_rate` (Hz) params shown only
  in that mode (GUI conditional visibility).
- Optional pre/post filter per type is a stretch goal — keep `dist_tone`
  one-knob for v1.

---

## Implementation order & milestones

| # | Milestone | Files touched | Est. |
|---|-----------|---------------|------|
| 1 | Mod matrix core + preset field, no GUI yet | `mod_matrix.rs`, `voice.rs`, `presets.rs` | M |
| 2 | LFO×3 + Env3 routed through matrix | `lfo.rs`, `envelope.rs`, `voice.rs`, `params.rs` | M |
| 3 | Matrix canvas panel + thumbnails | `matrix_panel.rs`, `panels.rs`, `widgets.rs` | M |
| 4 | Tempo sync (host + standalone BPM) | `plugin.rs`, `engine.rs`, `lfo.rs`, `effects.rs` | S–M |
| 5 | Preset format v2 + migration | `presets.rs`, `preset_library.rs` | M |
| 6 | Preset browser canvas + factory bank | `preset_browser.rs`, `app.rs`, assets | M–L |
| 7 | Filter types (ladder, comb, SVF extras) | `filter.rs`, `voice.rs`, `params.rs` | M |
| 8 | Distortion unit + FX panel slot | `distortion.rs`, `effects.rs`, `panels.rs` | S–M |
| 9 | Plugin param exposure + polish | `plugin.rs`, `param_spec.rs` | S |

Each milestone ends compilable and with a `cargo test` pass (add DSP unit
tests for ladder/comb/waveshapers and serde tests for v1→v2 migration).

## Open questions for you

- Per-voice LFOs (3 × polyphonic LFO instances) or global monophonic LFOs?
  Per-voice is the usual synth answer but triples LFO DSP cost.
- Presets embedded factory bank vs. shipped `.json` files?
- Should matrix amounts be host-automatable in the plugin (adds ~48 params)
  or preset-only for now?
- Distortion position: fixed first-in-chain, or reorderable FX slots later?

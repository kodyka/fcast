# Phase 15 — Camera Capture Controls reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-15-camera-capture-controls.md`][spec] to the current `senders/android` tree.
**Goal:** add a `CameraPage` settings sub-page (camera-source / resolution / framerate cyclers, mirror / stabilization / tap-to-focus toggles, zoom slider with snap-preset chips) wired into the `Panel` overlay layer, and link to it from `FullSettingsPage` under the existing `AUDIO & VIDEO` section (introduced in Phase 14).
**Scope:** Slint UI only. **No Rust changes.** All controls flip inline `in-out` properties on `CameraPage` itself. Phase 8 swaps these for `Bridge.*` reads + setters tied to Android `CameraX` / GStreamer `camerasrc`.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-15-camera-capture-controls.md

> **Read Phase 14's guide first.** This guide is structurally identical to [`PHASE-14-reimplement-instructions.md`][p14] — same chrome, same row primitives, same gotchas. Where Phase 14 already explains a pattern in depth, this guide just points at it. The new things in Phase 15 are: a `for ... in [literal-array]:` loop (zoom presets), a small internal `PresetChip` component, and a continuous `Slider` whose value is compared against discrete preset values.

[p14]: ./PHASE-14-reimplement-instructions.md

---

## Why this guide exists

Phase 15 is the second sub-page in the `AUDIO & VIDEO` section (after Phase 14's `Audio` row). The PHASE-15 spec snippet:

- Uses `(value + 1) mod N` infix → invalid Slint, must be `Math.mod(value + 1, N)`.
- Uses `(value + 1)` cycle handlers and `toggled => { ... = !... }` toggle handlers — both pitfalls already documented in Phase 14's gotchas section.
- Sketches the zoom-preset chip row as a `for preset in [0.5, 1.0, 2.0, 5.0]: PresetChip { ... }` loop without specifying where `PresetChip` lives — this guide pins it down (declare it inline in `camera_page.slint`, do not pollute `components/`).
- Concatenates `preset + "×"` with a `float`, which is **invalid Slint** — `+` doesn't autoconvert numeric to string. The guide replaces it with explicit interpolation.

After Phase 14 merges, the relevant chassis state is:

- `Panel { none, settings, debug, codec-test, audio }` — Phase 15 adds `camera`.
- `FullSettingsPage` has an `AUDIO & VIDEO` section with one `Audio` row — Phase 15 appends a `Camera` row in the same section (no new section).
- `pages/audio_page.slint` is the closest reference panel — the new `CameraPage` should follow its shape exactly.

This is **strictly additive** Slint work spread across **three existing files** (one Bridge edit, one main.slint edit, one settings_page.slint edit) plus **one new file** (`pages/camera_page.slint`).

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'CameraPage\|Panel\.camera\|mock-camera-idx\|mock-zoom-level\|PresetChip' \
    senders/android/ui/

# Panel enum has 5 variants after Phase 14 (Phase 15 adds one more):
grep -n 'export enum Panel' -A 8 senders/android/ui/bridge.slint
# Expected: variants `none, settings, debug, codec-test, audio`.

# main.slint routes 4 panel overlays after Phase 14:
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected: 4 matches (settings / debug / codec-test / audio).

# FullSettingsPage already has the AUDIO & VIDEO section with exactly one row:
grep -n 'AUDIO & VIDEO\|Panel\.audio' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches (section title + Audio row's clicked handler).
```

After this guide is applied:

```sh
# CameraPage exists and exports a Rectangle.
grep -rn 'export component CameraPage inherits Rectangle' senders/android/ui/
# Expected: 1 match in pages/camera_page.slint.

# PresetChip is declared INSIDE camera_page.slint (not lifted to components/).
grep -rn 'export component PresetChip\|component PresetChip' senders/android/ui/
# Expected: 1 match — `component PresetChip inherits Rectangle {` in camera_page.slint.
#           NOT in components/ — keep zoom-specific UI co-located.

# Panel enum has 6 variants
grep -n 'camera,' senders/android/ui/bridge.slint
# Expected: 1 match.

# main.slint routes 5 overlays
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected: 5 matches.

# FullSettingsPage's AUDIO & VIDEO section has 2 rows now
grep -n 'Panel\.audio\|Panel\.camera' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

# CameraPage uses Math.mod and bound toggle args, never the spec's broken forms
grep -n 'mod\|toggled' senders/android/ui/pages/camera_page.slint
# Expected: 3 Math.mod matches + toggle handlers all `toggled(checked) => ...`.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-15-camera-page
cargo check -p android-sender   # baseline must pass
```

---

## Step 1 — Add `Panel.camera` variant in `bridge.slint`

```diff
 export enum Panel {
     none,
     settings,
     debug,
     codec-test,
     audio,
+    camera,
 }
```

Same reasoning as Phase 14 Step 1 — see [`PHASE-14-reimplement-instructions.md`][p14] §"Step 1". Lowercase variant name to match `audio`.

---

## Step 2 — Route `Panel.camera` in `main.slint`

```diff
 import { AudioPage }                    from "pages/audio_page.slint";
+import { CameraPage }                   from "pages/camera_page.slint";
 import { DebugPage, FullDebugPage }     from "pages/debug_page.slint";
```

```diff
     if Bridge.active-panel == Panel.audio:      AudioPage { }
+    if Bridge.active-panel == Panel.camera:     CameraPage { }
 }
```

Same reasoning as Phase 14 Step 2.

---

## Step 3 — Create `pages/camera_page.slint`

**File:** `senders/android/ui/pages/camera_page.slint` (new)

Three sections (`SOURCE`, `IMAGE`, `ZOOM`) plus an internal `PresetChip` component declared in the same file. The chip is declared as a non-`export`ed `component` so it lives only inside `camera_page.slint` — keep zoom-specific UI co-located until a second use site appears (Phase 27 utils backlog covers eventual extraction).

### New file

```slint
// camera_page.slint — Camera capture settings sub-page (UI-only placeholder).
//
// Reachable from FullSettingsPage's "Camera" row in the AUDIO & VIDEO
// section, which sets `Bridge.active-panel = Panel.camera`. All controls
// flip inline `in-out` properties on this component — no Rust round-trip.
// Phase 8 will swap the inline state for Bridge.* setters tied to Android
// CameraX / GStreamer camerasrc once Rust camera capability lands.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsValueRow,
    SettingsToggleRow,
    SettingsSliderRow,
} from "../components/settings_rows.slint";

// Internal component — not exported. Used only inside CameraPage's zoom row.
// If a second use site appears, lift to components/ in a follow-up phase
// (Phase 27 utils backlog).
component PresetChip inherits Rectangle {
    in property <string> label;
    in property <bool>   active: false;
    callback clicked();

    height: 32px;
    width: max(48px, t.preferred-width + 16px);
    border-radius: 16px;
    background: root.active
        ? Theme.accent-active
        : (ta.pressed ? Theme.surface-card.brighter(20%) : Theme.surface-card);

    ta := TouchArea {
        clicked => { root.clicked(); }
    }
    t := Text {
        text: root.label;
        color: Theme.text-primary;
        horizontal-alignment: center;
        vertical-alignment: center;
        font-size: Theme.font-size-label;
    }
}

export component CameraPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <int>   mock-camera-idx:     1;    // 0 Front / 1 Back / 2 External
    in-out property <int>   mock-resolution-idx: 2;    // 0 480p / 1 720p / 2 1080p / 3 4K
    in-out property <int>   mock-framerate-idx:  1;    // 0 24 / 1 30 / 2 60 fps
    in-out property <bool>  mock-mirror-front:   true;
    in-out property <bool>  mock-stabilization:  true;
    in-out property <bool>  mock-tap-to-focus:   true;
    in-out property <float> mock-zoom-level:     1.0;  // 0.5 .. 5.0

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Camera";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    clicked => { Bridge.active-panel = Panel.none; }
                }
            }
        }

        // ── Body (scrollable) ───────────────────────────────────────────
        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── Section: SOURCE ─────────────────────────────────────
                SettingsSection {
                    title: "SOURCE";
                    SettingsValueRow {
                        title: "Camera";
                        value: ["Front", "Back", "External"][root.mock-camera-idx];
                        clicked => {
                            root.mock-camera-idx = Math.mod(root.mock-camera-idx + 1, 3);
                        }
                    }
                    SettingsValueRow {
                        title: "Resolution";
                        value: ["480p", "720p", "1080p", "4K"][root.mock-resolution-idx];
                        clicked => {
                            root.mock-resolution-idx = Math.mod(root.mock-resolution-idx + 1, 4);
                        }
                    }
                    SettingsValueRow {
                        title: "Framerate";
                        value: ["24 fps", "30 fps", "60 fps"][root.mock-framerate-idx];
                        clicked => {
                            root.mock-framerate-idx = Math.mod(root.mock-framerate-idx + 1, 3);
                        }
                    }
                }

                // ── Section: IMAGE ──────────────────────────────────────
                SettingsSection {
                    title: "IMAGE";
                    SettingsToggleRow {
                        title: "Mirror front camera";
                        checked: root.mock-mirror-front;
                        toggled(checked) => { root.mock-mirror-front = checked; }
                    }
                    SettingsToggleRow {
                        title: "Video stabilization";
                        checked: root.mock-stabilization;
                        toggled(checked) => { root.mock-stabilization = checked; }
                    }
                    SettingsToggleRow {
                        title: "Tap to focus";
                        checked: root.mock-tap-to-focus;
                        toggled(checked) => { root.mock-tap-to-focus = checked; }
                    }
                }

                // ── Section: ZOOM ───────────────────────────────────────
                SettingsSection {
                    title: "ZOOM";
                    SettingsSliderRow {
                        title: "Current zoom";
                        unit: "×";
                        minimum: 0.5;
                        maximum: 5.0;
                        value <=> root.mock-zoom-level;
                        // No unit conversion needed — slider domain matches
                        // storage domain. See gotcha below for when `<=>` is
                        // safe vs. when you must use the Phase-14 split form.
                    }

                    // Preset chip row.  `for` over an inline literal array
                    // keeps the preset list one-line-per-edit. The chip is
                    // an internal component declared above.
                    Rectangle {
                        height: 48px;
                        HorizontalLayout {
                            padding-left:  Theme.padding-screen;
                            padding-right: Theme.padding-screen;
                            spacing: 8px;
                            alignment: start;
                            for preset in [0.5, 1.0, 2.0, 5.0]: PresetChip {
                                // Numeric → string conversion: use string
                                // interpolation `\{...}`, NOT the `+` operator.
                                // `+` between a float and a string is rejected
                                // by the Slint compiler (no implicit numeric
                                // → string coercion).
                                label: preset == 1.0 ? "1×" : "\{preset}×";
                                active: root.mock-zoom-level == preset;
                                clicked => { root.mock-zoom-level = preset; }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`component PresetChip inherits Rectangle { ... }` (no `export`)** — internal-only declarations are valid Slint and don't pollute the public component namespace. Per [file.mdx][file], every top-level component is reachable from inside the file; only `export` makes it visible to importers. Keep `PresetChip` non-exported until it's needed in a second file.
- **`background: root.active ? Theme.accent-active : (ta.pressed ? Theme.surface-card.brighter(20%) : Theme.surface-card);`** — same active/pressed/idle ladder used by `QuickActionButton` in `components/control_bar.slint`. The `.brighter(20%)` color builder is documented in the [colors][colors] reference.
- **`width: max(48px, t.preferred-width + 16px);`** — chip auto-sizes to its label with horizontal padding, but never shrinks below 48 px (Material minimum touch target). `t.preferred-width` reads the inner `Text`'s natural width; see [text.mdx][text] for `preferred-width`. The `t :=` ID in front of the `Text { ... }` block is required to address the element from elsewhere in the tree.
- **`for preset in [0.5, 1.0, 2.0, 5.0]: PresetChip { ... }`** — Slint allows literal array models in `for` loops. Each iteration has the loop variable in scope; no `index` / `model` boilerplate needed. See [repetition-and-data-models.mdx][repeat].
- **`label: preset == 1.0 ? "1×" : "\{preset}×";`** — string interpolation `\{...}` converts the float to its natural string form. The spec's `preset + "×"` is invalid Slint (no `+` between `float` and `string`) and was caught by the Slint compiler the first time anyone tried to compile it. Use interpolation. See [expressions-and-statements.mdx][expressions].
- **`active: root.mock-zoom-level == preset`** — float equality is exact in Slint (no NaN, no epsilon). Because the only writer to `mock-zoom-level` is the chip's own `clicked => { root.mock-zoom-level = preset; }`, the value is always exactly one of the listed floats — no FP drift to worry about. (The slider _can_ produce intermediate floats like `1.234`, in which case the chip equality is false and no chip lights up, which is the desired behaviour.)
- **Cyclers use `Math.mod(idx + 1, N)`** — same canonical form as Phase 14. The infix `mod` operator does not exist in Slint; see [`PHASE-14-reimplement-instructions.md`][p14] §"Gotcha 2".
- **Toggles use `toggled(checked) => { ... = checked }`** — bind the argument, never `!root.mock-*`. See [`PHASE-14-reimplement-instructions.md`][p14] §"Gotcha 1".
- **Zoom slider uses `value <=> root.mock-zoom-level` directly** — no unit conversion needed because slider domain (0.5..5.0) matches storage domain (0.5..5.0). The Phase 14 split form (`value: x*100; changed(v) => x=v/100`) is only required when slider display ≠ storage units. **`<=>` is the cleaner pattern when units match** because it avoids the read-only-slider trap entirely. See [`PHASE-14-reimplement-instructions.md`][p14] §"Gotcha 3" for when each form applies.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean.

---

## Step 4 — Append to `FullSettingsPage`'s `AUDIO & VIDEO` section

**File:** `senders/android/ui/pages/settings_page.slint`

Phase 14 created the `AUDIO & VIDEO` section. Phase 15 appends a sibling row to it — **no new section**.

### Diff

```diff
                 // ── Section: AUDIO & VIDEO ────────────────────────────────
                 SettingsSection {
                     title: "AUDIO & VIDEO";
                     SettingsValueRow {
                         title: "Audio";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.audio; }
                     }
+                    SettingsValueRow {
+                        title: "Camera";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.camera; }
+                    }
                 }
```

No new imports needed — `Bridge` and `Panel` are already imported by Phase 14.

### Build check

```sh
cargo build -p android-sender
```

Exit-criterion build — must pass.

---

## Sanity grep before commit

```sh
# 1. Panel enum has 6 variants.
grep -n 'export enum Panel' -A 8 senders/android/ui/bridge.slint

# 2. main.slint routes 5 panels.
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected: 5 matches.

# 3. CameraPage cyclers use Math.mod (3 sites).
grep -n 'Math\.mod' senders/android/ui/pages/camera_page.slint
# Expected: 3 matches.

# 4. CameraPage toggles bind args (3 sites).
grep -n 'toggled(checked)' senders/android/ui/pages/camera_page.slint
# Expected: 3 matches.

# 5. PresetChip declared inline, NOT in components/.
grep -rn 'PresetChip' senders/android/ui/
# Expected: only inside pages/camera_page.slint.

# 6. No invalid `preset + "×"` concatenation (it would not compile).
grep -n '+ "×"\|"×" +' senders/android/ui/pages/camera_page.slint
# Expected: (empty)

# 7. CameraPage fills the window.
grep -n 'width: 100%\|height: 100%' senders/android/ui/pages/camera_page.slint
# Expected: 2 matches.

# 8. Settings root has 2 rows in AUDIO & VIDEO.
grep -n 'Panel\.audio\|Panel\.camera' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/
git status
# Expected (4 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/camera_page.slint
git commit -m "feat(slint-ui): Phase 15 — camera capture controls sub-page (UI-only)"
```

---

## Gotchas (Phase 15 specific)

The three Phase 14 gotchas (toggle feedback, `Math.mod`, slider unit conversion) all apply here — re-read [`PHASE-14-reimplement-instructions.md`][p14] §"Gotchas". Phase 15 adds three more:

### Gotcha 4 — Numeric → string concatenation

**Symptom:** Slint compiler error `expected 'string', got 'float'` or similar around `preset + "×"`.

**Cause:** the spec snippet writes `label: preset + "×";`, which assumes `+` autoconverts numeric to string (JS-style). Slint's `+` is strictly arithmetic on numerics or string-on-string; there is no implicit coercion.

**Fix:** use string interpolation.

```slint
label: preset == 1.0 ? "1×" : "\{preset}×";   // ✅
```

### Gotcha 5 — Internal vs exported components

**Symptom:** future PR introduces `import { PresetChip } from "../pages/camera_page.slint";` from a sibling file → unrelated breakage when `camera_page.slint` is later refactored.

**Cause:** `PresetChip` is camera-zoom-specific. Importing it from another page would couple unrelated concerns.

**Fix:** declare `PresetChip` as a top-level component **without** `export`. The compiler still allows it to be used inside the same file. If a second use case appears, lift to `components/preset_chip.slint` in a separate phase (Phase 27 utils backlog) — never inline an import path that crosses page boundaries.

### Gotcha 6 — Float equality on `active`

**Symptom:** preset chips never highlight because `mock-zoom-level == 1.0` is `false` after a slider drag landed at `1.0000001`.

**Cause:** if you also bind `mock-zoom-level` from anywhere that produces non-exact floats (e.g. a future Bridge setter converting from a fixed-point integer), the equality test will spuriously fail.

**Fix (Phase 15-only — UI is the only writer):** the slider's `value <=> root.mock-zoom-level` is the only continuous writer, but it can produce arbitrary floats. The chips will only light up when the user explicitly taps a chip — this is acceptable for the placeholder. **Do not** add a `Math.abs(mock-zoom-level - preset) < 0.001` check yet; defer to Phase 8 once the real zoom value source is known. Note this limitation in the PR description.

---

## Exit criteria checklist

- [ ] `bridge.slint` exposes `Panel.camera`.
- [ ] `main.slint` shows `CameraPage` based on `Panel.camera`.
- [ ] `CameraPage` renders `SOURCE` / `IMAGE` / `ZOOM` sections.
- [ ] All three cyclers (Camera / Resolution / Framerate) flip stub state with `Math.mod(...)`.
- [ ] All three toggles (Mirror / Stabilization / Tap to focus) bind the argument.
- [ ] Zoom slider uses `<=>` two-way bind (units match).
- [ ] Four preset chips (`0.5×` / `1×` / `2×` / `5×`) snap the slider value and visually highlight when their float exactly matches `mock-zoom-level`.
- [ ] `FullSettingsPage`'s `AUDIO & VIDEO` section has two rows: `Audio` + `Camera`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <int>    camera-idx;
+    in property <int>    camera-resolution-idx;
+    in property <int>    camera-framerate-idx;
+    in-out property <bool> camera-mirror-front;
+    in-out property <bool> camera-stabilization;
+    in-out property <bool> camera-tap-to-focus;
+    in-out property <float> camera-zoom-level;
+
+    callback set-camera(int);
+    callback set-camera-resolution(int);
+    callback set-camera-framerate(int);
+    callback set-camera-zoom(float);
```

Functional integration (deferred):
- `set-camera(int)` → Android `CameraX.cameraSelector` (FRONT / BACK / EXTERNAL).
- `set-camera-resolution(int)` → CameraX `ResolutionStrategy`.
- `set-camera-framerate(int)` → CameraX `FrameRateRange`.
- `set-camera-zoom(float)` → CameraX `Camera.cameraControl.setZoomRatio(...)`.
- Mirror / stabilization / tap-to-focus → CameraX `setMirrorMode` / `Preview.Builder.setStabilizationMode` / `MeteringPointFactory`.

---

## Slint-doc references used

- **`Panel.camera` enum extension** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **Internal (non-exported) component declaration** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`for preset in [0.5, 1.0, 2.0, 5.0]: PresetChip { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **String interpolation `"\{preset}×"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`Math.mod(a, b)`** — `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`.
- **`Slider.value` two-way `<=>`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx`.
- **`TouchArea.pressed`** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`Rectangle.background` and `Color.brighter(percent)`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx` and `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **`Text.preferred-width`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`.
- **`ScrollView` auto-derived viewport** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`.
- **Conditional element `if Bridge.active-panel == Panel.camera: CameraPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`SettingsValueRow` / `SettingsToggleRow` / `SettingsSliderRow` / `SettingsSection`** — FCast components in `senders/android/ui/components/settings_rows.slint`. Property names finalised by [PR #8](https://github.com/varxe-alt/fcast/pull/8).

---

## What's NOT in this guide

- **Real `CameraX` capability detection** — Phase 8.
- **Capture preview surface** — Phase 12 (`CapturePreview`); does not cross-cut here.
- **HDR / portrait / cinematic capture toggles** — deferred indefinitely (Moblin-specific).
- **Per-camera zoom range capability detection** — `CameraX.cameraInfo.zoomState` lookup, Phase 8.
- **Float-equality tolerance on preset chip `active:`** — defer to Phase 8 once real zoom source is known. UI-only build is fine with exact equality.
- **Pinch-to-zoom on the preview surface** — Phase 12 + a future polish phase.
- **`@tr(...)` wrapping** of `"Camera"` / `"Front"` / `"Back"` / `"External"` / `"480p"` / `"720p"` / `"1080p"` / `"4K"` / `"24 fps"` / `"30 fps"` / `"60 fps"` / `"SOURCE"` / `"IMAGE"` / `"ZOOM"` / `"Mirror front camera"` / `"Video stabilization"` / `"Tap to focus"` / `"Current zoom"` → Phase 9 (localization sweep).

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-15-camera-capture-controls.md
[p14]: ./PHASE-14-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[text]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx
[colors]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx

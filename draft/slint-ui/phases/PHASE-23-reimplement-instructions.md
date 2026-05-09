# Phase 23 — Local Recording Controls reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-23-local-recording.md`][spec] to the current `senders/android` tree.
**Goal:** add a `RecordingPage` settings sub-page with a state-cycling record button (idle / recording / paused / finalizing), a 1-second elapsed counter driven by a Slint `Timer`, format/folder cyclers, audio toggle, and a read-only disk-free row. Wired into the `Panel` overlay layer; linked from `FullSettingsPage` `ADVANCED` section.
**Scope:** Slint UI only. **No Rust changes.** No `MediaRecorder`, no actual file output, no `StatFs` probe. State machine lives entirely in Slint properties.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-23-local-recording.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] and [`PHASE-22-reimplement-instructions.md`][p22] first.** Phase 22 introduces the `Timer` element and the `ADVANCED` section; both are reused here. The new things in Phase 23 are: a `RecordingState` enum compared against in expressions, a single button whose visual state cycles by enum, and a non-trivial `mock-elapsed-s += 1` increment in a Timer callback (Slint compound assignment).

[p14]: ./PHASE-14-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md

---

## Why this guide exists

Phase 23 is the second `ADVANCED` sub-page (after Phase 22's `Network`). It's the simplest of Wave 2's UI-only additions, but introduces three patterns not yet in the chassis:

1. **`RecordingState` enum used in property bindings**, e.g. `running: root.mock-state == RecordingState.recording`. Slint's reactive engine re-evaluates the boolean whenever the enum changes — no manual `watch` setup needed.
2. **Compound assignment `mock-elapsed-s += 1`** inside a Timer body. Slint supports `+=` / `-=` etc. on numeric properties (per [expressions-and-statements.mdx][expressions]) — `triggered => { root.mock-elapsed-s += 1; }` is exactly what you want.
3. **Time formatting `HH:MM:SS`** without a stdlib date helper. Slint's `Math.floor()` + `Math.mod()` + string interpolation gets you there; the guide pins down a small inline expression that handles single-digit pad-zero correctly.

After Phases 14 + 15 + 16 + 21 + 22 merge:

- `Panel { ..., network }`. Phase 23 adds `recording`.
- `FullSettingsPage` has the `ADVANCED` section with one `Network` row. Phase 23 appends a `Recording` row.
- `bridge.slint` exports `BitratePreset`, `NetworkInterface` structs. Phase 23 adds a `RecordingState` enum.

This is **strictly additive** Slint work spread across **three existing files** plus **one new file**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'RecordingPage\|Panel\.recording\|RecordingState\|mock-elapsed-s' \
    senders/android/ui/

# Phase 22's ADVANCED section is in place:
grep -n 'ADVANCED\|Panel\.network' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.
```

After this guide is applied:

```sh
grep -n 'export enum RecordingState' senders/android/ui/bridge.slint   # Expected: 1
grep -n 'recording,' senders/android/ui/bridge.slint                   # Expected: 2 (in Panel enum + RecordingState enum)
grep -n 'export component RecordingPage' senders/android/ui/pages/recording_page.slint   # Expected: 1
grep -n 'Timer {' senders/android/ui/pages/recording_page.slint        # Expected: 1
grep -n 'Math\.mod\|Math\.floor' senders/android/ui/pages/recording_page.slint   # Expected: 3+ (HH:MM:SS formatting)
grep -n 'Panel\.recording' senders/android/ui/pages/settings_page.slint   # Expected: 1
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-23-recording-page
cargo check -p android-sender
```

---

## Step 1 — Add `RecordingState` enum + `Panel.recording` in `bridge.slint`

```diff
+export enum RecordingState {
+    idle,
+    recording,
+    paused,
+    finalizing,
+}
+
 export struct NetworkInterface { ... }
```

```diff
 export enum Panel {
     ...
     network,
+    recording,
 }
```

`RecordingState` is a closed set — the four states cover the lifecycle. If a future phase needs an `error` state, append it (don't replace `finalizing`).

---

## Step 2 — Route `Panel.recording` in `main.slint`

```diff
 import { NetworkPage }                  from "pages/network_page.slint";
+import { RecordingPage }                from "pages/recording_page.slint";
```

```diff
     if Bridge.active-panel == Panel.network:         NetworkPage { }
+    if Bridge.active-panel == Panel.recording:       RecordingPage { }
 }
```

---

## Step 3 — Create `pages/recording_page.slint`

**File:** `senders/android/ui/pages/recording_page.slint` (new)

### New file

```slint
// recording_page.slint — Local recording controls (UI-only placeholder).
//
// Reachable from FullSettingsPage's "Recording" row in the ADVANCED
// section. State machine lives entirely in Slint properties; no
// MediaRecorder integration. Phase 8 will swap mock-state for a
// Bridge-published RecordingState and wire start/pause/stop callbacks
// to a Rust-side recorder pipeline.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, RecordingState } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton, PrimaryButton, DestructiveButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsValueRow,
    SettingsToggleRow,
} from "../components/settings_rows.slint";

export component RecordingPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <RecordingState> mock-state:        RecordingState.idle;
    in-out property <int>             mock-elapsed-s:    0;
    in-out property <int>             mock-format-idx:   0;        // MP4 / MKV / WebM
    in-out property <int>             mock-folder-idx:   0;        // App / Movies / Custom
    in-out property <bool>            mock-record-audio: true;
    in-out property <int>             mock-disk-free-mb: 12480;

    // Pure-derived display strings.
    property <string> elapsed-display: format-elapsed(root.mock-elapsed-s);
    property <string> disk-free-display:
        root.mock-disk-free-mb >= 1024
            ? "\{Math.round(root.mock-disk-free-mb / 102.4) / 10} GB"
            : "\{root.mock-disk-free-mb} MB";

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // ── 1-second tick driving the elapsed counter ────────────────────────
    Timer {
        interval: 1s;
        running: root.mock-state == RecordingState.recording;
        triggered => { root.mock-elapsed-s += 1; }
    }

    // Pure helper. Slint global functions are one-line expressions; for
    // multi-line logic, wrap in a `function` declaration like this one.
    pure function format-elapsed(total-s: int) -> string {
        return "\{Math.floor(total-s / 3600)}:"
             + (Math.mod(Math.floor(total-s / 60), 60) < 10
                 ? "0\{Math.mod(Math.floor(total-s / 60), 60)}"
                 : "\{Math.mod(Math.floor(total-s / 60), 60)}")
             + ":"
             + (Math.mod(total-s, 60) < 10
                 ? "0\{Math.mod(total-s, 60)}"
                 : "\{Math.mod(total-s, 60)}");
    }

    // Centralised state transitions — keeps the button click handler
    // readable.
    function on-record-clicked() {
        if (root.mock-state == RecordingState.idle) {
            root.mock-state = RecordingState.recording;
            root.mock-elapsed-s = 0;
        } else if (root.mock-state == RecordingState.recording) {
            root.mock-state = RecordingState.paused;
        } else if (root.mock-state == RecordingState.paused) {
            root.mock-state = RecordingState.recording;
        }
        // RecordingState.finalizing is reached only via the Stop button.
    }

    function on-stop-clicked() {
        if (root.mock-state == RecordingState.recording
         || root.mock-state == RecordingState.paused) {
            root.mock-state = RecordingState.finalizing;
            // UI-only: finalize is instant.
            root.mock-state = RecordingState.idle;
            root.mock-elapsed-s = 0;
        }
    }

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Recording";
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

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── Section: RECORDING ──────────────────────────────────
                SettingsSection { title: "RECORDING"; }
                Rectangle {
                    height: 200px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    VerticalLayout {
                        alignment: center;
                        spacing: Theme.spacing-default;

                        // Big record button — visual state cycles by enum.
                        Rectangle {
                            width: 96px;
                            height: 96px;
                            border-radius: 48px;
                            background:
                                root.mock-state == RecordingState.idle      ? #cc0000
                                : root.mock-state == RecordingState.recording ? #cc0000
                                : root.mock-state == RecordingState.paused    ? Theme.accent-active
                                : Theme.surface-primary;
                            // Inner glyph: red dot (idle), white square (recording),
                            // chevron (paused), spinner placeholder (finalizing).
                            Rectangle {
                                width: root.mock-state == RecordingState.recording ? 32px : 48px;
                                height: root.mock-state == RecordingState.recording ? 32px : 48px;
                                border-radius:
                                    root.mock-state == RecordingState.recording
                                        ? 4px       // square stop hint
                                        : 24px;     // dot
                                background: white;
                            }
                            TouchArea {
                                clicked => { root.on-record-clicked(); }
                            }
                        }

                        // Elapsed time HH:MM:SS.
                        Text {
                            text: root.elapsed-display;
                            color: Theme.text-primary;
                            font-size: Theme.font-size-heading;
                            horizontal-alignment: center;
                        }

                        // Stop button — only enabled when recording or paused.
                        HorizontalLayout {
                            alignment: center;
                            spacing: Theme.spacing-default;
                            DestructiveButton {
                                label: "Stop";
                                enabled: root.mock-state == RecordingState.recording
                                    || root.mock-state == RecordingState.paused;
                                clicked => { root.on-stop-clicked(); }
                            }
                        }
                    }
                }

                // ── Section: OUTPUT ─────────────────────────────────────
                SettingsSection {
                    title: "OUTPUT";
                    SettingsValueRow {
                        title: "Format";
                        value: ["MP4", "MKV", "WebM"][root.mock-format-idx];
                        clicked => {
                            root.mock-format-idx = Math.mod(root.mock-format-idx + 1, 3);
                        }
                    }
                    SettingsValueRow {
                        title: "Folder";
                        value: ["App", "Movies", "Custom"][root.mock-folder-idx];
                        clicked => {
                            root.mock-folder-idx = Math.mod(root.mock-folder-idx + 1, 3);
                        }
                    }
                    SettingsToggleRow {
                        title: "Record audio";
                        checked: root.mock-record-audio;
                        toggled(checked) => { root.mock-record-audio = checked; }
                    }
                    SettingsValueRow {
                        title: "Disk free";
                        value: root.disk-free-display;
                        show-chevron: false;
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`Timer { running: root.mock-state == RecordingState.recording; }`** — Slint's [`Timer.running`][timer] is a reactive expression. Whenever `mock-state` changes, the binding is re-evaluated; the timer auto-starts when `running` becomes `true` and auto-stops when it becomes `false`. Pausing the recording stops the elapsed counter; resuming resumes it.
- **`triggered => { root.mock-elapsed-s += 1; }`** — Slint's `+=` compound assignment on numeric properties. Per [expressions-and-statements.mdx][expressions], `+=`, `-=`, `*=`, `/=` are valid in callback bodies. Cleaner than `root.mock-elapsed-s = root.mock-elapsed-s + 1;`.
- **`pure function format-elapsed(total-s: int) -> string { ... }`** — Slint allows pure functions on a component. Marking it `pure` lets it be used in property bindings (impure functions can only run from callback bodies). See [functions-and-callbacks.mdx][callbacks].
- **HH:MM:SS formatter via `Math.floor` + `Math.mod` + interpolation + zero-pad ternary** — Slint has no `printf`-style formatter or padding helper. The ternary `< 10 ? "0\{x}" : "\{x}"` is the canonical zero-pad workaround; same idiom in the existing `codec_test_page.slint`'s timing display. See [math.mdx][mathmod].
- **`property <string> elapsed-display: format-elapsed(root.mock-elapsed-s);`** — declaring it as a derived property (not inlining the call site in the `Text { text: ... }`) means the function is called once per `mock-elapsed-s` change and the result is cached. Inlining works too but evaluates more often if multiple bindings reference the elapsed time.
- **Ternary chain on `background:`** for the big record button visual state — Slint allows nested ternaries. Per [expressions-and-statements.mdx][expressions]. Each branch returns a `brush` (color literal `#cc0000` or theme reference).
- **Inner glyph sizing controlled by ternary** on width/height/border-radius — the same Rectangle morphs from a 48-px dot to a 32-px square based on state. Explicit CSS-style transitions (`animate width: 200ms ease;`) are Phase polish; defer.
- **`DestructiveButton.enabled: <expr>`** — assumes `DestructiveButton` exposes an `in property <bool> enabled` matching the standard pattern in `components/buttons.slint`. Confirm against the local definition before applying; if `enabled` doesn't exist there, add a wrapping `if root.mock-state == RecordingState.recording || ... : DestructiveButton { ... }` instead.
- **`finalizing` state is "set then immediately reset to `idle`"** — UI-only build: there's no real finalize step. The state visit is preserved (so a future `if mock-state == finalizing: Spinner { ... }` overlay would briefly appear) without any actual delay. When Phase 8 wires this up, the Rust side will hold `finalizing` until the muxer actually flushes.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Append to `FullSettingsPage`'s `ADVANCED` section

**File:** `senders/android/ui/pages/settings_page.slint`

```diff
                 // ── Section: ADVANCED ─────────────────────────────────────
                 SettingsSection {
                     title: "ADVANCED";
                     SettingsValueRow {
                         title: "Network";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.network; }
                     }
+                    SettingsValueRow {
+                        title: "Recording";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.recording; }
+                    }
                 }
```

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. RecordingState enum + Panel.recording present.
grep -n 'export enum RecordingState\|^\s*recording,\b' senders/android/ui/bridge.slint
# Expected: 2+ matches (RecordingState decl + Panel.recording variant + RecordingState.recording variant).

# 2. RecordingPage uses a Timer reactively.
grep -n 'running: root.mock-state == RecordingState.recording' \
    senders/android/ui/pages/recording_page.slint
# Expected: 1 match.

# 3. RecordingPage uses Math.floor + Math.mod for HH:MM:SS.
grep -n 'Math\.floor\|Math\.mod' senders/android/ui/pages/recording_page.slint
# Expected: 5+ matches (formatter + cycler handlers).

# 4. Compound assignment on elapsed counter.
grep -n 'mock-elapsed-s += 1' senders/android/ui/pages/recording_page.slint
# Expected: 1 match.

# 5. ADVANCED section has 2 rows.
grep -n 'Panel\.network\|Panel\.recording' senders/android/ui/pages/settings_page.slint
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
#   new file:   senders/android/ui/pages/recording_page.slint
git commit -m "feat(slint-ui): Phase 23 — local recording controls (UI-only)"
```

---

## Gotchas (Phase 23 specific)

### Gotcha 16 — Compound assignment is in callback bodies only

**Symptom:** writing `mock-elapsed-s: mock-elapsed-s + 1;` at the property-binding level loops forever (or fails to compile depending on Slint version).

**Cause:** property bindings are reactive expressions, not imperative statements. `mock-elapsed-s: mock-elapsed-s + 1` would set up a self-referential binding (re-evaluate-then-write-then-re-evaluate). Slint's compiler usually rejects this; older versions accepted it and produced infinite loops.

**Fix:** compound assignment (`+=`) is valid only inside `triggered => { ... }`, `clicked => { ... }`, or `function ... { ... }` bodies — places where Slint expects imperative side-effecting statements. Never at the binding level.

### Gotcha 17 — `Timer.running` re-evaluates on enum change

**Symptom (subtle):** if the elapsed counter is implemented as `Timer { running: true; }` and gated on enum inside the `triggered =>` body via `if (mock-state == RecordingState.recording) { mock-elapsed-s += 1; }`, the timer keeps firing even when paused — wasting CPU.

**Fix (already in the snippet):** put the gate on `running:`, not in the body. Slint will stop the underlying OS-level timer when `running: false`. Keep the body unconditional.

### Gotcha 18 — `pure function` requirement for use in bindings

**Symptom:** Slint compiler error "function `format-elapsed` is not pure" when writing `text: format-elapsed(root.mock-elapsed-s);`.

**Cause:** [functions-and-callbacks.mdx][callbacks] requires functions used in property bindings to be marked `pure`. Pure means: returns a value, no observable side effects, only reads inputs.

**Fix:** add the `pure` modifier — `pure function format-elapsed(total-s: int) -> string { ... }`. The function in this guide already has it. If you copy from another component without the modifier, add it.

### Gotcha 19 — Single zero-pad expression duplicated three times

**Symptom:** the HH:MM:SS formatter looks unwieldy because the same zero-pad-ternary is repeated for hours, minutes, seconds.

**Cause:** Slint has no string-format primitive or recursive helper. You can't write `pad2(x)` because it would recursively call itself with `x < 10 ? ... : ...`, and Slint pure functions can't recursively reference themselves cleanly.

**Fix:** extract a `pad2(n: int) -> string` pure function on the component:

```slint
pure function pad2(n: int) -> string {
    return n < 10 ? "0\{n}" : "\{n}";
}
pure function format-elapsed(total-s: int) -> string {
    return "\{Math.floor(total-s / 3600)}:"
         + root.pad2(Math.mod(Math.floor(total-s / 60), 60))
         + ":"
         + root.pad2(Math.mod(total-s, 60));
}
```

The `root.pad2(...)` qualified call form is required when calling one component-scope pure function from inside another. The original snippet inlined the ternary three times for clarity; if you prefer the helper form, both work.

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `RecordingState` enum (4 variants) and `Panel.recording`.
- [ ] `main.slint` routes `Panel.recording`.
- [ ] Big record button visual state cycles `idle → recording → paused → recording` on tap.
- [ ] Elapsed counter ticks once per second only when `mock-state == RecordingState.recording`.
- [ ] Pause stops the counter; resume resumes from where it stopped.
- [ ] Stop button is disabled when idle, enabled when recording or paused; tapping it returns to idle and resets elapsed.
- [ ] Format / Folder cyclers cycle on click using `Math.mod`.
- [ ] Audio toggle binds the `toggled(checked)` argument.
- [ ] `Disk free` row shows formatted MB / GB.
- [ ] `FullSettingsPage`'s `ADVANCED` section has 2 rows: `Network` + `Recording`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <RecordingState> recording-state;
+    in property <int>             recording-elapsed-s;
+    in property <int>             recording-disk-free-mb;
+
+    callback start-recording();
+    callback pause-recording();
+    callback resume-recording();
+    callback stop-recording();
+    callback set-recording-format(int);
+    callback set-recording-folder(int);
+    callback set-recording-audio(bool);
```

- `recording-state` ← Rust polls `MediaRecorder` state; pushes via `slint::Weak`. The Slint-side state machine helpers (`on-record-clicked` / `on-stop-clicked`) are deleted; their bodies become Rust callback dispatches.
- `recording-elapsed-s` ← Rust ticks once per second via `tokio::time::interval`. The Slint-side `Timer` is deleted.
- `recording-disk-free-mb` ← Rust polls `StatFs.availableBlocksLong * blockSizeLong` periodically.
- Format / folder cyclers swap to `set-recording-*(int)` callbacks; Rust holds the canonical idx.
- The "Quick-action shortcut" task (PHASE-23 task 23-C) lands in Phase 17 (`PHASE-17-quick-action-customization.md`); the `id: "record"` entry is added to `mock-quick-actions` and routes through `Bridge.active-panel = Panel.recording`.

---

## Slint-doc references used

- **`export enum RecordingState`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **Compound assignment `+=`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`pure function name(arg: type) -> type { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **`Math.floor`, `Math.mod`, `Math.round`** — `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`.
- **String interpolation `"\{n}"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **Nested ternary `cond ? a : (cond ? b : c)`** — same.
- **`if (...) { ... } else if (...) { ... }` imperative blocks** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Conditional element `if Bridge.active-panel == Panel.recording: RecordingPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`PrimaryButton`, `DestructiveButton`, `TextButton`** — FCast components in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real `MediaRecorder` start/stop.** Phase 8.
- **Real `StatFs` disk-free probe.** Phase 8.
- **Real file output / Storage Access Framework integration.** Phase 8.
- **Trim / clip post-processing.** Out of scope.
- **Background-recording service.** Out of scope.
- **Quick-action shortcut for record.** Phase 17.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-23-local-recording.md
[p14]: ./PHASE-14-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[callbacks]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
[mathmod]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx
[timer]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx

# Phase 14 — Audio Capture Controls reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-14-audio-capture-controls.md`][spec] to the current `senders/android` tree.
**Goal:** add an `AudioPage` settings sub-page (source cycler, mute toggle, input-gain slider, bitrate cycler, codec read-only row), wire it into the `Panel` overlay layer added in Phase 7, and link to it from `FullSettingsPage` under a new `AUDIO & VIDEO` section.
**Scope:** Slint UI only. **No Rust changes.** All four interactive controls flip inline `in-out` properties on `AudioPage` itself — no `Bridge` callbacks, no Rust round-trip. Phase 8 (Rust bridge) will swap the inline state for `Bridge.*` properties later; Phase 14 is the UI-only placeholder.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-14-audio-capture-controls.md

---

## Why this guide exists

Phase 14 is the first of the **fan-out of UI-only settings sub-pages** unblocked by Phase 7's Panel routing chassis (Phases 14, 15, 16, 18, 19, 21, 22, 23, 26, …). It is intentionally simple — three new code edits across three files plus one new file — but the spec snippet has three small pitfalls that have already been caught and fixed in earlier phases. This guide:

1. Restates the spec's intent against the **actual** post-Phase-7 baseline (the codebase uses `Math.mod(...)` and `width: 100%` — slightly different from the spec snippet's wording).
2. Calls out the three traps so they don't recur (toggle feedback bug, missing `std-widgets` import, missing root dimensions).
3. Cites the specific upstream Slint docs (mirrored under `draft/slint-ui/docs/`) that justify each pattern.

After Phase 7 merged, the relevant chassis state is:

- `bridge.slint` exports `Panel { none, settings, debug, codec-test }` — Phase 14 extends this with one variant: `audio`.
- `main.slint` has a panel overlay layer (`if Bridge.active-panel == Panel.X: XPage { }`) — Phase 14 adds one more conditional.
- `FullSettingsPage` (in `pages/settings_page.slint`) renders four sections: `RECEIVER`, `VIDEO QUALITY`, `CODEC & DEBUG`, `ABOUT` — Phase 14 inserts a fifth: `AUDIO & VIDEO`.
- `components/settings_rows.slint` already exports `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`, `SettingsSliderRow`. **All five row props use `title:`, callbacks are `clicked()` / `toggled(bool)` / `changed(float)`** — these names are post-PR-#8 canonical and Phase 14 uses them as-is.
- `components/buttons.slint` already exports `TextButton` for the Done button.
- `pages/codec_test_page.slint` is the closest reference for "panel-style page with header chrome" — the new `AudioPage` should follow its shape exactly.

This is **strictly additive** Slint work spread across **three existing files** (one Bridge edit, one main.slint edit, one settings_page.slint edit) plus **one new file** (`pages/audio_page.slint`). Rust does not need to be touched.

### Audit before you start

Run these from the repo root to confirm the pre-state:

```sh
# Should be ZERO matches before you start:
grep -rn 'AudioPage\|Panel\.audio\|mock-source-idx\|mock-input-gain\|mock-bitrate-idx' \
    senders/android/ui/

# Panel enum currently has exactly 4 variants (Phase 14 adds one):
grep -n 'export enum Panel' -A 6 senders/android/ui/bridge.slint
# Expected:
#   19:export enum Panel {
#   20:    none,
#   21:    settings,
#   22:    debug,
#   23:    codec-test,
#   24:}

# main.slint has exactly 3 panel overlays (Phase 14 adds one):
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected (3 matches — the re-export line uses bare `Panel`, no dot):
#   68:    if Bridge.active-panel == Panel.settings:   FullSettingsPage { }
#   69:    if Bridge.active-panel == Panel.debug:      FullDebugPage { }
#   70:    if Bridge.active-panel == Panel.codec-test: CodecTestPage { }

# FullSettingsPage currently has exactly 4 SettingsSection blocks (plus the
# import line at the top):
grep -n 'SettingsSection\b' senders/android/ui/pages/settings_page.slint
# Expected: 5 occurrences — 1 import + 4 sections (RECEIVER, VIDEO QUALITY,
# CODEC & DEBUG, ABOUT).

# Existing cycle-on-click handlers use Math.mod(a, b), not infix `mod`:
grep -n 'Math\.mod\b\|\bmod\b' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches, both Math.mod(...) — Phase 14 will follow the same pattern.

# Confirm the panel-style page reference is on disk:
test -f senders/android/ui/pages/codec_test_page.slint && echo "OK"
```

After this guide is applied:

```sh
# AudioPage exists and is exported
grep -rn 'export component AudioPage' senders/android/ui/
# Expected:
#   senders/android/ui/pages/audio_page.slint:NN:export component AudioPage inherits Rectangle {

# Panel enum has 5 variants
grep -n 'export enum Panel' -A 7 senders/android/ui/bridge.slint
# Expected: ... codec-test, audio, ...

# main.slint has 4 panel overlays
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected: 4 matches (one per `if Bridge.active-panel == Panel.X:` branch —
# the re-export line uses bare `Panel` and is not matched by `Panel\.`).

# FullSettingsPage has 5 SettingsSection blocks (plus the import line)
grep -n 'SettingsSection\b' senders/android/ui/pages/settings_page.slint
# Expected: 6 occurrences — 1 import + 5 sections (adds AUDIO & VIDEO).

# Cycle-on-click handlers in audio_page.slint use Math.mod, not infix mod
grep -n 'mod' senders/android/ui/pages/audio_page.slint
# Expected: 2 matches, both Math.mod(...) — never `(x + 1) mod N` infix.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-14-audio-page
```

Build commands you'll run as you go:

```sh
cargo check -p android-sender                  # fastest sanity check
cargo build -p android-sender                  # full build (exit criterion)
```

---

## Step 1 — Add `Panel.audio` variant in `bridge.slint`

**File:** `senders/android/ui/bridge.slint`
**Spec:** PHASE-14 task 14-B (first half).

The `Panel` enum is the single source of truth for which overlay page is shown. Adding a new variant is a one-line change.

### Diff

```diff
 export enum Panel {
     none,
     settings,
     debug,
     codec-test,
+    audio,
 }
```

### Why each piece

- **Enum extension is the entire bridge change.** No new property, no new callback. `Bridge.active-panel: Panel` already exists from Phase 7 and accepts the new value automatically — enums are closed value sets, but extending the set in Slint and rebuilding fans the new variant out to every consumer (`if Bridge.active-panel == Panel.audio: ...`) without any further wiring. See [structs-and-enums.mdx][structs] (`export enum CardSuit { clubs, diamonds, hearts, spade }`) and [properties.mdx][props] (`in-out` direction).
- **Trailing comma is required to stay consistent with the existing enum** — Slint accepts both with and without a trailing comma on the last variant, but every other enum in `bridge.slint` (`AppState`, `StatusSeverity`) uses trailing commas. Match the existing style.
- **Variant name is `audio` (lowercase, kebab-style).** All existing variants are lowercase; `codec-test` shows that hyphenation works. Stick to lowercase to keep the `Panel.audio` reference style uniform.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. The new variant is unused at this point; Slint allows that.

---

## Step 2 — Route `Panel.audio` in `main.slint`

**File:** `senders/android/ui/main.slint`
**Spec:** PHASE-14 task 14-B (second half).

`main.slint` already has the panel overlay layer added in Phase 7. We add one import and one conditional element.

### Diff

```diff
 import { CodecTestPage }                from "pages/codec_test_page.slint";
+import { AudioPage }                    from "pages/audio_page.slint";
 import { DebugPage, FullDebugPage }     from "pages/debug_page.slint";
```

```diff
     if Bridge.active-panel == Panel.settings:   FullSettingsPage { }
     if Bridge.active-panel == Panel.debug:      FullDebugPage { }
     if Bridge.active-panel == Panel.codec-test: CodecTestPage { }
+    if Bridge.active-panel == Panel.audio:      AudioPage { }
 }
```

### Why each piece

- **Conditional elements are the canonical Slint way to swap overlay pages.** Per [file.mdx][file]: *"The syntax is `if condition : id := Element { ... }`."* Only the matching branch is instantiated, so the four panels can coexist as siblings without any of them paying construction cost when hidden.
- **Order matters only for visual stacking, not for correctness.** All four `if` blocks are mutually exclusive (a single `Panel` value), so put the `Panel.audio` branch last alongside the other three to keep the file diff small.
- **The panel overlays are siblings of the `VerticalLayout`, not children.** Phase 7 deliberately placed them outside the `VerticalLayout` so they render *above* the `CastControlBar` rather than competing with it for vertical space. Don't move the new branch inside the `VerticalLayout` — see [positioning-and-layouts.mdx][layouts] on layout vs non-layout containers.

### Build check

```sh
cargo check -p android-sender
```

This will fail with `cannot find import 'pages/audio_page.slint'` because the file doesn't exist yet. That's expected — Step 3 creates it.

---

## Step 3 — Create `pages/audio_page.slint`

**File:** `senders/android/ui/pages/audio_page.slint` (new)
**Spec:** PHASE-14 task 14-A.

Panel-style page following the same chrome as `CodecTestPage`: a fixed-height `surface-card` header with a title and a `Done` button, then a `ScrollView` of two `SettingsSection` blocks for `INPUT` and `ENCODING`.

### New file

```slint
// audio_page.slint — Audio capture settings sub-page (UI-only placeholder).
//
// Reachable from FullSettingsPage's "Audio" row in the new "AUDIO & VIDEO"
// section, which sets `Bridge.active-panel = Panel.audio`. All four
// interactive controls flip inline `in-out` properties on this component
// — no `Bridge` callbacks, no Rust round-trip. Phase 8 will swap these
// inline mocks for Bridge.* setters wired to Android `AudioRecord` and
// the GStreamer audio capsfilter / encoder bitrate.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx
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

export component AudioPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    // All flips live on this component. Phase 8 swaps these for Bridge
    // setters + Rust-backed handlers (AudioRecord source, GStreamer
    // audio capsfilter, encoder bitrate).
    in-out property <bool>  mock-muted:        false;
    in-out property <int>   mock-source-idx:   0;    // 0 Mic / 1 System / 2 Both
    in-out property <float> mock-input-gain:   0.7;  // 0..1 (display in 0..100%)
    in-out property <int>   mock-bitrate-idx:  1;    // 0 64 / 1 128 / 2 192 / 3 256

    // Fill the parent (the panel overlay layer in main.slint passes
    // parent.width / parent.height through implicitly via 100%).
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
                    text: "Audio";
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

                // ── Section: INPUT ──────────────────────────────────────
                SettingsSection {
                    title: "INPUT";
                    SettingsValueRow {
                        title: "Source";
                        value: ["Microphone", "System audio", "Both"][root.mock-source-idx];
                        clicked => {
                            root.mock-source-idx = Math.mod(root.mock-source-idx + 1, 3);
                        }
                    }
                    SettingsToggleRow {
                        title: "Mute";
                        checked: root.mock-muted;
                        toggled(checked) => { root.mock-muted = checked; }
                    }
                    SettingsSliderRow {
                        title: "Input gain";
                        unit: "%";
                        minimum: 0;
                        maximum: 100;
                        // Display 0..100; storage 0..1. Conversion lives
                        // entirely in the changed handler — see "Slider unit
                        // conversion" in the gotchas section below.
                        value: root.mock-input-gain * 100;
                        changed(v) => { root.mock-input-gain = v / 100; }
                    }
                }

                // ── Section: ENCODING ───────────────────────────────────
                SettingsSection {
                    title: "ENCODING";
                    SettingsValueRow {
                        title: "Bitrate";
                        value: ["64 kbps", "128 kbps", "192 kbps", "256 kbps"][root.mock-bitrate-idx];
                        clicked => {
                            root.mock-bitrate-idx = Math.mod(root.mock-bitrate-idx + 1, 4);
                        }
                    }
                    SettingsValueRow {
                        title: "Codec";
                        value: "AAC-LC";
                        show-chevron: false;
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`width: 100%; height: 100%;`** — `AudioPage` is rendered as a sibling conditional inside `MainWindow`, **not** inside a layout container. Without explicit dimensions it would size to its content and float at `(0, 0)` with whatever intrinsic size its children compute. `100%` makes it cover the parent. The Phase 14 spec snippet only sets `background:` on the root and omits the dimensions — that omission would leave the panel collapsed. See [positioning-and-layouts.mdx][layouts] and [rectangle.mdx][rect]. (The same pattern is used by `FullSettingsPage` and `CodecTestPage` — match them verbatim.)
- **`background: Theme.surface-primary`** — opaque body so the page underneath doesn't bleed through. `surface-primary` (`#0b1020`) is the canonical body chrome from `theme.slint`.
- **Header `Rectangle { height: 56px; background: Theme.surface-card }`** — Material-style fixed-height header bar identical to the Phase 7 panels. `surface-card` (`#222633`) gives it a subtle contrast against the body. The 56 px is intentionally a literal (Phase 2-H carved out an exception for intrinsic widget heights).
- **`HorizontalLayout { padding: Theme.padding-screen }`** with a stretching `Text` and a trailing `TextButton` — the canonical "title left, action right" pattern. `horizontal-stretch: 1` on the title pushes the button to the right edge.
- **`TextButton { clicked => { Bridge.active-panel = Panel.none; } }`** — close button writes the property directly. No `close-panel()` callback (Phase 7 deliberately avoided that round-trip).
- **`ScrollView { VerticalLayout { ... } }`** — `ScrollView` auto-derives `viewport-width` / `viewport-height` from its child layout when one is present. See [scrollview.mdx][scrollview]: *"If the ScrollView contains a layout, the default value for the `viewport-width` and `viewport-height` is the minimum size of that layout."* The body can scroll when `INPUT` + `ENCODING` exceed the viewport.
- **`SettingsSection { title: "..."; ... }` + nested rows** — `SettingsSection` is a `VerticalLayout` with a `@children` slot that prepends a small grey label header (`components/settings_rows.slint:134-148`). All rows use `title:` (post-PR-#8 canonical name).
- **`SettingsValueRow { value: [ "..." ][idx]; clicked => { idx = Math.mod(idx + 1, N); } }`** — the canonical "cycle on tap" pattern from Phase 7. Indexed string lookups on inline array literals are documented in [repetition-and-data-models.mdx][repeat] + [expressions-and-statements.mdx][expressions]. **Use `Math.mod(a, b)`, not `(a) mod b` infix** — see the gotchas section below.
- **`SettingsValueRow { title: "Codec"; value: "AAC-LC"; show-chevron: false; }`** — read-only info row. The `show-chevron: false` removes the trailing `›` glyph (`components/settings_rows.slint:38`). Same pattern as the App version / FCast protocol rows in `FullSettingsPage`'s `ABOUT` section.
- **`SettingsToggleRow { toggled(checked) => { root.mock-muted = checked; } }`** — bind the new value, never compute it. The handler signature is `toggled(bool)` (`components/settings_rows.slint:76`). See "Toggle feedback bug" in the gotchas section.
- **`SettingsSliderRow { value: root.mock-input-gain * 100; changed(v) => { root.mock-input-gain = v / 100; } }`** — one-way bind for display, two-way push-back via the callback. See "Slider unit conversion" in the gotchas section.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. `AudioPage` is now exported and referenced from `main.slint`; the build resolves.

If the check fails with `unknown property: 'mock-source-idx'` or similar, you've likely shadowed a stub property in a parent scope — re-paste the file from this guide.

---

## Step 4 — Link to `AudioPage` from `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`
**Spec:** PHASE-14 task 14-C.

Adds a fifth section (`AUDIO & VIDEO`) with a single row that opens the audio panel.

### Diff

Insert the new section between `VIDEO QUALITY` and `CODEC & DEBUG`. The placement is intentional: the existing `VIDEO QUALITY` section already covers max resolution and max framerate, so `AUDIO & VIDEO` reads as the natural next sibling for capture-side controls. (Future Phases 15 and 16 will extend this section with rows for `Camera` and `Bitrate presets`.)

```diff
                 // ── Section: VIDEO QUALITY ────────────────────────────────
                 // Cycle-on-click is a placeholder UX. Real picker panels
                 // land in `futures/`.
                 SettingsSection {
                     title: "VIDEO QUALITY";
                     SettingsValueRow {
                         title: "Max resolution";
                         value: ["480p", "720p", "1080p", "1440p"][root.resolution-idx];
                         clicked => {
                             root.resolution-idx = Math.mod(root.resolution-idx + 1, 4);
                         }
                     }
                     SettingsValueRow {
                         title: "Max framerate";
                         value: ["24 fps", "30 fps", "60 fps"][root.framerate-idx];
                         clicked => {
                             root.framerate-idx = Math.mod(root.framerate-idx + 1, 3);
                         }
                     }
                 }

+                // ── Section: AUDIO & VIDEO ────────────────────────────────
+                // Sub-pages for capture-side controls.  Phase 14 adds the
+                // Audio entry; Phase 15 will append Camera; Phase 16 will
+                // append Bitrate presets.
+                SettingsSection {
+                    title: "AUDIO & VIDEO";
+                    SettingsValueRow {
+                        title: "Audio";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.audio; }
+                    }
+                }
+
                 // ── Section: CODEC & DEBUG ────────────────────────────────
                 SettingsSection {
                     title: "CODEC & DEBUG";
```

### Why each piece

- **`SettingsValueRow { value: "Open"; clicked => { Bridge.active-panel = Panel.audio; } }`** — same pattern the existing `H.264 encoder test` row uses to open `Panel.codec-test`. The row is read as "Audio … Open ›" — the trailing `›` (default `show-chevron: true`) hints that tapping opens a sub-page.
- **No new imports needed.** `Bridge` and `Panel` are already imported at the top of `settings_page.slint` (line 22). The new row uses only existing imports.
- **Section title `AUDIO & VIDEO` (not `AUDIO`)** — anticipates Phases 15 (Camera) and 16 (Bitrate presets) landing in the same section. Avoids a section rename in two PRs' time. The PHASE-14 spec calls out this naming explicitly: *"add a row in a new 'AUDIO & VIDEO' section."*
- **Section placement matters slightly.** Inserting between `VIDEO QUALITY` and `CODEC & DEBUG` keeps capture-side concerns adjacent and pushes diagnostic concerns (`CODEC & DEBUG`) and meta concerns (`ABOUT`) toward the bottom. The spec doesn't pin the exact ordering — pick this one and the future Camera / Bitrate phases stay diff-clean.

### Build check

```sh
cargo build -p android-sender
```

This is the **exit-criterion build** — must pass. If it fails, re-check that `Panel.audio` is in `bridge.slint` and that `pages/audio_page.slint` exists.

---

## Sanity grep before commit

```sh
# 1. Panel enum has all 5 variants exactly once.
grep -n 'export enum Panel' -A 7 senders/android/ui/bridge.slint
# Expected: variants `none, settings, debug, codec-test, audio` in that order.

# 2. main.slint routes all 4 panels.
grep -n 'Panel\.' senders/android/ui/main.slint
# Expected: 4 matches — one per `if Bridge.active-panel == Panel.X:` branch.

# 3. AudioPage exists and inherits Rectangle.
grep -n 'export component AudioPage inherits Rectangle' \
    senders/android/ui/pages/audio_page.slint
# Expected: 1 match.

# 4. AudioPage uses Math.mod, never infix `mod` (post-PR-#8 canonical).
grep -n '\bmod\b' senders/android/ui/pages/audio_page.slint
# Expected: 2 matches, both Math.mod(...).

# 5. AudioPage's toggle handler binds the argument, never recomputes.
grep -n 'toggled(' senders/android/ui/pages/audio_page.slint
# Expected: 1 match — `toggled(checked) => { root.mock-muted = checked; }`.
# A `!root.mock-muted` form would be the feedback bug fixed by PR #8.

# 6. FullSettingsPage has the new AUDIO & VIDEO section with exactly one row.
grep -n 'AUDIO & VIDEO\|Panel\.audio' senders/android/ui/pages/settings_page.slint
# Expected:
#   ~line: title: "AUDIO & VIDEO";
#   ~line: clicked => { Bridge.active-panel = Panel.audio; }

# 7. AudioPage explicitly fills the window — no implicit-size traps.
grep -n 'width: 100%\|height: 100%' senders/android/ui/pages/audio_page.slint
# Expected: 2 matches (width and height).

# 8. AudioPage imports ScrollView from std-widgets (not just relying on
#    a transitive import).
grep -n 'import { ScrollView } from "std-widgets.slint"' \
    senders/android/ui/pages/audio_page.slint
# Expected: 1 match.

# 9. No accidental `label:` on settings rows (the PR-#8 trap).
grep -n 'label:.*Source\|label:.*Mute\|label:.*Input gain\|label:.*Bitrate\|label:.*Codec\|label:.*Audio' \
    senders/android/ui/pages/audio_page.slint senders/android/ui/pages/settings_page.slint
# Expected: (empty)

# 10. Cargo build passes.
cargo build -p android-sender
```

If commands 1-9 all match expectations and command 10 succeeds, you're spec-compliant.

```sh
git add senders/android/ui/
git status
# Expected (4 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/audio_page.slint
git commit -m "feat(slint-ui): Phase 14 — audio capture controls sub-page (UI-only)"
```

---

## Gotchas (read these or pay later)

These three pitfalls are not in the PHASE-14 spec snippet but have already been caught and fixed in earlier phases. Re-applying them costs more than reading them now.

### Gotcha 1 — Toggle feedback bug

**Symptom:** the Mute toggle visually flips on tap but the bound `mock-muted` property doesn't update — or it updates twice and lands back on the old value, so the next tap appears to do nothing.

**Cause:** The PHASE-14 spec snippet writes the toggle handler as

```slint
SettingsToggleRow {
    title: "Mute";
    checked: root.mock-muted;
    toggled => { root.mock-muted = !root.mock-muted; }   // ❌ feedback bug
}
```

This is the form fixed by PR #8 in earlier phases. The bug:

1. User taps the inner `CheckBox` → it flips `self.checked`.
2. `SettingsToggleRow.checked <=> CheckBox.checked` two-way binding propagates the new value to `row.checked`.
3. `row.toggled(self.checked)` fires, passing the **new** value.
4. Consumer's handler runs `root.mock-muted = !root.mock-muted`, which flips the property to the **opposite** of where the inner CheckBox just landed.

Result: visual state and `mock-muted` diverge by one tap.

**Fix:** bind the argument, never recompute.

```slint
SettingsToggleRow {
    title: "Mute";
    checked: root.mock-muted;
    toggled(checked) => { root.mock-muted = checked; }   // ✅ canonical
}
```

The handler signature is declared as `callback toggled(bool)` in `components/settings_rows.slint:76`. See [functions-and-callbacks.mdx][callbacks] on callback parameter binding.

### Gotcha 2 — `Math.mod(a, b)`, not `(a) mod b`

**Symptom:** Slint compiler error `unknown identifier 'mod'` or `expected ')'`.

**Cause:** The PHASE-14 spec snippet uses an infix `mod` operator:

```slint
clicked => { root.mock-source-idx = (root.mock-source-idx + 1) mod 3; }   // ❌ not Slint
```

Slint does not have a `mod` infix operator. Modulo is a global function: [`Math.mod(T, T) -> T`][mathmod]. The post-PR-#8 baseline of `senders/android/ui/pages/settings_page.slint` proves the canonical form:

```slint
clicked => {
    root.resolution-idx = Math.mod(root.resolution-idx + 1, 4);   // ✅ canonical
}
```

**Fix:** Use `Math.mod(a, b)` exactly as the existing VIDEO QUALITY rows do.

```slint
clicked => {
    root.mock-source-idx = Math.mod(root.mock-source-idx + 1, 3);
}
```

(Slint's per-type `i.mod(N)` method is also valid per `reference/common.mdx`, but the codebase has standardised on the global `Math.mod(...)` form. Match it for diff-cleanliness.)

### Gotcha 3 — Slider unit conversion (0..1 storage ↔ 0..100 display)

**Symptom:** Either the slider snaps to the leftmost or rightmost stop only, or the printed `\{root.mock-input-gain}%` reads `70%` then `7000%` after a single tweak.

**Cause:** The `mock-input-gain` model is `float` in `0..1` (matching the eventual GStreamer `volume` element scale). The `SettingsSliderRow` is `minimum: 0; maximum: 100;` for legibility (whole-percent display). Two-way binding the two directly with `<=>` mismatches the units:

```slint
SettingsSliderRow {
    minimum: 0;
    maximum: 100;
    value <=> root.mock-input-gain;   // ❌ now value lives in 0..1, but slider expects 0..100
}
```

**Fix:** keep storage and display separate. The Phase 14 spec snippet has it right — replicate it exactly:

```slint
SettingsSliderRow {
    title: "Input gain";
    unit: "%";
    minimum: 0;
    maximum: 100;
    value: root.mock-input-gain * 100;             // display = storage × 100
    changed(v) => { root.mock-input-gain = v / 100; }   // storage = display ÷ 100
}
```

The `SettingsSliderRow.value` is `in-out` (`components/settings_rows.slint:103`), so a one-way `value: <expr>` binding plus a `changed(v) => { ... }` push-back is well-defined: the slider re-renders whenever `mock-input-gain` changes from any source, and the user's drags push back through `changed`. See [slider.mdx][slider]: *"### changed(float) — The value was changed"* and [properties.mdx][props] on `in-out` direction.

If you skip the `changed` handler and write only `value: root.mock-input-gain * 100;`, the slider becomes **read-only** (drags will be rejected because the property would be reset to the bound expression on every frame). Always pair the one-way `value:` expression with a `changed(v) =>` push-back when the slider is meant to be interactive.

---

## Exit criteria checklist (mirrors `PHASE-14-audio-capture-controls.md`)

- [ ] `bridge.slint` exposes `Panel.audio` in addition to `none, settings, debug, codec-test`.
- [ ] `main.slint` shows `AudioPage` based on `Bridge.active-panel == Panel.audio`.
- [ ] `AudioPage` renders the two sections (`INPUT` / `ENCODING`) with inline stub state.
- [ ] All four interactive controls flip stub state on tap:
  - [ ] `Source` cycles through `Microphone → System audio → Both`.
  - [ ] `Mute` toggle flips `mock-muted` (and binds the argument, not `!root.mock-muted`).
  - [ ] `Input gain` slider drags update `mock-input-gain` continuously and the percentage label live.
  - [ ] `Bitrate` cycles through `64 / 128 / 192 / 256 kbps`.
- [ ] `Codec` row reads `AAC-LC` and is non-interactive (`show-chevron: false`).
- [ ] Done button closes the panel from Slint (`Bridge.active-panel = Panel.none`).
- [ ] `FullSettingsPage` has a new `AUDIO & VIDEO` section with an `Audio` row that opens `Panel.audio`.
- [ ] `cargo build -p android-sender` passes.
- [ ] All cycle-on-click handlers use `Math.mod(...)` (post-PR-#8 canonical form).
- [ ] All toggle handlers bind their argument (`toggled(checked) => { ... = checked }`).
- [ ] `AudioPage` root has `width: 100%; height: 100%; background: Theme.surface-primary;`.

---

## When Phase 8 reactivates

When Rust takes over audio capture state, the migration from this UI-only state is mostly **a binding swap** + **a few new Bridge setters**. The shape will look like this:

### `bridge.slint`

```diff
+    in property <int>    audio-source-idx;
+    in-out property <bool>   audio-muted;
+    in-out property <float>  audio-input-gain;
+    in property <int>    audio-bitrate-idx;
+    in property <string> audio-codec;
+
+    callback set-audio-source(int);
+    callback set-audio-bitrate(int);
+    callback set-audio-input-gain(float);
+    callback set-audio-muted(bool);
```

### `pages/audio_page.slint`

Swap each `mock-*` `in-out` property for a `Bridge.audio-*` read; replace each `Math.mod` cycle handler with a call to the corresponding `Bridge.set-audio-*` callback (which Rust uses to validate the new value before pushing it back into the `Bridge.audio-*` property). The `Codec` row's `"AAC-LC"` literal becomes `Bridge.audio-codec`.

### Functional integration

- `set-audio-source(int)` → Android `AudioRecord.AudioSource` selection (`MIC`, `INTERNAL_AUDIO`, dual-source mix).
- `set-audio-bitrate(int)` → GStreamer `audioconvert ! avenc_aac bitrate=<n>` re-pipeline-rebuild on bitrate change.
- `set-audio-input-gain(float)` → GStreamer `volume` element on the source side of the mux.
- `set-audio-muted(bool)` → toggle the `volume` element's `mute` property without touching gain.

These wiring details belong to **Phase 8 (Rust bridge)** and stay parked here.

---

## Slint-doc references used

These cite the **local mirror** at `draft/slint-ui/docs/` (PR #11). All paths are repo-relative.

- **`Panel` enum extension** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` (`export enum CardSuit { clubs, diamonds, hearts, spade }` is the canonical form).
- **Inline `in-out` properties on `AudioPage` (`mock-muted`, `mock-source-idx`, `mock-input-gain`, `mock-bitrate-idx`)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`. Reactivity works the same for literal defaults and `Bridge.*` reads; Phase 8's swap is mechanical.
- **Conditional element syntax `if Bridge.active-panel == Panel.audio: AudioPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx` (search for *"The syntax is `if condition : id := Element { ... }`"*).
- **Indexed string lookup `["Microphone", "System audio", "Both"][root.mock-source-idx]`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx` + `expressions-and-statements.mdx`. Slint allows array literals to be indexed in expressions.
- **`Math.mod(a, b)` global function (NOT `(a) mod b` infix)** — `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`: *"### mod(T, T) -> T — Perform a modulo operation, where T is some numeric type."* The infix `mod` form does not exist in Slint and is rejected by the compiler.
- **`ScrollView { VerticalLayout { ... } }` auto-derived viewport** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`: *"If the ScrollView contains a layout, the default value for the `viewport-width` and `viewport-height` is the minimum size of that layout."*
- **`HorizontalLayout` / `VerticalLayout` with `padding`, `spacing`, `horizontal-stretch`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`. Bare `*Layout` has no implicit margins (unlike `*Box`); `*-stretch: 1` allocates leftover main-axis space.
- **Window-fill panel root via `width: 100%; height: 100%; background: …`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx` and `positioning-and-layouts.mdx`. Conditional elements in a non-layout parent (Window / Rectangle) don't inherit a layout — they need explicit dimensions.
- **`Slider.value` (in-out) + `changed(float)` callback** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx`. The doc shows `changed(value) => { debug("New value: ", value); }`. `SettingsSliderRow` (`components/settings_rows.slint:98-132`) wraps `Slider`, mirrors `value` two-way, and re-emits `changed(float)`.
- **`CheckBox.toggled()` + `checked` property (in-out)** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx`. The std-widget callback is `toggled()` with no arguments; `SettingsToggleRow` wraps it and re-emits `toggled(bool)` so consumers can bind `toggled(checked) => ...` directly.
- **`SettingsValueRow` / `SettingsToggleRow` / `SettingsSliderRow` / `SettingsSection`** — these are FCast components, declared in `senders/android/ui/components/settings_rows.slint`. Their property names (`title:`, `value:`, `checked:`, `show-chevron:`, `enabled:`) and callbacks (`clicked()`, `toggled(bool)`, `changed(float)`) were finalised by [PR #8](https://github.com/varxe-alt/fcast/pull/8) and Phase 14 uses them as-is.
- **`Bridge.active-panel = Panel.none` from a Slint callback body** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`. Callback bodies are statement blocks; assignments to `in-out` properties on globals are allowed.
- **Combobox-as-alternative-picker note** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/combobox.mdx`. The PHASE-14 spec mentions ComboBox under "Slint best practices" as a future picker option; Phase 14 uses cycle-on-click for diff-cleanliness with Phase 7's existing rows.

---

## What's NOT in this guide

These belong to other phases and stay parked:

- **Real `AudioRecord` source selection** (Mic / System / dual) → Phase 8 (Rust bridge) + Android `AudioRecord.AudioSource`.
- **Real GStreamer audio capsfilter / encoder bitrate change** → Phase 8 + the audio-side mirroring pipeline.
- **VU meter / live level monitoring** → a future polish phase (mentioned in PHASE-14 "What's NOT in this phase").
- **Per-app audio source filtering** (Android 10+ `AudioPlaybackCaptureConfiguration`) → out of scope until system-audio capture is wired.
- **Picker panels** for source / bitrate (replacing cycle-on-click with a proper "Select a value" sub-panel) → `futures/` or a future polish phase.
- **Quick-action surface for the mic button** (the spec preamble's *"quick-action surface for audio capture (mic source, output gain, mute)"* mention) → not in PHASE-14's task list. Phase 17 (`PHASE-17-quick-action-customization.md`) owns the quick-action reorder/enable/overflow surface; a `mic` quick-action id can be added there once the Audio sub-page exists. **Don't add a quick-action button in this phase** — keep the diff small.
- **`@tr(...)` wrapping** of `"Audio"` / `"INPUT"` / `"ENCODING"` / `"Source"` / `"Mute"` / `"Input gain"` / `"Bitrate"` / `"Codec"` / `"Microphone"` / `"System audio"` / `"Both"` / `"AAC-LC"` / `"64 kbps"` / `"AUDIO & VIDEO"` / `"Open"` / `"Done"` → Phase 9 (localization sweep).
- **`PanelHeader` component** factored out of `FullSettingsPage`, `CodecTestPage`, and `AudioPage` — defer until a fourth panel exists or the duplication crosses ~25 LoC.
- **Phase 15 (Camera)** and **Phase 16 (Bitrate presets)** — both will append rows to the new `AUDIO & VIDEO` section in `FullSettingsPage` once they land. Phase 14 is intentionally limited to the Audio entry to keep the diff focused.
- **Tests** — Phase 10 (`PHASE-10-testing.md`) runs alongside every UI phase; it owns build-validation and any future Slint-side smoke tests. No test wiring belongs in this guide.

If you want to do any of the above, pause and write a separate phase doc first.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-14-audio-capture-controls.md
[props]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[layouts]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[callbacks]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
[scrollview]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
[slider]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx
[checkbox]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx
[mathmod]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx
[rect]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx

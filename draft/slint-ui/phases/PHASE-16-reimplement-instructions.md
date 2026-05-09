# Phase 16 — Bitrate & Quality Presets reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-16-bitrate-quality-presets.md`][spec] to the current `senders/android` tree.
**Goal:** add `BitratePresetsPage` (list of presets with active highlight + "Add preset" button) and `BitratePresetEditPage` (single-preset name + bitrate editor) wired into the `Panel` overlay layer, plus a `Bitrate` row in `FullSettingsPage`'s `AUDIO & VIDEO` section.
**Scope:** Slint UI only. **No Rust changes.** All preset state lives on `BitratePresetsPage` itself as an `[BitratePreset]` array; the edit page operates on UI-only stub state and does not actually persist back to the parent (real persistence requires lifting the model to `Bridge`, deferred to Phase 8).

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-16-bitrate-quality-presets.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] first.** Same panel chrome, same row primitives, same gotchas. The new things in Phase 16 are: a `struct` declaration in `bridge.slint`, a `for entry in mock-presets:` loop with selection state, a `LineEdit` in the editor, and a multi-page navigation (list → edit → back-to-list, not back-to-none).

[p14]: ./PHASE-14-reimplement-instructions.md

---

## Why this guide exists

Phase 16 is the third sub-page in `AUDIO & VIDEO` (after Audio in Phase 14, Camera in Phase 15). The PHASE-16 spec snippet introduces three patterns not yet seen in the chassis:

1. **Struct declaration in `bridge.slint`** — `BitratePreset { id, name, bitrate-kbps, active }`. Already a sibling pattern of `QuickAction` and `ReceiverItem` in the same file, so the syntax is well-known. Spec says: *"add the struct (placeholder — not bound to any `Bridge` setter)"*. Keep it that way; do **not** add an `[BitratePreset]` `Bridge` property — Phase 8 owns that.
2. **Reactive list re-rendering** when toggling `active` flags on entries inside an `in-out property <[BitratePreset]>`. Slint `for` loops over array properties **do** re-render reactively, but **only if the entire array is reassigned** — mutating an individual element's field in place is not observed. The guide pins this down.
3. **Multi-page panel navigation**: list → edit → back-to-list. The edit page's `Save` / `Cancel` buttons must write `Bridge.active-panel = Panel.bitrate-presets` (not `Panel.none`) so the user lands back on the list. Same idiom as Phase 21's About sub-pages.

The spec also has the same `Math.mod` and `toggled => !x` pitfalls as Phase 14; this guide does not re-document them but does re-flag them in the `Sanity grep` step.

After Phase 14 + 15 merge:

- `Panel { none, settings, debug, codec-test, audio, camera }` — Phase 16 adds **two** variants: `bitrate-presets` and `bitrate-preset-edit`.
- `FullSettingsPage`'s `AUDIO & VIDEO` section has 2 rows — Phase 16 appends a `Bitrate` row.
- `bridge.slint` exports `QuickAction`, `ReceiverItem`, `StatusItem` structs — Phase 16 adds `BitratePreset` next to them.

This is **strictly additive** Slint work spread across **three existing files** plus **two new files** (`pages/bitrate_presets_page.slint` and `pages/bitrate_preset_edit_page.slint`).

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'BitratePreset\|bitrate-presets\|bitrate-preset-edit\|mock-presets\|mock-selected-idx' \
    senders/android/ui/

# Existing struct exports (sibling pattern for the new BitratePreset):
grep -n '^export struct' senders/android/ui/bridge.slint
# Expected:
#   26:export struct QuickAction {
#   41:export struct ReceiverItem {

# Phase 14/15 panel chassis present:
grep -n 'Panel\.audio\|Panel\.camera' senders/android/ui/main.slint
# Expected: 2 matches.

# AUDIO & VIDEO section already has Audio + Camera rows:
grep -n 'AUDIO & VIDEO\|Panel\.audio\|Panel\.camera' senders/android/ui/pages/settings_page.slint
# Expected: 3 matches.
```

After this guide is applied:

```sh
# BitratePreset struct exported.
grep -n 'export struct BitratePreset' senders/android/ui/bridge.slint
# Expected: 1 match.

# Panel enum extended with two variants.
grep -n 'bitrate-presets\|bitrate-preset-edit' senders/android/ui/bridge.slint
# Expected: 2 matches (both inside the Panel enum).

# Two new pages exist and are exported.
grep -rn 'export component BitratePresetsPage\|export component BitratePresetEditPage' \
    senders/android/ui/

# main.slint routes both panels.
grep -n 'Panel\.bitrate' senders/android/ui/main.slint
# Expected: 2 matches.

# AUDIO & VIDEO section has 3 rows now.
grep -n 'Panel\.audio\|Panel\.camera\|Panel\.bitrate-presets' \
    senders/android/ui/pages/settings_page.slint
# Expected: 3 matches.

# Edit page's Save/Cancel return to the list (NOT to Panel.none).
grep -n 'Bridge\.active-panel = Panel\.' senders/android/ui/pages/bitrate_preset_edit_page.slint
# Expected: 2 matches, both `Panel.bitrate-presets`.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-16-bitrate-presets
cargo check -p android-sender
```

---

## Step 1 — Add `BitratePreset` struct + `Panel` variants in `bridge.slint`

```diff
 export struct ReceiverItem {
     name:    string,
     address: string,
 }
+
+export struct BitratePreset {
+    id:           string,
+    name:         string,
+    bitrate-kbps: int,
+    active:       bool,
+}
```

```diff
 export enum Panel {
     none,
     settings,
     debug,
     codec-test,
     audio,
     camera,
+    bitrate-presets,
+    bitrate-preset-edit,
 }
```

### Why each piece

- **No `[BitratePreset]` `Bridge` property.** The spec explicitly says the struct is a placeholder. Adding it here would commit to a wire format Phase 8 might not want. Keep struct-only.
- **Two enum variants, not one.** The list page and the edit page are separate panels; either could be opened from outside the other (the spec's quick-action shortcut opens `bitrate-presets` directly). Treat them as siblings, not parent/child.
- **Hyphenated variant names** (`bitrate-presets`, `bitrate-preset-edit`) match the existing `codec-test` precedent.

---

## Step 2 — Route both panels in `main.slint`

```diff
 import { CameraPage }                   from "pages/camera_page.slint";
+import { BitratePresetsPage }           from "pages/bitrate_presets_page.slint";
+import { BitratePresetEditPage }        from "pages/bitrate_preset_edit_page.slint";
 import { DebugPage, FullDebugPage }     from "pages/debug_page.slint";
```

```diff
     if Bridge.active-panel == Panel.camera:     CameraPage { }
+    if Bridge.active-panel == Panel.bitrate-presets:     BitratePresetsPage { }
+    if Bridge.active-panel == Panel.bitrate-preset-edit: BitratePresetEditPage { }
 }
```

---

## Step 3 — Create `pages/bitrate_presets_page.slint`

**File:** `senders/android/ui/pages/bitrate_presets_page.slint` (new)

A scrollable list of preset cards plus a trailing "Add preset" button. Tapping a card sets it active; tapping "Add preset" opens the edit page.

### New file

```slint
// bitrate_presets_page.slint — Bitrate presets list (UI-only placeholder).
//
// Reachable from FullSettingsPage's "Bitrate" row (sets
// `Bridge.active-panel = Panel.bitrate-presets`) or directly from a
// quick-action shortcut once Phase 17 lands. The active preset is
// highlighted; tapping a card flips the active flag for that row only.
// "Add preset" opens BitratePresetEditPage as a sibling panel.
//
// Real persistence (writing edited presets back to a global model) is
// deferred to Phase 8. This page operates on a self-contained
// `mock-presets` array; the edit page does NOT mutate it.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, BitratePreset } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton, PrimaryButton } from "../components/buttons.slint";

export component BitratePresetsPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <[BitratePreset]> mock-presets: [
        { id: "low",  name: "Low",     bitrate-kbps: 1500,  active: false },
        { id: "med",  name: "Medium",  bitrate-kbps: 4000,  active: true  },
        { id: "high", name: "High",    bitrate-kbps: 8000,  active: false },
        { id: "max",  name: "Maximum", bitrate-kbps: 15000, active: false },
    ];

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // Reassigns the entire array with one row marked active. Mutating an
    // element field in place is NOT observed by Slint's reactive system —
    // see "Gotcha 7" below.
    function select(id: string) {
        root.mock-presets = [
            { id: root.mock-presets[0].id, name: root.mock-presets[0].name,
              bitrate-kbps: root.mock-presets[0].bitrate-kbps,
              active: root.mock-presets[0].id == id },
            { id: root.mock-presets[1].id, name: root.mock-presets[1].name,
              bitrate-kbps: root.mock-presets[1].bitrate-kbps,
              active: root.mock-presets[1].id == id },
            { id: root.mock-presets[2].id, name: root.mock-presets[2].name,
              bitrate-kbps: root.mock-presets[2].bitrate-kbps,
              active: root.mock-presets[2].id == id },
            { id: root.mock-presets[3].id, name: root.mock-presets[3].name,
              bitrate-kbps: root.mock-presets[3].bitrate-kbps,
              active: root.mock-presets[3].id == id },
        ];
    }

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Bitrate presets";
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

        // ── Body ────────────────────────────────────────────────────────
        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                for preset[i] in root.mock-presets: Rectangle {
                    height: 64px;
                    border-radius: Theme.radius-card;
                    background: preset.active
                        ? Theme.accent-active
                        : (ta.pressed ? Theme.surface-card.brighter(20%) : Theme.surface-card);

                    ta := TouchArea {
                        clicked => { root.select(preset.id); }
                    }
                    HorizontalLayout {
                        padding-left:  Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        VerticalLayout {
                            alignment: center;
                            horizontal-stretch: 1;
                            Text {
                                text: preset.name;
                                color: Theme.text-primary;
                                font-size: Theme.font-size-body;
                            }
                            Text {
                                text: "\{preset.bitrate-kbps} kbps";
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                        }
                        if preset.active: Text {
                            text: "✓";
                            color: Theme.text-primary;
                            font-size: Theme.font-size-heading;
                            vertical-alignment: center;
                        }
                    }
                }

                PrimaryButton {
                    label: "Add preset";
                    clicked => { Bridge.active-panel = Panel.bitrate-preset-edit; }
                }
            }
        }
    }
}
```

### Why each piece

- **`for preset[i] in root.mock-presets: Rectangle { ... }`** — Slint's `for var[index] in model:` syntax exposes both the element and the index; we don't actually use `i` here but it's the canonical form, and it's shown in [repetition-and-data-models.mdx][repeat]. (Drop the `[i]` if you want, but keep it for symmetry with the spec.)
- **`function select(id: string) { root.mock-presets = [ ... ]; }`** — wraps the reassignment so the row taps look clean. Slint allows component-local functions (see [functions-and-callbacks.mdx][callbacks]). The body explicitly rebuilds each entry instead of using a helper because Slint has no array `.map` operator and no spread syntax — see "Gotcha 7" below.
- **String interpolation `"\{preset.bitrate-kbps} kbps"`** — same idiom as Phase 15. `+` would be invalid here too.
- **`if preset.active: Text { text: "✓"; ... }`** — conditional element scoped to each row. Per [file.mdx][file], `if` inside a layout works the same as at the root level: only matching iterations instantiate the element. The `✓` glyph is a literal U+2713; safe in the codebase (other phases already use Unicode arrows / chevrons).
- **`PrimaryButton { label: "Add preset"; clicked => { Bridge.active-panel = Panel.bitrate-preset-edit; } }`** — opens the edit panel without round-tripping through Rust.
- **No `mock-selected-idx` property** — the spec mentions it but the active-flag-on-each-entry pattern is sufficient and more direct (matches the data shape shown in [`Bridge.quick-actions`][bridge-quick] and friends, where `active: bool` is a per-entry flag).

[bridge-quick]: ../../../senders/android/ui/bridge.slint

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Create `pages/bitrate_preset_edit_page.slint`

**File:** `senders/android/ui/pages/bitrate_preset_edit_page.slint` (new)

A form: name `LineEdit`, bitrate `SettingsSliderRow` (range 500..20000 kbps step 500), `Save` and `Cancel` buttons. Both buttons return to `Panel.bitrate-presets` — the edit page does **not** persist anywhere (deferred per spec).

### New file

```slint
// bitrate_preset_edit_page.slint — Single-preset editor (UI-only).
//
// Reachable from BitratePresetsPage's "Add preset" button (and, in a
// future phase, from per-row edit affordances). Save and Cancel both
// just return to the list — no persistence in this phase.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx

import { ScrollView, LineEdit } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsSliderRow,
} from "../components/settings_rows.slint";

export component BitratePresetEditPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <string> mock-name:         "New preset";
    in-out property <float>  mock-bitrate-kbps: 4000.0;

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
                spacing: Theme.spacing-default;
                TextButton {
                    label: "Cancel";
                    // Return to the LIST, not the cast screen.
                    clicked => { Bridge.active-panel = Panel.bitrate-presets; }
                }
                Text {
                    text: "Edit preset";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Save";
                    // UI-only — no persistence to BitratePresetsPage's
                    // mock-presets. Real persistence requires lifting the
                    // model to a global / Bridge property; deferred to
                    // Phase 8.
                    clicked => { Bridge.active-panel = Panel.bitrate-presets; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                SettingsSection {
                    title: "NAME";
                    Rectangle {
                        height: 56px;
                        HorizontalLayout {
                            padding-left:  Theme.padding-screen;
                            padding-right: Theme.padding-screen;
                            LineEdit {
                                placeholder-text: "Preset name";
                                text <=> root.mock-name;
                                horizontal-stretch: 1;
                            }
                        }
                    }
                }

                SettingsSection {
                    title: "BITRATE";
                    SettingsSliderRow {
                        title: "Bitrate";
                        unit: " kbps";
                        minimum: 500;
                        maximum: 20000;
                        value <=> root.mock-bitrate-kbps;
                        // Domain matches storage; `<=>` is safe.
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`Cancel` and `Save` both return to `Panel.bitrate-presets`** — never `Panel.none`. That's the back-stack invariant: the edit page is a child of the list page, so dismissing it should land on the parent. The spec calls this out: *"Save button does `Bridge.active-panel = Panel.bitrate-presets;`"*.
- **`text <=> root.mock-name`** — `LineEdit.text` is `in-out` per [lineedit.mdx][lineedit]. Two-way binding to a stub `string` property is the canonical form.
- **`SettingsSliderRow { ... value <=> root.mock-bitrate-kbps; }`** — domain (500..20000) matches storage (`mock-bitrate-kbps: float`), so the Phase 14 split form (`value: x*100; changed(v) => x=v/100`) is unnecessary. `<=>` two-way is the cleaner pattern when units match.
- **No real persistence.** The spec is explicit: *"Save button does `Bridge.active-panel = Panel.bitrate-presets;` (no real mutation of the parent's `mock-presets`)"*. Sidecar comments document why.
- **Header layout has `Cancel` on the left, title centred, `Save` on the right** — the canonical "modal editor" header. This differs from the other panels (which have title left, Done right) on purpose: an editor signals destructive intent, so the cancel affordance should be the left-most. Match iOS / Material editor conventions.

### Build check

```sh
cargo build -p android-sender
```

---

## Step 5 — Append to `FullSettingsPage`'s `AUDIO & VIDEO` section

**File:** `senders/android/ui/pages/settings_page.slint`

```diff
                 // ── Section: AUDIO & VIDEO ────────────────────────────────
                 SettingsSection {
                     title: "AUDIO & VIDEO";
                     SettingsValueRow {
                         title: "Audio";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.audio; }
                     }
                     SettingsValueRow {
                         title: "Camera";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.camera; }
                     }
+                    SettingsValueRow {
+                        title: "Bitrate";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.bitrate-presets; }
+                    }
                 }
```

---

## Sanity grep before commit

```sh
# 1. BitratePreset struct + 2 Panel variants present in bridge.slint
grep -n 'BitratePreset\|bitrate-presets\|bitrate-preset-edit' senders/android/ui/bridge.slint
# Expected: 4+ matches (struct decl + 4 field lines OR struct + 2 Panel variants + struct uses).

# 2. Both pages exist and are exported.
grep -rn 'export component BitratePresetsPage\|export component BitratePresetEditPage' \
    senders/android/ui/

# 3. main.slint routes both.
grep -n 'Panel\.bitrate' senders/android/ui/main.slint
# Expected: 2 matches.

# 4. Edit page's Cancel + Save both return to the list, never Panel.none.
grep -n 'Panel\.bitrate-presets\|Panel\.none' senders/android/ui/pages/bitrate_preset_edit_page.slint
# Expected: 2 matches of Panel.bitrate-presets, 0 matches of Panel.none.

# 5. List page reassigns the entire array (no in-place mutation).
grep -n 'mock-presets = \[' senders/android/ui/pages/bitrate_presets_page.slint
# Expected: 1 match (inside `function select`).

# 6. AUDIO & VIDEO section has 3 rows.
grep -n 'Panel\.audio\|Panel\.camera\|Panel\.bitrate-presets' \
    senders/android/ui/pages/settings_page.slint
# Expected: 3 matches.

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/
git status
# Expected (5 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/bitrate_presets_page.slint
#   new file:   senders/android/ui/pages/bitrate_preset_edit_page.slint
git commit -m "feat(slint-ui): Phase 16 — bitrate presets list + editor (UI-only)"
```

---

## Gotchas (Phase 16 specific)

The Phase 14 gotchas (toggle feedback, `Math.mod`, slider unit conversion) all apply when extending these pages. Phase 16 adds:

### Gotcha 7 — In-place struct field mutation is not reactive

**Symptom:** tapping a preset row updates `mock-presets[i].active` (in your head) but the UI doesn't re-render — the previous active row stays highlighted.

**Cause:** Slint's reactive system observes **property writes**, not field-level mutations. Writing `mock-presets[2].active = true` is not a property write to `mock-presets`; it's a (currently rejected) field mutation. Slint's compiler in some versions rejects it outright; in others it appears to succeed but the UI does not re-render. In either case, **it does not work**.

**Fix:** reassign the entire array. The `function select(id: string) { root.mock-presets = [ ... ]; }` form in this guide rebuilds every entry from scratch. This is verbose but correct. Slint sees a fresh property write, the `for` loop re-evaluates, each row's `preset.active` reads the new value.

When the array grows past ~10 entries, lift to a `Bridge` global with a Rust-backed `set-active(string)` callback rather than maintaining the verbose rebuild in Slint. That's Phase 8's responsibility.

### Gotcha 8 — Edit page must return to the list, not to `Panel.none`

**Symptom:** user taps Save → cast screen pops up instead of returning to the bitrate presets list. User loses context.

**Cause:** the natural reflex is to write `Bridge.active-panel = Panel.none;` (matching the Done button on every other panel). But the edit page's parent is the **list page**, not the cast screen.

**Fix:** Save and Cancel both write `Bridge.active-panel = Panel.bitrate-presets;`. This is the same idiom Phase 21's About sub-pages will use. Document it in the page header comment so future edits don't regress.

### Gotcha 9 — `LineEdit.text` is `in-out`, not `in`

**Symptom:** binding `text: root.mock-name` (one-way) compiles but typing in the field does not update `mock-name`.

**Cause:** [`LineEdit.text`][lineedit] is `in-out`. A one-way bind (`text: root.mock-name`) makes the property read-only from the user's perspective.

**Fix:** use two-way `<=>` binding: `text <=> root.mock-name`. Same applies to any std-widget with an `in-out property` (Slider, CheckBox).

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `BitratePreset` struct and `Panel.bitrate-presets` + `Panel.bitrate-preset-edit` variants.
- [ ] `main.slint` routes both panels.
- [ ] `BitratePresetsPage` lists 4 stub presets with `Medium` highlighted by default.
- [ ] Tapping a preset card highlights it and unhighlights the previous one (entire array reassigned).
- [ ] "Add preset" opens `BitratePresetEditPage`.
- [ ] `BitratePresetEditPage` shows a name `LineEdit` + bitrate `SettingsSliderRow` (500..20000 kbps).
- [ ] Cancel + Save both return to `Panel.bitrate-presets` (never `Panel.none`).
- [ ] `FullSettingsPage`'s `AUDIO & VIDEO` section has 3 rows: Audio + Camera + Bitrate.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

- Lift `mock-presets` to a `Bridge.bitrate-presets: [BitratePreset]` `in` property populated by Rust.
- Replace `function select(...)` with a `callback set-active-preset(string)` callback that Rust handles by rewriting the active flag and re-publishing the array.
- Wire `Save` to a real `callback save-preset(BitratePreset)` and `Add preset` to a `callback create-preset()` that allocates a new id Rust-side.
- Real persistence: Rust serialises the preset list to disk (probably via `serde_json` to the app's data dir).
- Real bitrate change: when active preset switches, Rust re-pipelines GStreamer with the new `x264enc bitrate=<n>` (or `amcvidenc`).
- Quick-action shortcut (PHASE-16 task 16-D) lands once Phase 17 (`PHASE-17-quick-action-customization.md`) provides the customisation surface; the `mock-quick-actions` model in `components/control_bar.slint` already routes id `"bitrate"` through `Bridge.invoke-action` — Phase 17 swaps that to `Bridge.active-panel = Panel.bitrate-presets`.

---

## Slint-doc references used

- **`export struct BitratePreset { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`Panel` enum extension** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`for preset[i] in root.mock-presets: Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **`function select(id: string) { ... }` component-local functions** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **String interpolation `"\{preset.bitrate-kbps} kbps"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **Conditional row content `if preset.active: Text { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`LineEdit.text` (in-out) + `placeholder-text`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx`.
- **`Slider.value` two-way `<=>`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx`.
- **`ScrollView` auto-derived viewport** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`.
- **`PrimaryButton`, `DestructiveButton`, `TextButton`** — FCast components in `senders/android/ui/components/buttons.slint`.
- **`SettingsSection`, `SettingsSliderRow`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`Bridge.active-panel = Panel.bitrate-presets` from a Slint callback body** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.

---

## What's NOT in this guide

- **Real preset persistence.** Phase 8 owns the Rust-side storage.
- **Live encoder bitrate reconfiguration.** Phase 8 + GStreamer pipeline rewiring.
- **Per-preset codec / profile / framerate overrides.** Future polish phase.
- **Preset reordering (drag-and-drop or ▲/▼ buttons).** Spec mentions this is deferred to Phase 27 utils backlog (`PHASE-27-utils-backlog.md`).
- **Per-preset deletion.** Same — Phase 27 covers swipe-action equivalent.
- **`mock-selected-idx: int` companion property.** The spec mentions it but the active-flag-on-each-entry approach used in this guide is sufficient and avoids a derived-state-out-of-sync trap.
- **Quick-action shortcut for `bitrate`.** Phase 17 owns customisation; the `id: "bitrate"` entry is already in `mock-quick-actions` and currently routes through `Bridge.invoke-action`. Phase 17 will swap that line to a Slint-side panel switch.
- **`@tr(...)` wrapping** of `"Bitrate presets"` / `"Bitrate"` / `"NAME"` / `"BITRATE"` / `"Edit preset"` / `"Save"` / `"Cancel"` / `"Add preset"` / `"Low"` / `"Medium"` / `"High"` / `"Maximum"` / `"Preset name"` / `"Open"` / `"Done"` — Phase 9 (localization sweep).

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-16-bitrate-quality-presets.md
[p14]: ./PHASE-14-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[callbacks]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
[lineedit]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx

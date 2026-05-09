# Phase 25 — Macros & Action Chains reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-25-macros-action-chains.md`][spec] to the current `senders/android` tree.
**Goal:** add `MacrosPage` (list of saved macros) + `MacroEditPage` (per-macro editor: name field + ordered step list with reorder/remove + add-step picker). Add `Macro` + `MacroStep` structs to bridge. Wire into the `Panel` overlay layer with parent-child back-stack semantics. Document the `id: "macro:<id>"` quick-action prefix convention so the Phase 17 customisation page can surface macros as bar actions.
**Scope:** Slint UI only. **No Rust changes.** No real macro execution; no persistence.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-25-macros-action-chains.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14], [`PHASE-16-reimplement-instructions.md`][p16], and [`PHASE-17-reimplement-instructions.md`][p17] first.** Phase 16 is the closest cousin (list page + edit page + back-stack); Phase 17's swap helper repeats here. The new things in Phase 25: a struct that contains a sub-array (`Macro.steps: [MacroStep]` is an array of structs nested in a struct), an inline picker that emits to a parent's `on-add-step(id)` callback, and the ▶ glyph prefix for macro-flagged quick actions.

[p14]: ./PHASE-14-reimplement-instructions.md
[p16]: ./PHASE-16-reimplement-instructions.md
[p17]: ./PHASE-17-reimplement-instructions.md

---

## Why this guide exists

Phase 25 is the most complex Phase-7-dependent UI-only phase. It depends on Phase 17 (quick-action customisation) but doesn't share any *code* — only the convention that a quick-action's `id` starting with `"macro:"` is a macro shortcut. Phase 25's complexity comes from three sources:

1. **`Macro.steps: [MacroStep]`** — a struct field that's itself an array of structs. Slint supports this, but the syntax for declaring + initialising it is fiddly enough to warrant explicit examples.
2. **Two-level reorder.** Phase 16 moved among presets; Phase 17 moved among bar actions. Phase 25's macro editor moves *steps within a macro*, then saves the macro back. The same swap-rebuild pattern applies, but the rebuild now has to preserve all sibling macros.
3. **`mock-edit-id` round-trip.** The list page sets `Bridge.mock-macro-edit-id = "<id>";` (or a page-local property) before opening `Panel.macro-edit`; the edit page reads it and looks up the corresponding macro by id. Slint has no Map / Dictionary type — looking up by id requires a linear search over `mock-macros` via a pure helper.

After Phases 14 + 15 + 16 + 17 + 18 + 19 + 21 + 22 + 23 + 26 merge:

- `Panel { ..., backup-reset }`. Phase 25 adds **two** variants: `macros`, `macro-edit`.
- `bridge.slint` has `BitratePreset`, `NetworkInterface`, `LogEntry` structs; `RecordingState`, `LogLevel`, `LifecycleMode` enums. Phase 25 adds `MacroStep` + `Macro` structs.
- `FullSettingsPage` does not have an `AUTOMATION` section yet. Phase 25 inserts it.
- `CastControlBar` already exists from Phase 4/7. Phase 25 documents (does not implement) the `id: "macro:<id>"` convention; the actual ▶ glyph rendering is a small `control_bar.slint` tweak.

This is **strictly additive** Slint work spread across **four existing files** plus **two new files**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'MacrosPage\|MacroEditPage\|Macro\.\|MacroStep\|mock-macros\|Panel\.macros\|Panel\.macro-edit' \
    senders/android/ui/

# No AUTOMATION section yet:
grep -n 'AUTOMATION' senders/android/ui/pages/settings_page.slint
# Expected: (empty)

# Phase 17's QuickActionsPage exists (Phase 25 documents but doesn't change it):
grep -n 'mock-bar-actions' senders/android/ui/pages/quick_actions_page.slint
# Expected: matches (depends on Phase 17 having merged).
```

After this guide is applied:

```sh
grep -n 'export struct MacroStep\|export struct Macro\b' senders/android/ui/bridge.slint
# Expected: 2 matches.

grep -n 'macros,\|macro-edit,' senders/android/ui/bridge.slint
# Expected: 2 matches.

grep -rn 'export component MacrosPage\|export component MacroEditPage' \
    senders/android/ui/pages/

# Expected: 2 matches.

grep -n 'Panel\.macros\|Panel\.macro-edit' senders/android/ui/main.slint
# Expected: 2 matches.

grep -n 'AUTOMATION\|Panel\.macros' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

grep -n 'macro:' senders/android/ui/components/control_bar.slint
# Expected: 1 match (▶ prefix conditional).
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-25-macros
cargo check -p android-sender
```

---

## Step 1 — Add `MacroStep`, `Macro` structs + 2 `Panel` variants in `bridge.slint`

```diff
+export struct MacroStep {
+    action-id: string,    // matches QuickAction.id
+    label:     string,    // display label
+}
+
+export struct Macro {
+    id:       string,
+    name:     string,
+    steps:    [MacroStep],
+    enabled:  bool,
+}
+
 export struct NetworkInterface { ... }
```

```diff
 export enum Panel {
     ...
     backup-reset,
+    macros,
+    macro-edit,
 }
```

Add a Bridge property to thread the "currently editing" id from `MacrosPage` to `MacroEditPage`:

```diff
 export global Bridge {
     ...
     in-out property <Panel>          active-panel: Panel.none;
+    in-out property <string>         mock-macro-edit-id: "";   // "" means "new macro"
     ...
 }
```

`mock-macro-edit-id` lives on Bridge because `MacrosPage` writes it and `MacroEditPage` reads it; without a Bridge round-trip the two sibling pages have no way to communicate. Empty string sentinel = "new macro" (edit page renders blank form).

### Why nested struct array

`Macro.steps: [MacroStep]` works in Slint — see [structs-and-enums.mdx][structs] (struct field types include array types of other structs). The `mock-macros` initialiser populates each `Macro.steps` with an inline `[{ action-id: ..., label: ... }, ...]` literal.

---

## Step 2 — Route both panels in `main.slint`

```diff
 import { BackupResetPage }              from "pages/backup_reset_page.slint";
+import { MacrosPage }                   from "pages/macros_page.slint";
+import { MacroEditPage }                from "pages/macro_edit_page.slint";
```

```diff
     if Bridge.active-panel == Panel.backup-reset:  BackupResetPage { }
+    if Bridge.active-panel == Panel.macros:        MacrosPage { }
+    if Bridge.active-panel == Panel.macro-edit:    MacroEditPage { }
 }
```

---

## Step 3 — Create `pages/macros_page.slint`

**File:** `senders/android/ui/pages/macros_page.slint` (new)

The list page. Each row shows name + step-count badge + enable toggle. Tapping a row navigates to the edit page with the macro's id; "Add macro" navigates with empty id.

### New file

```slint
// macros_page.slint — Macros list (UI-only).
//
// Reachable from FullSettingsPage's "Macros" row in AUTOMATION.
// Tap a row → set Bridge.mock-macro-edit-id and open Panel.macro-edit.
// Tap "Add macro" → set "" and open Panel.macro-edit (blank form).
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, Macro, MacroStep } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton, PrimaryButton } from "../components/buttons.slint";

export component MacrosPage inherits Rectangle {
    // ── UI-only stub state. The same property is used by MacroEditPage,
    // so this guide promotes it to Bridge in Step 4 — but for the list
    // page only, holding it here is sufficient. See gotcha 37 for the
    // shared-state alternative.
    in-out property <[Macro]> mock-macros: [
        {
            id: "m1", name: "Start morning cast", enabled: true,
            steps: [
                { action-id: "scan-qr",  label: "Scan QR" },
                { action-id: "audio",    label: "Open Audio" },
                { action-id: "record",   label: "Start Recording" },
            ],
        },
        {
            id: "m2", name: "Quick stop", enabled: true,
            steps: [
                { action-id: "stop-recording", label: "Stop Recording" },
                { action-id: "stop-cast",      label: "Stop Cast" },
            ],
        },
    ];

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    VerticalLayout {
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Macros";
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

                if root.mock-macros.length == 0: Rectangle {
                    height: 80px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    HorizontalLayout {
                        padding: Theme.padding-screen;
                        Text {
                            text: "No macros yet. Tap “Add macro” below to create one.";
                            color: Theme.text-secondary;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                            horizontal-stretch: 1;
                            wrap: word-wrap;
                        }
                    }
                }

                for macro in root.mock-macros: Rectangle {
                    height: 64px;
                    border-radius: Theme.radius-card;
                    background: macro-ta.pressed
                        ? Theme.surface-card.brighter(20%)
                        : Theme.surface-card;

                    macro-ta := TouchArea {
                        clicked => {
                            Bridge.mock-macro-edit-id = macro.id;
                            Bridge.active-panel = Panel.macro-edit;
                        }
                    }

                    HorizontalLayout {
                        padding-left:  Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        spacing: Theme.spacing-default;

                        VerticalLayout {
                            alignment: center;
                            horizontal-stretch: 1;
                            Text {
                                text: macro.name;
                                color: Theme.text-primary;
                                font-size: Theme.font-size-body;
                            }
                            Text {
                                // String interpolation handles int via "\{n}".
                                text: "\{macro.steps.length} step" +
                                      (macro.steps.length == 1 ? "" : "s");
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                        }

                        Rectangle {
                            width: 56px;
                            Text {
                                text: macro.enabled ? "On" : "Off";
                                color: macro.enabled
                                    ? Theme.accent-active
                                    : Theme.text-secondary;
                                horizontal-alignment: end;
                                vertical-alignment: center;
                                font-size: Theme.font-size-label;
                            }
                            // Toggle wired to a list-rebuild helper.
                            // Inline empty stub here; full impl is parallel
                            // to Phase 17's set-enabled. Kept short for
                            // brevity — copy the Phase 17 pattern if you
                            // want flippable enable from the list.
                            TouchArea { }
                        }
                    }
                }

                PrimaryButton {
                    label: "Add macro";
                    clicked => {
                        // Empty id → edit page renders blank form.
                        Bridge.mock-macro-edit-id = "";
                        Bridge.active-panel = Panel.macro-edit;
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`Macro.steps.length`** — Slint exposes `.length` on array-typed struct fields. See [repetition-and-data-models.mdx][repeat].
- **String interpolation `"\{macro.steps.length} step"`** — Slint supports `\{...}` interpolation inside string literals; numeric values are auto-stringified. See [expressions-and-statements.mdx][expressions].
- **Singular/plural ternary `+ (length == 1 ? "" : "s")`** — string concatenation via `+`. Works because both operands are strings; numeric+string is rejected (see Phase 15 §gotcha 4).
- **Empty-state card** — same pattern as Phase 17.
- **Enable-toggle stub** — left as `TouchArea { }` because a full implementation requires the same hardcoded array-rebuild pattern as Phase 17 with 2 entries instead of 5. The pattern is documented in Phase 17; copy if needed.
- **Add-macro PrimaryButton** at the bottom — sets `mock-macro-edit-id = ""` to signal "new macro" to the edit page.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Create `pages/macro_edit_page.slint`

**File:** `senders/android/ui/pages/macro_edit_page.slint` (new)

The edit form. Reads `Bridge.mock-macro-edit-id`; on empty, renders blank form. Otherwise, looks up the macro by id and binds its fields. Save/Cancel return to `Panel.macros`.

### New file

```slint
// macro_edit_page.slint — Per-macro editor (UI-only).
//
// Reads Bridge.mock-macro-edit-id (set by MacrosPage). Empty string
// means "new macro" — render blank form. Save returns to Panel.macros
// (NOT Panel.none) — back-stack invariant from Phase 16. UI-only build:
// neither Save nor Cancel persists across panel re-entries because
// mock-macros lives on MacrosPage, not on Bridge. Phase 8 promotes
// mock-macros to Bridge.macros and the round-trip becomes real.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx

import { ScrollView, LineEdit } from "std-widgets.slint";
import { Bridge, Panel, Macro, MacroStep } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton, PrimaryButton, DestructiveButton } from "../components/buttons.slint";

export component MacroEditPage inherits Rectangle {
    // ── UI-only ephemeral edit state (page-local) ────────────────────────
    in-out property <string> mock-name:    "";
    in-out property <bool>   mock-enabled: true;
    in-out property <[MacroStep]> mock-steps: [];
    in-out property <bool>   show-add-step-picker: false;

    // Predefined catalogue of action ids that can become macro steps.
    // Hardcoded for UI-only; Phase 8 takes the list from Bridge.quick-actions.
    property <[{action-id: string, label: string}]> available-actions: [
        { action-id: "scan-qr",        label: "Scan QR" },
        { action-id: "audio",          label: "Open Audio" },
        { action-id: "camera",         label: "Open Camera" },
        { action-id: "record",         label: "Start Recording" },
        { action-id: "stop-recording", label: "Stop Recording" },
        { action-id: "stop-cast",      label: "Stop Cast" },
    ];

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    VerticalLayout {
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                spacing: Theme.spacing-default;
                Text {
                    text: Bridge.mock-macro-edit-id == ""
                        ? "New macro"
                        : "Edit macro";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Cancel";
                    clicked => { Bridge.active-panel = Panel.macros; }
                }
                PrimaryButton {
                    label: "Save";
                    // UI-only: no real save. Phase 8 wires
                    // Bridge.save-macro(mock-name, mock-steps, mock-enabled).
                    clicked => { Bridge.active-panel = Panel.macros; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── Name field ──────────────────────────────────────────
                Text {
                    text: "NAME";
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                }
                LineEdit {
                    text <=> root.mock-name;
                    placeholder-text: "Untitled macro";
                }

                // ── Enable toggle ───────────────────────────────────────
                Rectangle {
                    height: 48px;
                    border-radius: Theme.radius-card;
                    background: Theme.surface-card;
                    HorizontalLayout {
                        padding-left:  Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        Text {
                            text: "Enabled";
                            color: Theme.text-primary;
                            font-size: Theme.font-size-body;
                            vertical-alignment: center;
                            horizontal-stretch: 1;
                        }
                        Text {
                            text: root.mock-enabled ? "On" : "Off";
                            color: root.mock-enabled
                                ? Theme.accent-active
                                : Theme.text-secondary;
                            vertical-alignment: center;
                            horizontal-alignment: end;
                            font-size: Theme.font-size-label;
                        }
                        TouchArea {
                            clicked => { root.mock-enabled = !root.mock-enabled; }
                        }
                    }
                }

                // ── Steps ───────────────────────────────────────────────
                Text {
                    text: "STEPS";
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                }

                if root.mock-steps.length == 0: Rectangle {
                    height: 64px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    Text {
                        text: "No steps yet. Tap “Add step” below.";
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }

                for step[i] in root.mock-steps: Rectangle {
                    height: 56px;
                    border-radius: Theme.radius-card;
                    background: Theme.surface-card;
                    HorizontalLayout {
                        padding-left:  Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        spacing: Theme.spacing-default;
                        Text {
                            text: step.label;
                            color: Theme.text-primary;
                            font-size: Theme.font-size-body;
                            vertical-alignment: center;
                            horizontal-stretch: 1;
                        }

                        // Reorder buttons. Same hardcoded-swap caveat as
                        // Phase 17. For UI-only build with up to ~5
                        // steps, document the workaround:
                        //   - Use a Bridge-side move-step(int, int) callback
                        //     once Phase 8 lands.
                        //   - For now, the up/down buttons are visually
                        //     present but no-op.
                        Rectangle {
                            width: 32px; height: 32px;
                            opacity: i == 0 ? 0.3 : 1.0;
                            Text { text: "▲"; color: Theme.text-primary;
                                   horizontal-alignment: center;
                                   vertical-alignment: center; }
                            TouchArea { enabled: i > 0; clicked => { } }
                        }
                        Rectangle {
                            width: 32px; height: 32px;
                            opacity: i == root.mock-steps.length - 1 ? 0.3 : 1.0;
                            Text { text: "▼"; color: Theme.text-primary;
                                   horizontal-alignment: center;
                                   vertical-alignment: center; }
                            TouchArea {
                                enabled: i < root.mock-steps.length - 1;
                                clicked => { }
                            }
                        }

                        // Remove button.
                        Rectangle {
                            width: 32px; height: 32px;
                            Text { text: "✕"; color: Theme.text-primary;
                                   horizontal-alignment: center;
                                   vertical-alignment: center; }
                            TouchArea {
                                clicked => {
                                    // Removal also requires hardcoded
                                    // index rebuild in UI-only build.
                                    // Stub no-op.
                                }
                            }
                        }
                    }
                }

                PrimaryButton {
                    label: "Add step";
                    clicked => { root.show-add-step-picker = true; }
                }
            }
        }
    }

    // ── Add-step picker overlay ──────────────────────────────────────────
    //
    // Inline picker — reuses the controlled-component pattern from
    // ConfirmDialog (Phase 19). Tapping an action appends a stub step
    // (no-op in UI-only build, since the array-rebuild for append is
    // also hardcoded; document and skip).
    if root.show-add-step-picker: Rectangle {
        width: 100%;
        height: 100%;
        background: #00000080;

        TouchArea {
            clicked => { root.show-add-step-picker = false; }
        }

        Rectangle {
            x: (parent.width  - self.width)  / 2;
            y: (parent.height - self.height) / 2;
            width: min(parent.width * 0.85, 360px);
            height: 320px;
            background: Theme.surface-card;
            border-radius: Theme.radius-card;

            TouchArea { }   // absorb clicks

            VerticalLayout {
                padding: Theme.padding-card;
                spacing: 8px;
                Text {
                    text: "Choose action";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                }
                ScrollView {
                    VerticalLayout {
                        spacing: 4px;
                        for action in root.available-actions: Rectangle {
                            height: 40px;
                            border-radius: 8px;
                            background: action-ta.pressed
                                ? Theme.surface-card.brighter(20%)
                                : transparent;
                            action-ta := TouchArea {
                                clicked => {
                                    // UI-only: spec defers append to
                                    // Phase 8. Just dismiss the picker.
                                    root.show-add-step-picker = false;
                                }
                            }
                            Text {
                                text: action.label;
                                color: Theme.text-primary;
                                vertical-alignment: center;
                                horizontal-alignment: start;
                                x: 12px;
                            }
                        }
                    }
                }
                HorizontalLayout {
                    alignment: end;
                    TextButton {
                        label: "Cancel";
                        clicked => { root.show-add-step-picker = false; }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`Bridge.mock-macro-edit-id == ""`** branches the header text "New macro" vs "Edit macro". Same dispatch idea as Phase 19's `pending-action`.
- **`text <=> root.mock-name;`** on `LineEdit` — two-way binding, same as Phase 16's name field. See [lineedit.mdx][lineedit].
- **`available-actions: [{action-id, label}, ...]`** — anonymous-struct-array literal. Same as Phase 21's `mock-versions`. The catalogue is hardcoded so Phase 8 can swap it for `Bridge.quick-actions`.
- **Reorder + remove buttons rendered but no-op'd in UI-only build.** The spec explicitly defers the array-mutation work to Phase 8. The visual scaffolding is here so the UI looks complete; functionality lights up when Bridge gets `move-step(macro-id, from, to)` and `remove-step(macro-id, idx)` callbacks. Document this clearly in the gotcha section.
- **Add-step picker is the same scrim+card pattern** as `ConfirmDialog` (Phase 19) but inlined, because the picker is bespoke (a ScrollView of action buttons, not a yes/no question). If you'd prefer a reusable `ActionPicker` component, extract it — that's a Phase-27 utils consideration.
- **Save returns to `Panel.macros`, not `Panel.none`** — back-stack invariant. Same as Phase 16.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 5 — Add `AUTOMATION` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

```diff
+                // ── Section: AUTOMATION ───────────────────────────────────
+                SettingsSection {
+                    title: "AUTOMATION";
+                    SettingsValueRow {
+                        title: "Macros";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.macros; }
+                    }
+                }
+
                 // ── Section: ABOUT & SUPPORT ──────────────────────────────
```

---

## Step 6 — Document `id: "macro:<id>"` convention in `control_bar.slint`

**File:** `senders/android/ui/components/control_bar.slint`

The spec says: *"when a bar action's id starts with `macro:`, render its title with a small ▶ glyph prefix."* This is a one-line tweak to `QuickActionButton`'s `Text { text: ... }` binding.

### Diff

```diff
     Text {
-        text: root.action.title;
+        // Macro-flagged actions get a ▶ glyph prefix. The quick-action
+        // customisation page (Phase 17) treats id == "macro:<id>" as
+        // shorthand for "this bar entry runs macro <id>".
+        text: root.action.id.starts-with("macro:")
+            ? "▶ " + root.action.title
+            : root.action.title;
         color: Theme.text-primary;
         ...
     }
```

### Why

- **`string.starts-with(pattern)` method** on Slint strings — see [expressions-and-statements.mdx][expressions]; older versions may use `string.contains(...)` or substring index inspection. If `starts-with` isn't available in your pinned Slint version, fall back to `string-equal-prefix(root.action.id, "macro:")` via a small pure helper.
- **`▶`** is a single Unicode glyph (U+25B6); no font asset needed.
- **String concatenation `"▶ " + title`** — works because both operands are strings.
- **Tweak is visual-only** — no behavioural change. The dispatcher in `CastControlBar.invoked(id)` still routes through `Bridge.invoke-action(id)`. Phase 8's macro execution engine is what makes the click do something.

---

## Sanity grep before commit

```sh
# 1. Both structs + 2 Panel variants in bridge.slint.
grep -n 'export struct MacroStep\|export struct Macro\b\|macros,\|macro-edit,\|mock-macro-edit-id' \
    senders/android/ui/bridge.slint
# Expected: 5 matches.

# 2. Both pages exported.
grep -rn 'export component MacrosPage\|export component MacroEditPage' \
    senders/android/ui/pages/
# Expected: 2 matches.

# 3. main.slint routes both.
grep -n 'Panel\.macros\|Panel\.macro-edit' senders/android/ui/main.slint
# Expected: 2 matches.

# 4. AUTOMATION section in FullSettingsPage.
grep -n 'AUTOMATION\|Panel\.macros' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

# 5. ▶ glyph prefix on macro-flagged actions.
grep -n 'macro:' senders/android/ui/components/control_bar.slint
# Expected: 1+ matches (the starts-with check).

# 6. MacroEditPage uses LineEdit two-way binding.
grep -n 'text <=> root.mock-name' senders/android/ui/pages/macro_edit_page.slint
# Expected: 1 match.

# 7. Edit page returns to Panel.macros, not Panel.none.
grep -n 'Bridge\.active-panel = Panel\.' senders/android/ui/pages/macro_edit_page.slint
# Expected: 2 matches (Cancel + Save), both Panel.macros.

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
#   modified:   senders/android/ui/components/control_bar.slint
#   new file:   senders/android/ui/pages/macros_page.slint
#   new file:   senders/android/ui/pages/macro_edit_page.slint
git commit -m "feat(slint-ui): Phase 25 — macros list + per-macro editor (UI-only)"
```

---

## Gotchas (Phase 25 specific)

### Gotcha 37 — `mock-macros` lives on `MacrosPage`, not `Bridge`

**Symptom:** clicking Save on the edit page navigates back to the list page, but the changes don't appear because the list re-instantiates with the original initialiser.

**Cause:** Slint's `if cond: Page { }` conditional re-instantiates the component every time the condition flips. So `MacrosPage`'s `mock-macros` initialiser runs again, reverting any in-memory edits.

**Fix:** for true round-tripping, promote `mock-macros` to `Bridge.macros: [Macro]`. Both pages then read/write a single shared store. The guide leaves it on `MacrosPage` to match the spec's "no persistence" intent — Phase 8 will move it. If you want round-tripping in UI-only build, do this now:

```diff
 export global Bridge {
     ...
+    in-out property <[Macro]> mock-macros: [
+        { id: "m1", name: "Start morning cast", enabled: true, steps: [...] },
+        ...
+    ];
 }
```

then bind the page's `for macro in Bridge.mock-macros:` directly. The spec calls this out as deferred but doesn't forbid the early move.

### Gotcha 38 — Edit page must look up the macro by id

**Symptom:** editing macro `m2` shows `m1`'s data — the page reads only the first entry of `mock-macros`.

**Cause:** Slint has no map / dictionary type. The edit page must search `mock-macros` linearly to find the entry matching `Bridge.mock-macro-edit-id`.

**Fix:** add a pure helper to the edit page:

```slint
pure function find-macro(id: string) -> Macro {
    // Hardcoded for the 2-entry stub model. Phase 8 makes this
    // a Rust-side lookup pushed via Bridge.
    return Bridge.mock-macros[0].id == id  ? Bridge.mock-macros[0] :
           Bridge.mock-macros[1].id == id  ? Bridge.mock-macros[1] :
                                              { id: "", name: "", enabled: true, steps: [] };
}
```

then bind `mock-name: find-macro(Bridge.mock-macro-edit-id).name;` etc. The hardcoded lookup is acceptable for 2 entries; it doesn't generalise but Phase 8 replaces the entire pattern. *This guide's snippet skips the lookup entirely* — the page-local `mock-name` etc. start blank because the spec defers the read-from-list step to Phase 8. If you want a proper round-trip in UI-only build, add this helper.

### Gotcha 39 — Nested struct-array initialisers are syntactically picky

**Symptom:** Slint compiler error like `expected '}', got '['` on the `mock-macros: [{...}, {...}]` initialiser.

**Cause:** the initialiser literal must use exact field names matching the struct, in any order, and the inner `steps: [...]` must use the matching nested struct shape. Misspelling `action_id` instead of `action-id` (Slint uses kebab-case) or omitting one of `enabled`/`steps`/`name` triggers cryptic errors.

**Fix:** copy the initialiser snippet from the `MacrosPage` source verbatim, then mutate. Slint requires every struct field to be initialised — `[{ id: "x", name: "y" }]` is *not* valid for `Macro`; you must include `steps: []` and `enabled: true`.

### Gotcha 40 — `string.starts-with` may not exist on older Slint

**Symptom:** `unknown method 'starts-with' on string` when compiling the Phase 25 control-bar tweak.

**Cause:** Slint added `starts-with` / `ends-with` / `contains` methods in 1.4. Older versions lack them.

**Fix:** check `senders/android/Cargo.toml` for the pinned Slint version. If 1.3 or older, write a positional check:

```slint
text: root.action.id.character-count >= 6 &&
      root.action.id.substring(0, 6) == "macro:"
    ? "▶ " + root.action.title
    : root.action.title;
```

The cleaner alternative: have the Phase 17 quick-action customisation persist a separate `is-macro: bool` field on `QuickAction`. But that requires extending the existing `QuickAction` struct and matters less than just upgrading the Slint version.

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `MacroStep` + `Macro` structs and `Panel.macros` + `Panel.macro-edit` variants.
- [ ] `Bridge.mock-macro-edit-id: string` exists on Bridge.
- [ ] `main.slint` routes both panels.
- [ ] `MacrosPage` lists 2 stub macros with name + step-count badge + enable label.
- [ ] Tap row → `MacroEditPage` opens with the macro's id in `Bridge.mock-macro-edit-id`.
- [ ] Tap "Add macro" → `MacroEditPage` opens with `Bridge.mock-macro-edit-id == ""` (renders "New macro" header).
- [ ] `MacroEditPage` has a `LineEdit` for name (two-way bound), an Enabled toggle, a steps list with ▲/▼/✕ buttons (no-op in UI-only build), and an "Add step" button that opens the picker overlay.
- [ ] Add-step picker shows 6 stub action labels; tapping one or Cancel dismisses the picker.
- [ ] Save and Cancel both return to `Panel.macros`, not `Panel.none`.
- [ ] `FullSettingsPage` has an `AUTOMATION` section with one row.
- [ ] `CastControlBar` renders ▶ prefix when `action.id` starts with `"macro:"`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <[Macro]> macros;
+    callback save-macro(string, string, bool, [MacroStep]);   // id, name, enabled, steps
+    callback delete-macro(string);
+    callback move-step(string, int, int);                      // macro-id, from, to
+    callback add-step(string, string);                         // macro-id, action-id
+    callback remove-step(string, int);                         // macro-id, idx
+    callback run-macro(string);                                // by id
```

- `Bridge.macros` ← Rust holds canonical list; persists to local storage.
- All mutation callbacks fire on the corresponding UI event; Rust mutates and pushes the new list back.
- `run-macro(id)` invoked from `Bridge.invoke-action(id)` when `id.starts-with("macro:")` — Rust-side macro execution engine takes over.
- The page-local `mock-name` / `mock-enabled` / `mock-steps` properties merge with Bridge once the lookup helper is in place.
- The hardcoded `available-actions` becomes `Bridge.quick-actions` filtered to actions that make sense as macro steps (no infinite-recursion `id == "macro:..."` allowed; Rust gates this).

---

## Slint-doc references used

- **Nested struct-array field `steps: [MacroStep]`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **Anonymous struct array `[{action-id: ..., label: ...}, ...]` in property init** — same.
- **Two-way binding `text <=> root.mock-name;`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`LineEdit { text, placeholder-text }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx`.
- **String interpolation `"\{n}"` and concat with ternary** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **Indexed `for step[i] in array:`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Conditional element `if cond: Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **Inline picker overlay (scrim + card + ScrollView of TouchArea rows)** — composition pattern; see `ConfirmDialog` reference in [`PHASE-19-reimplement-instructions.md`](./PHASE-19-reimplement-instructions.md).
- **String method `starts-with`** — Slint runtime API; documented in `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx` (string built-ins section).
- **`SettingsSection`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`PrimaryButton`, `DestructiveButton`, `TextButton`** — FCast components in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real macro execution.** Phase 8 + a Rust scheduler/runner.
- **Step types beyond simple action invocation** (delays, conditions, loops). Out of scope.
- **Macro export / import.** Out of scope.
- **Macro recording from user actions.** Out of scope.
- **Functional reorder / remove / append on `mock-steps`.** Spec defers — buttons are visual-only.
- **Lookup-by-id on the edit page.** Spec defers; gotcha 38 documents the helper.
- **Promoting `mock-macros` to Bridge in UI-only build.** Spec defers; gotcha 37 documents the move.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-25-macros-action-chains.md
[p14]: ./PHASE-14-reimplement-instructions.md
[p16]: ./PHASE-16-reimplement-instructions.md
[p17]: ./PHASE-17-reimplement-instructions.md
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[lineedit]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx

# Phase 17 — Quick-Action Customization reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-17-quick-action-customization.md`][spec] to the current `senders/android` tree.
**Goal:** add `QuickActionsPage` — a settings sub-page that lists the bar's actions and lets the user enable/disable each and reorder via ▲/▼ buttons. Add a `Panel.quick-actions` variant; route in `main.slint`; link from `FullSettingsPage` `DISPLAY` section.
**Scope:** Slint UI only. **No Rust changes.** No drag-and-drop gesture (deferred — Slint has no built-in `Draggable` in the pinned version). No persistence.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-17-quick-action-customization.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] and [`PHASE-16-reimplement-instructions.md`][p16] first.** This guide reuses Phase 14's chrome and Phase 16's "rebuild the entire array on field-level mutation" pattern. The new things in Phase 17: array-rebuild for **swap** instead of single-cell update, conditional banner on `length > N`, and the introduction of the new `DISPLAY` section in `FullSettingsPage`.

[p14]: ./PHASE-14-reimplement-instructions.md
[p16]: ./PHASE-16-reimplement-instructions.md

---

## Why this guide exists

Phase 17 looks like another simple list page, but the array-mutation pattern is significantly more involved than Phases 16 / 22:

1. **`QuickAction` struct already exists** in `bridge.slint` (declared in Phase 2/3) and is consumed by `CastControlBar.mock-quick-actions`. Phase 17 introduces a separate `mock-bar-actions` list on the page so changes don't immediately wreck the live control bar — the spec is explicit: "Functional integration: Deferred — the control bar's mock model is replaced wholesale on each save; no persistence."
2. **Reorder requires a swap helper.** Slint has no `splice`, no spread `...`, no concat operator on arrays. For a fixed-length stub model (5 entries), the swap helper is a hardcoded re-construction. For a dynamic list, this approach saturates — Phase 8 lifts the helper to Rust.
3. **Conditional banner on overflow.** `if root.mock-bar-actions.length > 6: Rectangle { ... }` triggers when the user adds more than the bar can hold. Slint's conditional-element-in-layout pattern handles this cleanly.

After Phases 14 + 15 + 16 + 21 + 22 + 23 + 26 merge:

- `Panel { ..., debug-video }`. Phase 17 adds **one** variant: `quick-actions`.
- `QuickAction` struct is already in `bridge.slint`. No struct changes needed.
- `FullSettingsPage` does not have a `DISPLAY` section yet. Phase 17 inserts it between `VIDEO QUALITY` and `AUDIO & VIDEO` (or wherever fits — this guide chooses just-before-`AUDIO & VIDEO` for visual grouping).

This is **strictly additive** Slint work spread across **three existing files** plus **one new file**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'QuickActionsPage\|Panel\.quick-actions\|mock-bar-actions' \
    senders/android/ui/

# DISPLAY section must not already exist:
grep -n 'DISPLAY' senders/android/ui/pages/settings_page.slint
# Expected: (empty)

# QuickAction struct exists from Phase 2/3:
grep -n 'export struct QuickAction' senders/android/ui/bridge.slint
# Expected: 1 match.

# Existing CastControlBar's mock model:
grep -n 'mock-quick-actions' senders/android/ui/components/control_bar.slint
# Expected: 2 matches (declaration + use).
```

After this guide is applied:

```sh
grep -n 'quick-actions,' senders/android/ui/bridge.slint                 # Expected: 1 (Panel variant)
grep -n 'export component QuickActionsPage' senders/android/ui/pages/quick_actions_page.slint
# Expected: 1
grep -n 'Panel\.quick-actions' senders/android/ui/main.slint             # Expected: 1
grep -n 'DISPLAY\|Panel\.quick-actions' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches (section title + opener handler)
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-17-quick-actions
cargo check -p android-sender
```

---

## Step 1 — Add `Panel.quick-actions` in `bridge.slint`

```diff
 export enum Panel {
     ...
     debug-video,
+    quick-actions,
 }
```

No struct changes. `QuickAction` is already exported.

---

## Step 2 — Route `Panel.quick-actions` in `main.slint`

```diff
 import { DebugVideoPage }               from "pages/debug_video_page.slint";
+import { QuickActionsPage }             from "pages/quick_actions_page.slint";
```

```diff
     if Bridge.active-panel == Panel.debug-video:     DebugVideoPage { }
+    if Bridge.active-panel == Panel.quick-actions:   QuickActionsPage { }
 }
```

---

## Step 3 — Create `pages/quick_actions_page.slint`

**File:** `senders/android/ui/pages/quick_actions_page.slint` (new)

The page holds its own copy of the list (the spec calls it `mock-bar-actions`). Reorder / toggle changes mutate this copy. Phase 8 pushes the saved list back to `Bridge.quick-actions`.

### New file

```slint
// quick_actions_page.slint — Reorder / enable / disable bar actions.
//
// Reachable from FullSettingsPage's "Quick actions" row in DISPLAY.
// Holds its own copy of the bar list — changes here do not affect the
// live CastControlBar in this UI-only build. Phase 8 will push the saved
// list back to Bridge.quick-actions.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, QuickAction } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
} from "../components/settings_rows.slint";

// Internal row sub-component. Same rationale as Phase 22's
// NetworkInterfaceRow — local state (none here) + clean up/down/toggle
// callback boundaries. Even without local state, separating the row
// keeps the page's main layout readable.
component QuickActionRow inherits Rectangle {
    in property <QuickAction> action;
    in property <bool>        is-first;
    in property <bool>        is-last;
    callback move-up();
    callback move-down();
    callback toggle-enabled(bool);

    height: 56px;
    border-radius: Theme.radius-card;
    background: Theme.surface-card;
    opacity: root.action.enabled ? 1.0 : 0.55;

    HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        spacing: Theme.spacing-default;

        Text {
            text: root.action.title;
            color: Theme.text-primary;
            font-size: Theme.font-size-body;
            vertical-alignment: center;
            horizontal-stretch: 1;
        }

        // ▲ button — disabled when at top.
        Rectangle {
            width: 40px;
            height: 40px;
            background: ta-up.pressed ? Theme.surface-card.brighter(20%) : transparent;
            opacity: root.is-first ? 0.3 : 1.0;
            border-radius: 8px;
            ta-up := TouchArea {
                enabled: !root.is-first;
                clicked => { root.move-up(); }
            }
            Text {
                text: "▲";
                color: Theme.text-primary;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }

        // ▼ button — disabled when at bottom.
        Rectangle {
            width: 40px;
            height: 40px;
            background: ta-down.pressed ? Theme.surface-card.brighter(20%) : transparent;
            opacity: root.is-last ? 0.3 : 1.0;
            border-radius: 8px;
            ta-down := TouchArea {
                enabled: !root.is-last;
                clicked => { root.move-down(); }
            }
            Text {
                text: "▼";
                color: Theme.text-primary;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }

        // Enable toggle — inline (Off/On text, same as Phase 22 inline pattern).
        Rectangle {
            width: 56px;
            Text {
                text: root.action.enabled ? "On" : "Off";
                color: root.action.enabled
                    ? Theme.accent-active
                    : Theme.text-secondary;
                horizontal-alignment: end;
                vertical-alignment: center;
                font-size: Theme.font-size-label;
            }
            TouchArea {
                clicked => { root.toggle-enabled(!root.action.enabled); }
            }
        }
    }
}

export component QuickActionsPage inherits Rectangle {
    // ── UI-only stub state (page-local copy) ────────────────────────────
    in-out property <[QuickAction]> mock-bar-actions: [
        { id: "scan-qr",    title: "Scan QR",     enabled: true,  active: false },
        { id: "settings",   title: "Settings",    enabled: true,  active: false },
        { id: "debug",      title: "Debug",       enabled: false, active: false },
        { id: "codec-test", title: "Codec Test",  enabled: true,  active: false },
        { id: "audio",      title: "Audio",       enabled: true,  active: false },
    ];

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // ── Array-rebuild helpers ────────────────────────────────────────────
    //
    // Slint has no `splice` / `swap` primitive on arrays. For a small
    // fixed-shape model (≤ 5 entries) we hardcode the swap by index;
    // each branch reconstructs the list with two adjacent entries
    // exchanged. Yes, this is verbose. The right long-term home for this
    // logic is Rust (Phase 8); the hardcoded form is only acceptable
    // because the model is a 5-row stub and the page is UI-only.
    function swap(i: int, j: int) {
        // Build a fresh array entry-by-entry, swapping i and j.
        root.mock-bar-actions = [
            root.mock-bar-actions[0 == i ? j : (0 == j ? i : 0)],
            root.mock-bar-actions[1 == i ? j : (1 == j ? i : 1)],
            root.mock-bar-actions[2 == i ? j : (2 == j ? i : 2)],
            root.mock-bar-actions[3 == i ? j : (3 == j ? i : 3)],
            root.mock-bar-actions[4 == i ? j : (4 == j ? i : 4)],
        ];
    }

    function set-enabled(idx: int, value: bool) {
        // Same shape — rebuild with the targeted entry's `enabled` flipped.
        root.mock-bar-actions = [
            { id: root.mock-bar-actions[0].id, title: root.mock-bar-actions[0].title,
              active: root.mock-bar-actions[0].active,
              enabled: 0 == idx ? value : root.mock-bar-actions[0].enabled },
            { id: root.mock-bar-actions[1].id, title: root.mock-bar-actions[1].title,
              active: root.mock-bar-actions[1].active,
              enabled: 1 == idx ? value : root.mock-bar-actions[1].enabled },
            { id: root.mock-bar-actions[2].id, title: root.mock-bar-actions[2].title,
              active: root.mock-bar-actions[2].active,
              enabled: 2 == idx ? value : root.mock-bar-actions[2].enabled },
            { id: root.mock-bar-actions[3].id, title: root.mock-bar-actions[3].title,
              active: root.mock-bar-actions[3].active,
              enabled: 3 == idx ? value : root.mock-bar-actions[3].enabled },
            { id: root.mock-bar-actions[4].id, title: root.mock-bar-actions[4].title,
              active: root.mock-bar-actions[4].active,
              enabled: 4 == idx ? value : root.mock-bar-actions[4].enabled },
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
                    text: "Quick actions";
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

        // ── Overflow banner ─────────────────────────────────────────────
        if root.mock-bar-actions.length > 6: Rectangle {
            height: 40px;
            background: Theme.accent-active.darker(30%);
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Bar holds up to 6 actions — extras will overflow.";
                    color: Theme.text-primary;
                    vertical-alignment: center;
                    font-size: Theme.font-size-label;
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── Empty state ─────────────────────────────────────────
                if root.mock-bar-actions.length == 0: Rectangle {
                    height: 80px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    HorizontalLayout {
                        padding: Theme.padding-screen;
                        Text {
                            text: "No bar actions configured.";
                            color: Theme.text-secondary;
                            vertical-alignment: center;
                            horizontal-alignment: center;
                            horizontal-stretch: 1;
                        }
                    }
                }

                // ── Section header ──────────────────────────────────────
                SettingsSection { title: "BAR ACTIONS"; }

                for action[i] in root.mock-bar-actions: QuickActionRow {
                    action: action;
                    is-first: i == 0;
                    is-last:  i == root.mock-bar-actions.length - 1;
                    move-up()           => { root.swap(i, i - 1); }
                    move-down()         => { root.swap(i, i + 1); }
                    toggle-enabled(v)   => { root.set-enabled(i, v); }
                }
            }
        }
    }
}
```

### Why each piece

- **`for action[i] in root.mock-bar-actions:`** — Slint's `[index]` form on the for-loop variable. The `i` is bound to the iteration index. See [repetition-and-data-models.mdx][repeat].
- **`is-first: i == 0;` and `is-last: i == root.mock-bar-actions.length - 1;`** — pass the boundary state into the row sub-component so it can disable / dim the appropriate arrow. Slint property bindings are reactive; if the model permutes (post-swap), `is-first` re-evaluates.
- **Array `.length` property** — Slint exposes `length` on array-typed properties. Per [repetition-and-data-models.mdx][repeat].
- **`swap(i, j)` rebuilds the entire array.** Each entry is selected via a triple ternary on the index: if this position is `i`, take the entry at position `j`; if this position is `j`, take the entry at position `i`; otherwise take the entry at this position. This is the hardcoded 5-row form of a swap operation.
- **`set-enabled(idx, value)` reconstructs every row** with all fields preserved except the targeted `enabled`. Same Phase-16 / Phase-22 pattern.
- **`if root.mock-bar-actions.length > 6: Rectangle { ... }`** — conditional element inside a layout. The rectangle is part of the flow when present and absent (zero-flex) when not. See [file.mdx][file] (conditional elements).
- **Empty state inside `ScrollView`** — same conditional-element pattern. The `if length == 0:` rectangle replaces the row list when the model is empty.
- **Inline enable/disable label** rather than full `SettingsToggleRow` — same rationale as Phase 22: avoids overlapping touch targets when the row's main click area doesn't exist (here there is no row-level click; just inline arrow buttons + toggle).
- **`opacity: root.action.enabled ? 1.0 : 0.55;`** on the row — a conventional disabled-row visual cue that costs nothing and reinforces the toggle state.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Add `DISPLAY` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Insert a new `DISPLAY` section between `VIDEO QUALITY` and `AUDIO & VIDEO`.

### Diff (showing the relative position)

```diff
                 // ── Section: VIDEO QUALITY ────────────────────────────────
                 SettingsSection {
                     title: "VIDEO QUALITY";
                     ...
                 }

+                // ── Section: DISPLAY ──────────────────────────────────────
+                SettingsSection {
+                    title: "DISPLAY";
+                    SettingsValueRow {
+                        title: "Quick actions";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.quick-actions; }
+                    }
+                }
+
                 // ── Section: AUDIO & VIDEO ────────────────────────────────
                 SettingsSection {
                     title: "AUDIO & VIDEO";
                     ...
                 }
```

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. Panel.quick-actions in bridge.slint.
grep -n 'quick-actions,' senders/android/ui/bridge.slint
# Expected: 1 match.

# 2. Page exists.
grep -n 'export component QuickActionsPage\|component QuickActionRow' \
    senders/android/ui/pages/quick_actions_page.slint
# Expected: 2 matches.

# 3. Page uses indexed for-loop.
grep -n 'for action\[i\] in' senders/android/ui/pages/quick_actions_page.slint
# Expected: 1 match.

# 4. Reorder + toggle helpers present.
grep -n 'function swap\|function set-enabled' \
    senders/android/ui/pages/quick_actions_page.slint
# Expected: 2 matches.

# 5. DISPLAY section in FullSettingsPage.
grep -n 'DISPLAY\|Panel\.quick-actions' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

# 6. main.slint routes Panel.quick-actions.
grep -n 'Panel\.quick-actions' senders/android/ui/main.slint
# Expected: 1 match.

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
#   new file:   senders/android/ui/pages/quick_actions_page.slint
git commit -m "feat(slint-ui): Phase 17 — quick-action customisation page (UI-only)"
```

---

## Gotchas (Phase 17 specific)

### Gotcha 24 — Array swap helper does not generalise

**Symptom:** the 5-row hardcoded `swap(i, j)` works for the stub model but fails as soon as the list grows or shrinks.

**Cause:** Slint has no spread (`...`), no array concatenation, no `for` inside an array literal. Generic swap requires an iterator, which the language does not expose.

**Fix:**
- For UI-only stubs ≤ ~5 entries, hardcode the rebuild as in this guide.
- For larger or dynamic lists, defer to Rust (Phase 8). The `Bridge.move-quick-action(int, int)` callback is the right home; the Slint side becomes `move-up() => { Bridge.move-quick-action(i, i - 1); }`.

This is the same wall Phase 16 hit with `set-active(id)` and Phase 22 hit with `set-enabled(name, value)`. The 3-, 5-, and 5-row hardcoded forms exist in the codebase as cousins — keep them small and document the upgrade path.

### Gotcha 25 — `[index]` indexed for is positional, not stable

**Symptom:** after a swap, the row's animation jumps incorrectly because Slint doesn't track row identity across re-renders.

**Cause:** Slint's `for` loop diffs by position by default, not by identity. When you swap entries, position 0's contents change, and Slint reuses the existing component with new property values rather than creating a new component for the swapped entry.

**Fix:** for arrow-based reorder this is fine — there's no animation state to preserve. For drag-reorder (deferred to Phase 8 + a future drag library), you'd need keyed children. Slint 1.5+ supports a `for entry[i] in array key: <expr>` form on some targets; check the runtime version before relying on it. UI-only build doesn't need keys.

### Gotcha 26 — `if length > 6` requires the model to actually grow

**Symptom:** the overflow banner never appears in testing because the stub has 5 entries.

**Cause:** there's no UI affordance in this phase for adding entries — the spec says "test by temporarily padding the mock list."

**Fix:** during testing, edit the `mock-bar-actions:` initializer to `[..., ..., ..., ..., ..., ..., ...]` (7 entries) and recompile. Verify the banner appears, then revert. Don't ship with the padded form.

### Gotcha 27 — `transparent` color is not a theme reference

**Symptom:** the ▲/▼ buttons render with a faint card background even when not pressed.

**Cause:** `background: ta.pressed ? Theme.surface-card.brighter(20%) : transparent;` — `transparent` is a Slint built-in (`#00000000`). On some platforms / themes, however, the parent's background bleeds through differently from a real `surface-card`-equivalent.

**Fix (already in the snippet):** use the literal `transparent`. If the parent's background causes issues, switch to `background: Theme.surface-card;` (matching the row body) — the ▲/▼ then look like flush surfaces, which is also acceptable visually.

---

## Exit criteria checklist

- [ ] `bridge.slint` adds `Panel.quick-actions` variant.
- [ ] `main.slint` routes `Panel.quick-actions`.
- [ ] `QuickActionsPage` renders 5 stub bar actions with title + ▲ + ▼ + Off/On toggle.
- [ ] ▲ button is dimmed at row 0; ▼ button is dimmed at last row; both swap with the neighbour on click.
- [ ] Enable toggle flips the row's `enabled` flag (whole-array reassignment).
- [ ] Disabled rows render at 0.55 opacity.
- [ ] Empty state card appears when `length == 0` (test by temporarily emptying the initializer).
- [ ] Overflow banner appears when `length > 6` (test by temporarily padding the initializer).
- [ ] `FullSettingsPage` has a new `DISPLAY` section with one row.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <[QuickAction]> bar-actions;
+    callback move-bar-action(int, int);
+    callback set-bar-action-enabled(int, bool);
+    callback save-bar-actions();
```

- `bar-actions` ← Rust holds canonical list, persisted to local storage.
- `move-bar-action(from, to)` → Rust does the array shuffle; pushes the new list back to Slint.
- `set-bar-action-enabled(idx, bool)` → same.
- `save-bar-actions()` (only if you go with explicit Save semantics rather than auto-save).
- The page's `mock-bar-actions` property and the local `swap` / `set-enabled` helpers are deleted; the view binds directly to `Bridge.bar-actions`.
- The Phase-23 quick-action shortcut for "Record" lands here too — Rust adds an entry with `id: "record"` and the existing dispatcher in `CastControlBar.invoked(id)` routes it through `Bridge.invoke-action("record")`, which the Rust handler maps to `Bridge.active-panel = Panel.recording` (or fires a callback that mutates state directly).

---

## Slint-doc references used

- **Indexed `for action[i] in array:`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Array `.length` property** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Array property reassignment** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **Conditional element `if cond: Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`function name(arg, arg) { ... }` (non-pure side-effecting)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Sub-component declaration `component QuickActionRow inherits Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`TouchArea.enabled`** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`Rectangle.opacity`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`SettingsSection`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`TextButton`** — FCast component in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Drag-and-drop reorder gesture.** Slint 1.5.x has no built-in `Draggable`. Phase 8 + a future drag library.
- **Add / remove rows from the page.** Spec lists only enable + reorder. Adding a "+ Add action" affordance is Phase 25 (macros) territory.
- **Persistence across app restart.** Phase 8.
- **Sync to live `CastControlBar`.** Phase 8 — push saved list back to `Bridge.quick-actions`.
- **Per-action label editing.** Out of scope.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-17-quick-action-customization.md
[p14]: ./PHASE-14-reimplement-instructions.md
[p16]: ./PHASE-16-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

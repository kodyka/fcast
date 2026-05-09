# Phase 19 — Settings Backup & Reset (with ConfirmDialog) reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-19-settings-backup-reset.md`][spec] to the current `senders/android` tree.
**Goal:** add a reusable `ConfirmDialog` component plus a `BackupResetPage` settings sub-page exposing export / import (pretend-success banner) and three destructive reset actions (each gated through a `ConfirmDialog`). Wired into the `Panel` overlay layer; linked from `FullSettingsPage` `DATA` section.
**Scope:** Slint UI only. **No Rust changes.** No real file I/O, no real persistence reset.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-19-settings-backup-reset.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] and [`PHASE-22-reimplement-instructions.md`][p22] first.** Phase 22 introduces the auto-hide banner Timer used here; Phase 14 introduces the panel-overlay routing pattern. The new things in Phase 19: a reusable component with both `confirmed()` and `dismissed()` callbacks (the first reusable shared overlay), and a "pending action" pattern where the page records *which* destructive action the user just confirmed and runs the appropriate cleanup banner on confirm.

[p14]: ./PHASE-14-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md

---

## Why this guide exists

Phase 19 is the **first phase to introduce a reusable shared component** — `ConfirmDialog` lives in `components/`, not in the page that uses it. Future phases (20 cast history, 27 utils) will reuse it. Implementing it correctly the first time matters; wrong shape now creates churn later.

The spec's main subtleties:

1. **`ConfirmDialog` component shape.** Two callbacks (`confirmed()`, `dismissed()`); two button variants (`PrimaryButton` / `DestructiveButton`) selected by an `in property <bool> destructive`; full-window scrim background.
2. **Pending-action state on the page.** When the user clicks "Reset all settings", the page must show the dialog with the right title/body. When the user clicks "Clear cast history", same dialog component but different content. The cleanest pattern: page records a `pending-action: string` and the dialog content + the on-confirm handler dispatch on it.
3. **Auto-hide banners** — same Timer pattern as Phase 22's Wi-Fi Aware banner, but parameterised by message.

After Phases 14 + 15 + 16 + 17 + 21 + 22 + 23 + 26 merge:

- `Panel { ..., quick-actions }`. Phase 19 adds **one** variant: `backup-reset`.
- `bridge.slint` has `BitratePreset`, `NetworkInterface`, `LogEntry` structs and `RecordingState`, `LogLevel` enums. Phase 19 adds **no struct** — `ConfirmDialog` takes plain string properties.
- `FullSettingsPage` does not have a `DATA` section yet. Phase 19 inserts it.

This is **strictly additive** Slint work spread across **three existing files** plus **two new files** (`components/confirm_dialog.slint`, `pages/backup_reset_page.slint`).

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'ConfirmDialog\|BackupResetPage\|Panel\.backup-reset\|pending-action' \
    senders/android/ui/

# No DATA section yet:
grep -n 'DATA' senders/android/ui/pages/settings_page.slint
# Expected: (empty)
```

After this guide is applied:

```sh
grep -n 'export component ConfirmDialog' senders/android/ui/components/confirm_dialog.slint
# Expected: 1 match.

grep -n 'backup-reset,' senders/android/ui/bridge.slint
# Expected: 1 (Panel variant)

grep -n 'export component BackupResetPage' senders/android/ui/pages/backup_reset_page.slint
# Expected: 1.

grep -n 'Panel\.backup-reset' senders/android/ui/main.slint
# Expected: 1.

grep -n 'DATA\|Panel\.backup-reset' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-19-backup-reset
cargo check -p android-sender
```

---

## Step 1 — Add `Panel.backup-reset` in `bridge.slint`

```diff
 export enum Panel {
     ...
     quick-actions,
+    backup-reset,
 }
```

No struct or property additions on Bridge. The dialog state lives on the page, not on Bridge — it's an ephemeral UI flag that the Rust side does not need.

---

## Step 2 — Create `components/confirm_dialog.slint`

**File:** `senders/android/ui/components/confirm_dialog.slint` (new)

A self-contained overlay that the consumer instantiates inline (`if root.show-dialog: ConfirmDialog { ... }`). The `confirmed()` and `dismissed()` callbacks let the consumer decide what to do in each case.

### New file

```slint
// confirm_dialog.slint — Reusable destructive/confirmation overlay.
//
// Designed to be instantiated inside a parent's conditional:
//
//     if root.show-dialog: ConfirmDialog {
//         title: "Delete forever?";
//         body:  "This cannot be undone.";
//         destructive: true;
//         confirm-label: "Delete";
//         confirmed  => { root.show-dialog = false; root.do-delete(); }
//         dismissed  => { root.show-dialog = false; }
//     }
//
// The component does NOT hide itself on confirm/dismiss — the consumer
// owns the visibility state. Keeps the component side-effect-free and
// composable across multiple dialog use cases.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx

import { Theme } from "../theme.slint";
import { PrimaryButton, DestructiveButton, TextButton } from "buttons.slint";

export component ConfirmDialog inherits Rectangle {
    in property <string> title;
    in property <string> body;
    in property <string> confirm-label: "Confirm";
    in property <string> dismiss-label: "Cancel";
    in property <bool>   destructive: false;
    callback confirmed();
    callback dismissed();

    width: 100%;
    height: 100%;
    background: #00000080;     // 50% opaque scrim — TODO Theme.scrim later

    // Outer scrim TouchArea: tapping outside the dialog dismisses.
    TouchArea {
        clicked => { root.dismissed(); }
    }

    // ── Centered card ────────────────────────────────────────────────────
    Rectangle {
        x: (parent.width  - self.width)  / 2;
        y: (parent.height - self.height) / 2;
        width: min(parent.width * 0.85, 480px);
        height: card-content.preferred-height;
        background: Theme.surface-card;
        border-radius: Theme.radius-card;

        // Inner TouchArea swallows clicks so they don't bubble to the
        // scrim and dismiss the dialog.
        TouchArea {
            // No clicked => — just absorbs.
        }

        card-content := VerticalLayout {
            padding: Theme.padding-card;
            spacing: 12px;

            Text {
                text: root.title;
                color: Theme.text-primary;
                font-size: Theme.font-size-heading;
            }

            Text {
                text: root.body;
                color: Theme.text-secondary;
                font-size: Theme.font-size-body;
                wrap: word-wrap;
            }

            HorizontalLayout {
                alignment: end;
                spacing: 8px;

                TextButton {
                    label: root.dismiss-label;
                    clicked => { root.dismissed(); }
                }

                if root.destructive: DestructiveButton {
                    label: root.confirm-label;
                    clicked => { root.confirmed(); }
                }
                if !root.destructive: PrimaryButton {
                    label: root.confirm-label;
                    clicked => { root.confirmed(); }
                }
            }
        }
    }
}
```

### Why each piece

- **`width: 100%; height: 100%;`** — fills the parent. The consumer (`BackupResetPage` or any future caller) places the dialog as a sibling of the page's main `VerticalLayout`, so it covers everything.
- **Outer + inner TouchArea pair** — outer dismisses on click; inner absorbs clicks so the card's interior doesn't trigger dismiss. This is the canonical scrim pattern. The inner has `clicked` callback because Slint requires *some* callback signature for a `TouchArea` to claim events.
- **`card-content := VerticalLayout`** — the named identifier lets the parent `Rectangle` size to `card-content.preferred-height`, so the card grows to fit body text without overflow. `preferred-height` is computed from the layout's children; per [positioning-and-layouts.mdx][positioning].
- **Two `if` blocks** for `DestructiveButton` vs `PrimaryButton` — Slint's conditional element. Only one is instantiated based on `destructive`. The button-shape difference matters: `DestructiveButton` is red-coded; `PrimaryButton` is accent-coded. Mixing them in a single Component with a `kind` property works too, but the `if` form is cleaner here.
- **No internal hide on confirm/dismiss** — the component is stateless. The consumer reading `confirmed()` is responsible for setting `show-dialog = false;`. This matches React's controlled-component pattern: dialog state is owned by the parent.
- **`Theme.scrim` left as TODO** — the spec calls this out. A real `Theme.scrim` color belongs in `theme.slint`, but adding it is Phase-9 / theming territory. The inline `#00000080` is acceptable for now; mark with comment.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 3 — Create `pages/backup_reset_page.slint`

**File:** `senders/android/ui/pages/backup_reset_page.slint` (new)

Two sections (`BACKUP` + `RESET`), an inline pending-action string, an auto-hide banner, and the `ConfirmDialog` instance.

### New file

```slint
// backup_reset_page.slint — Settings backup / import / reset (UI-only).
//
// Reachable from FullSettingsPage's "Backup & reset" row in DATA.
// Export/Import buttons trigger pretend-success banners; destructive
// rows go through a ConfirmDialog. UI-only — no real file I/O, no real
// persistence reset.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsValueRow,
} from "../components/settings_rows.slint";
import { ConfirmDialog } from "../components/confirm_dialog.slint";

export component BackupResetPage inherits Rectangle {
    // ── Banner state (auto-hide via Timer, same pattern as Phase 22) ────
    property <bool>   banner-visible: false;
    property <string> banner-message: "";

    // ── Dialog state ────────────────────────────────────────────────────
    //
    // pending-action is one of:
    //   ""               — no dialog
    //   "reset-all"      — Reset all settings
    //   "clear-history"  — Clear cast history
    //   "clear-receivers"— Clear known receivers
    property <string> pending-action: "";

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // Banner auto-hide.
    Timer {
        interval: 3s;
        running: root.banner-visible;
        triggered => { root.banner-visible = false; }
    }

    // Helper: show a banner with the given message.
    function show-banner(message: string) {
        root.banner-message = message;
        root.banner-visible = true;
    }

    // Helper: confirm-dispatch — dispatched by the dialog's confirmed()
    // callback based on the pending-action.
    function on-confirm() {
        if (root.pending-action == "reset-all") {
            root.show-banner("Settings reset (placeholder).");
        } else if (root.pending-action == "clear-history") {
            root.show-banner("Cast history cleared (placeholder).");
        } else if (root.pending-action == "clear-receivers") {
            root.show-banner("Known receivers cleared (placeholder).");
        }
        root.pending-action = "";
    }

    // Helper: pure derivation of the dialog title for the current action.
    pure function dialog-title(action: string) -> string {
        return action == "reset-all"        ? "Reset all settings?" :
               action == "clear-history"    ? "Clear cast history?"  :
               action == "clear-receivers"  ? "Clear known receivers?" :
                                               "";
    }
    pure function dialog-body(action: string) -> string {
        return action == "reset-all"
                 ? "All settings will be restored to defaults. This cannot be undone."
             : action == "clear-history"
                 ? "Past cast sessions will be removed from the history list."
             : action == "clear-receivers"
                 ? "Known receivers will be forgotten and rediscovered next scan."
             : "";
    }
    pure function dialog-confirm-label(action: string) -> string {
        return action == "reset-all"        ? "Reset" :
               action == "clear-history"    ? "Clear" :
               action == "clear-receivers"  ? "Clear" :
                                               "OK";
    }

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Backup & reset";
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

        // ── Banner ──────────────────────────────────────────────────────
        if root.banner-visible: Rectangle {
            height: 40px;
            background: Theme.accent-active.darker(20%);
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: root.banner-message;
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

                // ── Section: BACKUP ─────────────────────────────────────
                SettingsSection {
                    title: "BACKUP";
                    SettingsValueRow {
                        title: "Export settings to file";
                        value: "";
                        clicked => {
                            root.show-banner("Exported to ~/fcast-backup.json (placeholder).");
                        }
                    }
                    SettingsValueRow {
                        title: "Import settings from file";
                        value: "";
                        clicked => {
                            root.show-banner("Settings imported (placeholder).");
                        }
                    }
                }

                // ── Section: RESET ──────────────────────────────────────
                SettingsSection {
                    title: "RESET";
                    SettingsValueRow {
                        title: "Reset all settings";
                        value: "";
                        clicked => { root.pending-action = "reset-all"; }
                    }
                    SettingsValueRow {
                        title: "Clear cast history";
                        value: "";
                        clicked => { root.pending-action = "clear-history"; }
                    }
                    SettingsValueRow {
                        title: "Clear known receivers";
                        value: "";
                        clicked => { root.pending-action = "clear-receivers"; }
                    }
                }
            }
        }
    }

    // ── Dialog overlay (last in the parent so it paints on top) ──────────
    if root.pending-action != "": ConfirmDialog {
        title:          root.dialog-title(root.pending-action);
        body:           root.dialog-body(root.pending-action);
        confirm-label:  root.dialog-confirm-label(root.pending-action);
        destructive:    true;     // all three reset actions are destructive
        confirmed => {
            root.on-confirm();
            // pending-action is reset to "" inside on-confirm(), which
            // collapses the conditional and removes the dialog.
        }
        dismissed => { root.pending-action = ""; }
    }
}
```

### Why each piece

- **`property <string> pending-action: "";`** — empty string represents "no dialog". Slint enums would be more strictly typed, but a string is fine here because the action set is local and small. If you'd rather type-strictify, declare a local `enum BackupAction { none, reset-all, clear-history, clear-receivers }` in the file (Slint allows file-scope enums per [structs-and-enums.mdx][structs]).
- **`pure function dialog-title(action: string) -> string`** — pure-helper-returning-string pattern, same as Phase 23's HH:MM:SS formatter. Allows the `ConfirmDialog`'s `title:` binding to react to `pending-action` without cluttering the dialog instantiation.
- **`if root.pending-action != "": ConfirmDialog { ... }`** at the bottom — last in the parent so it paints on top of the rest. Same layering rationale as Phase 18's lifecycle overlays.
- **Two helpers (`show-banner`, `on-confirm`)** centralise the dispatch. The five `clicked => { ... }` handlers in the page body each do one thing and route through the helper.
- **`confirmed => { root.on-confirm(); }`** — the dialog's confirm callback runs the banner; `on-confirm()` clears `pending-action` last, which collapses the conditional and removes the dialog. This is the controlled-component pattern.
- **`dismissed => { root.pending-action = ""; }`** — Cancel just clears the dialog; no banner.
- **`destructive: true` is hardcoded** — all three reset actions are destructive. If a non-destructive action was added later, change to `destructive: root.pending-action != "non-destructive-id"` or compute via another pure function.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Route `Panel.backup-reset` in `main.slint`

```diff
 import { QuickActionsPage }             from "pages/quick_actions_page.slint";
+import { BackupResetPage }              from "pages/backup_reset_page.slint";
```

```diff
     if Bridge.active-panel == Panel.quick-actions: QuickActionsPage { }
+    if Bridge.active-panel == Panel.backup-reset:  BackupResetPage { }
 }
```

---

## Step 5 — Add `DATA` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Insert a `DATA` section. This guide places it just before `ABOUT & SUPPORT`.

### Diff

```diff
+                // ── Section: DATA ─────────────────────────────────────────
+                SettingsSection {
+                    title: "DATA";
+                    SettingsValueRow {
+                        title: "Backup & reset";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.backup-reset; }
+                    }
+                }
+
                 // ── Section: ABOUT & SUPPORT ──────────────────────────────
                 SettingsSection {
                     title: "ABOUT & SUPPORT";
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
# 1. Panel.backup-reset present.
grep -n 'backup-reset,' senders/android/ui/bridge.slint
# Expected: 1.

# 2. ConfirmDialog component present.
grep -n 'export component ConfirmDialog\|callback confirmed\|callback dismissed' \
    senders/android/ui/components/confirm_dialog.slint
# Expected: 3 matches.

# 3. ConfirmDialog has both Primary and Destructive button branches.
grep -n 'PrimaryButton\|DestructiveButton' \
    senders/android/ui/components/confirm_dialog.slint
# Expected: 2 matches (one each).

# 4. BackupResetPage uses ConfirmDialog.
grep -n 'ConfirmDialog' senders/android/ui/pages/backup_reset_page.slint
# Expected: 2 matches (import + instantiation).

# 5. main.slint routes Panel.backup-reset.
grep -n 'Panel\.backup-reset' senders/android/ui/main.slint
# Expected: 1.

# 6. DATA section in FullSettingsPage.
grep -n 'DATA\|Panel\.backup-reset' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

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
#   new file:   senders/android/ui/components/confirm_dialog.slint
#   new file:   senders/android/ui/pages/backup_reset_page.slint
git commit -m "feat(slint-ui): Phase 19 — settings backup/reset + reusable ConfirmDialog (UI-only)"
```

---

## Gotchas (Phase 19 specific)

### Gotcha 33 — Inner card TouchArea must absorb clicks

**Symptom:** tapping anywhere inside the dialog card (including on the title text or body text) dismisses the dialog instead of doing nothing.

**Cause:** Slint click events bubble up to the nearest containing `TouchArea`. Without an inner `TouchArea` on the card, clicks on the card body propagate to the outer scrim's `TouchArea` and trigger `dismissed()`.

**Fix (already in this guide's Step 2):** add an inner `TouchArea { }` (no `clicked` body) inside the card `Rectangle`. Slint stops propagation at the first matching TouchArea, even one without a callback.

If your version of Slint requires explicit handling, add a no-op clicked: `TouchArea { clicked => { } }`.

### Gotcha 34 — Stateless dialog requires consumer discipline

**Symptom:** the dialog persists after confirm because the parent didn't reset `pending-action`.

**Cause:** the dialog component does not hide itself; the consumer must clear the visibility flag.

**Fix:** in every `confirmed => { ... }` and `dismissed => { ... }` consumer-side handler, ensure the visibility flag is cleared. The page's `on-confirm()` helper in this guide does this; copying the pattern in future consumers is the right move.

### Gotcha 35 — Dialog overlay sits inside the page, not in `main.slint`

**Symptom:** future consumer puts the dialog in `main.slint` instead of inside the page that owns the state, leading to leaky abstractions.

**Cause:** the Phase 18 lifecycle overlays live in `main.slint` because they're orthogonal to all panels (lock can engage from any panel). The `ConfirmDialog` is *scoped to one panel*, so it lives inside that panel's component.

**Fix:** keep dialog instantiation **inside** the page that owns its state. If multiple pages need the same dialog, instantiate one each — there's no shared state.

### Gotcha 36 — `card-content.preferred-height` only works inside a layout

**Symptom:** `height: card-content.preferred-height;` doesn't compute correctly when the inner element isn't a layout.

**Cause:** `preferred-height` is a Slint layout-system primitive. It works on `VerticalLayout`, `HorizontalLayout`, etc. — not on raw `Rectangle`s.

**Fix (already in this guide's Step 2):** the inner element is `card-content := VerticalLayout`, which has a `preferred-height`. If you change the inner to a `Rectangle`, you'd need to compute height manually or use a hardcoded `height: 200px`.

---

## Exit criteria checklist

- [ ] `bridge.slint` adds `Panel.backup-reset` variant.
- [ ] `components/confirm_dialog.slint` exists and exports `ConfirmDialog`.
- [ ] `ConfirmDialog` accepts `title`, `body`, `confirm-label`, `dismiss-label`, `destructive` properties.
- [ ] `ConfirmDialog` exposes `confirmed()` and `dismissed()` callbacks.
- [ ] `ConfirmDialog` renders `DestructiveButton` when `destructive == true`, `PrimaryButton` otherwise.
- [ ] Tapping outside the dialog card dismisses; tapping inside the card does nothing.
- [ ] `BackupResetPage` opens from settings root.
- [ ] Export and Import rows show a transient success banner that auto-hides after 3 seconds.
- [ ] Each of the three destructive rows opens `ConfirmDialog` with appropriate title / body / confirm label.
- [ ] On confirm, dialog closes and a "Reset complete" / "History cleared" / "Receivers cleared" banner appears.
- [ ] On cancel, dialog closes with no banner.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    callback export-settings();
+    callback import-settings();
+    callback reset-settings();
+    callback clear-cast-history();
+    callback clear-known-receivers();
+    in property <string> last-banner-message;
+    in-out property <bool> banner-visible;
```

- `export-settings()` → JNI to launch `ACTION_CREATE_DOCUMENT` Storage Access Framework intent; on success, Rust pushes a banner message.
- `import-settings()` → similarly with `ACTION_OPEN_DOCUMENT`.
- `reset-settings()` / `clear-cast-history()` / `clear-known-receivers()` → Rust resets the corresponding persistence; pushes banner.
- `last-banner-message` + `banner-visible` move to Bridge so Rust can drive banner content. The page's `on-confirm()` helper becomes a switch on `pending-action` invoking the right callback.

---

## Slint-doc references used

- **Component declaration with `in property` + `callback`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` and `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Conditional element `if cond: PrimaryButton { }` / `if !cond: DestructiveButton { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`width: min(parent.width * 0.85, 480px);` (mixed-unit min)** — `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`.
- **Named layout via `name := VerticalLayout { ... }` and `name.preferred-height`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **TouchArea click absorption (inner blocks bubbling to outer)** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **String comparison in pure functions and conditional bindings** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`SettingsSection`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`PrimaryButton`, `DestructiveButton`, `TextButton`** — FCast components in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real file picker / Storage Access Framework integration.** Phase 8.
- **Real persistence read / write / clear.** Phase 8.
- **Backup format / schema versioning.** Out of scope; would land alongside the persistence layer.
- **iCloud / Google Drive cloud backup.** Out of scope.
- **Animated dialog enter/exit transitions.** Defer to polish phase; Slint supports `animate opacity` on the scrim and slide transitions on the card if desired.
- **Promote `Theme.scrim` to `theme.slint`.** Defer.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-19-settings-backup-reset.md
[p14]: ./PHASE-14-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx

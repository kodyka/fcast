# Phase 20 — Cast History reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-20-cast-history.md`][spec] to the current `senders/android` tree.
**Goal:** add `CastHistoryPage` (list of past cast sessions) + `CastHistoryDetailPage` (per-session detail). Add a `CastHistoryEntry` struct, two `Panel` variants (`cast-history`, `cast-history-detail`), and a `Bridge.selected-history-id: string` thread-through property. **First phase to reuse `ConfirmDialog`** (from Phase 19) — "Clear all" toolbar button gates a destructive confirm.
**Scope:** Slint UI only. **No Rust changes.** No real cast event log; entries come from inline mock model.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-20-cast-history.md

> **Read [`PHASE-19-reimplement-instructions.md`][p19] and [`PHASE-23-reimplement-instructions.md`][p23] first.** Phase 19 introduces `ConfirmDialog`; this guide is the first to import and reuse it. Phase 23 introduces the HH:MM:SS formatter helper used here for duration display. Phase 25's id-thread-through pattern (`Bridge.mock-macro-edit-id` → list-to-edit-page handoff) repeats here as `Bridge.selected-history-id` → list-to-detail handoff.

[p19]: ./PHASE-19-reimplement-instructions.md
[p23]: ./PHASE-23-reimplement-instructions.md

---

## Why this guide exists

Phase 20 is structurally a sibling of Phase 16 (list + edit) and Phase 25 (list + edit), but with two distinguishing elements:

1. **Status-pill severity coloring.** Each row's status (`Completed` / `Cancelled` / `Failed`) gets a coloured pill — green/amber/red. Slint has no enum-keyed style map, so the color expression is a triple-ternary, same shape as Phase 26's `level-as-int` helper.
2. **First reuse of `ConfirmDialog`.** Phase 19's component went into `components/`; Phase 20 imports it and wires `confirmed => { root.mock-history = []; }` to clear the list. The interesting bit: the consumer has to clear its own visibility state; the dialog doesn't hide itself.
3. **Read-only detail page.** No editing — purely an info display. Same chrome as the other detail pages but no Save/Cancel; just Done.

After Phases 14 + 15 + 16 + 17 + 18 + 19 + 21 + 22 + 23 + 25 + 26 merge:

- `Panel { ..., macro-edit }`. Phase 20 adds **two** variants: `cast-history`, `cast-history-detail`.
- `bridge.slint` already has the `DATA` section infrastructure (added by Phase 19). Phase 20 adds **one row** to `DATA` linking to `cast-history`.
- `ConfirmDialog` is already in `components/confirm_dialog.slint`. Phase 20 imports it.

This is **strictly additive** Slint work spread across **three existing files** plus **two new files**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'CastHistoryEntry\|CastHistoryPage\|CastHistoryDetailPage\|Panel\.cast-history\|selected-history-id\|mock-history' \
    senders/android/ui/

# DATA section already exists (added by Phase 19):
grep -n 'DATA' senders/android/ui/pages/settings_page.slint
# Expected: 1 match.

# ConfirmDialog already exists (added by Phase 19):
grep -n 'export component ConfirmDialog' senders/android/ui/components/confirm_dialog.slint
# Expected: 1 match.
```

After this guide is applied:

```sh
grep -n 'export struct CastHistoryEntry' senders/android/ui/bridge.slint
# Expected: 1 match.

grep -n 'cast-history,\|cast-history-detail,\|selected-history-id' senders/android/ui/bridge.slint
# Expected: 3 matches (2 enum + 1 property).

grep -rn 'export component CastHistoryPage\|export component CastHistoryDetailPage' \
    senders/android/ui/pages/
# Expected: 2 matches.

grep -n 'Panel\.cast-history\b\|Panel\.cast-history-detail' senders/android/ui/main.slint
# Expected: 2 matches.

grep -n 'Panel\.cast-history\b' senders/android/ui/pages/settings_page.slint
# Expected: 1 match (DATA-section opener).
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-20-cast-history
cargo check -p android-sender
```

---

## Step 1 — Add struct + panels + thread-through property in `bridge.slint`

```diff
+export struct CastHistoryEntry {
+    id:         string,
+    receiver:   string,
+    started-at: string,    // pre-formatted display string
+    duration-s: int,
+    status:     string,    // "Completed" / "Cancelled" / "Failed"
+}
+
 export struct Macro { ... }
```

```diff
 export enum Panel {
     ...
     macro-edit,
+    cast-history,
+    cast-history-detail,
 }
```

```diff
 export global Bridge {
     ...
     in-out property <string>         mock-macro-edit-id: "";
+    in-out property <string>         selected-history-id: "";
     ...
 }
```

### Why a string status field, not an enum

The spec uses a string. Slint enums would be cleaner, but stringly-typed status survives the Phase 8 promotion better — the Rust side maps `enum CastStatus { Completed, Cancelled, Failed }` to a single string field for display. Mid-flight, the UI just renders text. If you'd rather type-strictify, declare:

```slint
export enum CastStatus { completed, cancelled, failed }
```

then make `CastHistoryEntry.status: CastStatus` and use a pure `pure function status-label(s: CastStatus) -> string` helper. The string form in this guide is simpler and matches the spec; the enum form is what Phase 8 will likely want.

---

## Step 2 — Route both panels in `main.slint`

```diff
 import { MacroEditPage }                from "pages/macro_edit_page.slint";
+import { CastHistoryPage }              from "pages/cast_history_page.slint";
+import { CastHistoryDetailPage }        from "pages/cast_history_detail_page.slint";
```

```diff
     if Bridge.active-panel == Panel.macro-edit:    MacroEditPage { }
+    if Bridge.active-panel == Panel.cast-history:        CastHistoryPage { }
+    if Bridge.active-panel == Panel.cast-history-detail: CastHistoryDetailPage { }
 }
```

---

## Step 3 — Create `pages/cast_history_page.slint`

**File:** `senders/android/ui/pages/cast_history_page.slint` (new)

The list page. Each row: receiver name + status pill + started-at + duration. Trailing "Clear all" toolbar button gates `ConfirmDialog`.

### New file

```slint
// cast_history_page.slint — Past cast sessions list (UI-only).
//
// Reachable from FullSettingsPage's "Cast history" row in DATA.
// Tap a row → set Bridge.selected-history-id, open Panel.cast-history-detail.
// "Clear all" → ConfirmDialog (Phase 19) → empty mock-history on confirm.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, CastHistoryEntry } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import { ConfirmDialog } from "../components/confirm_dialog.slint";

export component CastHistoryPage inherits Rectangle {
    in-out property <[CastHistoryEntry]> mock-history: [
        { id: "h1", receiver: "Living Room TV",
          started-at: "Today 19:42",     duration-s: 5400, status: "Completed" },
        { id: "h2", receiver: "Office Display",
          started-at: "Today 11:15",     duration-s: 600,  status: "Completed" },
        { id: "h3", receiver: "Kitchen Chromecast",
          started-at: "Yesterday 22:08", duration-s: 30,   status: "Cancelled" },
        { id: "h4", receiver: "Living Room TV",
          started-at: "Yesterday 20:33", duration-s: 7200, status: "Completed" },
        { id: "h5", receiver: "Office Display",
          started-at: "Mon 09:50",       duration-s: 0,    status: "Failed"    },
    ];

    in-out property <bool> show-clear-confirm: false;

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // ── Helpers ──────────────────────────────────────────────────────────

    // HH:MM:SS formatter — same shape as Phase 23's elapsed counter helper.
    pure function format-duration(total-s: int) -> string {
        return "\{Math.floor(total-s / 3600)}:"
             + (Math.mod(Math.floor(total-s / 60), 60) < 10
                 ? "0\{Math.mod(Math.floor(total-s / 60), 60)}"
                 : "\{Math.mod(Math.floor(total-s / 60), 60)}")
             + ":"
             + (Math.mod(total-s, 60) < 10
                 ? "0\{Math.mod(total-s, 60)}"
                 : "\{Math.mod(total-s, 60)}");
    }

    // Status → pill colour. Triple ternary on string equality. The
    // colour names are illustrative; substitute Theme.* tokens once
    // the design system has dedicated success/warning/error colours.
    pure function status-color(status: string) -> color {
        return status == "Completed" ? #2e7d32  :   // green
               status == "Cancelled" ? #ed6c02  :   // amber
               status == "Failed"    ? #c62828  :   // red
                                        Theme.surface-card.brighter(20%);
    }

    VerticalLayout {
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                spacing: Theme.spacing-default;

                Text {
                    text: "Cast history";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }

                TextButton {
                    label: "Clear all";
                    clicked => { root.show-clear-confirm = true; }
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

                if root.mock-history.length == 0: Rectangle {
                    height: 80px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    Text {
                        text: "No casts yet.";
                        color: Theme.text-secondary;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }

                for entry in root.mock-history: Rectangle {
                    height: 72px;
                    border-radius: Theme.radius-card;
                    background: row-ta.pressed
                        ? Theme.surface-card.brighter(20%)
                        : Theme.surface-card;

                    row-ta := TouchArea {
                        clicked => {
                            Bridge.selected-history-id = entry.id;
                            Bridge.active-panel = Panel.cast-history-detail;
                        }
                    }

                    HorizontalLayout {
                        padding-left:  Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        spacing: Theme.spacing-default;

                        VerticalLayout {
                            alignment: center;
                            horizontal-stretch: 1;

                            HorizontalLayout {
                                spacing: 8px;
                                alignment: start;
                                Text {
                                    text: entry.receiver;
                                    color: Theme.text-primary;
                                    font-size: Theme.font-size-body;
                                    vertical-alignment: center;
                                }
                                // Status pill — fixed-size rounded rect.
                                Rectangle {
                                    width: status-text.preferred-width + 16px;
                                    height: 20px;
                                    background: root.status-color(entry.status);
                                    border-radius: 10px;
                                    status-text := Text {
                                        text: entry.status;
                                        color: white;
                                        font-size: Theme.font-size-label;
                                        horizontal-alignment: center;
                                        vertical-alignment: center;
                                    }
                                }
                            }

                            Text {
                                text: entry.started-at + "  •  "
                                    + root.format-duration(entry.duration-s);
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                        }

                        Text {
                            text: "›";
                            color: Theme.text-secondary;
                            vertical-alignment: center;
                            font-size: 20px;
                        }
                    }
                }
            }
        }
    }

    // ── Clear-all confirmation ───────────────────────────────────────────
    if root.show-clear-confirm: ConfirmDialog {
        title: "Clear all cast history?";
        body:  "This removes all past cast session entries. Cannot be undone.";
        confirm-label: "Clear";
        destructive: true;
        confirmed => {
            root.mock-history = [];
            root.show-clear-confirm = false;
        }
        dismissed => {
            root.show-clear-confirm = false;
        }
    }
}
```

### Why each piece

- **`pure function format-duration(total-s: int) -> string`** — copy-paste from Phase 23. If you'd rather DRY, extract to a shared utility module (Phase 27 territory) but Slint's import system makes this awkward — pure functions don't export across files in a clean way. Duplicating the 9-line helper across two consumers is acceptable.
- **`pure function status-color(status: string) -> color`** — Slint's `color` type is a first-class return type. Pure functions can return any built-in type. See [structs-and-enums.mdx][structs] (return types) and [colors-and-brushes.mdx][colors] for the `#xxxxxx` literals.
- **Status pill width via `status-text.preferred-width + 16px`** — the inner `Text { ... }` is named (`status-text :=`); the pill `Rectangle` reads its `preferred-width` to size flush around the label. Same pattern as Phase 19's dialog `card-content.preferred-height`. See [positioning-and-layouts.mdx][positioning].
- **`color: white;`** on the pill text — Slint accepts a few named colors (`white`, `black`, `red`, `transparent`). For everything else use `#rrggbb` or `Theme.*`.
- **`›` character** for the disclosure indicator — single Unicode U+203A. Same convention as iOS settings rows.
- **`if root.show-clear-confirm: ConfirmDialog { ... }`** — controlled-component pattern from Phase 19. The dialog's `confirmed` callback both empties the list and clears the visibility flag. Order matters: clear visibility *after* mutating, so any failed mutation leaves the dialog up. (In UI-only build, the `mock-history = []` assignment can't fail — but the discipline matters when Phase 8 reactivates and the assignment becomes a callback.)

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Create `pages/cast_history_detail_page.slint`

**File:** `senders/android/ui/pages/cast_history_detail_page.slint` (new)

Read-only detail page. Reads `Bridge.selected-history-id` to look up the entry; renders all fields plus footer button.

### New file

```slint
// cast_history_detail_page.slint — Per-cast-session detail (UI-only).
//
// Reads Bridge.selected-history-id (set by CastHistoryPage). Looks up
// the entry by id from a hardcoded duplicate of the list page's mock
// model. UI-only — no real cast event log lookup.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, CastHistoryEntry } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton, PrimaryButton } from "../components/buttons.slint";

export component CastHistoryDetailPage inherits Rectangle {
    // The detail page needs read access to the same model as the list.
    // Slint can't share a page-local property between two siblings;
    // either (a) duplicate the initialiser here, or (b) promote
    // mock-history to Bridge. The spec's "no persistence" wording
    // suggests (a) for the UI-only build. Promote to Bridge later.
    in-out property <[CastHistoryEntry]> mock-history: [
        { id: "h1", receiver: "Living Room TV",
          started-at: "Today 19:42",     duration-s: 5400, status: "Completed" },
        { id: "h2", receiver: "Office Display",
          started-at: "Today 11:15",     duration-s: 600,  status: "Completed" },
        { id: "h3", receiver: "Kitchen Chromecast",
          started-at: "Yesterday 22:08", duration-s: 30,   status: "Cancelled" },
        { id: "h4", receiver: "Living Room TV",
          started-at: "Yesterday 20:33", duration-s: 7200, status: "Completed" },
        { id: "h5", receiver: "Office Display",
          started-at: "Mon 09:50",       duration-s: 0,    status: "Failed"    },
    ];

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // ── Lookup helper. Hardcoded for the 5-row stub model — generalise
    // when promoting mock-history to Bridge.
    pure function find-entry(id: string) -> CastHistoryEntry {
        return root.mock-history[0].id == id  ? root.mock-history[0] :
               root.mock-history[1].id == id  ? root.mock-history[1] :
               root.mock-history[2].id == id  ? root.mock-history[2] :
               root.mock-history[3].id == id  ? root.mock-history[3] :
               root.mock-history[4].id == id  ? root.mock-history[4] :
                                                  { id: "", receiver: "(unknown)",
                                                    started-at: "", duration-s: 0,
                                                    status: "" };
    }

    pure function format-duration(total-s: int) -> string {
        return "\{Math.floor(total-s / 3600)}:"
             + (Math.mod(Math.floor(total-s / 60), 60) < 10
                 ? "0\{Math.mod(Math.floor(total-s / 60), 60)}"
                 : "\{Math.mod(Math.floor(total-s / 60), 60)}")
             + ":"
             + (Math.mod(total-s, 60) < 10
                 ? "0\{Math.mod(total-s, 60)}"
                 : "\{Math.mod(total-s, 60)}");
    }

    pure function status-color(status: string) -> color {
        return status == "Completed" ? #2e7d32  :
               status == "Cancelled" ? #ed6c02  :
               status == "Failed"    ? #c62828  :
                                        Theme.surface-card.brighter(20%);
    }

    // The currently-selected entry, derived reactively from Bridge property.
    property <CastHistoryEntry> entry: root.find-entry(Bridge.selected-history-id);

    VerticalLayout {
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                spacing: 8px;

                Text {
                    text: root.entry.receiver;
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }

                Rectangle {
                    width: status-pill-text.preferred-width + 16px;
                    height: 24px;
                    background: root.status-color(root.entry.status);
                    border-radius: 12px;
                    status-pill-text := Text {
                        text: root.entry.status;
                        color: white;
                        font-size: Theme.font-size-label;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                    }
                }

                TextButton {
                    label: "Done";
                    clicked => { Bridge.active-panel = Panel.cast-history; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                Rectangle {
                    height: row-content.preferred-height + 16px;
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    row-content := VerticalLayout {
                        padding: Theme.padding-screen;
                        spacing: 12px;

                        // Started.
                        HorizontalLayout {
                            Text {
                                text: "Started";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text {
                                text: root.entry.started-at;
                                color: Theme.text-primary;
                            }
                        }

                        // Duration.
                        HorizontalLayout {
                            Text {
                                text: "Duration";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text {
                                text: root.format-duration(root.entry.duration-s);
                                color: Theme.text-primary;
                            }
                        }

                        // Status (text form).
                        HorizontalLayout {
                            Text {
                                text: "Status";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text {
                                text: root.entry.status;
                                color: Theme.text-primary;
                            }
                        }

                        // Mock metrics — UI-only stubs. Phase 8 promotes to
                        // additional CastHistoryEntry fields populated by Rust.
                        HorizontalLayout {
                            Text {
                                text: "Avg bitrate";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text { text: "—"; color: Theme.text-primary; }
                        }
                        HorizontalLayout {
                            Text {
                                text: "Peak bitrate";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text { text: "—"; color: Theme.text-primary; }
                        }
                        HorizontalLayout {
                            Text {
                                text: "Dropped frames";
                                color: Theme.text-secondary;
                                horizontal-stretch: 1;
                            }
                            Text { text: "—"; color: Theme.text-primary; }
                        }
                    }
                }

                // Footer action.
                PrimaryButton {
                    label: "Cast again to " + root.entry.receiver;
                    // UI-only: no real re-cast. Phase 8 wires
                    // Bridge.invoke-action("cast-to:<receiver-id>").
                    clicked => { }
                }
            }
        }
    }
}
```

### Why each piece

- **`property <CastHistoryEntry> entry: root.find-entry(Bridge.selected-history-id);`** — initial-value binding from the Bridge property. Whenever `Bridge.selected-history-id` changes (because the list page wrote it), Slint re-evaluates the binding and `entry` updates. The page's text bindings (`root.entry.receiver`, etc.) re-render. **Caveat:** if the consumer ever writes `root.entry = X` imperatively, the binding breaks (Phase 18 §gotcha 31). Don't write to `entry` from anywhere; let the binding stay reactive.
- **`find-entry(id)` hardcoded for 5 rows** — same gotcha as Phase 25's macro lookup. Generalise once `mock-history` lives on Bridge.
- **`row-content := VerticalLayout` + `Rectangle.height: row-content.preferred-height + 16px;`** — the card grows to fit its content (6 rows of metric labels) without overflow.
- **Mock metrics rendered as `"—"` em-dash** — the spec's "from inline stub data" is loose; em-dash is the canonical "no data" affordance. Real values populate when Phase 8 promotes the struct.
- **Done button returns to `Panel.cast-history`** — back-stack invariant. Same as Phase 16/21/25.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 5 — Add row to existing `DATA` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

The `DATA` section already exists (added by Phase 19's "Backup & reset" row). Phase 20 inserts a sibling row.

### Diff

```diff
                 SettingsSection {
                     title: "DATA";
                     SettingsValueRow {
                         title: "Backup & reset";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.backup-reset; }
                     }
+                    SettingsValueRow {
+                        title: "Cast history";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.cast-history; }
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
# 1. Struct + 2 panels + Bridge property.
grep -n 'export struct CastHistoryEntry\|cast-history,\|cast-history-detail,\|selected-history-id' \
    senders/android/ui/bridge.slint
# Expected: 4 matches.

# 2. Both pages.
grep -rn 'export component CastHistoryPage\|export component CastHistoryDetailPage' \
    senders/android/ui/pages/
# Expected: 2 matches.

# 3. main.slint routes both.
grep -n 'Panel\.cast-history\b\|Panel\.cast-history-detail' senders/android/ui/main.slint
# Expected: 2 matches.

# 4. ConfirmDialog imported and used in list page.
grep -n 'ConfirmDialog' senders/android/ui/pages/cast_history_page.slint
# Expected: 2 matches (import + instantiation).

# 5. status-color + format-duration helpers exist on both pages.
grep -n 'pure function status-color\|pure function format-duration' \
    senders/android/ui/pages/cast_history_page.slint \
    senders/android/ui/pages/cast_history_detail_page.slint
# Expected: 4 matches.

# 6. DATA section has the new Cast history row.
grep -n 'Cast history\|Panel\.cast-history\b' senders/android/ui/pages/settings_page.slint
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
#   new file:   senders/android/ui/pages/cast_history_page.slint
#   new file:   senders/android/ui/pages/cast_history_detail_page.slint
git commit -m "feat(slint-ui): Phase 20 — cast session history list + detail (UI-only)"
```

---

## Gotchas (Phase 20 specific)

### Gotcha 41 — Two pages with their own copies of the mock model drift apart

**Symptom:** clearing history on the list page doesn't clear it on the detail page (or future re-list); the two model copies are independent.

**Cause:** each page declares its own `in-out property <[CastHistoryEntry]> mock-history: [...]` with the same initialiser. The detail page only reads, but if a future feature lets the detail page mutate (e.g. "delete this entry"), the two copies will diverge.

**Fix:** either (a) accept the duplication in UI-only build (the detail page is read-only), or (b) promote `mock-history` to `Bridge.mock-history`. Option (b) is closer to where Phase 8 needs to land:

```diff
 export global Bridge {
     ...
+    in-out property <[CastHistoryEntry]> mock-history: [...];
 }
```

Then both pages bind to `Bridge.mock-history` directly. The list page's "Clear all" handler becomes `Bridge.mock-history = [];` — automatically reflected on detail.

### Gotcha 42 — Status pill width must use `preferred-width + padding`

**Symptom:** the status pill renders too narrow, clipping text; or too wide, looking floppy.

**Cause:** `Rectangle` has no intrinsic size; without an explicit `width`, it defaults to filling its layout slot. The pill needs to size flush around its label, which means measuring the label's intrinsic width.

**Fix (already in this guide):** name the inner `Text { ... }` (`status-text :=`) and bind `Rectangle.width: status-text.preferred-width + 16px;`. The `+ 16px` is internal padding (8px on each side). Adjust to taste.

### Gotcha 43 — `color: white` is one of the few named colors Slint accepts

**Symptom:** `color: lightgreen;` fails with "unknown color".

**Cause:** Slint's color name set is limited compared to CSS. Per [colors-and-brushes.mdx][colors], the named set is small (`white`, `black`, `red`, `transparent`, plus a handful of basics). For anything else, use `#rrggbb` or `#rrggbbaa`.

**Fix:** stick to hex literals (or `Theme.*` tokens that resolve to hex internally). `#2e7d32` (green), `#ed6c02` (amber), `#c62828` (red) match Material Design 3 system colors.

### Gotcha 44 — `Bridge.selected-history-id` is the only handoff between list and detail

**Symptom:** detail page renders the wrong entry because the list page didn't set `selected-history-id` before flipping `active-panel`.

**Cause:** the list page's `clicked => { ... }` handler must set `Bridge.selected-history-id = entry.id;` *before* `Bridge.active-panel = Panel.cast-history-detail;` — otherwise the detail page instantiates with a stale id.

**Fix (already in this guide):** the order is correct in the snippet. If you reorder, the detail page's `find-entry` reads the previous id. Slint property writes are synchronous within a callback, so as long as both writes happen in the same handler body, the order is what determines correctness.

### Gotcha 45 — Reusing `ConfirmDialog` requires the consumer to import it

**Symptom:** Slint compiler error `unknown component 'ConfirmDialog'` even though the component exists.

**Cause:** Slint imports are file-scoped. Phase 19's `components/confirm_dialog.slint` doesn't auto-export to consumers; the consumer must declare the import explicitly.

**Fix (already in this guide's Step 3):** `import { ConfirmDialog } from "../components/confirm_dialog.slint";`. Same pattern as importing `SettingsSection` from `settings_rows.slint`.

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `CastHistoryEntry` struct.
- [ ] `bridge.slint` extends `Panel` with `cast-history` and `cast-history-detail`.
- [ ] `bridge.slint` adds `selected-history-id: string` thread-through property.
- [ ] `main.slint` routes both panels.
- [ ] `CastHistoryPage` lists 5 stub entries with status pill (green/amber/red).
- [ ] Tap row → sets `Bridge.selected-history-id` + opens detail.
- [ ] Empty state appears when `mock-history` is `[]`.
- [ ] "Clear all" toolbar button opens `ConfirmDialog` from Phase 19.
- [ ] Confirm clears `mock-history`; Cancel dismisses without changes.
- [ ] `CastHistoryDetailPage` renders the selected entry's receiver, status, started-at, duration (HH:MM:SS).
- [ ] "Cast again to <receiver>" footer button is present (no-op).
- [ ] Done returns to `Panel.cast-history` (not `Panel.none`).
- [ ] `FullSettingsPage` `DATA` section has a new "Cast history" row.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <[CastHistoryEntry]> history;
+    callback clear-history();
+    callback delete-history-entry(string);   // by id
+    callback recast(string);                  // by id — re-engages cast to that receiver
```

- `Bridge.history` ← Rust holds canonical event log; persisted to local storage.
- `clear-history()` → Rust empties the log; pushes empty list back.
- `delete-history-entry(id)` → Rust removes one; pushes new list.
- `recast(id)` → Rust resolves the receiver and starts a cast; the UI footer button calls this.
- Drop the page-local `mock-history` initialisers from both pages; bind `for entry in Bridge.history:` directly.
- Drop the `find-entry` helper from the detail page; instead, make Bridge expose a `selected-history-entry: CastHistoryEntry` derived property updated by Rust whenever `selected-history-id` changes.
- Status string can become an enum (`CastStatus`) — Rust maps to display string via Bridge function.
- Mock metrics (`Avg bitrate`, `Peak bitrate`, `Dropped frames`) become real `CastHistoryEntry` fields populated by Rust.

---

## Slint-doc references used

- **`export struct CastHistoryEntry`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`pure function status-color(s: string) -> color`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Color literals `#rrggbb`, named colors, `color` type** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **`Math.floor`, `Math.mod`** — `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`.
- **String interpolation `"\{n}"` with int** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`for entry in array:`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Conditional element `if cond: Component { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`name := Text { ... }` + `name.preferred-width`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **Property binding from `Bridge.selected-history-id`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`ConfirmDialog`** — FCast component in `senders/android/ui/components/confirm_dialog.slint` (added by Phase 19).
- **`SettingsSection`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`TextButton`, `PrimaryButton`** — FCast components in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real cast event log from Rust.** Phase 8.
- **Persistence (clearing the list resets on reload).** Phase 8.
- **Statistics aggregation** ("most-cast receiver this week"). Out of scope.
- **Export history as CSV / JSON.** Out of scope.
- **`ListView` virtualisation.** The 5-row stub is small; a regular `for` inside `ScrollView` is fine. If/when Rust pushes hundreds of entries, swap to `ListView` (same as Phase 26's debug log page) — the migration is local, no struct changes.
- **Filter chips by status.** Out of scope; the dataset is small. If/when promoted to Bridge with hundreds of entries, copy Phase 26's filter-chip pattern.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-20-cast-history.md
[p19]: ./PHASE-19-reimplement-instructions.md
[p23]: ./PHASE-23-reimplement-instructions.md
[colors]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx

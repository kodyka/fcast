# Phase 26 — Debug Log Viewer reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-26-debug-log-viewer.md`][spec] to the current `senders/android` tree.
**Goal:** add `DebugLogPage` (virtualised log scroller with level-filter chips) and `DebugVideoPage` (pipeline overlay toggles + element-state read-only table). Wired into the `Panel` overlay layer; replace the existing `Show debug panel` toggle in `FullSettingsPage`'s `CODEC & DEBUG` section with two `Open` rows.
**Scope:** Slint UI only. **No Rust changes.** No `tracing` capture; no GStreamer introspection. Log entries are inline mock data; pipeline element list is hardcoded.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-26-debug-log-viewer.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] first.** The new things in Phase 26 are: `ListView` instead of `ScrollView` (for virtualisation), index comparison on enums (`level-as-int(entry.level) >= mock-min-level-idx`), monospace text styling, and the **deletion of an existing toggle** in `FullSettingsPage`.

[p14]: ./PHASE-14-reimplement-instructions.md

---

## Why this guide exists

Phase 26 is the third `ADVANCED` sub-page (after Phase 22 `Network` + Phase 23 `Recording`). It's the most pattern-heavy of Wave 2 because it adds:

1. **`ListView` virtualisation.** The spec is explicit: a 200-entry log inside a `ScrollView` will scroll smoothly only if Slint actually virtualises the children. `ScrollView` instantiates *all* children regardless of viewport; `ListView` is the virtualising widget. Use it when the model can grow beyond ~50 rows.
2. **`LogLevel` enum compared by ordinal.** Slint enums don't have `as int` casts directly. The standard workaround is a small `pure function level-as-int(level: LogLevel) -> int { ... }` mapping each variant to a position in the severity hierarchy. The filter chip bar then sets `mock-min-level-idx: int` and the visibility filter is `level-as-int(entry.level) >= mock-min-level-idx`.
3. **Replacing an existing toggle.** Phase 7 baseline has `SettingsToggleRow { title: "Show debug panel"; checked: root.debug-panel; toggled(checked) => { root.debug-panel = checked; } }` in `CODEC & DEBUG`. Phase 26 either removes it (recommended) or keeps it for backward compat (per spec). This guide chooses **remove**, because the new pages cover the use case more precisely.
4. **Two new pages, both linked from the same section.** `Debug log` opens `Panel.debug-log`; `Video pipeline` opens `Panel.debug-video`.

After Phases 14 + 15 + 16 + 21 + 22 + 23 merge:

- `Panel { ..., recording }`. Phase 26 adds **two**: `debug-log`, `debug-video`.
- `FullSettingsPage`'s `CODEC & DEBUG` section has 2 entries (`H.264 encoder test` + `Show debug panel` toggle). Phase 26 replaces the toggle with two `Open` rows; the codec test row is unchanged.
- `bridge.slint` exports `BitratePreset`, `NetworkInterface` structs and `RecordingState` enum. Phase 26 adds `LogLevel` enum + `LogEntry` struct.

This is **strictly additive** (apart from the toggle removal) Slint work spread across **three existing files** plus **two new files**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'DebugLogPage\|DebugVideoPage\|LogLevel\|LogEntry\|mock-log\|mock-min-level-idx' \
    senders/android/ui/

# The toggle to be replaced:
grep -n 'Show debug panel\|debug-panel' senders/android/ui/pages/settings_page.slint
# Expected: 4 matches (property decl, decl init, checked binding, toggled handler).
```

After this guide is applied:

```sh
grep -n 'export enum LogLevel\|export struct LogEntry' senders/android/ui/bridge.slint
# Expected: 2 matches.

grep -n 'debug-log\|debug-video' senders/android/ui/bridge.slint
# Expected: 2 matches (Panel variants).

grep -rn 'export component DebugLogPage\|export component DebugVideoPage' senders/android/ui/
# Expected: 2 matches.

grep -n 'ListView' senders/android/ui/pages/debug_log_page.slint
# Expected: 1 match.

# Old toggle removed, two new rows in place.
grep -n 'Show debug panel\|debug-panel\|Panel\.debug-log\|Panel\.debug-video' \
    senders/android/ui/pages/settings_page.slint
# Expected: 2 matches (Panel.debug-log + Panel.debug-video clicked handlers); Show debug panel + debug-panel gone.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-26-debug-pages
cargo check -p android-sender
```

---

## Step 1 — Add `LogLevel` enum + `LogEntry` struct + 2 `Panel` variants in `bridge.slint`

```diff
+export enum LogLevel {
+    trace,
+    debug,
+    info,
+    warning,
+    error,
+}
+
+export struct LogEntry {
+    level:     LogLevel,
+    timestamp: string,
+    target:    string,
+    message:   string,
+}
+
 export struct NetworkInterface { ... }
```

```diff
 export enum Panel {
     ...
     recording,
+    debug-log,
+    debug-video,
 }
```

Variant order in `LogLevel` matches conventional severity ordering (trace → error). The integer index that `level-as-int(...)` returns will use the same order.

---

## Step 2 — Route both panels in `main.slint`

```diff
 import { RecordingPage }                from "pages/recording_page.slint";
+import { DebugLogPage }                 from "pages/debug_log_page.slint";
+import { DebugVideoPage }               from "pages/debug_video_page.slint";
```

```diff
     if Bridge.active-panel == Panel.recording:       RecordingPage { }
+    if Bridge.active-panel == Panel.debug-log:       DebugLogPage { }
+    if Bridge.active-panel == Panel.debug-video:     DebugVideoPage { }
 }
```

---

## Step 3 — Create `pages/debug_log_page.slint`

**File:** `senders/android/ui/pages/debug_log_page.slint` (new)

### New file

```slint
// debug_log_page.slint — Virtualised log scroller with level filtering (UI-only).
//
// Reachable from FullSettingsPage's "Debug log" row in CODEC & DEBUG.
// Uses ListView (NOT ScrollView) so a model of 10k+ entries renders
// smoothly. Filter chips set a minimum severity level. Phase 8 swaps
// mock-log for a Bridge-published [LogEntry] tailing real tracing
// events.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx (font-family)

import { ListView } from "std-widgets.slint";
import { Bridge, Panel, LogLevel, LogEntry } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";

// Filter chip — local state lives on the page root (mock-min-level-idx),
// so the chip itself is stateless and only emits clicked.
component FilterChip inherits Rectangle {
    in property <string> label;
    in property <bool>   active: false;
    callback clicked();

    height: 28px;
    width: max(48px, t.preferred-width + 16px);
    border-radius: 14px;
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

export component DebugLogPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <[LogEntry]> mock-log: [
        { level: LogLevel.info,    timestamp: "12:34:56.012",
          target: "fcast::discovery", message: "mDNS scan started" },
        { level: LogLevel.debug,   timestamp: "12:34:56.087",
          target: "fcast::discovery", message: "Resolved Living Room TV at 192.168.1.50" },
        { level: LogLevel.warning, timestamp: "12:34:56.220",
          target: "fcast::net",       message: "Reconnect attempt 1/3" },
        { level: LogLevel.error,   timestamp: "12:34:56.510",
          target: "fcast::encoder",   message: "Encoder negotiation failed: H264 not advertised" },
        { level: LogLevel.trace,   timestamp: "12:34:56.612",
          target: "slint",            message: "Layout pass 423 (12ms)" },
    ];
    in-out property <int> mock-min-level-idx: 1; // 0=trace .. 4=error

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // Slint enums don't have a built-in cast to int. Map each variant
    // explicitly so the filter chip-index comparison works.
    pure function level-as-int(level: LogLevel) -> int {
        return level == LogLevel.trace   ? 0 :
               level == LogLevel.debug   ? 1 :
               level == LogLevel.info    ? 2 :
               level == LogLevel.warning ? 3 :
                                            4;   // error
    }

    pure function level-color(level: LogLevel) -> color {
        return level == LogLevel.trace   ? #888888 :
               level == LogLevel.debug   ? #4080ff :
               level == LogLevel.info    ? #20a020 :
               level == LogLevel.warning ? #f0a020 :
                                            #e02020;   // error
    }

    pure function level-name(level: LogLevel) -> string {
        return level == LogLevel.trace   ? "TRACE" :
               level == LogLevel.debug   ? "DEBUG" :
               level == LogLevel.info    ? "INFO" :
               level == LogLevel.warning ? "WARN" :
                                            "ERROR";
    }

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Debug log";
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

        // ── Filter chip bar ─────────────────────────────────────────────
        Rectangle {
            height: 48px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding-left:  Theme.padding-screen;
                padding-right: Theme.padding-screen;
                spacing: 6px;
                alignment: start;

                for label[i] in ["Trace", "Debug", "Info", "Warning", "Error"]: FilterChip {
                    label: label;
                    active: root.mock-min-level-idx == i;
                    clicked => { root.mock-min-level-idx = i; }
                }
            }
        }

        // ── Body — virtualised log list ─────────────────────────────────
        ListView {
            for entry in root.mock-log: Rectangle {
                // Filter rule: rows below min level collapse to zero
                // height. ListView still allocates a slot but the visible
                // height is 0 — the filter is logically O(N) but the
                // visual cost is virtualised.
                height: root.level-as-int(entry.level) >= root.mock-min-level-idx
                    ? 56px
                    : 0px;
                clip: true;

                HorizontalLayout {
                    // 4-px wide level color stripe.
                    Rectangle {
                        width: 4px;
                        background: root.level-color(entry.level);
                    }

                    VerticalLayout {
                        padding-left:  8px;
                        padding-right: Theme.padding-screen;
                        padding-top:    6px;
                        padding-bottom: 6px;
                        spacing: 2px;

                        // Top line: timestamp (mono) + level name + target.
                        HorizontalLayout {
                            spacing: 8px;
                            Text {
                                text: entry.timestamp;
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                                font-family: "monospace";
                            }
                            Text {
                                text: root.level-name(entry.level);
                                color: root.level-color(entry.level);
                                font-size: Theme.font-size-label;
                                font-family: "monospace";
                            }
                            Text {
                                text: entry.target;
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                                font-family: "monospace";
                                horizontal-stretch: 1;
                                overflow: elide;
                            }
                        }

                        // Bottom line: message, full width, word-wrapped.
                        Text {
                            text: entry.message;
                            color: Theme.text-primary;
                            font-size: Theme.font-size-label;
                            font-family: "monospace";
                            wrap: word-wrap;
                        }
                    }
                }
            }
        }

        // ── Bottom toolbar ──────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                spacing: Theme.spacing-default;
                alignment: end;
                TextButton {
                    label: "Clear";
                    // No-op in UI-only build — Phase 8 will clear the
                    // Rust-side ring buffer.
                    clicked => { root.mock-log = []; }
                }
                TextButton {
                    label: "Copy all";
                    clicked => { }   // no-op — Phase 8 wires Bridge.copy-log()
                }
            }
        }
    }
}
```

### Why each piece

- **`ListView` instead of `ScrollView`.** [`ListView`][listview] is the virtualising container — it instantiates row components only for the visible viewport plus a small overscan. With a 10k-entry mock log, this matters. `ScrollView` would instantiate all 10k rows.
- **Filter rule is `height: cond ? 56px : 0px;` with `clip: true;`.** Slint's `ListView` does not have a built-in filter / predicate. The pragma is to render every row but collapse below-min-level rows to zero height. With `clip: true;` the row is invisible. ListView still allocates the slot, but the layout cost is constant per row regardless of model size. For very long logs (100k+), this approach saturates because you walk the entire model — at that scale, lift filtering to Rust (Phase 8).
- **`pure function level-as-int(level: LogLevel) -> int`** — the canonical workaround for Slint's lack of enum-to-int casting. The compiler unrolls the ternary chain to a switch table. See [functions-and-callbacks.mdx][callbacks] for `pure function` syntax.
- **Three `pure function` helpers (`level-as-int`, `level-color`, `level-name`)** rather than one big switch — keeps each helper's signature focused. They're called from inside the `for` loop body (`root.level-as-int(entry.level)`); the qualified `root.` prefix is required to call a component-local pure function from inside a nested element scope.
- **`font-family: "monospace"`** — generic font name, no asset bundling required. [`Text`][text] doc lists `monospace` (and `serif`, `sans-serif`) as platform-mapped generic families.
- **`for label[i] in ["Trace", ...]: FilterChip { active: root.mock-min-level-idx == i; }`** — index `i` from the `for` loop (Slint exposes `[index]` on the iterator) gives us the chip's ordinal. Comparing it to `mock-min-level-idx` highlights the correct chip.
- **`Clear` button does `root.mock-log = []`** — empty array literal. ListView re-renders empty. Phase 8 will replace this with `Bridge.clear-log()`.
- **Inner glyph layout uses three `Text` elements stacked horizontally** — timestamp, level name, target. The level-name `color: root.level-color(entry.level);` is the only place the severity color appears in the text itself; the 4px stripe handles the rest.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Create `pages/debug_video_page.slint`

**File:** `senders/android/ui/pages/debug_video_page.slint` (new)

### New file

```slint
// debug_video_page.slint — Pipeline overlay toggles + element state table (UI-only).
//
// Reachable from FullSettingsPage's "Video pipeline" row in CODEC & DEBUG.
// Pipeline element list is hardcoded — Phase 8 will publish real
// gst_element_iterate_recurse() output via Bridge.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsToggleRow,
} from "../components/settings_rows.slint";

export component DebugVideoPage inherits Rectangle {
    in-out property <bool> mock-show-element-graph:    false;
    in-out property <bool> mock-show-buffer-timestamps: false;
    in-out property <bool> mock-show-negotiated-caps:   false;
    in-out property <bool> mock-show-keyframe-markers:  false;

    // Static read-only element state list — a struct-typed array literal
    // works here because the size is fixed and the data is stub.
    property <[{element: string, state: string}]> mock-element-states: [
        { element: "src",          state: "PLAYING" },
        { element: "videoconvert", state: "PLAYING" },
        { element: "x264enc",      state: "PLAYING" },
        { element: "rtph264pay",   state: "PLAYING" },
        { element: "udpsink",      state: "PLAYING" },
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
                    text: "Video pipeline";
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

                SettingsSection {
                    title: "PIPELINE OVERLAY";
                    SettingsToggleRow {
                        title: "Show element graph";
                        checked: root.mock-show-element-graph;
                        toggled(checked) => { root.mock-show-element-graph = checked; }
                    }
                    SettingsToggleRow {
                        title: "Show buffer timestamps";
                        checked: root.mock-show-buffer-timestamps;
                        toggled(checked) => { root.mock-show-buffer-timestamps = checked; }
                    }
                    SettingsToggleRow {
                        title: "Show negotiated caps";
                        checked: root.mock-show-negotiated-caps;
                        toggled(checked) => { root.mock-show-negotiated-caps = checked; }
                    }
                    SettingsToggleRow {
                        title: "Show keyframe markers";
                        checked: root.mock-show-keyframe-markers;
                        toggled(checked) => { root.mock-show-keyframe-markers = checked; }
                    }
                }

                SettingsSection { title: "PIPELINE STATE"; }
                Rectangle {
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    VerticalLayout {
                        padding: Theme.padding-screen;
                        spacing: 4px;
                        for state in root.mock-element-states: HorizontalLayout {
                            Text {
                                text: state.element;
                                color: Theme.text-primary;
                                font-size: Theme.font-size-body;
                                font-family: "monospace";
                                horizontal-stretch: 1;
                            }
                            Text {
                                text: state.state;
                                color: Theme.accent-active;
                                font-size: Theme.font-size-body;
                                font-family: "monospace";
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

- **No struct-in-bridge.** The `[{element: string, state: string}]` is an inline anonymous-struct array; same pattern as Phase 21's `mock-versions`. The data is purely stub for this UI-only build, so a named `PipelineElement` struct in `bridge.slint` would be premature commitment.
- **`ScrollView` not `ListView`.** The element list is short (5 entries, fixed). `ListView` virtualisation buys nothing here.
- **Four toggles** match the spec's overlay options. Same `toggled(checked) => ... = checked;` pattern as Phase 14.
- **PIPELINE STATE section uses a manual `Rectangle + VerticalLayout`** rather than `SettingsSection` children, because the rows are 2-column (element name + state) and don't fit the `SettingsValueRow` shape.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 5 — Replace `Show debug panel` toggle with two `Open` rows in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Delete the `debug-panel` property and the toggle row; add two `SettingsValueRow`s opening the new panels.

### Diff

```diff
-    in-out property <bool>   debug-panel:    false;
-
     ...

                 // ── Section: CODEC & DEBUG ────────────────────────────────
                 SettingsSection {
                     title: "CODEC & DEBUG";
                     SettingsValueRow {
                         title: "H.264 encoder test";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.codec-test; }
                     }
-                    SettingsToggleRow {
-                        title: "Show debug panel";
-                        checked: root.debug-panel;
-                        toggled(checked) => { root.debug-panel = checked; }
-                    }
+                    SettingsValueRow {
+                        title: "Debug log";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.debug-log; }
+                    }
+                    SettingsValueRow {
+                        title: "Video pipeline";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.debug-video; }
+                    }
                 }
```

### Why

The Phase 7 `Show debug panel` toggle was a placeholder for the long-term debug surface. Now that real debug pages exist, the toggle is redundant. The spec gives an option to keep it for backward compatibility; this guide chooses removal because:
- The new pages cover the use case more precisely.
- Keeping a redundant toggle creates mental load (which one is the "real" debug surface?).
- `debug-panel` is not consumed elsewhere in the chassis (verified by `grep -rn 'debug-panel' senders/android/ui/`).

If you find a downstream consumer of `debug-panel` you weren't expecting (e.g. a sub-component conditionally rendering on it), keep the toggle.

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. LogLevel enum + LogEntry struct + 2 Panel variants in bridge.slint.
grep -n 'export enum LogLevel\|export struct LogEntry\|debug-log,\|debug-video,' \
    senders/android/ui/bridge.slint
# Expected: 4 matches.

# 2. Both pages exported.
grep -rn 'export component DebugLogPage\|export component DebugVideoPage' senders/android/ui/

# 3. main.slint routes both.
grep -n 'Panel\.debug-log\|Panel\.debug-video' senders/android/ui/main.slint
# Expected: 2 matches.

# 4. DebugLogPage uses ListView (virtualisation).
grep -n 'ListView' senders/android/ui/pages/debug_log_page.slint
# Expected: 1 match.

# 5. Old debug-panel property + Show debug panel toggle removed.
grep -n 'debug-panel\|Show debug panel' senders/android/ui/pages/settings_page.slint
# Expected: (empty)

# 6. New rows in CODEC & DEBUG.
grep -n 'Panel\.debug-log\|Panel\.debug-video' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches.

# 7. level-as-int helper covers all 5 levels.
grep -c 'LogLevel\.' senders/android/ui/pages/debug_log_page.slint
# Expected: 15 matches (5 levels × 3 helper functions = 15).

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
#   new file:   senders/android/ui/pages/debug_log_page.slint
#   new file:   senders/android/ui/pages/debug_video_page.slint
git commit -m "feat(slint-ui): Phase 26 — debug log viewer + video pipeline state (UI-only)"
```

---

## Gotchas (Phase 26 specific)

### Gotcha 20 — `ListView` requires direct children to be the row element, not a wrapper

**Symptom:** `ListView { Rectangle { for entry in mock-log: Rectangle { ... } } }` either fails to compile or virtualises incorrectly (allocates only one slot for the entire wrapper Rectangle).

**Cause:** [`ListView`][listview] expects the `for` loop to be its **direct** child (or a `Layout` containing the `for`). The virtualiser inspects the immediate-child structure to infer row instantiation.

**Fix:** put the `for entry in ...:` directly inside `ListView`, not wrapped:

```slint
ListView {
    for entry in root.mock-log: Rectangle { ... }   // ✅
}
```

Not:

```slint
ListView {
    Rectangle {
        for entry in root.mock-log: Rectangle { ... }   // ❌
    }
}
```

### Gotcha 21 — Enum-to-int conversion has no built-in

**Symptom:** Slint compiler error `cannot convert LogLevel to int` when writing `level-as-int(entry.level) >= mock-min-level-idx`.

**Cause:** Slint enums are nominal types with no implicit ordinal. A built-in cast like `entry.level as int` does not exist.

**Fix:** explicit ternary chain in a `pure function`. Same pattern is used by GTK GtkExpression port and other C-flavored UI DSLs that don't expose enum ordinals.

### Gotcha 22 — `font-family: "monospace"` requires the platform to ship a monospace font

**Symptom:** on some Android skins, the log appears in proportional font even with `font-family: "monospace"`.

**Cause:** Slint maps `"monospace"` / `"sans-serif"` / `"serif"` to platform generic families. On platforms missing the family, fallback varies.

**Fix (defer to polish phase):** ship a bundled mono font (e.g. JetBrains Mono Regular) in `senders/android/ui/fonts/` and reference it by exact name. Phase-26 UI-only build is fine with the platform-generic name; document the fallback caveat in the page header comment.

### Gotcha 23 — `clip: true` on the row is required for the height-zero filter

**Symptom:** filtered-out rows still bleed visible content — text leaks above/below the next row.

**Cause:** `height: 0px` does not automatically clip child elements. Without `clip: true`, the inner `Text` overflows the row's bounding rectangle.

**Fix (already in the snippet):** add `clip: true;` to the `Rectangle` whose height is dynamic. See [rectangle.mdx][rect].

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `LogLevel` enum (5 variants), `LogEntry` struct, `Panel.debug-log` + `Panel.debug-video` variants.
- [ ] `main.slint` routes both panels.
- [ ] `DebugLogPage` renders 5 stub entries with level color stripes, monospace timestamps + targets + messages.
- [ ] Filter chip bar has 5 chips (Trace / Debug / Info / Warning / Error). Tapping a chip highlights it and hides rows whose level ordinal is below the chip's.
- [ ] List uses `ListView` (verify by inflating `mock-log` to 200 entries — scroll stays smooth).
- [ ] `Clear` button empties the log; `Copy all` is a no-op.
- [ ] `DebugVideoPage` renders 4 toggles (`PIPELINE OVERLAY` section) + 5-row read-only element-state table (`PIPELINE STATE` section).
- [ ] `FullSettingsPage`'s `CODEC & DEBUG` section has 3 entries: `H.264 encoder test`, `Debug log`, `Video pipeline`. The old `Show debug panel` toggle is gone; the `debug-panel` property is gone.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <[LogEntry]> log-events;
+    in property <int>         pipeline-clock-rate-ns;
+    in property <[{element: string, state: string}]> pipeline-elements;
+
+    callback clear-log();
+    callback copy-log();
+    callback set-min-log-level(int);
+    callback set-overlay(string, bool);
```

- `log-events` ← Rust `tracing-subscriber` layer pushes events; `slint::Weak<DebugLogPage>` updates the model. Cap at e.g. 5000 entries (ring buffer).
- `pipeline-elements` ← Rust polls `gst_element_iterate_recurse(pipeline)` periodically; pushes the (name, state) pairs.
- `clear-log()` → Rust drops the ring buffer.
- `copy-log()` → JNI to `ClipboardManager.setPrimaryClip(ClipData.newPlainText(...))`.
- `set-overlay("element-graph", bool)` → Rust enables/disables a GStreamer `dot dump` periodic timer or a custom `videofilter` that draws timestamps onto frames.

---

## Slint-doc references used

- **`export enum LogLevel` and `export struct LogEntry`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`ListView` virtualisation** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx`.
- **`for label[i] in [...]: FilterChip { ... }` indexed for-loop** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **`pure function name(arg) -> ret { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Nested ternary in pure-function bodies** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`Text.font-family`, `Text.font-size`, `Text.color`, `Text.wrap`, `Text.overflow`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`.
- **`Rectangle.clip: true`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **Conditional element `if Bridge.active-panel == Panel.debug-log: DebugLogPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`SettingsSection`, `SettingsToggleRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`TextButton`** — FCast component in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Live `tracing` event capture.** Phase 8 + a `tracing-subscriber` layer.
- **Log file export / sharing.** Phase 8.
- **Real GStreamer pipeline introspection.** Phase 8.
- **Search / regex filtering of log messages.** Out of scope; would require a `LineEdit` filter input + Rust-side regex match.
- **Per-row "Copy this entry" affordance.** Out of scope.
- **Severity-based row background tinting.** Defer to polish phase; the 4-px color stripe is sufficient.
- **Animated scroll-to-bottom on new log entry.** Slint `ListView` does not currently expose programmatic scroll; defer.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-26-debug-log-viewer.md
[p14]: ./PHASE-14-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[callbacks]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
[listview]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx
[text]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx
[rect]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx

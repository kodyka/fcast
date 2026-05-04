# Phase 7 — Settings Navigation & FCast-Specific Pages reimplementation guide (UI-only)

**Audience:** developer applying the **updated** [`draft/slint-ui/phases/PHASE-7-settings-pages.md`][spec] to the current `senders/android` tree.
**Goal:** stand up a real `FullSettingsPage` (header + 4 sections) and a panel-routing chassis (`Panel` enum + `active-panel` property) so the user can open/close Settings / Debug / CodecTest panels from Slint without any Rust callbacks.
**Scope:** Slint UI only. **No Rust changes.** Existing `AppState.SelectingSettings` / `SettingsPageView` / `Bridge.show-debug` / `Bridge.quick-actions` / `Bridge.invoke-action(...)` stay intact — Phase 7 builds the new panel layer alongside them.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-7-settings-pages.md

---

## Why this guide exists

Audit of the current `migrate` branch shows the settings surface is on its **Phase 1** shape:

- `bridge.slint` has **no `Panel` enum** and **no `active-panel` property**. There is a legacy `Bridge.show-debug: bool` which gates a debug section inside `ConnectView`.
- `main.slint` routes only the five `AppState` views inside a single `Rectangle { vertical-stretch: 1 }`. There is **no panel overlay layer** above the page stack.
- `pages/settings_page.slint` exports only `SettingsPageView` (the **pre-cast quality picker** for `AppState.SelectingSettings`). There is **no `FullSettingsPage`** yet — but the file's header comment already anticipates it: *"Renamed from SelectingSettingsView → SettingsPageView to avoid collision with the full FullSettingsPage component added in Phase 7."*
- `pages/codec_test_page.slint` does not exist.
- `components/control_bar.slint` reads from `Bridge.quick-actions` (Rust-populated) and dispatches every click through `Bridge.invoke-action(id)`. There is **no inline `mock-quick-actions` model** of the kind PHASE-7 task 7-C presupposes — that's a deviation between Phase 4's spec and Phase 4's actual implementation (PR #5 went all-in on Rust-driven dispatch). This guide picks one path through the deviation and documents the alternative.
- `components/settings_rows.slint` already exports `SettingsTextRow`, `SettingsValueRow` (with `clicked()` callback + `show-chevron: bool`), `SettingsToggleRow` (with `toggled(bool)` callback), `SettingsSliderRow`, and `SettingsSection` (`@children`-based wrapper). All five use `title:` (not `label:`) — that property name was nailed down by PR #8.
- `components/buttons.slint` already exports `TextButton`, `PrimaryButton`, `DestructiveButton`, `LoadingView`. No changes needed.

So this is **strictly additive** Slint work spread across **five files** (one Bridge edit, two new components, two existing-file rewrites). Rust does not need to be touched.

### Audit before you start

Run these from the repo root to confirm the pre-state:

```sh
# Should be ZERO matches before you start:
grep -rn 'Panel\b\|active-panel\|FullSettingsPage\|CodecTestPage' \
    senders/android/ui/

# Settings page currently exports only the quality picker, not FullSettingsPage:
grep -n 'export component' senders/android/ui/pages/settings_page.slint
# Expected:
#   22:export component SettingsPageView inherits Rectangle {

# Control bar still dispatches everything through Rust:
grep -n 'Bridge\.quick-actions\|Bridge\.invoke-action' senders/android/ui/components/control_bar.slint
# Expected:
#   47:        for action in Bridge.quick-actions: QuickActionButton {
#   49:            invoked(id) => { Bridge.invoke-action(id); }

# All 5 row components exist with `title:` (post-PR-#8):
grep -n 'export component\|in property <string> title' senders/android/ui/components/settings_rows.slint
```

After this guide is applied:

```sh
# `Panel` enum + `active-panel` in bridge.slint
grep -n 'Panel\|active-panel' senders/android/ui/bridge.slint
# Expected (counts vary):
#   line: export enum Panel { ... }
#   line: in-out property <Panel> active-panel: Panel.none;

# main.slint has the overlay layer
grep -n 'active-panel\|Panel\.' senders/android/ui/main.slint
# Expected ~6 matches (1 import, 3 conditional blocks)

# FullSettingsPage and CodecTestPage exist
grep -rn 'export component FullSettingsPage\|export component CodecTestPage' \
    senders/android/ui/
# Expected:
#   senders/android/ui/pages/settings_page.slint:NN:export component FullSettingsPage inherits Rectangle {
#   senders/android/ui/pages/codec_test_page.slint:NN:export component CodecTestPage inherits Rectangle {

# Control bar opens panels from Slint without invoke-action round-trip
grep -n 'Bridge\.active-panel = Panel\.' senders/android/ui/components/control_bar.slint
# Expected ~3 matches (settings/debug/codec-test branches)
```

---

## Prerequisites

```sh
git fetch origin
git checkout migrate
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-7-ui-only
```

Build commands you'll run as you go:

```sh
cargo check -p android-sender                  # fastest sanity check
cargo build -p android-sender                  # full debug build
./gradlew :app:assembleDebug                   # APK (final visual)
```

Slint docs you'll cite are now mirrored locally under `draft/slint-ui/docs/` (PR #11). Direct paths are listed in the references footer at the bottom.

---

## Step 1 — Add `Panel` enum + `active-panel` property to `bridge.slint`

**File:** `senders/android/ui/bridge.slint`
**Spec:** PHASE-7 task 7-A.

The enum gets exported alongside `AppState`. The property is `in-out` so Slint can both write it (when the user taps a quick action or the Done button) and read it (the conditional blocks in `main.slint`). **No Rust callbacks** — Slint reads/writes the value directly.

### Diff

```diff
 export enum AppState {
     Disconnected,
     Connecting,
     SelectingSettings,
     WaitingForMedia,
     Casting,
 }

+export enum Panel {
+    none,
+    settings,
+    debug,
+    codec-test,
+}
+
 export struct QuickAction {
     id:      string,
     title:   string,
     enabled: bool,
     active:  bool,
 }

 ...

 export global Bridge {
     // ── Data properties (Rust → Slint) ──────────────────────────────────
     in property <[string]> devices: [
         // "Device 1", "Device 2",
     ];
     in-out property <AppState> app-state: AppState.Disconnected;
+    in-out property <Panel>    active-panel: Panel.none;
     in-out property <bool>     show-debug: false;
     in-out property <string>   test-status: "";
     in property <[QuickAction]> quick-actions: [];
     in property <[StatusItem]> status-items: [];
```

### Why

- **`enum Panel { none, settings, debug, codec-test }`** — Slint enums are closed value sets, type-checked at compile time, and pattern-match cleanly inside `if` chains ([structs-and-enums.mdx][structs]). Using `Panel.none` as the inactive sentinel is more explicit than `bool show-settings` / `bool show-codec-test` / etc., and keeps the routing chain mutually exclusive (only one panel can be active at a time, by construction).
- **`in-out` (not `in`)** — `in` properties are read-only from Slint and writable only from Rust. `in-out` allows both sides to read and write. Spec 7-A is explicit: *"Slint reads/writes `active-panel` directly. Wiring those callbacks is parked in `futures/`."* ([properties.mdx][props]).
- **Default `Panel.none`** — the app boots with no panel open. Same pattern as Phase 5's `mock-status-items` and Phase 6's `mock-empty: false` defaults — "off" is the safe boot state.
- **Don't remove `show-debug` yet** — the legacy `if Bridge.show-debug: DebugPage { }` inside `ConnectView` keeps working. Step 5 adds a parallel panel route. Phase 8 cleans up.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. The new enum + property are unused at this point but unused enums and properties are not errors in Slint 1.15.1.

---

## Step 2 — Build `FullSettingsPage` in `pages/settings_page.slint`

**File:** `senders/android/ui/pages/settings_page.slint`
**Spec:** PHASE-7 tasks 7-D, 7-E, 7-F, 7-G, 7-H.

We add `FullSettingsPage` as a **second exported component** in the same file, alongside the existing `SettingsPageView` (which still serves the `AppState.SelectingSettings` quality picker). The header comment in the file already anticipates this collision; we honour it by giving the new component the documented name.

### Diff

```diff
 // settings_page.slint — Pre-cast quality selection screen.
 // Shown when AppState == SelectingSettings.
 //
 // Renamed from SelectingSettingsView → SettingsPageView to avoid collision
 // with the full FullSettingsPage component added in Phase 7.
 //
 // MainWindow in main.slint references SettingsPageView.  The Rust side uses
 // AppState::SelectingSettings (unchanged) — no Rust changes needed here.
+//
+// Phase 7 adds FullSettingsPage in this same file.  It is a UI-only panel
+// reachable from `Bridge.active-panel == Panel.settings`.  All controls
+// flip inline `in-out` properties on the page itself — no Rust round-trip.

-import { VerticalBox } from "std-widgets.slint";
+import { VerticalBox, ScrollView } from "std-widgets.slint";
 import { Utils, VideoResolutionPicker, FrameratePicker } from "../../../../sdk/mirroring_core/ui/common.slint";
-import { Bridge } from "../bridge.slint";
+import { Bridge, Panel } from "../bridge.slint";
 import { Theme } from "../theme.slint";
-import { PrimaryButton, DestructiveButton } from "../components/buttons.slint";
+import { PrimaryButton, DestructiveButton, TextButton } from "../components/buttons.slint";
+import {
+    SettingsSection,
+    SettingsValueRow,
+    SettingsToggleRow,
+} from "../components/settings_rows.slint";

 export component SettingsPageView inherits Rectangle {
     ... // (unchanged — keep the existing pre-cast quality picker)
 }
+
+export component FullSettingsPage inherits Rectangle {
+    // ── UI-only stub state ───────────────────────────────────────────────
+    // All flips live on this component. Phase 8 swaps these for Bridge
+    // setters + Rust-backed handlers.
+    in-out property <int>    resolution-idx: 2;
+    in-out property <int>    framerate-idx:  2;
+    in-out property <bool>   mdns-enabled:   true;
+    in-out property <bool>   debug-panel:    false;
+    in-out property <string> mock-app-version: "0.0.1-dev";
+
+    // Fill the parent (overlay layer in main.slint passes parent.width/height).
+    width: parent.width;
+    height: parent.height;
+    background: Theme.surface-primary;
+
+    VerticalLayout {
+        // ── Header ────────────────────────────────────────────────────────
+        Rectangle {
+            height: 56px;
+            background: Theme.surface-card;
+            HorizontalLayout {
+                padding: Theme.padding-screen;
+                Text {
+                    text: "Settings";
+                    color: Theme.text-primary;
+                    font-size: Theme.font-size-heading;
+                    vertical-alignment: center;
+                    horizontal-stretch: 1;
+                }
+                TextButton {
+                    label: "Done";
+                    clicked => { Bridge.active-panel = Panel.none; }
+                }
+            }
+        }
+
+        // ── Body (scrollable) ─────────────────────────────────────────────
+        ScrollView {
+            VerticalLayout {
+                spacing: Theme.spacing-default;
+                padding: Theme.padding-screen;
+
+                // ── Section: RECEIVER ─────────────────────────────────────
+                SettingsSection {
+                    title: "RECEIVER";
+                    SettingsValueRow {
+                        title: "Discovered receivers";
+                        value: "3 found";
+                        // UI-only — would open the connect page.
+                        clicked => { /* placeholder */ }
+                    }
+                    SettingsToggleRow {
+                        title: "mDNS discovery";
+                        checked: root.mdns-enabled;
+                        toggled(checked) => { root.mdns-enabled = checked; }
+                    }
+                }
+
+                // ── Section: VIDEO QUALITY ────────────────────────────────
+                // Cycle-on-click is a placeholder UX. Real picker panels
+                // land in `futures/`.
+                SettingsSection {
+                    title: "VIDEO QUALITY";
+                    SettingsValueRow {
+                        title: "Max resolution";
+                        value: ["480p", "720p", "1080p", "1440p"][root.resolution-idx];
+                        clicked => {
+                            root.resolution-idx = (root.resolution-idx + 1) mod 4;
+                        }
+                    }
+                    SettingsValueRow {
+                        title: "Max framerate";
+                        value: ["24 fps", "30 fps", "60 fps"][root.framerate-idx];
+                        clicked => {
+                            root.framerate-idx = (root.framerate-idx + 1) mod 3;
+                        }
+                    }
+                }
+
+                // ── Section: CODEC & DEBUG ────────────────────────────────
+                SettingsSection {
+                    title: "CODEC & DEBUG";
+                    SettingsValueRow {
+                        title: "H.264 encoder test";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.codec-test; }
+                    }
+                    SettingsToggleRow {
+                        title: "Show debug panel";
+                        checked: root.debug-panel;
+                        toggled(checked) => { root.debug-panel = checked; }
+                    }
+                }
+
+                // ── Section: ABOUT ────────────────────────────────────────
+                SettingsSection {
+                    title: "ABOUT";
+                    SettingsValueRow {
+                        title: "App version";
+                        value: root.mock-app-version;
+                        show-chevron: false;
+                    }
+                    SettingsValueRow {
+                        title: "FCast protocol";
+                        value: "v3";
+                        show-chevron: false;
+                    }
+                }
+            }
+        }
+    }
+}
```

### Why each piece

- **`width: parent.width; height: parent.height;`** — `FullSettingsPage` is rendered as a sibling conditional inside `MainWindow`, **not** inside a layout container. Without explicit dimensions it would size to its content and float at `(0, 0)` with whatever intrinsic size its children compute. Binding to `parent.width` / `parent.height` makes it cover the entire window. See [positioning-and-layouts.mdx][layouts] — *"Inside a Window or any non-layout container element, child elements have their own `width` / `height` and use `x` / `y` to position themselves."*
- **`background: Theme.surface-primary`** — the panel body needs to be opaque or the page underneath bleeds through. `surface-primary` (#0b1020) matches the rest of the app's body chrome.
- **Header `Rectangle { height: 56px; background: Theme.surface-card }`** — Material-style fixed-height header bar. `surface-card` (#222633) gives it a subtle contrast against the body. The 56 px is one of the few unscaled fixed values in the spec — leave it as a literal (Phase 2-H carved out an exception for intrinsic widget heights).
- **`HorizontalLayout { padding: Theme.padding-screen }`** with one `Text` (`horizontal-stretch: 1`) and one `TextButton` — the canonical "title on the left, action on the right" pattern. `horizontal-stretch: 1` on the title gives it all leftover space, pushing the button to the right edge.
- **`TextButton { clicked => { Bridge.active-panel = Panel.none; } }`** — the close button writes the property directly. No `Bridge.close-panel()` callback needed — that was deliberate per spec 7-A.
- **`ScrollView { VerticalLayout { ... } }`** — `ScrollView` is the std-widget for a scrollable viewport that auto-derives `viewport-width` / `viewport-height` from its child layout when one is present ([scrollview.mdx][scrollview]). Putting a `VerticalLayout` directly inside means the inner sections can be longer than the screen and the user can scroll naturally.
- **`SettingsSection { title: "..."; ... }` + nested rows** — `SettingsSection` is a `VerticalLayout` with a `@children` slot ([components/settings_rows.slint:134-148](https://github.com/varxe-alt/fcast/blob/migrate/senders/android/ui/components/settings_rows.slint#L134-L148)). The header comment renders as a small grey label above the rows. **All rows use `title:`, not `label:`** — that name was canonicalised by PR #8.
- **`SettingsValueRow { clicked => { ... } }`** — the row already exposes a `clicked()` callback ([components/settings_rows.slint:39-47](https://github.com/varxe-alt/fcast/blob/migrate/senders/android/ui/components/settings_rows.slint#L39-L47)). The cycle-on-click pattern (`(root.resolution-idx + 1) mod 4`) is the spec's recommended placeholder UX. **Use the `mod` infix operator, not `mod(a, b)` function-call form** — that compile-error trap was caught by Devin Review on PR #6 and fixed by PR #8.
- **`SettingsValueRow { show-chevron: false }`** in the ABOUT section — the row already has a `show-chevron: bool` property ([components/settings_rows.slint:38](https://github.com/varxe-alt/fcast/blob/migrate/senders/android/ui/components/settings_rows.slint#L38)). Setting it to `false` removes the trailing `›` glyph for read-only info rows.
- **`SettingsToggleRow { checked: ...; toggled(checked) => { ... = checked } }`** — the row's `toggled` callback signature is `toggled(bool)` ([components/settings_rows.slint:76](https://github.com/varxe-alt/fcast/blob/migrate/senders/android/ui/components/settings_rows.slint#L76)) and the inner `CheckBox` already has `checked <=> root.checked` two-way binding. The handler should write `root.mdns-enabled = checked` (using the argument), **not** `root.mdns-enabled = !root.mdns-enabled`. The latter creates a feedback bug if the inner CheckBox toggles itself before the callback runs (the property and the visual would diverge).
- **Indexed string lookup `["480p", "720p", "1080p", "1440p"][root.resolution-idx]`** — Slint supports array indexing on inline literals ([repetition-and-data-models.mdx][repeat]). It's the cleanest way to display a "current value" string from a small enum-like int without defining a separate model struct.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. `FullSettingsPage` is exported but not yet referenced — that's fine, Slint allows unused components.

If the check fails with "no property `clicked` on `SettingsValueRow`", you've hit Phase 4 build artefacts that don't match the post-PR-#8 row signatures. Run `cargo clean -p android-sender` and re-check.

---

## Step 3 — Create `pages/codec_test_page.slint`

**File:** `senders/android/ui/pages/codec_test_page.slint` (new)
**Spec:** PHASE-7 task 7-I.

Minimal panel: same header chrome as `FullSettingsPage`, a `PrimaryButton` to "run" the test (UI-only no-op), and a hard-coded multiline log preview.

### New file

```slint
// codec_test_page.slint — H.264 encoder smoke-test panel (UI-only placeholder).
//
// Reachable from FullSettingsPage's "H.264 encoder test" row, which sets
// `Bridge.active-panel = Panel.codec-test`.  The "Run encoder test" button
// is a no-op in this phase; Phase 8 wires it to a Rust handler that exercises
// MediaCodec.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { PrimaryButton, TextButton } from "../components/buttons.slint";

export component CodecTestPage inherits Rectangle {
    // UI-only stub state.
    in-out property <string> mock-log:
        "[ready]\n"
        + "Codec     : video/avc (H.264)\n"
        + "Profile   : Baseline\n"
        + "Resolution: 1280x720\n"
        + "Framerate : 30 fps\n"
        + "Bitrate   : 4 Mbps\n"
        + "\n"
        + "Press \"Run encoder test\" to start.";

    width: parent.width;
    height: parent.height;
    background: Theme.surface-primary;

    VerticalLayout {
        // ── Header ────────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Codec test";
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

        // ── Body ──────────────────────────────────────────────────────────
        VerticalLayout {
            padding: Theme.padding-screen;
            spacing: Theme.spacing-default;

            PrimaryButton {
                label: "Run encoder test";
                // UI-only — no real test runner in this phase.
                clicked => { /* placeholder: would launch a Rust MediaCodec smoke test */ }
            }

            ScrollView {
                vertical-stretch: 1;
                Text {
                    text: root.mock-log;
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                    wrap: word-wrap;
                }
            }
        }
    }
}
```

### Why each piece

- **`mock-log` as `in-out property <string>`** — a developer can flip it from another component or a debug session to verify the scroll/wrap behaviour with a longer log. The `+` operator on string literals is plain Slint expression syntax ([expressions-and-statements.mdx][expressions]).
- **`width: parent.width; height: parent.height; background: Theme.surface-primary;`** — same overlay-fill pattern as `FullSettingsPage`. Same reasoning.
- **Header chrome is duplicated, not factored out** — yes. There's no `PanelHeader` component yet. Spec 7-I is minimal and copying ~14 lines of header is cheaper than introducing a new component for two call sites. **If you add a third panel** (`futures/` has a few candidates), refactor into `components/panel_header.slint` first.
- **`PrimaryButton` for "Run"** — this is a destructive-ish action (it'll exercise the encoder), so use `PrimaryButton` (steelblue) rather than `TextButton`. `DestructiveButton` is reserved for stop/disconnect verbs.
- **`ScrollView { vertical-stretch: 1; Text { wrap: word-wrap } }`** — the log can grow longer than the screen. `vertical-stretch: 1` makes the scrollview take all leftover vertical space inside the body `VerticalLayout`. `wrap: word-wrap` on the inner `Text` keeps long lines from overflowing horizontally ([text.mdx][text]).
- **`font-size: Theme.font-size-label`** for the log — small monospace-ish text is conventional for terminal-style output. We don't have a monospace font tokenised yet (would be Phase 9's job along with localization); body 12 px is acceptable for now.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. `CodecTestPage` is exported but not yet referenced anywhere (Step 5 hooks it into `main.slint`).

---

## Step 4 — Refactor `CastControlBar` to open panels from Slint

**File:** `senders/android/ui/components/control_bar.slint`
**Spec:** PHASE-7 task 7-C (with a documented deviation — see "Two paths" below).

PHASE-7 spec 7-C says *"In whichever `.slint` file declares the bar's quick-action stub model (Phase 4-G's inline `mock-quick-actions`), append…"*. **There is no inline `mock-quick-actions` model on `migrate`.** Phase 4 (PR #5) merged with Rust-driven dispatch — `Bridge.quick-actions` is populated from `lib.rs::build_quick_actions` and every click round-trips through `Bridge.invoke-action(id)`. The spec assumed the spec; the implementation went pragmatic.

Two paths through this gap. **Pick Path A** unless you have a specific reason to keep Phase 4 behaviour intact.

### Path A — Spec-aligned: convert the bar to inline `mock-quick-actions` (recommended)

This is the strict UI-only path. The bar declares its own stub model, and click dispatch is in-Slint (with a fallback to `invoke-action` for non-panel ids so debug/test buttons still work).

#### Diff

```diff
 // control_bar.slint — Persistent bottom action bar and quick-action buttons.
-// Stub created in Phase 1 to establish the import chain.
-// Components are implemented in Phase 4.
+// Implemented in Phase 4. Rewired in Phase 7 to (a) read from an inline
+// `mock-quick-actions` model and (b) open Slint-side panels directly via
+// `Bridge.active-panel` for `settings` / `debug` / `codec-test` ids.
 //
 // Will export: CastControlBar, QuickActionButton
-// Requires bridge.slint to export: QuickAction struct, invoke-action callback
+// Requires bridge.slint to export: QuickAction struct, Panel enum,
+// active-panel property, invoke-action callback (for non-panel ids).
 // Reference:  draft/moblin-ui/Moblin/View/ControlBar/ (design ref only)

 import { Theme } from "../theme.slint";
-import { Bridge, QuickAction } from "../bridge.slint";
+import { Bridge, QuickAction, Panel } from "../bridge.slint";

 export component QuickActionButton inherits Rectangle {
     ... // (unchanged)
 }

 export component CastControlBar inherits Rectangle {
+    // UI-only stub model. Phase 8 swaps for `Bridge.quick-actions` driven
+    // by Rust. The spec calls these out as the canonical Phase-7 set:
+    //   settings, debug, codec-test, scan-qr, migrated-server.
+    in-out property <[QuickAction]> mock-quick-actions: [
+        { id: "settings",        title: "Settings",       enabled: true, active: false },
+        { id: "debug",           title: "Debug",          enabled: true, active: false },
+        { id: "codec-test",      title: "Codec test",     enabled: true, active: false },
+        { id: "scan-qr",         title: "Scan QR",        enabled: true, active: false },
+        { id: "migrated-server", title: "Migrated srv",   enabled: true, active: false },
+    ];
+
     height: Theme.control-bar-height;
     background: Theme.surface-bar;

     HorizontalLayout {
         padding: Theme.padding-card;
         spacing: Theme.spacing-default;
         alignment: start;

-        for action in Bridge.quick-actions: QuickActionButton {
+        for action in root.mock-quick-actions: QuickActionButton {
             action: action;
-            invoked(id) => { Bridge.invoke-action(id); }
+            invoked(id) => {
+                // Panel-opening ids stay in Slint — no Rust round-trip.
+                if (id == "settings")    { Bridge.active-panel = Panel.settings;   return; }
+                if (id == "debug")       { Bridge.active-panel = Panel.debug;      return; }
+                if (id == "codec-test")  { Bridge.active-panel = Panel.codec-test; return; }
+                // Non-panel ids still go through the Rust handler.
+                Bridge.invoke-action(id);
+            }
         }
     }
 }
```

#### Why

- **`mock-quick-actions` declared on `CastControlBar`** — the spec assumes a stub model; this is the smallest place to put it. Phase 8 can either delete this property and switch the loop back to `Bridge.quick-actions`, or keep it as an "inline default" with the loop bound to a derived property that prefers `Bridge.quick-actions` when populated.
- **`return;` after each `Bridge.active-panel = ...` assignment** — Slint callback bodies are statement blocks. Without `return;`, every branch of the chain executes, and the trailing `Bridge.invoke-action(id)` would also fire, generating a phantom Rust callback for each panel open. `return;` short-circuits cleanly. (Slint has `if/else if/else`, but the if-then-return chain matches the spec snippet's style and is symmetric with the Phase-7 settings page click handlers.) See [functions-and-callbacks.mdx][callbacks] for callback body semantics.
- **Fallback to `Bridge.invoke-action(id)` for non-panel ids** — `scan-qr` and `migrated-server` still need their existing Rust handlers (in `lib.rs`). Don't break Phase 4's working integration.
- **`alignment: start` on the `HorizontalLayout`** — keep what Phase 4 set. Five 80 px buttons left-aligned with 8 px spacing fit comfortably on a phone.

#### Visual side effect

Until this step, the bar shows whatever Rust is pushing into `Bridge.quick-actions`. After this step, the bar shows the **5 ids hard-coded above**, regardless of what Rust pushes. Phase 8's job is to merge the two sources or delete the mock once Rust pushes the same set.

### Path B — Mixed (Rust-driven dispatch + Slint intercept) — alternative

Use this path **only if** you've already invested in the Rust-side `build_quick_actions` builder and want to keep Phase 4's `invoke-action` round-trip intact for routing.

#### Diff (smaller)

```diff
 import { Theme } from "../theme.slint";
-import { Bridge, QuickAction } from "../bridge.slint";
+import { Bridge, QuickAction, Panel } from "../bridge.slint";

 export component CastControlBar inherits Rectangle {
     height: Theme.control-bar-height;
     background: Theme.surface-bar;

     HorizontalLayout {
         padding: Theme.padding-card;
         spacing: Theme.spacing-default;
         alignment: start;

         for action in Bridge.quick-actions: QuickActionButton {
             action: action;
-            invoked(id) => { Bridge.invoke-action(id); }
+            invoked(id) => {
+                // Open panels in Slint without a Rust round-trip.
+                if (id == "settings")    { Bridge.active-panel = Panel.settings;   return; }
+                if (id == "debug")       { Bridge.active-panel = Panel.debug;      return; }
+                if (id == "codec-test")  { Bridge.active-panel = Panel.codec-test; return; }
+                Bridge.invoke-action(id);
+            }
         }
     }
 }
```

Plus you must add **one Rust line** in `lib.rs::build_quick_actions` (or wherever the action builder lives) to push a `QuickAction { id: "settings".into(), title: "Settings".into(), enabled: true, active: false }` so the row appears in the bar at all. **This is technically a Rust change** and breaks the "UI-only" guarantee. If you take Path B, document it in the PR.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. Visual: bar still works the same way for non-panel actions; new panel ids no longer round-trip through Rust.

---

## Step 5 — Add panel routing to `main.slint`

**File:** `senders/android/ui/main.slint`
**Spec:** PHASE-7 task 7-B.

Three sibling conditional blocks at the `MainWindow` root. Because Slint draws children in declaration order, putting the panel `if` blocks **after** the existing `VerticalLayout` makes them render on top of the page stack and control bar — which is the visual we want for an overlay.

### Diff

```diff
 // ── Bridge + AppState: import for local use, re-export for the Rust backend ──
-import { Bridge, AppState } from "bridge.slint";
-export { Bridge, AppState }
+import { Bridge, AppState, Panel } from "bridge.slint";
+export { Bridge, AppState, Panel }

 // ── Page component imports ────────────────────────────────────────────────────
 import { ConnectView }                   from "pages/connect_page.slint";
 import { ConnectingView }               from "pages/connecting_page.slint";
-import { SettingsPageView }             from "pages/settings_page.slint";
+import { SettingsPageView, FullSettingsPage } from "pages/settings_page.slint";
 import { WaitingForMediaView,
          CastingView }                  from "pages/casting_page.slint";
+import { CodecTestPage }                from "pages/codec_test_page.slint";
+import { DebugPage }                    from "pages/debug_page.slint";
 import { Theme } from "theme.slint";
 import { CastControlBar } from "components/control_bar.slint";

 // ── Root window ───────────────────────────────────────────────────────────────
 export component MainWindow inherits Window {
     background: Theme.surface-primary;

     VerticalLayout {
         Rectangle {
             vertical-stretch: 1;
             // Conditional elements: only the matching view is instantiated.
             // Slint docs: draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
             if Bridge.app-state == AppState.Disconnected:      ConnectView { }
             if Bridge.app-state == AppState.Connecting:        ConnectingView { }
             if Bridge.app-state == AppState.SelectingSettings: SettingsPageView { }
             if Bridge.app-state == AppState.WaitingForMedia:   WaitingForMediaView { }
             if Bridge.app-state == AppState.Casting:           CastingView { }
         }
         CastControlBar { }
     }
+
+    // ── Panel overlay layer ───────────────────────────────────────────────────
+    // Sibling to the VerticalLayout; declared AFTER so panels render on top of
+    // the page + control bar. Each panel root is a Rectangle that fills the
+    // window via `width/height: parent.*`, hiding what's behind.
+    if Bridge.active-panel == Panel.settings:   FullSettingsPage { }
+    if Bridge.active-panel == Panel.debug:      DebugPage { }
+    if Bridge.active-panel == Panel.codec-test: CodecTestPage { }
 }
```

### Why each piece

- **`import { Panel } from "bridge.slint"`** + **`export { ... Panel }`** — same re-export pattern Phase 1 documented in `main.slint`'s header comment for `Bridge` / `AppState`. The Rust side won't bind `Panel` (Phase 7 has no Rust changes), but it costs nothing to re-export and Phase 8 will benefit.
- **Three sibling `if` blocks at `MainWindow` root, after the `VerticalLayout`** — Slint conditional elements live where they're declared and render on top of earlier siblings (declaration order = paint order in the absence of `z` overrides). See [file.mdx][file] *"The syntax is `if condition : id := Element { ... }`"* for the conditional syntax, and [positioning-and-layouts.mdx][layouts] for the painting model.
- **Each panel component is responsible for filling the window** — `FullSettingsPage` and `CodecTestPage` set `width: parent.width; height: parent.height` (Step 2 / Step 3). `DebugPage` does **not** — see the gotcha below.
- **The legacy `if Bridge.show-debug: DebugPage { }` inside `ConnectView` stays** — Phase 7 only adds a parallel route. Phase 8 will pick one and remove the other.

### Gotcha — `DebugPage inherits VerticalBox`

`DebugPage` is currently `export component DebugPage inherits VerticalBox` ([senders/android/ui/pages/debug_page.slint:20](https://github.com/varxe-alt/fcast/pages/debug_page.slint#L20)) — a layout, not a Rectangle, with no `width: parent.width; height: parent.height` bindings. When used as a panel overlay it will float at the top with intrinsic size and the page stack will show through behind it. There are three ways to handle this in Phase 7:

1. **Leave it as-is (placeholder)** — the panel renders compact at the top. Document the cosmetic gap in the PR. Cleanest minimal-effort path.
2. **Wrap inline in `main.slint`** — replace the bare `DebugPage { }` with a `Rectangle` that fills the window and puts the `DebugPage` inside, plus a header bar. ~10 extra lines in `main.slint`.
3. **Refactor `DebugPage` to inherit `Rectangle` and add a header bar** — cleanest long-term but a separate file edit and a small risk of regressing the `if Bridge.show-debug: DebugPage { }` site inside `ConnectView` (which expects layout-style behaviour).

For Phase 7 take **option 1** — the spec just says *"Register in `main.slint` (already done in 7-B)"* without prescribing chrome. Note the limitation in the PR description.

### Build check

```sh
cargo check -p android-sender
cargo build -p android-sender
./gradlew :app:assembleDebug
```

Visual on device:
1. Boot the app — connect screen, control bar at the bottom with five quick-action cards (Settings / Debug / Codec test / Scan QR / Migrated srv on Path A).
2. Tap **Settings** — full settings panel slides over the page. Header reads "Settings" with a "Done" button on the right. Four sections visible. Toggle the mDNS row — the checkbox flips. Tap "Max resolution" — the value cycles `720p → 1080p → 1440p → 480p → 720p`. Same for framerate.
3. Tap "H.264 encoder test" — codec-test panel replaces the settings panel. Header reads "Codec test", "Run encoder test" button does nothing (no-op as designed), the log Text shows the multi-line stub.
4. Tap **Done** in either panel — back to the page underneath.
5. Tap **Debug** in the bar — `DebugPage` shows up (compact, no header — option 1 above). Tap a quick action again to switch panels.

---

## Step 6 — Final audit + commit

```sh
# 1. Panel enum + property exist in bridge.
grep -n 'Panel\b\|active-panel' senders/android/ui/bridge.slint
# Expected ~3 matches (enum body, in-out property line, the comment if you added one).

# 2. main.slint imports + uses Panel in three conditional blocks.
grep -n 'Panel\b' senders/android/ui/main.slint
# Expected ~5 matches:
#   - import line
#   - export line
#   - 3 conditional bodies

# 3. Both new components exported.
grep -rn 'export component FullSettingsPage\|export component CodecTestPage' \
    senders/android/ui/
# Expected:
#   senders/android/ui/pages/settings_page.slint:NN:export component FullSettingsPage inherits Rectangle {
#   senders/android/ui/pages/codec_test_page.slint:NN:export component CodecTestPage inherits Rectangle {

# 4. Control bar opens panels from Slint, no Rust round-trip for those ids.
grep -n 'Bridge\.active-panel = Panel\.' senders/android/ui/components/control_bar.slint
# Expected exactly 3 matches (settings/debug/codec-test branches).

# 5. No accidental `mod(a, b)` function-call form (was a recurring trap in
#    Phases 28+ and got fixed by PR #8).
grep -rn 'mod(' senders/android/ui/
# Expected: (empty)

# 6. No `label:` typo on settings rows (was a recurring trap fixed by PR #8).
grep -rn 'label:.*Discovered\|label:.*mDNS\|label:.*Max resolution\|label:.*Max framerate\|label:.*App version' \
    senders/android/ui/
# Expected: (empty)

# 7. No accidental `Theme.spacing-section` (token doesn't exist; only spacing-default does).
grep -rn 'Theme\.spacing-section' senders/android/ui/
# Expected: (empty)
```

If commands 5/6/7 return empty and 1/2/3/4 show only the rows above, you're spec-compliant.

```sh
git add senders/android/ui/
git status
# Expected (5 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/components/control_bar.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/codec_test_page.slint
git commit -m "feat(slint-ui): Phase 7 — settings panel + Panel routing chassis (UI-only)"
```

---

## Exit criteria checklist (mirrors `PHASE-7-settings-pages.md`)

- [ ] `bridge.slint` exposes `Panel` enum + `active-panel` (no callbacks).
- [ ] `main.slint` shows `FullSettingsPage` / `DebugPage` / `CodecTestPage` based on `active-panel`.
- [ ] The control bar's "Settings" stub action sets `Bridge.active-panel = Panel.settings`.
- [ ] `FullSettingsPage` renders the four sections (RECEIVER / VIDEO QUALITY / CODEC & DEBUG / ABOUT) with inline stub state.
- [ ] Toggle rows flip their stub property when tapped (no Rust round-trip).
- [ ] Done button closes the panel from Slint.
- [ ] `cargo build -p android-sender` passes.
- [ ] All 5 row component instances use `title:` (post-PR-#8 canonical name).
- [ ] All cycle-on-click handlers use `mod` infix operator (post-PR-#8 canonical syntax).

---

## When Phase 8 reactivates

When Rust takes over panel routing and persisted settings, the migration from this UI-only state is mostly **a binding swap** + **a few new Bridge setters**:

### `bridge.slint`

```diff
+    callback open-panel(Panel);
+    callback close-panel();
+    in property <int>    rust-resolution-idx;
+    in property <int>    rust-framerate-idx;
+    in property <bool>   rust-mdns-enabled;
+    in property <string> app-version;
```

`active-panel` stays as `in-out` so Slint can still close panels locally.

### `pages/settings_page.slint` (`FullSettingsPage`)

Swap the inline mock state for `Bridge.*` reads, and the `mod` cycle handlers for picker-panel callbacks (or push the index up to Rust through a `set-resolution-idx(int)` setter). The `mock-app-version` literal becomes `Bridge.app-version`.

### `components/control_bar.slint`

If you took Path A in Step 4: drop `mock-quick-actions` and switch the loop back to `for action in Bridge.quick-actions:`. Rust pushes panel-opening ids alongside the existing scan-qr / migrated-server ones; the Slint-side intercept can stay (so panels still don't round-trip through Rust) or be removed (let Rust do it via `open-panel`).

### `pages/debug_page.slint`

Refactor to `inherits Rectangle` and add a header bar with a Done button writing `Bridge.active-panel = Panel.none`. Drop the legacy `if Bridge.show-debug: DebugPage { }` site inside `ConnectView`. Pick whichever option for the gotcha in Step 5 you want to keep.

The `mock-*` properties on `FullSettingsPage` and `CodecTestPage` can be deleted at that point or left as design-time defaults (Slint won't warn).

---

## Slint-doc references used

These cite the **local mirror** at `draft/slint-ui/docs/` (PR #11). All paths are repo-relative.

- **`Panel` enum + `Bridge.active-panel` property** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` (closed value sets) and `properties.mdx` (`in-out` direction).
- **Inline `in-out` properties on `FullSettingsPage` (`resolution-idx`, `mdns-enabled`, `mock-app-version`, etc.)** — `properties.mdx`. Reactivity works the same for literal defaults and `Bridge.*` setters; swap is mechanical.
- **Conditional element syntax `if Bridge.active-panel == Panel.settings: FullSettingsPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx` (search for *"The syntax is `if condition : id := Element { ... }`"*).
- **`for action in root.mock-quick-actions: QuickActionButton { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Indexed string lookup `["480p", "720p", "1080p", "1440p"][root.resolution-idx]`** — `repetition-and-data-models.mdx` + `expressions-and-statements.mdx`. Slint allows array literals to be indexed in expressions.
- **`mod` infix operator (NOT `mod(a, b)` function)** — `expressions-and-statements.mdx`. The function-call form `mod(a, b)` is invalid Slint syntax and was caught by Devin Review on PR #6, fixed by PR #8.
- **`ScrollView { VerticalLayout { ... } }` auto-derived viewport** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`. *"If the ScrollView contains a layout, the default value for the `viewport-width` and `viewport-height` is the minimum size of that layout."*
- **`HorizontalLayout` / `VerticalLayout` with `padding`, `spacing`, `horizontal-stretch`, `vertical-stretch`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`. Bare `*Layout` has no implicit margins (unlike `*Box`); `*-stretch: 1` allocates leftover space along the main axis.
- **Window-fill panel root via `width: parent.width; height: parent.height; background: …`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx` and `positioning-and-layouts.mdx`. Conditional elements in a non-layout parent (Window / Rectangle) don't inherit a layout — they need explicit dimensions.
- **`Text { wrap: word-wrap }` for the multi-line codec test log** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`.
- **`SettingsValueRow` / `SettingsToggleRow` / `SettingsSection` are FCast components, not stdlib** — declared in `senders/android/ui/components/settings_rows.slint`. Their property names (`title:`, `value:`, `checked:`, `show-chevron:`, `toggled(bool)`) were finalised by [PR #8](https://github.com/varxe-alt/fcast/pull/8). The Phase-7 spec was updated to match.
- **`Bridge.active-panel = Panel.none` from a Slint callback body** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`. Callback bodies are statement blocks; assignments to `in-out` properties on globals are allowed.

---

## What's NOT in this guide

These belong to other phases and stay parked:

- **Persisted settings** (writing `mdns-enabled` / `resolution-idx` / `framerate-idx` to disk and replaying on launch) → Phase 8 (Rust bridge hardening).
- **Real receiver count** populating `"Discovered receivers"` from Rust mDNS → Phase 8 + Phase 6's `Bridge.receivers` migration.
- **Real app-version string** from Cargo / Android `BuildConfig` → Phase 8.
- **Real H.264 encoder test runner** behind the `Run encoder test` button → Phase 8 + a new Rust handler that exercises `MediaCodec`.
- **Picker panels** for resolution / framerate (replacing cycle-on-click with a proper "Select a value" sub-panel) → `futures/` or a future polish phase.
- **`@tr(...)` wrapping** of "Settings" / "RECEIVER" / "VIDEO QUALITY" / "Done" / "Run encoder test" / etc. → Phase 9 (localization sweep).
- **`PanelHeader` component** factored out of `FullSettingsPage` and `CodecTestPage` — defer until a third panel exists.
- **Refactoring `DebugPage`** to inherit `Rectangle` with a header bar so it looks like a real panel → Phase 8 cleanup or a separate polish phase.
- **Dropping the legacy `Bridge.show-debug` toggle** and the `if Bridge.show-debug: DebugPage { }` inside `ConnectView` → Phase 8 cleanup.
- **Per-section sub-pages** (audio / camera / bitrate / network / etc.) — those each get their own UI-only phase (Phases 13–27, see `phases/README.md`).

If you want to do any of the above, pause and write a separate phase doc first.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-7-settings-pages.md
[props]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[layouts]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[callbacks]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx
[scrollview]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
[text]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx

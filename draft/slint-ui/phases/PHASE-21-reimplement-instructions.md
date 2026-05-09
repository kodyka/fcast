# Phase 21 — Help & Support reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-21-help-and-support.md`][spec] to the current `senders/android` tree.
**Goal:** add four documentation sub-pages (`AboutPage`, `VersionHistoryPage`, `AttributionsPage`, `HelpPage`) wired into the `Panel` overlay layer; replace the inline `ABOUT` section in `FullSettingsPage` with a single `About` navigation row that opens `Panel.about`.
**Scope:** Slint UI only. **No Rust changes.** All content is hardcoded English text and a stub `mock-versions` array. URL launching is deferred — Help page rows are visual-only no-ops.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-21-help-and-support.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] first.** Same panel chrome, same row primitives, same gotchas. The new things in Phase 21 are: a back-stack invariant (sub-page Done buttons must return to `Panel.about`, not `Panel.none`), inline struct-typed array literals (`[{version: ..., date: ..., notes: ...}]`), and the deletion + replacement of an existing inline section in `FullSettingsPage`.

[p14]: ./PHASE-14-reimplement-instructions.md

---

## Why this guide exists

Phase 21 is the only Phase-7-dependent sub-page that **modifies an existing section** (`ABOUT`) instead of just appending. The replacement is structurally simple but easy to bungle if you treat it like a new sub-page added next to Audio/Camera. Specifically:

- The Phase 7 baseline has an inline `ABOUT` `SettingsSection` in `FullSettingsPage` containing two `SettingsValueRow`s (`App version` + `FCast protocol`). Phase 21 **deletes that section** and replaces it with a single navigation row that opens `Panel.about`. The detail moves into `AboutPage`.
- Sub-page `Done` buttons inside `Help` / `Attributions` / `Version history` should return to `Panel.about` — not `Panel.none`. The same back-stack invariant as Phase 16's edit page.
- The existing `mock-app-version: string` property on `FullSettingsPage` migrates to `AboutPage`.

The PHASE-21 spec snippet has no compile pitfalls (no `mod` infix, no toggle handlers), but it does mention several Moblin features that explicitly do **not** translate (URL launching, license-text generation, deep-link issue reporting). The guide pins those down so an implementer doesn't accidentally try.

After Phase 14 + 15 + 16 merge:

- `Panel { none, settings, debug, codec-test, audio, camera, bitrate-presets, bitrate-preset-edit }`. Phase 21 adds **four** variants: `about`, `version-history`, `attributions`, `help`.
- `FullSettingsPage` has the `ABOUT` section in place. Phase 21 **deletes** it and replaces it with a single row in a new `ABOUT & SUPPORT` section (or whatever name; the guide uses `ABOUT & SUPPORT` to match Material conventions).

This is **additive** Slint work spread across **three existing files** plus **four new files**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'AboutPage\|VersionHistoryPage\|AttributionsPage\|HelpPage' \
    senders/android/ui/

# Existing inline ABOUT section to be replaced:
grep -n 'title: "ABOUT"\|App version\|FCast protocol' senders/android/ui/pages/settings_page.slint
# Expected: 3 matches in the inline ABOUT section.

# mock-app-version currently lives on FullSettingsPage:
grep -n 'mock-app-version' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches (declaration + use).
```

After this guide is applied:

```sh
# Four new pages exported.
grep -rn 'export component AboutPage\|export component VersionHistoryPage\|export component AttributionsPage\|export component HelpPage' \
    senders/android/ui/

# Panel enum has 4 new variants.
grep -n 'about,\|attributions,\|version-history,\|help,' senders/android/ui/bridge.slint
# Expected: 4 matches.

# main.slint routes all 4 panels.
grep -n 'Panel\.about\|Panel\.attributions\|Panel\.version-history\|Panel\.help' \
    senders/android/ui/main.slint
# Expected: 4 matches.

# Inline ABOUT section is gone from FullSettingsPage.
grep -n 'App version\|FCast protocol' senders/android/ui/pages/settings_page.slint
# Expected: (empty) — moved to AboutPage.

# The single About entry row is in place.
grep -n 'Panel\.about' senders/android/ui/pages/settings_page.slint
# Expected: 1 match.

# mock-app-version moved to AboutPage.
grep -rn 'mock-app-version' senders/android/ui/
# Expected: 1+ matches inside pages/about_page.slint, none in pages/settings_page.slint.

# Sub-page Done buttons return to Panel.about, not Panel.none.
grep -n 'Bridge\.active-panel = Panel\.' \
    senders/android/ui/pages/version_history_page.slint \
    senders/android/ui/pages/attributions_page.slint \
    senders/android/ui/pages/help_page.slint
# Expected: 3 matches, all `Panel.about`.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-21-help-support
cargo check -p android-sender
```

---

## Step 1 — Add 4 `Panel` variants in `bridge.slint`

```diff
 export enum Panel {
     none,
     settings,
     debug,
     codec-test,
     audio,
     camera,
     bitrate-presets,
     bitrate-preset-edit,
+    about,
+    version-history,
+    attributions,
+    help,
 }
```

No new structs, no callbacks. The version history list is small enough to live as an inline literal model on `VersionHistoryPage`; promoting it to `Bridge` would be a Phase 8 concern.

---

## Step 2 — Route all 4 panels in `main.slint`

```diff
 import { BitratePresetEditPage }        from "pages/bitrate_preset_edit_page.slint";
+import { AboutPage }                    from "pages/about_page.slint";
+import { VersionHistoryPage }           from "pages/version_history_page.slint";
+import { AttributionsPage }             from "pages/attributions_page.slint";
+import { HelpPage }                     from "pages/help_page.slint";
 import { DebugPage, FullDebugPage }     from "pages/debug_page.slint";
```

```diff
     if Bridge.active-panel == Panel.bitrate-preset-edit: BitratePresetEditPage { }
+    if Bridge.active-panel == Panel.about:           AboutPage { }
+    if Bridge.active-panel == Panel.version-history: VersionHistoryPage { }
+    if Bridge.active-panel == Panel.attributions:    AttributionsPage { }
+    if Bridge.active-panel == Panel.help:            HelpPage { }
 }
```

---

## Step 3 — Create `pages/about_page.slint`

**File:** `senders/android/ui/pages/about_page.slint` (new)

The About page is the parent of the other three sub-pages. It shows app metadata + three navigation rows.

### New file

```slint
// about_page.slint — App metadata + entry to the support sub-pages.
//
// Reachable from FullSettingsPage's "About" row (sets
// `Bridge.active-panel = Panel.about`). The three sub-page rows here
// (`Version history`, `Open source attributions`, `Help & support`) open
// sibling panels that, on Done, return to `Panel.about` rather than
// `Panel.none` — see the back-stack invariant in the per-page Done
// handlers.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsValueRow,
} from "../components/settings_rows.slint";

export component AboutPage inherits Rectangle {
    // Migrated from FullSettingsPage.
    in-out property <string> mock-app-version: "0.0.1-dev";

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
                    text: "About";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    // Top-level page → return to cast screen.
                    clicked => { Bridge.active-panel = Panel.none; }
                }
            }
        }

        // ── Body ────────────────────────────────────────────────────────
        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── App identity ────────────────────────────────────────
                Rectangle {
                    height: 96px;
                    HorizontalLayout {
                        spacing: Theme.spacing-default;
                        // Placeholder app icon — colored rounded square.
                        Rectangle {
                            width: 64px;
                            height: 64px;
                            border-radius: 12px;
                            background: Theme.accent-active;
                            // Real PNG/SVG asset lands in a future polish phase.
                        }
                        VerticalLayout {
                            alignment: center;
                            horizontal-stretch: 1;
                            Text {
                                text: "FCast";
                                color: Theme.text-primary;
                                font-size: Theme.font-size-heading;
                            }
                            Text {
                                text: "Local-network screen casting";
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                        }
                    }
                }

                // ── Version metadata ────────────────────────────────────
                SettingsSection {
                    title: "VERSION";
                    SettingsValueRow {
                        title: "App version";
                        value: root.mock-app-version;
                        show-chevron: false;
                    }
                    SettingsValueRow {
                        title: "FCast protocol";
                        value: "v3";
                        show-chevron: false;
                    }
                }

                // ── Sub-page navigation ─────────────────────────────────
                SettingsSection {
                    title: "MORE";
                    SettingsValueRow {
                        title: "Version history";
                        value: "";
                        clicked => { Bridge.active-panel = Panel.version-history; }
                    }
                    SettingsValueRow {
                        title: "Open source attributions";
                        value: "";
                        clicked => { Bridge.active-panel = Panel.attributions; }
                    }
                    SettingsValueRow {
                        title: "Help & support";
                        value: "";
                        clicked => { Bridge.active-panel = Panel.help; }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`Done` returns to `Panel.none`.** AboutPage is reached from the settings root, so dismissing it should land on the cast screen. The sub-pages (Version history / Attributions / Help) reach AboutPage via Done, not the cast screen, because they're conceptually nested inside About.
- **Placeholder icon as a colored Rectangle.** Slint's `Image` element supports SVG/PNG, but ports usually defer the asset selection to a polish phase. A `Rectangle { background: Theme.accent-active; border-radius: 12px; }` is the visual placeholder — see [`PHASE-7-reimplement-instructions.md`](./PHASE-7-reimplement-instructions.md) for the same pattern in the receiver-card avatar.
- **Two `SettingsValueRow` entries with `show-chevron: false`** for `App version` + `FCast protocol`. These are read-only display rows, so suppressing the chevron is correct — same pattern Phase 7 used in the inline ABOUT section.
- **`value: ""`** on the three navigation rows. Empty value string + chevron-default-true gives a "tap to open" affordance without spurious right-side text. (The PHASE-21 spec didn't specify, but this matches the in-tree `Audio` / `Camera` / `Bitrate` rows from Phases 14–16.)
- **`mock-app-version: string` migrated from `FullSettingsPage`.** Mechanical move; do not bind to a Bridge property yet.

---

## Step 4 — Create `pages/version_history_page.slint`

**File:** `senders/android/ui/pages/version_history_page.slint` (new)

A scrollable list of release entries. Each entry is rendered with a `for entry in mock-versions:` loop over an inline struct-typed array.

### New file

```slint
// version_history_page.slint — Release notes list (UI-only placeholder).
//
// Reachable only from AboutPage; Done returns to Panel.about (NOT
// Panel.none) to preserve the parent → child → parent navigation flow.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";

export component VersionHistoryPage inherits Rectangle {
    // Inline struct-typed model. Newest first.
    in-out property <[{version: string, date: string, notes: string}]> mock-versions: [
        { version: "0.0.1-dev", date: "2026-05-03",
          notes: "UI placeholder build (Phases 0–11)." },
        { version: "0.0.0",     date: "2026-04-15",
          notes: "Initial Slint port commit." },
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
                    text: "Version history";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    // Back-stack invariant: return to parent (About), not
                    // to the cast screen.
                    clicked => { Bridge.active-panel = Panel.about; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                for entry in root.mock-versions: Rectangle {
                    border-radius: Theme.radius-card;
                    background: Theme.surface-card;
                    VerticalLayout {
                        padding: Theme.padding-screen;
                        spacing: 4px;
                        HorizontalLayout {
                            spacing: 8px;
                            Text {
                                text: entry.version;
                                color: Theme.text-primary;
                                font-size: Theme.font-size-body;
                                horizontal-stretch: 1;
                            }
                            Text {
                                text: entry.date;
                                color: Theme.text-secondary;
                                font-size: Theme.font-size-label;
                            }
                        }
                        Text {
                            text: entry.notes;
                            color: Theme.text-secondary;
                            font-size: Theme.font-size-label;
                            wrap: word-wrap;
                        }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **Inline struct-typed array literal `[{version: ..., date: ..., notes: ...}]`** — Slint allows anonymous struct types in property declarations and array literals. The compiler infers the struct shape from the literal. See [structs-and-enums.mdx][structs] (the section on anonymous structs).
- **`for entry in root.mock-versions:`** — single-iterable for-loop; index not needed here. See [repeat][repeat].
- **`wrap: word-wrap`** on the notes `Text` — multi-line release notes need explicit wrap. Default is no-wrap, which truncates. See [text.mdx][text].
- **`Done` returns to `Panel.about`** — back-stack invariant. Same as Phase 16's edit page returning to its list.

---

## Step 5 — Create `pages/attributions_page.slint`

**File:** `senders/android/ui/pages/attributions_page.slint` (new)

Long-form scrolling text listing third-party libraries. Plain `Text` entries inside a `ScrollView` — no taps, no links.

### New file

```slint
// attributions_page.slint — Open-source library attributions (UI-only).
//
// Hand-curated list. Real license-text generation requires Rust-side
// crate-graph traversal (e.g. cargo-about) → Phase 8.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";

export component AttributionsPage inherits Rectangle {
    // Inline struct-typed model — name + license one-liner.
    in-out property <[{name: string, license: string}]> mock-attributions: [
        { name: "Slint",        license: "GPLv3 / Royalty-Free / Commercial" },
        { name: "GStreamer",    license: "LGPL-2.1" },
        { name: "ExoPlayer",    license: "Apache-2.0" },
        { name: "Tokio",        license: "MIT" },
        { name: "Serde",        license: "MIT or Apache-2.0" },
        { name: "mdns-sd",      license: "MIT or Apache-2.0" },
        { name: "rustls",       license: "Apache-2.0 or ISC or MIT" },
        { name: "ring",         license: "OpenSSL / ISC" },
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
                    text: "Attributions";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    clicked => { Bridge.active-panel = Panel.about; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                Text {
                    text: "FCast incorporates the following open source libraries.";
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                    wrap: word-wrap;
                }

                for entry in root.mock-attributions: Rectangle {
                    height: 48px;
                    HorizontalLayout {
                        Text {
                            text: entry.name;
                            color: Theme.text-primary;
                            font-size: Theme.font-size-body;
                            vertical-alignment: center;
                            horizontal-stretch: 1;
                        }
                        Text {
                            text: entry.license;
                            color: Theme.text-secondary;
                            font-size: Theme.font-size-label;
                            vertical-alignment: center;
                        }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **No `clicked` handlers on rows.** The spec is explicit: *"No links / no tappable items in UI-only build."* Real URL launching requires `android.intent.action.VIEW` via JNI, which is Phase 8.
- **Hand-curated license list.** Auto-generation requires `cargo-about` or similar; deferred to a future phase.
- **Single intro paragraph + `for entry`** keeps the page data-driven without a "header row" layout trap (no need for first-cell alignment with subsequent rows).

---

## Step 6 — Create `pages/help_page.slint`

**File:** `senders/android/ui/pages/help_page.slint` (new)

Three sections (`GETTING STARTED`, `TROUBLESHOOTING`, `CONTACT`). Contact rows are no-op clickable; getting-started + troubleshooting are static text.

### New file

```slint
// help_page.slint — Getting started / FAQ / contact (UI-only).
//
// Contact rows are clickable but do nothing — real URL launching
// requires Bridge.open-url(string) on the Rust side. Phase 8.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsValueRow,
} from "../components/settings_rows.slint";

export component HelpPage inherits Rectangle {
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
                    text: "Help & support";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    clicked => { Bridge.active-panel = Panel.about; }
                }
            }
        }

        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                // ── Getting started ─────────────────────────────────────
                SettingsSection {
                    title: "GETTING STARTED";
                }
                Rectangle {
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    VerticalLayout {
                        padding: Theme.padding-screen;
                        spacing: 8px;
                        Text {
                            text: "Cast your first stream in three steps.";
                            color: Theme.text-primary;
                            font-size: Theme.font-size-body;
                            wrap: word-wrap;
                        }
                        Text {
                            text: "1. On the receiver, install the FCast TV/desktop app.\n"
                                + "2. On this device, ensure both are on the same Wi-Fi network.\n"
                                + "3. Tap the receiver from the discovery list to start casting.";
                            color: Theme.text-secondary;
                            font-size: Theme.font-size-label;
                            wrap: word-wrap;
                        }
                    }
                }

                // ── Troubleshooting ─────────────────────────────────────
                SettingsSection {
                    title: "TROUBLESHOOTING";
                }
                Rectangle {
                    background: Theme.surface-card;
                    border-radius: Theme.radius-card;
                    VerticalLayout {
                        padding: Theme.padding-screen;
                        spacing: 8px;
                        Text {
                            text: "• No receivers found? Make sure mDNS is enabled on your network.\n"
                                + "• Stream stutters? Lower the bitrate in Audio & Video → Bitrate.\n"
                                + "• Cast disconnects after a few seconds? Disable battery optimisation for FCast in system settings.";
                            color: Theme.text-secondary;
                            font-size: Theme.font-size-label;
                            wrap: word-wrap;
                        }
                    }
                }

                // ── Contact ─────────────────────────────────────────────
                SettingsSection {
                    title: "CONTACT";
                    SettingsValueRow {
                        title: "Documentation";
                        value: "fcast.app";
                        // No-op: real URL launching is Phase 8.
                        clicked => { }
                    }
                    SettingsValueRow {
                        title: "Source code";
                        value: "github.com/varxe-alt/fcast";
                        clicked => { }
                    }
                    SettingsValueRow {
                        title: "Report an issue";
                        value: "GitHub Issues";
                        clicked => { }
                    }
                }
            }
        }
    }
}
```

### Why each piece

- **`SettingsSection { title: "GETTING STARTED"; }` followed by a separate `Rectangle { ... }`** — the `SettingsSection` component renders only the title header; passing children to it is for `SettingsValueRow` / `SettingsToggleRow` / `SettingsSliderRow`, not free-form text. For free-form sections, use `SettingsSection` for the heading + a sibling `Rectangle` with the content. (Verify the actual `SettingsSection` definition before applying — it accepts `@children` only when shape-compatible. If your local version uses a stricter slot, render the section title manually with a `Text` element instead.)
- **Multi-line strings via `\n` + `+` concatenation.** Slint's `+` operator on string-on-string is valid (only numeric-on-string is rejected — see Phase 15 §"Gotcha 4"). Each numbered step on its own line uses an explicit `\n`. See [expressions-and-statements.mdx][expressions].
- **`clicked => { }`** — empty-body callback. Slint accepts empty callbacks; this preserves the row's pressed state (TouchArea inside `SettingsValueRow` still flashes) without doing anything. Once `Bridge.open-url(string)` lands in Phase 8, swap to `clicked => { Bridge.open-url("https://fcast.app"); }`.

---

## Step 7 — Replace inline ABOUT in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Delete the inline `ABOUT` section (3 rows), delete the `mock-app-version` declaration, replace with a single `About` navigation row in a new `ABOUT & SUPPORT` section.

### Diff

```diff
-    in-out property <string> mock-app-version: "0.0.1-dev";
-
     in-out property <bool>   mock-mdns-enabled:           true;
```

```diff
-                // ── Section: ABOUT ────────────────────────────────────────
-                SettingsSection {
-                    title: "ABOUT";
-                    SettingsValueRow {
-                        title: "App version";
-                        value: root.mock-app-version;
-                        show-chevron: false;
-                    }
-                    SettingsValueRow {
-                        title: "FCast protocol";
-                        value: "v3";
-                        show-chevron: false;
-                    }
-                }
+                // ── Section: ABOUT & SUPPORT ──────────────────────────────
+                SettingsSection {
+                    title: "ABOUT & SUPPORT";
+                    SettingsValueRow {
+                        title: "About";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.about; }
+                    }
+                }
             }
         }
     }
 }
```

### Why each piece

- **`mock-app-version` deletion is intentional** — the property migrates to `AboutPage` (set as a stub with the same `"0.0.1-dev"` value). Phase 8 will lift it to `Bridge.app-version` either at `FullSettingsPage` or `AboutPage`; either site is fine, but a single property in a single page is cleaner.
- **One row replaces two** — the protocol-version row was an info-only display. It moves into AboutPage's `VERSION` section. Don't keep a duplicate.

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. All four pages exist and exported.
grep -rn 'export component AboutPage\|export component VersionHistoryPage\|export component AttributionsPage\|export component HelpPage' \
    senders/android/ui/

# 2. All four panel variants in bridge.slint.
grep -n 'about,\|attributions,\|version-history,\|help,' senders/android/ui/bridge.slint
# Expected: 4 matches (inside Panel enum body).

# 3. main.slint routes all four.
grep -n 'Panel\.about\|Panel\.attributions\|Panel\.version-history\|Panel\.help' \
    senders/android/ui/main.slint
# Expected: 4 matches.

# 4. Inline ABOUT removed from FullSettingsPage.
grep -n '"App version"\|"FCast protocol"\|title: "ABOUT"' senders/android/ui/pages/settings_page.slint
# Expected: (empty) — moved to AboutPage.

# 5. mock-app-version migrated.
grep -rn 'mock-app-version' senders/android/ui/
# Expected: only matches inside pages/about_page.slint.

# 6. About entry row present in FullSettingsPage.
grep -n 'Panel\.about' senders/android/ui/pages/settings_page.slint
# Expected: 1 match.

# 7. Sub-pages return to Panel.about (back-stack invariant).
grep -n 'Bridge\.active-panel = Panel\.' \
    senders/android/ui/pages/version_history_page.slint \
    senders/android/ui/pages/attributions_page.slint \
    senders/android/ui/pages/help_page.slint
# Expected: 3 matches, each `Panel.about`.

# 8. AboutPage's Done returns to Panel.none (top-level page).
grep -n 'Bridge\.active-panel = Panel\.' senders/android/ui/pages/about_page.slint
# Expected: 4 matches — 1 Panel.none (Done), 3 sub-page openers
# (Panel.version-history, Panel.attributions, Panel.help).

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/
git status
# Expected (7 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/about_page.slint
#   new file:   senders/android/ui/pages/version_history_page.slint
#   new file:   senders/android/ui/pages/attributions_page.slint
#   new file:   senders/android/ui/pages/help_page.slint
git commit -m "feat(slint-ui): Phase 21 — about / version history / attributions / help pages (UI-only)"
```

---

## Gotchas (Phase 21 specific)

The Phase 14 gotchas (toggle feedback, `Math.mod`, slider unit conversion) don't apply directly here — Phase 21 has no toggles, no cyclers, no sliders. But three new ones:

### Gotcha 10 — Sub-page `Done` returns to parent, not root

**Symptom:** user opens `About → Help` then taps `Done` → cast screen pops up. They lose context and must re-navigate.

**Cause:** copy-pasting the `Done` handler from a top-level page (e.g. AudioPage) gives `Panel.none`. For a child of About, that's wrong.

**Fix:** child sub-pages write `Bridge.active-panel = Panel.about;` in their Done handler. Only `AboutPage`'s own Done writes `Panel.none`. The `Sanity grep` step explicitly checks this.

### Gotcha 11 — `SettingsSection { @children }` shape

**Symptom:** Help page free-form text inside `SettingsSection { ... }` either doesn't render or breaks the section's title styling.

**Cause:** `SettingsSection` (in `components/settings_rows.slint`) is shaped to receive `SettingsValueRow` / `SettingsToggleRow` / `SettingsSliderRow` only. Embedding free-form `Text` or layouts inside it may either fail to compile or render with the wrong padding/divider treatment. The exact shape depends on whether `SettingsSection` uses `@children` or named slots; check the local definition before adding free-form content.

**Fix:** for free-form sections (Getting started / Troubleshooting), render the section title with a `SettingsSection { title: "..."; }` containing **no children**, then use a sibling `Rectangle` for the content body. The visual result matches a standard section but the body is independently styled.

If your local `SettingsSection` is more permissive (accepts `@children` of any kind), the workaround is unnecessary — the guide's snippet works as-is.

### Gotcha 12 — Anonymous struct types in array property declarations

**Symptom:** Slint compiler error `expected type, got '{'` on `in-out property <[{version: string, date: string, notes: string}]>`.

**Cause:** Slint's anonymous-struct syntax inside an array-type binding requires the `{...}` to be parsed inside the property-type bracket. Older Slint versions (pre-1.5) reject this syntax; newer versions (1.5+) accept it as documented in [structs-and-enums.mdx][structs].

**Fix (preferred):** confirm the project's Slint version (look in `senders/android/Cargo.toml` for `slint = "..."`). If ≥ 1.5, the inline anonymous-struct array works. If older, declare a named struct in `bridge.slint`:

```slint
export struct VersionEntry {
    version: string,
    date:    string,
    notes:   string,
}
```

then use `in-out property <[VersionEntry]> mock-versions: [...];`. The named-struct form is always safe.

---

## Exit criteria checklist

- [ ] `bridge.slint` has 4 new `Panel` variants (`about`, `version-history`, `attributions`, `help`).
- [ ] `main.slint` routes all 4.
- [ ] `AboutPage` shows app icon placeholder, name, tagline, version metadata, three sub-page rows.
- [ ] Tapping each sub-page row opens the corresponding panel.
- [ ] `VersionHistoryPage` lists 2 stub entries with version/date/notes.
- [ ] `AttributionsPage` lists 8 stub libraries with license one-liners.
- [ ] `HelpPage` shows getting-started + troubleshooting + 3 contact rows; contact rows are clickable but no-op.
- [ ] All sub-pages' Done buttons return to `Panel.about`, not `Panel.none`.
- [ ] `FullSettingsPage`'s inline `ABOUT` section is replaced by a single `About` navigation row.
- [ ] `mock-app-version` migrated from `FullSettingsPage` to `AboutPage`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <string> app-version;
+    in property <string> protocol-version;
+    in property <[VersionEntry]> version-history;
+    in property <[Attribution]>  attributions;
+    callback open-url(string);
```

Functional integration:
- `app-version` ← `env!("CARGO_PKG_VERSION")` from Rust.
- `protocol-version` ← FCast protocol library constant.
- `version-history` ← bundled JSON parsed at startup, or compiled-in via `include_str!`.
- `attributions` ← `cargo-about` output integrated into the build.
- `open-url(string)` → JNI to `Intent.ACTION_VIEW`. Wire to the three Help page contact rows.

---

## Slint-doc references used

- **`Panel` enum extension** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **Anonymous struct types in array properties** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`for entry in root.mock-versions: Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **String concatenation `"line A\n" + "line B"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`Text.wrap: word-wrap`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`.
- **`ScrollView` auto-derived viewport** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`.
- **Conditional element `if Bridge.active-panel == Panel.about: AboutPage { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **Empty callback body `clicked => { }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **`SettingsSection`, `SettingsValueRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.
- **`TextButton`** — FCast component in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real URL launching (`Intent.ACTION_VIEW`).** Phase 8.
- **Auto-generated license text.** Requires `cargo-about` or similar in the build pipeline; Phase 8 / build-system work.
- **Issue-report deep links.** Same as URL launching.
- **Live changelog from a remote server.** Out of scope; would require a Rust HTTP client + parser; Phase 8 plus an explicit feature phase.
- **Real app icon asset.** Polish phase.
- **Issue-template pre-fill data.** Out of scope.
- **`@tr(...)` wrapping** of `"About"` / `"Version history"` / `"Attributions"` / `"Help & support"` / `"Documentation"` / `"Source code"` / `"Report an issue"` / etc. — Phase 9 (localization sweep).

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-21-help-and-support.md
[p14]: ./PHASE-14-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[structs]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx
[text]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx

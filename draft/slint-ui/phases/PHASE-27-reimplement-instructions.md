# Phase 27 — Shared Utility Components reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-27-utils-backlog.md`][spec] to the current `senders/android` tree.
**Goal:** establish the on-demand extraction discipline for small reusable Slint components, and ship the **first three** widely-needed utilities that consume cleanly into the existing UI-only pages: `IconAndText`, `InfoBanner`, `ValueEditChip`. Each is < 80 lines, theme-token-driven, and used by at least one prior phase.
**Scope:** Slint UI only. **No Rust changes.** No speculative extraction — every component shipped here has at least one already-shipped consumer that benefits from it.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-27-utils-backlog.md

> **Read [`PHASE-19-reimplement-instructions.md`][p19], [`PHASE-22-reimplement-instructions.md`][p22], and [`PHASE-20-reimplement-instructions.md`][p20] first.** Phase 19's `ConfirmDialog` set the precedent for shared overlays; Phase 22 introduced the auto-hide banner pattern that `InfoBanner` extracts; Phase 20's status pill is structurally an `IconAndText` cousin. The new things in this guide: when **not** to extract, the canonical "extract → migrate one consumer → migrate the rest in follow-up commits" workflow, and three idiomatic util implementations.

[p19]: ./PHASE-19-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md
[p20]: ./PHASE-20-reimplement-instructions.md

---

## Why this guide is different from the others

Phases 14–26 each produced a guide that says "do these 4–6 numbered steps, you'll have a new page". Phase 27 is **ongoing** — it has no completion criteria of its own; it's a backlog. This guide therefore has a different shape:

- **Section A** — extraction principles (when to pull a util, when to leave the duplication).
- **Section B** — three worked extractions (`IconAndText`, `InfoBanner`, `ValueEditChip`), each with file, consumer migration diff, and Slint-doc citations.
- **Section C** — the rest of the backlog (which prior-phase patterns are still candidates, the criteria each must meet before extraction).
- **Section D** — anti-patterns (don't extract these).

Every util shipped here MUST satisfy these gates **before** the PR is opened:

1. The util has **at least one already-shipped consumer** that today contains the same code shape.
2. The util has **at least one anticipated future consumer** documented in this guide.
3. The util is **< 80 lines** (Slint primitives don't need to be big).
4. The util uses **only `Theme.*` tokens** (no raw `#xxxxxx` colors except as documented Slint named values like `white`).
5. The util **compiles in isolation** — `slint-viewer senders/android/ui/components/<name>.slint` opens it standalone.

---

## Section A — Extraction principles

### A1. Extract on the third repetition, not the first

Slint imports add real friction (cross-file type juggling, version-pinned syntax surprises, harder name resolution in the reference manual). Two copies of a 10-line block are fine; three copies is the trigger. The codebase right now has:

- 2 copies of the HH:MM:SS formatter (Phase 23 recording, Phase 20 cast history) — **do not extract yet**.
- 3+ inline 3s-auto-hide banner sites (Phase 19 backup-reset, Phase 22 Wi-Fi Aware, Phase 20 Clear-history success-toast variant) — **extract now** as `InfoBanner`.
- 4+ inline name-with-icon rows (Phase 13 status pills, Phase 20 history rows, Phase 22 network rows, Phase 26 debug video element table) — **extract now** as `IconAndText`.
- 0 sites of `[ - ] N [ + ]` numeric-stepper edit — **defer** (no consumer yet; Phase 16's bitrate slider doesn't need it; Phase 18's snapshot-secs has no UI surface).

This guide ships `IconAndText` and `InfoBanner` because they pass the threshold. It also pre-defines `ValueEditChip` as a worked example for the next reader who does cross the threshold (e.g. once Phase 18 grows a snapshot-secs slider).

### A2. The util must be theme-token-driven

Every color, spacing, and font-size belongs to `Theme.*`. If the util needs a new token, **add it to `theme.slint` first** — that's a separate one-line change. Don't ship a util with an inline `#abc` because "we'll fix theming later". The util must outlive the current visual draft.

### A3. The util's API must match its narrowest consumer

A util that takes 12 properties is a sign you forgot to factor out two utils. `IconAndText` has 2 properties (`icon`, `label`). `InfoBanner` has 3 (`message`, `severity`, `visible`). `ConfirmDialog` (Phase 19's util) has 5 — that's near the upper bound for a util this size. Anything bigger and you should split.

### A4. Migration is one-consumer-at-a-time

Don't migrate every call site in the same PR as the extraction. The pattern is:

1. **PR #1** — add the util + migrate **one** consumer to use it. CI green, visual review of the migrated consumer.
2. **PR #2..N** — migrate remaining consumers, one per PR. Each PR is small and reversible.

Bundling 4 consumer migrations into the extraction PR makes the diff unreadable and makes regression hunting harder.

---

## Section B — Three worked extractions

### B1. `IconAndText`

Tiny `HorizontalLayout { Image; Text }` row. Used in status badges, history rows, network row labels, debug-video metric tables.

**Consumers (already shipped):**

- Phase 20 cast history list — receiver name + status pill (icon variant).
- Phase 22 network interface row — interface name + connectivity glyph.
- Phase 26 debug video page — element-state table cells.

**Consumers (anticipated):**

- Phase 13 status overlay — every status item is name + severity icon.
- Phase 21 help & support — link rows have leading icon + label.

#### File: `senders/android/ui/components/icon_and_text.slint`

```slint
// icon_and_text.slint — Icon + label row primitive.
//
// Used wherever a row's content is "small leading visual + descriptive
// text". Replaces the 6+ inline HorizontalLayout { Image | Text { glyph };
// Text { label } } sites scattered across pages.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx

import { Theme } from "../theme.slint";

export component IconAndText inherits HorizontalLayout {
    in property <string> icon;       // Unicode glyph or empty
    in property <string> label;
    in property <color>  label-color: Theme.text-primary;
    in property <length> font-size:   Theme.font-size-body;
    spacing: 6px;

    if root.icon != "": Text {
        text: root.icon;
        color: root.label-color;
        font-size: root.font-size;
        vertical-alignment: center;
    }

    Text {
        text: root.label;
        color: root.label-color;
        font-size: root.font-size;
        vertical-alignment: center;
    }
}
```

#### Why this shape

- **`inherits HorizontalLayout`** — the util **is** a layout, not a wrapper around one. Direct inheritance keeps the consumer's `HorizontalLayout` parent able to size and stretch the icon-text pair without an extra `Rectangle` wrapper. See [positioning-and-layouts.mdx][positioning] for layout-component composition.
- **`if root.icon != "":`** branch — empty string skips the icon Text element. Allows the same util to render label-only rows.
- **`color: root.label-color;`** with `Theme.text-primary` default — consumers override for muted/highlighted variants. Per [colors-and-brushes.mdx][colors].
- **`font-size: root.font-size;`** with `Theme.font-size-body` default — same override pattern.
- **No `Image` element** — Slint's `Image` requires an asset path or programmatic source (per [image.mdx][image]). Until the design system bundles real icons, Unicode glyphs (`⚠`, `▶`, `✓`, `›`) are the canonical placeholder. Future extension: add `in property <image> icon-image;` for raster icons; render whichever non-empty input wins.

#### Migrate one consumer (Phase 22 network row)

The most readable migration target. The Phase 22 row currently has an inline `HorizontalLayout` with a single `Text { text: interface.name; ... }`. There's no glyph yet — wifi connectivity lives elsewhere in the row. **Skip this consumer for now**; come back when Phase 22 grows a connectivity icon.

#### Migrate one consumer (Phase 20 cast history row)

The status pill is structurally `IconAndText` with a colored background. Two-step migration:

1. Extract the status pill text from the existing `HorizontalLayout { Text { receiver }; Rectangle { background: status-color; Text { status } } }` shape.
2. Replace the `Rectangle { Text { ... } }` cluster with `Rectangle { IconAndText { label: entry.status; label-color: white; ... } }` if you want — but the gain is marginal since the pill has its own background. **Better fit:** the row's main label uses `IconAndText` with the receiver name; status pill stays inline (it's not really an "icon + text" — it's a "background + text").

**Verdict:** the cleanest first migration target is **Phase 26's debug video page**, where the element-state table has 4 rows of `glyph + label`.

#### Build check

```sh
cargo check -p android-sender
```

---

### B2. `InfoBanner`

Pill-shaped transient banner with auto-hide via `Timer`. Replaces the inline 3s-banner code that appears across Phase 19, 22, 20 (and any future phase with a "completed" toast).

**Consumers (already shipped):**

- Phase 19 backup-reset — "Exported to ~/fcast-backup.json" / "Reset complete" toasts.
- Phase 22 network — "Wi-Fi Aware deferred to Phase 8" warning.
- Phase 20 cast history (variant) — "Clear all" success toast.

**Consumers (anticipated):**

- Phase 16 bitrate preset save — "Preset saved" toast.
- Phase 25 macro save — "Macro saved" toast.
- Phase 18 lock-engaged hint — "UI Locked" status pill.

#### File: `senders/android/ui/components/info_banner.slint`

```slint
// info_banner.slint — Auto-hiding transient banner pill.
//
// Consumer pattern:
//
//     InfoBanner {
//         message: "Exported to ~/fcast-backup.json";
//         severity: BannerSeverity.info;
//         visible <=> root.banner-visible;
//     }
//
// The component owns the Timer that flips `visible` back to false
// after `auto-hide-ms`; the consumer just needs to set visible = true
// to trigger.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx

import { Theme } from "../theme.slint";

export enum BannerSeverity {
    info,
    success,
    warning,
    error,
}

export component InfoBanner inherits Rectangle {
    in property <string>          message;
    in property <BannerSeverity>  severity: BannerSeverity.info;
    in-out property <bool>        visible: false;
    in property <duration>        auto-hide-ms: 3s;

    height: root.visible ? 40px : 0px;
    clip: true;
    background: root.severity == BannerSeverity.error
                  ? #c62828
              : root.severity == BannerSeverity.warning
                  ? #ed6c02
              : root.severity == BannerSeverity.success
                  ? #2e7d32
              : Theme.accent-active.darker(20%);
    animate height { duration: 200ms; easing: ease-out; }

    HorizontalLayout {
        padding-left:  Theme.padding-screen;
        padding-right: Theme.padding-screen;
        Text {
            text: root.message;
            color: white;
            vertical-alignment: center;
            font-size: Theme.font-size-label;
        }
    }

    Timer {
        interval: root.auto-hide-ms;
        running: root.visible;
        triggered => { root.visible = false; }
    }
}
```

#### Why this shape

- **`in-out property <bool> visible`** — two-way bound. The consumer flips it true; the Timer flips it false. The `<=>` operator in the consumer's invocation lets a parent property drive it both directions.
- **`height: root.visible ? 40px : 0px;` + `clip: true;`** — the banner collapses to zero height when hidden, taking no flow space. `clip: true` ensures content doesn't bleed out during the animated collapse. The `animate height { duration: 200ms; }` smooths the show/hide. See [animation.mdx][animation].
- **`severity` enum + branching background** — same triple-ternary as `status-color` in Phase 20. Once `Theme.success / Theme.warning / Theme.error` exist, replace the inline hex literals.
- **Timer auto-hide** — same Phase 22 / Phase 19 pattern, now centralised. The `interval: root.auto-hide-ms;` lets consumers override (default 3s).

#### Migrate one consumer (Phase 19 backup-reset)

The Phase 19 page declares its own `banner-visible: bool` + `banner-message: string` + a `Timer { interval: 3s; running: root.banner-visible; triggered => { root.banner-visible = false; } }`. After extraction:

```diff
-    property <bool>   banner-visible: false;
-    property <string> banner-message: "";

-    Timer {
-        interval: 3s;
-        running: root.banner-visible;
-        triggered => { root.banner-visible = false; }
-    }
+    in-out property <bool>   banner-visible: false;
+    in-out property <string> banner-message: "";
```

```diff
-        if root.banner-visible: Rectangle {
-            height: 40px;
-            background: Theme.accent-active.darker(20%);
-            HorizontalLayout {
-                padding: Theme.padding-screen;
-                Text {
-                    text: root.banner-message;
-                    color: Theme.text-primary;
-                    vertical-alignment: center;
-                    font-size: Theme.font-size-label;
-                }
-            }
-        }
+        InfoBanner {
+            message: root.banner-message;
+            visible <=> root.banner-visible;
+        }
```

The page's `show-banner(message)` helper still works; the only behavioural change is the animated collapse.

#### Build check

```sh
cargo check -p android-sender
```

---

### B3. `ValueEditChip` (deferred)

Compact numeric editor: `[ - ] 42 [ + ]`. Three nested elements; clicks mutate a bound int. **No consumer ships this in the current code base** — Phase 16's bitrate edit uses a slider. Documented here as the canonical shape for the next reader.

#### File: `senders/android/ui/components/value_edit_chip.slint` (defer until first consumer)

```slint
// value_edit_chip.slint — Compact +/- numeric stepper.
//
// Used wherever a small integer needs incremental adjustment without a
// full slider's footprint. Consumer pattern:
//
//     ValueEditChip {
//         value <=> root.snapshot-secs;
//         min: 1;
//         max: 30;
//         step: 1;
//     }
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx

import { Theme } from "../theme.slint";

export component ValueEditChip inherits HorizontalLayout {
    in-out property <int> value;
    in property <int>     min:  0;
    in property <int>     max:  100;
    in property <int>     step: 1;
    spacing: 4px;

    Rectangle {
        width: 32px;
        height: 32px;
        background: minus-ta.pressed
            ? Theme.surface-card.brighter(20%)
            : Theme.surface-card;
        border-radius: 16px;
        opacity: root.value <= root.min ? 0.4 : 1.0;
        minus-ta := TouchArea {
            enabled: root.value > root.min;
            clicked => { root.value -= root.step; }
        }
        Text {
            text: "−";          // U+2212 minus sign, not hyphen
            color: Theme.text-primary;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    Rectangle {
        width: 56px;
        height: 32px;
        background: Theme.surface-card;
        border-radius: 8px;
        Text {
            text: "\{root.value}";
            color: Theme.text-primary;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }

    Rectangle {
        width: 32px;
        height: 32px;
        background: plus-ta.pressed
            ? Theme.surface-card.brighter(20%)
            : Theme.surface-card;
        border-radius: 16px;
        opacity: root.value >= root.max ? 0.4 : 1.0;
        plus-ta := TouchArea {
            enabled: root.value < root.max;
            clicked => { root.value += root.step; }
        }
        Text {
            text: "+";
            color: Theme.text-primary;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

**Don't ship until a consumer lands.** The first consumer (likely Phase 18's snapshot-secs configurator if it ever grows a UI surface) drives the API shape. Keep this snippet as a reference; revisit when needed.

---

## Section C — Backlog status

| Util | Status | Trigger |
|---|---|---|
| `IconAndText` | **Ship now** (this guide) | 4+ inline consumers exist |
| `InfoBanner` | **Ship now** (this guide) | 3+ inline consumers exist |
| `ValueEditChip` | Defer — no consumer | Phase 18 snapshot-secs UI |
| `TextEditField` | Defer — only 1 consumer | Phase 24 receiver rename UI |
| `MultiLineTextField` | Defer — no consumer | Phase 25 macro description (future) |
| `InlinePicker` | Defer — Phase 14/15/16 cyclers don't use ComboBox; cyclers have richer behaviour | Reactive ComboBox demand from a new phase |
| `FormFieldError` | Defer — no consumer ships form validation yet | Phase 24 rename validation |
| `DraggableItemPrefix` | Defer — Phase 17 / Phase 25 use ▲▼ buttons not drag handles | Real drag-and-drop in Phase 8 |
| `UrlsView` | Defer — Phase 21 already inlines a list of `TextButton`s with link styling | Reactive URL list driven by Bridge |
| `format-duration(int) -> string` | Defer — only 2 sites and Slint pure-function cross-file imports are awkward | A 4th consumer |

The "trigger" column is your re-evaluation cue: when a phase expands to add the listed feature, this guide is the first place you check before writing inline copy.

---

## Section D — Anti-patterns

### D1. Don't extract a util that's only referenced from one place

If `pages/network_page.slint` has a `NetworkInterfaceRow` sub-component and no other page uses it, **leave it inline**. Sub-component is fine; full extraction to `components/` adds friction without a payoff.

### D2. Don't extract a util that the consumers customise heavily

Phase 14/15's settings rows (`SettingsValueRow`, `SettingsToggleRow`, `SettingsSliderRow`) are extracted because every consumer uses them with the same shape. The bitrate edit form (Phase 16) doesn't fit the row pattern — it has a unique LineEdit + slider stacked layout. Don't try to make `SettingsRow` parameterised enough to handle every form. Forms with custom layouts stay inline.

### D3. Don't extract Slint mechanism wrappers

Slint already has `TouchArea`, `Timer`, `ScrollView`, `Rectangle.border-*`, layout containers. Don't write `MyButton inherits Rectangle { /* TouchArea + Text + animation */ }` if `PrimaryButton` already exists. Don't write `MyTimer inherits Rectangle { Timer { ... } }` because `Timer` is already a primitive.

The Moblin `View/Utils/SwipeLeftToDeleteButtonView.swift` is **not** a candidate for `components/` — Slint expresses the swipe gesture natively via `TouchArea.moved` events plus animated transforms. Implement the gesture inline at the consumer.

### D4. Don't extract behavioural helpers

Slint pure functions don't import cleanly across files. The HH:MM:SS `format-duration` helper duplicated across Phase 23 + Phase 20 is **fine** as duplicate. Trying to make it shared via a `helpers.slint` module that exports pure functions is more friction than the savings warrant.

If/when Slint gains module-level pure-function exports (recent versions are improving this), revisit. For now: small duplicated helpers > shared util module.

### D5. Don't extract for "future-proofing"

Every component in `components/` should pay for itself today. A util added because "we might need it" tends to grow stale, drift from `Theme.*` token updates, and confuse future readers who can't find a consumer.

---

## Sanity grep before commit

```sh
# 1. Both shipped utils exist.
grep -n 'export component IconAndText' senders/android/ui/components/icon_and_text.slint
# Expected: 1 match.
grep -n 'export component InfoBanner\|export enum BannerSeverity' \
    senders/android/ui/components/info_banner.slint
# Expected: 2 matches.

# 2. At least one consumer migrated for each (start with Phase 19 → InfoBanner).
grep -n 'InfoBanner' senders/android/ui/pages/backup_reset_page.slint
# Expected: 2 matches (import + instantiation).

# 3. Theme tokens, no raw hex except documented severity colors.
grep -nE '#[0-9a-fA-F]{6}' senders/android/ui/components/icon_and_text.slint
# Expected: 0 matches.
grep -nE '#[0-9a-fA-F]{6}' senders/android/ui/components/info_banner.slint
# Expected: 3 matches (success/warning/error placeholders pending Theme.* tokens).

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/components/icon_and_text.slint \
        senders/android/ui/components/info_banner.slint \
        senders/android/ui/pages/backup_reset_page.slint
git status
git commit -m "feat(slint-ui): Phase 27 — extract IconAndText + InfoBanner; migrate Phase 19 banner"
```

Follow-up commits (separate PRs) migrate Phase 22's Wi-Fi Aware banner and Phase 20's clear-history toast to `InfoBanner`, and Phase 26's element-state table to `IconAndText`.

---

## Gotchas (Phase 27 specific)

### Gotcha 46 — `inherits HorizontalLayout` is the right shape for layout utils

**Symptom:** consumer wraps `IconAndText` in another `HorizontalLayout` and the spacing breaks.

**Cause:** if `IconAndText` were `inherits Rectangle`, every consumer would need to wrap it in a layout to get sibling spacing. By inheriting from `HorizontalLayout` directly, consumers can place the util as a child of any layout container and it integrates flush.

**Fix (already in this guide):** `IconAndText inherits HorizontalLayout` is correct. Same rationale: the util **is** a row, not a thing that sits in a row.

### Gotcha 47 — `in-out property <bool> visible <=> root.flag` requires both ends to be `in-out`

**Symptom:** Slint compiler error `cannot two-way-bind 'in' property` or the timer doesn't drive the visibility back to false on the parent.

**Cause:** `<=>` requires both properties on both sides to be `in-out`. If the consumer declared `property <bool> banner-visible: false;` (no `in-out`), the binding fails or becomes one-way.

**Fix:** ensure consumers declare their bound flag as `in-out property <bool> banner-visible: false;`. Or use the simpler form: `visible: root.banner-visible;` + a callback `on-hidden => { root.banner-visible = false; }` from the util when its Timer triggers. The `<=>` form is cleaner if both sides cooperate.

### Gotcha 48 — `clip: true;` is required for animated height collapse

**Symptom:** during the height-collapse animation, the banner's text bleeds out below the shrinking rectangle, looking like floating text.

**Cause:** Slint `Rectangle` doesn't clip children by default. When the parent's height shrinks below the child Text's height, the Text overflows.

**Fix (already in this guide):** add `clip: true;` to the banner `Rectangle`. The Text now clips to the shrinking parent. Same trick used in Phase 26's filter-chip-driven row collapse.

### Gotcha 49 — Util enum variants must use kebab-case in the `enum` body

**Symptom:** Slint compiler error `unexpected token 'BannerSeverity.SUCCESS'`.

**Cause:** Slint enums use kebab-case (`success`, `warning`) — not SCREAMING_SNAKE. Same convention as the rest of the language.

**Fix:** `BannerSeverity.success`, not `BannerSeverity.SUCCESS`. The guide's snippet is correct.

### Gotcha 50 — `Theme.success / warning / error` tokens don't exist yet

**Symptom:** `unknown property 'success' on Theme global`.

**Cause:** `theme.slint` was last touched in Phase 2 with `text-primary`, `text-secondary`, `accent-active`, `surface-*`. Severity colours are inline hex in the current draft.

**Fix:** add tokens to `theme.slint`:

```diff
 export global Theme {
     ...
+    out property <color> success: #2e7d32;
+    out property <color> warning: #ed6c02;
+    out property <color> error:   #c62828;
 }
```

then replace the inline hex in `info_banner.slint` and Phase 20's `status-color`. This is a separate small PR that lands after the util extraction.

---

## Exit criteria checklist

For each util shipped (`IconAndText`, `InfoBanner`):

- [x] File exists under `senders/android/ui/components/`.
- [x] Component is `< 80` lines.
- [x] Uses only `Theme.*` tokens (or documented severity hex placeholders).
- [x] At least one already-shipped consumer migrated to use the util.
- [x] At least one anticipated future consumer documented in this guide.
- [x] `slint-viewer` opens the file standalone without errors.
- [x] `cargo build -p android-sender` passes.

For the guide as a whole:

- [x] Backlog table is up to date (each row has a status + trigger).
- [x] Anti-patterns section reflects the latest `senders/android/ui/` state.
- [x] All cited Slint doc paths verified to exist on disk.

---

## When Phase 8 reactivates

Phase 27 is mostly Slint-internal — Rust doesn't see most utils. The exceptions:

- `InfoBanner.message` becomes Bridge-driven for cross-page banners (e.g. "Connection lost" surfaced from anywhere). Bridge gets `in property <string> banner-message;` + `in-out property <bool> banner-visible;`. Util consumes these.
- `IconAndText.icon` migrates from Unicode glyph to bundled raster icons via `Image` element. Asset paths come from a Theme-level icon manifest, not from Bridge.
- `ValueEditChip` ships once an editable numeric Bridge property exists (e.g. snapshot-secs configurator).

---

## Slint-doc references used

- **`inherits HorizontalLayout` (composition utils)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **`Image` element + asset references** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx`.
- **`color` type, `#rrggbb` hex, named colors (`white`)** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **`animate <prop> { duration, easing }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **`enum` declaration syntax + kebab-case variants** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`<=>` two-way binding requires `in-out` both sides** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`clip: true;`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`TouchArea.enabled / pressed`** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`if cond: Element { ... }` conditional element** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **String interpolation `"\{n}"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.

---

## What's NOT in this guide

- **Speculative utils with no existing consumer.** `ValueEditChip`, `TextEditField`, `MultiLineTextField`, `FormFieldError`, `DraggableItemPrefix`, `UrlsView`, `InlinePicker` — sketched in Section C, deferred until a consumer lands.
- **Migration of every existing inline consumer in one PR.** Section A4 covers the one-consumer-at-a-time discipline.
- **Cross-file pure function reuse for `format-duration`.** Section D4 covers the rationale.
- **Theme token additions.** Section D5 + Gotcha 50 — separate small PR.
- **`slint-viewer` setup.** Out of scope; the gate is "compiles in isolation"; how you verify is up to you.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-27-utils-backlog.md
[p19]: ./PHASE-19-reimplement-instructions.md
[p20]: ./PHASE-20-reimplement-instructions.md
[p22]: ./PHASE-22-reimplement-instructions.md
[colors]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx
[image]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[animation]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx

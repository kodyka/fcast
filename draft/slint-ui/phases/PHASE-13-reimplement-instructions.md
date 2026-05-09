# Phase 13 — Status Badges Row reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-13-status-badges-row.md`][spec] to the current `senders/android` tree.
**Goal:** add a compact right-aligned row of three status badges (network type, thermal state, battery percentage) sitting above `CastControlBar` in `MainWindow`. **No real telemetry** — badges read from inline `mock-*` properties on the component until Phase 8 wires Android `BatteryManager` / `PowerManager.ThermalEventListener` / `ConnectivityManager`.
**Scope:** Slint UI only. **No Rust changes.** One new component file (~80 lines) + one-line embed in `main.slint`.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-13-status-badges-row.md

> **Read [`PHASE-12-reimplement-instructions.md`][p12] first** for the declaration-order = z-order rule. Phase 13's `StatusBadgesRow` sits in `main.slint`'s outer `VerticalLayout` between the panel-routing layer and `CastControlBar` — it's not an overlay, it's a flow element. The placement is therefore order-sensitive, same as Phase 12's StatusOverlay.

[p12]: ./PHASE-12-reimplement-instructions.md

---

## Why this guide exists

Phase 13 is the smallest UI-only phase after Phase 12. Two distinguishing patterns:

1. **Internal `Badge` sub-component.** The file declares a non-exported `component Badge` plus the exported `StatusBadgesRow`. Same pattern as Phase 17's internal `QuickActionRow`. Internal components reduce duplication without polluting the import graph.
2. **Severity-keyed colour selection without a string-comparison ladder.** The thermal badge uses a triple-ternary on `mock-thermal == "..."`. This guide documents the pure-function alternative (cleaner if a fourth severity is added) and pins down the spec's "use string equality, not enum" decision.

After Phases 5 + 6 + 7 merge:

- `senders/android/ui/main.slint` exists with structure: outer `Window` → `VerticalLayout` → page chassis (`if Bridge.app-state == ...:` chain) → panel-routing (`if Bridge.active-panel == ...:` chain) → `CastControlBar`.
- No status badges anywhere; the casting page only has `StatusOverlay` (different element).
- `theme.slint` exposes `text-secondary`, `text-primary`, `error`, `surface-overlay`, `radius-pill` (or `radius-card` if `radius-pill` isn't yet defined; see Gotcha 67).

Phase 13 adds **one new file** plus a **one-line edit to `main.slint`**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'StatusBadgesRow\|status_badges' senders/android/ui/

# Theme tokens we use (verify they exist post-Phase 2):
grep -nE 'text-secondary|text-primary|surface-overlay|radius-pill|radius-card|error|warning|font-size-label|padding-screen' \
    senders/android/ui/theme.slint | head -10
# Expected: at least 6 matches.

# main.slint exists and has CastControlBar:
grep -n 'CastControlBar' senders/android/ui/main.slint
# Expected: 2 matches (1 import + 1 instantiation).
```

After this guide is applied:

```sh
grep -n 'export component StatusBadgesRow\|component Badge' \
    senders/android/ui/components/status_badges.slint
# Expected: 2 matches (1 internal Badge + 1 exported StatusBadgesRow).

grep -n 'StatusBadgesRow' senders/android/ui/main.slint
# Expected: 2 matches (1 import + 1 instantiation, immediately above CastControlBar).
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-13-status-badges
cargo check -p android-sender
```

---

## Step 1 — Create `components/status_badges.slint`

**File:** `senders/android/ui/components/status_badges.slint` (new)

Internal `Badge` (pill background + glyph + value) + exported `StatusBadgesRow` (right-aligned `HorizontalLayout` of three Badges with severity logic).

### New file

```slint
// status_badges.slint — Compact pill row of network / thermal / battery badges.
//
// UI-only stub. Each badge reads from a top-level mock-* property; severity
// branches on string equality (mock-thermal) or numeric threshold
// (mock-battery-pct).
//
// Phase 8 migration replaces stub properties with Bridge:
//   mock-battery-pct → Bridge.battery-pct       (BatteryManager.EXTRA_LEVEL)
//   mock-charging    → Bridge.charging          (BatteryManager.EXTRA_PLUGGED)
//   mock-thermal     → Bridge.thermal-state     (PowerManager.ThermalEventListener)
//   mock-network     → Bridge.network-type      (ConnectivityManager.activeNetworkInfo)
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx

import { Theme } from "../theme.slint";

// Internal sub-component — not exported. Two Text children inside a pill.
component Badge inherits Rectangle {
    in property <string> icon-glyph;
    in property <string> value;
    in property <color>  fg: Theme.text-secondary;

    height: 22px;
    border-radius: self.height / 2;
    background: Theme.surface-overlay;

    HorizontalLayout {
        padding-left:  8px;
        padding-right: 8px;
        spacing: 4px;
        Text {
            text: root.icon-glyph;
            color: root.fg;
            font-size: Theme.font-size-label;
            vertical-alignment: center;
        }
        Text {
            text: root.value;
            color: root.fg;
            font-size: Theme.font-size-label;
            vertical-alignment: center;
        }
    }
}

export component StatusBadgesRow inherits Rectangle {
    in-out property <int>    mock-battery-pct: 87;
    in-out property <bool>   mock-charging:    false;
    in-out property <string> mock-thermal:     "Nominal";
    in-out property <string> mock-network:     "Wi-Fi";

    height: 28px;
    background: transparent;

    HorizontalLayout {
        alignment: end;
        spacing: 6px;
        padding-right: Theme.padding-screen;

        Badge {
            icon-glyph: "📶";
            value: root.mock-network;
        }

        Badge {
            icon-glyph: root.mock-thermal == "Critical" ? "🔥" : "🌡";
            value: root.mock-thermal;
            // `*-fg` severity tokens — bright variants suitable as text on
            // surface-overlay; `Theme.error` / `Theme.warning` are
            // background fills.
            fg: root.mock-thermal == "Critical" ? Theme.error-fg
              : root.mock-thermal == "Serious"  ? Theme.warning-fg
              :                                    Theme.text-secondary;
        }

        Badge {
            icon-glyph: root.mock-charging ? "⚡" : "🔋";
            value: "\{root.mock-battery-pct}%";
            fg: root.mock-battery-pct < 20 ? Theme.error-fg : Theme.text-secondary;
        }
    }
}
```

### Why each piece

- **Internal `component Badge inherits Rectangle`** without `export` — local sub-component, only `StatusBadgesRow` consumes it. Slint allows multiple component declarations per file; only `export` makes them visible to other files. See [file.mdx][file].
- **`border-radius: self.height / 2;`** — pill shape regardless of how `height` changes. If a future variant ships `height: 24px` for a denser layout, the radius adapts.
- **`background: Theme.surface-overlay;`** — consistent with the existing status-pill scheme (Phase 5 / Phase 20). If `surface-overlay` doesn't yet exist, add it in `theme.slint` first (see Gotcha 67).
- **`HorizontalLayout { alignment: end; ... }`** — right-aligns the three badges. The row's `width` defaults to filling the parent; `alignment: end` packs children to the right edge. Per [positioning-and-layouts.mdx][positioning].
- **`"\{root.mock-battery-pct}%"`** string interpolation — Slint formats `int` natively. Concatenation via `+` would also work but interpolation is the canonical form. Per [expressions-and-statements.mdx][expressions].
- **Triple-ternary on thermal** — Slint has no `match`/`switch`. Same pattern as Phase 20's `status-color`. For a fourth severity, the chain grows linearly; if it grows past four, hoist to a `pure function thermal-fg(thermal: string) -> color`.
- **Battery `fg` threshold at 20%** — matches platform convention. If the design wants 15% or 10%, change the literal — don't introduce a property unless multiple consumers need different thresholds.
- **Emoji glyphs** — same convention as Phase 12 (`● LIVE`). Phase 8 + Phase 27 may swap to bundled raster icons via `Image { source: @image-url(...) }`.

### Build check

```sh
cargo check -p android-sender
slint-viewer senders/android/ui/components/status_badges.slint   # optional standalone preview
```

---

## Step 2 — Embed in `MainWindow`

**File:** `senders/android/ui/main.slint`

The status badges row sits **immediately above** `CastControlBar`. Order-sensitive: declaration order is z-order, and the badges should not overlap `CastControlBar` content (they're a separate row in the flow, not an overlay).

### Diff

```diff
+import { StatusBadgesRow } from "components/status_badges.slint";
 import { CastControlBar }  from "components/cast_control_bar.slint";
 ...
```

```diff
     VerticalLayout {
         // ...page chassis...
         // ...panel-routing chain...

+        StatusBadgesRow { }
         CastControlBar  { }
     }
```

### Why each piece

- **Above `CastControlBar`, not inside it** — keeps `CastControlBar`'s width/layout unchanged. If integrated as a child, the bar's existing positioning (Phase 4) breaks. Separate-row pattern is also easier to migrate to a different position later (e.g. inside a phone notch / status-bar area).
- **No properties bound at the embed site** — defaults render the "good" state (charging false, battery 87%, thermal Nominal, network Wi-Fi). Phase 8 binds each property to its Bridge counterpart.
- **Don't set explicit `width:` / `height:`** — `inherits Rectangle` + `height: 28px` is already declared inside the component. Setting them here would shadow the component's declaration and invite future drift.

### Build check

```sh
cargo build -p android-sender
```

---

## Step 3 — Severity preview matrix

Test each severity branch by temporarily editing the embed site, run `slint-viewer`, then revert. **Don't commit the test edits.**

| Test | Edit | Expected |
|---|---|---|
| Low battery | `mock-battery-pct: 8;` | Battery glyph + "8%" rendered in `Theme.error-fg` (bright red). |
| Charging | `mock-charging: true;` | Glyph flips from 🔋 to ⚡; colour stays text-secondary. |
| Critical thermal | `mock-thermal: "Critical";` | Thermal glyph flips to 🔥, fg in `Theme.error-fg` (bright red). |
| Serious thermal | `mock-thermal: "Serious";` | Thermal stays 🌡 but fg in `Theme.warning-fg` (bright amber). |
| Cellular | `mock-network: "5G";` | Value renders "5G" instead of "Wi-Fi"; no other change. |

Use `git stash` per Phase 10 § Gotcha 65 so test edits don't accidentally land:

```sh
git stash push -- senders/android/ui/main.slint
# Edit StatusBadgesRow embed for the test.
slint-viewer senders/android/ui/main.slint
# Verify visually.
git stash pop
```

---

## Sanity grep before commit

```sh
# 1. Component file exists with both internal Badge and exported StatusBadgesRow.
grep -n 'component Badge inherits\|export component StatusBadgesRow' \
    senders/android/ui/components/status_badges.slint
# Expected: 2 matches.

# 2. Imported and instantiated in main.slint.
grep -n 'StatusBadgesRow' senders/android/ui/main.slint
# Expected: 2 matches.

# 3. Declared above CastControlBar in main.slint (order-sensitive).
awk '/StatusBadgesRow *{/{sb=NR} /CastControlBar *{/{cb=NR} END{
    if (sb && cb && sb < cb) print "OK: StatusBadgesRow declared above CastControlBar";
    else print "FAIL: order broken or one missing";
}' senders/android/ui/main.slint

# 4. No new top-level theme tokens snuck in:
grep -nE '#[0-9a-fA-F]{6}' senders/android/ui/components/status_badges.slint
# Expected: 0 matches (all colours via Theme.*).

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/components/status_badges.slint senders/android/ui/main.slint
git status
# Expected (2 files):
#   modified:   senders/android/ui/main.slint
#   new file:   senders/android/ui/components/status_badges.slint
git commit -m "feat(slint-ui): Phase 13 — status badges row above CastControlBar (UI-only)"
```

---

## Gotchas (Phase 13 specific)

### Gotcha 66 — Internal `component Badge` clashes with same-named imports

**Symptom:** Slint compiler error `duplicate component name 'Badge'` if another file already exports a `Badge`.

**Cause:** Slint's component namespace is per-file; a `component Badge` in `status_badges.slint` doesn't conflict with one in `other.slint` unless both are imported into the same file. But if the project later exports `Badge` from a shared utils module, every consumer file has to choose which one wins.

**Fix:** keep `Badge` non-exported. If a future utils consumer needs it, rename to `StatusPillBadge` to avoid colliding with a future generic `Badge` component.

### Gotcha 67 — `Theme.radius-pill` may not exist yet

**Symptom:** Slint compiler error `unknown property 'radius-pill' on Theme global`.

**Cause:** the spec's snippet uses `border-radius: Theme.radius-pill;`. The current `theme.slint` (post-Phase 2) has `radius-card` but may not have `radius-pill`.

**Fix (already in this guide):** use `border-radius: self.height / 2;` instead — derives the pill radius from height with no theme dependency. If you'd prefer to canonicalise via theme, add the token in a separate one-line PR:

```diff
 export global Theme {
     ...
+    out property <length> radius-pill: 999px;
 }
```

then `border-radius: Theme.radius-pill;` works. The `999px` literal is a "max" sentinel — Slint clamps `border-radius` at `min(width, height) / 2`.

### Gotcha 68 — Emoji glyphs render inconsistently across font fallbacks

**Symptom:** the 📶 / 🔋 / ⚡ glyphs render as monochrome boxes or different shapes on Android vs. desktop preview.

**Cause:** Slint uses the system font fallback chain. Android's emoji set varies by Material font version; some emojis render as black-and-white outline rather than colour.

**Fix:** for design preview, accept the variance — `slint-viewer` on Linux desktop will render different glyphs than Android. **Phase 8 migration target:** swap to bundled SVG/PNG icons via `Image { source: @image-url("../assets/icons/wifi.svg"); }`. Until then, document the variance in the PR description so reviewers know the on-device look is the source of truth.

### Gotcha 69 — Thermal `Theme.warning` may not exist yet

**Symptom:** Slint compiler error `unknown property 'warning' on Theme global`.

**Cause:** post-Phase 2 theme has `error` but not `warning`. The amber severity colour is a Phase-27 deferred token (Phase 27 §gotcha 50).

**Fix:** add `warning` to `theme.slint`:

```diff
 export global Theme {
     ...
+    out property <color> warning: #ed6c02;
 }
```

This single-line theme PR is shared with Phase 20 (cast history pill), Phase 27 (`InfoBanner.severity == warning`), and Phase 26 (debug log Warn level chip). Land the theme PR first, then Phase 13 can reference `Theme.warning` cleanly.

### Gotcha 70 — `alignment: end` on the row's HorizontalLayout, not the row itself

**Symptom:** badges render left-aligned despite the spec showing right-aligned.

**Cause:** confusing `alignment: end` on the outer `Rectangle` (which has no effect — Rectangle doesn't lay out children) with `alignment: end` on the inner `HorizontalLayout` (which packs the layout's children to the end edge).

**Fix (already in this guide):** declare `alignment: end` on the `HorizontalLayout`, **not** the outer `Rectangle`. The outer `Rectangle` is just a transparent background.

---

## Exit criteria checklist

- [ ] `components/status_badges.slint` exists with internal `component Badge` + exported `StatusBadgesRow`.
- [ ] `Badge` height is 22px; `border-radius` is `self.height / 2` (true pill shape).
- [ ] `StatusBadgesRow` height is 28px; background `transparent`.
- [ ] `HorizontalLayout` uses `alignment: end` and `padding-right: Theme.padding-screen`.
- [ ] All three badges (network, thermal, battery) render in nominal state (Wi-Fi / Nominal / 87%).
- [ ] Battery glyph flips between 🔋 and ⚡ based on `mock-charging`.
- [ ] Battery fg goes red (`Theme.error-fg`) when `mock-battery-pct < 20`.
- [ ] Thermal glyph flips between 🌡 and 🔥 when `mock-thermal == "Critical"`.
- [ ] Thermal fg is amber (`Theme.warning-fg`) for `"Serious"` and red for `"Critical"`.
- [ ] `main.slint` imports `StatusBadgesRow` and instantiates it immediately above `CastControlBar`.
- [ ] No raw hex colours in `status_badges.slint` — all via `Theme.*`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
 export global Bridge {
     ...
+    in property <int>    battery-pct;
+    in property <bool>   charging;
+    in property <string> thermal-state;     // Nominal / Fair / Serious / Critical
+    in property <string> network-type;      // Wi-Fi / 5G / LTE / Ethernet / None
 }
```

```rust
// In lib.rs:
ui.global::<Bridge>().set_battery_pct(battery_manager.level());
ui.global::<Bridge>().set_charging(battery_manager.is_charging());
ui.global::<Bridge>().set_thermal_state(power_manager.thermal_state().into());
ui.global::<Bridge>().set_network_type(connectivity_manager.active_type().into());
```

```diff
     in-out property <int>    mock-battery-pct: 87;
     in-out property <bool>   mock-charging:    false;
     in-out property <string> mock-thermal:     "Nominal";
     in-out property <string> mock-network:     "Wi-Fi";
+    // After Phase 8: replace the four mock-* properties with bindings:
+    in property <int>    battery-pct  <=> Bridge.battery-pct;
+    in property <bool>   charging     <=> Bridge.charging;
+    in property <string> thermal      <=> Bridge.thermal-state;
+    in property <string> network      <=> Bridge.network-type;
```

The component's internal logic doesn't change; only the source of the values does.

Future enhancements (deferred):
- **Tap-to-expand thermal sheet** — opens a panel showing thermal history graph.
- **Mute mic indicator** — Phase 14 audio reactivates → fourth badge for `Bridge.audio-muted`.
- **Network speed display** — kbps under the network type, requires periodic Rust polling.

---

## Slint-doc references used

- **Multiple component declarations per file (internal `component` without `export`)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`HorizontalLayout { alignment, spacing, padding-* }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **`Rectangle { border-radius }` + `self.height` reference** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`color` type, `Theme.*` token references** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **String interpolation `"\{n}%"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **Ternary expressions for severity branching** — same.
- **`background: transparent`** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx` (named colour).
- **`Theme` global tokens** — FCast file `senders/android/ui/theme.slint` (Phase 2).
- **`CastControlBar`** — FCast component `senders/android/ui/components/cast_control_bar.slint` (Phase 4).

---

## What's NOT in this guide

- **Real Android `BatteryManager` / `PowerManager.ThermalEventListener` / `ConnectivityManager` integration.** Phase 8.
- **Tap-to-expand thermal detail sheet.** Out of scope; would be a separate Panel variant.
- **Mute-mic / cast-active fourth badge.** Out of scope; revisit when Phase 14 lands the audio-muted property.
- **Bundled raster icons** instead of emoji glyphs. Phase 27 (`IconAndText` + raster asset migration).
- **`@tr(...)` wrapping** of `"Wi-Fi"`, `"Nominal"`, etc. Phase 9 sweep.
- **Animation when a severity transitions** (e.g. battery dropping below 20%). Out of scope; would use `animate <prop> { ... }` on `Badge.fg`.
- **Status badges row inside the casting page rather than at the global MainWindow level.** Out of scope; revisit if the design changes.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-13-status-badges-row.md
[p12]: ./PHASE-12-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx

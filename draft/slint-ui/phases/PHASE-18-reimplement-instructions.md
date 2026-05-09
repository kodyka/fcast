# Phase 18 — Privacy & Lifecycle Modes reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-18-privacy-lifecycle-modes.md`][spec] to the current `senders/android` tree.
**Goal:** add three full-screen overlays (`LockOverlay`, `StealthOverlay`, `SnapshotCountdown`) that render on top of the entire `MainWindow` based on a new `Bridge.lifecycle: LifecycleMode` property. Add settings rows under a new `PRIVACY` section to enter each mode. Lock overlay implements a long-press-to-unlock animation (1.5s ring fill).
**Scope:** Slint UI only. **No Rust changes.** No `KeyguardManager`, no `FLAG_SECURE`, no real cast-start triggered by countdown.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-18-privacy-lifecycle-modes.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] and [`PHASE-23-reimplement-instructions.md`][p23] first.** Phase 23 introduces the `Timer` element used here; Phase 14 introduces the panel-overlay routing pattern. The new things in Phase 18: full-window overlays layered above `MainWindow`'s normal content (not just overlays inside the existing `Panel.x` layer), an animated long-press progress ring driven by `pressed-since` time delta, and a numeric countdown using `animate <prop>: <duration> ease;` directly on a Text width.

[p14]: ./PHASE-14-reimplement-instructions.md
[p23]: ./PHASE-23-reimplement-instructions.md

---

## Why this guide exists

Phase 18 is the first phase to add **non-Panel overlays**. Phases 14-17, 21-23, 26 all extend the `Panel` enum and route inside the existing overlay layer; their overlays *replace* the normal content. Phase 18's lifecycle modes are different:

- **`Bridge.lifecycle: LifecycleMode`** is orthogonal to `Bridge.active-panel`. A user can be in `Panel.audio` (Audio settings) when the Lock UI activates — both should be visible (Audio settings underneath, Lock overlay on top), and on unlock, the Audio panel should still be there.
- **Each overlay is a separate file** (`components/lock_overlay.slint`, `stealth_overlay.slint`, `snapshot_countdown.slint`) rather than living inside a shared `pages/` directory, because they're not pages — they're full-window scrims that don't follow the Phase-7 panel chrome.
- **Layering order matters.** `MainWindow` renders normal content (cast screen + active panel) first, then the lifecycle overlay on top, so the Slint conditional `if Bridge.lifecycle == LifecycleMode.lock-screen: LockOverlay { ... }` must sit **after** the panel layer in `main.slint`.

The most subtle parts:

1. **Long-press unlock ring** — animated progress around a glyph. Slint's `animate <prop>` clause on a property declaration makes this a one-liner once the timing source is correct. The trick is gating the timer to "TouchArea is currently pressed" without losing accumulated time on a brief release.
2. **Snapshot countdown** — pure numeric Timer + a single text element. Easier than the lock; mirrors Phase 23's elapsed counter but counting *down*.
3. **Stealth dismiss-on-tap** — full-window TouchArea + `clicked => { Bridge.lifecycle = LifecycleMode.normal; }`. Trivial.

After Phases 14 + 15 + 16 + 17 + 21 + 22 + 23 + 26 merge:

- `Panel { ..., quick-actions }`. Phase 18 does not add any `Panel` variant — overlays are gated by `Bridge.lifecycle`, a separate property.
- `bridge.slint` has `BitratePreset`, `NetworkInterface`, `LogEntry` structs and `RecordingState`, `LogLevel` enums. Phase 18 adds `LifecycleMode` enum + `Bridge.lifecycle` + `Bridge.mock-snapshot-secs`.
- `FullSettingsPage` does not have a `PRIVACY` section yet. Phase 18 inserts it.

This is **strictly additive** Slint work spread across **three existing files** plus **three new files**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'LifecycleMode\|LockOverlay\|StealthOverlay\|SnapshotCountdown\|Bridge\.lifecycle' \
    senders/android/ui/

# No PRIVACY section yet:
grep -n 'PRIVACY' senders/android/ui/pages/settings_page.slint
# Expected: (empty)
```

After this guide is applied:

```sh
grep -n 'export enum LifecycleMode\|in-out property <LifecycleMode> lifecycle' \
    senders/android/ui/bridge.slint
# Expected: 2 matches (enum decl + Bridge property).

grep -rn 'export component LockOverlay\|export component StealthOverlay\|export component SnapshotCountdown' \
    senders/android/ui/components/
# Expected: 3 matches.

grep -n 'LifecycleMode\.' senders/android/ui/main.slint
# Expected: 3+ matches (one per overlay-conditional layering).

grep -n 'PRIVACY\|LifecycleMode\.' senders/android/ui/pages/settings_page.slint
# Expected: 4 matches (section title + 3 lifecycle-set handlers).
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-18-lifecycle-overlays
cargo check -p android-sender
```

---

## Step 1 — Add `LifecycleMode` enum + Bridge properties in `bridge.slint`

```diff
+export enum LifecycleMode {
+    normal,
+    lock-screen,
+    stealth,
+    snapshot-countdown,
+}
+
 export struct NetworkInterface { ... }
```

```diff
 export global Bridge {
     ...
     in-out property <Panel>          active-panel: Panel.none;
+    in-out property <LifecycleMode>  lifecycle:    LifecycleMode.normal;
+    in-out property <int>            mock-snapshot-secs: 5;
     ...
 }
```

`mock-snapshot-secs` lives on Bridge (not on `SnapshotCountdown` itself) because the countdown's start value is configured from outside the overlay — the settings PRIVACY row that triggers it sets `mock-snapshot-secs = 5; lifecycle = LifecycleMode.snapshot-countdown;`. A future per-cast configurator could change the value before triggering.

---

## Step 2 — Create `components/lock_overlay.slint`

**File:** `senders/android/ui/components/lock_overlay.slint` (new)

A full-window scrim with a centered card. Long-press progress ring fills over 1.5s; on completion, lifecycle resets to normal.

### New file

```slint
// lock_overlay.slint — Full-window UI lock with long-press unlock.
//
// Layered above MainWindow's normal content via main.slint when
// Bridge.lifecycle == LifecycleMode.lock-screen. The overlay is a full
// black scrim plus a centered card; pressing the unlock glyph and
// holding for 1.5s clears Bridge.lifecycle.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx

import { Bridge, LifecycleMode } from "../bridge.slint";
import { Theme } from "../theme.slint";

export component LockOverlay inherits Rectangle {
    width: 100%;
    height: 100%;
    background: #000000d0;            // 0xd0 ≈ 80% opaque scrim

    // Hold-progress: 0.0 (just touched) → 1.0 (held 1.5s, unlock).
    // The Timer below ramps it while pressed; on release we clear it.
    property <float> hold-progress: 0.0;

    Rectangle {
        x: (parent.width  - self.width)  / 2;
        y: (parent.height - self.height) / 2;
        width: 240px;
        height: 240px;
        background: Theme.surface-card;
        border-radius: 16px;

        VerticalLayout {
            alignment: center;
            spacing: 16px;

            // ── Lock glyph + progress ring (parent for stacking) ─────────
            Rectangle {
                width: 96px;
                height: 96px;
                x: (parent.width - self.width) / 2;

                // Track ring (faint).
                Rectangle {
                    width: parent.width;
                    height: parent.height;
                    border-radius: parent.width / 2;
                    border-width: 4px;
                    border-color: Theme.surface-card.brighter(40%);
                }

                // Progress ring — implemented as a clipped arc using
                // overlapping Rectangles isn't ideal, but Slint exposes
                // Path with arc commands. For a UI-only build, a simpler
                // visual cue is acceptable: a Rectangle whose `width`
                // grows with hold-progress, providing a horizontal fill
                // progress bar across the bottom of the glyph card.
                //
                // If your Slint version (1.5+) exposes Path with `M`/`A`
                // arc commands, prefer the arc form — see the gotcha
                // section.

                // Lock glyph centered. Use an emoji-style fallback or a
                // Path; here a simple Text placeholder:
                Text {
                    text: "🔒";
                    color: Theme.text-primary;
                    font-size: 48px;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }

                hold-area := TouchArea {
                    // Reactive timer is gated on pressed; we cannot use a
                    // plain Slint animation because release-then-press
                    // should resume from 0 (not jump-to-current).
                }
            }

            // ── Horizontal hold-progress fill bar ────────────────────────
            Rectangle {
                height: 6px;
                width: 192px;
                x: (parent.width - self.width) / 2;
                background: Theme.surface-card.brighter(20%);
                border-radius: 3px;
                clip: true;

                Rectangle {
                    width: parent.width * root.hold-progress;
                    height: parent.height;
                    background: Theme.accent-active;
                    animate width { duration: 100ms; easing: ease; }
                }
            }

            Text {
                text: "UI Locked";
                color: Theme.text-primary;
                font-size: Theme.font-size-heading;
                horizontal-alignment: center;
            }

            Text {
                text: "Press and hold for 1.5s to unlock.";
                color: Theme.text-secondary;
                font-size: Theme.font-size-label;
                horizontal-alignment: center;
                wrap: word-wrap;
            }
        }
    }

    // ── Hold-progress driver ─────────────────────────────────────────────
    //
    // Slint's animation system can drive `hold-progress` to 1.0 while the
    // touch is pressed, but `animate <prop>` interpolates between
    // sequential property assignments — it does not natively expose a
    // "ramp while gated" form. The Timer-based driver below is the
    // explicit form that handles release/re-press cleanly.
    Timer {
        interval: 50ms;
        running: hold-area.pressed && root.hold-progress < 1.0;
        triggered => {
            // 1.5s hold = 30 ticks at 50ms = 1/30 step per tick.
            root.hold-progress += 1.0 / 30.0;
            if (root.hold-progress >= 1.0) {
                root.hold-progress = 1.0;
                Bridge.lifecycle = LifecycleMode.normal;
            }
        }
    }

    // Reset progress when finger lifts before completion. Done by binding
    // to hold-area.pressed transitioning false → true.
    //
    // Slint has no edge-trigger primitive; the cleanest pragma is a
    // `changed pressed => { ... }` callback inside the TouchArea, which
    // fires on every transition.
    //
    // Add to the hold-area := TouchArea body:
    //
    //   changed pressed => {
    //       if (!self.pressed) {
    //           root.hold-progress = 0.0;
    //       }
    //   }
    //
    // (Older Slint versions used `pressed-changed`; newer use
    // `changed pressed`. Check the version pinned in
    // senders/android/Cargo.toml.)
}
```

### Why each piece

- **Background `#000000d0`** — explicit RGBA hex. Slint accepts `#rrggbbaa`. The `d0` ≈ 208/255 ≈ 81.5% opaque. See [colors-and-brushes.mdx][colors].
- **Centered card via `x: (parent.width - self.width) / 2;`** — Slint anchoring expressions. Per [positioning-and-layouts.mdx][positioning].
- **Hold-progress driver: Timer at 50ms, gated by `hold-area.pressed`.** This is the cleanest cross-version idiom. A pure-`animate` form requires Slint 1.5+ with a sane release-state convention; the timer form works on 1.4 and 1.5+.
- **Edge-trigger reset on release** via `changed pressed` (newer) or `pressed-changed` (older) callback — checks `if (!self.pressed) { hold-progress = 0; }`. The exact callback name depends on Slint version; both forms are documented in [toucharea.mdx][toucharea]. Falling back to a Timer that sets the property to 0 when `running: !hold-area.pressed && hold-progress > 0;` also works but adds a second timer.
- **Animated horizontal fill bar** instead of arc-stroked progress ring — simpler. Slint's `Path` element with `M` (move-to) + `A` (arc) commands could draw a real ring, but the API surface is more involved. The horizontal bar with `animate width { duration: 100ms; ease; }` gives the user smooth visual feedback without committing to Path. See [animation.mdx][animation].
- **`Bridge.lifecycle = LifecycleMode.normal;`** on completion — clears the lock; the conditional in `main.slint` removes the overlay from the tree.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 3 — Create `components/stealth_overlay.slint`

**File:** `senders/android/ui/components/stealth_overlay.slint` (new)

A near-black full-window rectangle with a tiny "Tap to wake" hint at the bottom.

### New file

```slint
// stealth_overlay.slint — Near-black full-window scrim, tap to wake.
//
// Layered above MainWindow's normal content via main.slint when
// Bridge.lifecycle == LifecycleMode.stealth. Tapping anywhere clears
// Bridge.lifecycle. UI-only — does not engage SYSTEM_UI_FLAG_HIDE_NAVIGATION
// or any real screen-dimming behaviour.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx

import { Bridge, LifecycleMode } from "../bridge.slint";
import { Theme } from "../theme.slint";

export component StealthOverlay inherits Rectangle {
    width: 100%;
    height: 100%;
    background: #050505;

    TouchArea {
        clicked => { Bridge.lifecycle = LifecycleMode.normal; }
    }

    Text {
        text: "Tap to wake";
        color: #404040;
        font-size: Theme.font-size-label;
        horizontal-alignment: center;
        vertical-alignment: center;
        x: (parent.width  - self.width)  / 2;
        // Offset toward bottom (15% margin from bottom edge).
        y: parent.height * 0.85;
    }
}
```

### Why each piece

- **Single full-window `TouchArea`** — clicks anywhere dismiss the overlay. Per [toucharea.mdx][toucharea], a `TouchArea` without explicit width/height fills the parent rectangle.
- **Color-`#050505`** — near-black, not pure black, so the OLED screen still draws (looks "alive"); cosmetic.
- **Hint text at `y: parent.height * 0.85`** — fractional positioning. Slint allows arithmetic in property expressions. The text is dim (`#404040`) so it's barely visible, matching the "stealth" intent.

---

## Step 4 — Create `components/snapshot_countdown.slint`

**File:** `senders/android/ui/components/snapshot_countdown.slint` (new)

A big numeric countdown in the center of the screen. Counts down from `Bridge.mock-snapshot-secs` to 0; on reaching 0, clears the lifecycle.

### New file

```slint
// snapshot_countdown.slint — Pre-cast countdown overlay.
//
// Layered above MainWindow's normal content via main.slint when
// Bridge.lifecycle == LifecycleMode.snapshot-countdown. Counts down from
// Bridge.mock-snapshot-secs to 0; on 0, clears Bridge.lifecycle (UI-only
// — does NOT actually start a cast; Phase 8 wires that).
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx

import { Bridge, LifecycleMode } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "buttons.slint";

export component SnapshotCountdown inherits Rectangle {
    // Local copy of Bridge.mock-snapshot-secs so we can decrement without
    // mutating the Bridge value (which the settings row controls).
    property <int> remaining: Bridge.mock-snapshot-secs;

    width: 100%;
    height: 100%;
    background: #00000080;

    // 1-second tick decrementing remaining; on 0, clear lifecycle.
    Timer {
        interval: 1s;
        running: root.remaining > 0;
        triggered => {
            root.remaining -= 1;
            if (root.remaining <= 0) {
                Bridge.lifecycle = LifecycleMode.normal;
            }
        }
    }

    VerticalLayout {
        alignment: center;
        spacing: 32px;

        Text {
            text: "\{root.remaining}";
            color: Theme.text-primary;
            // Animated size pulse — Slint allows animating font-size.
            font-size: 144px;
            horizontal-alignment: center;
            animate font-size { duration: 200ms; easing: ease-out; }
        }

        Text {
            text: "Casting in…";
            color: Theme.text-secondary;
            font-size: Theme.font-size-heading;
            horizontal-alignment: center;
        }

        TextButton {
            label: "Cancel";
            x: (parent.width - self.width) / 2;
            clicked => { Bridge.lifecycle = LifecycleMode.normal; }
        }
    }
}
```

### Why each piece

- **`property <int> remaining: Bridge.mock-snapshot-secs;`** — initial-value binding. Slint reads `Bridge.mock-snapshot-secs` *once* at component instantiation; subsequent changes to the Bridge property do not propagate. This is the correct semantics: the settings page sets the duration, then triggers the overlay; the overlay then has its own copy to decrement. If you want continuous reactivity, declare the binding as `: Bridge.mock-snapshot-secs;` without `property` (a binding into an existing slot, not a new local) — but here we need the local mutability.
- **`Timer { interval: 1s; running: root.remaining > 0; triggered => { ... } }`** — same pattern as Phase 23's elapsed counter. Auto-stops at 0.
- **`animate font-size { duration: 200ms; }`** — a tiny pulse on each digit change, giving the countdown a sense of urgency. See [animation.mdx][animation]. Most numeric properties (including `font-size`) are animatable.
- **Cancel button below** — same cancel pattern as Phase 16's edit-page form. Returns to `LifecycleMode.normal` immediately.

---

## Step 5 — Layer overlays in `main.slint`

**File:** `senders/android/ui/main.slint`

The lifecycle overlays must sit **after** the panel layer in the file, so they paint on top.

### Diff

```diff
 import { QuickActionsPage }             from "pages/quick_actions_page.slint";
+import { LockOverlay }                  from "components/lock_overlay.slint";
+import { StealthOverlay }               from "components/stealth_overlay.slint";
+import { SnapshotCountdown }            from "components/snapshot_countdown.slint";
```

```diff
 export component MainWindow inherits Window {
     ...
     // ── Cast screen + control bar ────────────────────────────────────────
     ...

     // ── Panel overlay layer (Phases 7+) ──────────────────────────────────
     if Bridge.active-panel == Panel.settings:    FullSettingsPage { }
     ...
     if Bridge.active-panel == Panel.quick-actions: QuickActionsPage { }
+
+    // ── Lifecycle overlay layer (Phase 18) ──────────────────────────────
+    // These render *above* the Panel layer, so a Lock/Stealth/Countdown
+    // can engage while the user is on a settings sub-page.
+    if Bridge.lifecycle == LifecycleMode.lock-screen:        LockOverlay { }
+    if Bridge.lifecycle == LifecycleMode.stealth:            StealthOverlay { }
+    if Bridge.lifecycle == LifecycleMode.snapshot-countdown: SnapshotCountdown { }
 }
```

The `if` chain here is **not** mutually exclusive with the panel layer — both layers paint. The lifecycle layer uses different property gating (`Bridge.lifecycle`) than the panel layer (`Bridge.active-panel`).

---

## Step 6 — Add `PRIVACY` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Insert a new `PRIVACY` section (this guide places it immediately before `ABOUT & SUPPORT`, but `ADVANCED` works too — choose based on visual grouping).

### Diff

```diff
                 // ── Section: ADVANCED ─────────────────────────────────────
                 SettingsSection {
                     title: "ADVANCED";
                     ...
                 }
+
+                // ── Section: PRIVACY ──────────────────────────────────────
+                SettingsSection {
+                    title: "PRIVACY";
+                    SettingsValueRow {
+                        title: "Lock UI";
+                        value: "";
+                        clicked => {
+                            Bridge.lifecycle = LifecycleMode.lock-screen;
+                        }
+                    }
+                    SettingsValueRow {
+                        title: "Stealth mode";
+                        value: "";
+                        clicked => {
+                            Bridge.lifecycle = LifecycleMode.stealth;
+                        }
+                    }
+                    SettingsValueRow {
+                        title: "Cast with countdown";
+                        value: "";
+                        clicked => {
+                            Bridge.mock-snapshot-secs = 5;
+                            Bridge.lifecycle = LifecycleMode.snapshot-countdown;
+                        }
+                    }
+                }
+
                 // ── Section: ABOUT & SUPPORT ──────────────────────────────
                 SettingsSection {
                     title: "ABOUT & SUPPORT";
                     ...
                 }
```

Add the `LifecycleMode` import to `settings_page.slint`:

```diff
-import { Bridge, Panel } from "../bridge.slint";
+import { Bridge, Panel, LifecycleMode } from "../bridge.slint";
```

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. LifecycleMode + Bridge properties.
grep -n 'export enum LifecycleMode\|in-out property <LifecycleMode> lifecycle\|mock-snapshot-secs' \
    senders/android/ui/bridge.slint
# Expected: 3 matches.

# 2. Three overlay components.
grep -rn 'export component LockOverlay\|export component StealthOverlay\|export component SnapshotCountdown' \
    senders/android/ui/components/
# Expected: 3 matches (one per file).

# 3. main.slint layers all three lifecycle overlays.
grep -n 'LifecycleMode\.lock-screen\|LifecycleMode\.stealth\|LifecycleMode\.snapshot-countdown' \
    senders/android/ui/main.slint
# Expected: 3 matches.

# 4. PRIVACY section in FullSettingsPage.
grep -n 'PRIVACY\|LifecycleMode\.' senders/android/ui/pages/settings_page.slint
# Expected: 4 matches (1 section + 3 setters).

# 5. LockOverlay Timer + hold-area integration present.
grep -n 'Timer {\|hold-area\|hold-progress' senders/android/ui/components/lock_overlay.slint
# Expected: 4+ matches.

# 6. SnapshotCountdown Timer present.
grep -n 'Timer {' senders/android/ui/components/snapshot_countdown.slint
# Expected: 1 match.

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
#   new file:   senders/android/ui/components/lock_overlay.slint
#   new file:   senders/android/ui/components/stealth_overlay.slint
#   new file:   senders/android/ui/components/snapshot_countdown.slint
git commit -m "feat(slint-ui): Phase 18 — lock / stealth / snapshot countdown overlays (UI-only)"
```

---

## Gotchas (Phase 18 specific)

### Gotcha 28 — Lifecycle overlays must paint **above** panel overlays

**Symptom:** lock screen activates while in `Panel.audio`, but Audio settings render on top of the lock — defeating the lock.

**Cause:** Slint paints elements in source order within a parent. Whatever's declared *later* in `MainWindow` paints on top. If the lifecycle conditional is placed *before* the panel conditionals, panels paint on top.

**Fix (already in this guide's Step 5):** declare the panel layer first, then the lifecycle layer. Verify with grep — `grep -n 'LifecycleMode\.\|Panel\.' senders/android/ui/main.slint` and ensure the line numbers for `LifecycleMode.` are higher than for `Panel.`.

### Gotcha 29 — `animate <prop>` requires the property to be already declared

**Symptom:** Slint compiler error `cannot animate inline property`.

**Cause:** `animate width { duration: 100ms; }` must be a sibling block of an explicit `width: <expr>;` binding inside the same element. You cannot animate a property that doesn't have a binding, and you cannot animate a property declared on a parent element.

**Fix:** ensure the animated property has an explicit binding on the same element. The progress-bar fill in this guide is correct: the fill `Rectangle` has `width: parent.width * root.hold-progress;` followed by `animate width { ... }` in the same element body.

### Gotcha 30 — `changed pressed => { ... }` callback name varies by Slint version

**Symptom:** Slint compiler error `unknown callback 'changed pressed'` (older versions) or `unknown callback 'pressed-changed'` (newer).

**Cause:** Slint deprecated `<prop>-changed` in favor of `changed <prop> => { ... }` around 1.4. Both forms exist in the wild.

**Fix:** check the project's pinned Slint version (`grep '^slint = ' senders/android/Cargo.toml`). If 1.3 or older, use `pressed-changed`. If 1.4+, use `changed pressed`. The guide's snippet uses the newer form; adjust if your version differs.

### Gotcha 31 — `property <int> remaining: Bridge.mock-snapshot-secs;` is a one-shot read

**Symptom:** changing `Bridge.mock-snapshot-secs` from another part of the app while the countdown is running has no effect on the displayed value.

**Cause:** Slint `property <T> name: <expr>;` declarations create a *binding*. The binding is evaluated once initially and re-evaluated whenever its dependencies change. **However**, once the property is *written to* imperatively (in our case `root.remaining -= 1;` from the Timer), the binding is **broken** and replaced with the imperative value. Subsequent dependency changes are ignored.

**Fix:** this is the **desired** behaviour for the countdown — the overlay needs its own mutable counter. To re-establish reactivity (e.g. if a settings row resets the countdown mid-flight), explicitly write `root.remaining = Bridge.mock-snapshot-secs;` from the trigger that wants to reset it. The guide's snapshot-countdown trigger is the settings row's `clicked => { ... }`, which sets both `Bridge.mock-snapshot-secs = 5;` *before* `Bridge.lifecycle = ...;` — but because the conditional `if Bridge.lifecycle == ...: SnapshotCountdown { }` re-instantiates the component on the rising edge, the property's binding fires fresh each entry. So the pattern works correctly under normal use. Only edge case: keep `mock-snapshot-secs` constant during a countdown.

### Gotcha 32 — `Path` arc rings need Slint 1.5+ commands

**Symptom:** writing a `Path { commands: "M ... A ..."; }` to render a true progress ring fails on older Slint versions or renders incorrectly.

**Cause:** Slint's `Path` element accepts SVG-like path commands, but the supported subset varies by version. Arc (`A`) commands sometimes don't render anti-aliased on older builds.

**Fix:** either (a) use the horizontal-bar form in this guide (works everywhere), or (b) verify your Slint version supports `Path { commands: "M cx cy A r r 0 0 1 ..."; stroke: ...; stroke-width: 4px; }`. See [path.mdx][path] for the supported commands. The guide's bar form is the conservative choice.

---

## Exit criteria checklist

- [ ] `bridge.slint` adds `LifecycleMode` enum (4 variants), `Bridge.lifecycle: LifecycleMode`, `Bridge.mock-snapshot-secs: int`.
- [ ] `main.slint` layers `LockOverlay`, `StealthOverlay`, `SnapshotCountdown` **after** the panel layer.
- [ ] `LockOverlay`: full-window scrim with centered card, lock glyph, "UI Locked" text, "press and hold for 1.5s" hint, animated fill bar showing hold progress.
- [ ] Pressing and holding the lock glyph for 1.5s clears `Bridge.lifecycle`.
- [ ] Releasing before 1.5s resets the progress to 0.
- [ ] `StealthOverlay`: near-black full-window with dim "Tap to wake" hint at bottom; tapping anywhere clears `Bridge.lifecycle`.
- [ ] `SnapshotCountdown`: ticks 5 → 4 → 3 → 2 → 1 → 0 over 5 seconds; on 0, clears `Bridge.lifecycle`.
- [ ] Cancel button on countdown clears `Bridge.lifecycle` immediately.
- [ ] `FullSettingsPage`'s new `PRIVACY` section has 3 trigger rows.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    callback engage-lock();
+    callback engage-stealth();
+    callback start-snapshot-countdown(int);
```

- `engage-lock()` → Rust calls Android `KeyguardManager` to actually lock the device (or just sets a UI flag if the app stays foregrounded).
- `engage-stealth()` → Rust hides the cast preview UI from system screenshot/recording APIs by setting `WindowManager.LayoutParams.FLAG_SECURE`.
- `start-snapshot-countdown(secs)` → Rust starts the countdown and the actual cast on completion. The Slint side becomes a thin overlay; the Timer-based counter moves to Rust.
- The `Bridge.lifecycle` property remains as the gating signal; Rust pushes it.

---

## Slint-doc references used

- **`export enum LifecycleMode`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **`animate <property> { duration; easing; }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx`.
- **`changed <prop> => { ... }` callback / `<prop>-changed` (older)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` and `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`TouchArea.pressed`** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`Rectangle.background: #rrggbbaa`** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **Conditional element `if cond: Component { }` in Window body** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **Property binding broken on imperative write (`property <int> name: <expr>;` semantics)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`Path` arc commands** (referenced as alternative for the progress ring) — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/path.mdx`.
- **`TextButton`** — FCast component in `senders/android/ui/components/buttons.slint`.

---

## What's NOT in this guide

- **Real `KeyguardManager` / device lock integration.** Phase 8.
- **`FLAG_SECURE` / screenshot-blocking application.** Phase 8.
- **Inactivity-driven auto-stealth.** Out of scope; would require Rust-side activity-tracking timer.
- **Real cast-start triggered by countdown.** Phase 8.
- **Configurable countdown duration via slider.** Out of scope; `mock-snapshot-secs` is settable but no UI exposes the setter.
- **True arc progress ring on the lock overlay.** Defer to polish phase or Slint 1.5+ migration.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-18-privacy-lifecycle-modes.md
[p14]: ./PHASE-14-reimplement-instructions.md
[p23]: ./PHASE-23-reimplement-instructions.md
[colors]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[animation]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx
[toucharea]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx
[path]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/path.mdx

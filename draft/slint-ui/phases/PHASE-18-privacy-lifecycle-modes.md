# Phase 18 — Privacy & Lifecycle Modes Placeholder

> Three lifecycle overlays that apply on top of the running app: Lock screen
> (UI lock to prevent accidental taps during a cast), Stealth mode (visually
> minimize the running app), and Snapshot countdown (timed cast-start
> countdown). **UI-only — no real lock / inactivity / snapshot behaviour.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3
**Functional integration:** Deferred — no Android `KeyguardManager` /
`FLAG_SECURE` / `View.SYSTEM_UI_FLAG_*` integration.
**Moblin source analogues:**
- `View/Main/LockScreenView.swift`
- `View/Main/StealthModeView.swift`
- `View/Main/SnapshotCountdownView.swift`

**Files:**
- `senders/android/ui/components/lock_overlay.slint` — new
- `senders/android/ui/components/stealth_overlay.slint` — new
- `senders/android/ui/components/snapshot_countdown.slint` — new
- `senders/android/ui/bridge.slint` — `LifecycleMode` enum + property
- `senders/android/ui/main.slint` — overlay layering

---

## Tasks

### 18-A — `LifecycleMode` enum

- [ ] In `bridge.slint`:

  ```slint
  export enum LifecycleMode {
      normal,
      lock-screen,
      stealth,
      snapshot-countdown,
  }
  ```

- [ ] Add to `Bridge`:

  ```slint
      in-out property <LifecycleMode> lifecycle: LifecycleMode.normal;
      in-out property <int> mock-snapshot-secs: 5;
  ```

---

### 18-B — `LockOverlay` component

- [ ] `lock_overlay.slint`: a full-window translucent overlay with a centered
  card showing a lock glyph + "UI Locked" + "Tap and hold for 1.5s to unlock".
- [ ] Implement the long-press unlock by accumulating in a Slint `Timer`
  (`property <duration> hold-elapsed`), and on completion set
  `Bridge.lifecycle = LifecycleMode.normal`.
- [ ] Visual unlock-progress ring around the lock glyph (animated `Path` or
  ring of `Rectangle` segments rotating into focus).

---

### 18-C — `StealthOverlay` component

- [ ] Renders a near-black screen with a tiny "Tap to wake" hint at the
  bottom. Tapping anywhere flips `Bridge.lifecycle = LifecycleMode.normal`.

---

### 18-D — `SnapshotCountdown` component

- [ ] Big numeric countdown from `Bridge.mock-snapshot-secs` to 0 using a
  Slint `Timer { interval: 1s; running: ...; triggered => { ... } }`.
- [ ] On reaching 0, set `Bridge.lifecycle = LifecycleMode.normal` (UI-only —
  doesn't actually start a cast).
- [ ] Cancel button below the countdown returns to `normal`.

---

### 18-E — Layer overlays into `MainWindow`

- [ ] In `main.slint`:

  ```slint
  if Bridge.lifecycle == LifecycleMode.lock-screen:        LockOverlay { }
  if Bridge.lifecycle == LifecycleMode.stealth:            StealthOverlay { }
  if Bridge.lifecycle == LifecycleMode.snapshot-countdown: SnapshotCountdown { }
  ```

---

### 18-F — Settings entries in `FullSettingsPage`

- [ ] Add a "PRIVACY" section with three trigger rows:

  ```
  Lock UI                  → Bridge.lifecycle = LifecycleMode.lock-screen
  Stealth mode             → Bridge.lifecycle = LifecycleMode.stealth
  Cast with countdown      → Bridge.lifecycle = LifecycleMode.snapshot-countdown
  ```

- [ ] These rows are the only way to enter each mode in the UI-only build.

---

## Exit criteria

1. Each of the three lifecycle modes can be entered from settings.
2. Lock overlay long-press unlock works (visually animates a progress ring).
3. Stealth overlay dismisses on any tap.
4. Snapshot countdown ticks 5→0 and dismisses itself.
5. Cancel button on countdown returns to normal immediately.
6. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real `KeyguardManager` / device lock integration.
- `FLAG_SECURE` / screenshot-blocking application.
- Real cast-start triggered by the countdown.
- Inactivity-driven auto-stealth.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Lock/LockSettingsView.swift`
- `View/Settings/StealthMode/StealthModeSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Lock/LockSettingsView.swift (excerpt)
Form {
    Section { Toggle("Require unlock to start cast", isOn: $lock.enabled) }
    Section { Toggle("Stealth mode (hide cast UI)", isOn: $stealth.enabled) }
    Section { Slider(value: $snapshot.countdownSeconds, in: 0...10) }
}
```

**Mapping notes:**

Lock / stealth / snapshot-countdown each become a `SettingsToggleRow` or
`SettingsSliderRow` on a single page. The UI is intentionally minimal —
each mode is just a flag on the bridge. Stealth mode visually demonstrates
itself by dimming the entire `MainWindow` content via a `Rectangle {
background: black; opacity: <stealth-active ? 1 : 0>; }` overlay during
the placeholder demo.

**Relevant Slint docs:**
- [Slider](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx)
- [Switch](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/switch.mdx)

## Slint best practices applied here

- **`Timer { interval: 1s; running: ...; triggered => ... }`** is the
  canonical Slint pattern for countdowns. Pair with an integer `in-out`
  property that decrements on each `triggered`.
- **Long-press unlock via accumulating duration property** avoids needing a
  `TouchArea.pressed-changed` callback chain — set a timer that runs while
  pressed and stop it when released or on completion.
- **Three small overlays > one big mode-switch component.** Keeps each
  lifecycle UI scope-isolated and individually previewable.

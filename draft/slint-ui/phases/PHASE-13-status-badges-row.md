# Phase 13 — Status Badges Row Placeholder

> Add a compact row of status badges (battery, thermal, network) above the
> control bar. **UI-only — no real telemetry.** Badges read from inline
> `in-out` properties.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 4 (control bar)
**Functional integration:** Deferred — Rust telemetry source not in place.
**Moblin source analogues:**
- `ControlBar/BatteryView.swift`
- `ControlBar/ThermalStateSheetView.swift`

**Files:**
- `senders/android/ui/components/status_badges.slint` — new
- `senders/android/ui/main.slint` — insert above `CastControlBar`

---

## Goal

Render a tiny pill row showing battery %, thermal state, and network type.
Mirrors Moblin's top-of-bar status strip but without any device API access.

---

## Tasks

### 13-A — Component `StatusBadgesRow`

- [ ] Create `senders/android/ui/components/status_badges.slint`:

  ```slint
  import { Theme } from "../theme.slint";

  component Badge inherits Rectangle {
      in property <string> icon-glyph;
      in property <string> value;
      in property <color>  fg: Theme.text-secondary;

      height: 22px;
      border-radius: Theme.radius-pill;
      background: Theme.surface-overlay;

      HorizontalLayout {
          padding-left:  8px;
          padding-right: 8px;
          spacing: 4px;
          Text { text: root.icon-glyph; color: root.fg; font-size: Theme.font-size-label; }
          Text { text: root.value;       color: root.fg; font-size: Theme.font-size-label; }
      }
  }

  export component StatusBadgesRow inherits Rectangle {
      // UI-only stub state.
      in-out property <int>    mock-battery-pct: 87;
      in-out property <bool>   mock-charging:    false;
      in-out property <string> mock-thermal:     "Nominal";   // Nominal / Fair / Serious / Critical
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
              fg: root.mock-thermal == "Critical" ? Theme.error
                : root.mock-thermal == "Serious"  ? Theme.warning
                :                                   Theme.text-secondary;
          }
          Badge {
              icon-glyph: root.mock-charging ? "⚡" : "🔋";
              value: root.mock-battery-pct + "%";
              fg: root.mock-battery-pct < 20 ? Theme.error : Theme.text-secondary;
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 13-B — Place above `CastControlBar` in `MainWindow`

- [ ] In `main.slint`, add `StatusBadgesRow` immediately above the control bar
  in the `VerticalLayout` chassis:

  ```slint
  StatusBadgesRow { }
  CastControlBar  { }
  ```

- [ ] **Build check.**

---

### 13-C — Severity preview matrix

- [ ] Verify each severity renders correctly by temporarily flipping the stub
  values:
  - [ ] `mock-battery-pct: 8` → red battery glyph + value.
  - [ ] `mock-charging: true` → ⚡ glyph.
  - [ ] `mock-thermal: "Critical"` → red 🔥 + value.
  - [ ] `mock-thermal: "Serious"` → amber 🌡 + value.
- [ ] Revert all to nominal before commit.

---

## Exit criteria

1. `StatusBadgesRow` renders three badges (network, thermal, battery) right-aligned.
2. Severity coloring works for low battery (<20%) and serious/critical thermal.
3. Charging state flips the battery glyph.
4. Row sits above `CastControlBar` and does not overlap.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real Android `BatteryManager` integration.
- Real `PowerManager.ThermalEventListener`.
- Real network type from `ConnectivityManager`.
- Tap-to-expand thermal detail sheet.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/ControlBar/StatusBarView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/ControlBar/StatusBarView.swift (excerpt)
HStack {
    BatteryView(level: model.battery.level, charging: model.battery.charging)
    ThermalView(state: model.thermal.state)
    NetworkSignalView(level: model.network.signalLevel)
    if model.audio.muted { Image(systemName: "mic.slash.fill") }
}
.padding(.horizontal, 8)
.background(.black.opacity(0.5))
.clipShape(Capsule())
```

**Mapping notes:**

SwiftUI's `HStack` with conditional `Image` icons collapses neatly to a
Slint `HorizontalLayout` of `Badge` components. The capsule background uses
`border-radius: self.height / 2` on the outer `Rectangle`. Each badge is
gated by `if root.mock-<flag>:` rather than a `visible:` property so unused
badges aren't even instantiated.

**Relevant Slint docs:**
- [HorizontalLayout](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/layouts/horizontalbox.mdx)
- [Conditional elements](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/component.mdx)

## Slint best practices applied here

- **Internal `component Badge` + exported `StatusBadgesRow`.** Keeping
  reusable sub-components private prevents accidental external use of an
  unstable API.
- **Severity-driven `fg:` color binding via ternary chain** is idiomatic for
  compact status indicators where dropping into `match`-style logic would be
  overkill.

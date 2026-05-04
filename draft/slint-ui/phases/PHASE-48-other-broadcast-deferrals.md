# Phase 48 — Other Broadcast Deferrals Placeholder

> **UI-only.** Catch-all phase for everything else flagged "defer (other)" in
> `NOT-APPLICABLE.md` — camera level / draw-on-stream / fixed horizon /
> location overlay / stream grid / stream overlay layout / command-copy /
> placeholder strip. **Pure visual placeholders.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Permanently deferred.

**Moblin source analogues** (~12 files in the "defer (other)" bucket):
- `View/Stream/Overlay/StreamOverlayCameraLevelView.swift`
- `View/Stream/Overlay/StreamOverlayDrawOnStreamView.swift`
- `View/Stream/Overlay/StreamOverlayFixedHorizonView.swift`
- `View/Stream/Overlay/StreamOverlayLocationView.swift`
- `View/Stream/StreamGridView.swift`
- `View/Stream/Overlay/StreamOverlayLeftView.swift`
- `View/Stream/Overlay/StreamOverlayView.swift`
- `View/Settings/CommandCopy/CommandCopyView.swift`
- `View/Utils/PlaceholderView.swift`
- `View/ExternalDisplay/ExternalDisplayView.swift`

**Files to add:**
- `senders/android/ui/components/camera_level_overlay.slint`
- `senders/android/ui/components/draw_on_stream_overlay.slint`
- `senders/android/ui/components/fixed_horizon_overlay.slint`
- `senders/android/ui/components/location_overlay.slint`
- `senders/android/ui/components/stream_grid_overlay.slint`
- `senders/android/ui/components/stream_overlay_root.slint` — left+right
- `senders/android/ui/pages/command_copy_settings_page.slint`
- `senders/android/ui/pages/_apple/external_display_root_page.slint`

---

## Goal

Render Moblin's broadcast-side overlays (camera level, fixed horizon, draw
on stream, GPS overlay, grid overlay, etc.) as placeholders that can be
shown over the cast preview from Phase 12.

---

## Moblin source pattern

These are mostly thin SwiftUI overlays composited onto the live camera
preview. Examples:

```swift
// StreamOverlayCameraLevelView.swift (excerpt)
GeometryReader { geo in
    Path { path in
        let center = CGPoint(x: geo.size.width / 2, y: geo.size.height / 2)
        path.move(to: CGPoint(x: center.x - 40, y: center.y))
        path.addLine(to: CGPoint(x: center.x + 40, y: center.y))
    }
    .stroke(Color.white, lineWidth: 2)
    .rotationEffect(.degrees(-tilt))
}

// StreamOverlayLocationView.swift (excerpt)
HStack { Image("location"); Text(coordinatesLabel); Text(speedLabel) }
    .padding(8)
    .background(.black.opacity(0.5))
```

In Slint, simple shapes (lines, dots, dashed grids) are built with
`Rectangle` + `Path`.

---

## Tasks

### 48-A — `CameraLevelOverlay`

```slint
in property <float> mock-tilt-degrees: -8;       // -45 .. +45

Rectangle {
    width: 120px; height: 4px;
    background: white;
    border-radius: 2px;
    rotation-angle: root.mock-tilt-degrees * 1deg;
    rotation-origin-x: parent.width / 2;
    rotation-origin-y: parent.height / 2;
    x: parent.width / 2 - 60px;
    y: parent.height / 2 - 2px;
}

// Reference horizon line (always level)
Rectangle {
    width: 140px; height: 1px;
    background: #ffffff66;
    x: parent.width / 2 - 70px;
    y: parent.height / 2 - 0.5px;
}
```

### 48-B — `DrawOnStreamOverlay`

- [ ] Canvas (`Rectangle` with `clip: true`) plus a stub
  `mock-strokes: [{ id, points: [{x, y}], color }]`. No real
  drawing logic — just a few sample strokes encoded in `mock-strokes`.

### 48-C — `FixedHorizonOverlay`

- [ ] Single horizontal line at `parent.height / 2` with two short
  vertical tick marks. Static, no rotation.

### 48-D — `LocationOverlay`

```slint
in property <string> mock-coords: "47.6062° N, 122.3321° W";
in property <string> mock-speed:  "0 km/h";
in property <string> mock-altitude: "23 m";

Rectangle {
    background: #00000088;
    border-radius: 6px;
    HorizontalLayout {
        padding: 8px; spacing: 8px;
        Text { text: "📍"; font-size: 14px; vertical-alignment: center; }
        VerticalLayout {
            Text { text: root.mock-coords;   color: white; font-size: 12px; }
            HorizontalLayout {
                spacing: 8px;
                Text { text: root.mock-speed;    color: white; font-size: 12px; }
                Text { text: root.mock-altitude; color: white; font-size: 12px; }
            }
        }
    }
}
```

### 48-E — `StreamGridOverlay`

- [ ] 3×3 thirds-grid using `for` loops over 2 vertical and 2
  horizontal `Rectangle { width: 1px; height: 100%; background:
  #ffffff44; }` lines.

### 48-F — `StreamOverlayRoot`

- [ ] Combines all left-aligned overlays (chat, location) and
  right-aligned overlays (HUD, audio meter). Render shapes on
  appropriate edges, gated by inline `mock-show-*` toggles.

### 48-G — `CommandCopySettingsPage`

- [ ] List of pre-defined commands (`mock-commands: [{ name,
  template, last-output }]`) with copy-to-clipboard button (no-op
  in placeholder).

### 48-H — `ExternalDisplayRootPage` (under `_apple/`)

- [ ] Reference-only — Moblin renders a separate scene on an
  external display. FCast Android does not. Placeholder lives at
  `_apple/external_display_root_page.slint`.

---

## Exit criteria

1. All overlay components render their visual primitives (lines,
   text, shapes).
2. `StreamOverlayRoot` correctly toggles individual overlays via
   `mock-show-*` flags.
3. `CommandCopySettingsPage` lists 3 mock commands.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real accelerometer / IMU input (camera level / fixed horizon).
- Real touch drawing on stream.
- Real GPS / location services.
- Real external display detection.

---

## Slint best practices applied here

- **`rotation-angle: degrees * 1deg`** — Slint's `angle` type accepts
  `deg` / `rad` / `turn` literals. Multiplying a numeric `mock-tilt`
  by `1deg` produces an angle.
- **`#ffffff44`** for grid lines — RGBA hex enables sub-pixel
  translucency without alpha-channel arithmetic.
- **Static `[...]` strokes for draw-on-stream** keeps the placeholder
  small while still showing the layered-overlay model.

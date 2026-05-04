# Phase 15 — Camera Capture Controls Placeholder

> Settings sub-page for camera capture parameters (front/back, resolution,
> framerate, mirror, stabilization, zoom). **UI-only — Rust camera capability
> deferred.** All controls flip inline `in-out` properties.

**Status:** `[ ] Not started — blocked by Rust camera capability for live data, but UI placeholder is unblocked`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — `senders/android/` has no `CameraX` /
GStreamer camera source today. The placeholder UI is intentionally exhaustive
so that when Rust capability lands, only the binding source changes.
**Moblin source analogues (8 files):**
- `Settings/Camera/CameraSettingsView.swift`
- `Settings/Camera/CameraControls/CameraControlsView.swift`
- `Settings/Camera/MirrorFrontCamera/MirrorFrontCameraView.swift`
- `Settings/Camera/TapScreenToFocus/TapScreenToFocusSettingsView.swift`
- `Settings/Camera/VideoStabilization/VideoStabilizationSettingsView.swift`
- `Settings/Camera/Zoom/ZoomSettingsView.swift`
- `Settings/Camera/Zoom/ZoomPresetSettingsView.swift`
- `Settings/Camera/Zoom/ZoomSwitchToSettingsView.swift`

**Files:**
- `senders/android/ui/pages/camera_page.slint` — new
- `senders/android/ui/pages/settings_page.slint` — link from settings root
- `senders/android/ui/bridge.slint` — `Panel.camera` variant

---

## Tasks

### 15-A — `CameraPage` panel

- [ ] Create `senders/android/ui/pages/camera_page.slint` with header + Done
  button + scrolling sections (mirror Phase 14 layout).
- [ ] Inline stub state:

  ```slint
  in-out property <int>   mock-camera-idx:       1;  // Front / Back / External
  in-out property <int>   mock-resolution-idx:   2;  // 480p / 720p / 1080p / 4k
  in-out property <int>   mock-framerate-idx:    1;  // 24 / 30 / 60 fps
  in-out property <bool>  mock-mirror-front:     true;
  in-out property <bool>  mock-stabilization:    true;
  in-out property <bool>  mock-tap-to-focus:     true;
  in-out property <float> mock-zoom-level:       1.0; // 0.5 .. 5.0
  ```

- [ ] Sections:

  ```
  SOURCE
    Camera                    Front / Back / External   (cycle on click)
    Resolution                480p / 720p / 1080p / 4K (cycle)
    Framerate                 24 / 30 / 60 fps         (cycle)

  IMAGE
    Mirror front camera       toggle
    Video stabilization       toggle
    Tap to focus              toggle

  ZOOM
    Current zoom              slider (0.5 .. 5.0)
    Presets                   "Wide" / "1×" / "2×" / "5×" — value rows that
                              snap the slider on click
  ```

- [ ] **Build check.**

---

### 15-B — Zoom presets row

- [ ] Implement preset chips that set `mock-zoom-level` directly:

  ```slint
  HorizontalLayout {
      spacing: 8px;
      // Generated from a model so the count is data-driven.
      for preset in [0.5, 1.0, 2.0, 5.0]: PresetChip {
          label: preset == 1.0 ? "1×" : preset + "×";
          active: root.mock-zoom-level == preset;
          clicked => { root.mock-zoom-level = preset; }
      }
  }
  ```

  `PresetChip` is a small internal component: a rounded `Rectangle` with
  `background:` switching on `active`.

---

### 15-C — Bridge + settings root linking

- [ ] In `bridge.slint`, extend `Panel`: add `camera`.
- [ ] In `main.slint`, add `if Bridge.active-panel == Panel.camera: CameraPage { }`.
- [ ] In `FullSettingsPage`, add a row:

  ```slint
  SettingsValueRow {
      label: "Camera";
      value: "Open";
      clicked => { Bridge.active-panel = Panel.camera; }
  }
  ```

---

## Exit criteria

1. `CameraPage` opens and closes via the panel routing.
2. All toggles, value-pickers, and the zoom slider flip stub state.
3. Zoom preset chips snap the slider value and visually highlight the active preset.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real `CameraX` / GStreamer camera source selection.
- Real resolution/framerate negotiation with the encoder.
- Live front/back mirror toggle on the actual capture pipeline.
- HDR / portrait / cinematic capture toggles (deferred indefinitely).
- Camera preview surface (that's Phase 12's `CapturePreview` — they don't
  cross-cut here).

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Camera/CameraSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Camera/CameraSettingsView.swift (excerpt)
Form {
    Section {
        Picker("Position", selection: $stream.cameraPosition) {
            ForEach(SettingsCameraPosition.allCases, id: \.self) { Text($0.toString()) }
        }
        Picker("Resolution", selection: $stream.resolution) {
            ForEach(supportedResolutions, id: \.self) { Text($0.toString()) }
        }
        Picker("FPS", selection: $stream.fps) {
            ForEach(supportedFps, id: \.self) { Text("\($0)") }
        }
    }
    Section {
        Toggle("Mirror front camera", isOn: $database.color.frontCameraMirrored)
        Toggle("Stabilization", isOn: $stream.videoStabilization)
    }
}
```

**Mapping notes:**

Camera position / resolution / FPS pickers map to three `SettingsValueRow`
cyclers driven by inline option arrays. Mirror & stabilization toggles map
1:1 to `SettingsToggleRow`. The capture preview rectangle from Phase 12 sits
above this form so the user sees the source they're configuring.

**Relevant Slint docs:**
- [ComboBox](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/combobox.mdx)

## Slint best practices applied here

- **Internal `PresetChip` component + a model-driven `for` loop** keeps the
  preset list data-driven and easy to extend.
- **`active:` boolean comparison on a continuous slider value** is idiomatic
  for snapping discrete chips against a continuous control.

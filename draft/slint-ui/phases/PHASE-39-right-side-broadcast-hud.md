# Phase 39 — Right-side Broadcast HUD Placeholder

> **UI-only.** Right-edge floating panel with broadcast controls (audio
> level, camera controls, replay, scene selector, beauty/face/pixellate/
> whirlpool effects, video preview, zoom presets). **No real video pipeline.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 4, 13 (status badges row)
**Functional integration:** Permanently deferred.

**Moblin source analogues** (13 files):
- `View/Stream/Overlay/Right/StreamOverlayRightView.swift` — root
- `View/Stream/Overlay/Right/StreamOverlayRightAudioMeter*View.swift` (×3)
- `View/Stream/Overlay/Right/StreamOverlayRightCameraView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightReplayView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightSceneSelectorView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightBeautyView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightFaceView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightPinchView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightPixellateView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightWhirlpoolView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightVideoPreviewView.swift`
- `View/Stream/Overlay/Right/StreamOverlayRightZoomPresetsView.swift`

**Files to add:**
- `senders/android/ui/components/right_hud.slint` — root container
- `senders/android/ui/components/right_hud_audio_meter.slint`
- `senders/android/ui/components/right_hud_camera.slint`
- `senders/android/ui/components/right_hud_replay.slint`
- `senders/android/ui/components/right_hud_scene_selector.slint`
- `senders/android/ui/components/right_hud_beauty.slint`
- `senders/android/ui/components/right_hud_face.slint`
- `senders/android/ui/components/right_hud_pinch.slint`
- `senders/android/ui/components/right_hud_pixellate.slint`
- `senders/android/ui/components/right_hud_whirlpool.slint`
- `senders/android/ui/components/right_hud_video_preview.slint`
- `senders/android/ui/components/right_hud_zoom_presets.slint`

---

## Goal

Render the right-edge floating broadcast HUD with all its sub-panels —
toggleable from the cast control bar (Phase 4) but rendered as a static
demo here.

---

## Moblin source pattern

```swift
// StreamOverlayRightView.swift (excerpt)
VStack {
    StreamOverlayRightAudioMeterView(...)
    StreamOverlayRightCameraView(...)
    StreamOverlayRightSceneSelectorView(...)
    StreamOverlayRightZoomPresetsView(...)
    if database.beauty.enabled { StreamOverlayRightBeautyView(...) }
    if database.face.enabled   { StreamOverlayRightFaceView(...) }
    // ...
}
.frame(width: 80)
.background(.black.opacity(0.4))
```

A vertical stack on the right edge with conditional sub-panels.

---

## Tasks

### 39-A — `RightHud` root

```slint
export component RightHud inherits Rectangle {
    in-out property <bool> mock-show-beauty: false;
    in-out property <bool> mock-show-face: false;
    in-out property <bool> mock-show-pixellate: false;
    in-out property <bool> mock-show-pinch: false;
    in-out property <bool> mock-show-whirlpool: false;

    width: 96px;
    background: #00000088;
    border-radius: Theme.radius-card;

    VerticalLayout {
        spacing: 6px;
        padding: 6px;
        alignment: start;

        RightHudAudioMeter   {}
        RightHudCamera       {}
        RightHudSceneSelector {}
        RightHudZoomPresets  {}
        RightHudReplay       {}
        RightHudVideoPreview {}

        if root.mock-show-beauty:    RightHudBeauty    {}
        if root.mock-show-face:      RightHudFace      {}
        if root.mock-show-pixellate: RightHudPixellate {}
        if root.mock-show-pinch:     RightHudPinch     {}
        if root.mock-show-whirlpool: RightHudWhirlpool {}
    }
}
```

### 39-B — `RightHudAudioMeter`

```slint
in-out property <float> mock-level-db: -18;       // dBFS
in-out property <float> mock-peak-db: -6;

VerticalLayout {
    Text { text: "Audio"; color: white; font-size: 10px; horizontal-alignment: center; }
    Rectangle {
        width: 24px;
        height: 96px;
        background: #00000088;
        // Live bar — mapping -60dB..0dB into 0..96px
        Rectangle {
            x: 0; y: parent.height - (parent.height * (root.mock-level-db + 60) / 60);
            width: parent.width;
            background: root.mock-level-db > -3 ? Theme.error
                                                : (root.mock-level-db > -12 ? Theme.warning : Theme.success);
        }
    }
    Text { text: "\{root.mock-level-db} dB"; color: white; font-size: 9px; horizontal-alignment: center; }
}
```

### 39-C — `RightHudCamera`

- [ ] Buttons: Front/Back toggle, exposure compensation (±2 EV cycler),
  ISO cycler, white-balance picker — all mocked.

### 39-D — `RightHudReplay`

- [ ] "Save replay" button (placeholder), buffer length cycler
  (`["10s", "30s", "60s", "5min"]`).

### 39-E — `RightHudSceneSelector`

- [ ] Vertical list of scene buttons (uses Phase 33's `mock-scenes`).
  Active scene gets `Theme.accent` border.

### 39-F — `RightHudBeauty`

```slint
in-out property <int>   mock-mode-idx: 0;        // 0 = smoothness, 1 = shape
in-out property <float> mock-radius: 0.5;
in-out property <float> mock-strength: 0.5;

VerticalLayout {
    SettingsValueRow { label: "Mode"; value: ["Smoothness", "Shape"][root.mock-mode-idx];
        clicked => { root.mock-mode-idx = mod(root.mock-mode-idx + 1, 2); } }
    SettingsSliderRow { label: "Radius"; min: 0; max: 1;
                        value: root.mock-radius * 100;
                        changed(v) => { root.mock-radius = v / 100; } }
    SettingsSliderRow { label: "Strength"; min: 0; max: 1;
                        value: root.mock-strength * 100;
                        changed(v) => { root.mock-strength = v / 100; } }
}
```

> Mirrors `StreamOverlayRightBeautyView.swift`'s smoothness/shape modes
> with their radius/strength sliders.

### 39-G — `RightHudFace`

- [ ] AR face filter toggle, intensity slider, filter picker (cycler).

### 39-H — `RightHudPinch` / `RightHudWhirlpool` / `RightHudPixellate`

- [ ] Each: enable toggle, single intensity slider, target radius
  slider. (These are pixel effects — render the slider only;
  preview is omitted.)

### 39-I — `RightHudVideoPreview`

- [ ] Picture-in-picture preview placeholder: 80×60 rectangle with
  "preview" label.

### 39-J — `RightHudZoomPresets`

- [ ] List of zoom level buttons (`["0.5×", "1×", "2×", "3×", "5×"]`)
  with active highlight.

### 39-K — Embed in `MainWindow`

- [ ] In Phase 4's `MainWindow`, anchor `RightHud` to the right edge
  when `Bridge.show-right-hud` (a new bool — but here the toggle is
  *self-mutated by a quick-action button* with no Rust binding).

---

## Exit criteria

1. `RightHud` renders a 96px wide right-edge stack with audio meter +
   camera + scene-selector + zoom-presets always visible.
2. Beauty / face / pinch / pixellate / whirlpool sub-panels appear
   when their `mock-show-*` toggle flips.
3. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real video sample rendering anywhere.
- Real audio dBFS level (the `mock-level-db` is static).
- Real ISP controls (camera exposure, white balance, ISO).
- Real beauty/face/pinch/pixellate/whirlpool shaders.
- Real replay buffer (Phase 40 handles the *settings* placeholder).

---

## Slint best practices applied here

- **`#00000088`** RGBA hex for translucent background — Slint's color
  literal supports the `#RRGGBBAA` shorthand.
- **`if root.mock-show-X: ...` conditional sub-panels** keep the HUD
  collapsed by default; each sub-panel is independently togglable
  with no Rust state.
- **Vertical bar level meter** built from a single nested `Rectangle`
  whose `y` is bound to the dBFS value — no `Path` or shaders
  needed for the placeholder.

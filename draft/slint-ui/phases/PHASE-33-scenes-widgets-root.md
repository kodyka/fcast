# Phase 33 — Scenes & Widget Editor (Root) Placeholder

> **UI-only.** Scenes list, per-scene canvas summary, widget library list,
> per-widget root chrome. **No real scene compositor, no real widget render.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Permanently deferred — FCast does not author scenes.

**Moblin source analogues** (~10 files):
- `View/Settings/Scenes/ScenesSettingsView.swift` — root list
- `View/Settings/Scenes/Scene/{SceneSettingsView,SceneWidgetSettingsView}.swift`
- `View/Settings/Scenes/Widgets/WidgetsSettingsView.swift` — library
- `View/Settings/Scenes/Widgets/Widget/{WidgetSettingsView,WidgetWizardSettingsView}.swift`
- `View/Settings/Scenes/AutoSwitchers/AutoSwitchersSettingsView.swift`
- `View/Settings/Scenes/DisconnectProtection/DisconnectProtectionSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/scenes_settings_page.slint`
- `senders/android/ui/pages/scene_settings_page.slint`
- `senders/android/ui/pages/scene_widget_settings_page.slint`
- `senders/android/ui/pages/widgets_settings_page.slint`
- `senders/android/ui/pages/widget_settings_page.slint`
- `senders/android/ui/pages/auto_switchers_settings_page.slint`
- `senders/android/ui/pages/disconnect_protection_settings_page.slint`

---

## Goal

Render Moblin's two-level scene architecture:
- **Scene** = ordered list of widget instances (each with placement)
- **Widget** = a definition (text, image, scoreboard, etc.) that can be
  reused across scenes

Build the placeholder routing chassis here; per-widget content lives in
Phases 34–37.

---

## Moblin source pattern

```swift
// View/Settings/Scenes/Scene/SceneSettingsView.swift (excerpt)
Form {
    Section {
        TextEditNavigationView(value: $scene.name)
        Picker("Camera", selection: $scene.cameraPosition) { ... }
    }
    Section("Widgets") {
        ForEach(scene.widgets) { widget in
            NavigationLink { SceneWidgetSettingsView(widget: widget) } label: {
                HStack { DraggableItemPrefixView(); Text(widget.name) }
            }
        }
        AddButtonView { /* picker that opens widget library */ }
    }
}
```

In Slint the canvas summary is rendered as a small `Rectangle` with overlays
positioned absolutely (a *miniature*) — enough to see where each widget sits
without compositing real video.

---

## Tasks

### 33-A — Add Panel variants

- [ ] `Panel.scenes`, `Panel.scene`, `Panel.scene-widget`, `Panel.widgets`,
  `Panel.widget`, `Panel.auto-switchers`, `Panel.disconnect-protection`.

### 33-B — Mock structs (in `bridge.slint`)

```slint
export struct Scene {
    id: int,
    name: string,
    camera-position-idx: int,        // 0=back, 1=front, 2=external
    enabled: bool,
}

export struct WidgetInstance {
    id: int,
    widget-name: string,             // ref to Widget.name
    x: percent,                      // 0..100
    y: percent,
    w: percent,
    h: percent,
    enabled: bool,
}

export struct Widget {
    id: int,
    name: string,
    kind: string,                    // "text" / "image" / "alerts" / etc.
}
```

### 33-C — `ScenesSettingsPage` (root list)

- [ ] Inline `mock-scenes: [Scene]` (3 example scenes), reorder via ▲/▼,
  long-press popup with Duplicate / Delete actions.
- [ ] "Scene transitions" sub-section (matching Moblin's `SceneSwitching`
  inner view) with picker `["Cut", "Fade"]`, force-transition toggle.

### 33-D — `SceneSettingsPage`

- [ ] Sections:
  - GENERAL: name, camera-position cycler
  - **CANVAS** (placeholder mini-canvas):

    ```slint
    Rectangle {
        height: 180px;
        background: black;
        border-color: Theme.surface-overlay;
        border-width: 1px;
        for w in root.mock-widget-instances: Rectangle {
            x: w.x;
            y: w.y;
            width: w.w;
            height: w.h;
            background: w.enabled ? Theme.accent-pressed : Theme.surface-overlay;
            opacity: 0.7;
            Text {
                text: w.widget-name;
                color: white;
                font-size: 10px;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
        }
    }
    ```

  - WIDGETS list with reorder + remove.
  - "Add widget" → opens the widget library picker (Phase 33-F).

### 33-E — `SceneWidgetSettingsPage` (per-instance placement)

- [ ] Position editor (X/Y `SettingsValueRow` cyclers in 5% steps), size
  editor, "Lock aspect ratio" toggle, opacity slider.
- [ ] Delete button → removes from parent scene's `mock-widget-instances`.

### 33-F — `WidgetsSettingsPage` (library)

- [ ] List of widget definitions (`mock-widgets: [Widget]` with ~15 stub
  entries spanning all kinds: text, image, slideshow, browser, scoreboard,
  alerts, …).
- [ ] "Create widget" → opens `WidgetWizardSettingsPage` (Phase 37).

### 33-G — `WidgetSettingsPage` (per-widget root)

- [ ] Renders the widget-kind editor by routing on `mock-kind`. Each
  routed sub-editor is built in Phases 34–36.
- [ ] Common header: name (editable), kind label (read-only).

### 33-H — `AutoSwitchersSettingsPage`

- [ ] Toggle "Enabled", schedule rules list (`mock-rules: [{ trigger,
  scene-name, condition }]`).

### 33-I — `DisconnectProtectionSettingsPage`

- [ ] Toggle "Show fallback overlay on stream disconnect", picker for
  fallback scene, hold-time slider.

---

## Exit criteria

1. `ScenesSettingsPage` lists 3 mock scenes with reorder.
2. Opening a scene shows a miniature canvas with 4–5 widget rectangles
   positioned via `x: ...%`.
3. Adding a widget appends to `mock-widget-instances` (rectangle appears
   in the miniature).
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real video compositing.
- Real widget rendering (text, image, scoreboard, etc.).
- Real source switching, transitions, fades.
- Real disconnect detection (placeholder picks a "fallback scene" but
  nothing fires it).

---

## Slint best practices applied here

- **`x: percent` / `width: percent`** — Slint accepts `%` values for
  layout coordinates against the parent's resolved size, which matches
  the relative placement model SwiftUI uses with `GeometryReader`.
- **`for w in mock-widget-instances:`** inside a single Rectangle gives
  the *miniature canvas* — Slint doesn't need a `ZStack`; absolute
  positioning happens automatically when children carry `x`/`y`.
- **Two-level enum routing** (`Panel.scenes` → `Panel.scene` →
  `Panel.scene-widget`) keeps each surface independent; back navigation
  resets the deeper panel before opening a higher-level one.

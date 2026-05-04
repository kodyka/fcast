# Phase 38 — Local Overlays + Position/Size Editors Placeholder

> **UI-only.** Local overlays root list (overlays added on top of *every*
> scene, not per-scene) plus the shared widget-position / widget-size
> editor surface. **No real overlay rendering.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 33
**Functional integration:** Permanently deferred.

**Moblin source analogues** (3 files):
- `View/Settings/LocalOverlays/LocalOverlaysSettingsView.swift`
- `View/Settings/LocalOverlays/WidgetPositionView.swift`
- `View/Settings/LocalOverlays/WidgetSizeView.swift`

**Files to add:**
- `senders/android/ui/pages/local_overlays_settings_page.slint`
- `senders/android/ui/components/widget_position_editor.slint`
- `senders/android/ui/components/widget_size_editor.slint`

---

## Goal

Render Moblin's "always-on" overlays panel and the canvas editor
components used to position widgets, both per-scene and globally.

---

## Moblin source pattern

```swift
// LocalOverlaysSettingsView.swift (excerpt)
List {
    ForEach(database.localOverlays) { overlay in
        NavigationLink { /* overlay editor */ } label: {
            Toggle(overlay.name, isOn: $overlay.enabled)
        }
    }
}
```

The position/size editors are tappable canvases that show drag handles —
in Slint this becomes a `Rectangle` with corner handles (4 small
`Rectangle`s with `TouchArea`s).

---

## Tasks

### 38-A — `LocalOverlaysSettingsPage`

```slint
export struct LocalOverlay {
    id: int,
    name: string,
    enabled: bool,
    widget-name: string,         // ref to a Widget by name
}

export component LocalOverlaysSettingsPage inherits Rectangle {
    in-out property <[LocalOverlay]> mock-overlays: [
        { id: 1, name: "Watermark",      enabled: true,  widget-name: "Logo" },
        { id: 2, name: "Streamer name",  enabled: false, widget-name: "Text-host" },
        { id: 3, name: "Donate banner",  enabled: false, widget-name: "Image-banner" },
    ];

    VerticalLayout {
        for o[i] in root.mock-overlays: SettingsToggleRow {
            label: o.name + " · " + o.widget-name;
            checked: o.enabled;
            toggled => {
                root.mock-overlays[i].enabled = !o.enabled;
            }
        }
    }
}
```

### 38-B — `WidgetPositionEditor`

```slint
export component WidgetPositionEditor inherits Rectangle {
    in-out property <percent> mock-x: 10%;
    in-out property <percent> mock-y: 10%;
    in-out property <percent> mock-w: 30%;
    in-out property <percent> mock-h: 20%;

    background: black;
    height: 240px;

    // Widget rectangle
    Rectangle {
        x: root.mock-x; y: root.mock-y;
        width: root.mock-w; height: root.mock-h;
        background: Theme.accent;
        opacity: 0.6;
        border-color: white;
        border-width: 2px;

        // Corner drag handles (placeholder — TouchAreas adjust mock-w / mock-h)
        Rectangle {
            x: parent.width - 10px; y: parent.height - 10px;
            width: 10px; height: 10px;
            background: white;
            ta := TouchArea {
                moved => {
                    root.mock-w = root.mock-w + (self.mouse-x - self.pressed-x) / parent.width * 100%;
                    root.mock-h = root.mock-h + (self.mouse-y - self.pressed-y) / parent.height * 100%;
                }
            }
        }
    }

    // X / Y cyclers as a fallback for keyboard / non-pointer environments
    HorizontalLayout {
        SettingsValueRow { label: "X"; value: "\{root.mock-x}%";
            clicked => { root.mock-x = mod(root.mock-x + 5%, 100%); } }
        SettingsValueRow { label: "Y"; value: "\{root.mock-y}%";
            clicked => { root.mock-y = mod(root.mock-y + 5%, 100%); } }
    }
}
```

### 38-C — `WidgetSizeEditor`

- [ ] Two sliders (W, H) with a "Lock aspect" toggle. When locked,
  changing W proportionally changes H. The aspect-lock behaviour is
  one bound expression: `mock-h = root.mock-aspect-locked ?
  root.mock-w * 9 / 16 : root.mock-h;`

### 38-D — Embed in scene editor

- [ ] In `scene_widget_settings_page.slint` (Phase 33), embed
  `WidgetPositionEditor` and `WidgetSizeEditor` so the user can
  drag-position widgets in addition to the X/Y cyclers.

---

## Exit criteria

1. `LocalOverlaysSettingsPage` lists 3 overlays with toggleable state.
2. `WidgetPositionEditor` renders a black canvas with a draggable
   widget rectangle; cyclers also move it.
3. `WidgetSizeEditor` resizes the rectangle; aspect-lock works.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real compositing of overlays onto a video stream.
- Z-order / blending modes.
- Snapping / alignment guides.

---

## Slint best practices applied here

- **`percent` type** — Slint supports `%` literals as a first-class
  unit; arithmetic stays in percentage space without conversion to
  px/parent-relative.
- **Drag handles via `TouchArea.moved` + `mouse-x` / `pressed-x` deltas**
  is the canonical Slint pattern for drag interactions; documented in
  the [TouchArea reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/toucharea.mdx).
- **Reactive aspect lock** via `mock-h = locked ? mock-w * 9/16 :
  mock-h` is a one-liner — Slint's reactive system handles the
  dependency tracking.

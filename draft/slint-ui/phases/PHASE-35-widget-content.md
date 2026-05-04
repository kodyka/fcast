# Phase 35 — Widget Content (Text / Image / Slideshow / Scoreboard / VTuber / etc.) Placeholder

> **UI-only.** Per-content-widget editor pages. Each is a `.slint` page that
> takes inline `mock-*` state and renders the form. **No real rendering of
> any widget** — the scene canvas in Phase 33 just shows a labelled
> rectangle.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 33
**Functional integration:** Permanently deferred.

**Moblin source analogues** (~17 files):
- Text:        `Widget/Text/WidgetTextSettingsView.swift`
- Image:       `Widget/Image/WidgetImageSettingsView.swift`
- Slideshow:   `Widget/Slideshow/WidgetSlideshowSettingsView.swift`
- Snapshot:    `Widget/Snapshot/WidgetSnapshotSettingsView.swift`
- QR code:     `Widget/QrCode/WidgetQrCodeSettingsView.swift`
- Map:         `Widget/Map/WidgetMapSettingsView.swift`
- Bingo:       `Widget/BingoCard/WidgetBingoCardSettingsView.swift`
- Wheel:       `Widget/WheelOfLuck/WidgetWheelOfLuckSettingsView.swift`
- Scoreboard:  `Widget/Scoreboard/{WidgetScoreboardSettingsView,…GenericSettingsView,…ModularSettingsView,…PadelSettingsView}.swift`
- VTuber:      `Widget/VTuber/WidgetVTuberSettingsView.swift`
- PNGTuber:    `Widget/PngTuber/WidgetPngTuberSettingsView.swift`
- Video src:   `Widget/VideoSource/WidgetVideoSourceSettingsView.swift`
- Browser:     `Widget/Browser/WidgetBrowserSettingsView.swift`
- Sub-scene:   `Widget/Scene/WidgetSceneSettingsView.swift`
- Crop:        `Widget/Crop/WidgetCropSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/widget_text_settings_page.slint`
- `senders/android/ui/pages/widget_image_settings_page.slint`
- `senders/android/ui/pages/widget_slideshow_settings_page.slint`
- `senders/android/ui/pages/widget_snapshot_settings_page.slint`
- `senders/android/ui/pages/widget_qr_code_settings_page.slint`
- `senders/android/ui/pages/widget_map_settings_page.slint`
- `senders/android/ui/pages/widget_bingo_settings_page.slint`
- `senders/android/ui/pages/widget_wheel_settings_page.slint`
- `senders/android/ui/pages/widget_scoreboard_settings_page.slint`
- `senders/android/ui/pages/widget_scoreboard_generic_page.slint`
- `senders/android/ui/pages/widget_scoreboard_modular_page.slint`
- `senders/android/ui/pages/widget_scoreboard_padel_page.slint`
- `senders/android/ui/pages/widget_vtuber_settings_page.slint`
- `senders/android/ui/pages/widget_pngtuber_settings_page.slint`
- `senders/android/ui/pages/widget_video_source_settings_page.slint`
- `senders/android/ui/pages/widget_browser_settings_page.slint`
- `senders/android/ui/pages/widget_scene_settings_page.slint`
- `senders/android/ui/pages/widget_crop_settings_page.slint`

---

## Goal

Provide a placeholder UI for each Moblin widget kind so the routing in
Phase 33 (`Panel.widget` + the kind dispatch) has somewhere to land.

---

## Moblin source pattern

Each widget kind exposes a small set of fields. Examples:

```swift
// WidgetTextSettingsView.swift (excerpt)
TextEditNavigationView(value: $widget.text.formatString)
ColorPicker("Color", selection: $widget.text.color)
Slider(value: $widget.text.size, in: 10...96)
Picker("Anchor", selection: $widget.text.anchor) { ForEach(TextAnchor.allCases) {...} }

// WidgetSlideshowSettingsView.swift (excerpt)
List {
    ForEach(widget.slideshow.images) { image in
        NavigationLink { /* image editor */ } label: { /* thumbnail row */ }
    }
}
Slider(value: $widget.slideshow.intervalSeconds, in: 1...60)

// WidgetWheelOfLuckSettingsView.swift (excerpt)
List {
    ForEach(widget.wheelOfLuck.items) { item in
        TextEditNavigationView(value: $item.label)
    }
}
```

---

## Tasks

### 35-A — Add Panel variants

- [ ] One per file: `Panel.widget-text`, `Panel.widget-image`, …,
  `Panel.widget-crop`. Group routing through Phase 33's
  `WidgetSettingsPage` dispatcher.

### 35-B — `WidgetTextSettingsPage`

```slint
in-out property <string> mock-format: "Hello {name} — {timestamp}";
in-out property <int>    mock-font-size: 32;
in-out property <int>    mock-anchor-idx: 0;       // top-left/center/etc.
in-out property <color>  mock-text-color: white;
in-out property <bool>   mock-show-stroke: true;

VerticalLayout {
    SettingsTextRow { label: "Text"; text: root.mock-format;
                      edited(s) => { root.mock-format = s; } }
    SettingsSliderRow { label: "Size"; min: 10; max: 96; value: root.mock-font-size;
                        changed(v) => { root.mock-font-size = v; } }
    SettingsValueRow { label: "Anchor"; value: anchor-options[root.mock-anchor-idx];
                       clicked => { root.mock-anchor-idx = mod(root.mock-anchor-idx + 1, 9); } }
    SettingsToggleRow { label: "Stroke"; checked: root.mock-show-stroke;
                        toggled => { root.mock-show-stroke = !root.mock-show-stroke; } }

    // Live preview of the text
    Rectangle {
        height: 80px;
        background: Theme.surface-overlay;
        Text {
            text: root.mock-format;
            color: root.mock-text-color;
            font-size: root.mock-font-size * 1px;
            stroke: root.mock-show-stroke ? black : transparent;
            stroke-width: root.mock-show-stroke ? 2px : 0px;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

### 35-C — `WidgetImageSettingsPage`

- [ ] File picker stub (`SettingsValueRow` cycler over 5 mock filenames),
  fit picker (`["Fit", "Fill", "Stretch"]`), opacity slider.

### 35-D — `WidgetSlideshowSettingsPage`

- [ ] `mock-images: [{ id, name, duration-s }]` list with reorder, "Add
  image" stub, interval slider, transition picker.

### 35-E — `WidgetSnapshotSettingsPage`

- [ ] Source picker (`["Capture", "Upload"]`), duration cycler.

### 35-F — `WidgetQrCodeSettingsPage`

- [ ] URL field, "Use device pairing URL" toggle, foreground/background
  color pickers (cyclers over a small palette), preview Rectangle.

### 35-G — `WidgetMapSettingsPage`

- [ ] Lat/Lon `SettingsTextRow`s (numeric), zoom slider, marker style
  picker. Preview = a black rectangle with a label "(map preview)".

### 35-H — `WidgetBingoCardSettingsPage`

- [ ] 5×5 grid editor: `mock-cells: [{ idx, text, marked }]` (length 25).
  Tap a cell to toggle `marked`. Render with `GridLayout`.

### 35-I — `WidgetWheelOfLuckSettingsPage`

- [ ] `mock-items: [string]` editable list, "Spin (preview)" button that
  does nothing functionally, color palette cycler.

### 35-J — `WidgetScoreboardSettingsPage` (root + 3 variants)

- [ ] Variant picker → routes to `widget_scoreboard_{generic,modular,padel}`.
  Each variant shows team-name fields, score `SettingsValueRow` cyclers
  (-1 / +1), period picker.

### 35-K — `WidgetVTuberSettingsPage`

- [ ] Avatar file stub, calibration "tap to recalibrate" button, mouth
  open threshold slider.

### 35-L — `WidgetPngTuberSettingsPage`

- [ ] `mock-states: [{ name, image, mouth-open }]` list, talk-threshold
  slider.

### 35-M — `WidgetVideoSourceSettingsPage`

- [ ] Source picker (`["Front camera", "Back camera", "External", "Screen"]`)
  — read-only stub (no real source switching).

### 35-N — `WidgetBrowserSettingsPage`

- [ ] URL field, refresh-interval slider, "Allow JS" toggle, preview =
  rectangle with title "(browser preview)".

### 35-O — `WidgetSceneSettingsPage`

- [ ] Sub-scene picker over the same `mock-scenes` from Phase 33.

### 35-P — `WidgetCropSettingsPage`

- [ ] X/Y/W/H sliders, preview = rectangle with crop overlay outline.

---

## Exit criteria

1. Each of the 18 widget pages renders its form.
2. Live previews on text / image / scoreboard / wheel / bingo / crop
   pages reflect their `mock-*` state.
3. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Any actual rendering of widgets onto a scene canvas.
- File / image / video asset import.
- Real map / geolocation.
- Real VTuber / PNGTuber face tracking.

---

## Slint best practices applied here

- **`stroke` / `stroke-width` on `Text`** is the built-in Slint
  property that mirrors Moblin's `StrokeModifier.swift`.
- **`GridLayout`** for the 5×5 bingo card is far simpler than chained
  `HorizontalLayout` rows — see [GridLayout reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/layouts/gridbox.mdx).
- **Reactive previews** — preview rectangles bind directly to the
  `mock-*` state; no callbacks required to refresh them.

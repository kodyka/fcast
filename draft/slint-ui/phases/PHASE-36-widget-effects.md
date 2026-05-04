# Phase 36 — Widget Effects Placeholder

> **UI-only.** Per-effect parameter editors (LUT, dewarp 360, opacity,
> shape mask, anamorphic, remove-background). **No GPU shader work, no
> MetalPetal port.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 33
**Functional integration:** Permanently deferred.

**Moblin source analogues** (7 files):
- `View/Settings/Scenes/Widgets/Widget/Effects/WidgetEffectsView.swift` — root
- `…/AnamorphicLensEffectView.swift`
- `…/Dewarp360EffectView.swift`
- `…/LutEffectView.swift`
- `…/OpacityEffectView.swift`
- `…/RemoveBackgroundEffectView.swift`
- `…/ShapeEffectView.swift`

**Files to add:**
- `senders/android/ui/pages/widget_effects_page.slint` — root list
- `senders/android/ui/pages/widget_effect_<kind>_page.slint` × 6

---

## Goal

Render Moblin's effects parameter forms — sliders, color pickers, file
pickers — without any GPU pipeline behind them.

---

## Moblin source pattern

```swift
// LutEffectView.swift (excerpt)
List {
    Section {
        ForEach(database.color.bundledLuts) { lut in
            Button { setLut(id: lut.id) } label: {
                HStack { Image(...); Text(lut.name); Spacer(); if isSelected { ... } }
            }
        }
    } header: { Text("Bundled LUTs") }

    Section {
        Slider(value: $effect.intensity, in: 0...1)
    } header: { Text("Intensity") }
}
```

Each effect is a list of parameters with a slider/toggle/picker each.

---

## Tasks

### 36-A — `WidgetEffectsPage` (root list)

```slint
export struct WidgetEffect {
    id: string,
    label: string,
    enabled: bool,
    panel: Panel,
}

export component WidgetEffectsPage inherits Rectangle {
    in-out property <[WidgetEffect]> mock-effects: [
        { id: "lut",        label: "LUT (color grading)", enabled: false, panel: Panel.widget-effect-lut },
        { id: "anamorphic", label: "Anamorphic lens",     enabled: false, panel: Panel.widget-effect-anamorphic },
        { id: "dewarp",     label: "360° dewarp",         enabled: false, panel: Panel.widget-effect-dewarp },
        { id: "opacity",    label: "Opacity",             enabled: true,  panel: Panel.widget-effect-opacity },
        { id: "shape",      label: "Shape mask",          enabled: false, panel: Panel.widget-effect-shape },
        { id: "remove-bg",  label: "Remove background",   enabled: false, panel: Panel.widget-effect-remove-bg },
    ];

    VerticalLayout {
        for e in root.mock-effects: HorizontalLayout {
            SettingsToggleRow { label: e.label; checked: e.enabled;
                toggled => { /* update mock-effects[i].enabled */ } }
            TextButton { label: "›";
                clicked => { Bridge.active-panel = e.panel; } }
        }
    }
}
```

### 36-B — `WidgetEffectLutPage`

- [ ] List of `mock-bundled-luts: [{ id: int, name: string }]` (10 stub
  presets), single-selection radio behaviour. Intensity slider 0–1.

### 36-C — `WidgetEffectAnamorphicPage`

- [ ] Squeeze ratio cycler (`["1.0x", "1.33x", "1.55x", "1.79x", "2.0x"]`),
  desqueeze direction picker.

### 36-D — `WidgetEffectDewarpPage`

- [ ] FOV slider (60–360°), pitch/yaw/roll sliders, projection picker
  (`["Fisheye", "Equirect", "Cylindrical"]`).

### 36-E — `WidgetEffectOpacityPage`

- [ ] Single slider 0–1 with live numeric value display.

### 36-F — `WidgetEffectShapePage`

- [ ] Shape picker (`["Circle", "Rounded rect", "Heart", "Star"]`),
  feather slider, size slider, X/Y position cyclers.

### 36-G — `WidgetEffectRemoveBgPage`

- [ ] Engine picker (`["Default", "Fast", "High quality"]`), threshold
  slider, edge-feather slider, "preview" rectangle.

---

## Exit criteria

1. `WidgetEffectsPage` lists 6 effects with toggle + chevron.
2. Toggling enables/disables (visually); navigating opens the per-effect
   page.
3. Each per-effect page renders its sliders/pickers.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Any GPU shader / MetalPetal / SkiaSL implementation.
- Real LUT file loading.
- Real background-segmentation engine (TFLite, MediaPipe).

---

## Slint best practices applied here

- **`mod(idx + 1, options.length)`** picker pattern is reused — no
  combobox sprawl.
- **`property <[T]>` array literal** lets the bundled LUT list ride
  inline with no Rust setup.
- **`SettingsToggleRow + chevron`** is the cheapest navigation pattern
  for "this effect is on; tap to configure" — matches the SwiftUI shape
  exactly.

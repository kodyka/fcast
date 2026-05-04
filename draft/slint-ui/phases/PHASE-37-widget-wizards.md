# Phase 37 — Widget Wizards Placeholder

> **UI-only.** First-run "create a new widget" wizards for each kind
> (text / image / slideshow / browser / bingo / PNGTuber / VTuber / video
> source / wheel-of-luck). **No real widget instantiation logic.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 30 (wizard chrome), 33 (widget library), 35 (per-widget pages)
**Functional integration:** Permanently deferred.

**Moblin source analogues** (9 files):
- `View/Settings/Scenes/Widgets/Widget/WidgetWizardSettingsView.swift` — root picker
- `Widget/Wizard/WidgetWizardBingoCardSettingsView.swift`
- `Widget/Wizard/WidgetWizardBrowserSettingsView.swift`
- `Widget/Wizard/WidgetWizardImageSettingsView.swift`
- `Widget/Wizard/WidgetWizardPngTuberSettingsView.swift`
- `Widget/Wizard/WidgetWizardSlideshowSettingsView.swift`
- `Widget/Wizard/WidgetWizardTextSettingsView.swift`
- `Widget/Wizard/WidgetWizardVTuberSettingsView.swift`
- `Widget/Wizard/WidgetWizardVideoSourceSettingsView.swift`
- `Widget/Wizard/WidgetWizardWheelOfLuckSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/widget_wizard_root_page.slint` — kind picker
- `senders/android/ui/pages/widget_wizard_<kind>_page.slint` × 9

---

## Goal

Provide a wizard-style step flow for creating new widgets, reusing the
`WizardStepChrome` component from Phase 30.

---

## Moblin source pattern

Each wizard is a 2–3 step flow (kind → name → essential parameters), then
calls back to the parent to create the widget. The placeholder ends at
"Save" by appending to the same `mock-widgets` list from Phase 33.

```swift
// WidgetWizardSettingsView.swift (excerpt)
List {
    NavigationLink { WidgetWizardTextSettingsView(...) } label: { Label("Text", systemImage: "textformat") }
    NavigationLink { WidgetWizardImageSettingsView(...) } label: { Label("Image", systemImage: "photo") }
    NavigationLink { WidgetWizardSlideshowSettingsView(...) } label: { Label("Slideshow", systemImage: "play.rectangle") }
    // ...
}
```

---

## Tasks

### 37-A — `WidgetWizardRootPage` (kind picker)

```slint
export struct WidgetKind {
    id: string,
    label: string,
    icon: image,           // optional — null in the placeholder
    panel: Panel,
}

export component WidgetWizardRootPage inherits Rectangle {
    in-out property <[WidgetKind]> mock-kinds: [
        { id: "text",       label: "Text",         icon: image{}, panel: Panel.widget-wizard-text },
        { id: "image",      label: "Image",        icon: image{}, panel: Panel.widget-wizard-image },
        { id: "slideshow",  label: "Slideshow",    icon: image{}, panel: Panel.widget-wizard-slideshow },
        { id: "browser",    label: "Browser source", icon: image{}, panel: Panel.widget-wizard-browser },
        { id: "bingo",      label: "Bingo card",   icon: image{}, panel: Panel.widget-wizard-bingo },
        { id: "pngtuber",   label: "PNGTuber",     icon: image{}, panel: Panel.widget-wizard-pngtuber },
        { id: "vtuber",     label: "VTuber",       icon: image{}, panel: Panel.widget-wizard-vtuber },
        { id: "video-src",  label: "Video source", icon: image{}, panel: Panel.widget-wizard-video-src },
        { id: "wheel",      label: "Wheel of luck", icon: image{}, panel: Panel.widget-wizard-wheel },
    ];

    VerticalLayout {
        for k in root.mock-kinds: SettingsValueRow {
            label: k.label; value: "Create";
            clicked => { Bridge.active-panel = k.panel; }
        }
    }
}
```

### 37-B — Per-kind wizard

Each `widget_wizard_<kind>_page.slint` uses `WizardStepChrome` from
Phase 30:

- **Step 0** — Name (`SettingsTextRow`)
- **Step 1** — Kind-specific essentials (e.g. text format string,
  image filename, slideshow first image, browser URL, bingo size,
  VTuber model file, etc.)
- **Step 2** — Confirm + "Save"

```slint
// widget_wizard_text_page.slint
import { WizardStepChrome } from "../components/wizard_step_chrome.slint";

export component WidgetWizardTextPage inherits Rectangle {
    in-out property <int>    mock-step: 0;
    in-out property <string> mock-name: "Text widget";
    in-out property <string> mock-format: "Hello, world!";
    in-out property <int>    mock-font-size: 32;

    VerticalLayout {
        if root.mock-step == 0:
        SettingsTextRow { label: "Name"; text: root.mock-name;
                          edited(s) => { root.mock-name = s; } }

        if root.mock-step == 1:
        VerticalLayout {
            SettingsTextRow { label: "Text"; text: root.mock-format;
                              edited(s) => { root.mock-format = s; } }
            SettingsSliderRow { label: "Size"; min: 10; max: 96;
                                value: root.mock-font-size;
                                changed(v) => { root.mock-font-size = v; } }
        }

        if root.mock-step == 2:
        VerticalLayout {
            Text { text: "Create '\{root.mock-name}'?"; }
            Text { text: "Text: " + root.mock-format; }
        }

        WizardStepChrome {
            step: root.mock-step; total-steps: 3;
            back => { root.mock-step = root.mock-step - 1; }
            next => { root.mock-step = root.mock-step + 1; }
            cancel => { Bridge.active-panel = Panel.widgets; }
            finish => {
                // Append to parent's mock-widgets — passed via global?
                Bridge.active-panel = Panel.widgets;
            }
        }
    }
}
```

> The `Bridge.active-panel = Panel.widgets;` after Finish is the
> placeholder for "save the widget" — actual `mock-widgets` mutation
> happens via a Slint global; see Phase 8 (deferred) for how this
> migrates to a Rust setter.

### 37-C — Repeat for the other 8 kinds

Each gets its own 2–3 step flow with the kind's essential parameters:
- **Image** — file picker stub, fit picker.
- **Slideshow** — first image picker, transition picker.
- **Browser** — URL field, refresh interval.
- **Bingo** — board size cycler (3×3 / 5×5 / 7×7).
- **PNGTuber** — image folder picker, talk threshold.
- **VTuber** — model file picker, calibration tap.
- **Video source** — source picker.
- **Wheel of luck** — initial item count, color palette picker.

---

## Exit criteria

1. `WidgetWizardRootPage` lists 9 widget kinds.
2. Each wizard cycles through its 3 steps.
3. "Finish" returns to `Panel.widgets`.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Actual widget creation in any persistent store.
- File / image / model picker dialogs.
- Validation of names (no duplicate-detection).

---

## Slint best practices applied here

- **`if root.mock-step == N: ...`** branching for wizard steps is
  preferred over a `Switch`-style selector — Slint's conditional
  element instantiation is cheap, and it keeps the file linear.
- **`WizardStepChrome` reuse** from Phase 30 keeps the navigation
  language consistent across all wizards.
- **Slint global for inter-page mutation** is the post-UI migration
  story — for now, mutating across panels is parked in Phase 8.

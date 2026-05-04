# Phase 25 — Macros & Action Chains Placeholder

> Settings sub-page for defining macros (named action chains). **UI-only.**
> Macros are stored as inline mock model and run no real commands.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7, 17 (quick-action customization)
**Functional integration:** Deferred — needs a macro execution engine in Rust.
**Moblin source analogues:**
- `Settings/Macros/MacrosSettingsView.swift`
- `ControlBar/QuickButton/QuickButtonMacrosView.swift`

**Files:**
- `senders/android/ui/pages/macros_page.slint` — new (list)
- `senders/android/ui/pages/macro_edit_page.slint` — new (per-macro editor)
- `senders/android/ui/bridge.slint` — `Macro` + `MacroStep` structs + `Panel.macros*`

---

## Tasks

### 25-A — `Macro` / `MacroStep` structs

- [ ] In `bridge.slint`:

  ```slint
  export struct MacroStep {
      action-id: string,    // matches QuickAction.id
      label:     string,    // display label
  }

  export struct Macro {
      id:       string,
      name:     string,
      steps:    [MacroStep],
      enabled:  bool,
  }
  ```

---

### 25-B — `MacrosPage` (list)

- [ ] Inline mock model:

  ```slint
  in-out property <[Macro]> mock-macros: [
      {
          id: "m1", name: "Start morning cast", enabled: true,
          steps: [
              { action-id: "scan-qr",       label: "Scan QR" },
              { action-id: "audio",         label: "Open Audio" },
              { action-id: "record",        label: "Start Recording" },
          ],
      },
      {
          id: "m2", name: "Quick stop", enabled: true,
          steps: [
              { action-id: "stop-recording", label: "Stop Recording" },
              { action-id: "stop-cast",      label: "Stop Cast" },
          ],
      },
  ];
  ```

- [ ] Each row: name + step count badge ("3 steps") + enable toggle. Tap row
  → opens edit page with selected macro id.
- [ ] Trailing "Add macro" `PrimaryButton` opens a blank edit page.

---

### 25-C — `MacroEditPage`

- [ ] Top: name `LineEdit`, enable toggle.
- [ ] Body: vertical list of step rows (label + remove button + ▲▼ reorder).
- [ ] Bottom: "Add step" `PrimaryButton` opens an inline picker showing all
  available action ids (sourced from a hard-coded list of bar action ids).
- [ ] Save / Cancel buttons in the header. Save returns to the list page;
  in UI-only build, neither persists past the macros list's `in-out`
  property.

---

### 25-D — Quick-action shortcut for "Run macro X"

- [ ] In Phase 17's quick-action customization, document that macros can be
  surfaced as bar actions with id `"macro:<id>"`. This phase wires the visual
  side: when a bar action's id starts with `macro:`, render its title with a
  small ▶ glyph prefix.
- [ ] The glyph prefix is purely cosmetic — actual macro execution wiring is
  parked in `futures/`.

---

### 25-E — Bridge + linking

- [ ] Extend `Panel`: `macros`, `macro-edit`.
- [ ] Route in `main.slint`.
- [ ] Link from `FullSettingsPage` "AUTOMATION" section (new section
  introduced here).

---

## Exit criteria

1. Macros list page renders 2 stub macros with step-count badges.
2. Tap row → edit page opens populated; tap "Add macro" → edit page opens
   blank.
3. Step reorder (▲▼) shifts steps within a macro's `steps` array.
4. Add step picker shows all known action ids; selecting one appends to the
   list.
5. Save returns to list (no real persistence).
6. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real macro execution (no scheduler / runner in Rust).
- Step types beyond simple action invocation (e.g. delays, conditions).
- Macro export / import.
- Macro recording from user actions.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Macros/MacrosSettingsView.swift`
- `View/ControlBar/QuickButton/QuickButtonMacrosView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Macros/MacrosSettingsView.swift (excerpt)
Form {
    Section {
        ForEach(database.macros) { macro in
            NavigationLink { MacroEditView(macro: macro) } label: {
                Toggle(macro.name, isOn: $macro.enabled)
            }
        }
    }
}
```

**Mapping notes:**

A list of saved macros, each editable as an ordered list of action steps.
Per-macro editor uses the same drag-handle / reorder pattern from Phase
16/17 (▲/▼ buttons). Each step row is a `SettingsValueRow` cycling
through known action IDs (toggle-cast / scan-qr / start-recording / etc).

**Relevant Slint docs:**
- [for index in array](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/repetition.mdx)

## Slint best practices applied here

- **Two-page list/edit pattern** mirrors Phase 16's bitrate presets — keep
  consistent navigation patterns across settings sub-features.
- **`for step in macro.steps:`** drives the editor view from the macro's own
  data, not a separate selection model. Removing / reordering edits the
  array in place.
- **Cosmetic-only glyph prefix for `macro:` action ids** keeps quick-action
  customization (Phase 17) and macros loosely coupled — no enum addition,
  just a string-prefix convention.

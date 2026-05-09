# Phase 17 — Quick-Action Customization Placeholder

> Settings sub-page for reordering / hiding / labelling quick actions in the
> control bar. **UI-only.** Drag-reorder is faked with up/down arrow buttons;
> changes mutate the inline mock list.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 4, 7
**Functional integration:** Deferred — the control bar's mock model is
replaced wholesale on each "save"; no persistence.
**Moblin source analogues:**
- `Settings/Display/QuickButtons/QuickButtonsSettingsView.swift`
- `Settings/Display/QuickButtons/QuickButtonsButtonSettingsView.swift`

**Files:**
- `senders/android/ui/pages/quick_actions_page.slint` — new
- `senders/android/ui/bridge.slint` — `Panel.quick-actions` variant

---

## Tasks

### 17-A — `QuickActionsPage` list

- [ ] Stub model:

  ```slint
  in-out property <[QuickAction]> mock-bar-actions: [
      { id: "scan-qr",         title: "Scan QR",          enabled: true,  active: false },
      { id: "settings",        title: "Settings",         enabled: true,  active: false },
      { id: "debug",           title: "Debug",            enabled: false, active: false },
      { id: "codec-test",      title: "Codec Test",       enabled: true,  active: false },
      { id: "audio",           title: "Audio",            enabled: true,  active: false },
  ];
  ```

- [ ] Render each entry as a row with: title, an enable toggle, and a pair of
  up/down arrows to reorder.
- [ ] Reorder logic in Slint: when the user clicks ▲, swap the entry with the
  preceding one in `mock-bar-actions` by writing a new array literal. Slint
  does not have an in-place `splice`; the idiomatic pattern is to rebuild the
  list:

  ```slint
  // Pseudocode — actual implementation uses index-based array building.
  if (idx > 0) {
      root.mock-bar-actions = swap-pair(root.mock-bar-actions, idx, idx - 1);
  }
  ```

  Implementation hint: pre-bind a `swap` helper as an inline `pure function`
  on the page if Slint version supports it; otherwise Rust ships the model
  later and reorder is a Rust-side concern.

---

### 17-B — Empty / over-full handling

- [ ] If `mock-bar-actions.length == 0`, show an empty-state card: "Tap an
  action below to add it to the bar."
- [ ] If `mock-bar-actions.length > 6`, show an info banner: "Bar holds up to
  6 actions — extras will overflow."

---

### 17-C — Bridge + linking

- [ ] Extend `Panel` enum with `quick-actions`.
- [ ] In `FullSettingsPage`, link to it from a "DISPLAY" section row.
- [ ] In `main.slint`, route `Panel.quick-actions` → `QuickActionsPage`.

---

## Exit criteria

1. List renders 5 stub actions with enable-toggle + ▲/▼ buttons.
2. Toggling enable flips the row's stub state.
3. Reordering with arrows visually shifts entries (UI-only — no persistence).
4. Overflow info banner appears when count > 6 (test by temporarily padding
   the mock list).
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Persisting reorder / enable changes.
- Drag-and-drop reorder gesture (Slint lacks a built-in drag API in 1.15.x;
  arrow-button reorder is the pragmatic placeholder).
- Per-action label customization beyond the default `title`.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/ControlBar/QuickButtonsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/ControlBar/QuickButtonsView.swift (excerpt — reorder)
List {
    ForEach(database.quickButtons) { button in
        Toggle(button.label, isOn: $button.enabled)
    }
    .onMove { from, to in database.quickButtons.move(fromOffsets: from, toOffset: to) }
}
```

**Mapping notes:**

The order of action cards in the Cast Control Bar is editable on a
"Customize quick actions" page. Each row shows the action label, an
enabled toggle, and ▲/▼ reorder buttons. Reorder mutates
`mock-quick-actions` via `[...].swap(i, j)`-style operations expressed
as new array literals.

**Relevant Slint docs:**
- [ListView reorder pattern](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx)

## Slint best practices applied here

- **Up/down arrow reorder is the simplest model mutation pattern.** Real
  drag-and-drop would require either a Slint version with a `Draggable`
  component or Rust-side gesture coordination — neither is available in the
  pinned futo fork.
- **Banner-on-overflow is just a `Rectangle` with `if list.length > N`.**
  No need for a separate notification system.

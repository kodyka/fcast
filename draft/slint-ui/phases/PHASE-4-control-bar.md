# Phase 4 — Control Bar and Quick Actions

> Build the persistent bottom action strip and model-driven quick action buttons.
> Inspired by Moblin `View/ControlBar` (23 files); replaces the hardcoded debug button list.
> Reference: `draft/moblin-ui/Moblin/View/ControlBar/`

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 3 (buttons)
**Unlocks:** Phase 5 (status overlay sits above the control bar), Phase 8 (Rust populates actions)
**Related files:**
- `senders/android/ui/components/control_bar.slint` — stub from Phase 1-H
- `senders/android/ui/bridge.slint` — `QuickAction` struct + callbacks added here
- `senders/android/src/lib.rs` — Rust must populate `quick-actions` model

**Moblin source analogues (reference only):**

| Moblin file | Slint equivalent |
|---|---|
| `ControlBarPortraitView.swift` / `ControlBarLandscapeView.swift` | `CastControlBar` |
| `StreamButton.swift` | `CastButton` |
| `QuickButtonsView.swift` | `QuickActionButton` model loop |
| `BatteryView.swift` | `StatusBadge` (simplified, Phase 5) |

---

## Tasks

### 4-A — Add `QuickAction` struct to `bridge.slint`

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```
  export struct QuickAction {
      id:      string,
      title:   string,
      enabled: bool,
      active:  bool,
  }
  ```

- [ ] Add to `Bridge`:

  ```
      in property <[QuickAction]> quick-actions: [];
      callback invoke-action(string);
  ```

- [ ] **Build check** — Rust `lib.rs` may need updating if it already accesses `Bridge` properties.

---

### 4-B — Implement `QuickActionButton` in `control_bar.slint`

- [ ] Open `senders/android/ui/components/control_bar.slint`.
- [ ] Add imports: `import { Theme } from "../theme.slint"; import { Bridge, QuickAction } from "../bridge.slint";`.
- [ ] Implement `export component QuickActionButton`:

  ```
  export component QuickActionButton inherits Rectangle {
      in property <QuickAction> action;
      callback invoked(string);

      height: 48px;
      width: 80px;
      border-radius: Theme.radius-card;
      background: root.action.active
          ? Theme.accent-active
          : (ta.pressed ? Theme.surface-card.brighter(20%) : Theme.surface-card);
      opacity: root.action.enabled ? 1.0 : 0.45;

      ta := TouchArea {
          enabled: root.action.enabled;
          clicked => { root.invoked(root.action.id); }
      }
      Text {
          text: root.action.title;
          color: Theme.text-primary;
          horizontal-alignment: center;
          vertical-alignment: center;
          font-size: Theme.font-size-label;
          wrap: word-wrap;
      }
  }
  ```

- [ ] **Build check.**

---

### 4-C — Implement `CastControlBar` in `control_bar.slint`

- [ ] Implement `export component CastControlBar`:

  ```
  export component CastControlBar inherits Rectangle {
      height: Theme.control-bar-height;
      background: Theme.surface-bar;

      HorizontalLayout {
          padding: Theme.padding-card;
          spacing: Theme.spacing-default;
          alignment: start;

          for action in Bridge.quick-actions: QuickActionButton {
              action: action;
              invoked(id) => { Bridge.invoke-action(id); }
          }
      }
  }
  ```

  > Phase 2-A defines `Theme.surface-bar` (`#111827`). Do not embed raw hex literals in
  > component files — Phase 2-J will fail the audit if you do.

- [ ] **Build check.**

---

### 4-D — Add `CastControlBar` to `main.slint`

- [ ] Import `CastControlBar` in `main.slint`.
- [ ] Restructure `MainWindow` to use a `VerticalLayout` so the page stack and the
  control bar share the screen automatically (no per-page bottom padding needed):

  ```
  export component MainWindow inherits Window {
      VerticalLayout {
          // Page stack — takes the remaining vertical space.
          // The Rectangle wrapper is intentional: layouts position children, but the
          // `if Bridge.app-state == ...` conditionals need a parent that positions
          // them at (0, 0) covering the full slot, not stacked vertically.
          Rectangle {
              vertical-stretch: 1;
              if Bridge.app-state == AppState.Disconnected:      ConnectView { }
              if Bridge.app-state == AppState.Connecting:        ConnectingView { }
              if Bridge.app-state == AppState.SelectingSettings: SettingsPageView { }
              if Bridge.app-state == AppState.WaitingForMedia:   WaitingForMediaView { }
              if Bridge.app-state == AppState.Casting:           CastingView { }
          }

          // Control bar — takes its natural height at the bottom.
          CastControlBar { }
      }
  }
  ```

- [ ] **Why `VerticalLayout` instead of absolute positioning?** Using
  `CastControlBar { y: parent.height - self.height; ... }` as a sibling of the page
  conditionals causes the bottom `Theme.control-bar-height` band of every page to be
  hidden behind the bar. The layout above subtracts the bar height once and gives
  pages the rest. Phase 7's `Panel` overlays should be added as siblings of the
  `VerticalLayout` (not inside it), so they overlay both the pages and the bar.

- [ ] Verify the bar is pinned to the bottom in `slint-viewer` or on device.
- [ ] **Build check.**

---

### 4-E — Implement `CastButton` state machine in `casting_page.slint`

Replace the simple "Stop" / "Cancel" buttons in `CastingView` and `WaitingForMediaView`
with a context-aware `CastButton` that changes label and style based on `AppState`.

- [ ] Import `Bridge, AppState` from `bridge.slint`, `PrimaryButton`, `DestructiveButton`, `LoadingView`
  from components.
- [ ] Add a `CastButton` component or inline state logic. The button only renders inside
  `WaitingForMediaView` and `CastingView`, so only those states need a row in the table:

  | State | Button text | Style | Callback |
  |---|---|---|---|
  | `WaitingForMedia` | "Cancel" | `DestructiveButton` | `Bridge.stop-casting()` |
  | `Casting` | "Stop Casting" | `DestructiveButton` | `Bridge.stop-casting()` |

  > `Disconnected` and `Connecting` route to `ConnectView` / `ConnectingView` in
  > `MainWindow`, which already provide their own primary actions ("Scan QR" /
  > "Cancel"). Putting them in `CastButton` would be dead code unless `CastButton` is
  > later moved into `CastControlBar` (a possible refinement in Phase 7).

- [ ] Confirm spinner is shown for `Connecting` and `WaitingForMedia` via `LoadingView` from Phase 3.
- [ ] **Build check.**

---

### 4-F — Wire existing debug callbacks through `invoke-action`

Replace the direct debug callback buttons in `debug_page.slint` with quick actions, so the
debug panel becomes data-driven.

- [ ] Define action ids as constants or literals in `debug_page.slint`:

  | Action id | Old callback |
  |---|---|
  | `"scan-qr"` | `Bridge.scan-qr()` |
  | `"migrated-server"` | `Bridge.start-migrated-server()` |
  | `"test-getinfo"` | `Bridge.test-legacy-getinfo()` |
  | `"test-crossfade"` | `Bridge.test-legacy-crossfade()` |
  | `"test-smoke"` | `Bridge.test-smoke-graph()` |

- [ ] In Rust `lib.rs`, add handler for `Bridge.invoke-action`:

  ```rust
  bridge.on_invoke_action(move |id| {
      match id.as_str() {
          "scan-qr"           => { /* call scan_qr logic */ }
          "migrated-server"   => { /* start migrated server */ }
          "test-getinfo"      => { /* run getinfo */ }
          "test-crossfade"    => { /* run crossfade */ }
          "test-smoke"        => { /* run smoke graph */ }
          _                   => {}
      }
  });
  ```

- [ ] The original direct callbacks (`Bridge.scan-qr`, `Bridge.start-migrated-server`, etc.) can
  remain in `bridge.slint` for now — migrate them to `invoke-action` incrementally.
- [ ] **Build check.**

---

### 4-G — Populate `quick-actions` from Rust

- [ ] In `lib.rs`, after `slint::include_modules!()`, populate initial quick actions:

  ```rust
  let actions = vec![
      QuickAction { id: "scan-qr".into(), title: "Scan QR".into(), enabled: true, active: false },
  ];
  // Only add debug actions when show_debug is true:
  if show_debug {
      actions.extend([
          QuickAction { id: "migrated-server".into(), title: "Start Server".into(), enabled: true, active: false },
          QuickAction { id: "test-getinfo".into(),    title: "GetInfo".into(),      enabled: true, active: false },
          QuickAction { id: "test-crossfade".into(),  title: "Crossfade".into(),    enabled: true, active: false },
          QuickAction { id: "test-smoke".into(),      title: "Smoke Graph".into(),  enabled: true, active: false },
      ]);
  }
  let model = std::rc::Rc::new(slint::VecModel::from(actions));
  bridge.set_quick_actions(model.into());
  ```

- [ ] Verify control bar renders the expected buttons.
- [ ] **Build check.**

---

## Exit criteria

Phase 4 is complete when:

1. `control_bar.slint` exports `QuickActionButton` and `CastControlBar`.
2. `bridge.slint` defines `QuickAction` struct and `invoke-action` callback.
3. `CastControlBar` is visible at the bottom of `MainWindow` in all app states.
4. All original debug test actions are reachable through `invoke-action`.
5. `CastButton` shows the correct label/style for each of the 5 `AppState` values.
6. `cargo build -p android-sender` passes cleanly.

---

## Notes

- The `VerticalLayout` chassis in 4-D handles the bar/page split automatically — no
  per-page `padding-bottom` is needed. Phase 7's `Panel` overlays should be added as
  siblings of the `VerticalLayout` so they overlay both the page and the bar.
- Portrait-only for now; landscape variant (horizontal sidebar) can be added later if FCast
  sender needs landscape locking. Moblin's `ControlBarLandscapeView` is the reference.

---

## Slint best practices applied here

- **Model-driven `for` loops are the canonical way to render a variable list of items.**
  `for action in Bridge.quick-actions: QuickActionButton { ... }` is reactive: when Rust
  pushes/removes from the underlying `VecModel`, the bar updates without any explicit
  refresh logic.
- **Layouts subtract space, absolute positioning fights for it.** `VerticalLayout` with
  `vertical-stretch: 1` on the page slot gives the bar its natural height and the pages
  the rest, no math required. Reference:
  [`guide/language/coding/positioning-and-layouts.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx).
- **Theme tokens for surface colors, never raw hex.** Phase 2-J grep audit will fail if
  you re-introduce hex literals here. Always add a token first.

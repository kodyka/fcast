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
      background: #111827;   // TODO Phase 2: add Theme.surface-bar token

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

- [ ] **Build check.**

---

### 4-D — Add `CastControlBar` to `main.slint`

- [ ] Import `CastControlBar` in `main.slint`.
- [ ] Add to `MainWindow`:

  ```
  export component MainWindow inherits Window {
      // ... existing AppState page routing ...

      CastControlBar {
          y: parent.height - self.height;
          width: parent.width;
      }
  }
  ```

- [ ] Verify the bar is pinned to the bottom in `slint-viewer` or on device.
- [ ] **Build check.**

---

### 4-E — Implement `CastButton` state machine in `casting_page.slint`

Replace the simple "Stop" / "Cancel" buttons with a context-aware `CastButton` that changes
label and style based on `AppState`.

- [ ] Import `Bridge, AppState` from `bridge.slint`, `PrimaryButton`, `DestructiveButton`, `LoadingView`
  from components.
- [ ] Add a `CastButton` component or inline state logic in `CastingView`:

  | State | Button text | Style | Callback |
  |---|---|---|---|
  | `Disconnected` | "Connect" | `PrimaryButton` | `Bridge.scan-qr()` |
  | `Connecting` | "Cancel" | `DestructiveButton` | `Bridge.stop-casting()` |
  | `WaitingForMedia` | "Cancel" | `DestructiveButton` | `Bridge.stop-casting()` |
  | `Casting` | "Stop Casting" | `DestructiveButton` | `Bridge.stop-casting()` |

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

- The control bar overlaps page content at the bottom. Pages should add `padding-bottom:
  Theme.control-bar-height` or equivalent to avoid content being hidden behind the bar.
  This is best addressed in Phase 7 when the full page layouts are finalized.
- Portrait-only for now; landscape variant (horizontal sidebar) can be added later if FCast
  sender needs landscape locking. Moblin's `ControlBarLandscapeView` is the reference.

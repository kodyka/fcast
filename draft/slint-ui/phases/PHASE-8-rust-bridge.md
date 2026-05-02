# Phase 8 — Rust Bridge Hardening

> Wire all new Slint structs and callbacks introduced in Phases 4–7 into `lib.rs`.
> Keep Slint globals shallow. Do not recreate Moblin's monolithic `Model` — use
> small, focused Rust helpers that push data into typed Slint models.

**Status:** `[ ] Not started`
**Depends on:** Phase 4 (`QuickAction`), Phase 5 (`StatusItem`), Phase 6 (`ReceiverItem`), Phase 7 (`Panel`, `app-version`)
**Related files:**
- `senders/android/src/lib.rs` — all bridge wiring lives here
- `senders/android/ui/bridge.slint` — source of truth for the Slint API surface

---

## New Bridge properties introduced across Phases 4–7

| Phase | Property / Callback | Rust type | Direction |
|---|---|---|---|
| 4 | `quick-actions: [QuickAction]` | `VecModel<QuickAction>` | Rust → Slint |
| 4 | `invoke-action(string)` | closure / handler | Slint → Rust |
| 5 | `status-items: [StatusItem]` | `VecModel<StatusItem>` | Rust → Slint |
| 6 | `devices: [ReceiverItem]` | `VecModel<ReceiverItem>` | Rust → Slint |
| 7 | `active-panel: Panel` | `Panel` enum | bidirectional |
| 7 | `open-panel(Panel)` | closure | Slint → Rust |
| 7 | `close-panel()` | closure | Slint → Rust |
| 7 | `app-version: string` | `SharedString` | Rust → Slint |
| 7 | `mdns-enabled: bool` | `bool` | bidirectional |
| 7 | `set-mdns(bool)` | closure | Slint → Rust |

---

## Tasks

### 8-A — Create `ui_state.rs` helper module

Rather than putting all model management directly in `lib.rs`, extract it into a small
`ui_state` module:

- [ ] Create `senders/android/src/ui_state.rs`.
- [ ] Define a struct `UiState` that holds `Weak<MainWindow>` and the shared models:

  ```rust
  use std::rc::Rc;
  use slint::{VecModel, Weak};
  use crate::{MainWindow, QuickAction, ReceiverItem, StatusItem, Panel};

  pub struct UiState {
      handle:        Weak<MainWindow>,
      quick_actions: Rc<VecModel<QuickAction>>,
      receivers:     Rc<VecModel<ReceiverItem>>,
      status_items:  Rc<VecModel<StatusItem>>,
  }
  ```

- [ ] Add `pub mod ui_state;` to `lib.rs`.
- [ ] **Build check.**

---

### 8-B — Wire `quick-actions` model in Rust

- [ ] In `ui_state.rs`, implement `fn init(handle: Weak<MainWindow>) -> UiState`:

  ```rust
  pub fn init(handle: Weak<MainWindow>) -> Self {
      let quick_actions = Rc::new(VecModel::default());
      let receivers     = Rc::new(VecModel::default());
      let status_items  = Rc::new(VecModel::default());

      handle.upgrade_in_event_loop(move |w| {
          let bridge = w.global::<Bridge>();
          bridge.set_quick_actions(quick_actions.clone().into());
          bridge.set_devices(receivers.clone().into());
          bridge.set_status_items(status_items.clone().into());
      }).ok();

      Self { handle, quick_actions, receivers, status_items }
  }
  ```

- [ ] Call `UiState::init` after the `MainWindow` is created in `lib.rs`.
- [ ] **Build check.**

---

### 8-C — Wire `invoke-action` callback

- [ ] In `lib.rs`, in the post-init setup block, register:

  ```rust
  let bridge = window.global::<Bridge>();
  bridge.on_invoke_action({
      let weak = window.as_weak();
      move |id| {
          match id.as_str() {
              "scan-qr"         => { /* existing scan_qr logic */ }
              "migrated-server" => { /* start migrated server */ }
              "test-getinfo"    => { /* run getinfo */ }
              "test-crossfade"  => { /* run crossfade */ }
              "test-smoke"      => { /* run smoke test */ }
              "settings"        => {
                  if let Some(w) = weak.upgrade() {
                      w.global::<Bridge>().set_active_panel(Panel::Settings);
                  }
              }
              _ => {}
          }
      }
  });
  ```

- [ ] Keep old direct callbacks (`on_scan_qr`, `on_start_migrated_server`, etc.) as thin
  wrappers calling the same logic — do not remove them yet until confirmed no callers remain.
- [ ] **Build check.**

---

### 8-D — Wire `open-panel` and `close-panel` callbacks

- [ ] Register `on_open_panel`:

  ```rust
  bridge.on_open_panel({
      let weak = window.as_weak();
      move |panel| {
          if let Some(w) = weak.upgrade() {
              w.global::<Bridge>().set_active_panel(panel);
          }
      }
  });
  ```

- [ ] Register `on_close_panel`:

  ```rust
  bridge.on_close_panel({
      let weak = window.as_weak();
      move || {
          if let Some(w) = weak.upgrade() {
              w.global::<Bridge>().set_active_panel(Panel::None);
          }
      }
  });
  ```

- [ ] **Build check.**

---

### 8-E — Update `devices` model to push `ReceiverItem`

- [ ] Find the existing `DeviceEvent` handler in `lib.rs` (where `Bridge.devices` is updated).
- [ ] Replace the `String` push with `ReceiverItem`:

  ```rust
  DeviceEvent::Added(info) => {
      ui_state.receivers.push(ReceiverItem {
          name:    info.name.clone().into(),
          address: info.address.to_string().into(),
      });
  }
  DeviceEvent::Removed(addr) => {
      let addr_str: slint::SharedString = addr.to_string().into();
      if let Some(idx) = (0..ui_state.receivers.row_count())
          .find(|&i| ui_state.receivers.row_data(i).map_or(false, |r| r.address == addr_str))
      {
          ui_state.receivers.remove(idx);
      }
  }
  ```

- [ ] Confirm `connect-receiver` callback still receives the address string.
- [ ] **Build check.**

---

### 8-F — Populate `app-version` at startup

- [ ] After `MainWindow` is created:

  ```rust
  let bridge = window.global::<Bridge>();
  bridge.set_app_version(env!("CARGO_PKG_VERSION").into());
  ```

- [ ] **Build check.**

---

### 8-G — Populate `status-items` during cast lifecycle

- [ ] When `AppState` transitions to `Casting`, push initial items:

  ```rust
  fn on_cast_started(ui_state: &UiState, receiver_name: &str, encoder: &str) {
      let items = vec![
          StatusItem { label: "Receiver".into(), value: receiver_name.into(), severity: "info".into() },
          StatusItem { label: "Encoder".into(),  value: encoder.into(),       severity: "info".into() },
      ];
      // Replace the model contents
      ui_state.status_items.set_vec(items);
  }

  fn on_cast_stopped(ui_state: &UiState) {
      ui_state.status_items.set_vec(vec![]);
  }
  ```

- [ ] When a cast error occurs, update the affected item's severity:

  ```rust
  fn on_encoder_error(ui_state: &UiState, message: &str) {
      if let Some(mut item) = ui_state.status_items.row_data(1) {
          item.severity = "error".into();
          item.value    = message.into();
          ui_state.status_items.set_row_data(1, item);
      }
  }
  ```

- [ ] **Build check.**

---

### 8-H — Guard all Slint calls with `upgrade_in_event_loop`

- [ ] Audit every place `lib.rs` mutates a Bridge property or model.
- [ ] Confirm every mutation either:
  - Is already inside a Slint event loop callback (safe), or
  - Uses `handle.upgrade_in_event_loop(move |w| { ... })` to post to the UI thread.
- [ ] No `Rc<VecModel<T>>` should be mutated from a non-UI thread without `upgrade_in_event_loop`.
- [ ] **Build check.**

---

### 8-I — Remove deprecated direct callbacks

Once `invoke-action` fully covers all debug actions:

- [ ] Remove `on_start_migrated_server` direct callback registration from `lib.rs`.
- [ ] Remove `on_test_legacy_getinfo`, `on_test_legacy_crossfade`, `on_test_smoke_graph`.
- [ ] Remove corresponding callbacks from `bridge.slint`.
- [ ] **Build check.**
- [ ] Do this as a separate commit after confirming no regressions.

---

## Exit criteria

Phase 8 is complete when:

1. `ui_state.rs` module exists and manages all `VecModel` instances.
2. `quick-actions`, `devices`, `status-items` are all fed from Rust `VecModel` instances.
3. `invoke-action`, `open-panel`, `close-panel`, `set-mdns` callbacks are all wired in Rust.
4. `app-version` is set at startup.
5. No `Rc<VecModel<T>>` is mutated outside the Slint event loop thread.
6. `cargo build -p android-sender` passes.
7. `cargo test -p android-sender` passes.

---

## Notes

- Keep `UiState` small. Its job is only to hold the model `Rc`s and the window `Weak`.
  Do **not** put business logic, GStreamer state, or JNI calls in `UiState`.
- `Weak<MainWindow>` avoids the callback ownership cycle that would occur with `Rc<MainWindow>`.
  Always use `weak.upgrade()` and handle the `None` case (window may have been destroyed).
- `slint::VecModel::set_vec` replaces the entire contents atomically — prefer it over
  individual `push`/`remove` calls when the list is fully replaced (e.g. on cast start).

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
`ui_state` module. **`UiState` only holds the `Weak<MainWindow>`** — it does **not**
store `Rc<VecModel<T>>` directly, because `Rc` is `!Send` and any task spawned from
a background thread (mDNS, GStreamer callbacks, JNI) would refuse to compile.

- [ ] Create `senders/android/src/ui_state.rs`.
- [ ] Define a struct `UiState` that holds only the window handle:

  ```rust
  use std::rc::Rc;
  use slint::{ComponentHandle, Model, SharedString, VecModel, Weak};
  use crate::{MainWindow, QuickAction, ReceiverItem, StatusItem, StatusSeverity, Panel};

  /// UI thread is the only owner of the MainWindow handle. Cloning a `Weak`
  /// is `Send`, so this struct can travel across threads (tokio tasks, JNI
  /// callbacks, GStreamer bus handlers).
  #[derive(Clone)]
  pub struct UiState {
      handle: Weak<MainWindow>,
  }
  ```

- [ ] Add `pub mod ui_state;` to `lib.rs`.
- [ ] **Build check.**

---

### 8-B — Wire `quick-actions` model in Rust

- [ ] In `ui_state.rs`, implement `fn new(handle: Weak<MainWindow>) -> UiState`. The
  `Rc<VecModel<...>>` instances are constructed and consumed entirely on the UI thread
  inside `upgrade_in_event_loop` — they never escape to a background thread.

  ```rust
  use crate::Bridge;

  impl UiState {
      pub fn new(handle: Weak<MainWindow>) -> Self {
          let me = Self { handle };
          me.replace_quick_actions(vec![]);
          me.replace_devices(vec![]);
          me.replace_status_items(vec![]);
          me
      }

      /// Replace the entire quick-actions list.
      pub fn replace_quick_actions(&self, items: Vec<QuickAction>) {
          let _ = self.handle.upgrade_in_event_loop(move |w| {
              let model = Rc::new(VecModel::from(items));
              w.global::<Bridge>().set_quick_actions(model.into());
          });
      }

      pub fn replace_devices(&self, items: Vec<ReceiverItem>) {
          let _ = self.handle.upgrade_in_event_loop(move |w| {
              let model = Rc::new(VecModel::from(items));
              w.global::<Bridge>().set_devices(model.into());
          });
      }

      pub fn replace_status_items(&self, items: Vec<StatusItem>) {
          let _ = self.handle.upgrade_in_event_loop(move |w| {
              let model = Rc::new(VecModel::from(items));
              w.global::<Bridge>().set_status_items(model.into());
          });
      }
  }
  ```

- [ ] Call `UiState::new(window.as_weak())` after the `MainWindow` is created in `lib.rs`.
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
- [ ] Add an `add_device` / `remove_device` pair to `UiState` that hops back to the UI
  thread before mutating the `VecModel`. Keeping the model behind
  `upgrade_in_event_loop` is the only safe way to mutate it from a background
  discovery task. Note `use slint::Model;` so `row_count`/`row_data`/`remove` are in
  scope (these are trait methods, not inherent on `VecModel`):

  ```rust
  impl UiState {
      pub fn add_device(&self, info: DeviceInfo) {
          let item = ReceiverItem {
              name:    info.name.clone().into(),
              address: info.address.to_string().into(),
          };
          let _ = self.handle.upgrade_in_event_loop(move |w| {
              if let Some(model) = w.global::<Bridge>().get_devices()
                  .as_any().downcast_ref::<VecModel<ReceiverItem>>()
              {
                  model.push(item);
              }
          });
      }

      pub fn remove_device(&self, addr: SharedString) {
          let _ = self.handle.upgrade_in_event_loop(move |w| {
              if let Some(model) = w.global::<Bridge>().get_devices()
                  .as_any().downcast_ref::<VecModel<ReceiverItem>>()
              {
                  if let Some(idx) = (0..model.row_count())
                      .find(|&i| model.row_data(i).map_or(false, |r| r.address == addr))
                  {
                      model.remove(idx);
                  }
              }
          });
      }
  }
  ```

  Alternative pattern (simpler if discovery already has the full list): call
  `replace_devices(full_list)` instead of incremental add/remove.

- [ ] Wire `DeviceEvent::Added(info) => ui_state.add_device(info)` and
  `DeviceEvent::Removed(addr) => ui_state.remove_device(addr.to_string().into())`.
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

- [ ] When `AppState` transitions to `Casting`, replace the items via
  `replace_status_items` (which posts to the UI thread under the hood). Use the
  `StatusSeverity` enum from Phase 5-A, not magic strings:

  ```rust
  fn on_cast_started(ui_state: &UiState, receiver_name: &str, encoder: &str) {
      ui_state.replace_status_items(vec![
          StatusItem { label: "Receiver".into(), value: receiver_name.into(), severity: StatusSeverity::Info },
          StatusItem { label: "Encoder".into(),  value: encoder.into(),       severity: StatusSeverity::Info },
      ]);
  }

  fn on_cast_stopped(ui_state: &UiState) {
      ui_state.replace_status_items(vec![]);
  }
  ```

- [ ] When a cast error occurs, update the affected item's severity. The read-modify-write
  has to happen on the UI thread:

  ```rust
  pub fn set_encoder_error(&self, message: SharedString) {
      let _ = self.handle.upgrade_in_event_loop(move |w| {
          if let Some(model) = w.global::<Bridge>().get_status_items()
              .as_any().downcast_ref::<VecModel<StatusItem>>()
          {
              if let Some(mut item) = model.row_data(1) {
                  item.severity = StatusSeverity::Error;
                  item.value    = message;
                  model.set_row_data(1, item);
              }
          }
      });
  }
  ```

  > Avoid `VecModel::set_vec` — it is **not** part of the public API in older Slint
  > releases. The portable replacement is to construct a fresh `Rc<VecModel::from(vec)>`
  > and call `bridge.set_status_items(model.into())` (what `replace_status_items` does
  > above). This is also slightly faster than clear-and-push for full replacements.

- [ ] **Build check.**

---

### 8-H — Guard all Slint calls with `upgrade_in_event_loop`

- [ ] Audit every place `lib.rs` mutates a Bridge property or model.
- [ ] Confirm every mutation either:
  - Is already inside a Slint event loop callback (safe), or
  - Uses `handle.upgrade_in_event_loop(move |w| { ... })` to post to the UI thread.
- [ ] No `Rc<VecModel<T>>` should ever escape `upgrade_in_event_loop`. The pattern in
  this phase keeps every `Rc` short-lived and UI-thread-scoped — it is constructed
  inside the closure, handed to `set_<property>`, and then dropped.
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

- Keep `UiState` small. Its job is only to hold the `Weak<MainWindow>` handle.
  Do **not** put business logic, GStreamer state, or JNI calls in `UiState`.
- `Weak<MainWindow>` is `Send`, so `UiState` is `Send + Clone` and can be passed to
  any background task. Always use `upgrade_in_event_loop` to access the window;
  never `weak.upgrade()` directly from a non-UI thread.
- For full-list replacements, construct a fresh `Rc<VecModel::from(vec))>` and call
  `set_<property>(model.into())`. This is portable across Slint versions and avoids
  reaching for `VecModel::set_vec` (which may not be exposed in the pinned futo fork).

---

## Slint best practices applied here

- **`Rc<T>` is `!Send`. `Weak<MainWindow>` is `Send`.** All cross-thread state lives
  behind the `Weak`; the `Rc<VecModel<T>>` is constructed inside the
  `upgrade_in_event_loop` closure and dropped before the closure returns.
- **The `Model` trait must be in scope** to call `row_count` / `row_data` / `remove`.
  The `use slint::Model;` line at the top of `ui_state.rs` is mandatory.
- **Prefer `set_<property>(Rc::new(VecModel::from(vec)).into())` for full replacements**
  over per-item `push` / `remove`. Less code, atomic from the UI thread's perspective,
  and version-portable.
- **Use the typed Slint enums on the Rust side too.** `StatusSeverity::Info` is
  better than `"info".into()` — type-safe, exhaustive `match`, and survives renames.

# Phase 6 — Receiver List and Discovery UX

> Upgrade the flat `[string]` device list to a rich `ReceiverItem` model with name
> and address sub-text. Improve the empty-state and searching experience.

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 3 (components)
**Unlocks:** Phase 7 (settings can show receiver count), Phase 8 (Rust maps `DeviceInfo` to `ReceiverItem`)
**Related files:**
- `senders/android/ui/bridge.slint` — `ReceiverItem` struct + `devices` type change
- `senders/android/ui/pages/connect_page.slint` — `ListView` updated
- `senders/android/src/lib.rs` — `DeviceInfo` mapped to `ReceiverItem`

---

## Tasks

### 6-A — Add `ReceiverItem` struct to `bridge.slint`

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```
  export struct ReceiverItem {
      name:    string,
      address: string,
  }
  ```

- [ ] Change existing `in property <[string]> devices: []` to:

  ```
      in property <[ReceiverItem]> devices: [];
  ```

- [ ] **Build check** — `lib.rs` will fail to compile until 6-D updates the Rust side.
  Do 6-A and 6-D together in the same commit.

---

### 6-B — Update `connect_page.slint` `ListView`

- [ ] Open `senders/android/ui/pages/connect_page.slint`.
- [ ] Import `ReceiverItem` from `../bridge.slint`.
- [ ] Change the `for device in Bridge.devices` loop body from a single `Text` to a two-line card:

  ```
  for device in Bridge.devices: Rectangle {
      height: Theme.row-height + 18px;

      ta := TouchArea {
          clicked => Bridge.connect-receiver(device.address);
      }

      Rectangle {
          width: parent.width - 10px;
          height: parent.height - 8px;
          background: ta.pressed ? Theme.accent-pressed : Theme.surface-card;
          border-radius: Theme.radius-card;

          VerticalLayout {
              padding-left: Theme.padding-screen;
              padding-right: Theme.padding-screen;
              alignment: center;
              spacing: 2px;

              Text {
                  text: device.name;
                  color: Theme.text-primary;
                  font-size: Theme.font-size-body;
                  overflow: elide;
              }
              Text {
                  text: device.address;
                  color: Theme.text-secondary;
                  font-size: Theme.font-size-label;
                  overflow: elide;
              }
          }
      }
  }
  ```

- [ ] Update `Bridge.connect-receiver` call: it now passes `device.address` (was `device`).
  Verify `lib.rs` still receives the correct string type.
- [ ] **Build check.**

---

### 6-C — Improve empty-state "Searching" placeholder

Replace the plain static text with an animated spinner + label.

- [ ] Import `Spinner` from `std-widgets.slint` in `connect_page.slint`.
- [ ] Replace:

  ```
  if Bridge.devices.length == 0: Rectangle {
      height: 90px;
      ...
      Text { text: "Searching for receivers..."; }
  }
  ```

  With:

  ```
  if Bridge.devices.length == 0: Rectangle {
      height: 90px;
      border-radius: Theme.radius-card;
      background: Theme.surface-card;

      HorizontalLayout {
          alignment: center;
          spacing: Theme.spacing-default;

          Spinner {
              indeterminate: true;
              width: 24px;
              height: 24px;
          }
          Text {
              text: "Searching for receivers…";
              color: Theme.text-secondary;
              vertical-alignment: center;
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 6-D — Update Rust `lib.rs` to produce `ReceiverItem`

- [ ] Find where `Bridge.devices` is set from `DeviceInfo` in `lib.rs`.
- [ ] Map `DeviceInfo` → `ReceiverItem`:

  ```rust
  fn device_info_to_receiver_item(d: &DeviceInfo) -> ReceiverItem {
      ReceiverItem {
          name:    d.name.clone().into(),
          address: d.address.to_string().into(),
      }
  }
  ```

- [ ] Update the `VecModel` construction to use `ReceiverItem` instead of `String`.
- [ ] If the Rust code needs to inspect the model with `row_count()` / `row_data()` /
  `remove()` (e.g. to remove a device by address), add the trait import:

  ```rust
  use slint::Model;
  ```

  These are trait methods on `Model`, not inherent methods on `Rc<VecModel<T>>` or
  `VecModel<T>`. Inherent methods on `VecModel` (`push`, `set_row_data`, `iter`,
  `set_vec` if available) work without the import; trait methods do not.

  Example removal-by-address pattern:

  ```rust
  use slint::Model;

  fn remove_device(model: &Rc<VecModel<ReceiverItem>>, addr: &str) {
      let addr: SharedString = addr.into();
      if let Some(idx) = (0..model.row_count())
          .find(|&i| model.row_data(i).map_or(false, |r| r.address == addr))
      {
          model.remove(idx);
      }
  }
  ```

- [ ] `Bridge.connect-receiver` callback still receives a `SharedString` (the address) — confirm
  the Rust handler parses it correctly.
- [ ] **Build check.**

---

### 6-E — Add manual IP entry row (optional — Phase 6b)

> Defer unless there is a specific user need for manual IP receiver entry.

- [ ] Add a `LineEdit` + `PrimaryButton` row below the `ListView` in `connect_page.slint`:

  ```
  // Phase 6b — manual IP
  HorizontalLayout {
      spacing: Theme.spacing-default;
      ip-input := LineEdit {
          placeholder-text: "Receiver IP address";
          horizontal-stretch: 1;
      }
      PrimaryButton {
          label: "Connect";
          enabled: ip-input.text != "";
          clicked => Bridge.connect-receiver(ip-input.text);
      }
  }
  ```

- [ ] Add `import { LineEdit } from "std-widgets.slint";` to `connect_page.slint`.
- [ ] Wire the callback on the Rust side to attempt a direct TCP connection to the entered address.

---

## Exit criteria

Phase 6 is complete when:

1. `bridge.slint` defines `ReceiverItem { name, address }` and `devices` is typed as
   `[ReceiverItem]`.
2. `connect_page.slint` shows name + address for each discovered receiver.
3. Empty state shows an animated spinner with "Searching…" text.
4. `Bridge.connect-receiver` still receives the address string and Rust handles it correctly.
5. `cargo build -p android-sender` passes.
6. On-device test: mDNS discovery populates the list with names and addresses.

---

## Notes

- `connect-receiver` callback currently takes a raw `string`. After this phase it will pass
  the `address` field specifically. If Rust needs the display name too, consider changing
  the callback signature to `(name: string, address: string)` — but only if needed.
- The `height: Theme.row-height + 18px` for two-line rows ensures both lines have breathing
  room. Adjust during Phase 10 device testing if text is too cramped.
- "Forget" / saved receiver history is deferred to Phase 6b. It requires persistent storage
  on the Rust side, which is outside the scope of UI restructuring.

---

## Slint best practices applied here

- **Typed `[ReceiverItem]` model is preferable to `[string]`.** Composable, extensible
  (add `last-seen` / `kind` fields without changing the consumer), and survives the
  Rust `Model` trait's row API.
- **The `Model` trait must be in scope to call `row_count` / `row_data` / `remove`
  on a `Rc<VecModel<T>>`.** Add `use slint::Model;` to whichever module performs row
  inspection. `VecModel`'s inherent methods (`push`, `set_row_data`) are always
  available, but the trait methods are not.
- **`overflow: elide` on `Text` prevents long device names/addresses from breaking
  the row layout.** Combined with `width: 100%` or `horizontal-stretch: 1`, this is
  the standard Slint pattern for one-line strings of unknown length.

# Phase 6 — Receiver List and Discovery UX (UI-only placeholder)

> Upgrade the flat `[string]` device list to a rich `ReceiverItem` model with name
> and address sub-text, plus an animated empty state. **UI placeholder only — no
> Rust wiring changes.** The list reads from a stub model declared inline.

**Status:** `[ ] UI placeholder — no functionality`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 3 (components)
**Functional integration:** Deferred — Rust mDNS discovery still pushes the legacy
`[string]` list into `Bridge.devices`. The new UI consumes a separate stub property.
**Related files:**
- `senders/android/ui/bridge.slint` — `ReceiverItem` struct added here
- `senders/android/ui/pages/connect_page.slint` — `ListView` rebuilt to two-line cards using stub model

---

## Goal

Build the visual surface of the new receiver list (two-line card with name + address,
animated "Searching…" empty state, optional manual-IP row) without touching any Rust
discovery code. The existing legacy `Bridge.devices: [string]` stays for now and is
ignored by this phase's UI; live data is parked in `futures/`.

---

## Tasks

### 6-A — Add `ReceiverItem` struct to `bridge.slint`

Type definition only — no `Bridge` property yet.

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```slint
  export struct ReceiverItem {
      name:    string,
      address: string,
  }
  ```

- [ ] **Do not** change the existing `in property <[string]> devices: []`. Leaving the
  legacy property untouched keeps Rust building during the placeholder phase.
- [ ] **Build check.**

---

### 6-B — Declare inline stub model on `ConnectView`

- [ ] Open `senders/android/ui/pages/connect_page.slint`.
- [ ] Import `ReceiverItem` from `../bridge.slint`.
- [ ] Add an inline mock at the top of `ConnectView`:

  ```slint
  export component ConnectView inherits Rectangle {
      // UI-only placeholder. Live data lives in `futures/` until Rust mDNS maps
      // `DeviceInfo` → `ReceiverItem`.
      in-out property <[ReceiverItem]> mock-devices: [
          { name: "Living Room TV",     address: "192.168.1.50" },
          { name: "Office Display",     address: "192.168.1.51" },
          { name: "Kitchen Chromecast", address: "192.168.1.52:46899" },
      ];
      // ... existing layout
  }
  ```

- [ ] **Build check.**

---

### 6-C — Replace the existing one-line list body with two-line cards

- [ ] In the same file, replace the existing `for device in Bridge.devices: ...` loop with:

  ```slint
  for device in root.mock-devices: Rectangle {
      height: Theme.row-height + 18px;

      ta := TouchArea {
          // UI-only — no real connect handler in this phase.
          clicked => { /* placeholder: would call connect-receiver(device.address) */ }
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

- [ ] **Build check.**

---

### 6-D — Improve empty-state with animated spinner

- [ ] Import `Spinner` from `std-widgets.slint` in `connect_page.slint`.
- [ ] Add an `in-out property <bool> mock-empty: false;` to make the empty-state easy
  to preview by flipping a single property.
- [ ] Replace the existing static "Searching…" placeholder with:

  ```slint
  if root.mock-empty || root.mock-devices.length == 0: Rectangle {
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

- [ ] **Build check.** Manually flip `mock-empty: true` once to verify the empty state
  visually, then revert.

---

### 6-E — Add manual-IP row (placeholder)

- [ ] Import `LineEdit` from `std-widgets.slint`.
- [ ] Add the row below the device list:

  ```slint
  HorizontalLayout {
      spacing: Theme.spacing-default;
      ip-input := LineEdit {
          placeholder-text: "Receiver IP address";
          horizontal-stretch: 1;
      }
      PrimaryButton {
          label: "Connect";
          enabled: ip-input.text != "";
          // UI-only — no real connect handler in this phase.
          clicked => { /* placeholder */ }
      }
  }
  ```

- [ ] **Build check.**

---

## Exit criteria

1. `bridge.slint` defines `ReceiverItem { name, address }` (struct only — no `Bridge` property added).
2. `connect_page.slint` shows two-line cards (name + address) for each entry in
   `mock-devices`.
3. Empty state shows an animated spinner with "Searching…" text when `mock-empty` is true
   or `mock-devices.length == 0`.
4. Manual-IP row renders with a `LineEdit` + `PrimaryButton`. Click is a no-op.
5. Existing `Bridge.devices: [string]` is untouched — Rust still builds.
6. `cargo build -p android-sender` passes.

---

## What's NOT in this phase (deferred)

- Mapping `DeviceInfo` → `ReceiverItem` in Rust.
- Replacing the legacy `Bridge.devices: [string]` with `[ReceiverItem]`.
- `connect-receiver(address)` callback wiring.
- Manual-IP row wired to a real TCP connect.
- "Forget" / saved receiver history (needs persistent storage).
- `@tr(...)` wrapping (Phase 9).

---

## Slint best practices applied here

- **Typed `[ReceiverItem]` model is preferable to `[string]`.** Composable, extensible
  (add `last-seen` / `kind` fields without changing the consumer).
- **`overflow: elide` on `Text` prevents long device names/addresses from breaking
  the row layout.** Combined with `width: 100%` or `horizontal-stretch: 1`, this is
  the standard Slint pattern for one-line strings of unknown length.
- **A boolean `mock-empty` property paired with `||`** lets a developer flip between
  populated and empty states with a single toggle for visual QA — far easier than
  emptying the list and remembering to refill it.

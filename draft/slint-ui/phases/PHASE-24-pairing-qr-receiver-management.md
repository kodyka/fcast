# Phase 24 — Pairing QR & Receiver Management Placeholder

> Pairing surface (QR display for the receiver to scan FCast sender info) plus
> per-receiver context menu (rename, forget, set-as-default). **UI-only — no
> real QR generation, no persistent rename/forget.**

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 6 (receiver list)
**Functional integration:** Deferred — QR generation requires a Rust QR encoder
(e.g. `qrcode` crate); rename / forget require persistence.
**Moblin source analogues:**
- `Utils/QrCodeImageView.swift`
- `Utils/NameEditView.swift`
- `Utils/ContextMenu/*.swift`
- `Utils/SwipeLeftTo/*.swift` (long-press / swipe gestures)

**Files:**
- `senders/android/ui/pages/pairing_page.slint` — new
- `senders/android/ui/components/qr_placeholder.slint` — new (renders a fake QR grid)
- `senders/android/ui/components/receiver_context_menu.slint` — new
- `senders/android/ui/pages/receiver_rename_page.slint` — new
- `senders/android/ui/bridge.slint` — `Panel.pairing`, `Panel.receiver-rename`

---

## Tasks

### 24-A — `QrPlaceholder` component

- [ ] Create `senders/android/ui/components/qr_placeholder.slint`. Render a
  21×21 grid of `Rectangle`s in alternating colors driven by a fixed pseudo-
  random seed so the output looks QR-like at a glance:

  ```slint
  // 21×21 = 441 cells; pre-computed pattern hardcoded inline.
  // For UI-only build, ship a hand-drawn pattern that looks like a real QR.
  ```

- [ ] Three big alignment squares (top-left, top-right, bottom-left) drawn as
  nested `Rectangle`s.
- [ ] **Build check.**

---

### 24-B — `PairingPage`

- [ ] Header + Done button.
- [ ] Body:
  - Big centered `QrPlaceholder` (240×240px).
  - Below: receiver target ("This device" + IP from a stub property).
  - "Refresh" `TextButton` re-renders the QR (UI-only — pattern is static so
    visually nothing changes; flash a brief overlay to confirm the tap).
  - "Copy address" row (no real clipboard write — UI-only).

---

### 24-C — `ReceiverContextMenu` component

- [ ] A floating menu that appears as a popover when a receiver row is
  long-pressed. Options:
  - Rename
  - Forget receiver (destructive)
  - Set as default
  - Disconnect (only when this receiver is currently active)
- [ ] Implement long-press detection on the receiver row in `connect_page.slint`
  using a Slint `TouchArea` with a duration-tracking timer (pattern from Phase 18-B).
- [ ] On long-press, set a `selected-receiver-id` property on the page and
  show the menu. Tapping outside the menu dismisses it.

---

### 24-D — `ReceiverRenamePage`

- [ ] Simple form: a `LineEdit` pre-populated with the current name + Save /
  Cancel buttons. Save closes the panel and updates the inline `mock-devices`
  entry's `name` field (UI-only — not persisted across builds).
- [ ] **Build check.**

---

### 24-E — Forget receiver flow

- [ ] Forget option in context menu opens `ConfirmDialog` from Phase 19.
- [ ] On confirm, remove the entry from the page-level `mock-devices` list.

---

### 24-F — Bridge + linking

- [ ] Extend `Panel`: `pairing`, `receiver-rename`.
- [ ] Add a `quick-action` entry id `"pair"` in the bar stub model.
- [ ] In `connect_page.slint`, add a "Pair via QR" `TextButton` next to the
  manual-IP row that opens `Panel.pairing`.

---

## Exit criteria

1. Pairing page renders QR placeholder + receiver target.
2. Long-press on a receiver row in `ConnectView` opens context menu.
3. Rename → opens rename page → Save updates the inline list.
4. Forget → opens ConfirmDialog → Confirm removes the entry.
5. Set-as-default flips the entry's `is-default` flag (add to `ReceiverItem`
   struct in this phase if not present).
6. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real QR encoding from sender data.
- Real Bluetooth / NFC pairing.
- Persistent rename / forget / default (resets on reload).
- Swipe-left gesture on receiver rows (long-press is the placeholder).

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Utils/QrCodeImageView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Utils/QrCodeImageView.swift (excerpt)
struct QrCodeImageView: View {
    let payload: String
    var body: some View {
        Image(uiImage: makeQrCode(payload))
            .interpolation(.none)
            .resizable()
            .aspectRatio(1, contentMode: .fit)
    }
}
```

**Mapping notes:**

Slint has no built-in QR code element. The placeholder renders a black
rectangle with a white grid (`for x in 21: for y in 21: Rectangle {...}`)
plus a centered text label "(QR code preview)". Real QR rendering wires
through Rust (using a crate like `qrcode`) in Phase 8.

**Relevant Slint docs:**
- [for / repeated elements](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/repetition.mdx)
- [Image element (for real QR)](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/image.mdx)

## Slint best practices applied here

- **A page-level `selected-receiver-id` property** drives the contextual menu
  state. The menu component itself is stateless — it reads the id and renders.
- **Long-press detection via `TouchArea` + timer** is the standard Slint
  workaround for the absent dedicated long-press gesture in 1.15.x.
- **`ReceiverContextMenu` as a floating popover** uses `z` ordering rather
  than a separate window — simpler than overlay routing and avoids backdrop
  click coordination.

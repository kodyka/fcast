# MVP-PHASE-7 — Step 1: promote `Bridge.devices` from `[string]` to `[ReceiverItem]`

> Part 1 of 5. Parent doc: [`MVP-PHASE-7-receiver-item-promotion.md`](./MVP-PHASE-7-receiver-item-promotion.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Change one line in `senders/android/ui/bridge.slint`: the type of the
`devices` `in property` from `[string]` to `[ReceiverItem]`. The
`ReceiverItem` struct already exists in the same file (lines 110-118
of `bridge.slint`), so no new Slint type definition is needed.

After this step:

- The Slint side compiles cleanly only if the Rust side
  ([Step 2](./MVP-PHASE-7-STEP-2-update-receivers-in-ui.md))
  also lands in the same commit (since `set_devices` will now
  expect `ModelRc<ReceiverItem>`, not `ModelRc<SharedString>`).
- The connect-page iterator's `device` symbol becomes a struct
  reference (`device.id`, `device.name`, `device.address`) instead
  of a raw `string`.

This is a **trivial, single-line change**. The work happens in
the dependent steps. This file exists to keep the diff scoped.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Bridge.devices` declaration | `senders/android/ui/bridge.slint:145` |
| `ReceiverItem` struct declaration | `senders/android/ui/bridge.slint:110-118` |
| The connect-page iterator that reads `Bridge.devices` | `senders/android/ui/pages/connect_page.slint:80-127` |
| Rust side that calls `set_devices` | `senders/android/src/lib.rs:659-674` (`update_receivers_in_ui`) |

### 1.2 Why `ReceiverItem` already exists

Inspecting `bridge.slint:110-118`:

```slint
export struct ReceiverItem {
    id: string,
    name: string,
    address: string,
    ip: string,
    port: int,
    kind: string,
    is-default: bool,
}
```

The struct was declared in an earlier phase as preparation. The
PHASE-1 implementation (`MVP-PHASE-1-connect-receiver-wiring.md`)
deliberately deferred its use, instead populating `Bridge.devices`
as `[string]` for MVP simplicity. PHASE-7 finally adopts the richer
type.

### 1.3 Why one line, not a bigger refactor

The `ReceiverItem` type is **fully declared and exported**. The
only thing missing is the `devices` property's type annotation.
Changing it is one line. The cascade effect (Rust `set_devices`
type mismatch, connect-page iterator field access) is what the
remaining four STEP files address.

---

## 2. The change

**File:** `senders/android/ui/bridge.slint`

**Before** (around line 145):

```slint
// ── Data properties (Rust → Slint) ──────────────────────────────────
in property <[string]> devices: [
    // "Device 1", "Device 2",
];
```

**After:**

```slint
// ── Data properties (Rust → Slint) ──────────────────────────────────
in property <[ReceiverItem]> devices: [
    // { id: "Device 1", name: "Device 1", address: "...", ip: "...",
    //   port: 46899, kind: "fcast", is-default: false },
];
```

### 2.1 Why the default value is an empty array, not a sample

A Slint `in property <[ReceiverItem]>` defaults to `[]` (empty
list) if no initialiser is provided. Don't ship sample data — it
would either:

- Render in the UI when the Rust side hasn't pushed anything yet
  (confusing visual state during cold start).
- Get overwritten by `set_devices(&empty_model)` on first Rust
  call (extra Slint re-render, briefly).

Keep the array literal empty (the commented-out example is
documentation-only).

### 2.2 `[ReceiverItem]` vs `[ReceiverItem; N]`

Slint's `[T]` is a runtime-sized model — what we want. There is no
`[T; N]` fixed-size syntax for properties (only for layout slots).
So the only thing to change is the type inside the angle brackets.

---

## 3. Verification

### 3.1 Slint compile

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Strictly speaking this step **does not compile** without
[Step 2](./MVP-PHASE-7-STEP-2-update-receivers-in-ui.md), because
the Rust `update_receivers_in_ui` will then fail with:

```
error[E0271]: type mismatch resolving
  `<VecModel<SharedString> as ...>::Item == ReceiverItem`
```

That's expected. The combined Step 1 + 2 squash is what gets a
clean build. Don't ship Step 1 in isolation.

### 3.2 Grep

```bash
grep -n 'in property <\[ReceiverItem\]> devices' senders/android/ui/bridge.slint
# → exactly 1 match
grep -n 'in property <\[string\]> devices'      senders/android/ui/bridge.slint
# → exactly 0 matches (the line was rewritten in place)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to update the Rust side

If you land Step 1 alone, the build is broken. Cluster Steps 1 + 2
together — the Rust `update_receivers_in_ui()` rewrite is what
makes Step 1 actually work.

### P2 — Renaming `ReceiverItem`

Tempting to rename to e.g. `DeviceItem` for consistency with
`Bridge.devices`. Don't — `ReceiverItem` is intentionally named
after the **role** (a discovered receiver) rather than the property
name. Future flows may have multiple sources of `ReceiverItem`s
(history, favourites, etc.).

### P3 — Sample data in the array literal

As noted in §2.1, don't ship sample data in `bridge.slint`. The
Rust side populates `devices` on every mDNS discovery event. Sample
data would leak into release builds and confuse users.

### P4 — Slint defaults for empty list

If the array literal is omitted entirely, Slint should default to
`[]` for `[T]`. If your version of Slint complains, set the
default explicitly:

```slint
in property <[ReceiverItem]> devices: [];
```

### P5 — Don't touch `mock-devices` here

`ConnectView`'s `mock-devices` (line 22) and `mock-empty`
(line 25) are unrelated `in-out property`s with their own typing —
they were not used to feed `Bridge.devices`. Leave them for
[Step 5](./MVP-PHASE-7-STEP-5-cleanup-mock-devices.md), which is
the optional cleanup step.

---

## 5. Next step

Once this lands, [Step 2](./MVP-PHASE-7-STEP-2-update-receivers-in-ui.md)
rewrites the Rust `update_receivers_in_ui()` to construct
`ReceiverItem` structs from the discovered devices map and call
`set_devices` with a `ModelRc<ReceiverItem>` instead of
`ModelRc<SharedString>`.

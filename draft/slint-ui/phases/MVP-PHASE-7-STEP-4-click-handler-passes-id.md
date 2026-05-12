# MVP-PHASE-7 — Step 4: pass `device.id` to `Bridge.connect-receiver(...)`

> Part 4 of 5. Parent doc: [`MVP-PHASE-7-receiver-item-promotion.md`](./MVP-PHASE-7-receiver-item-promotion.md).
> Previous: [Step 3 — connect-page field reads](./MVP-PHASE-7-STEP-3-connect-page-field-reads.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Change one Slint expression: the click handler that invokes
`Bridge.connect-receiver(...)` must pass `device.id` (the stable
mDNS service name) instead of `device` (which is now a struct
reference, not a string).

After this step, tapping a row connects to the correct receiver
identified by its mDNS service name. The Rust handler
(`lib.rs:1800-1807`) already does `self.devices.get(&device_name)`
— so as long as `device.id` is the mDNS service name (per Step 2),
the lookup remains correct.

This is a **trivial Slint-only step** — one expression. It exists
as a separate STEP file because skipping it leaves the build
broken (Slint can't auto-convert `ReceiverItem` to `string`).

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Click handler in the connect-page iterator | `senders/android/ui/pages/connect_page.slint:85-87` (approx) |
| `Bridge.connect-receiver(...)` callback signature | `senders/android/ui/bridge.slint:153` — already takes `string` (the receiver id) |
| Rust receiver: `connect_receiver` handler | `senders/android/src/lib.rs:1800-1807` (approx) |

### 1.2 Why the callback signature is unchanged

PHASE-1 already wired `Bridge.connect-receiver: callback (id: string);`
in `bridge.slint:153`. The signature was forward-compatible — the
parameter has always been logically a receiver id; PHASE-1 just
happened to use the receiver name as a stand-in id (because that
was the only available datum at the time).

PHASE-7 makes the parameter's name finally match its semantics
without changing the type.

### 1.3 Why `device.id` (not `device.name`)

`device.id` is the mDNS service name, which is what the Rust
`self.devices: HashMap<String, DeviceInfo>` uses as the key. Using
`device.name` would also work today (they're identical post-Step 2),
but conceptually they may diverge: PHASE-24 might let users rename
receivers, at which point `device.name` could differ from
`device.id`.

**Always pass the id, not the name**, when invoking actions.

---

## 2. The change

**File:** `senders/android/ui/pages/connect_page.slint`

**Before** (post-PHASE-1, line 85-87 approx):

```slint
clicked => {
    Bridge.connect-receiver(device);
}
```

**After:**

```slint
clicked => {
    Bridge.connect-receiver(device.id);
}
```

### 2.1 Why no Rust-side changes

The Rust handler `connect_receiver(&self, device_name: SharedString)`
already receives a `String`-equivalent. The argument's value passes
through unchanged — it's just that **what the value contains** is
guaranteed to be the mDNS service name (Step 2), not just whatever
was in the `[string]` model.

```rust
// senders/android/src/lib.rs, ~line 1800-1807
fn connect_receiver(&self, device_name: SharedString) -> Result<()> {
    let device_name = device_name.to_string();
    let device_info = self
        .devices
        .get(&device_name)
        .ok_or_else(|| anyhow!("device not found: {device_name}"))?;
    // …existing connect flow…
}
```

No change here — but verify the call site **does** receive
`device.id` as the value. After Step 4, this is guaranteed.

### 2.2 Why this is its own step (not folded into Step 3)

You could merge this into Step 3's diff. Splitting it gives the
git history a clearer record:

- "Step 3: field reads" → display layer.
- "Step 4: id pass-through" → behavioural / action wiring.

Easier to revert one without the other. Also makes the Slint
compiler error surface (P1 below) easier to reason about.

---

## 3. Verification

### 3.1 Slint compile

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. If Step 4 isn't landed:

```
error: callback 'connect-receiver' expects string, got ReceiverItem
   --> senders/android/ui/pages/connect_page.slint:86:36
    |
86  |     Bridge.connect-receiver(device);
    |                             ^^^^^^
```

The fix is the one-character `.id` suffix.

### 3.2 Runtime smoke

1. Launch the app with mDNS discovery active.
2. Tap a receiver row.
3. Verify in `adb logcat` that the connect attempt fires with the
   correct receiver name:

```bash
adb logcat | grep -E 'connect_receiver|device_name'
```

Expected log line: `connect_receiver: device_name="FCast Receiver"`
(or whatever the mDNS service name is).

### 3.3 Grep

```bash
grep -n 'connect-receiver(device' senders/android/ui/pages/connect_page.slint
# → expected: connect-receiver(device.id)
# → unexpected: connect-receiver(device) — bare reference
```

---

## 4. Pitfalls specific to this step

### P1 — `connect-receiver(device.name)` works today but is wrong

`device.name == device.id` post-Step-2, so the user-visible
behaviour is identical. But conceptually wrong — and a future
PHASE that allows user-renaming will break silently. Always use
`.id`.

### P2 — Forgetting that the Rust side does `to_string()`

The Slint `string` → Rust `SharedString` → Rust `String`
conversion is implicit at the callback boundary. Don't add
`.to_string()` on the Slint side — it's not a valid Slint method
on strings. The conversion happens in slint-generated glue code.

### P3 — Empty `device.id`

If `update_receivers_in_ui` (Step 2) accidentally emits an empty
string for `id`, the click handler will pass `""` and the Rust
lookup will fail with `"device not found: "`. Defend against this
in Step 2 by ensuring `name` is non-empty before constructing the
`ReceiverItem`. The mDNS `name` field is always non-empty per
RFC 6763, so this is paranoia.

### P4 — Multiple click handlers on the row

The connect-page row might have layered click handlers (one for
short-press, one for long-press timer). Make sure you're editing
the right one. The short-press handler invokes
`connect-receiver(...)`; the long-press timer invokes the context
menu.

### P5 — Coalescing into a single button

Tempted to:

```slint
TouchArea {
    clicked => { Bridge.connect-receiver(device.id); }
}
```

…but the existing structure handles long-press separately
(PHASE-1 §6.2). Leave the timer / press / release wiring alone —
only update the **value** passed to `connect-receiver`.

---

## 5. Next step

[Step 5](./MVP-PHASE-7-STEP-5-cleanup-mock-devices.md) (optional)
removes the unused `mock-devices` / `mock-empty` properties from
`ConnectView`. The implementation is complete and shipping-ready
after Step 4; Step 5 is pure code-hygiene.

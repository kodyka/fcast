# MVP-PHASE-7 — Step 2: rewrite `update_receivers_in_ui()` to emit `ReceiverItem`

> Part 2 of 5. Parent doc: [`MVP-PHASE-7-receiver-item-promotion.md`](./MVP-PHASE-7-receiver-item-promotion.md).
> Previous: [Step 1 — Bridge property type](./MVP-PHASE-7-STEP-1-bridge-property-type.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Rewrite `update_receivers_in_ui()` in `senders/android/src/lib.rs` to:

- Iterate `self.devices` (the mDNS discovery cache) the same way as
  today.
- For each device, construct a `ReceiverItem` struct with:
  - `id` = mDNS service name (stable across the session)
  - `name` = same as `id` for now
  - `address` = `"<ip>:<port>"`
  - `ip` = first discovered IPv4/IPv6 address
  - `port` = the TCP port
  - `kind` = `"fcast"` or `"chromecast"` (feature-gated)
  - `is_default` = `false` (will be set from persistent storage in a
    future PHASE-24)
- Push the resulting `Vec<ReceiverItem>` via `VecModel<ReceiverItem>`
  to `Bridge.set_devices`.

After this step paired with
[Step 1](./MVP-PHASE-7-STEP-1-bridge-property-type.md), the Rust and
Slint sides agree on the type. The connect-page iterator still
compiles (because the Slint compiler tolerates `device.name` for
non-string types), but reads now return the right fields.

This is the **largest step in PHASE-7** (~40 Rust lines of net
change). It's also the step that exercises the most external
dependencies: `fcast_sender_sdk::IpAddr`, `ProtocolType`,
`ReceiverItem` (slint-generated), and the existing devices iterator.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `update_receivers_in_ui` | `senders/android/src/lib.rs:659-674` |
| `self.devices: HashMap<String, DeviceInfo>` | `lib.rs:480-485` (approx) |
| `DeviceInfo` struct (where addresses / port / protocol live) | `senders/sdk/fcast_sender_sdk/src/discovery.rs` (approx) |
| `ReceiverItem` Slint struct (generated Rust) | `senders/android/ui/bridge.slint:110-118` (declaration) → `target/.../slint-build/.../bridge.rs::ReceiverItem` |
| `Bridge.set_devices(...)` (generated Rust) | accessed via `ui.global::<Bridge>().set_devices(...)` |
| `fcast_sender_sdk::IpAddr` | `senders/sdk/fcast_sender_sdk/src/lib.rs::IpAddr` (enum V4/V6 of `String`) |
| `fcast_sender_sdk::device::ProtocolType` | same crate, `device` module |

### 1.2 Why the mDNS service name is the stable id

See parent doc §1.3 — the mDNS service name (e.g.
`"FCast Receiver._fcast._tcp.local."`) is unique within a session
and survives transient address changes (IP renewal, network
reconnect). For PHASE-7 scope, that's sufficient. PHASE-24 will
introduce a persistent identifier from the receiver's hardware fingerprint.

### 1.3 Why `address: "<ip>:<port>"`

The connect-page UI (PHASE-7 [Step 3](./MVP-PHASE-7-STEP-3-connect-page-field-reads.md))
displays the secondary line as `device.address`. Picking
`"192.168.1.42:46899"` matches the legacy ChromeCast / Roku
discovery UI conventions and is human-recognisable.

If the user wants just the IP, they can format it client-side
(no current need; the secondary line is informational).

---

## 2. The change

### 2.1 Add the import (find the right path first)

**File:** `senders/android/src/lib.rs` (near the top, with the
other `use` statements):

```rust
use crate::ReceiverItem;
```

The exact import path depends on where slint-build re-exports
`ReceiverItem`. Confirm with:

```bash
grep -nE 'pub struct ReceiverItem|use .*ReceiverItem' \
    senders/android/src/lib.rs target/aarch64-linux-android/debug/build/*/out/*.rs
```

If the slint build script re-exports it as `crate::ReceiverItem`,
the import is `use crate::ReceiverItem;`. If nested, adjust (e.g.
`use crate::ui::ReceiverItem;` or `use crate::main::ReceiverItem;`).

### 2.2 Rewrite the function body

**File:** `senders/android/src/lib.rs`

**Before** (lines 659-674):

```rust
fn update_receivers_in_ui(&mut self) -> Result<()> {
    let receivers = self
        .devices
        .iter()
        .filter(|(_, info)| !info.addresses.is_empty() && info.port != 0)
        .map(|(name, _)| slint::SharedString::from(name))
        .collect::<Vec<slint::SharedString>>();
    self.ui_weak.upgrade_in_event_loop(move |ui| {
        let model = std::rc::Rc::new(slint::VecModel::<slint::SharedString>::from_iter(
            receivers.into_iter(),
        ));
        ui.global::<Bridge>().set_devices(model.into());
    })?;

    Ok(())
}
```

**After:**

```rust
fn update_receivers_in_ui(&mut self) -> Result<()> {
    let receivers = self
        .devices
        .iter()
        .filter(|(_, info)| !info.addresses.is_empty() && info.port != 0)
        .map(|(name, info)| {
            let first_addr = info
                .addresses
                .first()
                .map(|a| match a {
                    fcast_sender_sdk::IpAddr::V4(s) => s.clone(),
                    fcast_sender_sdk::IpAddr::V6(s) => format!("[{s}]"),
                })
                .unwrap_or_default();

            let kind = match info.protocol {
                fcast_sender_sdk::device::ProtocolType::FCast => "fcast",
                #[cfg(feature = "chromecast")]
                fcast_sender_sdk::device::ProtocolType::Chromecast => "chromecast",
            };

            ReceiverItem {
                id: name.clone().into(),                       // mDNS service name (§1.2)
                name: name.clone().into(),
                address: format!("{first_addr}:{}", info.port).into(),
                ip: first_addr.into(),
                port: info.port as i32,
                kind: kind.into(),
                is_default: false,                              // PHASE-24
            }
        })
        .collect::<Vec<ReceiverItem>>();

    self.ui_weak.upgrade_in_event_loop(move |ui| {
        let model = std::rc::Rc::new(slint::VecModel::<ReceiverItem>::from_iter(
            receivers.into_iter(),
        ));
        ui.global::<Bridge>().set_devices(model.into());
    })?;

    Ok(())
}
```

### 2.3 Why `[ipv6]` brackets

IPv6 literals contain `:` separators that collide with the `:` in
`"<ip>:<port>"`. Wrapping in `[...]` is the URL-style escape (per
RFC 3986). The connect-page UI displays the result as-is — users
seeing `[fe80::1]:46899` recognise it as an IPv6 receiver.

### 2.4 Why `info.port as i32`

Slint's `int` type is `i32`. The mDNS `port: u16` cast is lossless
(0-65535 fits in `i32`). Don't `try_into()` — `u16` → `i32` is
infallible.

### 2.5 Why `is_default: false` (always)

PHASE-7 has no persistent-storage backing for "default receiver".
PHASE-24 (post-MVP) introduces a SharedPreferences-backed default.
For now, always `false`; the UI's badge rendering for `is_default`
is a no-op.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean** once [Step 1](./MVP-PHASE-7-STEP-1-bridge-property-type.md)
is in. Most likely failures:

- `error[E0432]: unresolved import crate::ReceiverItem` —
  slint-build re-exports `ReceiverItem` at a different path. Run
  the §2.1 grep to find the right one.
- `error[E0271]: type mismatch resolving ...VecModel<SharedString>` —
  Step 1 not landed. Re-check `bridge.slint`.
- `error[E0599]: no method named into on type SharedString` —
  ambiguous `.into()` resolution. Use `slint::SharedString::from(...)`
  explicitly:

```rust
id: slint::SharedString::from(name.as_str()),
```

### 3.2 Runtime smoke

Launch the app on a device with an mDNS-discoverable receiver.
Verify the connect-page list shows:

- Primary line: the receiver name (e.g. "FCast Receiver").
- Secondary line: `192.168.1.x:46899` (or `[fe80::x]:46899` on IPv6).
- The `kind` field is unused visually in PHASE-7 but populated for
  future icon-by-protocol rendering.

### 3.3 Grep

```bash
grep -nE 'ReceiverItem\s*\{|set_devices\(' senders/android/src/lib.rs
# → at least 2 matches:
#   - The constructor inside .map(...)
#   - The set_devices call inside upgrade_in_event_loop
```

---

## 4. Pitfalls specific to this step

### P1 — Wrong import path for `ReceiverItem`

Slint-build's output module structure varies between versions. If
`use crate::ReceiverItem;` fails, run:

```bash
find target -name '*.rs' -path '*slint*' -newer Cargo.toml | xargs grep -l 'pub struct ReceiverItem' | head -1
```

…and inspect the discovered file to determine the right path
(common: `crate::main::ReceiverItem`, `crate::bridge::ReceiverItem`,
or `crate::ui::ReceiverItem`).

### P2 — `info.addresses.first()` returning `None`

The `.filter()` clause already excludes empty `addresses`, so
`.first()` should always return `Some(_)`. `.unwrap_or_default()`
covers the unreachable case without panicking. **Don't** add an
`expect("addresses non-empty after filter")` — clippy will flag it.

### P3 — `ProtocolType::Chromecast` under disabled feature

`fcast_sender_sdk` gates `ProtocolType::Chromecast` behind a
feature flag (`chromecast`). The `#[cfg(feature = "chromecast")]`
in the match arm gates the **arm**, not the **variant** — Rust
requires both to agree.

If the feature is disabled in your build, the match is exhaustive
with just the `FCast` arm. If enabled, both arms are required.
Don't omit the `#[cfg]` — that would break the no-`chromecast`
build with an unused-variant warning.

### P4 — `device.address` formatting differs from your expectation

Tempted to use `device.ip + ":" + device.port` directly in
Slint? Don't — Slint's string concatenation is awkward and the
formatting (especially IPv6 brackets) is easier in Rust. Pre-format
in `update_receivers_in_ui` and pass the final string.

### P5 — Forgetting `slint::ModelRc::from` vs `.into()`

`set_devices(...)` takes `ModelRc<ReceiverItem>`.
`std::rc::Rc::new(slint::VecModel::<ReceiverItem>::from_iter(...))` is
`Rc<VecModel<ReceiverItem>>`. The `.into()` lands at
`ModelRc<ReceiverItem>` via `From<Rc<VecModel<T>>>`. If the
`.into()` resolves to the wrong target, use the explicit form:

```rust
ui.global::<Bridge>().set_devices(slint::ModelRc::from(model));
```

### P6 — `format!("[{s}]")` allocates twice

Once for the inner `s.clone()`, once for the `format!`. For
hot-path code this would matter; for the rare mDNS discovery event
(≤ once per second under heavy churn), it's fine. Don't
prematurely optimise.

### P7 — `SharedString::from(name)` vs `name.clone().into()`

Both work. The `.clone().into()` form drops one allocation level
(the `String → SharedString` conversion handles the move). The
`SharedString::from(name)` form is more explicit. Pick one and use
consistently.

---

## 5. Next step

Once this lands together with Step 1, [Step 3](./MVP-PHASE-7-STEP-3-connect-page-field-reads.md)
updates the connect-page Slint iterator to read
`device.name` / `device.address` / `device.id` fields instead of
treating `device` as a raw `string`.

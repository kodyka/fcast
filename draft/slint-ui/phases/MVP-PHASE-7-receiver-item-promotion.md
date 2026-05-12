# MVP-PHASE-7 — `ReceiverItem` promotion (Tier 2.1 polish)

> **Small post-MVP polish.** MVP-PHASE-1 unblocks the connect path by
> iterating `Bridge.devices: [string]` and treating each entry as a
> receiver name. That works, but it loses information: the
> `DeviceInfo` Rust-side has IP, port, protocol kind, and we want the
> connect-page to show address + kind, and the long-press / forget /
> rename flows to key off a stable id.
>
> This phase promotes `Bridge.devices` from `[string]` to
> `[ReceiverItem]` (already declared at `bridge.slint:110-118`) and
> updates the two Rust call sites + the connect-page iterator.

---

## 0. Goal

After this phase:

- `Bridge.devices: [ReceiverItem]` (not `[string]`).
- `update_receivers_in_ui()` builds `ReceiverItem` records from
  `DeviceInfo`.
- The connect-page iterator reads `device.name`, `device.address`,
  `device.kind`, `device.id` — restoring the field accesses
  removed in MVP-PHASE-1 Step 4 / Step 5.
- The long-press context menu carries a **stable id** instead of the
  receiver name. Rename / forget / set-default flows can persist by
  that id once they're implemented.
- `Bridge.connect-receiver(string)` callback signature **unchanged**:
  still receives a single string (the name, or the id — see §1.4).
  The Rust handler still looks up by name from `self.devices`.

This phase is **purely additive** for the data flow. No Rust event
handlers move; no new commands; no new Slint globals.

---

## 1. Pre-flight

### 1.1 What already exists (do not re-create)

| Component | Location |
|---|---|
| `ReceiverItem` struct (already declared) | `senders/android/ui/bridge.slint:110-118` |
| `Bridge.devices: [string]` (current state) | `senders/android/ui/bridge.slint:145` |
| `update_receivers_in_ui()` (current `[string]` writer) | `senders/android/src/lib.rs:659-674` |
| `Bridge.connect-receiver` callback | `senders/android/ui/bridge.slint:235`, `lib.rs:1800-1807` |
| `DeviceInfo` source-of-truth struct | `sdk/sender/fcast-sender-sdk/src/device.rs:64-71` |
| `DeviceInfo::protocol` (FCast/Chromecast) | `sdk/sender/fcast-sender-sdk/src/device.rs:23-26` |
| `self.devices: HashMap<String, DeviceInfo>` | `senders/android/src/lib.rs` (search `devices: HashMap`) |
| `add_or_update_device()` | `senders/android/src/lib.rs:676-680` |
| Connect-page iterator (post-PHASE-1) | `senders/android/ui/pages/connect_page.slint:69-100, 113-126` |

### 1.2 What needs to change

| File | Edit |
|---|---|
| `senders/android/ui/bridge.slint` | Change `in property <[string]> devices` → `in property <[ReceiverItem]> devices` (line 145). |
| `senders/android/src/lib.rs` | Rewrite `update_receivers_in_ui()` to emit `ReceiverItem` records (lines 659-674). |
| `senders/android/ui/pages/connect_page.slint` | Restore `device.name` / `device.address` field reads (the MVP-PHASE-1 changes assumed `[string]`). |

Approximate scope: **~50 lines across 3 files**.

### 1.3 What's a stable id?

`DeviceInfo` itself **has no id field** — the de-facto identity today
is the **mDNS service name** (which is what
`self.devices: HashMap<String, DeviceInfo>` is keyed on). Two options:

- (a) Use the service name as the id. Trivially stable across the
  session; not stable across renames.
- (b) Hash `(addresses[0], port, protocol)` and use that. Stable across
  rename, unstable if the device moves networks.

**For this phase, (a)** — match what `connect_with_device_info` keys
off, and what `Bridge.connect-receiver(name)` already sends. (b) is a
post-PHASE-24 polish.

### 1.4 The `connect-receiver` callback signature

`Bridge.connect-receiver(string)` (the callback) takes a **single
string**. After this phase, the connect-page passes
`device.id` to it (instead of `device` itself, as in MVP-PHASE-1). The
Rust side (`lib.rs:1800-1807`) keeps doing `self.devices.get(&name)`
— so the id we pass **must** be the mDNS service name (option (a)
above), not a derived hash.

If we ever switch to (b), update both `connect-receiver` callers **and**
the Rust lookup (introduce a `HashMap<id, DeviceInfo>` instead of
keying by name). Out of scope here.

---

## 2. Steps

### 2.1 Step 1 — change the Bridge property type

**File:** `senders/android/ui/bridge.slint`

**Before** (line 145):

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
    // { id: "Device 1", name: "Device 1", address: "...", ip: "...", port: 46899, kind: "fcast", is-default: false },
];
```

`ReceiverItem` is already declared at lines 110-118 of the same file
and already exported from the same module — no `import` change needed
in `connect_page.slint`.

### 2.2 Step 2 — rewrite `update_receivers_in_ui()`

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
                // MVP-PHASE-7 §1.3 option (a): the mDNS service name
                // doubles as the id. Stable across the session.
                id: name.clone().into(),
                name: name.clone().into(),
                address: format!("{first_addr}:{}", info.port).into(),
                ip: first_addr.into(),
                port: info.port as i32,
                kind: kind.into(),
                is_default: false, // PHASE-24 will set this from persistent storage.
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

You'll need this import near the top of `lib.rs`:

```rust
use crate::ReceiverItem; // generated by slint_build::compile("ui/main.slint")
```

Exact crate path depends on where the slint-build re-exports
`ReceiverItem` — search for it:

```bash
grep -n 'pub struct ReceiverItem\|use crate::ReceiverItem' \
    senders/android/src/lib.rs
```

If `slint::compile_modules` exposes it as `crate::ReceiverItem`, the
above works. If it's nested, e.g. `crate::ui::ReceiverItem`, adjust
the import accordingly.

### 2.3 Step 3 — restore the connect-page field reads

**File:** `senders/android/ui/pages/connect_page.slint`

The MVP-PHASE-1 step 4 + step 5 changes flattened the field reads
because `device` was a `string`. Now that `Bridge.devices: [ReceiverItem]`,
restore them.

**Before (post-PHASE-1)** (lines 90-100):

```slint
Timer {
    interval: 600ms;
    running: parent.lp-armed;
    triggered => {
        parent.lp-armed = false;
        // MVP: Bridge.devices is [string]. We don't have a stable id yet
        // (see MVP-PHASE-7). Use the receiver name for both fields.
        root.context-receiver-id = device;
        root.context-receiver-name = device;
        root.context-menu-y = (parent.height * idx) + 100px;
        root.show-context-menu = true;
    }
}
```

**After:**

```slint
Timer {
    interval: 600ms;
    running: parent.lp-armed;
    triggered => {
        parent.lp-armed = false;
        // Long press detected.
        root.context-receiver-id = device.id;
        root.context-receiver-name = device.name;
        root.context-menu-y = (parent.height * idx) + 100px;
        root.show-context-menu = true;
    }
}
```

And for the row body (lines 113-126 post-PHASE-1):

**Before:**

```slint
VerticalLayout {
    padding-left: Theme.padding-screen;
    padding-right: Theme.padding-screen;
    alignment: center;

    Text {
        text: device;
        color: Theme.text-primary;
        font-size: Theme.font-size-body;
        overflow: elide;
    }
    // Secondary address row removed: Bridge.devices is [string].
    // Restored in MVP-PHASE-7 once promoted to [ReceiverItem].
}
```

**After:**

```slint
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
```

### 2.4 Step 4 — update the click handler to pass the id

**File:** `senders/android/ui/pages/connect_page.slint`

**Before (post-PHASE-1)** (line 85-87):

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

The Rust handler (`lib.rs:1800-1807`) does
`self.devices.get(&device_name)` — and `device_name` here is
`device.id`, which is the mDNS service name (§1.3 option (a)). So the
lookup remains correct.

### 2.5 Step 5 — clean up `mock-devices` / `mock-empty` (optional)

Once `Bridge.devices` is `[ReceiverItem]`, the `mock-devices` /
`mock-empty` `in-out property`s on `ConnectView` (lines 20-25) are
fully dead code. Removing them in this phase tightens the file:

```slint
// senders/android/ui/pages/connect_page.slint

export component ConnectView inherits Rectangle {
    // UI-only state. The receiver list comes from Bridge.devices.

    // Context menu state.
    in-out property <bool>   show-context-menu: false;
    in-out property <string> context-receiver-id;
    in-out property <string> context-receiver-name;
    in-out property <length> context-menu-y: 0px;

    // Forget confirmation state.
    in-out property <bool> show-forget-confirm: false;

    /* …existing layout… */
}
```

Optional, low-priority. If any other file imports `mock-devices` from
`ConnectView` (it shouldn't — those are private to the page), the
Slint compiler will tell you.

---

## 3. Verification

### 3.1 Compile-time checks

```bash
# 1. Slint compiles cleanly.
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android

# 2. The connect-page reads ReceiverItem fields, not raw strings.
grep -n 'device\.id\|device\.name\|device\.address' \
    senders/android/ui/pages/connect_page.slint
# → expect 4 matches (id in long-press + click, name in long-press +
#   row body, address in row body).

# 3. Bridge.devices is [ReceiverItem].
grep -n 'devices' senders/android/ui/bridge.slint
# → expect: `in property <[ReceiverItem]> devices`

# 4. update_receivers_in_ui pushes ReceiverItem.
grep -n 'ReceiverItem\|update_receivers_in_ui' \
    senders/android/src/lib.rs
# → expect: at least one constructor of ReceiverItem inside
#   update_receivers_in_ui.
```

### 3.2 Runtime smoke

1. Build & install: `xtask android-sender build && adb install ...`.
2. Open the app.
3. **Expected:** the connect-page lists discovered receivers with
   two-line rows — name on top, `ip:port` below.
4. Long-press a row. The context menu opens. Rename → enters
   `receiver-rename` panel. Forget → opens the confirm dialog.
5. Tap a row. Connection proceeds exactly as in MVP-PHASE-1 (because
   the click handler still passes a single string —
   the id, which is the mDNS service name).

### 3.3 Verify the stable-id contract

```bash
adb logcat | grep -E 'ConnectToDevice\(|connect_with_device_info'
```

Tap a receiver. The log should show:

```
… on_connect_receiver: "Living Room TV"
… Event::ConnectToDevice("Living Room TV")
… connect_with_device_info(name="Living Room TV", …)
```

i.e. the string the Slint side passes (`device.id`) round-trips
through `Bridge.connect-receiver` → Rust → `HashMap::get(&name)`.

---

## 4. Common pitfalls

### P1 — Slint complains `cannot infer type of empty array literal`

If you write:

```slint
in property <[ReceiverItem]> devices: [];
```

…Slint may warn about the inferred type. Either add an inline default
record (already commented above) or leave the trailing comment block
in place — Slint accepts both.

### P2 — Rust `set_devices(...)` type mismatch

`slint_build` generates a `set_devices` setter whose signature
**depends** on the declared type:

| Slint type | Generated Rust signature |
|---|---|
| `in property <[string]> devices` | `set_devices(model: ModelRc<SharedString>)` |
| `in property <[ReceiverItem]> devices` | `set_devices(model: ModelRc<ReceiverItem>)` |

So **after** Step 1, the Rust call
`set_devices(VecModel::<SharedString>::from_iter(...))` becomes a
compile error. Fixing it requires Step 2 in the same commit. (This is
also why this phase is one PR, not two.)

### P3 — `kind: "chromecast"` under a disabled feature

The match in §2.2 has a `#[cfg(feature = "chromecast")]` arm. If the
`chromecast` feature is **off** in the Android build (it currently is),
the `ProtocolType` enum has only the `FCast` variant — the match is
exhaustive without the chromecast arm. If you ever enable
`chromecast`, the match becomes non-exhaustive without it — so leaving
the `#[cfg]` arm in is forward-compatible.

### P4 — `device.address` formatting differs from `device.ip + ":" + device.port`

The example in §2.2 sets `address = "{first_addr}:{port}"` and `ip =
first_addr`. The connect-page row reads `device.address`. If a screen
elsewhere reads `device.ip` and concatenates `:port`, you get the
same string — but **don't** rely on that. Source-of-truth is
`device.address`; `device.ip` + `device.port` are for places that
need numeric routing decisions (e.g. WHEP URL construction).

### P5 — `is_default` is hard-coded `false`

The `is-default` field on `ReceiverItem` is for "auto-connect on
launch" / "starred receiver" behaviour (PHASE-24). For MVP, leave it
`false`. When PHASE-24 lands, populate it from a persistent
preferences key — **not** from `DeviceInfo`, which has no such
concept.

### P6 — `mock-devices` references from elsewhere

If you remove `mock-devices` in §2.5, double-check that no other
page or test snapshot imports it:

```bash
grep -rn 'mock-devices' senders/android/ui/
# → expect: only the (deleted) connect_page.slint line.
```

If there's a stray ref, the Slint compiler will surface it as
"unknown property" — no silent breakage.

---

## 5. Stop conditions

The phase is "done" when:

1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. All four `grep` recipes in §3.1 produce the expected output.
3. The runtime smoke in §3.2 / §3.3 succeeds:
   - Connect page shows name + `ip:port` for each receiver.
   - Long-press opens the context menu; Rename / Forget reach their
     panels.
   - Tap connects exactly as it does after MVP-PHASE-1.
4. **`Bridge.devices` is `[ReceiverItem]` everywhere:**

```bash
grep -n '\[string\]> devices\|\[ReceiverItem\]> devices' \
    senders/android/ui/bridge.slint
# → expect: exactly one `[ReceiverItem]> devices` match, zero `[string]> devices`.
```

5. **No `device` is treated as a raw string in the iterator:**

```bash
grep -nC1 'for device\[' senders/android/ui/pages/connect_page.slint | \
    grep -E 'text: device\s*;|text: device\.|Bridge\.connect-receiver\(device\)'
# → expect: zero matches of `text: device;` (treats device as a string).
# → expect: zero matches of `Bridge.connect-receiver(device)` (should be `(device.id)`).
```

---

## 6. Why this matters

This phase doesn't unlock new functionality — it **stops throwing
information away**. After MVP-PHASE-1, the connect-page knew only
"there's a receiver with this name". After this phase, it knows the
name, the address, the protocol, and a stable id — which is what:

- **PHASE-24** (pairing-qr-receiver-management) needs to persist
  "starred" receivers.
- **PHASE-17** (quick-action-customization) needs to bind
  "cast to <named receiver>" quick actions.
- **PHASE-25** (macros) needs to make
  `connect-to-receiver(id)` a macro step.

All three of those depend on having stable ids on the connect page.
This phase makes that possible at near-zero cost.

It is **not an MVP gate.** Ship MVP-PHASE-1 first; this can land any
time afterward — even as a same-week follow-up — without touching the
cast loop.

# MVP-PHASE-1 — Connect-receiver wiring
 
> **The only MVP-gating change.** ~10 lines in one Slint file. After this
> phase ships, tapping a discovered receiver actually connects to it —
> which unblocks the entire downstream cast loop, which has already been
> wired all the way through to the FCast receiver.
 
---
 
## 0. Goal
 
Replace the placeholder click handler on the connect-page's receiver row
with a real `Bridge.connect-receiver(...)` call, and switch the iterator
from the page-local `mock-devices` to the Rust-pushed `Bridge.devices`.
 
Before this phase: tapping a real receiver does nothing.
After this phase: tapping a real receiver fires
`Event::ConnectToDevice(name)` in Rust → FCast SDK `connect()` →
`AppState::Connecting` → `AppState::SelectingSettings` → on user-confirm
→ MediaProjection consent → `AppState::Casting` → frames flow to
receiver.
 
---
 
## 1. Pre-flight
 
### 1.1 What's already wired (do not touch)
 
| Endpoint | Status | Citation |
|---|---|---|
| `Bridge.connect-receiver(string)` callback | Declared | `senders/android/ui/bridge.slint:235` |
| Rust handler for `connect-receiver` | Registered | `senders/android/src/lib.rs:1800-1810` |
| `Event::ConnectToDevice(...)` channel | Wired | `senders/android/src/lib.rs:1804` |
| `handle_event` for `ConnectToDevice` | Wired | `senders/android/src/lib.rs:747` |
| `connect_with_device_info(...)` | Wired | `senders/android/src/lib.rs:711` |
| `update_receivers_in_ui()` pushes to `Bridge.devices` | Wired | `senders/android/src/lib.rs:659-680` |
| `Bridge.devices` declared as `[string]` | Declared | `senders/android/ui/bridge.slint:145` |
| mDNS service-found JNI callback | Wired | `senders/android/src/lib.rs:2125-2225` |
 
### 1.2 What's blocking (the M1 gap)
 
| File | Line | Current state |
|---|---|---|
| `senders/android/ui/pages/connect_page.slint` | 46 | Empty-state branch keys off `mock-empty \|\| mock-devices.length == 0` |
| `senders/android/ui/pages/connect_page.slint` | 69-72 | Populated-state branch iterates `root.mock-devices` |
| `senders/android/ui/pages/connect_page.slint` | 85-87 | `clicked => { /* placeholder: would call connect-receiver(device.address) */ }` |
| `senders/android/ui/pages/connect_page.slint` | 96-97 | Long-press handler reads `device.id` and `device.name` (struct fields, not on `[string]`) |
| `senders/android/ui/pages/connect_page.slint` | 116, 122 | Row body reads `device.name` and `device.address` |
 
The data model mismatch (`Bridge.devices` is `[string]`, `mock-devices`
is `[ReceiverItem]`) means we can't just swap the iterator — we also
have to flatten field accesses. That's the bulk of this phase.
 
### 1.3 Post-MVP follow-up (out of scope here)
 
Promoting `Bridge.devices` to `[ReceiverItem]` (already declared at
`bridge.slint:110-118`) is **MVP-PHASE-7**. It is not required to ship
MVP — but once shipped, the connect-page can show address / kind / id
fields and the long-press / rename / forget flows can use stable
receiver ids.
 
---
 
## 2. Steps
 
### 2.1 Step 1 — switch the empty-state condition
 
**File:** `senders/android/ui/pages/connect_page.slint`
 
**Before** (line 46):
 
```slint
if root.mock-empty || root.mock-devices.length == 0: Rectangle {
    height: 90px;
    border-radius: Theme.radius-card;
    background: Theme.surface-card;
    // …spinner + "Searching for receivers…" label…
}
```
 
**After:**
 
```slint
if Bridge.devices.length == 0: Rectangle {
    height: 90px;
    border-radius: Theme.radius-card;
    background: Theme.surface-card;
    // …spinner + "Searching for receivers…" label…
}
```
 
The `root.mock-empty` toggle existed so designers could preview the
empty-state in the Slint viewer. Once we read `Bridge.devices` directly,
the empty state is driven by the real backend.
 
### 2.2 Step 2 — switch the populated-state condition + iterator
 
**Before** (lines 69-72):
 
```slint
if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
    spacing: Theme.spacing-default;
 
    for device[idx] in root.mock-devices: Rectangle {
```
 
**After:**
 
```slint
if Bridge.devices.length > 0: VerticalLayout {
    spacing: Theme.spacing-default;
 
    for device[idx] in Bridge.devices: Rectangle {
```
 
### 2.3 Step 3 — wire the click handler
 
**Before** (lines 85-87):
 
```slint
ta := TouchArea {
    changed pressed => {
        if self.pressed {
            parent.lp-armed = true;
        } else {
            parent.lp-armed = false;
        }
    }
    clicked => {
        /* placeholder: would call connect-receiver(device.address) */
    }
}
```
 
**After:**
 
```slint
ta := TouchArea {
    changed pressed => {
        if self.pressed {
            parent.lp-armed = true;
        } else {
            parent.lp-armed = false;
        }
    }
    clicked => {
        Bridge.connect-receiver(device);
    }
}
```
 
`device` here is a `string` (because `Bridge.devices` is `[string]`).
The Rust handler at `lib.rs:1800` takes a single `&str` and sends
`Event::ConnectToDevice(device_name.to_string())` — receiver names are
the join key between `Bridge.devices` and the Rust device map.
 
### 2.4 Step 4 — flatten the long-press / context-menu field reads
 
**Before** (lines 96-99):
 
```slint
Timer {
    interval: 600ms;
    running: parent.lp-armed;
    triggered => {
        parent.lp-armed = false;
        // Long press detected.
        root.context-receiver-id = device.id;
        root.context-receiver-name = device.name;
        root.context-menu-y = (parent.height * idx) + 100px; // Rough y calculation
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
        // MVP: Bridge.devices is [string]. We don't have a stable id yet
        // (see MVP-PHASE-7). Use the receiver name for both fields.
        root.context-receiver-id = device;
        root.context-receiver-name = device;
        root.context-menu-y = (parent.height * idx) + 100px;
        root.show-context-menu = true;
    }
}
```
 
The rename / forget / set-default flows on the context menu are still
no-ops at this phase (see `connect_page.slint:199-210`) — they're
post-Phase-24 work. The fix here is just to keep the context-menu
positioning working without compile errors.
 
### 2.5 Step 5 — flatten the row body field reads
 
**Before** (lines 115-126):
 
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
 
**After:**
 
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
 
Removing the secondary address line slightly shortens the row. If you
want to keep the row height stable, leave `spacing: 2px` and add a
spacer `Rectangle { height: Theme.font-size-label; }` placeholder until
MVP-PHASE-7 promotes the data model.
 
### 2.6 Step 6 — clean up unused `mock-*` properties (optional, low-priority)
 
You can leave the `mock-devices` and `mock-empty` properties declared at
`connect_page.slint:20-25` for a follow-up cleanup PR. They are
harmless — Slint silently allows unused `in-out` properties. If you
prefer a clean slate, delete those two `in-out property` declarations
and the trailing comment block.
 
---
 
## 3. Verification
 
### 3.1 Compile-time checks
 
```bash
# 1. No more references to mock-devices in the iterator / click path.
grep -n 'mock-devices' senders/android/ui/pages/connect_page.slint
 
# Expected: no matches in the populated-state branch (lines 69-130).
# Stray refs only in the `in-out property <[ReceiverItem]> mock-devices`
# declaration (line 20) and the comment block above it — harmless.
 
# 2. New references to Bridge.devices / Bridge.connect-receiver.
grep -n 'Bridge.devices\|Bridge.connect-receiver' \
     senders/android/ui/pages/connect_page.slint
 
# Expected: at least 2 matches — one for the empty-state condition,
# one for the iterator, one for the click handler.
 
# 3. The build compiles.
cargo +nightly check -p fcast-sender-android \
    --target aarch64-linux-android
```
 
### 3.2 Runtime smoke
 
On a real device (or emulator with an FCast receiver on the same
network):
 
1. Build & install: `xtask android-sender build && adb install ...`.
2. Open the app. The Connect page appears.
3. Within ~5 s, a real receiver appears in the list (or "Searching…"
   stays if none reachable).
4. Tap the row. UI must transition `Disconnected → Connecting`.
5. After SDK handshake, UI must transition to `SelectingSettings`.
6. `adb logcat | grep -E 'ConnectToDevice|connect_with_device_info'`
   must show both lines.
 
If step 4 fails, verify with:
 
```bash
adb logcat | grep -E 'on_connect_receiver|GLOB_EVENT_CHAN'
```
 
You should see the `on_connect_receiver` invocation followed by a
`GLOB_EVENT_CHAN.0.send(...)` for `Event::ConnectToDevice`.
 
---
 
## 4. Common pitfalls
 
### P1 — Trying to call `Bridge.connect-receiver(device.name)` against `[string]`
 
If you forget to flatten the field reads in Step 4 / Step 5, the Slint
compiler errors with:
 
```
error: cannot index 'string' with field 'name'
```
 
Resolution: `device` *is* the name when `Bridge.devices: [string]`.
 
### P2 — Empty-state never showing
 
If `Bridge.devices.length == 0` is true at startup, the page should
show the spinner. If you accidentally typed `>` instead of `==` the
empty-state appears only after devices are populated.
 
### P3 — Multiple receivers with the same name
 
The Rust handler dispatches via name match
(`Application.devices: HashMap<String, ...>` keyed on name). If two
receivers share a name, only the first registered will be reachable
from the UI. Document as a known limitation; the fix is MVP-PHASE-7.
 
### P4 — Forgetting `import { Bridge } from "../bridge.slint";`
 
`connect_page.slint:10` already imports `Bridge`, so this is a no-op
here. But if you re-organise the file split, double-check the import.
 
### P5 — `Bridge.devices` mutates while the for-loop is iterating
 
Slint handles this correctly: the `for` re-evaluates when the model
changes, and the `Timer` / `TouchArea` per-row state resets. The
`mDNS` discovery thread pushes via `update_receivers_in_ui()` which
runs through `ui_weak.upgrade_in_event_loop(...)`, so all updates
happen on the Slint thread. No race.
 
### P6 — Long-press menu's `disconnect-clicked` still a no-op
 
Line 207-210 still has `// UI-only: no-op. Phase 8 wires to
Bridge.disconnect-receiver(id).` That callback doesn't exist on the
Bridge yet (see `bridge.slint:235-247` — only `connect-receiver`,
`stop-casting`, etc.). Don't fix it in this phase; it's
Phase-24 territory. The MVP does not need disconnect-from-row.
 
---
 
## 5. Stop conditions
 
The phase is "done" when **all** of the following hold:
 
1. `grep -n 'mock-devices' senders/android/ui/pages/connect_page.slint`
   matches only inside the unused `in-out property` declaration (or
   matches nothing if you also did Step 6).
2. `grep -n 'placeholder: would call connect-receiver' senders/android/ui`
   returns zero matches.
3. `cargo check` succeeds.
4. On a real device, tapping a discovered receiver causes the UI to
   leave `AppState::Disconnected` within 500 ms.
5. `adb logcat` shows `ConnectToDevice(...)` and
   `connect_with_device_info(...)` paired up.
6. The user-facing flow defined in
   `MVP-PHASE-implementation-instructions.md` §9.1 is end-to-end green.
 
---
 
## 6. Reading order if this phase fails
 
If Step 3 lands and tapping still does nothing, walk the chain in
order:
 
1. `lib.rs:1800-1810` — is the closure actually wired?
2. `lib.rs:747-794` — does `handle_event` reach
   `connect_with_device_info`?
3. `lib.rs:711-720` — does `connect_with_device_info` create the FCast
   device?
4. `Application.devices` HashMap — is the receiver's name actually a
   key? Compare against what `update_receivers_in_ui` pushes
   (`lib.rs:659-674`).
 
If step 4 misses, the issue is **MVP-PHASE-7** (you need
`[ReceiverItem]` with a stable id, not `[string]` keyed by display
name). File a follow-up; MVP can still ship if all your receivers have
distinct names.

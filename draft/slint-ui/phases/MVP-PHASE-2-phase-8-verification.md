# MVP-PHASE-2 — Phase-8 verification
 
> Verification of the Phase-8-shipped wirings that previously needed M2–M5
> work. **No new functionality.** This phase exists so you can confirm
> the cast-loop edges actually behave correctly once MVP-PHASE-1 unblocks
> the front door.
 
---
 
## 0. Goal
 
After this phase, you have:
 
- A documented procedure for verifying that A1 (status-items push), A2
  (app-version push), M3 (MediaProjection denial rollback), and M4
  (Stop button cleanup) work end-to-end.
- A clear list of the **two known Phase-8 deferrals** that remain
  inside the cast loop (the receiver/encoder/network status-items push
  on `Event::CaptureStarted` is *deferred*; the on-stop clear is *also*
  deferred). Both are acceptable for MVP and slated for follow-up.
- A one-line Rust diff for the rare case where `Bridge.app-version` is
  not pushed (defensive: in the audit it is pushed, but list the
  recovery just in case).
 
---
 
## 1. Pre-flight
 
### 1.1 What Phase 8 actually wired (verified against `master`)
 
| Cluster | What | Status | Citation |
|---|---|---|---|
| A1 (status-items, device-level) | Network / thermal / battery badges pushed via `push_status(...)` on a polling timer | **Wired** | `senders/android/src/lib.rs:1114-1144` |
| A1 (status-items, cast-loop) | Receiver name / encoder name / network info on `Event::CaptureStarted` | **Deferred** | `senders/android/src/lib.rs:546-549` (`build_status_items` helper is `currently unused`), `lib.rs:875-925, 955-961` (CaptureStarted/StartCast branches say "Phase 8 (deferred): wire Bridge.status-items here") |
| A2 (app-version) | `Bridge.app_version = env!("CARGO_PKG_VERSION")` at startup | **Wired** | `senders/android/src/lib.rs:1148-1149` |
| A3 (network-interfaces) | `Bridge.network_interfaces` pushed | **Wired** | `senders/android/src/lib.rs:1699` |
| A4 (recording-state) | `Bridge.recording_state` + `recording_elapsed_s` on Recording events | **Wired** | `senders/android/src/lib.rs:1846-1914` |
| A5 (log-entries) | `Bridge.log_entries` pushed by debug log subscriber | **Wired** | `PHASE-8-Section-2-cluster-A-readonly-view-models.md` |
| M3 (MediaProjection denial → rollback) | Already wired pre-Phase-8 | **Wired** | `senders/android/src/lib.rs:734-925` (`Event::EndSession`, `Event::CaptureStarted` timeout) |
| M4 (Stop button → clean rollback) | Already wired pre-Phase-8 | **Wired** | `senders/android/src/lib.rs:682-720` (`stop_cast`), `lib.rs:1822-1830` (`on_stop_casting`) |
 
### 1.2 What's known to be deferred (do **not** treat as MVP gates)
 
These two deferrals from `lib.rs:546-549` and `lib.rs:955-961` are
documented as Phase 8 carry-overs but **not** MVP-blocking:
 
| Carry-over | Where | Why deferred |
|---|---|---|
| Push `build_status_items(receiver_name, encoder_name, network_info)` into `Bridge.status_items` on `Event::CaptureStarted` | `lib.rs:955-961`, `lib.rs:875-925` | The casting-overlay component currently renders mock-status-items inline; the Rust-driven receiver/encoder/network badges are a polish improvement, not a cast-correctness requirement. |
| Clear `Bridge.status_items` on `Event::EndSession` and `Event::CaptureStopped` | `lib.rs:740, 834, 856` | Stale badges briefly visible after stop; cosmetic. |
 
If you want to drop both deferrals during MVP polish, see **§2.5**
below — it's a ~10-line Rust diff. **Not required to ship MVP.**
 
---
 
## 2. Steps
 
### 2.1 Step 1 — verify A1 (device-level status badges)
 
```bash
adb shell am force-stop org.fcast.android.sender
adb shell am start -n org.fcast.android.sender/.MainActivity
adb logcat | grep -E 'set_status_items|StatusItem'
```
 
**Expected (within ~3 s of app launch):**
- Three `StatusItem`s pushed into `Bridge.status_items`:
  network / thermal / battery.
 
**In-UI check:** Open the casting overlay (during a cast — but only if
MVP-PHASE-1 has shipped). The status overlay renders these three badges
via `senders/android/ui/components/status_overlay.slint`.
 
**If absent:** The `push_status` callback at `lib.rs:1114` is not being
invoked. Check whether the polling timer that drives it is started.
This is **not** an MVP gate — A1 is shipped, so any regression here is
worth a follow-up bug, not blocking the cast.
 
### 2.2 Step 2 — verify A2 (app-version)
 
```bash
adb shell am start -n org.fcast.android.sender/.MainActivity
# Open Settings → About in the UI.
```
 
**Expected:** Version string matches
`senders/android/Cargo.toml` `[package].version`.
 
**If empty or "0.0.1-dev":** Verify the push at `lib.rs:1148-1149`
ran. The `env!("CARGO_PKG_VERSION")` macro is resolved at compile
time — a release build picks up the version from `Cargo.toml`.
 
**If still wrong after rebuild:**
 
```rust
// In `senders/android/src/lib.rs`, immediately after the
// `MainWindow::new()` call (currently lines 1148-1149):
 
ui.global::<Bridge>()
    .set_app_version(env!("CARGO_PKG_VERSION").into());
```
 
This line is already there. If it isn't on your branch, **add it** —
~1 line.
 
### 2.3 Step 3 — verify M3 (MediaProjection denial rollback)
 
This is the most important MVP-adjacent verification because a stuck
state would leave the app unusable.
 
```bash
adb logcat | grep -E 'EndSession|stop_cast|onActivityResult|ACTION_MEDIA_PROJECTION'
```
 
**Procedure:**
 
1. With MVP-PHASE-1 shipped, tap a receiver, confirm settings.
2. When the MediaProjection consent dialog appears, **tap Cancel**.
 
**Expected (within ~500 ms of the Cancel tap):**
- `MainActivity.java` logs `onActivityResult(RESULT_CANCELED)` (no
  `ACTION_MEDIA_PROJECTION_STARTED` broadcast).
- Rust receives no `Event::CaptureStarted` for the cancelled session.
- UI returns to `AppState::SelectingSettings` or `Disconnected` after
  the receiver's session-timeout (see Application loop at
  `senders/android/src/lib.rs:855+`).
- No black `WaitingForMedia` screen persists.
 
**If the UI hangs in `WaitingForMedia`:** the timeout path is missing.
That's a real bug — file it, but it does **not** affect MVP shipability
if you document "do not cancel the MediaProjection prompt" as a known
limitation.
 
### 2.4 Step 4 — verify M4 (Stop button cleanup)
 
```bash
adb logcat | grep -E 'on_stop_casting|EndSession|stop_cast|stopCapture|disconnect'
```
 
**Procedure:**
 
1. Start a cast (MVP-PHASE-1 + grant MediaProjection).
2. With phone screen on receiver, tap **Stop** in the app.
 
**Expected log sequence (within ~1 s):**
 
```
on_stop_casting                            # callback fired
GLOB_EVENT_CHAN ... Event::EndSession      # event sent
handle_event Event::EndSession{disconnect:true}
stop_cast(true)                            # entry to cleanup
  MainActivity.stopCapture() invoked       # via JNI
  VirtualDisplay released
  FCast device.stop_playback()
  FCast device.disconnect()
  GStreamer pipeline shut down
AppState::Disconnected                     # final state
```
 
**In-UI check:** Connect page reappears within 1 s of the Stop tap.
Black frame on receiver disappears.
 
**If the UI hangs in `Casting`:** check the FCast device `disconnect()`
future at `lib.rs:712-720`. Likely cause is awaiting a TCP write that's
already dead. Adding a `tokio::time::timeout` wrapper is post-MVP
polish.
 
### 2.5 Step 5 (optional, post-MVP polish) — push cast-loop status items
 
If you want to drop the two known Phase-8 deferrals during MVP work,
this is the ~10-line Rust diff. **Not required for MVP.**
 
**File:** `senders/android/src/lib.rs`
 
#### 2.5.1 On capture start
 
**Inside `handle_event`'s `Event::CaptureStarted` arm**
(currently around `lib.rs:875-925`, find the comment
`// Phase 8 (deferred): wire Bridge.status-items here`):
 
```rust
// Before:
Event::CaptureStarted => {
    // …existing wiring (set_active_panel, transition AppState, etc.)…
    // Phase 8 (deferred): wire Bridge.status-items here from
    // build_status_items(&_receiver_name, _encoder_name, &_network_info).
}
 
// After:
Event::CaptureStarted => {
    // …existing wiring (set_active_panel, transition AppState, etc.)…
 
    let receiver_name = self
        .active_device
        .as_ref()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|| String::from("(unknown receiver)"));
    let encoder_name = String::from("WebRTC (BaseWebRTCSink internal)");
    let network_info = String::from("WHEP");
 
    let items = build_status_items(&receiver_name, &encoder_name, &network_info);
    self.ui_weak.upgrade_in_event_loop(move |ui| {
        let model: slint::ModelRc<crate::StatusItem> =
            std::rc::Rc::new(slint::VecModel::from(items)).into();
        ui.global::<Bridge>().set_status_items(model);
    })?;
}
```
 
The `build_status_items` helper already exists at `lib.rs:546` — it's
currently marked unused. This step revives it.
 
#### 2.5.2 On capture stop
 
In the three places that hold
`// Phase 8 (deferred): clear Bridge.status-items here`
(`lib.rs:740, 834, 856`):
 
```rust
// Replace each `// Phase 8 (deferred): clear Bridge.status-items here.`
// comment with:
 
self.ui_weak.upgrade_in_event_loop(move |ui| {
    let empty: slint::ModelRc<crate::StatusItem> =
        std::rc::Rc::new(slint::VecModel::<crate::StatusItem>::default()).into();
    ui.global::<Bridge>().set_status_items(empty);
})?;
```
 
This will conflict with A1's device-level badge push (which also writes
to `Bridge.status_items`). If both producers exist, the **last writer
wins** and you'll see flickering. The fix is either:
 
- Split into two model properties: `status_items_cast` +
  `status_items_device`, with `status_overlay.slint` concat'ing both.
- Or merge cast badges into the existing `push_status` snapshot path
  (`lib.rs:1114-1144`).
 
Recommend: **leave this deferral alone for MVP**; treat it as
post-MVP architectural cleanup. The status overlay shows correct
device-level badges today.
 
---
 
## 3. Verification
 
### 3.1 Greps
 
```bash
# A2 push is present.
grep -n 'set_app_version' senders/android/src/lib.rs
#  → expect: lib.rs:1148-1149
 
# A1 device-level push exists.
grep -n 'push_status\|set_status_items' senders/android/src/lib.rs
#  → expect: 1114, 1142
 
# M3 / M4 stop path exists.
grep -n 'fn stop_cast\|on_stop_casting\|Event::EndSession' senders/android/src/lib.rs
#  → expect: 682, 738, 1822, etc.
 
# Deferrals are still documented.
grep -n 'Phase 8 (deferred)' senders/android/src/lib.rs
#  → expect: 542, 740, 834, 856, 955  (5 matches)
```
 
### 3.2 In-UI verification matrix
 
| Verification | Procedure | Expected |
|---|---|---|
| A1 (device badges) | Cast or open status overlay | 3 badges visible: network 📶, thermal 🌡, battery 🔋 |
| A2 (app-version) | Settings → About | Real version string |
| M3 (MediaProjection denial) | Cancel consent dialog | UI rolls back to `SelectingSettings` |
| M4 (Stop button) | Tap Stop during cast | UI → Disconnected within 1 s |
 
---
 
## 4. Common pitfalls
 
### P1 — A1 verification fails because the casting overlay isn't open
 
A1's badges only render in the `CastingView` (status overlay is
embedded there). If you can't reach `Casting` because MVP-PHASE-1
hasn't shipped, you can't visually verify A1. Use the
`adb logcat | grep set_status_items` check instead.
 
### P2 — A2 shows "0.0.1-dev"
 
`env!("CARGO_PKG_VERSION")` is resolved at compile time. Stale builds
on the device → uninstall first, then rebuild + reinstall:
 
```bash
adb uninstall org.fcast.android.sender
cargo +nightly build -p fcast-sender-android --release ...
adb install ...
```
 
### P3 — M4 logs `disconnect` but UI hangs
 
The FCast device `disconnect()` future is awaiting an unreachable TCP
write. Wrap with `tokio::time::timeout(Duration::from_secs(2), ...)`
in `stop_cast` (`lib.rs:682`). **Not** MVP-required; file as
follow-up.
 
### P4 — M3 leaves the UI in `WaitingForMedia` forever
 
The timeout watchdog in `Application::run_event_loop` is supposed to
roll back. If it's silently failing, the `Event::CaptureStarted`
expectation hasn't been resolved within the timeout window. Check
`lib.rs:855+` for a `tokio::time::sleep` / `tokio::select!` pattern.
 
### P5 — Step 2.5 cast-loop status push creates flicker
 
A1's `push_status` and §2.5's CaptureStarted push both write
`Bridge.status_items`. They will overwrite each other. Solve via the
two approaches in §2.5.2 — or accept the cosmetic flicker for MVP and
revisit post-ship.
 
---
 
## 5. Stop conditions
 
The phase is "done" when:
 
1. All four verifications in §3.2 pass on a real device.
2. The `grep` checks in §3.1 produce the expected matches.
3. The two documented deferrals in §1.2 are tracked in your issue
   tracker as post-MVP polish, **not** as MVP gates.
 
You do **not** need to land §2.5. It's listed for completeness; the
MVP ships without it.

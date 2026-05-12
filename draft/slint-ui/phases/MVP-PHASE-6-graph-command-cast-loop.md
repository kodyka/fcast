# MVP-PHASE-6 — Graph-command cast loop (Tier 1.3)

> **The surface-unification step.** After MVP-PHASE-4 added a
> screen-capture source node, and MVP-PHASE-5 added a Whep destination
> family, the migration runtime knows how to build the entire cast
> pipeline as a graph. **This phase flips the switch.** It replaces
> the bespoke `Event::StartCast` / `Event::EndSession` GStreamer
> plumbing inside `senders/android/src/lib.rs` with calls into
> `migration::runtime::handle_command(...)`. Surface A becomes a thin
> orchestrator over Surface B.

---

## 0. Goal

Today, the Android sender has **two** parallel cast paths:

| Surface | Driver | Pipeline construction site |
|---|---|---|
| A (legacy) | `Event::StartCast` / `Event::CaptureStarted` | `senders/android/src/lib.rs:875-961` (`appsrc` → `WhepSink::new`) |
| B (migrated) | HTTP `MIGRATION_COMMAND_BIND` + `Smoke Graph` quick-action | `senders/android/src/migration/runtime.rs` + `node_manager.rs` |

After this phase, **only Surface B exists**. The `Event::StartCast` /
`Event::EndSession` handlers become ~50-line adapters that:

1. Issue `CreateScreenCaptureSource` (PHASE-4 node).
2. Issue `CreateDestination { family: Whep { server_port: 0 }, … }`
   (PHASE-5 family).
3. Issue `Connect { src_id, sink_id, video: true }`.
4. Issue `Start { id: dst_id }`.
5. Poll `getinfo` until `bound_port_v4` is populated, then send the
   WHEP URL to the active FCast receiver via `device.load(...)` —
   exactly as the legacy `Event::SignallerStarted` handler did
   (`lib.rs:754-794`).

On `Event::EndSession` / `Event::CaptureStopped`, the adapter issues
`Disconnect` + `Remove` for both nodes, and `mcore::transmission::WhepSink`
becomes dead code for Android (kept only for desktop, behind
`#[cfg(not(target_os = "android"))]`).

---

## 1. Pre-flight

### 1.1 What MUST be shipped before this phase

| Prerequisite | Where |
|---|---|
| MVP-PHASE-4 (`Command::CreateScreenCaptureSource`) | `senders/android/src/migration/protocol.rs`, `nodes/screen_capture.rs` |
| MVP-PHASE-5 (`DestinationFamily::Whep` + `DestinationInfo.bound_port_v*`) | `senders/android/src/migration/protocol.rs`, `nodes/destination.rs` |
| MVP-PHASE-3 (Surface B runtime starts on app launch) | `senders/android/src/lib.rs:1035` (`start_graph_runtime()`) — already shipped pre-MVP |

If any of those is missing, this phase will compile but its smoke
test will fail at step 2 (CreateDestination returns "Unknown family"
or step 1 returns "Unknown command").

### 1.2 The five touch points in `lib.rs`

| # | Line | What it does today | What it does after this phase |
|---|---|---|---|
| 1 | `lib.rs:738-746` | `Event::EndSession` → `stop_cast(true)` (legacy WhepSink shutdown). | Issue `Disconnect L1 + Remove cap-1 + Remove tv-1` graph commands, then `stop_cast` calls become no-ops. |
| 2 | `lib.rs:754-794` | `Event::SignallerStarted` → build WHEP URL → `device.load(...)`. | Replaced by a `tokio::spawn` polling `getinfo` until `bound_port_v4` is `Some(_)`, then identical `device.load(...)`. |
| 3 | `lib.rs:875-961` | `Event::CaptureStarted` + `Event::StartCast` → `appsrc` + `WhepSink::new`. | Issue `CreateScreenCaptureSource cap-1 + CreateDestination tv-1 + Connect L1 + Start tv-1` graph commands. |
| 4 | `lib.rs:704-706` | `stop_cast` → `tx_sink.shutdown()`. | Remove the `tx_sink` field entirely on Android; the shutdown is implicit via `Remove tv-1`. |
| 5 | `lib.rs:537, 602, 943-950` | `tx_sink: Option<WhepSink>` field + `tx_sink = Some(WhepSink::new(...))`. | Delete the field on `#[cfg(target_os = "android")]`. |

Approximate scope: **~200–300 lines of Rust, all in one file
(`senders/android/src/lib.rs`)**, net –300 +200 (the cast loop body
collapses to a sequence of `handle_command` calls).

### 1.3 Why one big diff and not five small ones?

The five touch points share state (`tx_sink`, `our_source_url`,
`local_address`, `current_device_id`). Touching them piecemeal would
leave intermediate commits with broken invariants (e.g. `tx_sink =
None` but `Event::SignallerStarted` still expecting to read it). Land
this as one atomic PR, and keep the legacy paths behind
`#[cfg(not(target_os = "android"))]` so the desktop sender is
unaffected.

---

## 2. Steps

### 2.1 Step 1 — define node IDs in one place

**File:** `senders/android/src/lib.rs` (top of the file, after the
`lazy_static!` block at line 71):

```rust
// senders/android/src/lib.rs

// Graph node IDs for the unified cast loop (MVP-PHASE-6).
// One source, one destination, one link — the entire cast loop.
#[cfg(target_os = "android")]
const CAST_SOURCE_ID: &str = "cast-screen-1";
#[cfg(target_os = "android")]
const CAST_DESTINATION_ID: &str = "cast-whep-1";
#[cfg(target_os = "android")]
const CAST_LINK_ID: &str = "cast-link-1";
```

Hard-coded IDs are fine because there is only one cast at a time.
The legacy code does the same implicitly with `Option<WhepSink>`.

### 2.2 Step 2 — replace `Event::StartCast` / `Event::CaptureStarted`

**File:** `senders/android/src/lib.rs`

**Before** (lines 875-961 — the `Event::CaptureStarted` body that
builds `appsrc` + `WhepSink`):

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted => {
    set_capture_active(true);
    let appsrc = gst_app::AppSrc::builder()
        .caps(/* … */)
        .is_live(true)
        /* … */
        .build();

    let mut caps = None::<gst::Caps>;
    appsrc.set_callbacks(/* large need-data closure */);

    let source_config = SourceConfig::Video(mcore::VideoSource::Source(appsrc));

    self.tx_sink = Some(mcore::transmission::WhepSink::new(
        source_config,
        self.event_tx.clone(),
        tokio::runtime::Handle::current(),
        1920, 1080, 30,
    )?);

    // …status-items deferred…

    self.ui_weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().invoke_change_state(AppState::Casting);
    })?;
}
```

**After:**

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted => {
    set_capture_active(true);

    // Build the unified screen-capture → WHEP graph via the migration
    // runtime. Replaces the legacy WhepSink pipeline construction.
    use crate::migration::protocol::{Command, DestinationFamily};

    let scale_width = self.last_cast_request_scale_width.unwrap_or(1280);
    let scale_height = self.last_cast_request_scale_height.unwrap_or(720);
    let fps = self.last_cast_request_max_framerate.unwrap_or(30);

    let commands = [
        Command::CreateScreenCaptureSource {
            id: CAST_SOURCE_ID.into(),
            width: scale_width,
            height: scale_height,
            fps,
        },
        Command::CreateDestination {
            id: CAST_DESTINATION_ID.into(),
            family: DestinationFamily::Whep { server_port: 0 },
            audio: false,
            video: true,
        },
        Command::Connect {
            link_id: CAST_LINK_ID.into(),
            src_id: CAST_SOURCE_ID.into(),
            sink_id: CAST_DESTINATION_ID.into(),
            audio: false,
            video: true,
            config: None,
        },
        Command::Start {
            id: CAST_DESTINATION_ID.into(),
            cue_time: None,
            end_time: None,
        },
        Command::Start {
            id: CAST_SOURCE_ID.into(),
            cue_time: None,
            end_time: None,
        },
    ];

    for cmd in commands {
        let result = crate::migration::runtime::handle_command(cmd);
        if let crate::migration::protocol::CommandResult::Error(err) = result {
            error!(?err, "Failed to build unified cast graph");
            self.stop_cast(false).await?;
            return Ok(ShouldQuit::No);
        }
    }

    // Spawn the bound-port poll loop. When `getinfo` returns
    // `bound_port_v4 = Some(p)`, we forward it as the existing
    // Event::SignallerStarted, so the rest of the cast loop is
    // unchanged.
    let event_tx = self.event_tx.clone();
    tokio::spawn(async move {
        for _ in 0..200 {  // 200 × 100ms = 20s timeout, plenty for WHEP.
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let info_result = crate::migration::runtime::handle_command(
                crate::migration::protocol::Command::GetInfo {
                    id: Some(CAST_DESTINATION_ID.into()),
                },
            );
            if let crate::migration::protocol::CommandResult::Info(snapshot) = info_result {
                if let Some(crate::migration::protocol::NodeInfo::Destination(d)) =
                    snapshot.nodes.get(CAST_DESTINATION_ID)
                {
                    if let (Some(v4), Some(v6)) = (d.bound_port_v4, d.bound_port_v6) {
                        let _ = event_tx.send(Event::SignallerStarted {
                            bound_port_v4: v4,
                            bound_port_v6: v6,
                        });
                        return;
                    }
                }
            }
        }
        error!("Whep destination never bound a port within 20s — giving up");
    });

    self.ui_weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().invoke_change_state(AppState::Casting);
    })?;
}
```

The `last_cast_request_*` fields are new — populate them in the
`Event::StartCast` handler so that `CaptureStarted` knows what scale
factors the user picked:

```rust
// senders/android/src/lib.rs — fields on the event-loop struct
#[cfg(target_os = "android")]
last_cast_request_scale_width: Option<u32>,
#[cfg(target_os = "android")]
last_cast_request_scale_height: Option<u32>,
#[cfg(target_os = "android")]
last_cast_request_max_framerate: Option<u32>,
```

…and inside `Event::StartCast` (lines 963-1008), before the JNI
`startScreenCapture` call:

```rust
#[cfg(target_os = "android")]
Event::StartCast { scale_width, scale_height, max_framerate } => {
    self.last_cast_request_scale_width = Some(scale_width);
    self.last_cast_request_scale_height = Some(scale_height);
    self.last_cast_request_max_framerate = Some(max_framerate);

    // …existing JNI startScreenCapture call…
}
```

### 2.3 Step 3 — leave `Event::SignallerStarted` mostly alone

The existing `Event::SignallerStarted` handler (`lib.rs:754-794`)
already does the right thing: build the WHEP URL, push it to the FCast
receiver via `device.load(...)`. With Step 2 above, the bound-port
poll loop emits `Event::SignallerStarted` itself, so this handler
keeps working unchanged.

**One small change:** today the URL is built via
`self.tx_sink.as_ref().unwrap().get_play_msg(...)`. With `tx_sink`
gone, that helper needs to be either:

- (a) inlined: WHEP URL is `http://<local-addr>:<bound_port>/endpoint`,
  content-type is `application/sdp`. See `whep_signaller.rs:33-35`
  for the exact endpoint path.
- (b) moved out of `WhepSink` into a free function in `mcore` so the
  migration adapter can call it directly.

**(b) is preferred** because it keeps the `get_play_msg` URL
construction in one place:

```rust
// sdk/mirroring_core/src/transmission.rs (new free function)
pub fn build_whep_play_msg(addr: IpAddr, bound_port: u16) -> (String, String) {
    let host = addr_to_url_string(addr);
    let url = format!("http://{host}:{bound_port}/endpoint");
    ("application/sdp".to_string(), url)
}
```

Then **before** in `lib.rs:767-771`:

```rust
let (content_type, url) = self
    .tx_sink
    .as_ref()
    .unwrap()
    .get_play_msg(addr.into(), bound_port);
```

**After:**

```rust
let (content_type, url) =
    mcore::transmission::build_whep_play_msg(addr.into(), bound_port);
```

### 2.4 Step 4 — replace `Event::EndSession` / `stop_cast`

**File:** `senders/android/src/lib.rs`

**Before** (lines 682-709, `stop_cast`):

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    let android_app = self.android_app.clone();
    self.ui_weak.upgrade_in_event_loop(move |_| {
        call_java_method_no_args(&android_app, JavaMethod::StopCapture);
    })?;

    if let Some(active_device) = self.active_device.take() {
        tokio::spawn(async move {
            if stop_playback { /* stop_playback + disconnect */ }
        });
    }

    if let Some(mut tx_sink) = self.tx_sink.take() {
        tx_sink.shutdown();
    }

    Ok(())
}
```

**After:**

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    let android_app = self.android_app.clone();
    self.ui_weak.upgrade_in_event_loop(move |_| {
        call_java_method_no_args(&android_app, JavaMethod::StopCapture);
    })?;

    if let Some(active_device) = self.active_device.take() {
        tokio::spawn(async move {
            if stop_playback {
                debug!("Stopping playback");
                log_err!(active_device.stop_playback(), "Failed to stop playback");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            debug!("Disconnecting from active device");
            log_err!(active_device.disconnect(), "Failed to disconnect from active device");
        });
    }

    // NEW — tear down the unified cast graph. Removing the source
    // implicitly disconnects all of its consumer links; removing the
    // destination tears down its pipeline including the WHEP signaller.
    #[cfg(target_os = "android")]
    {
        use crate::migration::protocol::Command;
        for id in [CAST_LINK_ID] {
            let _ = crate::migration::runtime::handle_command(Command::Disconnect {
                link_id: id.into(),
            });
        }
        for id in [CAST_SOURCE_ID, CAST_DESTINATION_ID] {
            let _ = crate::migration::runtime::handle_command(Command::Remove {
                id: id.into(),
            });
        }
    }

    // Legacy WhepSink shutdown — kept only on non-Android targets.
    #[cfg(not(target_os = "android"))]
    if let Some(mut tx_sink) = self.tx_sink.take() {
        tx_sink.shutdown();
    }

    Ok(())
}
```

### 2.5 Step 5 — gate `tx_sink` to non-Android targets

**File:** `senders/android/src/lib.rs` (line 537, struct field):

```rust
// Before
tx_sink: Option<WhepSink>,

// After
#[cfg(not(target_os = "android"))]
tx_sink: Option<WhepSink>,
```

…and the corresponding `tx_sink: None` initialiser (line 602):

```rust
// Before
tx_sink: None,

// After
#[cfg(not(target_os = "android"))]
tx_sink: None,
```

All reads of `self.tx_sink` outside the `stop_cast` / `SignallerStarted`
paths in §2.3 / §2.4 must also be `cfg`'d:

```bash
grep -n 'self\.tx_sink' senders/android/src/lib.rs
# → expect 4-5 hits. Walk each and either delete it (Android path
#   replaced by graph commands) or wrap in #[cfg(not(target_os = "android"))].
```

### 2.6 Step 6 — keep the FRAME_PAIR producer untouched

The JNI side (`MainActivity.startScreenCapture` →
`nativeProcessFrame` → write to `FRAME_PAIR`) is **unchanged**. PHASE-4
already wired its consumer (`ScreenCaptureNode::wire_need_data`) to
read from `FRAME_PAIR`. The only thing that changes is **who builds
the GStreamer pipeline** that consumes those frames:

| Before | After |
|---|---|
| `lib.rs::Event::CaptureStarted` builds `appsrc` + `WhepSink` | `ScreenCaptureNode::build_live_pipeline` builds `appsrc` + `videoconvert` + `appsink`, and `DestinationNode` (Whep family) builds `BaseWebRTCSink`. The `StreamBridge` (`media_bridge.rs`) fans the source `appsink` into the destination `appsrc`. |

So FRAME_PAIR / CAPTURE_ACTIVE are still the cross-language
hand-off — they just feed a different consumer.

### 2.7 Step 7 — preserve the `set_capture_active(false)` calls

The legacy code calls `set_capture_active(false)` in
`Event::CaptureStopped` (line 850), `Event::CaptureCancelled` (line
854), and (transitively) on `stop_cast` via the `MainActivity.stopCapture`
JNI call. **Keep all of those** — they tell the JNI-side EncoderCallback
to stop pushing frames, which is what unblocks the
`FRAME_PAIR` consumer's `cvar.wait` loop in `ScreenCaptureNode`.

### 2.8 Step 8 — adjust the `mod migration` exports

**File:** `senders/android/src/migration/mod.rs` (currently only
re-exports `node_manager`, `runtime`, etc.):

```rust
// senders/android/src/migration/mod.rs

pub mod media_bridge;
pub mod messages;
pub mod node_manager;
pub mod nodes;
pub mod protocol;
pub mod runtime;
```

If your `protocol::CommandResult::Info(_)` and `NodeInfo::Destination(_)`
variants aren't re-exported at the crate root, the `lib.rs` snippets
in §2.2 will need explicit paths. Either:

- (a) Add `pub use protocol::{Command, CommandResult, DestinationFamily,
  NodeInfo};` to `migration/mod.rs`. Recommended — the call sites get
  shorter.
- (b) Live with the verbose paths.

### 2.9 Step 9 — flip the `mock-mvp-using-legacy` flag (optional)

If you want to roll out the unified path incrementally (instead of as
one big-bang switch), add a runtime feature flag:

```rust
// senders/android/src/lib.rs

#[cfg(target_os = "android")]
fn use_unified_cast_graph() -> bool {
    std::env::var("FCAST_UNIFIED_CAST_GRAPH").map(|v| v != "0").unwrap_or(true)
}
```

…then guard the Step 2 / Step 4 changes:

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted => {
    set_capture_active(true);
    if use_unified_cast_graph() {
        // …new graph-command path…
    } else {
        // …legacy WhepSink construction (the current code, unchanged)…
    }
}
```

This is **optional**, but useful for canary deployments. Default is
`true` (= unified). The user disables the flag via
`adb shell setprop debug.fcast.unified_cast_graph 0` + bridging
that into `env::var` at app startup.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android

# Non-Android targets must still build with the legacy WhepSink:
cargo +nightly check -p fcast-sender-desktop
```

Both clean.

### 3.2 Unit tests

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::tests
```

All previously-passing tests still pass. No new tests in this phase —
the change is structural, not behavioural; behaviour is covered by the
PHASE-4 and PHASE-5 tests.

### 3.3 On-device smoke

```bash
adb install -r target/aarch64-linux-android/release/apk/fcast-sender-android.apk
adb shell am force-stop org.fcast.android.sender
adb shell am start -n org.fcast.android.sender/.MainActivity

# Filter the cast-loop graph commands.
adb logcat | grep -E 'CAST_SOURCE_ID|CAST_DESTINATION_ID|CAST_LINK_ID|handle_command|SignallerStarted'
```

**Expected sequence on a successful cast:**

```
… on_connect_receiver: "Living Room TV"
… Handling event: ConnectToDevice(...)
… Handling event: FromDevice(... StateChanged(Connected ...))
… ChangeState(SelectingSettings)
… on_start_casting: 1280, 720, 30
… Handling event: StartCast { scale_width: 1280, scale_height: 720, max_framerate: 30 }
… Java method call: startScreenCapture(1280, 720, 30)
… ChangeState(WaitingForMedia)
… Handling event: CaptureStarted
… NodeManager::dispatch(CreateScreenCaptureSource { id: "cast-screen-1", ... })
… NodeManager::dispatch(CreateDestination { id: "cast-whep-1", family: Whep(...), ... })
… NodeManager::dispatch(Connect { link_id: "cast-link-1", ... })
… NodeManager::dispatch(Start { id: "cast-whep-1", ... })
… NodeManager::dispatch(Start { id: "cast-screen-1", ... })
… ChangeState(Casting)
… [tokio::spawn] getinfo … bound_port_v4: Some(39871), bound_port_v6: Some(39872)
… Handling event: SignallerStarted { bound_port_v4: 39871, bound_port_v6: 39872 }
… Sending play message: application/sdp http://192.168.1.42:39871/endpoint
```

**On stop:**

```
… on_stop_casting
… Handling event: EndSession { disconnect: true }
… ChangeState(Disconnected)
… Java method call: stopCapture()
… NodeManager::dispatch(Disconnect { link_id: "cast-link-1" })
… NodeManager::dispatch(Remove { id: "cast-screen-1" })
… NodeManager::dispatch(Remove { id: "cast-whep-1" })
… Disconnecting from active device
```

### 3.4 End-to-end cast

With a real FCast receiver on the same network:

1. Open the sender app.
2. Tap a discovered receiver row (relies on MVP-PHASE-1).
3. Confirm consent on the MediaProjection prompt.
4. The receiver displays the phone screen within ~2 s.
5. Tap **Stop** in the sender.
6. The receiver returns to its idle screen within ~1 s.

If any of these fail, check §3.3's log filter for the exact graph
command that didn't return `success`.

### 3.5 Negative test — kill the runtime mid-cast

```bash
adb logcat | grep -E 'shutdown_graph_runtime|stop_cast'
```

While casting, run:

```bash
adb shell run-as org.fcast.android.sender kill -SIGUSR2 $(adb shell pidof org.fcast.android.sender)
```

…or just background-kill the activity. On the next foreground, the
app should re-issue `start_graph_runtime()` (lib.rs:1035) and the
graph should be empty (no leftover nodes). Confirm with:

```bash
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | jq '.result.info.nodes'
# → {}
```

---

## 4. Common pitfalls

### P1 — `Event::SignallerStarted` fires twice

If both the legacy `WhepSink::new` path **and** the new graph path are
active simultaneously (e.g. you skipped Step 5's `#[cfg]` gate), you'll
get two `Event::SignallerStarted` events with different ports, and
the receiver will be told to pull from a random one of the two
servers — usually the wrong one. Symptom: the receiver shows "Cannot
connect to WHEP endpoint" or the stream is choppy.

**Fix:** ensure `self.tx_sink` is `#[cfg(not(target_os = "android"))]`
and that no Android-path code constructs `WhepSink::new`.

### P2 — `getinfo` returns `Info`, not `Success`

The `CommandResult::Info(snapshot)` variant is distinct from
`CommandResult::Success`. The §2.2 poll loop must `match` on `Info` —
matching on `Success` will silently drop the snapshot.

```rust
match crate::migration::runtime::handle_command(Command::GetInfo { id: Some(...) }) {
    CommandResult::Info(snapshot) => { /* use snapshot.nodes */ }
    CommandResult::Success | CommandResult::Error(_) => { /* unexpected */ }
}
```

### P3 — `last_cast_request_*` not populated → 0×0 capture

If `Event::CaptureStarted` fires before `Event::StartCast`'s
`last_cast_request_*` setters run, the unwrap-or-defaults at the top
of §2.2 kick in: 1280×720@30. That's a safe default but may not
match what the user selected in the settings page. To debug:

```bash
adb logcat | grep -E 'last_cast_request_scale|CreateScreenCaptureSource'
```

If you see `width: 1280, height: 720` but the user picked 1920×1080,
the event ordering is wrong. Fix by populating the `last_cast_request_*`
fields **synchronously** in the `Bridge.on_start_casting` callback
itself (lib.rs:1809-1819), not inside the async event handler.

### P4 — Removing a destination doesn't tear down its WHEP server

If the `Remove` command for `cast-whep-1` doesn't actually call
`teardown_live_pipeline()`, the WHEP TCP listener (in
`whep_signaller.rs::imp::Signaller`) stays bound until the process
exits. Symptom: a stale port stays open, and the next cast picks up
a *different* port — confusing logs, but not a correctness bug.

**Fix:** verify that `NodeManager::remove_node()` calls
`node.stop()` before dropping the `NodeRecord`. Search:

```bash
grep -n 'fn remove_node\|teardown_live_pipeline' \
    senders/android/src/migration/node_manager.rs \
    senders/android/src/migration/nodes/destination.rs
```

### P5 — The `tokio::spawn` poll loop outlives the cast

If the user starts a second cast before the first one's poll loop
times out, two poll loops race to emit `Event::SignallerStarted`.
Both will succeed (the runtime returns the latest `bound_port_v*`),
but the second `device.load(...)` may be issued before the first
WHEP server is fully torn down, leading to a brief "stream switching"
hiccup on the receiver.

**Mitigation:** include a generation counter in `Event::SignallerStarted`
or just check `self.active_device.is_some()` before issuing
`device.load(...)`. Pragmatic fix: the user can't realistically
start two casts inside the 20s timeout window — defer.

### P6 — Desktop sender still uses `WhepSink`

The diff in §2.5 puts `tx_sink` behind `#[cfg(not(target_os = "android"))]`.
The desktop sender (`senders/desktop/`) **also** uses `WhepSink` but
through a different binary entry point. The migration runtime is
Android-only for now. **Do not** propagate this change to the desktop
sender in this PR.

### P7 — `MIGRATION_COMMAND_BIND` is not required for the in-process path

`migration::runtime::handle_command(...)` is a **direct Rust call** —
it doesn't go through HTTP. So the cast loop works even when the
HTTP command server isn't bound. Don't make the cast loop depend on
`MIGRATION_COMMAND_BIND` being set; that's only for external smoke
testing.

---

## 5. Stop conditions

The phase is "done" when:

1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. All migration-runtime unit tests still pass.
3. The on-device cast in §3.3 / §3.4 succeeds end-to-end, with the
   exact graph-command log sequence in §3.3.
4. **No call to `mcore::transmission::WhepSink::new` remains under
   `#[cfg(target_os = "android")]`:**

```bash
grep -n 'WhepSink::new' senders/android/src/lib.rs
# → expect: zero matches, OR only matches inside #[cfg(not(target_os = "android"))]
```

5. **No `self.tx_sink` read remains under `#[cfg(target_os = "android")]`:**

```bash
grep -nB1 'self\.tx_sink' senders/android/src/lib.rs | grep -v 'cfg(not'
# → expect: zero matches that aren't already cfg-gated.
```

6. The on-stop tear-down in §3.3 logs `Remove(cast-screen-1)` and
   `Remove(cast-whep-1)` in that order — both within 200ms of the
   `EndSession` event.

---

## 6. Why this matters

This is the **final unification step**. After it ships:

| Surface | Status |
|---|---|
| Surface A (legacy WHEP cast loop on Android) | **Deleted.** The `Event::StartCast` body is now a 4-command graph builder. |
| Surface B (migration runtime) | The single canonical pipeline construction site for Android. |
| `mcore::transmission::WhepSink` | Desktop-only. Android cfg-gated out. |

This brings the FCast sender into alignment with the migration
runtime's design goal: **one node-graph API for every pipeline**,
whether the user assembled it via HTTP, JNI, or the cast loop.

Downstream cleanup (out of scope, but enabled by this phase):

- Removing the `mcore::transmission::WhepSink::new` Android path
  entirely (current cfg-gated dead code).
- Reusing the runtime's `Mixer` node for picture-in-picture during
  cast (today impossible because the WHEP sink lives outside the
  graph).
- Moving the cast loop's bitrate/resolution settings to live as
  `AddControlPoint` commands on the destination, rather than
  encoder constructor args.
- Recording (PHASE-23) becomes a second `Destination::LocalFile`
  node connected to the same source — no duplicate encoder chain.

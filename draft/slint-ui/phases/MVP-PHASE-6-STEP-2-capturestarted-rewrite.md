# MVP-PHASE-6 — Step 2: replace `Event::CaptureStarted` / `Event::StartCast` with graph commands

> Part 2 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 1 — node ID constants](./MVP-PHASE-6-STEP-1-node-id-constants.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Replace the bespoke `appsrc` + `WhepSink::new(...)` construction
inside `Event::CaptureStarted` (the heart of the legacy cast loop)
with a sequence of `migration::runtime::handle_command(...)` calls:

```
CreateScreenCaptureSource cast-screen-1
CreateDestination       cast-whep-1   (family: Whep { server_port: 0 })
Connect                 cast-link-1   (src: cast-screen-1, sink: cast-whep-1)
Start                   cast-whep-1
Start                   cast-screen-1
```

Plus a `tokio::spawn` polling loop that watches `getinfo` until the
WHEP destination's `bound_port_v4` is populated, then emits
`Event::SignallerStarted` exactly as the legacy
`WhepSink`-on-server-started signal used to.

`Event::StartCast` is also lightly touched — it stores the user's
chosen `scale_width / scale_height / max_framerate` on
`self.last_cast_request_*` so `CaptureStarted` can read them.

This is the **largest step in PHASE-6** (~150 Rust lines net change,
all in `lib.rs`). It's the literal flip of Surface A → Surface B.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Event::CaptureStarted` handler | `senders/android/src/lib.rs:875-961` |
| `Event::StartCast` handler | `lib.rs:963-1018` |
| `WhepSink::new` (the thing we're replacing) | `sdk/mirroring_core/src/transmission.rs:475-528` |
| `set_capture_active(true)` (kept — see [Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md)) | `lib.rs:1023` |
| Graph commands (defined in PHASE-4 + PHASE-5) | `senders/android/src/migration/protocol.rs` |
| `migration::runtime::handle_command` | `senders/android/src/migration/runtime.rs` |
| `Bridge.change-state(AppState::Casting)` (kept — UI signal) | `lib.rs:953-957` |
| `Event::SignallerStarted` (consumed by [Step 3](./MVP-PHASE-6-STEP-3-signaller-started-helper.md)) | `lib.rs:754-794` |

### 1.2 Why we still need `set_capture_active(true)`

The JNI-side `MainActivity.startScreenCapture(...)` writes frames
into `FRAME_PAIR` only while `CAPTURE_ACTIVE` is `true`. Both the
legacy cast loop and the new `ScreenCaptureNode` read from the same
global, so this gating mechanism is unchanged. See
[Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md)
for the full preservation rationale.

### 1.3 Why a `tokio::spawn` poll instead of a callback

The migration runtime is **synchronous** at its public API
(`handle_command(...)` returns immediately). The WHEP server's
`on-server-started` signal fires inside the GStreamer state machine
which the runtime drives on its 100ms refresh thread. There is no
clean callback we can register from outside the runtime.

Polling `getinfo` every 100ms with a 20s timeout is:

- Cheap (`getinfo` just reads the `nodes` HashMap).
- Bounded (20s, after which we log + abort).
- Naturally retries (the refresh thread fires the
  `on-server-started` signal handler before the next tick).

A cleaner future: add a `Command::WaitForPort { id, timeout_ms }`
that the runtime handles by sleeping until the slot is populated.
Out of scope for PHASE-6.

---

## 2. The change

### 2.1 Capture user-chosen scale factors in `Event::StartCast`

**File:** `senders/android/src/lib.rs`

Add three new fields to the event-loop struct (around line 537):

```rust
#[cfg(target_os = "android")]
last_cast_request_scale_width: Option<u32>,
#[cfg(target_os = "android")]
last_cast_request_scale_height: Option<u32>,
#[cfg(target_os = "android")]
last_cast_request_max_framerate: Option<u32>,
```

Initialise them to `None` in the constructor (around line 602):

```rust
#[cfg(target_os = "android")]
last_cast_request_scale_width: None,
#[cfg(target_os = "android")]
last_cast_request_scale_height: None,
#[cfg(target_os = "android")]
last_cast_request_max_framerate: None,
```

Populate them at the top of `Event::StartCast` (around lines 963-1008):

```rust
#[cfg(target_os = "android")]
Event::StartCast { scale_width, scale_height, max_framerate } => {
    // NEW — remember the user's choices for CaptureStarted to read.
    self.last_cast_request_scale_width = Some(scale_width);
    self.last_cast_request_scale_height = Some(scale_height);
    self.last_cast_request_max_framerate = Some(max_framerate);

    // …existing JNI MainActivity.startScreenCapture(...) call,
    //   unchanged…
}
```

### 2.2 Rewrite `Event::CaptureStarted`

**Before** (lines 875-961, abbreviated — the existing body that
builds `appsrc` + `WhepSink::new` is ~80 lines):

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted => {
    set_capture_active(true);
    let appsrc = gst_app::AppSrc::builder()
        .caps(/* … */)
        .is_live(true)
        /* … */
        .build();

    appsrc.set_callbacks(/* large need-data closure */);

    let source_config = SourceConfig::Video(mcore::VideoSource::Source(appsrc));

    self.tx_sink = Some(mcore::transmission::WhepSink::new(
        source_config,
        self.event_tx.clone(),
        tokio::runtime::Handle::current(),
        1920, 1080, 30,
    )?);

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
    use crate::migration::protocol::{Command, CommandResult, DestinationFamily};

    let scale_width  = self.last_cast_request_scale_width.unwrap_or(1280);
    let scale_height = self.last_cast_request_scale_height.unwrap_or(720);
    let fps          = self.last_cast_request_max_framerate.unwrap_or(30);

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
            src_id:  CAST_SOURCE_ID.into(),
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
        if let CommandResult::Error(err) = crate::migration::runtime::handle_command(cmd) {
            error!(?err, "Failed to build unified cast graph");
            self.stop_cast(false).await?;
            return Ok(ShouldQuit::No);
        }
    }

    // Spawn the bound-port poll loop. When `getinfo` returns
    // `bound_port_v4 = Some(p)` and `bound_port_v6 = Some(p)`, we
    // forward it as the existing Event::SignallerStarted, so the
    // rest of the cast loop is unchanged.
    let event_tx = self.event_tx.clone();
    tokio::spawn(async move {
        for _ in 0..200 {  // 200 × 100ms = 20s timeout, plenty for WHEP.
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let info = crate::migration::runtime::handle_command(
                crate::migration::protocol::Command::GetInfo {
                    id: Some(CAST_DESTINATION_ID.into()),
                },
            );
            if let crate::migration::protocol::CommandResult::Info(snapshot) = info {
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

### 2.3 Order matters: `Start dst` before `Start src`

The destination's WHEP signaller binds its TCP listener as part of
moving its pipeline to `Playing` (gated on `Start cast-whep-1`).
The source pipeline starts pushing buffers as soon as its
`Start` lands, which feeds `media_bridge::StreamBridge`. If you
start the source **first**, the bridge has no attached sink yet and
buffers are silently dropped for the ~1 refresh tick before the
destination catches up. Visible symptom: black/no-buffer frames in
the first second of the cast.

Always: `Start dst` → `Start src`.

### 2.4 Why `audio: false`

WHEP under Android via `BaseWebRTCSink` is video-only in PHASE-5 (no
`avenc_aac` chain). If MediaProjection gains audio-capture support
later, this becomes `audio: true` + a parallel audio appsrc on
`ScreenCaptureNode` — that's a future-phase change, not PHASE-6.

### 2.5 Failure semantics — partial graph teardown

If any of the five commands fails, we issue `self.stop_cast(false)`
to tear down anything that did succeed. `stop_cast` (rewritten in
[Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md)) is idempotent
on missing nodes (it logs but doesn't error). This means partial
graphs are auto-cleaned.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean** *after* this step also lands a corresponding
[Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md) (gating `tx_sink`
behind `cfg(not(target_os = "android"))`). Until then, the
compiler complains that `self.tx_sink` is never assigned on
Android — that's correct and fixed by Step 5.

### 3.2 Smoke (on-device, end-to-end)

The full smoke recipe is in §3 of the parent doc. The relevant
checkpoint after just Step 2:

```bash
# After tapping "Cast":
adb logcat | grep -E '(handle_command|signaller|Whep)'
# Expect:
#   handle_command CreateScreenCaptureSource ok
#   handle_command CreateDestination Whep ok
#   handle_command Connect ok
#   handle_command Start cast-whep-1 ok
#   handle_command Start cast-screen-1 ok
#   on-server-started bound_port_v4=Some(40000+)
#   Event::SignallerStarted bound_port_v4=40000+
```

If `on-server-started` fires but no `Event::SignallerStarted`
follows, the poll loop never matched both `bound_port_v4` and
`bound_port_v6` — check that PHASE-5 STEP-6 correctly populates
**both** slots (the signaller emits `(v4, v6)` as a single signal).

### 3.3 Grep

```bash
grep -nE 'WhepSink::new\b' senders/android/src/lib.rs
# → 0 matches on the Android cfg path (still present in #[cfg(not(target_os = "android"))] blocks for desktop).
grep -nE 'migration::runtime::handle_command' senders/android/src/lib.rs
# → at least 6 matches (5 in CaptureStarted, 1 in the spawned poll loop,
#   plus more after Step 4).
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to clone `self.event_tx` before the `tokio::spawn`

The poll loop closure must be `'static`, which means it can't hold
`&self`. Capture an owned `mpsc::Sender<Event>` clone instead.
Forgetting this is the most common borrow-checker error in this
step.

### P2 — Ordering of `Start dst` vs `Start src`

See §2.3 above. Reversing them produces a 1-frame black flicker at
cast start and (rarely) a permanent stall if the source's
`need-data` triggers before the bridge attaches.

### P3 — `last_cast_request_*` not populated yet

If the user somehow triggers `CaptureStarted` without
`StartCast` (impossible today, but defensive), the
`.unwrap_or(1280)` falls back to 720p30 defaults. Don't panic —
black-frame fallback is better than a crash.

### P4 — `tokio::time::sleep` in a non-tokio context

The cast loop runs under Slint's event loop. Make sure
`tokio::spawn` is invoked from inside a `tokio::Runtime::block_on`
context or you'll get `there is no reactor running`. The existing
code already runs cast handlers inside a `tokio::runtime::Handle::
current()` context — see how `tx_sink` constructors did it.

### P5 — `error!` macro and `?err`

The `error!(?err, "…")` syntax is `tracing`'s structured logging.
If `lib.rs` uses `log::error!` instead of `tracing::error!`, switch
to `error!("Failed to build unified cast graph: {err:?}")`.

### P6 — Returning `Ok(ShouldQuit::No)` from inside the loop

The `for cmd in commands { … }` block returns from the whole match
arm on the first error. Make sure your control flow returns
`Ok(ShouldQuit::No)` rather than just `()` — the surrounding
event-loop expects `Result<ShouldQuit, _>`.

### P7 — `Command::GetInfo { id: Some(_) }` vs `id: None`

`Some(CAST_DESTINATION_ID.into())` is preferred — the response
returns a `GetInfoResponse` whose `nodes` map contains only that
one key. With `id: None`, the response includes every node, which
is also fine but wastes serde work for a hot poll. Use `Some(...)`.

### P8 — Spurious `bound_port_v4 = Some(0)`

`WhepServerSignaller::server-port = 0` is the request-os-port mode.
The slot will eventually populate with the OS-picked port (40000+
range typically). If you see `bound_port_v4 = Some(0)` in the poll
output, that's a bug in the slot wiring — re-check PHASE-5 STEP-4
and STEP-6 for the value-passing logic.

---

## 5. Next step

Once this lands, [Step 3](./MVP-PHASE-6-STEP-3-signaller-started-helper.md)
adds the `mcore::transmission::build_whep_play_msg(addr, port)`
helper that the existing `Event::SignallerStarted` handler will use
instead of `self.tx_sink.as_ref().unwrap().get_play_msg(...)`.

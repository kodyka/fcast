# MVP-PHASE-6 — Step 7: preserve the `set_capture_active(false)` calls

> Part 7 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 6 — `FRAME_PAIR` unchanged](./MVP-PHASE-6-STEP-6-frame-pair-unchanged.md).
>
> **Doc-only.** This is a preservation step — no source tree files
> are modified.

---

## 0. Goal of this step

Confirm — and document — that the existing `set_capture_active(false)`
calls in `Event::CaptureStopped` (line 850), `Event::CaptureCancelled`
(line 854), and the transitive ones triggered by
`MainActivity.stopCapture` JNI call inside
[Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md)'s `stop_cast`
**must** be preserved.

They tell the JNI-side `EncoderCallback` to stop pushing frames,
which is what unblocks the `FRAME_PAIR` consumer's `cvar.wait` loop
inside `ScreenCaptureNode::wire_need_data(...)`. Without these
calls, the consumer can deadlock on a `Condvar::wait` that never
returns because no frame ever lands and no EOS signal is emitted.

This is a **preservation step** with no new code — just a
guarantee that the existing calls don't get accidentally removed
during the Step 2 / Step 4 edits.

---

## 1. Pre-flight

### 1.1 Live state — the existing `set_capture_active(false)` call sites

| Site | Location | Triggered by |
|---|---|---|
| `Event::CaptureStopped` body | `senders/android/src/lib.rs:850` | JNI `nativeCaptureStopped` (Java→Rust) |
| `Event::CaptureCancelled` body | `lib.rs:854` | User declined MediaProjection consent (or session ended unexpectedly) |
| `MainActivity.stopCapture()` JNI → JNI sets `CAPTURE_ACTIVE = false` | `MainActivity.java:801` + `nativeCaptureStopped` callback | `stop_cast` (Step 4) |

### 1.2 The contract: producer reads `CAPTURE_ACTIVE`

The JNI-side `EncoderCallback.onOutputBufferAvailable` calls
`nativeProcessFrame(bytes, w, h)` for every encoded frame. That
function checks `CAPTURE_ACTIVE` before writing to `FRAME_PAIR`:

```rust
// senders/android/src/lib.rs (~1900-1970, approximate body)
extern "C" fn nativeProcessFrame(/* … */) {
    if !CAPTURE_ACTIVE.load(Ordering::SeqCst) {
        return; // drop the frame
    }
    let frame = VideoFrame { /* … */ };
    let mut pair = FRAME_PAIR.0.lock().unwrap();
    *pair = Some(frame);
    FRAME_PAIR.1.notify_all();
}
```

When `set_capture_active(false)` lands, every subsequent
`nativeProcessFrame` is a no-op. The consumer's `Condvar::wait`
inside `ScreenCaptureNode::wire_need_data` times out (or returns
on the next state-machine tick), and the pipeline gracefully
moves to `Stopped`.

### 1.3 What happens if `set_capture_active(false)` is omitted

Symptom: cast stops in the UI (`AppState::Casting → Connected`),
but the JNI side keeps pushing frames. `FRAME_PAIR` keeps getting
overwritten. The consumer's `wire_need_data` keeps pushing buffers
into a stopped pipeline. GStreamer warning logs ensue. Battery
drain on the device.

The migration runtime's `Remove cast-source-1` (Step 4) does tell
the consumer to tear down, but the producer keeps running. **Both
sides must be told to stop.**

---

## 2. The change

**There is no source change in this step.** Confirm — but do **not
modify** — these three sites in `lib.rs`:

1. `lib.rs:~850` — `Event::CaptureStopped` body. Keep
   `set_capture_active(false)` exactly as-is.
2. `lib.rs:~854` — `Event::CaptureCancelled` body. Same.
3. `lib.rs::stop_cast` — the JNI call to
   `MainActivity.stopCapture()` (which transitively sets
   `CAPTURE_ACTIVE = false` via `nativeCaptureStopped`). Keep.

### 2.1 (Optional) Add a defensive `set_capture_active(false)` at the top of `stop_cast`

The current code relies on `MainActivity.stopCapture()` to
transitively trigger `nativeCaptureStopped` which sets
`CAPTURE_ACTIVE = false`. That's a JNI round-trip with a few
milliseconds of latency. To make `stop_cast` immediately stop
producer activity, you can add a direct call at the top of
`stop_cast` (before the `MainActivity.stopCapture` JNI call):

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    // NEW (defensive) — stop producer-side frame pushing
    // immediately, before the JNI round-trip.
    #[cfg(target_os = "android")]
    set_capture_active(false);

    let android_app = self.android_app.clone();
    self.ui_weak.upgrade_in_event_loop(move |_| {
        call_java_method_no_args(&android_app, JavaMethod::StopCapture);
    })?;

    // …rest of stop_cast (unchanged after Step 4)…
}
```

This is **optional defence in depth**. The cost is one extra
atomic write; the gain is a faster producer stop.

---

## 3. Verification

### 3.1 Grep — confirm preservation

```bash
grep -nE 'set_capture_active\(\s*false\s*\)' senders/android/src/lib.rs
# → expect ≥ 2 matches:
#   - Event::CaptureStopped body
#   - Event::CaptureCancelled body
#   (Plus optionally a third match if you took §2.1's defensive option.)
```

If the count drops to zero, you accidentally deleted the calls
during the Step 2 / Step 4 edits. Restore them.

### 3.2 On-device smoke

```bash
# After tapping "Stop cast":
adb logcat | grep -E '(set_capture_active|CAPTURE_ACTIVE|EncoderCallback)'
# Expect (within a few hundred milliseconds of tapping Stop):
#   set_capture_active(false)
#   CAPTURE_ACTIVE: false
#   EncoderCallback.onOutputBufferAvailable: dropping (capture inactive)
```

If you see the encoder still firing for >1s after tapping Stop,
the producer never received the signal. Check that
`MainActivity.stopCapture()` actually called
`nativeCaptureStopped` — possibly an unrelated Java-side bug.

---

## 4. Pitfalls specific to this step

### P1 — "Cleaning up" by removing the calls

While editing the same handler block in Step 2 (where
`Event::CaptureStarted` does `set_capture_active(true)`), the
`set_capture_active(false)` calls live in adjacent handlers
(`CaptureStopped`, `CaptureCancelled`). It's tempting to "tidy"
them out, thinking the migration runtime handles teardown. **It
doesn't.** The runtime tears down the *consumer*; the producer
needs its own signal.

### P2 — Inverting the producer/consumer relationship

Tempting (encapsulation) to have `ScreenCaptureNode::stop()` set
`CAPTURE_ACTIVE = false` itself. Don't — the node belongs to the
*migration runtime*, which is logically separate from the
JNI/MediaProjection producer. They're coupled through a *global
flag*, not through ownership. The current architecture is
correct; preserve it.

### P3 — Race between `set_capture_active(false)` and `Remove cast-source-1`

If the runtime processes `Remove cast-source-1` before the
producer sees `CAPTURE_ACTIVE = false`, there's a tiny window
where the producer writes a frame to `FRAME_PAIR` and nobody is
reading. That's *fine* — the next frame is overwritten by the
producer (since `FRAME_PAIR` only holds one slot), and once
`CAPTURE_ACTIVE = false` lands, no new frames are written. No
deadlock.

### P4 — `Event::CaptureCancelled` is also fired on permission denial

If the user denies MediaProjection consent,
`Event::CaptureCancelled` fires *without* a corresponding
`Event::CaptureStarted`. The `set_capture_active(false)` call is
still correct (and idempotent).

### P5 — `set_capture_active` argument confusion

`set_capture_active(true)` lives in `Event::CaptureStarted`
([Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)).
`set_capture_active(false)` lives in `Event::CaptureStopped` /
`Event::CaptureCancelled`. Don't transpose them or remove either —
both directions of the toggle are necessary.

---

## 5. Next step

Once this preservation is acknowledged, [Step 8](./MVP-PHASE-6-STEP-8-mod-migration-exports.md)
adds convenience re-exports to `senders/android/src/migration/mod.rs`
so the new `lib.rs` call sites can use shorter paths.

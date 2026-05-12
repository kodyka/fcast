# MVP-PHASE-6 — Step 6: leave `FRAME_PAIR` / `nativeProcessFrame` untouched

> Part 6 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 5 — gate `tx_sink`](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md).
>
> **Doc-only.** This is a documentation-only step — no source tree
> files are modified.

---

## 0. Goal of this step

Confirm — and document — that the JNI-side frame producer is
**unchanged** by PHASE-6. The Kotlin/Java code that captures via
`MediaProjection` and the Rust `nativeProcessFrame` JNI bridge that
writes into `FRAME_PAIR` are out of scope.

The **only** thing PHASE-6 changes is *who reads* from `FRAME_PAIR`:

| | Before | After |
|---|---|---|
| Producer | `MainActivity` → `MediaProjection` → `EncoderCallback` → `nativeProcessFrame` → `FRAME_PAIR` | unchanged |
| Consumer | `lib.rs::Event::CaptureStarted` builds `appsrc` + `WhepSink::new` | `ScreenCaptureNode::wire_need_data(...)` reads `FRAME_PAIR` from inside the migration runtime |

This is a **checkpoint step** with no code changes. It exists to
prevent accidentally "tidying up" the producer side while doing the
consumer-side rewrite.

---

## 1. Pre-flight

### 1.1 Live state — producer chain

| Component | Location | Touched by PHASE-6? |
|---|---|---|
| `MainActivity.startScreenCapture(w, h, fps)` (Kotlin entry) | `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:720` | **No** |
| `MainActivity.stopCapture()` (Kotlin entry) | `MainActivity.java:801` | **No** |
| `MediaProjection.createVirtualDisplay(...)` | `MainActivity.java:~780` | **No** |
| `EncoderCallback.onOutputBufferAvailable(...)` | `MainActivity.java:~840` | **No** |
| JNI bridge `nativeProcessFrame(bytes, w, h)` | `senders/android/src/lib.rs:1900-1970` | **No** |
| `process_frame(VideoFrame)` | `lib.rs:1900-1970` (in the JNI extern block) | **No** |
| `FRAME_PAIR: Mutex<Option<VideoFrame>> + Condvar` | `lib.rs:65-77` | **No** (still the same `lazy_static!`) |
| `CAPTURE_ACTIVE: AtomicBool` | `lib.rs:76` | **No** |
| `VideoFrame` struct + `bytes()` / `byte_len()` | `lib.rs:46-63` | **No** |

### 1.2 Live state — consumer chain (the part that changes)

| Component | Location | Touched by PHASE-6? |
|---|---|---|
| The legacy `Event::CaptureStarted` `appsrc` + need-data closure | `lib.rs:875-961` | **Yes** — [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) deletes the closure |
| `ScreenCaptureNode::wire_need_data(...)` (the new consumer) | PHASE-4 STEP-2: `senders/android/src/migration/nodes/screen_capture.rs:wire_need_data(...)` | **No** (PHASE-4 already shipped) |
| The legacy `WhepSink::new` consumer wrapping | `sdk/mirroring_core/src/transmission.rs:475-528` | **Indirectly** — no longer called on Android after [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) + [Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md) |

### 1.3 Why this checkpoint matters

It is **very tempting**, while editing `lib.rs` to delete the
`appsrc` + need-data closure (Step 2), to also "tidy up" the
producer-side `process_frame` function or the
`MainActivity.startScreenCapture(...)` flow. **Don't.**

Reasons:

- The producer side is a stable cross-language contract. Changes
  there ripple into Java, Kotlin, the AAR build, and the on-device
  permissions flow (MediaProjection requires user consent — risky
  surface).
- The consumer side has changed *which* GStreamer pipeline reads
  from `FRAME_PAIR`, but the data contract (I420 YUV bytes, width,
  height, frame ordering) is unchanged.
- Touching the producer in the same PR makes blame harder if
  something regresses on-device.

---

## 2. The change

**There is no source change in this step.** This step is purely
documentation in the parent doc and serves as a check before
proceeding to [Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md).

If you find yourself editing any of the files listed in §1.1, stop
and confirm with the reviewer whether the change should be split
into a separate PR.

### 2.1 (Optional) Add a comment near `FRAME_PAIR`

If you want to make the contract explicit at the producer site,
add a brief comment near the `lazy_static!` block at
`lib.rs:65-77`:

```rust
// FRAME_PAIR is the cross-language video-frame hand-off. The
// producer is the JNI `nativeProcessFrame` callback driven by
// `MediaProjection` (Java side). The consumer is the migration
// runtime's `ScreenCaptureNode::wire_need_data`.
//
// Don't change the producer contract without coordinating
// MediaProjection-side changes — touching this from the Rust side
// alone can stall encoder threads.
lazy_static! {
    pub static ref FRAME_PAIR: (Mutex<Option<VideoFrame>>, Condvar) =
        (Mutex::new(None), Condvar::new());
    pub static ref CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);
}
```

This comment is **strictly optional** — it adds documentation
weight to a file that's already heavy, but helps future readers.

---

## 3. Verification

### 3.1 No-edit confirmation

```bash
git diff --stat senders/android/src/lib.rs | head -1
# → expect the diff size to be Step 2 + 4 + 5 lines only, NOT touching
#   the FRAME_PAIR producer block (lib.rs:65-77 stays unchanged).

git diff senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java
# → expect: (empty)
```

### 3.2 Grep — confirm the consumer site moved

```bash
grep -nE '(FRAME_PAIR|process_frame)' senders/android/src/lib.rs
# → matches at lib.rs:65-77 (declaration) and lib.rs:1900-1970
#   (process_frame fn). NO matches in the body of Event::CaptureStarted
#   (which is now graph-commands-only).
```

### 3.3 Smoke (on-device)

Run the parent doc's full end-to-end smoke (§3 of the parent).
Visual check: the receiver shows the screen contents within ~1s.
This proves the producer→consumer hand-off still works through the
new path.

---

## 4. Pitfalls specific to this step

### P1 — Tempting refactor: "make FRAME_PAIR generic"

Tempting (avoid clones, add typed buffer pooling, etc.). **Don't
in this PR.** Producer-side changes belong to a separate phase
(provisionally MVP-PHASE-9 if/when needed).

### P2 — Tempting refactor: "lift FRAME_PAIR into ScreenCaptureNode"

Tempting (better encapsulation — the screen-capture node owns its
frame pipe). **Don't in this PR.** It would require splitting the
JNI `nativeProcessFrame` body to dispatch to *one specific node
instance*, which adds a JNI handle and breaks the
single-cast-at-a-time invariant. Defer.

### P3 — `gst_app::AppSrcCallbacks` differences

The legacy `Event::CaptureStarted` need-data closure ran in the
GStreamer-stream thread context. PHASE-4's
`ScreenCaptureNode::wire_need_data` runs in the same context. The
closure body is functionally equivalent — read `FRAME_PAIR`, push
buffer, return. Don't try to compare byte-for-byte; the *interface*
is what matters.

### P4 — `CAPTURE_ACTIVE` still gates the producer

Even though the consumer has moved, the `CAPTURE_ACTIVE` flag is
still what tells the JNI side to stop pushing. Don't replace it
with `nodes.contains_key(CAST_SOURCE_ID)` — that's the consumer's
view, not the producer's, and they should stay decoupled. See
[Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md).

---

## 5. Next step

Once this checkpoint is acknowledged, [Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md)
explicitly preserves the `set_capture_active(false)` calls in
`Event::CaptureStopped` / `Event::CaptureCancelled` — they tell the
JNI-side EncoderCallback to stop pushing frames.

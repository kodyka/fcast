# MVP-PHASE-4 — Step 2: define `ScreenCaptureNode`

> Part 2 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
> Previous: [Step 1 — protocol extension](./MVP-PHASE-4-STEP-1-protocol-extension.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a new `senders/android/src/migration/nodes/screen_capture.rs`
file containing:

- A `ScreenCaptureNode` struct that mirrors `SourceNode`'s scheduling
  / refresh / consumer-tracking surface.
- A `LiveScreenCapturePipeline` companion struct that owns the
  GStreamer pipeline once it's running (appsrc → videoconvert →
  appsink).
- A `build_live_pipeline()` method that constructs the pipeline.
- A `wire_need_data()` helper that bridges the existing
  `FRAME_PAIR` global into the new node's `appsrc`.

After this step, the node compiles in isolation but **isn't reachable
from the runtime** — that wire-up happens in
[Step 3](./MVP-PHASE-4-STEP-3-module-registration.md) (mod
registration) and [Step 4](./MVP-PHASE-4-STEP-4-node-record.md)
(extending `NodeRecord`).

This is the **largest step in PHASE-4** (~250 Rust lines, one new
file). It maps roughly 1:1 onto `nodes/source.rs` minus the audio
branches.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `FRAME_PAIR` (the producer that `nativeProcessFrame` writes into) | `senders/android/src/lib.rs:65-77` (`Mutex<Option<VideoFrame>>` + `Condvar`) |
| `CAPTURE_ACTIVE` flag | `senders/android/src/lib.rs:76` |
| `VideoFrame` struct + `bytes()` / `byte_len()` accessors | `senders/android/src/lib.rs:46-63` |
| Existing `need-data` consumer (the reference template) | `senders/android/src/lib.rs:1456-1620` |
| `SourceNode` (structural model for state machine + scheduling) | `senders/android/src/migration/nodes/source.rs:1-300` |
| `nodes::source::PREROLL_LEAD_TIME_SECONDS` | `nodes/source.rs:7` (`= 10`) |
| Sibling node modules | `senders/android/src/migration/nodes/destination.rs`, `mixer.rs`, `video_generator.rs` |

### 1.2 Why one file per node type

The existing `nodes/` directory already has one file per node kind
(`source.rs`, `destination.rs`, `mixer.rs`, `video_generator.rs`).
Following the same convention keeps the diff scoped and lets the
compiler give clean error messages with a single `mod
screen_capture;` declaration.

### 1.3 Why we recycle `NodeInfo::Source` instead of adding a variant

`as_info()` returns `NodeInfo::Source(SourceInfo {
uri: format!("screen://{}x{}@{}fps", …), … })`. This:

- Means the smoke-test in PHASE-3 sees the same `kind: "source"`
  shape it already understands.
- Saves a `NodeInfo` enum-variant churn (and the cascade of
  match-arm fixes in `protocol.rs`).
- Adding a `NodeInfo::ScreenCapture` variant later is a strictly
  additive change.

If you'd rather model it as its own kind, see §2.3 in
[Step 1](./MVP-PHASE-4-STEP-1-protocol-extension.md#23-optional-add-a-screencaptureinfo-variant-to-nodeinfo)
for the additive shape.

---

## 2. The change

**New file:** `senders/android/src/migration/nodes/screen_capture.rs`

```rust
// senders/android/src/migration/nodes/screen_capture.rs

use crate::migration::protocol::{NodeInfo, SourceInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use std::collections::BTreeSet;

const PREROLL_LEAD_TIME_SECONDS: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenCapturePipelineStage {
    Idle,
    Prerolling,
    Playing,
}

#[derive(Debug, Clone)]
pub struct LiveScreenCapturePipeline {
    pub pipeline: gst::Pipeline,
    pub appsrc: AppSrc,
    pub video_appsink: AppSink,
}

#[derive(Debug, Clone)]
pub struct ScreenCaptureNode {
    pub id: String,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_consumer_slot_ids: BTreeSet<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub stage: ScreenCapturePipelineStage,
    pub live_pipeline: Option<LiveScreenCapturePipeline>,
    pub last_error: Option<String>,
}

impl ScreenCaptureNode {
    pub fn new(id: String, width: u32, height: u32, fps: u32) -> Self {
        Self {
            id,
            width,
            height,
            fps,
            video_consumer_slot_ids: BTreeSet::new(),
            cue_time: None,
            end_time: None,
            state: State::Initial,
            stage: ScreenCapturePipelineStage::Idle,
            live_pipeline: None,
            last_error: None,
        }
    }

    pub fn as_info(&self) -> NodeInfo {
        NodeInfo::Source(SourceInfo {
            uri: format!("screen://{}x{}@{}fps", self.width, self.height, self.fps),
            video_consumer_slot_ids: Some(self.video_consumer_slot_ids.iter().cloned().collect()),
            audio_consumer_slot_ids: None,
            cue_time: self.cue_time,
            end_time: self.end_time,
            state: self.state,
        })
    }

    pub fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        // Same semantics as SourceNode::schedule.
        self.cue_time = cue_time;
        self.end_time = end_time;
        Ok(())
    }

    pub fn add_consumer_link(&mut self, link_id: &str, _audio: bool, video: bool) {
        if video {
            self.video_consumer_slot_ids.insert(link_id.to_string());
        }
    }

    pub fn remove_consumer_link(&mut self, link_id: &str) {
        self.video_consumer_slot_ids.remove(link_id);
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.advance_schedule(Utc::now());
        self.sync_live_pipeline()
    }

    pub fn live_video_appsink(&self) -> Option<AppSink> {
        self.live_pipeline.as_ref().map(|p| p.video_appsink.clone())
    }

    pub fn stop(&mut self) {
        self.state = State::Stopped;
        self.teardown_pipeline();
    }

    pub fn mark_error(&mut self, message: String) {
        self.last_error = Some(message);
        self.stop();
    }

    // ────── private ──────

    fn schedule_transition_due(&self, now: DateTime<Utc>) -> Option<State> {
        // 1:1 mirror of SourceNode::schedule_transition_due
        // (nodes/source.rs:433-463). Each call returns the *next* state
        // the node should occupy given `now`; `advance_schedule` loops
        // until no further transitions are due.
        //
        // States and triggers:
        //   Initial   → Starting   (at cue_time - PREROLL_LEAD_TIME_SECONDS)
        //   Initial   → Started    (immediately, when cue_time is None)
        //   Starting  → Started    (at cue_time, or immediately when None)
        //   Started   → Stopping   (at end_time)
        //   Stopping  → Stopped    (next refresh)
        match self.state {
            State::Initial => match self.cue_time {
                Some(cue) => {
                    let preroll_at = cue - Duration::seconds(PREROLL_LEAD_TIME_SECONDS);
                    if now >= preroll_at {
                        Some(State::Starting)
                    } else {
                        None
                    }
                }
                None => Some(State::Started),
            },
            State::Starting => {
                if self.cue_time.is_none_or(|cue| now >= cue) {
                    Some(State::Started)
                } else {
                    None
                }
            }
            State::Started => {
                if self.end_time.is_some_and(|end| now >= end) {
                    Some(State::Stopping)
                } else {
                    None
                }
            }
            State::Stopping => Some(State::Stopped),
            State::Stopped => None,
        }
    }

    fn apply_state_to_stage(&mut self) {
        self.stage = match self.state {
            State::Initial | State::Stopping | State::Stopped => {
                ScreenCapturePipelineStage::Idle
            }
            State::Starting => ScreenCapturePipelineStage::Prerolling,
            State::Started => ScreenCapturePipelineStage::Playing,
        };
    }

    fn advance_schedule(&mut self, now: DateTime<Utc>) -> bool {
        // Loop until `schedule_transition_due` reports `None` so that a
        // single refresh can collapse a long-overdue chain like
        // Stopping → Stopped, or (when both cue and end are in the past)
        // Initial → Starting → Started → Stopping → Stopped without
        // waiting for multiple refresh ticks.
        let mut changed = false;
        while let Some(next_state) = self.schedule_transition_due(now) {
            if next_state == self.state {
                break;
            }
            self.state = next_state;
            changed = true;
        }

        let old_stage = self.stage;
        self.apply_state_to_stage();
        changed || old_stage != self.stage
    }

    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        // `advance_schedule` already wrote `self.stage` via
        // `apply_state_to_stage`; this method just drives GStreamer to
        // match. Reading `self.stage` (instead of recomputing from
        // `self.state`) avoids duplicating the state→stage mapping.
        match self.stage {
            ScreenCapturePipelineStage::Idle => {
                self.teardown_pipeline();
                Ok(())
            }
            ScreenCapturePipelineStage::Prerolling
            | ScreenCapturePipelineStage::Playing => {
                self.build_live_pipeline()?;
                let gst_state = if self.stage == ScreenCapturePipelineStage::Prerolling {
                    gst::State::Paused
                } else {
                    gst::State::Playing
                };
                if let Some(p) = &self.live_pipeline {
                    p.pipeline
                        .set_state(gst_state)
                        .map_err(|e| format!("set_state({gst_state:?}) failed: {e}"))?;
                }
                Ok(())
            }
        }
    }

    fn build_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_some() {
            return Ok(());
        }

        let pipeline = gst::Pipeline::new();

        let appsrc = gst_app::AppSrc::builder()
            .name(&format!("screen-capture-appsrc-{}", self.id))
            .format(gst::Format::Time)
            .is_live(true)
            .do_timestamp(true)
            .stream_type(gst_app::AppStreamType::Stream)
            .caps(
                &gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("width", self.width as i32)
                    .field("height", self.height as i32)
                    .field("framerate", gst::Fraction::new(self.fps as i32, 1))
                    .build(),
            )
            .build();

        let videoconvert = gst::ElementFactory::make("videoconvert")
            .build()
            .map_err(|e| format!("videoconvert: {e}"))?;

        let appsink = gst_app::AppSink::builder()
            .name(&format!("screen-capture-appsink-{}", self.id))
            .sync(false)
            .build();

        pipeline
            .add_many([appsrc.upcast_ref(), &videoconvert, appsink.upcast_ref()])
            .map_err(|e| format!("pipeline.add_many: {e}"))?;
        gst::Element::link_many([appsrc.upcast_ref(), &videoconvert, appsink.upcast_ref()])
            .map_err(|e| format!("link_many: {e}"))?;

        // Wire the FRAME_PAIR consumer onto appsrc.
        Self::wire_need_data(&appsrc, self.width, self.height);

        self.live_pipeline = Some(LiveScreenCapturePipeline {
            pipeline,
            appsrc,
            video_appsink: appsink,
        });
        Ok(())
    }

    fn wire_need_data(appsrc: &AppSrc, _w: u32, _h: u32) {
        // Pull from `crate::FRAME_PAIR` (lib.rs:65-77).
        //
        // The existing cast loop's need-data handler at
        // senders/android/src/lib.rs:1456-1620 is the reference. The
        // key contract:
        //
        // 1. Take (not clone) FRAME_PAIR.0.lock()'s contents using
        //    std::mem::take(...). Releasing the lock between frames is
        //    critical because nativeProcessFrame writes into the same
        //    Mutex from the JNI thread.
        // 2. Build gst::Buffer of width * height * 3 / 2 bytes (I420 YUV).
        // 3. push_buffer(buf).
        // 4. Honor CAPTURE_ACTIVE — if false, push EOS and stop.
        //
        // See lib.rs:1485+ for the Condvar-timeout pattern that prevents
        // deadlock on capture stop.
        let appsrc_weak = appsrc.downgrade();
        appsrc.set_callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _size| {
                    let _ = appsrc_weak.upgrade(); // illustrative — wire properly

                    let frame_opt = {
                        let mut pair = crate::FRAME_PAIR.0.lock().unwrap();
                        std::mem::take(&mut *pair) // take, don't clone
                    };

                    let frame = match frame_opt {
                        Some(f) => f,
                        None => return, // no frame available; let need-data retrigger
                    };

                    let mut buf = gst::Buffer::with_size(frame.byte_len()).unwrap();
                    {
                        let buf_mut = buf.get_mut().unwrap();
                        let mut mapped = buf_mut.map_writable().unwrap();
                        mapped.copy_from_slice(frame.bytes());
                    }
                    let _ = appsrc.push_buffer(buf);
                })
                .build(),
        );
    }

    fn teardown_pipeline(&mut self) {
        if let Some(p) = self.live_pipeline.take() {
            let _ = p.pipeline.set_state(gst::State::Null);
        }
    }
}
```

This is **illustrative**, not committed. The exact contract of
`FRAME_PAIR` consumption (block vs poll, drop-old vs queue, EOS on
`CAPTURE_ACTIVE = false`) must match what `lib.rs:1456-1620` already
does, since that's the live cast loop's behaviour. The
`Condvar`-timeout pattern in `lib.rs:1485+` is the canonical
reference.

### 2.1 Why `appsrc.downgrade()` in the closure capture

`gst_app::AppSrcCallbacks::builder().need_data(F)` requires `F: FnMut +
Send + Sync + 'static`. Capturing the strong `AppSrc` directly creates
a reference cycle (appsrc → callback → appsrc). The cycle prevents the
pipeline from being dropped on `set_state(Null)`. Always capture a
`WeakRef<AppSrc>` and upgrade inside the closure — same pattern as the
existing destination/source nodes.

### 2.2 Why we don't store `AppSrcCallbacks` on the node

`AppSrcCallbacks` is installed via `appsrc.set_callbacks(...)` and
takes ownership; GStreamer holds it internally. The closure is
"forgotten" by Rust once installed, and `gst::Pipeline::set_state(Null)`
is what eventually drops it. This is why `ScreenCaptureNode` doesn't
need any `Box<dyn FnMut>` fields — the only things stored are
`gst::Pipeline`, `AppSrc`, and `AppSink`, all of which derive `Debug`
cleanly.

---

## 3. Verification

### 3.1 Compile check (after Step 3 registers the module)

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Strictly speaking this step doesn't compile *on its own* — the module
isn't registered in `nodes/mod.rs` until Step 3, so the compiler will
warn about an unused file. The combined Step 1 + 2 + 3 + 4 + 5 squash
is what gets a clean build.

Most likely failures:

- `cannot find value FRAME_PAIR in crate::` — check that the global
  is `pub` (or `pub(crate)`) in `lib.rs:65-77`. It's already
  `pub(crate)` today, so this should just work.
- `cannot move out of borrowed content` in `wire_need_data` — wrap
  any captured state in `Arc<Mutex<...>>`. The shown `std::mem::take`
  pattern is the right shape.
- `the trait Sync is not implemented for jni::JNIEnv` — if you
  accidentally captured a JNI handle. **Never** capture JNI types in
  the `need-data` closure; only read from globals.

### 3.2 Grep

```bash
grep -n 'pub struct ScreenCaptureNode' senders/android/src/migration/nodes/screen_capture.rs
# → 1 match
grep -n 'fn build_live_pipeline\|fn wire_need_data\|fn sync_live_pipeline' \
    senders/android/src/migration/nodes/screen_capture.rs
# → 3 matches
grep -n 'FRAME_PAIR' senders/android/src/migration/nodes/screen_capture.rs
# → 1 match (the lock in wire_need_data)
```

---

## 4. Pitfalls specific to this step

### P1 — `appsrc` caps must match what the YUV bytes actually are

`FRAME_PAIR` contains I420 planar YUV (per `nativeProcessFrame` →
`process_frame` at `lib.rs:1900-1970`). If you specify `NV12` or
`RGBA` in the caps, `videoconvert` will error at link time. Stick
with `I420`.

### P2 — Cloning `VideoFrame` is expensive

Don't `.clone()` the frame on every `need-data` callback —
`VideoFrame` holds a heap buffer of `width * height * 3 / 2` bytes
(roughly 1.3 MB at 1280x720). The existing cast loop uses
`std::mem::take(&mut *pair)`; mirror that to avoid allocation.

### P3 — Drop-old vs queue-up

The existing cast loop drops old frames in favour of new ones
(`Mutex<Option<VideoFrame>>` — only one slot). If your `need-data`
blocks on the `Condvar` forever, you'll deadlock when capture stops.
The `Condvar` timeout pattern in `lib.rs:1485+` (`.wait_timeout(...)`)
handles this; the simpler form shown above just returns when there's
no frame, letting GStreamer's `need-data` retrigger naturally.

### P4 — `gst_app::AppSrcCallbacks::builder()` requires `Send`

The closure captured into `need_data(...)` must be `Send + Sync +
'static`. Any JNI handle (`JNIEnv`, `JObject`) is **not** `Send`.
Don't capture anything Java-side in the callback; just read from the
global `FRAME_PAIR` which is a static `Mutex<Option<VideoFrame>>` and
`Send` by construction.

### P5 — Auto-derive `Debug` on `ScreenCaptureNode`

`gst::Pipeline`, `AppSrc`, and `AppSink` all implement `Debug`.
`AppSrcCallbacks` is **not** stored in the node (it's installed and
forgotten). All other fields derive `Debug` cleanly. If the compiler
complains about a missing `Debug` impl, check that you didn't
accidentally capture an `Arc<dyn FnMut>` somewhere.

### P6 — Empty `id` produces nonsense `appsrc` names

`format!("screen-capture-appsrc-{}", self.id)` with an empty `id`
produces `"screen-capture-appsrc-"` — GStreamer accepts that, but it
breaks debugging via `GST_DEBUG=*:5` since the element name no
longer identifies the node. Validate `!id.is_empty()` in the dispatch
arm ([Step 5](./MVP-PHASE-4-STEP-5-dispatch-arm.md)).

### P7 — `cue_time = None` must immediately transition to `Started`

`schedule_transition_due` returns `Some(State::Started)` directly
from the `State::Initial` arm when `cue_time` is `None`. Forgetting
this case (e.g. matching only `Some(cue)` and falling through)
means an unscheduled `createscreencapturesource` stays in `Initial`
forever and never spins up the pipeline — `Start` commands won't
help either because `Start` only sets `cue_time` / `end_time`, not
the state directly. The `match self.cue_time { Some(cue) => …,
None => Some(State::Started) }` shape in the snippet above
encodes this contract.

---

## 5. Next step

Once this lands, [Step 3](./MVP-PHASE-4-STEP-3-module-registration.md)
registers the new `screen_capture` module in
`senders/android/src/migration/nodes/mod.rs` so it can be imported
from `node_manager.rs`.

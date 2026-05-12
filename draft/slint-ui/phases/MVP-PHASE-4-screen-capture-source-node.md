# MVP-PHASE-4 — `ScreenCapture` source node (Tier 1.1)
 
> **First architectural unification step.** Today, the migration runtime
> only knows how to ingest from URIs (`fallbacksrc` / `uridecodebin`).
> This phase teaches it how to ingest **Android MediaProjection frames**
> from the live `FRAME_PAIR` static — turning the screen-mirror cast
> path into a regular node in the graph.
 
---
 
## 0. Goal
 
Add a `ScreenCapture` source node to the migration runtime so that the
existing JNI-driven frame producer (`nativeProcessFrame` →
`FRAME_PAIR`) can be wrapped as a graph node and connected to any
downstream sink (mixer or destination).
 
After this phase, you can issue:
 
```json
{"createscreencapturesource": {"id": "cap-1", "width": 1280, "height": 720, "fps": 30}}
```
 
…and the runtime will spin up a GStreamer pipeline that reads YUV
frames from `FRAME_PAIR` and exposes a video `appsink` for downstream
nodes (via `StreamBridge`).
 
This phase **does not** wire the cast loop to use the new node yet —
that happens in MVP-PHASE-6 after MVP-PHASE-5 adds the `Whep`
destination.
 
---
 
## 1. Pre-flight
 
### 1.1 What already exists (do not re-implement)
 
| Component | Location |
|---|---|
| Global frame channel | `senders/android/src/lib.rs:65-77` (`FRAME_PAIR: Mutex<Option<VideoFrame>>` + `FRAME_PAVAILABLE: Condvar`) |
| Frame consumer pattern | `senders/android/src/lib.rs:1456-1620` (the existing cast loop's `appsrc` `need-data` callback) |
| `VideoFrame` struct | `senders/android/src/lib.rs:46-63` |
| JNI `nativeProcessFrame` writer | `senders/android/src/lib.rs:1900-1970` (writes into `FRAME_PAIR`) |
| `CAPTURE_ACTIVE` flag | `senders/android/src/lib.rs:76` (gates whether frames should be consumed) |
| `MainActivity.startScreenCapture(w,h,fps)` | `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:720` |
| `MainActivity.stopCapture()` | `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:801` |
 
### 1.2 What needs to change
 
| File | Edit |
|---|---|
| `senders/android/src/migration/protocol.rs` | New `Command::CreateScreenCaptureSource { id, width, height, fps }` variant. |
| `senders/android/src/migration/nodes/screen_capture.rs` | **New file.** `ScreenCaptureNode` struct + `build_live_pipeline`. |
| `senders/android/src/migration/nodes/mod.rs` | Add `pub mod screen_capture;` and re-export. |
| `senders/android/src/migration/node_manager.rs` | Extend `NodeRecord` with `ScreenCapture(ScreenCaptureNode)`; add dispatch arm + capability flags. |
 
Approximate scope: **~250–400 lines of Rust across 1 new + 3 edited files**.
 
### 1.3 Why not "just reuse `Command::CreateSource` with a magic URI"?
 
Tempting (`uri: "screen://"`) but bad: `SourceNode` is hard-wired to
`fallbacksrc`/`uridecodebin` and will fail on a non-GStreamer URI
scheme. A dedicated variant keeps the pipeline graph correct and lets
us drop the unused audio path.
 
---
 
## 2. Steps
 
### 2.1 Step 1 — extend the JSON protocol
 
**File:** `senders/android/src/migration/protocol.rs` (add to the
`Command` enum at lines 37-106):
 
```rust
// senders/android/src/migration/protocol.rs
 
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    CreateVideoGenerator { id: String },
    CreateSource { /* … */ },
    CreateDestination { /* … */ },
    CreateMixer { /* … */ },
 
    // NEW —
    CreateScreenCaptureSource {
        id: String,
        #[serde(default = "default_capture_width")]
        width: u32,
        #[serde(default = "default_capture_height")]
        height: u32,
        #[serde(default = "default_capture_fps")]
        fps: u32,
    },
 
    Connect { /* … */ },
    /* … */
}
 
fn default_capture_width() -> u32 { 1280 }
fn default_capture_height() -> u32 { 720 }
fn default_capture_fps() -> u32 { 30 }
```
 
`#[serde(rename_all = "lowercase")]` means the JSON tag becomes
`"createscreencapturesource"` (matching the existing camelCase-free
convention).
 
### 2.2 Step 2 — define the node
 
**New file:** `senders/android/src/migration/nodes/screen_capture.rs`:
 
```rust
// senders/android/src/migration/nodes/screen_capture.rs
 
use crate::migration::protocol::{NodeInfo, State};
use chrono::{DateTime, Duration, Utc};
use gst::prelude::*;
use gst_app::{AppSink, AppSrc};
use std::collections::BTreeSet;
 
const PREROLL_LEAD_TIME_SECONDS: i64 = 2;
 
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
        // Reuse the SourceInfo shape so HTTP /getinfo callers see a
        // familiar structure. The `uri` field is decorative — a real
        // implementation could add a ScreenCaptureInfo variant.
        NodeInfo::Source(crate::migration::protocol::SourceInfo {
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
        // Same semantics as SourceNode::schedule. See nodes/source.rs.
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
 
    fn advance_schedule(&mut self, now: DateTime<Utc>) {
        // Identical state machine to SourceNode (nodes/source.rs):
        //   Initial → Starting (at cue_time - PREROLL)
        //   Starting → Started (at cue_time)
        //   Started → Stopping (at end_time)
        //   Stopping → Stopped
        // …with the additional rule: if cue_time is None, immediately
        // transition to Started on first refresh.
        // (Body intentionally elided — copy the equivalent block from
        // `nodes/source.rs::advance_schedule` and remove audio-specific
        // branches.)
        if self.cue_time.is_none() && self.state == State::Initial {
            self.state = State::Started;
        }
        let _ = now;
    }
 
    fn sync_live_pipeline(&mut self) -> Result<(), String> {
        let desired_stage = match self.state {
            State::Initial | State::Stopping | State::Stopped => {
                ScreenCapturePipelineStage::Idle
            }
            State::Starting => ScreenCapturePipelineStage::Prerolling,
            State::Started => ScreenCapturePipelineStage::Playing,
        };
 
        match (self.stage, desired_stage) {
            (a, b) if a == b => Ok(()),
            (_, ScreenCapturePipelineStage::Idle) => {
                self.teardown_pipeline();
                self.stage = ScreenCapturePipelineStage::Idle;
                Ok(())
            }
            (_, target) => {
                self.build_live_pipeline()?;
                let gst_state = match target {
                    ScreenCapturePipelineStage::Prerolling => gst::State::Paused,
                    ScreenCapturePipelineStage::Playing => gst::State::Playing,
                    ScreenCapturePipelineStage::Idle => unreachable!(),
                };
                if let Some(p) = &self.live_pipeline {
                    p.pipeline
                        .set_state(gst_state)
                        .map_err(|e| format!("set_state({gst_state:?}) failed: {e}"))?;
                }
                self.stage = target;
                Ok(())
            }
        }
    }
 
    fn build_live_pipeline(&mut self) -> Result<(), String> {
        if self.live_pipeline.is_some() {
            return Ok(());
        }
 
        let pipeline = gst::Pipeline::new();
 
        // appsrc — fed by the FRAME_PAIR consumer thread (Step 3 wires
        // this).
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
        // 1. Block on FRAME_PAVAILABLE.wait(...) until FRAME_PAIR is
        //    Some(VideoFrame).
        // 2. Build gst::Buffer of width*height*3/2 bytes (I420 YUV).
        // 3. push_buffer(buf).
        // 4. Honor CAPTURE_ACTIVE — if false, push EOS and stop.
        appsrc.set_callbacks(
            gst_app::AppSrcCallbacks::builder()
                .need_data(move |appsrc, _size| {
                    // Pseudo-code — copy structure from lib.rs:1456+.
                    let frame = {
                        let pair = crate::FRAME_PAIR.0.lock().unwrap();
                        if let Some(f) = pair.as_ref() {
                            f.clone()
                        } else {
                            return;
                        }
                    };
 
                    let mut buf = gst::Buffer::with_size(frame.byte_len()).unwrap();
                    {
                        let buf_mut = buf.get_mut().unwrap();
                        let mut mapped = buf_mut.map_writable().unwrap();
                        mapped.copy_from_slice(&frame.bytes());
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
does, since that's the live cast loop's behaviour.
 
### 2.3 Step 3 — register the module
 
**File:** `senders/android/src/migration/nodes/mod.rs`:
 
```rust
// senders/android/src/migration/nodes/mod.rs
 
pub mod control;
pub mod destination;
pub mod mixer;
pub mod source;
pub mod video_generator;
 
// NEW —
pub mod screen_capture;
 
pub use control::*;
pub use destination::*;
pub use mixer::*;
pub use source::*;
pub use video_generator::*;
 
// NEW —
pub use screen_capture::*;
```
 
### 2.4 Step 4 — extend `NodeRecord`
 
**File:** `senders/android/src/migration/node_manager.rs` (around
lines 21-26):
 
```rust
// senders/android/src/migration/node_manager.rs
 
enum NodeRecord {
    Source(SourceNode),
    Destination(DestinationNode),
    Mixer(MixerNode),
    VideoGenerator(VideoGeneratorNode),
 
    // NEW —
    ScreenCapture(ScreenCaptureNode),
}
```
 
…then thread the new variant through **every** `match self` arm in
`impl NodeRecord` (lines 28-160):
 
```rust
impl NodeRecord {
    fn can_output_audio(&self) -> bool {
        match self {
            Self::Source(node) => node.audio_enabled,
            Self::Mixer(node) => node.audio_enabled,
            Self::VideoGenerator(node) => node.audio_enabled,
            Self::Destination(_) => false,
 
            // NEW —
            Self::ScreenCapture(_) => false,
        }
    }
 
    fn can_output_video(&self) -> bool {
        match self {
            // …existing arms…
            Self::ScreenCapture(_) => true,
        }
    }
 
    fn can_input_audio(&self) -> bool {
        match self {
            // …existing arms…
            Self::ScreenCapture(_) => false,
        }
    }
 
    fn can_input_video(&self) -> bool {
        match self {
            // …existing arms…
            Self::ScreenCapture(_) => false,
        }
    }
 
    fn to_info(&self) -> NodeInfo {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.as_info(),
        }
    }
 
    fn set_schedule(&mut self, cue: Option<DateTime<Utc>>, end: Option<DateTime<Utc>>)
        -> Result<(), String> {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.schedule(cue, end),
        }
    }
 
    fn stop(&mut self) {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.stop(),
        }
    }
 
    fn mark_error(&mut self, m: String) {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.mark_error(m),
        }
    }
 
    fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.add_consumer_link(link_id, audio, video),
        }
    }
 
    fn remove_consumer_link(&mut self, link_id: &str) {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.remove_consumer_link(link_id),
        }
    }
 
    fn refresh_runtime(&mut self) {
        let result = match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.refresh(),
        };
        if let Err(err) = result { self.mark_error(err); }
    }
 
    fn output_audio_appsink(&self) -> Option<AppSink> {
        match self {
            // …existing arms…
            Self::ScreenCapture(_) => None,
        }
    }
 
    fn output_video_appsink(&self) -> Option<AppSink> {
        match self {
            // …existing arms…
            Self::ScreenCapture(node) => node.live_video_appsink(),
        }
    }
}
```
 
### 2.5 Step 5 — wire the dispatch arm
 
**File:** `senders/android/src/migration/node_manager.rs` (around
the `dispatch` method at line 316):
 
```rust
pub fn dispatch(&mut self, command: Command) -> CommandResult {
    if !self.started { self.started = true; }
    self.refresh_nodes();
 
    let (result, should_sync) = match command {
        Command::CreateVideoGenerator { id } => (self.create_video_generator(id), true),
        Command::CreateSource { id, uri, audio, video } =>
            (self.create_source(id, uri, audio, video), true),
        Command::CreateDestination { id, family, audio, video } =>
            (self.create_destination(id, family, audio, video), true),
        Command::CreateMixer { id, config, audio, video } =>
            (self.create_mixer(id, config, audio, video), true),
 
        // NEW —
        Command::CreateScreenCaptureSource { id, width, height, fps } =>
            (self.create_screen_capture_source(id, width, height, fps), true),
 
        Command::Connect { /* … */ } => /* … */,
        /* … */
    };
 
    if should_sync { self.sync_media_links(); }
    self.refresh_nodes();
    result
}
```
 
…and define the constructor below `create_source` (around line 413):
 
```rust
fn create_screen_capture_source(
    &mut self,
    id: String,
    width: u32,
    height: u32,
    fps: u32,
) -> CommandResult {
    if let Err(err) = self.ensure_unique_id(&id) {
        return CommandResult::Error(err);
    }
    if width == 0 || height == 0 || fps == 0 {
        return CommandResult::Error(format!(
            "ScreenCaptureSource {id} requires non-zero width/height/fps"
        ));
    }
 
    self.nodes.insert(
        id.clone(),
        NodeRecord::ScreenCapture(ScreenCaptureNode::new(id, width, height, fps)),
    );
    CommandResult::Success
}
```
 
### 2.6 Step 6 — add a unit test (smoke shape)
 
**File:** `senders/android/src/migration/node_manager.rs` (in the
`#[cfg(test)] mod tests` block near line 790):
 
```rust
#[test]
fn create_screen_capture_source_succeeds() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateScreenCaptureSource {
        id: "cap-1".into(),
        width: 1280,
        height: 720,
        fps: 30,
    });
    assert!(matches!(result, CommandResult::Success));
    assert!(manager.nodes.contains_key("cap-1"));
}
 
#[test]
fn screen_capture_source_validates_dimensions() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateScreenCaptureSource {
        id: "cap-bad".into(),
        width: 0,
        height: 720,
        fps: 30,
    });
    assert!(matches!(result, CommandResult::Error(_)));
}
```
 
These don't require GStreamer to be initialised — they validate just
the command-dispatch shape.
 
---
 
## 3. Verification
 
### 3.1 Compile check
 
```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```
 
Expect **clean** — most likely failures are:
 
- "non-exhaustive patterns" in one of the `match self` arms. Fix by
  re-checking every `match self` in `impl NodeRecord`.
- `cannot move out of borrowed content` in `wire_need_data` — wrap
  any captured state in `Arc<Mutex<...>>`.
 
### 3.2 Unit tests
 
```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::tests::create_screen_capture_source_succeeds \
    migration::node_manager::tests::screen_capture_source_validates_dimensions
```
 
Both green.
 
### 3.3 On-device smoke
 
The MVP doesn't need this to be tappable from the UI, but you can
smoke-test it via the `test-smoke` quick-action by extending the smoke
flow (post-merge, follow-up):
 
```bash
adb forward tcp:8080 tcp:8080
# After tapping `Migrated srv`:
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}'
# → {"id":null,"result":"success"}
 
curl -X POST http://127.0.0.1:8080/command -d '{"start":{"id":"cap-1"}}'
# → {"id":null,"result":"success"}
# (State transitions to "started"; pipeline tries to read FRAME_PAIR.)
 
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | jq '.result.info.nodes."cap-1"'
# → { "state": "started", "kind": "source", "video_consumer_slot_ids": [...] }
```
 
Without an active MediaProjection session, `FRAME_PAIR` is `None`, so
`need-data` just returns without pushing — the pipeline stays in
`Playing` but produces no buffers. That's the correct behaviour:
MVP-PHASE-6 wires `startScreenCapture(...)` to also issue the graph
command.
 
---
 
## 4. Common pitfalls
 
### P1 — `non-exhaustive patterns` after adding the variant
 
Rust's compiler will catch every missed match arm. Walk the error list
top-to-bottom. Note the rare ones:
 
- `pub fn output_video_appsink` (`node_manager.rs:~150`)
- `MixerNode::connect_input_slot` *might* be called against a
  ScreenCapture *source* via `add_consumer_link` — but the existing
  `Mixer(_) => mixer.connect_output_consumer(...)` arm covers this; no
  change needed because ScreenCapture only outputs.
 
### P2 — Cloning `VideoFrame` is expensive
 
`FRAME_PAIR` holds an owned `VideoFrame`. If you `.clone()` it on every
`need-data`, you allocate. The current cast loop uses a **take and
replace** pattern (`std::mem::take(&mut *pair)`) — mirror that to avoid
clones. See `lib.rs:1456+`.
 
### P3 — `appsrc` caps must match what the YUV bytes actually are
 
`FRAME_PAIR` contains I420 planar YUV (per `nativeProcessFrame` →
`process_frame` at `lib.rs:1900-1970`). If you specify `NV12` or
`RGBA` in the caps, `videoconvert` will error. Stick with `I420`.
 
### P4 — Drop-old vs queue-up
 
The existing cast loop drops old frames in favour of new ones
(`Mutex<Option<VideoFrame>>`). If your `need-data` blocks on a Condvar
forever, you'll deadlock when capture stops. The Condvar timeout
pattern in `lib.rs:1485+` handles this — copy it verbatim.
 
### P5 — `gst_app::AppSrcCallbacks::builder()` requires Send
 
The closure captured into `need_data(...)` must be `Send + Sync`. Any
JNI handle (`JNIEnv`, `JObject`) is **not** Send. Don't capture
anything Java-side in the callback; just read from the global
`FRAME_PAIR` which is a static `Mutex<Option<VideoFrame>>` and Send by
construction.
 
### P6 — Auto-derive `Debug` on `ScreenCaptureNode`
 
`gst::Pipeline` and `AppSink` implement `Debug`. `AppSrcCallbacks` is
**not** stored in the node (it's installed and forgotten). All other
fields derive `Debug` cleanly. If the compiler complains about a
missing `Debug` impl, check that you didn't accidentally capture an
`Arc<dyn FnMut>` somewhere.
 
---
 
## 5. Stop conditions
 
The phase is "done" when:
 
1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. The two unit tests in §3.2 pass.
3. The optional on-device smoke in §3.3 returns `success` for
   `createscreencapturesource` and `getinfo` shows the node in
   `state: started`.
4. The new node, command, and module are visible to all greps below:
 
```bash
grep -n 'CreateScreenCaptureSource\|ScreenCaptureNode' senders/android/src/migration/
# → expect: protocol.rs, node_manager.rs, nodes/screen_capture.rs, nodes/mod.rs
```
 
5. **No MVP cast-path change happens in this phase.** The existing
   screen-mirror cast loop (`Event::StartCast` → direct GStreamer
   pipeline → WHEP receiver) is untouched. That handover happens in
   MVP-PHASE-6.
 
---
 
## 6. Why this matters
 
This phase is the *bridge* between Surface A (legacy cast loop) and
Surface B (migration runtime). After this phase, the runtime can
ingest **the same frames** the cast loop already does — they just take
a different route through the graph. MVP-PHASE-5 then adds the `Whep`
destination, and MVP-PHASE-6 flips the cast loop to drive both via
graph commands instead of direct GStreamer pipeline construction. The
end result: one canonical media-graph API for all sources and sinks,
and a 50-line cast loop that just emits 4 JSON commands.

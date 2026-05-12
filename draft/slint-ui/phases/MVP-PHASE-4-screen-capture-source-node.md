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

## 2. Steps — split into six per-step files

To keep each step skimmable and reviewable in isolation, the
implementation is split across six per-step `MVP-PHASE-4-STEP-N-*.md`
files. Each file follows the same smaller five-section template
(Goal-of-this-step / Pre-flight / The change / Verification /
Next step) and is self-contained — you don't need to flip back to
this parent doc while implementing a single step.

| # | File | Scope | Net diff |
|---|---|---|---|
| 1 | [`MVP-PHASE-4-STEP-1-protocol-extension.md`](./MVP-PHASE-4-STEP-1-protocol-extension.md) | Add `Command::CreateScreenCaptureSource { id, width, height, fps }` with serde defaults `1280 / 720 / 30`. Backward-compatible wire format. | ~25 lines, 1 file (`protocol.rs`) |
| 2 | [`MVP-PHASE-4-STEP-2-screen-capture-node.md`](./MVP-PHASE-4-STEP-2-screen-capture-node.md) | Define `ScreenCaptureNode`, `LiveScreenCapturePipeline`, `build_live_pipeline`, and the `FRAME_PAIR → appsrc` consumer in a new file. **Largest step.** | ~250 Rust lines, 1 new file (`nodes/screen_capture.rs`) |
| 3 | [`MVP-PHASE-4-STEP-3-module-registration.md`](./MVP-PHASE-4-STEP-3-module-registration.md) | Add `pub mod screen_capture;` and `pub use screen_capture::*;` to `nodes/mod.rs`. | 2 lines, 1 file (`nodes/mod.rs`) |
| 4 | [`MVP-PHASE-4-STEP-4-node-record.md`](./MVP-PHASE-4-STEP-4-node-record.md) | Add `NodeRecord::ScreenCapture(ScreenCaptureNode)` and thread it through every `match self` arm in `impl NodeRecord` (~13 methods). | ~80 lines, 1 file (`node_manager.rs`) |
| 5 | [`MVP-PHASE-4-STEP-5-dispatch-arm.md`](./MVP-PHASE-4-STEP-5-dispatch-arm.md) | Add the `Command::CreateScreenCaptureSource` dispatch arm + `create_screen_capture_source(...)` constructor. | ~30 lines, 1 file (`node_manager.rs`) |
| 6 | [`MVP-PHASE-4-STEP-6-unit-tests.md`](./MVP-PHASE-4-STEP-6-unit-tests.md) | 8 host-runnable unit tests across `protocol.rs` and `node_manager.rs`. No GStreamer init required. | ~120 lines of tests across 2 files |

### Recommended landing order

```
Step 1 ──► Step 2 ──► Step 3 ──► Step 4 ──► Step 5 ──┐
                                                     ├── single squash-commit
                                                     ▼   (compile stays clean only
                                                Step 6 (tests)   once Steps 1+2+3+4+5 are all in)
```

Steps 1–5 are **all required to compile**; the compiler's
exhaustiveness check on `NodeRecord` match arms (Step 4) means the
build is red between Steps 2 and 4. The cleanest path is squashing
Steps 1+2+3+4+5 into one commit so the tree compiles between
commits. Step 6 is test-only and lands separately.

---

## 2b. Why the per-step split?

The original monolithic §2 block ran to ~560 lines with six
sub-steps interleaved. Splitting it gives:

- Per-step files small enough to review on a phone screen.
- Independent verification recipes per step (each step's §3 covers
  only that step's compile/test/grep checks).
- Step-specific pitfalls without scrolling past unrelated content.
- Easy follow-up PRs: if a reviewer asks for changes on Step 2
  only, you edit one file.

The pattern mirrors the per-step split applied to PHASE-5 and
PHASE-8 in the same PR.

---

> **Looking for inline §2.1 — §2.6?** The per-step content has
> moved into the six `MVP-PHASE-4-STEP-N-*.md` files listed in
> the table above. Each STEP file is self-contained — Goal,
> Pre-flight, The change, Verification, and Pitfalls for that
> step alone.

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

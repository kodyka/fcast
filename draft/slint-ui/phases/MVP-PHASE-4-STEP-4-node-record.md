# MVP-PHASE-4 — Step 4: extend `NodeRecord` and thread the variant through every match arm

> Part 4 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
> Previous: [Step 3 — module registration](./MVP-PHASE-4-STEP-3-module-registration.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add `ScreenCapture(ScreenCaptureNode)` to the `NodeRecord` enum in
`node_manager.rs` and extend **every** `match self` arm in
`impl NodeRecord` to handle the new variant. This is a Rust
exhaustiveness exercise — the compiler will catch every miss.

After this step the runtime *can carry* a `ScreenCaptureNode` in its
`HashMap<String, NodeRecord>`, but no `dispatch()` arm constructs one
yet. That wiring lands in
[Step 5](./MVP-PHASE-4-STEP-5-dispatch-arm.md).

This is the second-largest step in PHASE-4 (~80 Rust lines spread
across ~10 match arms). The change is mechanical — each arm gets one
new `Self::ScreenCapture(node) => …` line — but missing one breaks
the build.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `enum NodeRecord` | `senders/android/src/migration/node_manager.rs:21-26` |
| `impl NodeRecord` (the trait-like surface) | `node_manager.rs:28-160` |
| Existing variants (`Source / Destination / Mixer / VideoGenerator`) | same |
| `ScreenCaptureNode` (from Step 2) | `nodes/screen_capture.rs` |
| `crate::migration::nodes::ScreenCaptureNode` (path after Step 3) | `nodes/mod.rs` |

### 1.2 Why `NodeRecord` is an enum (not a `Box<dyn NodeTrait>`)

The existing codebase deliberately avoids a `dyn Node` trait because:

- Enum dispatch is `match`-based and zero-cost.
- Adding a new variant fails fast: the compiler lists every missed
  arm in `impl NodeRecord`. With `dyn`, missing a default method
  would silently use the trait default.
- The runtime tracks `NodeRecord` instances by ID in a single
  `HashMap`; an enum is the natural shape.

PHASE-4 follows that convention. Don't refactor to `dyn` here.

### 1.3 Which methods need new arms

Roughly 10 methods on `impl NodeRecord` (lines 28-160):

| Method | What `ScreenCapture` returns |
|---|---|
| `can_output_audio` | `false` |
| `can_output_video` | `true` |
| `can_input_audio` | `false` |
| `can_input_video` | `false` |
| `to_info` | `node.as_info()` |
| `schedule` | `node.schedule(cue, end)` |
| `stop` | `node.stop()` |
| `mark_error` | `node.mark_error(m)` |
| `add_consumer_link` | `node.add_consumer_link(link_id, audio, video)` |
| `remove_consumer_link` | `node.remove_consumer_link(link_id)` |
| `refresh_runtime` | `node.refresh()` (with err → `mark_error`) |
| `output_audio_appsink` | `None` |
| `output_video_appsink` | `node.live_video_appsink()` |

That's 13 methods. The compiler will list any you miss.

---

## 2. The change

**File:** `senders/android/src/migration/node_manager.rs`

### 2.1 Add the variant

Around lines 21-26:

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

### 2.2 Thread it through every `match self` arm

The body of `impl NodeRecord` (lines 28-160). Each method gets one
new arm; the existing arms are left untouched.

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
            Self::Source(node) => node.video_enabled,
            Self::Mixer(node) => node.video_enabled,
            Self::VideoGenerator(_) => true,
            Self::Destination(_) => false,

            // NEW —
            Self::ScreenCapture(_) => true,
        }
    }

    fn can_input_audio(&self) -> bool {
        match self {
            Self::Source(_) | Self::VideoGenerator(_) => false,
            Self::Destination(node) => node.audio_enabled,
            Self::Mixer(node) => node.audio_enabled,

            // NEW —
            Self::ScreenCapture(_) => false,
        }
    }

    fn can_input_video(&self) -> bool {
        match self {
            Self::Source(_) | Self::VideoGenerator(_) => false,
            Self::Destination(node) => node.video_enabled,
            Self::Mixer(node) => node.video_enabled,

            // NEW —
            Self::ScreenCapture(_) => false,
        }
    }

    fn to_info(&self) -> NodeInfo {
        match self {
            Self::Source(node) => node.as_info(),
            Self::Destination(node) => node.as_info(),
            Self::Mixer(node) => node.as_info(),
            Self::VideoGenerator(node) => node.as_info(),

            // NEW —
            Self::ScreenCapture(node) => node.as_info(),
        }
    }

    fn schedule(
        &mut self,
        cue_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
    ) -> Result<(), String> {
        match self {
            Self::Source(node) => node.schedule(cue_time, end_time),
            Self::Destination(node) => node.schedule(cue_time, end_time),
            Self::Mixer(node) => node.schedule(cue_time, end_time),
            Self::VideoGenerator(node) => node.schedule(cue_time, end_time),

            // NEW —
            Self::ScreenCapture(node) => node.schedule(cue_time, end_time),
        }
    }

    fn stop(&mut self) {
        match self {
            Self::Source(node) => node.stop(),
            Self::Destination(node) => node.stop(),
            Self::Mixer(node) => node.stop(),
            Self::VideoGenerator(node) => node.stop(),

            // NEW —
            Self::ScreenCapture(node) => node.stop(),
        }
    }

    fn mark_error(&mut self, m: String) {
        match self {
            Self::Source(node) => node.mark_error(m),
            Self::Destination(node) => node.mark_error(m),
            Self::Mixer(node) => node.mark_error(m),
            Self::VideoGenerator(node) => node.mark_error(m),

            // NEW —
            Self::ScreenCapture(node) => node.mark_error(m),
        }
    }

    fn add_consumer_link(&mut self, link_id: &str, audio: bool, video: bool) {
        match self {
            Self::Source(node) => node.add_consumer_link(link_id, audio, video),
            Self::Mixer(node) => node.add_consumer_link(link_id, audio, video),
            Self::VideoGenerator(node) => node.add_consumer_link(link_id, audio, video),
            Self::Destination(_) => {}

            // NEW —
            Self::ScreenCapture(node) => node.add_consumer_link(link_id, audio, video),
        }
    }

    fn remove_consumer_link(&mut self, link_id: &str) {
        match self {
            Self::Source(node) => node.remove_consumer_link(link_id),
            Self::Mixer(node) => node.remove_consumer_link(link_id),
            Self::VideoGenerator(node) => node.remove_consumer_link(link_id),
            Self::Destination(_) => {}

            // NEW —
            Self::ScreenCapture(node) => node.remove_consumer_link(link_id),
        }
    }

    fn refresh_runtime(&mut self) {
        let result = match self {
            Self::Source(node) => node.refresh(),
            Self::Destination(node) => node.refresh(),
            Self::Mixer(node) => node.refresh(),
            Self::VideoGenerator(node) => node.refresh(),

            // NEW —
            Self::ScreenCapture(node) => node.refresh(),
        };
        if let Err(err) = result {
            self.mark_error(err);
        }
    }

    fn output_audio_appsink(&self) -> Option<AppSink> {
        match self {
            Self::Source(node) => node.live_audio_appsink(),
            Self::Mixer(node) => node.live_audio_appsink(),
            Self::VideoGenerator(_) | Self::Destination(_) => None,

            // NEW —
            Self::ScreenCapture(_) => None,
        }
    }

    fn output_video_appsink(&self) -> Option<AppSink> {
        match self {
            Self::Source(node) => node.live_video_appsink(),
            Self::Mixer(node) => node.live_video_appsink(),
            Self::VideoGenerator(node) => node.live_video_appsink(),
            Self::Destination(_) => None,

            // NEW —
            Self::ScreenCapture(node) => node.live_video_appsink(),
        }
    }
}
```

### 2.3 Don't forget the `use` import

At the top of `node_manager.rs` (around line 15):

```rust
use crate::migration::nodes::{
    DestinationNode, MixerNode, SourceNode, VideoGeneratorNode,
    // NEW —
    ScreenCaptureNode,
};
```

If you skip this, the compiler tells you `cannot find type
ScreenCaptureNode in this scope` at the `enum NodeRecord` definition.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. Most likely failures:

- `error[E0004]: non-exhaustive patterns: Self::ScreenCapture(_) not
  covered` — you missed an arm. Walk the error list top-to-bottom and
  add a `Self::ScreenCapture(...) => …` line to each.
- `error[E0405]: cannot find type ScreenCaptureNode in this scope`
  — you forgot the `use` line in §2.3.

### 3.2 Exhaustiveness sanity check

```bash
grep -nE 'Self::ScreenCapture' senders/android/src/migration/node_manager.rs | wc -l
# → at least 13 (one per impl method + the enum variant)
```

If the count is below 13, look at the previous compile error list to
find missing arms.

### 3.3 Grep

```bash
grep -n 'enum NodeRecord' senders/android/src/migration/node_manager.rs
# → 1 match
grep -nE 'ScreenCapture\(ScreenCaptureNode\)' senders/android/src/migration/node_manager.rs
# → exactly 1 match (the variant declaration)
```

---

## 4. Pitfalls specific to this step

### P1 — `non-exhaustive patterns` after adding the variant

Rust's compiler will catch every missed match arm. Walk the error
list top-to-bottom. Note the rare ones:

- `pub fn output_video_appsink` (around line 150) — easy to miss if
  you only look at the top-level impl block.
- `MixerNode::connect_input_slot` *might* be called against a
  ScreenCapture *source* via `add_consumer_link` — but the existing
  `Mixer(_) => mixer.connect_output_consumer(...)` arm covers this;
  no change needed because ScreenCapture only outputs.

### P2 — `can_output_audio: false` is correct, not a TODO

PHASE-4 explicitly drops audio from the screen-capture path —
MediaProjection on Android doesn't capture system audio in any
useful way without API 29+ entitlements, and the existing cast loop
doesn't ship audio either. Don't return `node.audio_enabled` — there
is no `audio_enabled` field on `ScreenCaptureNode` (intentionally).

### P3 — `can_input_video: false` is also correct

`ScreenCapture` is a **source** node, not a sink. The
`can_input_video` flag controls whether the node can accept incoming
graph links (for downstream consumers). Sources never accept inputs.
Compare with `Self::VideoGenerator(_) => false` for the same reason.

### P4 — Don't accidentally widen `Destination` variant changes

While editing the file you'll touch every `match self` arm. Don't be
tempted to "clean up" the `Destination(_) => false` arms or merge
them with `Self::ScreenCapture(_) => false`. Each variant should
stay on its own line for diff readability and future-step
modifications.

### P5 — `refresh_runtime`'s error handling

The existing pattern is:

```rust
let result = match self { … };
if let Err(err) = result { self.mark_error(err); }
```

Don't re-implement the error handling inside the `ScreenCapture` arm
— it inherits the outer `if let Err` wrap. Returning `Err(...)`
from `node.refresh()` is enough.

### P6 — Forgetting `use AppSink`

The `output_audio_appsink` / `output_video_appsink` methods both
return `Option<AppSink>`. If you accidentally trim
`use gst_app::AppSink;` from the imports while reformatting, the
compiler complains at the existing source/mixer arms — confusing.
Run `cargo +nightly check` once before and once after the variant
change to isolate import errors from match-arm errors.

### P7 — Test code in the same file

`node_manager.rs` has a `#[cfg(test)] mod tests` block (around line
790). The exhaustiveness compiler errors apply to test code too —
any test that constructs a `NodeRecord` manually (rare, but check)
might need the new arm. The two new tests in
[Step 6](./MVP-PHASE-4-STEP-6-unit-tests.md) only call `dispatch()`,
so they don't need to touch enum patterns directly.

---

## 5. Next step

Once this lands, [Step 5](./MVP-PHASE-4-STEP-5-dispatch-arm.md)
wires the `Command::CreateScreenCaptureSource` dispatch arm in
`NodeManager::dispatch` and adds the
`create_screen_capture_source(...)` constructor.

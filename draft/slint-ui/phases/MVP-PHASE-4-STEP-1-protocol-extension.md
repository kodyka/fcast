# MVP-PHASE-4 — Step 1: extend the JSON protocol with `Command::CreateScreenCaptureSource`

> Part 1 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a new `CreateScreenCaptureSource` variant to the `Command` enum so
the migration runtime can accept:

```json
{
  "createscreencapturesource": {
    "id": "cap-1",
    "width": 1280,
    "height": 720,
    "fps": 30
  }
}
```

After this step the JSON shape is wired through serde, but no node is
constructed yet — that lands in
[Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md).

`width`, `height`, and `fps` all carry serde defaults
(`1280 / 720 / 30`) so the minimal accepted form is:

```json
{"createscreencapturesource": {"id": "cap-1"}}
```

…which is convenient for the Surface B smoke flow (PHASE-3) and for
the on-device quick-action that PHASE-6 will eventually wire up.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Command` enum | `senders/android/src/migration/protocol.rs:37-106` |
| Existing variants (`CreateVideoGenerator / CreateSource / CreateDestination / CreateMixer / Connect / Start / Reschedule / Remove / Disconnect / GetInfo / AddControlPoint / RemoveControlPoint`) | same |
| `#[serde(rename_all = "lowercase")]` (the tag-style convention) | `protocol.rs:38` |
| `FRAME_PAIR` writer (informs the dimensions we accept) | `senders/android/src/lib.rs:1900-1970` (`nativeProcessFrame` / `process_frame`) |
| `MainActivity.startScreenCapture(w,h,fps)` (which dimensions the user actually picks) | `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:720` |

### 1.2 Why a new variant (not overloading `CreateSource` with a `screen://` URI)

Tempting (`uri: "screen://"`) but bad: `SourceNode::build_source_element`
hard-wires `fallbacksrc` / `uridecodebin`, both of which will fail on
a non-GStreamer URI scheme. A dedicated variant lets us:

- Drop the unused audio path entirely (screen capture is video-only
  in PHASE-4).
- Pass `width / height / fps` as first-class fields (no URI query
  string parsing).
- Keep the pipeline-construction code in
  [Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md) linear —
  a separate `match` arm in `NodeRecord` and `dispatch`.
- Surface a `ScreenCaptureInfo` (or recycled `SourceInfo`) shape
  back via `getinfo` without polluting `SourceInfo` with screen-
  specific fields.

### 1.3 Why the serde defaults are `1280 / 720 / 30`

Those match the most common Android-side `startScreenCapture(...)`
invocation in the Java code (`MainActivity.java:720`) and the
"720p30" default used by the existing cast loop. Keeping them as
defaults means smoke-tests don't have to spell out every field.

---

## 2. The change

**File:** `senders/android/src/migration/protocol.rs`

Add to the `Command` enum (around lines 37-106):

```rust
// senders/android/src/migration/protocol.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    CreateVideoGenerator { id: String },

    CreateSource {
        id: String,
        uri: String,
        #[serde(default = "default_true")]
        audio: bool,
        #[serde(default = "default_true")]
        video: bool,
    },

    CreateDestination {
        id: String,
        family: DestinationFamily,
        #[serde(default = "default_true")]
        audio: bool,
        #[serde(default = "default_true")]
        video: bool,
    },

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
    Start { /* … */ },
    Reschedule { /* … */ },
    Remove { /* … */ },
    Disconnect { /* … */ },
    GetInfo { /* … */ },
    AddControlPoint { /* … */ },
    RemoveControlPoint { /* … */ },
}

fn default_capture_width() -> u32 { 1280 }
fn default_capture_height() -> u32 { 720 }
fn default_capture_fps() -> u32 { 30 }
```

`#[serde(rename_all = "lowercase")]` means the JSON tag becomes
`"createscreencapturesource"` (matching the existing camelCase-free
convention — same as `"createvideogenerator"`, `"createsource"`, etc.).

### 2.1 Where to place the defaults

The `fn default_capture_*` items should live **alongside the existing
`fn default_true`** in `protocol.rs` (around the bottom of the file,
above the `#[cfg(test)] mod tests` block). That keeps all serde
default helpers in one place and matches the file's existing
convention.

### 2.2 Why `u32`, not `u16` or `i32`

The Java side passes `int` to JNI which deserialises cleanly to
`u32`. Going to `u16` would be tighter (real screens never exceed
65535 px) but it forces a cast at the GStreamer boundary
(`gst::Caps::builder("video/x-raw").field("width", w as i32)`), and
`u32` matches what `nativeProcessFrame` already takes (`lib.rs:1900`).
Stay with `u32`.

### 2.3 (Optional) Add a `ScreenCaptureInfo` variant to `NodeInfo`

In [Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md) the node's
`as_info()` returns a recycled `NodeInfo::Source(SourceInfo {
uri: "screen://1280x720@30fps", … })` so we don't have to touch
`NodeInfo` in this step. If you'd rather model it cleanly, add:

```rust
// In NodeInfo (around lines 169-180 of protocol.rs):
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NodeInfo {
    Source(SourceInfo),
    Destination(DestinationInfo),
    Mixer(MixerInfo),
    VideoGenerator(VideoGeneratorInfo),

    // NEW —
    ScreenCapture(ScreenCaptureInfo),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenCaptureInfo {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub video_consumer_slot_ids: Option<Vec<String>>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
}
```

This is **strictly optional** — keeping `NodeInfo::Source` reuse is
simpler and lets the smoke-test in PHASE-3 continue to work
unchanged. Defer the new variant to PHASE-7 if/when receiver-side
consumers start branching on `kind: "screencapture"`.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. Most likely failures:

- `error[E0046]: not all trait items implemented` — happens if the
  derived `Deserialize` impl somehow doesn't see the new variant.
  Re-run after `cargo clean -p fcast-sender-android` if it persists.
- `cannot find value default_capture_width in this scope` — make
  sure the three `fn default_capture_*` helpers are in the same
  module (file-scope) as the `Command` enum, **not** inside it.

### 3.2 Unit tests (added in [Step 6](./MVP-PHASE-4-STEP-6-unit-tests.md))

Two host-runnable tests, no GStreamer init needed:

```rust
#[test]
fn screen_capture_command_deserialises() {
    let s = r#"{"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}"#;
    let cmd: Command = serde_json::from_str(s).unwrap();
    assert!(matches!(cmd, Command::CreateScreenCaptureSource {
        ref id, width: 1280, height: 720, fps: 30
    } if id == "cap-1"));
}

#[test]
fn screen_capture_command_uses_defaults_when_omitted() {
    let s = r#"{"createscreencapturesource":{"id":"cap-1"}}"#;
    let cmd: Command = serde_json::from_str(s).unwrap();
    assert!(matches!(cmd, Command::CreateScreenCaptureSource {
        ref id, width: 1280, height: 720, fps: 30
    } if id == "cap-1"));
}
```

### 3.3 Grep

```bash
grep -n 'CreateScreenCaptureSource' senders/android/src/migration/protocol.rs
# → exactly one match (the enum variant)
grep -nE 'default_capture_(width|height|fps)' senders/android/src/migration/protocol.rs
# → 6 matches (3 callsites + 3 fn definitions)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting `#[serde(default = …)]` makes `width`/`height`/`fps` mandatory

Without the per-field `#[serde(default = …)]`, the minimal
`{"createscreencapturesource":{"id":"cap-1"}}` form will fail
deserialisation with `missing field 'width'`. Don't lift the
defaults to the struct level — serde tag-style enums don't support
struct-level `#[serde(default)]` on tuple/struct variants.

### P2 — `#[serde(rename_all = "lowercase")]` is already set on the enum

You do **not** need to add `#[serde(rename = "createscreencapturesource")]`
to the variant. The `rename_all` on the enum handles it. Adding both
creates a double-rename that breaks deserialisation silently.

### P3 — Variant ordering in the enum

Place the new variant **after** the other `Create…` variants and
**before** `Connect`. Variant ordering doesn't affect serde
correctness, but it affects compiler-generated match-arm warnings
and helps reviewers scan the diff.

### P4 — Zero values

`width: 0`, `height: 0`, or `fps: 0` will all deserialise cleanly
(they're valid `u32` values), but they'll trip up the GStreamer
caps in [Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md).
Defer that validation to the dispatch arm in
[Step 5](./MVP-PHASE-4-STEP-5-dispatch-arm.md) rather than trying
to encode it at the serde layer.

---

## 5. Next step

Once this lands, [Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md)
defines the `ScreenCaptureNode` struct + GStreamer pipeline in a new
file at `senders/android/src/migration/nodes/screen_capture.rs`.

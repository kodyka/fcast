# MVP-PHASE-4 — Step 5: wire the dispatch arm and `create_screen_capture_source`

> Part 5 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
> Previous: [Step 4 — `NodeRecord` variant + match arms](./MVP-PHASE-4-STEP-4-node-record.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Connect `Command::CreateScreenCaptureSource { id, width, height, fps }`
(added in [Step 1](./MVP-PHASE-4-STEP-1-protocol-extension.md)) to a
new constructor that inserts a `ScreenCaptureNode` into the
`NodeManager`'s nodes map.

After this step the runtime end-to-end accepts the command via JSON,
constructs the node, and `getinfo` reports it. The GStreamer pipeline
itself only starts running once a `Start` command lands or the
default `cue_time = None` transition fires (see
[Step 2](./MVP-PHASE-4-STEP-2-screen-capture-node.md) §`advance_schedule`).

This is the **final wiring step** of PHASE-4. Step 6 is test-only.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `NodeManager::dispatch` | `senders/android/src/migration/node_manager.rs:316` |
| Existing constructors (`create_video_generator / create_source / create_destination / create_mixer`) | `node_manager.rs:386-460` |
| `ensure_unique_id` helper | `node_manager.rs:540-548` |
| `CommandResult::{Success, Error, Info}` | `protocol.rs:230-245` |
| `Command::CreateScreenCaptureSource` (added in Step 1) | `protocol.rs:86-95` |
| `ScreenCaptureNode::new(id, w, h, fps)` (defined in Step 2) | `nodes/screen_capture.rs:155-169` |
| `NodeRecord::ScreenCapture` variant (added in Step 4) | `node_manager.rs:21-26` |

### 1.2 Where the constructor lives

By convention, each `create_*` constructor sits in `impl NodeManager`
right after the `dispatch` block, in **the same order they appear in
`dispatch`**:

- `create_video_generator` — `node_manager.rs:386-405`
- `create_source` — `node_manager.rs:407-435`
- `create_destination` — `node_manager.rs:437-455`
- `create_mixer` — `node_manager.rs:457-475`

`create_screen_capture_source` should land **after `create_source`**
and before `create_destination` (since `CreateScreenCaptureSource` is
slotted between `CreateSource` and `CreateDestination` in the
`Command` enum — see [Step 1](./MVP-PHASE-4-STEP-1-protocol-extension.md)).
The compiler doesn't care; reviewers do.

### 1.3 What `should_sync = true` means

The `dispatch()` function returns `(result, should_sync)`. If
`should_sync` is true, `sync_media_links()` is called after the
command runs — this is needed for any command that creates,
destroys, or relinks a node. `CreateScreenCaptureSource` creates a
node, so `should_sync = true`.

---

## 2. The change

**File:** `senders/android/src/migration/node_manager.rs`

### 2.1 Add the dispatch arm

Around the `dispatch` method (line 316):

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
        Command::Start { /* … */ } => /* … */,
        /* … rest of the arms unchanged … */
    };

    if should_sync { self.sync_media_links(); }
    self.refresh_nodes();
    result
}
```

### 2.2 Add the constructor

Below `create_source` (around line 435):

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
    if id.is_empty() {
        return CommandResult::Error(
            "ScreenCaptureSource requires a non-empty id".to_string(),
        );
    }

    self.nodes.insert(
        id.clone(),
        NodeRecord::ScreenCapture(ScreenCaptureNode::new(id, width, height, fps)),
    );
    CommandResult::Success
}
```

### 2.3 (Optional) Validate `fps` upper bound

`fps > 240` is almost certainly a typo. Reject:

```rust
if fps > 240 {
    return CommandResult::Error(format!(
        "ScreenCaptureSource {id} fps {fps} exceeds the 240 cap"
    ));
}
```

Tradeoff: more error paths to test, but you catch programming errors
earlier. The existing `create_source` doesn't validate URIs against
known schemes, so this is a deviation from convention — defer to
reviewer preference.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. Most likely failures:

- `error[E0004]: non-exhaustive patterns: Command::CreateScreenCaptureSource { .. } not
  covered` — you only added the constructor, not the dispatch arm.
  Look at §2.1 again.
- `cannot find function ensure_unique_id in this scope` — make sure
  the constructor is inside `impl NodeManager`, not at module scope.
- `cannot find type ScreenCaptureNode in this scope` — Step 3
  (`pub mod screen_capture; pub use screen_capture::*;`) is missing.
  Cross-check against [Step 3](./MVP-PHASE-4-STEP-3-module-registration.md).

### 3.2 End-to-end smoke (with `MIGRATION_COMMAND_BIND` set)

```bash
adb forward tcp:8080 tcp:8080
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}'
# → {"id":null,"result":"success"}

curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | \
    jq '.result.info.nodes."cap-1"'
# → { "kind": "source", "uri": "screen://1280x720@30fps", "state": "initial", ... }
```

The node lives in `nodes` but no pipeline is running yet because no
`Start` was issued and `cue_time` was never set. That's expected
behaviour — the GStreamer pipeline only spins up on state transition
to `Starting` / `Started`.

### 3.3 Grep

```bash
grep -n 'Command::CreateScreenCaptureSource' senders/android/src/migration/node_manager.rs
# → 1 match (the dispatch arm)
grep -n 'fn create_screen_capture_source' senders/android/src/migration/node_manager.rs
# → 1 match
grep -n 'NodeRecord::ScreenCapture(ScreenCaptureNode::new' \
    senders/android/src/migration/node_manager.rs
# → 1 match (in the constructor)
```

---

## 4. Pitfalls specific to this step

### P1 — `id` shadowing in the constructor

```rust
self.nodes.insert(
    id.clone(),
    NodeRecord::ScreenCapture(ScreenCaptureNode::new(id, width, height, fps)),
);
```

The `id` argument is moved into `ScreenCaptureNode::new(id, …)`. The
`.clone()` on the previous line gives us a copy for the HashMap key.
Forgetting the `.clone()` produces `borrow of moved value: id`.

### P2 — `should_sync = true` is mandatory

Even though no link can yet point at the new node (no `Connect`
command has been issued), `sync_media_links()` walks every node and
rebuilds any inactive bridges. Setting `should_sync = false` is a
silent correctness bug — subsequent `Connect` commands would not
pick up the new node's `output_video_appsink` on the same `dispatch`
call.

### P3 — `ensure_unique_id` returns `Err` on duplicate

The error format is `"node id {id} already exists"` — matches the
existing convention from `create_source`. Don't customise this; it's
what the test in [Step 6](./MVP-PHASE-4-STEP-6-unit-tests.md)
asserts against.

### P4 — Zero-dimension validation

`0x0@0fps` deserialises cleanly from JSON (all valid `u32` values),
so the validation must happen at dispatch time. If you skip the
check here, GStreamer eventually errors at `gst::Caps::builder(...)`
when the pipeline tries to build — but the error message
("framerate 0/1 is invalid") is far less informative than the one
returned here.

### P5 — Dispatch arm ordering

`Command::CreateScreenCaptureSource` should sit **between**
`CreateMixer` and `Connect` in the match, mirroring the enum
declaration. The compiler doesn't care, but matching the source
order makes the diff trivially reviewable and aligns with the
existing convention (which orders by `Create…`, then `Connect`,
then `Start / Reschedule / Remove / Disconnect / GetInfo`).

### P6 — Don't insert via `entry().or_insert_with(...)`

Tempting (saves the `ensure_unique_id` call), but it silently
succeeds on duplicate IDs by leaving the existing node in place —
the caller has no way to tell the command was rejected. Use
`ensure_unique_id` + `insert` so duplicate creates return `Error`.

### P7 — `Command::CreateScreenCaptureSource` deserialises before this step's constructor exists

If you're commit-by-commit reviewing, Step 1's protocol changes will
add the enum variant; before Step 5's dispatch arm lands, the
compiler will fail with `non-exhaustive patterns`. That's intentional
and the reason the parent doc recommends squashing Steps 1+2+3+4+5
into a single commit for `cargo check` cleanliness between commits.

---

## 5. Next step

Once this lands, [Step 6](./MVP-PHASE-4-STEP-6-unit-tests.md)
adds host-runnable unit tests that exercise the dispatch arm without
requiring GStreamer initialisation.

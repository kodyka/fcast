# MVP-PHASE-4 — Step 6: add host-runnable unit tests

> Part 6 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
> Previous: [Step 5 — dispatch arm](./MVP-PHASE-4-STEP-5-dispatch-arm.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a small set of host-runnable unit tests (no GStreamer init
required) that verify:

- The protocol parses `createscreencapturesource` and applies serde
  defaults correctly.
- `NodeManager::dispatch` accepts the command, inserts a
  `NodeRecord::ScreenCapture`, and reports `Success`.
- Invalid dimensions (`width: 0`, `height: 0`, `fps: 0`) are
  rejected with `CommandResult::Error`.
- Duplicate IDs are rejected.
- `getinfo` returns the expected `kind: "source"` /
  `uri: "screen://…"` shape (recycled `NodeInfo::Source`).

The tests exercise only the command/dispatch layer; they do **not**
build the GStreamer pipeline (which requires `gst::init()` and on
Android also requires the runtime to be running on-device). Pipeline
behaviour is covered by the on-device smoke in §3.3 of the parent doc.

This is the last step of PHASE-4 and is **test-only**. Pure addition;
no source files are modified outside of the test module.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Existing protocol tests | `senders/android/src/migration/protocol.rs:#[cfg(test)] mod tests {…}` |
| Existing `NodeManager` tests | `senders/android/src/migration/node_manager.rs:790-1200 (approx)` |
| `NodeManager::default()` helper | `node_manager.rs:280-285` (already exists; constructs an empty manager) |
| `CommandResult` enum | `protocol.rs:230-245` |
| The two tests called out by the parent doc | `MVP-PHASE-4-screen-capture-source-node.md:§3.2` |

### 1.2 Why no GStreamer init in the tests

`gst::init()` requires the GStreamer runtime libraries which are
shipped on-device only (per `senders/android/app/jni/Android.mk`).
Host-side `cargo test` doesn't link against the platform binaries,
so any test that triggers `build_live_pipeline()` will panic.

The dispatch tests work around this because `dispatch` only inserts
a `NodeRecord` into the HashMap — it never calls `refresh()` until
the next `tick()` cycle (which the tests don't trigger).

### 1.3 Two test modules — protocol and node_manager

Tests are split across two `#[cfg(test)] mod tests` blocks:

- **`protocol.rs`** — Two tests verifying serde shape.
- **`node_manager.rs`** — Three tests verifying dispatch semantics.

Both files already have the `mod tests` block. The new tests are
strict additions.

---

## 2. The change

### 2.1 Protocol tests

**File:** `senders/android/src/migration/protocol.rs` (inside
`#[cfg(test)] mod tests { … }`):

```rust
#[test]
fn screen_capture_command_deserialises() {
    let json = r#"{"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}"#;
    let cmd: Command = serde_json::from_str(json).unwrap();
    match cmd {
        Command::CreateScreenCaptureSource { id, width, height, fps } => {
            assert_eq!(id, "cap-1");
            assert_eq!(width, 1280);
            assert_eq!(height, 720);
            assert_eq!(fps, 30);
        }
        other => panic!("expected CreateScreenCaptureSource, got {other:?}"),
    }
}

#[test]
fn screen_capture_command_uses_defaults_when_omitted() {
    let json = r#"{"createscreencapturesource":{"id":"cap-1"}}"#;
    let cmd: Command = serde_json::from_str(json).unwrap();
    match cmd {
        Command::CreateScreenCaptureSource { id, width, height, fps } => {
            assert_eq!(id, "cap-1");
            assert_eq!(width, 1280); // from default_capture_width
            assert_eq!(height, 720); // from default_capture_height
            assert_eq!(fps, 30);     // from default_capture_fps
        }
        other => panic!("expected CreateScreenCaptureSource, got {other:?}"),
    }
}

#[test]
fn screen_capture_command_roundtrips() {
    let cmd = Command::CreateScreenCaptureSource {
        id: "cap-1".into(),
        width: 1920,
        height: 1080,
        fps: 60,
    };
    let json = serde_json::to_string(&cmd).unwrap();
    let parsed: Command = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, Command::CreateScreenCaptureSource {
        ref id, width: 1920, height: 1080, fps: 60
    } if id == "cap-1"));
}
```

### 2.2 NodeManager dispatch tests

**File:** `senders/android/src/migration/node_manager.rs` (inside
`#[cfg(test)] mod tests { … }`, around line 790):

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

    match manager.nodes.get("cap-1").unwrap() {
        NodeRecord::ScreenCapture(node) => {
            assert_eq!(node.id, "cap-1");
            assert_eq!(node.width, 1280);
            assert_eq!(node.height, 720);
            assert_eq!(node.fps, 30);
            assert_eq!(node.state, State::Initial);
        }
        other => panic!("expected NodeRecord::ScreenCapture, got {other:?}"),
    }
}

#[test]
fn screen_capture_source_validates_dimensions() {
    let mut manager = NodeManager::default();

    for (w, h, fps, label) in [
        (0u32, 720u32, 30u32, "zero width"),
        (1280, 0, 30, "zero height"),
        (1280, 720, 0, "zero fps"),
    ] {
        let result = manager.dispatch(Command::CreateScreenCaptureSource {
            id: format!("cap-bad-{label}"),
            width: w,
            height: h,
            fps,
        });
        assert!(
            matches!(result, CommandResult::Error(_)),
            "expected Error for {label}, got {result:?}"
        );
    }
}

#[test]
fn screen_capture_source_rejects_duplicate_id() {
    let mut manager = NodeManager::default();
    let mk = || Command::CreateScreenCaptureSource {
        id: "cap-1".into(),
        width: 1280,
        height: 720,
        fps: 30,
    };
    assert!(matches!(manager.dispatch(mk()), CommandResult::Success));
    assert!(matches!(manager.dispatch(mk()), CommandResult::Error(_)));
}

#[test]
fn screen_capture_source_appears_in_getinfo() {
    let mut manager = NodeManager::default();
    manager.dispatch(Command::CreateScreenCaptureSource {
        id: "cap-1".into(),
        width: 1280,
        height: 720,
        fps: 30,
    });
    let result = manager.dispatch(Command::GetInfo { id: None });

    let info = match result {
        CommandResult::Info(info) => info,
        other => panic!("expected CommandResult::Info, got {other:?}"),
    };
    let node_info = info.nodes.get("cap-1").expect("cap-1 should be in getinfo");
    match node_info {
        NodeInfo::Source(s) => {
            assert_eq!(s.uri, "screen://1280x720@30fps");
            assert_eq!(s.state, State::Initial);
            assert!(s.audio_consumer_slot_ids.is_none());
        }
        other => panic!("expected NodeInfo::Source, got {other:?}"),
    }
}

#[test]
fn screen_capture_source_uses_serde_defaults_via_dispatch() {
    // Driving through the JSON path (not the typed enum) to confirm
    // serde defaults survive end-to-end.
    let json = r#"{"createscreencapturesource":{"id":"cap-1"}}"#;
    let cmd: Command = serde_json::from_str(json).unwrap();

    let mut manager = NodeManager::default();
    let result = manager.dispatch(cmd);
    assert!(matches!(result, CommandResult::Success));

    let node = manager.nodes.get("cap-1").unwrap();
    if let NodeRecord::ScreenCapture(n) = node {
        assert_eq!(n.width, 1280);
        assert_eq!(n.height, 720);
        assert_eq!(n.fps, 30);
    } else {
        panic!("expected ScreenCapture node");
    }
}
```

### 2.3 Why no `dispatch(Start)` test

Driving the state machine to `Started` triggers `sync_live_pipeline`
which constructs a `gst::Pipeline` — that requires `gst::init()`.
Defer to the on-device smoke in §3.3 of the parent doc. The host-
side tests cover everything that doesn't need GStreamer.

---

## 3. Verification

### 3.1 Run the tests

```bash
cargo +nightly test -p fcast-sender-android \
    migration::protocol::tests::screen_capture_command_deserialises \
    migration::protocol::tests::screen_capture_command_uses_defaults_when_omitted \
    migration::protocol::tests::screen_capture_command_roundtrips \
    migration::node_manager::tests::create_screen_capture_source_succeeds \
    migration::node_manager::tests::screen_capture_source_validates_dimensions \
    migration::node_manager::tests::screen_capture_source_rejects_duplicate_id \
    migration::node_manager::tests::screen_capture_source_appears_in_getinfo \
    migration::node_manager::tests::screen_capture_source_uses_serde_defaults_via_dispatch
```

All eight green.

Or, more concisely:

```bash
cargo +nightly test -p fcast-sender-android screen_capture
# → 8 passed; 0 failed
```

### 3.2 Grep

```bash
grep -nE 'fn (create_screen_capture_source|screen_capture_(source|command))' \
    senders/android/src/migration/{protocol,node_manager}.rs
# → 8 matches (3 in protocol.rs + 5 in node_manager.rs)
```

---

## 4. Pitfalls specific to this step

### P1 — Don't call `manager.tick()` in the tests

`NodeManager::tick()` calls `refresh_nodes()` which calls
`refresh_runtime()` on every node, which in turn calls
`ScreenCaptureNode::refresh()` → `sync_live_pipeline()` →
`build_live_pipeline()` → `gst::Pipeline::new()`. The last step
panics if `gst::init()` wasn't called. Stick to `dispatch()` calls
only.

### P2 — `NodeInfo::Source` vs a potential `NodeInfo::ScreenCapture` variant

The §2.2 test above asserts on `NodeInfo::Source`. If you took the
optional path in [Step 1](./MVP-PHASE-4-STEP-1-protocol-extension.md#23-optional-add-a-screencaptureinfo-variant-to-nodeinfo)
and added a `NodeInfo::ScreenCapture` variant, update the assertion
to match the new variant.

### P3 — `manager.dispatch(GetInfo { id: None })` returns all nodes

If you pass `id: Some("…")` instead of `None`, only that single
node is returned and the `info.nodes` map shape changes. Read the
existing `GetInfo` handler in `node_manager.rs` to confirm the
exact return shape (`CommandResult::Info(GetInfoResponse)`) before
asserting on it.

### P4 — Duplicate-ID error format

`ensure_unique_id` returns the error string `"node id {id} already
exists"`. The test in §2.2 only asserts `matches!(result,
CommandResult::Error(_))` — don't tighten this to a string match
unless you want the test to break on every minor wording tweak.

### P5 — Test isolation

Each test creates its own `NodeManager::default()`. Don't share state
between tests; `cargo test` runs them in parallel by default and
state-sharing is a common cause of flaky tests in this codebase.

### P6 — Compile-time gating

These tests run on any target supported by `cargo test -p
fcast-sender-android` — host and Android. If you wrap them in
`#[cfg(target_os = "android")]`, they won't run in CI on host. Don't
wrap them.

### P7 — `cargo +nightly test` on host might still pull in
`gst-app` linking

If the test target somehow ends up needing `gst::init()`, you'll
see `failed to load library: gstreamer-1.0.so`. This usually means
your test accidentally triggered `tick()` — re-read your code for
any `refresh*()` call.

---

## 5. Next step

This is the **last step in PHASE-4.** Run the full verification
recipe in the parent doc:

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
cargo +nightly test  -p fcast-sender-android screen_capture
```

…then proceed to [MVP-PHASE-5](./MVP-PHASE-5-whep-destination-family.md)
to add the `Whep` destination family, which is the first downstream
consumer of the `ScreenCapture` source node's `appsink` stream.

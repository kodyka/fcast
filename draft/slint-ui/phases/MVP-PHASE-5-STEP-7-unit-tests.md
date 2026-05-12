# MVP-PHASE-5 — Step 7: unit tests

> Part 7 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 6 — extend `LiveDestinationPipeline` to carry the port handle](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add host-runnable unit tests that validate Steps 1–6 together
**without requiring GStreamer initialisation, the WHEP signaller, or
the gst-rs-webrtc plugin** at test time:

| Test | Validates |
|---|---|
| `create_whep_destination_succeeds` | `NodeManager::dispatch` accepts `CreateDestination` with the `Whep` family. |
| `create_whep_destination_default_server_port_when_omitted` | `#[serde(default)]` on `server_port` (Step 1). |
| `whep_destination_serdes_roundtrip` | Wire format round-trips. |
| `whep_destination_info_carries_optional_bound_ports` | `DestinationInfo.bound_port_*` flows through `getinfo` (Steps 1+3). |
| `local_playback_destination_info_omits_bound_ports` | Wire format stays backward-compatible for non-Whep variants. |
| `whep_destination_node_default_bound_ports_are_none` | `DestinationNode::new` initialises both fields to `None` (Step 3). |
| `whep_destination_node_resets_bound_ports_on_stopped` | `refresh()` clears the fields when state-transitions to `Stopped` (Step 6). |

All tests run as `cargo test` on a Linux/macOS dev host — **no
Android emulator, no GStreamer registry, no WHEP signaller**
required. Pipeline-level smoke (verifying that `bound_port_v4`
becomes `Some(_)` after the real signaller starts) is in the
parent doc's §3.3 because it needs an Android device + gstreamer.

Some Step-6 tests in §3.2 of that step **do** require `gst::init()`
(they construct a `gst::Pipeline::default()`); those are kept
isolated under a `#[cfg(feature = "gst-test")]` gate.

---

## 1. Pre-flight

### 1.1 Where each test lives

Match the existing test placement convention:

| Test | Module |
|---|---|
| `create_whep_destination_*` | `node_manager.rs` (tests `NodeManager::dispatch`) |
| `whep_destination_info_carries_optional_bound_ports` | `node_manager.rs` (tests `Command::GetInfo`) |
| `whep_destination_serdes_*` | `protocol.rs` (tests serde shape) |
| `whep_destination_node_*` | `nodes/destination.rs` (tests `DestinationNode` mechanics) |
| `local_playback_destination_info_omits_bound_ports` | `node_manager.rs` (tests `Command::GetInfo`) |

### 1.2 Why these tests don't need `gst::init()`

The tests touch:

- **Type-level data**: `DestinationFamily`, `DestinationInfo`,
  `DestinationNode` — all plain Rust types.
- **Dispatch logic**: `NodeManager::dispatch` — calls
  `create_destination` which inserts a `NodeRecord` into a
  `HashMap`. **Does not build a pipeline.**
- **`refresh()`** — reads `Option<Arc<Mutex<...>>>`. The slot is
  pre-populated by the test, not by a real signaller.

`build_live_pipeline` is **not** called by any of these tests. It's
the only function that needs `gst::init()` (it creates real
GStreamer elements). PHASE-5 explicitly defers pipeline-level smoke
to the on-device test in the parent doc's §3.3.

### 1.3 Existing dev-deps

The tests use `serde_json`, which is already in
`senders/android/Cargo.toml` `[dev-dependencies]` (verified by the
existing protocol tests). No new dev-deps needed.

---

## 2. The change

### 2.1 In `senders/android/src/migration/node_manager.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod whep_destination_tests {
    use super::*;
    use crate::migration::protocol::{Command, DestinationFamily, CommandResult};

    #[test]
    fn create_whep_destination_succeeds() {
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "tv-1".into(),
            family: DestinationFamily::Whep { server_port: 0 },
            audio: false,
            video: true,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
        assert!(manager.nodes.contains_key("tv-1"));
    }

    #[test]
    fn create_whep_destination_with_explicit_port_succeeds() {
        let mut manager = NodeManager::default();
        let result = manager.dispatch(Command::CreateDestination {
            id: "tv-2".into(),
            family: DestinationFamily::Whep { server_port: 54321 },
            audio: false,
            video: true,
        });
        assert!(matches!(result, CommandResult::Success), "{result:?}");
    }

    #[test]
    fn whep_destination_info_carries_optional_bound_ports() {
        let mut manager = NodeManager::default();
        manager.dispatch(Command::CreateDestination {
            id: "tv-1".into(),
            family: DestinationFamily::Whep { server_port: 0 },
            audio: false,
            video: true,
        });
        let info = manager.dispatch(Command::GetInfo { id: Some("tv-1".into()) });
        // Before Start, the bound port is None — no signaller is running.
        if let CommandResult::Info(snapshot) = info {
            let dest = snapshot.nodes.get("tv-1").unwrap();
            match dest {
                NodeInfo::Destination(d) => {
                    assert!(matches!(&d.family, DestinationFamily::Whep { .. }));
                    assert!(d.bound_port_v4.is_none());
                    assert!(d.bound_port_v6.is_none());
                }
                _ => panic!("expected DestinationInfo, got {dest:?}"),
            }
        } else {
            panic!("expected Info, got {info:?}");
        }
    }

    #[test]
    fn local_playback_destination_info_omits_bound_ports() {
        let mut manager = NodeManager::default();
        manager.dispatch(Command::CreateDestination {
            id: "out".into(),
            family: DestinationFamily::LocalPlayback,
            audio: true,
            video: true,
        });
        let info = manager.dispatch(Command::GetInfo { id: Some("out".into()) });
        if let CommandResult::Info(snapshot) = info {
            let dest = snapshot.nodes.get("out").unwrap();
            // Serialize and confirm `bound_port_*` keys are absent.
            let json = serde_json::to_string(dest).unwrap();
            assert!(!json.contains("bound_port"));
        } else {
            panic!("expected Info, got {info:?}");
        }
    }
}
```

### 2.2 In `senders/android/src/migration/protocol.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod whep_protocol_tests {
    use super::*;

    #[test]
    fn whep_destination_serdes_roundtrip() {
        let original = DestinationFamily::Whep { server_port: 0 };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: DestinationFamily = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);

        // Wire format is the externally-tagged enum shape.
        assert!(json.starts_with(r#"{"Whep":{"server_port":0"#));
    }

    #[test]
    fn whep_destination_default_server_port_when_omitted() {
        let minimal: DestinationFamily =
            serde_json::from_str(r#"{"Whep":{}}"#).unwrap();
        if let DestinationFamily::Whep { server_port } = minimal {
            assert_eq!(server_port, 0);
        } else {
            panic!("expected Whep variant");
        }
    }

    #[test]
    fn whep_destination_info_bound_ports_skipped_when_none() {
        let info = DestinationInfo {
            family: DestinationFamily::Whep { server_port: 0 },
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            bound_port_v4: None,
            bound_port_v6: None,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("bound_port"));
    }

    #[test]
    fn whep_destination_info_bound_ports_emit_when_some() {
        let info = DestinationInfo {
            family: DestinationFamily::Whep { server_port: 0 },
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            bound_port_v4: Some(54321),
            bound_port_v6: Some(54322),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains(r#""bound_port_v4":54321"#));
        assert!(json.contains(r#""bound_port_v6":54322"#));
    }
}
```

### 2.3 In `senders/android/src/migration/nodes/destination.rs`

Add to the existing `#[cfg(test)] mod tests` block:

```rust
#[cfg(test)]
mod whep_node_tests {
    use super::*;

    #[test]
    fn whep_destination_node_default_bound_ports_are_none() {
        let node = DestinationNode::new(
            "tv-1".into(),
            DestinationFamily::Whep { server_port: 0 },
            false,
            true,
        );
        assert!(node.whep_bound_port_v4.is_none());
        assert!(node.whep_bound_port_v6.is_none());
    }

    #[test]
    fn whep_destination_as_info_propagates_bound_ports() {
        let mut node = DestinationNode::new(
            "tv-1".into(),
            DestinationFamily::Whep { server_port: 0 },
            false,
            true,
        );
        node.whep_bound_port_v4 = Some(54321);
        node.whep_bound_port_v6 = Some(54322);

        let NodeInfo::Destination(info) = node.as_info() else {
            panic!("expected DestinationInfo");
        };
        assert_eq!(info.bound_port_v4, Some(54321));
        assert_eq!(info.bound_port_v6, Some(54322));
    }

    #[test]
    fn whep_destination_node_resets_bound_ports_on_stopped() {
        let mut node = DestinationNode::new(
            "tv-1".into(),
            DestinationFamily::Whep { server_port: 0 },
            false,
            true,
        );
        node.whep_bound_port_v4 = Some(54321);
        node.whep_bound_port_v6 = Some(54322);
        node.state = State::Stopped;
        // Note: refresh() touches live_pipeline; the slot read is
        // skipped if live_pipeline is None. Only the reset-on-Stopped
        // branch executes here.
        node.refresh().unwrap();
        assert!(node.whep_bound_port_v4.is_none());
        assert!(node.whep_bound_port_v6.is_none());
    }

    #[test]
    fn migration_crate_can_import_whep_signaller() {
        // Compile-time check that the Step 5 re-export + shim are wired.
        let _signaller: Option<crate::whep_signaller_compat::WhepServerSignaller> = None;
        assert!(!crate::whep_signaller_compat::ON_SERVER_STARTED_SIGNAL_NAME.is_empty());
    }
}
```

### 2.4 Gated GStreamer-init tests (optional)

Tests that construct a `gst::Pipeline::default()` (e.g.
`refresh_reads_whep_bound_ports_from_live_pipeline_arc` from Step 6
§3.2) require `gst::init()` and are kept under a feature gate so
host CI doesn't need GStreamer installed:

```rust
#[cfg(test)]
#[cfg(feature = "gst-test")]
mod whep_gst_tests {
    use super::*;

    #[ctor::ctor]
    fn init_gst() {
        let _ = gst::init();
    }

    #[test]
    fn refresh_reads_whep_bound_ports_from_live_pipeline_arc() {
        use std::sync::{Arc, Mutex};

        let bound = Arc::new(Mutex::new(Some((54321u16, 54322u16))));
        let live = LiveDestinationPipeline {
            pipeline: gst::Pipeline::default(),
            video_appsrc: None,
            audio_appsrc: None,
            whep_bound_ports: Some(bound.clone()),
        };

        let mut node = DestinationNode::new(
            "tv-1".into(),
            DestinationFamily::Whep { server_port: 0 },
            false,
            true,
        );
        node.live_pipeline = Some(live);

        node.refresh().unwrap();
        assert_eq!(node.whep_bound_port_v4, Some(54321));
        assert_eq!(node.whep_bound_port_v6, Some(54322));
    }
}
```

Add the feature to `Cargo.toml`:

```toml
[features]
gst-test = []
```

(If the crate already has a `gst-test` feature or equivalent,
reuse it. Don't add a parallel feature.)

---

## 3. Verification

### 3.1 Run the test selectors

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::whep_destination_tests
```

Expect **4 tests** green.

```bash
cargo +nightly test -p fcast-sender-android \
    migration::protocol::whep_protocol_tests
```

Expect **4 tests** green.

```bash
cargo +nightly test -p fcast-sender-android \
    migration::nodes::destination::whep_node_tests
```

Expect **4 tests** green.

### 3.2 Full sweep

```bash
cargo +nightly test -p fcast-sender-android whep_
```

Expect **12+ tests** green (4 node_manager + 4 protocol + 4 node).

### 3.3 GStreamer-gated tests

```bash
cargo +nightly test -p fcast-sender-android --features gst-test \
    migration::nodes::destination::whep_gst_tests
```

Expect **1 test** green. Requires `libgstreamer-1.0-0` and
`gst::init()` to succeed on the host.

### 3.4 Grep recipe

```bash
grep -rn 'fn whep_' senders/android/src/migration/ --include='*.rs'
# → expect: ~12 entries (one per test).
```

---

## 4. Pitfalls specific to this step

### S7-P1 — Putting all tests in one module

Tempting but bad — splits concerns:
- `node_manager.rs` tests: dispatch + `NodeManager` state.
- `protocol.rs` tests: JSON serde shape.
- `nodes/destination.rs` tests: `DestinationNode` mechanics.

Each module has its own `#[cfg(test)] mod tests` block with its own
helpers. **Match the existing convention** — every `Destination*`
test in the repo follows this layout.

### S7-P2 — Using `.unwrap()` on `CommandResult`

`CommandResult` is `Success / Error(String) / Info(...)`. Calling
`.unwrap()` doesn't compile — it's not a `Result`. Use:

```rust
assert!(matches!(result, CommandResult::Success), "{result:?}");
```

The `"{result:?}"` interpolation prints the `Error(...)` payload on
failure — invaluable when CI logs are the only diagnostic.

### S7-P3 — Asserting on `manager.nodes.len()`

Brittle: `NodeManager::default()` may auto-create internal nodes
later. **Prefer:**

```rust
assert!(manager.nodes.contains_key("tv-1"));
```

### S7-P4 — Forgetting the GStreamer-gated tests

The GStreamer-init tests in §2.4 are **optional** — the core PHASE-5
correctness is covered by the gst-free tests in §2.1–§2.3. If the
host CI doesn't have GStreamer installed, the `gst-test` feature is
simply never activated and the tests are skipped.

Don't promote the gated tests to unconditional — that breaks host CI
that lacks `libgstreamer-1.0-0`.

### S7-P5 — Trying to test `build_live_pipeline` directly

```rust
#[test]
fn build_live_pipeline_constructs_whep_arm() {
    let mut node = DestinationNode::new(/* … */);
    node.build_live_pipeline().unwrap();  // ❌ needs gst::init + plugins
    assert!(node.live_pipeline.is_some());
}
```

Even with `gst::init()`, this requires the `gst-rs-webrtc` plugin
loaded — which depends on Android-specific GStreamer SDK builds.
This is the on-device smoke in the parent doc's §3.3, **not** a
unit test.

### S7-P6 — Asserting on `json.contains("bound_port_v4")` only

The `Option<u16>` serde representation can be `"bound_port_v4":null`
if `skip_serializing_if` is **forgotten** in Step 1. The naive
`!json.contains("bound_port_v4")` test catches only the absence of
the key, not the absence of a `null` value.

The `whep_destination_info_bound_ports_skipped_when_none` test in
§2.2 uses `!json.contains("bound_port")` which catches both the key
and any prefix. Match that.

---

## 5. Stop conditions for PHASE-5

After Steps 1–7, the phase is "done" when:

1. `cargo check` is clean across all targets (host + Android arm64).
2. All ~12 unit tests (§3.1–§3.2) pass on the host.
3. The on-device WHEP smoke in the parent doc's §3.3 displays the
   ball pattern on an FCast receiver within ~1s of `start`.
4. `getinfo` returns `bound_port_v4: Some(<u16>)` within ~500ms
   of `start` on the WHEP destination.
5. New surface area is visible to:

```bash
grep -n 'DestinationFamily::Whep' senders/android/src/migration/
# → expect: protocol.rs (Step 1), nodes/destination.rs (Steps 2+3+4+6),
#   node_manager.rs (tests from Step 7), tests in protocol.rs (Step 7).
```

6. The signaller re-export is in place:

```bash
grep -n 'pub mod whep_signaller' sdk/mirroring_core/src/lib.rs
# → expect: 1 match (Step 5 §2.1).

grep -rn 'whep_signaller_compat' senders/android/src/
# → expect: matches in lib.rs (mod) + nodes/destination.rs (use).
```

7. The legacy `mcore::transmission::WhepSink` is **unchanged**:

```bash
grep -n 'WhepSink::new' senders/android/src/lib.rs
# → expect: 1 match at the legacy call site (lib.rs:943-950) —
#   PHASE-6 removes this; PHASE-5 leaves it alone.
```

The phase is **complete** when all seven items hold. PHASE-5 is the
prerequisite for **MVP-PHASE-6** (graph-command cast loop), which
flips the live `Event::StartCast` handler to issue `createdestination` +
`connect` + `start` graph commands instead of constructing `WhepSink`
directly.

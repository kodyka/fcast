# MVP-PHASE-5 — Step 3: add bound-port fields on `DestinationNode`

> Part 3 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 2 — extend `DestinationPipelineProfile::from_family`](./MVP-PHASE-5-STEP-2-pipeline-profile.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Extend the `DestinationNode` struct with two new fields:

```rust
pub whep_bound_port_v4: Option<u16>,
pub whep_bound_port_v6: Option<u16>,
```

…and propagate them through `DestinationNode::new`, `Default`, all
test sites, and the `as_info()` method that builds `DestinationInfo`
for `getinfo`.

These are the **runtime-observed** counterparts to
`DestinationInfo.bound_port_v4` / `bound_port_v6` that
[Step 1](./MVP-PHASE-5-STEP-1-protocol-extension.md) added. The
producer (the signaller's `on-server-started` signal handler) is
wired up in [Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md);
the consumer (`refresh()` that reads the `Arc<Mutex<…>>` handoff)
is in [Step 6](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md).

This step **only** adds the storage. After it lands, the fields
exist on `DestinationNode` and flow through `as_info()` to
`DestinationInfo`, but no path writes to them yet. That's fine —
they default to `None` and the `skip_serializing_if = "Option::is_none"`
attribute on `DestinationInfo` (Step 1) hides them in the wire format.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationNode` struct | `senders/android/src/migration/nodes/destination.rs:107-121` |
| `DestinationNode::new` constructor | `senders/android/src/migration/nodes/destination.rs:128+` (search for `pub fn new`) |
| `Default for DestinationNode` impl | same file, search for `impl Default for DestinationNode` |
| `as_info()` method (or equivalent) | same file, search for `fn as_info` or `DestinationInfo {` |
| Existing `Destination*` test sites in `node_manager.rs` | `senders/android/src/migration/node_manager.rs:1196, 1232, 1254, 1271, 1288, 1308` (search for `DestinationFamily::LocalPlayback`) |

### 1.2 Why two fields, not one `(u16, u16)` tuple

`WhepServerSignaller` binds two listeners (IPv4 + IPv6) and emits
both ports in `on-server-started`. The consumer (the cast-loop
adapter in MVP-PHASE-6) usually wants only one of them — depending
on the receiver's network stack. Storing them as two `Option<u16>`
fields is more ergonomic:

```rust
let url = format!("http://[::1]:{}/whep", node.whep_bound_port_v6.unwrap_or(0));
// vs:
let url = format!("http://[::1]:{}/whep", node.whep_bound_ports.map(|p| p.1).unwrap_or(0));
```

A tuple also forces `Some((0, 0))` as a sentinel for "signaller
hasn't started yet", which is ambiguous with "signaller started but
bound to port 0" (impossible in practice — listener never returns 0
from `local_addr()` after `bind` succeeds — but ambiguous in the type
system).

### 1.3 Why not `pub` getters instead of `pub` fields

The other fields on `DestinationNode` are `pub` (e.g.
`pub state: State`, `pub last_error: Option<String>`). Match the
existing convention. Encapsulation via getters is out of scope.

---

## 2. The change

### 2.1 Add the fields to `DestinationNode`

**File:** `senders/android/src/migration/nodes/destination.rs`
(lines 107-121):

```rust
#[derive(Debug, Clone)]
pub struct DestinationNode {
    pub id: String,
    pub family: DestinationFamily,
    pub audio_enabled: bool,
    pub video_enabled: bool,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,
    pub pipeline: Option<DestinationPipelineProfile>,
    pub live_pipeline: Option<LiveDestinationPipeline>,
    pub last_error: Option<String>,

    // NEW — surfaced via DestinationInfo for WHEP destinations.
    pub whep_bound_port_v4: Option<u16>,
    pub whep_bound_port_v6: Option<u16>,
}
```

### 2.2 Initialize in `DestinationNode::new`

Search the file for `pub fn new(` and find the `DestinationNode { … }`
literal. Add the two new fields at the end:

```rust
impl DestinationNode {
    pub fn new(
        id: String,
        family: DestinationFamily,
        audio_enabled: bool,
        video_enabled: bool,
    ) -> Self {
        DestinationNode {
            id,
            family,
            audio_enabled,
            video_enabled,
            audio_slot_id: None,
            video_slot_id: None,
            cue_time: None,
            end_time: None,
            state: State::Initial,
            pipeline: None,
            live_pipeline: None,
            last_error: None,

            // NEW —
            whep_bound_port_v4: None,
            whep_bound_port_v6: None,
        }
    }
}
```

### 2.3 Initialize in `Default for DestinationNode` (if such impl exists)

If `nodes/destination.rs` derives `Default` on `DestinationNode` via
`#[derive(Default)]`, both new `Option<u16>` fields default to
`None` automatically — no change needed. If there's a hand-rolled
`impl Default`, add the two fields to it.

### 2.4 Plumb through `as_info()`

Find `as_info()` (or the equivalent method that builds
`DestinationInfo`) and add the two new fields:

```rust
pub fn as_info(&self) -> NodeInfo {
    NodeInfo::Destination(DestinationInfo {
        family: self.family.clone(),
        audio_slot_id: self.audio_slot_id.clone(),
        video_slot_id: self.video_slot_id.clone(),
        cue_time: self.cue_time,
        end_time: self.end_time,
        state: self.state,

        // NEW —
        bound_port_v4: self.whep_bound_port_v4,
        bound_port_v6: self.whep_bound_port_v6,
    })
}
```

### 2.5 Test sites in `node_manager.rs`

The existing tests at `node_manager.rs:1196, 1232, 1254, 1271, 1288,
1308` all hard-code `DestinationFamily::LocalPlayback`. None of them
construct `DestinationNode` directly — they go through `dispatch()`.
**No edits required.** The two new fields are populated via
`DestinationNode::new`, which is called inside `create_destination`.

### 2.6 Test sites that construct `DestinationNode` directly

Search for `DestinationNode {` (with the opening brace) — any
hand-rolled struct literal in tests will need the two new fields. As
of `master`, there are zero such sites in production code; tests
all go through `DestinationNode::new`. If you find any during the
edit, add `whep_bound_port_v4: None, whep_bound_port_v6: None`.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Step 1 + Step 2 + Step 3, expect a single remaining compile
error: `non-exhaustive patterns: 'DestinationFamily::Whep { ... }' not
covered` inside `build_live_pipeline`. That's fixed by Step 4.

### 3.2 Test that `as_info` carries the new fields

Drop into the existing `#[cfg(test)] mod tests` block in
`nodes/destination.rs`:

```rust
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
fn local_playback_destination_as_info_omits_bound_ports() {
    let node = DestinationNode::new(
        "out".into(),
        DestinationFamily::LocalPlayback,
        true,
        true,
    );
    let NodeInfo::Destination(info) = node.as_info() else {
        panic!("expected DestinationInfo");
    };
    assert!(info.bound_port_v4.is_none());
    assert!(info.bound_port_v6.is_none());
}
```

All three green.

### 3.3 Grep recipe

```bash
grep -n 'whep_bound_port' senders/android/src/migration/nodes/destination.rs
# → expect: 4+ matches — struct definition (2), DestinationNode::new (2),
#   as_info (2). All in destination.rs only.

grep -n 'bound_port_v' senders/android/src/migration/protocol.rs
# → expect: 2 matches in DestinationInfo (from Step 1).
```

---

## 4. Pitfalls specific to this step

### S3-P1 — Forgetting to update `DestinationNode::new`

If you add the fields to the struct definition but not to `new()`,
the compiler errors with `missing fields whep_bound_port_v4,
whep_bound_port_v6 in initializer of DestinationNode`. The
compiler error is loud; this is hard to miss. The pitfall is
sneaking in `#[derive(Default)]` on the struct (because
`Option<u16>::default()` is `None`) and removing the explicit
field-by-field constructor — that changes the public API and
breaks call sites.

**Don't add `#[derive(Default)]` as a shortcut.** The hand-rolled
`new()` is intentional.

### S3-P2 — Naming the fields `bound_port_v4` (no `whep_` prefix)

`DestinationInfo.bound_port_v4` has no prefix because it's
context-scoped to a destination. But `DestinationNode` carries
state for **all** destination families, including future ones that
might also have bound ports (e.g. a hypothetical SRT listener
destination). Prefix the storage fields with `whep_` to disambiguate;
keep the `DestinationInfo` field names short for ergonomic
serialization.

This is the standard "internal vs external naming" convention.

### S3-P3 — Initializing the fields to `Some(0)` instead of `None`

`Some(0)` is ambiguous with "signaller hasn't started yet" vs
"signaller bound to port 0". The pre-start sentinel is `None`.
Initialize as `None`; let the signal handler in
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) write `Some(actual_port)`
once `on-server-started` fires.

### S3-P4 — Reading the fields before `refresh()` runs

After [Step 6](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md),
the fields are populated **inside `refresh()`** — i.e. the
`whep_bound_port_*` only become `Some(_)` after the next tick of the
100ms refresh loop. Consumers polling `getinfo` synchronously after
`Start` will see `None` until at least one tick has passed.

For PHASE-6's cast-loop adapter, this is fine — the adapter polls
`getinfo` repeatedly. **Don't** add a synchronous getter that
bypasses `refresh()`; that introduces races against the signal
handler thread.

### S3-P5 — Resetting the fields on `Stop`

When `DestinationNode::stop()` tears down the pipeline, should it
reset `whep_bound_port_*` to `None`?

**Recommendation:** yes, in
[Step 6](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md). A
restarted destination gets a new bound port; stale values would
confuse consumers. Add `self.whep_bound_port_v4 = None;
self.whep_bound_port_v6 = None;` to the `Stopped` branch of
`refresh()` (or wherever the pipeline teardown happens).

---

## 5. Next step

After this lands, [Step 4 — wire the Whep arm in `build_live_pipeline`](./MVP-PHASE-5-STEP-4-build-live-pipeline.md)
adds the actual GStreamer pipeline construction: instantiate
`BaseWebRTCSink`, configure the signaller, register the
`on-server-started` callback that writes into the `Arc<Mutex<…>>`
handoff, and link the video chain. Step 4 is the largest step in
PHASE-5 — roughly 80 lines of Rust.

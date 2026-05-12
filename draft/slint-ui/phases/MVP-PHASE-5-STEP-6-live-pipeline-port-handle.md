# MVP-PHASE-5 — Step 6: extend `LiveDestinationPipeline` to carry the port handle

> Part 6 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 5 — re-export `WhepServerSignaller` into the migration crate](./MVP-PHASE-5-STEP-5-signaller-reexport.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add the `whep_bound_ports: Option<Arc<Mutex<Option<(u16, u16)>>>>`
field to `LiveDestinationPipeline` and extend `refresh()` to:

1. Read the `Arc<Mutex<…>>` slot.
2. Copy the bound ports into the `DestinationNode.whep_bound_port_v4`
   / `whep_bound_port_v6` fields that
   [Step 3](./MVP-PHASE-5-STEP-3-destination-node-fields.md) added.

This is the **consumer side** of the bound-port handshake.
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) is the
producer side (the signal handler that writes into the slot).
Together they close the loop:

```
signaller.on-server-started ──► Arc<Mutex<Option<(u16, u16)>>>
                                              │
                              [STEP 4 producer]
                                              │
                                              ▼
                              [STEP 6 consumer] ── refresh() ──► node.whep_bound_port_v4
                                                                  node.whep_bound_port_v6
                                                                                │
                                                                                ▼
                                                                  ── as_info() ──► DestinationInfo.bound_port_v4
                                                                                   DestinationInfo.bound_port_v6
                                                                                   (visible via `getinfo`)
```

After this step, a downstream consumer (the MVP-PHASE-6 cast-loop
adapter) can poll `getinfo` until `bound_port_v4` becomes `Some(_)`
and then construct the WHEP URL.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `LiveDestinationPipeline` struct | `senders/android/src/migration/nodes/destination.rs:22-27` |
| `DestinationNode::refresh` (or equivalent) | search for `fn refresh` in `nodes/destination.rs` |
| `poll_bus_messages` | same file (called from `refresh`) |
| `whep_bound_port_v4 / v6` on `DestinationNode` | added in Step 3 |
| `bound_port_v4 / v6` on `DestinationInfo` | added in Step 1 |
| Arc handoff in `build_live_pipeline` | added in Step 4 |

### 1.2 Why a shared `Arc<Mutex<Option<(u16, u16)>>>` and not a channel

Option (a) **Channel** (`mpsc::Sender<(u16, u16)>`):
- Pro: explicit hand-off, the receiver knows when a new port is
  available.
- Con: requires the receiver to call `try_recv` on each refresh
  tick; if the channel is empty, you have no current value (you'd
  need to cache the last received value separately).

Option (b) **Shared `Arc<Mutex<Option<…>>>`** (this step):
- Pro: the consumer reads the **current value** on each refresh
  tick — no buffering, no draining.
- Con: locks every read (negligible cost — `Mutex<Option<(u16, u16)>>`
  is uncontended).

The signaller emits `on-server-started` **exactly once** per
session. Buffering via a channel is overkill. Shared state with a
`Mutex` is simpler. Match the legacy pattern in
`mcore::transmission::WhepSink::new`.

### 1.3 Why `Option<Arc<…>>` on `LiveDestinationPipeline`

Only `Whep` destinations need the handoff. The four existing
families (Rtmp / Udp / LocalFile / LocalPlayback) leave it `None`.
This keeps the `LiveDestinationPipeline` struct uniform across
families (every variant has every field) without forcing the
non-WHEP arms to allocate an `Arc<Mutex<…>>` that's never written
to.

### 1.4 Why `refresh()` resets the fields to `None` on `Stopped`

A WHEP destination that's restarted gets a new bound port — the
signaller's listener is torn down and rebound. Leaving stale
port values in `whep_bound_port_*` would confuse the cast-loop
adapter (it would see the **old** port via `getinfo` between
`stop` and the next `on-server-started`).

Reset to `None` in the `Stopped` branch of `refresh()`. The fresh
`Some(port)` arrives via the signal handler when the new signaller
starts.

---

## 2. The change

### 2.1 Add the field to `LiveDestinationPipeline`

**File:** `senders/android/src/migration/nodes/destination.rs`
(lines 22-27):

```rust
#[derive(Debug, Clone)]
pub struct LiveDestinationPipeline {
    pub pipeline: gst::Pipeline,
    pub video_appsrc: Option<AppSrc>,
    pub audio_appsrc: Option<AppSrc>,

    // NEW — `Some(...)` for `DestinationFamily::Whep`, `None` otherwise.
    pub whep_bound_ports: Option<std::sync::Arc<std::sync::Mutex<Option<(u16, u16)>>>>,
}
```

### 2.2 Initialize `whep_bound_ports` in `build_live_pipeline`

For the non-WHEP arms (Rtmp, Udp, LocalFile, LocalPlayback), the
final struct-literal at the end of `build_live_pipeline` must add
`whep_bound_ports: None`.

For the WHEP arm
([Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) §2), the
struct-literal sets `whep_bound_ports: Some(bound_ports)`.

### 2.3 Extend `refresh()` to read the slot

Find `fn refresh` in `nodes/destination.rs`:

```rust
pub fn refresh(&mut self) -> Result<(), String> {
    // …existing schedule + pipeline sync…
    self.poll_bus_messages()?;

    // NEW — capture the bound port if the signaller has emitted it.
    if let Some(live) = self.live_pipeline.as_ref() {
        if let Some(handle) = live.whep_bound_ports.as_ref() {
            if let Ok(g) = handle.lock() {
                if let Some((v4, v6)) = *g {
                    self.whep_bound_port_v4 = Some(v4);
                    self.whep_bound_port_v6 = Some(v6);
                }
            }
        }
    }

    // NEW — reset bound ports when the pipeline tears down.
    if matches!(self.state, State::Stopped) {
        self.whep_bound_port_v4 = None;
        self.whep_bound_port_v6 = None;
    }

    Ok(())
}
```

After this, a downstream consumer (the MVP-PHASE-6 cast-loop adapter)
can poll `getinfo` until `bound_port_v4` is `Some(_)` and then use
it to construct the WHEP URL — replacing the legacy
`Event::SignallerStarted` callback flow.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Steps 1–6, all `DestinationFamily` matches are exhaustive,
all struct literals carry the new fields, and `refresh()` plumbs
the bound port through. Expect **clean**.

### 3.2 Test the refresh handoff

Drop into the `#[cfg(test)] mod tests` block in
`nodes/destination.rs`:

```rust
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

#[test]
fn refresh_resets_whep_bound_ports_when_stopped() {
    let mut node = DestinationNode::new(
        "tv-1".into(),
        DestinationFamily::Whep { server_port: 0 },
        false,
        true,
    );
    node.whep_bound_port_v4 = Some(54321);
    node.whep_bound_port_v6 = Some(54322);
    node.state = State::Stopped;
    node.refresh().unwrap();
    assert!(node.whep_bound_port_v4.is_none());
    assert!(node.whep_bound_port_v6.is_none());
}

#[test]
fn refresh_leaves_non_whep_destination_bound_ports_none() {
    let mut node = DestinationNode::new(
        "out".into(),
        DestinationFamily::LocalPlayback,
        true,
        true,
    );
    let live = LiveDestinationPipeline {
        pipeline: gst::Pipeline::default(),
        video_appsrc: None,
        audio_appsrc: None,
        whep_bound_ports: None,
    };
    node.live_pipeline = Some(live);
    node.refresh().unwrap();
    assert!(node.whep_bound_port_v4.is_none());
    assert!(node.whep_bound_port_v6.is_none());
}
```

All three green. **Caveat:** the first two tests construct a
`gst::Pipeline::default()` which requires `gst::init()` to have
been called at test-process startup. Either call `gst::init().unwrap()`
in a `#[ctor]` block at module top, or guard the test with
`#[cfg_attr(not(feature = "gst-test"), ignore)]` — match whichever
convention the existing tests in this file use.

### 3.3 Grep recipe

```bash
grep -nA1 'whep_bound_ports' senders/android/src/migration/nodes/destination.rs
# → expect: 6+ matches —
#     1. struct LiveDestinationPipeline (field declaration)
#     2. Whep arm in build_live_pipeline (writes Some(bound_ports))
#     3-6. Non-Whep arms (write None)
#     7. refresh() reads the field

grep -n 'whep_bound_port_v4\|whep_bound_port_v6' \
    senders/android/src/migration/nodes/destination.rs
# → expect: 6+ matches —
#     1-2. struct DestinationNode (field declarations, Step 3)
#     3-4. DestinationNode::new (Step 3)
#     5-6. as_info (Step 3)
#     7-8. refresh (this step)
#     9-10. refresh reset on Stopped (this step)
```

---

## 4. Pitfalls specific to this step

### S6-P1 — Locking the mutex inside the hot refresh path with `unwrap`

```rust
let g = handle.lock().unwrap();
if let Some((v4, v6)) = *g { /* … */ }
```

`Mutex::lock` returns `Err(_)` only if the mutex is **poisoned**
(a previous holder panicked while holding it). The signal handler
in [Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) returns
early on `vals.get` failure (with `?`), so it never panics inside
the lock. Poisoning is therefore impossible in practice.

`if let Ok(g) = handle.lock() { … }` (the snippet in §2.3) is the
defensive choice — refresh continues even if poisoning somehow
occurs. The `.unwrap()` shortcut would crash the migration runtime
on any poisoning event.

### S6-P2 — Forgetting `whep_bound_ports: None` on non-Whep arms

When Step 6 adds the new field, **every** `LiveDestinationPipeline { … }`
struct-literal must include it. Compiler error
`missing field 'whep_bound_ports' in initializer of LiveDestinationPipeline`
catches this immediately — but you have to walk each non-Whep arm
and add `whep_bound_ports: None`. Pull up the file in your editor
with a find on `LiveDestinationPipeline {` and edit each one.

### S6-P3 — Reading the slot before the signaller starts

The slot is `None` until the `on-server-started` signal fires. The
signal fires when the underlying `TcpListener::bind` returns — which
happens **during pipeline state-change to `Playing`**, not at
`build_live_pipeline` time.

Concretely: `getinfo` immediately after `start` will return
`bound_port_v4: None` for ~50–200ms while the listener binds. The
PHASE-6 cast-loop adapter polls in a loop until it sees `Some(_)`.

This is **correct behaviour**. Don't try to make `build_live_pipeline`
block until the port is bound — that would defeat the async design
of the migration runtime.

### S6-P4 — Resetting the slot but not the node fields

If you reset the `Arc<Mutex<…>>` slot's contents (via
`*handle.lock().unwrap() = None;`) but **not** the node's
`whep_bound_port_*` fields:

- `refresh()` reads `None` from the slot, but the `if let Some(…)`
  branch isn't entered.
- The node's fields keep their previous (stale) values.

The fix: reset the node fields when the pipeline state-transitions
to `Stopped`, not when the slot transitions to `None`. The snippet
in §2.3 does this correctly:

```rust
if matches!(self.state, State::Stopped) {
    self.whep_bound_port_v4 = None;
    self.whep_bound_port_v6 = None;
}
```

### S6-P5 — Cloning the `Arc` and dropping it before the pipeline lives

```rust
// In build_live_pipeline (Step 4):
let bound_ports = Arc::new(Mutex::new(None));
{
    let bound_ports = bound_ports.clone();
    signaller.connect(name, false, move |vals| { /* writes */ });
}
// Forgot to put `bound_ports` on LiveDestinationPipeline.
// → only the cloned-into-closure ref survives.
```

The closure-captured clone keeps the writer alive. But the **reader**
side (`refresh()`) needs its own reference, stored on
`LiveDestinationPipeline.whep_bound_ports`. If you forget to populate
that field, the slot is written but never read. `getinfo` always
returns `None`.

Always populate the struct field:

```rust
LiveDestinationPipeline {
    pipeline,
    video_appsrc,
    audio_appsrc,
    whep_bound_ports: Some(bound_ports), // ← keep both refs alive
}
```

### S6-P6 — Using `Arc<RwLock<…>>` instead of `Arc<Mutex<…>>`

`RwLock` is preferable when reads vastly outnumber writes **and**
contention matters. Here, the writer fires once per session and the
reader runs at 100ms ticks. Lock contention is zero in practice.
`Mutex` is one fewer concept; **stick with `Mutex`**.

---

## 5. Next step

After this lands, [Step 7 — add unit tests](./MVP-PHASE-5-STEP-7-unit-tests.md)
adds host-runnable tests that exercise the dispatch-time creation,
`getinfo` round-trip, and the bound-port serialization. The
on-device smoke (verifying that `bound_port_v4` becomes `Some(_)`
after `Start`) lives in the parent doc's §3.3 — it requires
GStreamer init on the test host, which is out of scope for unit
tests.

# MVP-PHASE-5 — Step 4: wire the Whep arm in `build_live_pipeline`

> Part 4 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 3 — add bound-port fields on `DestinationNode`](./MVP-PHASE-5-STEP-3-destination-node-fields.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Build the GStreamer pipeline for the `Whep` destination at start
time:

```
video_appsrc ──► videoconvert ──► BaseWebRTCSink(with_signaller(WhepServerSignaller))
                                                 │
                                                 ▼
                                        on-server-started signal
                                        ──► Arc<Mutex<Option<(u16, u16)>>>
                                        (read back by refresh() in Step 6)
```

This is the **architectural heart** of PHASE-5. The arm:

1. Constructs a `WhepServerSignaller` instance.
2. Wires its `on-server-started` callback to a shared
   `Arc<Mutex<Option<(u16, u16)>>>` handoff slot.
3. Sets `server-port` on the signaller (from the JSON config).
4. Constructs `BaseWebRTCSink::with_signaller(signaller)`.
5. Sets the WHEP bitrate properties (`min/start/max-bitrate`,
   `enable-mitigation-modes`, `stun-server`, `video-caps`).
6. Adds the sink to the pipeline and links the video chain.

This is the largest step in PHASE-5 — roughly **80 lines of Rust**,
one match arm.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `DestinationNode::build_live_pipeline` (the outer function) | `senders/android/src/migration/nodes/destination.rs:489+` |
| `create_webrtcsink` (the closest template) | `sdk/mirroring_core/src/transmission.rs:343-401` |
| `WhepSink::new` Android path (the actual cast loop) | `sdk/mirroring_core/src/transmission.rs:475-528` |
| WHEP bitrate constants | `sdk/mirroring_core/src/transmission.rs:19-22` |
| `WhepServerSignaller::on-server-started` signal name | `sdk/mirroring_core/src/whep_signaller.rs:7` |
| `Self::make_element` helper | `senders/android/src/migration/nodes/destination.rs` (search for `fn make_element`) |
| `LiveDestinationPipeline.whep_bound_ports` (the handoff field) | added in Step 6 |
| Re-export of `WhepServerSignaller` into the migration crate | added in Step 5 |

### 1.2 Why this arm doesn't use `make_element`

The other destination arms construct sinks via
`gst::ElementFactory::make("udpsink", None)`. `BaseWebRTCSink` is
**not** a registered factory — it's a Rust type. The constructor is:

```rust
gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller(
    gst_rs_webrtc::signaller::Signallable::from(signaller),
);
```

This returns a `BaseWebRTCSink` instance directly. To add it to the
pipeline, upcast to `gst::Element`:

```rust
let sink_element: gst::Element = sink.upcast();
pipeline.add(&sink_element)?;
```

This is the **same pattern** used by `mcore::transmission::WhepSink::new`
(see `transmission.rs:475-528`).

### 1.3 Why the bitrate constants need a copy or re-export

`WHEP_MIN_BITRATE`, `WHEP_START_BITRATE`, `WHEP_MAX_BITRATE` live in
`sdk/mirroring_core/src/transmission.rs:19-22` and are crate-private
to `mcore`. The migration module needs them too. Two options:

- (a) Re-export them: add
  `pub use transmission::{WHEP_MIN_BITRATE, WHEP_START_BITRATE, WHEP_MAX_BITRATE};`
  to `sdk/mirroring_core/src/lib.rs`. Single source of truth. **Preferred.**
- (b) Duplicate them under
  `senders/android/src/migration/constants.rs`. Pragmatic if WHEP
  Android tuning later diverges.

The snippet in §2 below uses option (b) — the path
`crate::migration::constants::WHEP_*` is illustrative. If you pick
(a), substitute `mcore::WHEP_MIN_BITRATE` etc.

### 1.4 The bound-port handshake

`WhepServerSignaller` emits the bound IPv4 and IPv6 ports as a
two-`u32`-arg signal named `on-server-started` (constant
`ON_SERVER_STARTED_SIGNAL_NAME` at `whep_signaller.rs:7`). The
legacy cast-loop subscribes to it in `transmission.rs:349-386` and
forwards as `Event::SignallerStarted`.

For the migration runtime, the consumer is the cast-loop adapter
**in PHASE-6**, which polls `getinfo` for `bound_port_v4` /
`bound_port_v6`. The producer is the signal handler in this step,
which writes into a shared `Arc<Mutex<Option<(u16, u16)>>>` slot.

`refresh()` (extended in
[Step 6](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md))
reads the slot on each tick and mirrors the value into
`whep_bound_port_v4` / `whep_bound_port_v6` on the
`DestinationNode`.

---

## 2. The change

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match &self.family { … }` block at line 489):

```rust
DestinationFamily::Whep { server_port } => {
    // We need to forward the bound port back to the node via the
    // signaller's `on-server-started` signal. Use a shared
    // Arc<Mutex<Option<(u16, u16)>>> as the hand-off:
    use std::sync::{Arc, Mutex};
    let bound_ports: Arc<Mutex<Option<(u16, u16)>>> = Arc::new(Mutex::new(None));

    // The `crate::whep_signaller_compat` path is the Step 5 re-export
    // shim. After Step 5 lands, replace with:
    //     use mcore::whep_signaller::{WhepServerSignaller, ON_SERVER_STARTED_SIGNAL_NAME};
    let signaller = crate::whep_signaller_compat::WhepServerSignaller::default();

    {
        let bound_ports = bound_ports.clone();
        signaller.connect(
            crate::whep_signaller_compat::ON_SERVER_STARTED_SIGNAL_NAME,
            false,
            move |vals| {
                let p4 = vals.get(1).and_then(|v| v.get::<u32>().ok())? as u16;
                let p6 = vals.get(2).and_then(|v| v.get::<u32>().ok())? as u16;
                *bound_ports.lock().unwrap() = Some((p4, p6));
                None
            },
        );
    }
    signaller.set_property("server-port", *server_port as u32);

    let sink = gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller(
        gst_rs_webrtc::signaller::Signallable::from(signaller),
    );

    // Match the bitrate / mitigation / STUN configuration used by
    // mcore::transmission::WhepSink::new on Android
    // (sdk/mirroring_core/src/transmission.rs:475-528).
    sink.set_property("min-bitrate", crate::migration::constants::WHEP_MIN_BITRATE);
    sink.set_property("start-bitrate", crate::migration::constants::WHEP_START_BITRATE);
    sink.set_property("max-bitrate", crate::migration::constants::WHEP_MAX_BITRATE);
    sink.set_property_from_str("enable-mitigation-modes", "downsampled");
    sink.set_property_from_str("stun-server", "");
    sink.set_property("video-caps", gst::Caps::builder("video/x-vp8").build());

    let sink_element: gst::Element = sink.upcast();
    pipeline
        .add(&sink_element)
        .map_err(|err| format!("Failed to add basewebrtcsink to whep pipeline: {err:?}"))?;

    if let Some(appsrc) = video_appsrc.as_ref() {
        let vconv = Self::make_element("videoconvert", None)?;
        pipeline.add(&vconv).map_err(|err| {
            format!("Failed to add videoconvert to whep pipeline: {err:?}")
        })?;

        gst::Element::link_many(
            [appsrc.upcast_ref::<gst::Element>(), &vconv, &sink_element].as_slice(),
        )
        .map_err(|err| format!("Failed to link whep video chain: {err:?}"))?;
    }
    // (No audio chain — matches mcore::transmission::WhepSink::new's
    //  Android path, which is currently video-only.)

    // Stash the Arc on the live pipeline so refresh() can read it
    // back into self.whep_bound_port_v* on subsequent ticks.
    // (See Step 6 for the LiveDestinationPipeline.whep_bound_ports
    //  field and the refresh() logic.)
    //
    // The caller (the outer build_live_pipeline driver) returns the
    // populated LiveDestinationPipeline. Add `whep_bound_ports:
    // Some(bound_ports)` to that struct literal here. Snippet:
    //
    //     LiveDestinationPipeline {
    //         pipeline,
    //         video_appsrc,
    //         audio_appsrc,
    //         whep_bound_ports: Some(bound_ports),
    //     }
}
```

That's ~80 lines, one match arm.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Steps 1+2+3+4 (and Step 5 for the re-export shim, and Step 6
for the `whep_bound_ports` field), all `DestinationFamily` matches
are exhaustive again — expect **clean**.

If you commit Step 4 before Step 5, the compile will fail with
`unresolved import 'crate::whep_signaller_compat'`. **Land Steps 4
and 5 together,** or apply a one-line stub
`pub mod whep_signaller_compat { pub use mcore::whep_signaller::*; }`
during the transition.

If you commit Step 4 before Step 6, the compile will fail with
`missing field 'whep_bound_ports' in initializer of LiveDestinationPipeline`.
**Land Steps 4 and 6 together,** or add the field with a `#[allow(dead_code)]`
attribute in a preparatory commit.

### 3.2 On-device smoke

After Steps 1–6 all land (this is a cross-cutting smoke that
verifies the full pipeline):

```bash
# Pre-reqs:
#   - MIGRATION_COMMAND_BIND=127.0.0.1:8080 set on app startup.
#   - adb forward tcp:8080 tcp:8080.

# Create the WHEP destination on port 0 (OS picks free port).
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createdestination":{"id":"tv-1","family":{"Whep":{"server_port":0}},"audio":false,"video":true}}'

# Create a video generator as the source.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createvideogenerator":{"id":"gen-1"}}'

# Connect.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"connect":{"link_id":"L1","src_id":"gen-1","sink_id":"tv-1","audio":false,"video":true}}'

# Start the destination first (so the signaller is up before frames flow).
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"tv-1"}}'

# Wait ~500ms for on-server-started.
sleep 0.5

# Query the bound port.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"getinfo":{"id":"tv-1"}}' | jq '.nodes."tv-1".bound_port_v4'
# → expect: a non-zero port number (e.g. 54321), populated by the
#   on-server-started signal handler.

# Start the source.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"gen-1"}}'
```

Then point an FCast receiver (or a `gst-launch-1.0 whepsrc
uri=http://PHONE_IP:54321/...`) at the URL constructed from
`bound_port_v4`. The ball pattern should render within ~1s.

### 3.3 Grep recipe

```bash
grep -nA3 'DestinationFamily::Whep {' \
    senders/android/src/migration/nodes/destination.rs
# → expect: two matches —
#     1. from_family (Step 2): 2 element-name strings
#     2. build_live_pipeline (THIS step): real pipeline construction

grep -n 'on-server-started\|ON_SERVER_STARTED' \
    senders/android/src/migration/nodes/destination.rs
# → expect: 1 match (the signal connect in this step).
```

---

## 4. Pitfalls specific to this step

### S4-P1 — Constructing `WebRTCSink` instead of `BaseWebRTCSink`

```rust
let sink = gst_rs_webrtc::webrtcsink::WebRTCSink::new();
// ❌ Wrong type — this doesn't accept a custom signaller.
```

`WebRTCSink` is the high-level wrapper that uses
`gst_rs_webrtc::signaller::WebRTCSignaller` (the default WebRTC
peer-connect signaller, not WHEP). For WHEP, you need
**`BaseWebRTCSink::with_signaller(custom_signaller)`** — same
pattern as `mcore::transmission::WhepSink::new`.

### S4-P2 — Forgetting `.upcast()` on the sink

```rust
let sink: BaseWebRTCSink = /* … */;
pipeline.add(&sink)?;
// ❌ BaseWebRTCSink doesn't implement Borrow<gst::Element>.
```

`pipeline.add` takes `&gst::Element`. `BaseWebRTCSink` is a subclass
of `gst::Element` — but the Rust type system requires an explicit
`.upcast()` to convert:

```rust
let sink_element: gst::Element = sink.upcast();
pipeline.add(&sink_element)?;
```

This matches `mcore::transmission::create_webrtcsink` (`transmission.rs:343-401`).

### S4-P3 — Wrong cast in the signal closure

```rust
let p4 = vals.get(1).and_then(|v| v.get::<u32>().ok())? as u16;
```

The signal emits `(SELF, u32, u32)` — three values where `vals[0]`
is the signaller object, `vals[1]` is the IPv4 port, `vals[2]` is
the IPv6 port. **Indices 1 and 2, not 0 and 1.** The `as u16` is
required because GLib signals use `u32` for compatibility.

Also: returning `None` from the closure (the `?` operator) when the
signal arguments don't unpack as expected is the right error mode —
it silently fails the signal handler, which is preferable to
panicking the GStreamer thread.

### S4-P4 — Forgetting the `Arc::clone` before the move closure

```rust
let bound_ports: Arc<Mutex<…>> = Arc::new(Mutex::new(None));
signaller.connect(name, false, move |vals| {
    *bound_ports.lock().unwrap() = Some(...);  // ← moves bound_ports
    None
});
// Later: LiveDestinationPipeline { whep_bound_ports: Some(bound_ports) }
// ❌ Compile error: bound_ports was moved into the closure.
```

Always clone the `Arc` before the `move` closure:

```rust
{
    let bound_ports = bound_ports.clone();
    signaller.connect(name, false, move |vals| {
        *bound_ports.lock().unwrap() = Some(...);
        None
    });
}
// `bound_ports` (the original) is still owned by the outer scope.
```

### S4-P5 — Setting properties on `signaller` after `with_signaller`

```rust
let sink = BaseWebRTCSink::with_signaller(signallable);
sink.signaller().set_property("server-port", port);  // ❌ wrong order
```

After `with_signaller`, the signaller is owned by the sink and
accessed via `sink.signaller()` — which may return a different
reference (a wrapped clone). Set all signaller properties (including
`server-port` and the `on-server-started` connect) **before**
calling `BaseWebRTCSink::with_signaller(...)`. The snippet in §2
does this correctly.

### S4-P6 — `video-caps` with the wrong caps string

```rust
sink.set_property("video-caps", gst::Caps::builder("video/x-h264").build());
// ⚠️  WebRTC supports H.264 but the negotiation cost is higher and
//     hardware encoder availability varies. VP8 is the safer default.
```

`mcore::transmission::WhepSink::new` uses `video/x-vp8` for Android.
Match that to keep the WHEP receiver compatibility surface identical
to the legacy cast loop. H.264 support is a follow-up — explicitly
out of scope for PHASE-5.

### S4-P7 — Forgetting `stun-server: ""`

```rust
sink.set_property_from_str("stun-server", "");
```

WHEP doesn't require a STUN server (the receiver pulls the stream
over HTTP from the bound port). Setting `stun-server` to a non-empty
value adds an ICE-restart roundtrip with no benefit — and may fail
if the configured STUN server is unreachable.

The `mcore::transmission::WhepSink::new` Android path explicitly
sets `stun-server: ""`. Match it.

### S4-P8 — Linking before adding to the pipeline

```rust
gst::Element::link_many([appsrc, &vconv, &sink_element].as_slice())?;
pipeline.add(&vconv)?;
pipeline.add(&sink_element)?;
// ❌ Linking elements that aren't in a pipeline yet silently fails
//    or links them in a detached graph.
```

**Always `pipeline.add(...)?` before `link_many`.** The order matters
because GStreamer's `link_many` only links elements that share a
parent (the pipeline).

The snippet in §2 does `pipeline.add` first, then `link_many`. Match
that order.

---

## 5. Next step

After this lands, [Step 5 — re-export the signaller into the migration crate](./MVP-PHASE-5-STEP-5-signaller-reexport.md)
exposes `WhepServerSignaller` from `mcore` so the migration crate can
import it. Without Step 5, this step's
`crate::whep_signaller_compat::WhepServerSignaller` import fails to
resolve.

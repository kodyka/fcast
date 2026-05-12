# MVP-PHASE-5 — `Whep` destination family (Tier 1.2)

> **Second architectural unification step.** Today, the migration
> runtime can send video/audio to RTMP, UDP, local files, or local
> playback — but **not** to a WHEP receiver. This phase adds a `Whep`
> variant to `DestinationFamily` and wires `BaseWebRTCSink` +
> `WhepServerSignaller` into `nodes/destination.rs::build_live_pipeline`,
> mirroring exactly what the live cast loop already does in
> `mcore::transmission::WhepSink`.

---

## 0. Goal

Extend the migration runtime's destination node so that the
graph-command server accepts:

```json
{
  "createdestination": {
    "id": "tv-1",
    "family": { "Whep": { "server_port": 0 } },
    "audio": false,
    "video": true
  }
}
```

…and the runtime spins up a WHEP server (via `WhepServerSignaller`)
that an FCast receiver can pull a low-latency WebRTC stream from.

After this phase ships, the runtime can do everything the legacy
`WhepSink` cast loop does — but as a regular node in the graph. This
is the **prerequisite** for MVP-PHASE-6, which flips
`Event::StartCast` to issue graph commands instead of constructing
`WhepSink` directly.

This phase **does not** touch the existing `mcore::transmission::WhepSink`
call site (`lib.rs:943-950`). That continues to work in parallel until
MVP-PHASE-6.

---

## 1. Pre-flight

### 1.1 What already exists (do not re-implement)

| Component | Location |
|---|---|
| `BaseWebRTCSink` factory + signaller wiring | `sdk/mirroring_core/src/transmission.rs:343-401` (`create_webrtcsink`) |
| `WhepServerSignaller` Glib object | `sdk/mirroring_core/src/whep_signaller.rs:1-575` |
| `on-server-started` signal name | `sdk/mirroring_core/src/whep_signaller.rs:7` (`ON_SERVER_STARTED_SIGNAL_NAME`) |
| Bitrate constants | `sdk/mirroring_core/src/transmission.rs:19-22` (`WHEP_MIN_BITRATE` / `WHEP_START_BITRATE` / `WHEP_MAX_BITRATE`) |
| Live `WhepSink::new` (Android path) | `sdk/mirroring_core/src/transmission.rs:475-528` |
| Cast-loop `Event::SignallerStarted` handler | `senders/android/src/lib.rs:754-794` (receiver pulls a WHEP URL once the port is bound) |
| Existing destination variants | `senders/android/src/migration/protocol.rs:126-138` (`Rtmp / Udp / LocalFile / LocalPlayback`) |
| Existing `DestinationFamily::*` dispatch arms | `senders/android/src/migration/nodes/destination.rs:39-89` (`DestinationPipelineProfile::from_family`), `:489-836` (`build_live_pipeline`) |

### 1.2 What needs to change

| File | Edit |
|---|---|
| `senders/android/src/migration/protocol.rs` | Add `DestinationFamily::Whep { server_port }`. Update `DestinationInfo` consumers if necessary (it stores the family by value, so the new variant flows through for free). |
| `senders/android/src/migration/nodes/destination.rs` | Extend `DestinationPipelineProfile::from_family` (line 39) and `build_live_pipeline` (line 489) with a new arm that adds `BaseWebRTCSink` + signaller. Expose the bound port via the existing `last_error` / status channels OR a new `DestinationNode` field. |
| `senders/android/Cargo.toml` | Ensure `gst-rs-webrtc` is in the migration crate's dependency set — currently it's pulled in transitively via `mcore`, but the migration module imports it directly, so add a direct dep. |
| `senders/android/src/migration/node_manager.rs` | No new arm needed (it already routes through `create_destination`); but the **tests** at line 1196 (and around 1232 / 1254 / 1271 / 1288 / 1308) all hard-code `DestinationFamily::LocalPlayback` — leave those alone, add new `whep`-specific tests. |

Approximate scope: **~150–250 lines of Rust across 2 edited files**
plus 1 `Cargo.toml` line.

### 1.3 Why not "extend `Rtmp` with a `whep` boolean"?

Tempting (one fewer enum variant) but bad: WHEP has no flv mux, no
`location` URI, no AAC audio, and the bound port is **emitted as an
event after the signaller starts**. Modelling it as its own family
keeps the pipeline construction code linear and the JSON protocol
self-documenting.

### 1.4 The "bound port" handshake

`BaseWebRTCSink` doesn't know its port until the signaller is started
and the underlying `TcpListener` (in `whep_signaller.rs::imp::Signaller`)
binds. The signaller emits an `on-server-started` signal with two
`u32` values: bound IPv4 port and bound IPv6 port (`whep_signaller.rs:7,
349-373`). The legacy cast loop subscribes to that signal in
`transmission.rs:349-386` and forwards it as `Event::SignallerStarted`.

For the migration runtime, the receiver still needs that port (to
construct the WHEP URL it sends in the FCast `Play` message). MVP-PHASE-6
threads it back. **In this phase**, just stash the bound port on
`DestinationNode` and surface it via `DestinationInfo` — the cast-loop
adapter in PHASE-6 can read it via `getinfo`.

---

## 2. Steps

### 2.1 Step 1 — extend the JSON protocol

**File:** `senders/android/src/migration/protocol.rs` (lines 125-138):

```rust
// senders/android/src/migration/protocol.rs

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DestinationFamily {
    Rtmp {
        uri: String,
    },
    Udp {
        host: String,
    },
    LocalFile {
        base_name: String,
        max_size_time: Option<u32>,
    },
    LocalPlayback,

    // NEW —
    Whep {
        /// `0` = OS-picks-free-port. The bound port is emitted via
        /// `DestinationInfo.bound_port` after the signaller starts.
        #[serde(default)]
        server_port: u16,
    },
}
```

Then extend `DestinationInfo` (lines 151-160) so the bound port is
visible via `getinfo`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct DestinationInfo {
    pub family: DestinationFamily,
    pub audio_slot_id: Option<String>,
    pub video_slot_id: Option<String>,
    pub cue_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub state: State,

    // NEW — populated only for `DestinationFamily::Whep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v4: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_port_v6: Option<u16>,
}
```

The `#[serde(default)]` + `skip_serializing_if = "Option::is_none"`
combo keeps the wire format backward-compatible for the four existing
non-WHEP variants.

### 2.2 Step 2 — extend the pipeline profile

**File:** `senders/android/src/migration/nodes/destination.rs` (lines
35-105, `DestinationPipelineProfile::from_family`):

```rust
impl DestinationPipelineProfile {
    fn from_family(family: &DestinationFamily, audio: bool, video: bool) -> Self {
        let mut elements = Vec::new();

        match family {
            DestinationFamily::Rtmp { .. } => { /* …existing… */ }
            DestinationFamily::Udp { .. } => { /* …existing… */ }
            DestinationFamily::LocalFile { .. } => { /* …existing… */ }
            DestinationFamily::LocalPlayback => { /* …existing… */ }

            // NEW —
            DestinationFamily::Whep { .. } => {
                elements.extend([
                    "videoconvert",
                    "basewebrtcsink", // illustrative — the real factory
                                      // is gst_rs_webrtc::webrtcsink::BaseWebRTCSink
                                      // constructed in Rust, not by name.
                ]);
                // WHEP currently sends video-only (matching the live
                // cast loop in transmission.rs:475-528). Audio support
                // is a follow-up; keep the `audio` flag honored but
                // emit no audio elements.
                let _ = audio;
            }
        }

        if !audio {
            elements.retain(|el| !el.contains("audio"));
        }
        if !video {
            elements.retain(|el| !el.contains("video") && !el.contains("h264"));
        }

        Self {
            family: family.clone(),
            elements: elements.into_iter().map(str::to_string).collect(),
            wait_for_eos_on_stop: true,
            stage: DestinationPipelineStage::Idle,
        }
    }
}
```

The element-name list is just for debug/printing — the actual
`BaseWebRTCSink` is instantiated by Rust constructor, not
`make_element("basewebrtcsink")`. (`BaseWebRTCSink` does not have a
factory name accessible via `ElementFactory::make`; it's a Rust
struct.)

### 2.3 Step 3 — add fields on `DestinationNode`

**File:** `senders/android/src/migration/nodes/destination.rs` (lines
107-121):

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

Update the `DestinationNode::new` constructor (line 128) and all
`Default`/test sites accordingly. The `to_info` method (search for
`DestinationInfo {`) gets the two extra fields wired:

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

### 2.4 Step 4 — wire the Whep arm in `build_live_pipeline`

**File:** `senders/android/src/migration/nodes/destination.rs`
(extend the `match &self.family { … }` block at line 489):

```rust
DestinationFamily::Whep { server_port } => {
    // We need to forward the bound port back to the node via the
    // signaller's `on-server-started` signal. Use a shared
    // Arc<Mutex<Option<(u16, u16)>>> as the hand-off:
    use std::sync::{Arc, Mutex};
    let bound_ports: Arc<Mutex<Option<(u16, u16)>>> = Arc::new(Mutex::new(None));

    let signaller = crate::whep_signaller_compat::WhepServerSignaller::default();
    // (See §2.5 for why this path is "compat" — the signaller lives
    // in the `mcore` crate today and we re-export it.)

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
    // (Use a `bound_ports` field on `LiveDestinationPipeline` —
    // see §2.6.)
}
```

The bitrate constants (`WHEP_MIN_BITRATE` etc.) currently live in
`sdk/mirroring_core/src/transmission.rs:19-22` as crate-private. The
migration module needs them too — either:

- (a) Re-export them: add `pub use transmission::{WHEP_MIN_BITRATE, …};`
  to `sdk/mirroring_core/src/lib.rs`, then `use mcore::{WHEP_MIN_BITRATE, …};`
  from the migration module. **Preferred** — single source of truth.
- (b) Duplicate them under a new `senders/android/src/migration/constants.rs`.
  Pragmatic if we later want WHEP-specific Android tuning.

The snippet above assumes (b) for clarity.

### 2.5 Step 5 — re-export the signaller into the migration module

`WhepServerSignaller` lives in `sdk/mirroring_core/src/whep_signaller.rs`
which is **not** a public module of `mcore`. Two options:

- (a) Expose it: add `pub mod whep_signaller;` to
  `sdk/mirroring_core/src/lib.rs` (currently a private `mod` if at all,
  but it's compiled into the crate already as it's used by
  `transmission.rs`). Then the migration module imports
  `mcore::whep_signaller::WhepServerSignaller`.
- (b) Move it: relocate `whep_signaller.rs` into a new
  `crates/whep-signaller/` crate and have both `mcore` and the
  migration runtime depend on it.

(a) is the minimum diff for this phase. (b) is the right end-state
once MVP-PHASE-6 has migrated cast.

For this phase, do (a):

```rust
// sdk/mirroring_core/src/lib.rs

mod whep_signaller;            // ← was: private mod
pub mod whep_signaller {       // ← NEW: re-expose.
    pub use super::whep_signaller_inner::*;
}
```

…and then in the migration crate:

```rust
// senders/android/src/migration/nodes/destination.rs (top)

use mcore::whep_signaller::{WhepServerSignaller, ON_SERVER_STARTED_SIGNAL_NAME};
```

This is the only place in this phase where you touch the SDK crate.
**Keep that change to a single `pub mod` re-export.**

### 2.6 Step 6 — extend `LiveDestinationPipeline` to carry the port handle

**File:** `senders/android/src/migration/nodes/destination.rs` (line
22-27):

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

Then in `refresh()` (or wherever `poll_bus_messages` is called from —
search for `fn refresh`):

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
    Ok(())
}
```

After this, a downstream consumer (the MVP-PHASE-6 cast-loop adapter)
can poll `getinfo` until `bound_port_v4` is `Some(_)` and then use it
to construct the WHEP URL — replacing the legacy
`Event::SignallerStarted` callback flow.

### 2.7 Step 7 — add unit tests

**File:** `senders/android/src/migration/node_manager.rs`
(in the `#[cfg(test)] mod tests` block):

```rust
#[test]
fn create_whep_destination_succeeds() {
    let mut manager = NodeManager::default();
    let result = manager.dispatch(Command::CreateDestination {
        id: "tv-1".into(),
        family: DestinationFamily::Whep { server_port: 0 },
        audio: false,
        video: true,
    });
    assert!(matches!(result, CommandResult::Success));
    assert!(manager.nodes.contains_key("tv-1"));
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
    // Before Start, the bound port is None.
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
```

These don't require GStreamer to be initialised — they validate the
command-dispatch and `DestinationInfo` shape only. A pipeline-level
smoke test (verifying that `bound_port_v4` becomes `Some(_)` after
`Start`) is **not** in scope here because it requires GStreamer init
on the test host; defer to the on-device smoke in §3.3.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. Most likely failures:

- "no field `whep_bound_port_v4` on `DestinationNode`" — you forgot
  to thread the new fields through every constructor / test stub.
- "unresolved import `mcore::whep_signaller`" — Step 5 not applied.
- "function `set_property_from_str` not in scope" — `gst::prelude::*`
  not in scope at the top of the file (it is, at line 3 — but
  double-check).

### 3.2 Unit tests

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::tests::create_whep_destination_succeeds \
    migration::node_manager::tests::whep_destination_info_carries_optional_bound_ports
```

Both green.

### 3.3 On-device smoke

Pre-req: MVP-PHASE-3 verified the migration runtime command server
is reachable via `MIGRATION_COMMAND_BIND=127.0.0.1:8080` +
`adb forward tcp:8080 tcp:8080`.

```bash
# 1. Create a video generator (synthesizes a ball pattern).
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createvideogenerator":{"id":"gen-1"}}'
# → {"id":null,"result":"success"}

# 2. Create a WHEP destination on a random port.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createdestination":{"id":"tv-1","family":{"Whep":{"server_port":0}},"audio":false,"video":true}}'
# → {"id":null,"result":"success"}

# 3. Connect them.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"connect":{"link_id":"L1","src_id":"gen-1","sink_id":"tv-1","audio":false,"video":true}}'
# → {"id":null,"result":"success"}

# 4. Start the destination.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"start":{"id":"tv-1"}}'
# → {"id":null,"result":"success"}

# 5. Poll getinfo until bound_port_v4 is populated.
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' \
     | jq '.result.info.nodes."tv-1"'
# → { "state": "started", "kind": "destination",
#     "family": { "Whep": { "server_port": 0 } },
#     "bound_port_v4": 39871,
#     "bound_port_v6": 39872, … }
```

Then on a separate host with `gst-play-1.0` available:

```bash
# Construct the WHEP URL — match the format mcore uses to send to FCast.
# (See sdk/mirroring_core/src/transmission.rs / tx_sink.get_play_msg
#  for the canonical shape.)

DEVICE_IP=$(adb shell ip route | awk '/wlan|rmnet/ {print $9; exit}')
WHEP_PORT=$(curl -s -X POST http://127.0.0.1:8080/command \
            -d '{"getinfo":{}}' \
            | jq -r '.result.info.nodes."tv-1".bound_port_v4')

# Open the WHEP endpoint in a WebRTC client (gst-webrtc, OBS, or any
# WHEP-capable player). The ball pattern should appear within ~1s.
echo "WHEP endpoint: http://${DEVICE_IP}:${WHEP_PORT}/endpoint"
```

If the ball pattern flows, this phase is **done**.

---

## 4. Common pitfalls

### P1 — `BaseWebRTCSink` is not a GStreamer-registered factory

```rust
gst::ElementFactory::make("basewebrtcsink").build()  // ← BAD: returns Err
```

It's a Rust subclass that has to be constructed via
`BaseWebRTCSink::with_signaller(...)`. The element-name in the
`DestinationPipelineProfile` is purely diagnostic.

### P2 — Re-exporting `whep_signaller` via `pub mod` vs `pub use`

```rust
// sdk/mirroring_core/src/lib.rs

mod whep_signaller;                           // private — current state
pub use whep_signaller::WhepServerSignaller;  // OK if you also re-export const
pub use whep_signaller::ON_SERVER_STARTED_SIGNAL_NAME;
```

is **not** the same as `pub mod whep_signaller;` — the latter exposes
the whole module path. Pick one and stick with it; for this phase,
`pub mod whep_signaller;` is the lower-friction choice.

### P3 — The `on-server-started` closure must outlive the signaller

`signaller.connect(...)` takes a closure with `'static` lifetime. The
`bound_ports: Arc<Mutex<Option<(u16, u16)>>>` shared with the closure
must be cloned **before** moving into the closure (`bound_ports.clone()`).
Otherwise the borrow checker rejects the move. The example in §2.4
does this — don't simplify it away.

### P4 — `server_port: 0` returns OS-picked port; non-zero returns the explicit port (or fails)

The signaller honours `server-port` literally:
- `0` → pick a free port at bind time (matching `TcpListener::bind(("...", 0))`
  semantics).
- non-zero → attempt to bind exactly that port; fails if taken.

For tests and on-device smoke, use `0`. For production with NAT
forwarding rules, use the configured port.

### P5 — `last_caps` cache races on first frame

The cast loop has a `caps = None::<gst::Caps>` cache (see
`lib.rs:890-934`) that pushes new caps onto `appsrc` only when they
change. The migration runtime's `StreamBridge` already does this
fanout (`media_bridge.rs:39-44`, `last_caps`), so you do **not** need
to duplicate the caps cache in this destination. The first frame
arriving on the `video_appsrc` from `StreamBridge` already has caps
applied.

### P6 — Audio is intentionally not wired

The legacy cast loop (`transmission.rs:475-528`) is **video-only** on
Android. This phase mirrors that. Wiring an audio chain (`audiotestsrc`
or pipewire) is a separate follow-up and depends on
`MainActivity.startScreenCapture`'s `MediaProjection.AudioCaptureSource`
plumbing — out of scope.

### P7 — Bitrate constants must be re-exported, not hard-coded

Don't inline `WHEP_MIN_BITRATE = MEGA_BIT / 2` in the migration
module — if `mcore` ever retunes these, the two cast paths diverge.
Step 5 + 6 above prefer re-exporting from `mcore`.

---

## 5. Stop conditions

The phase is "done" when:

1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. The two unit tests in §3.2 pass.
3. The on-device smoke in §3.3:
   - `getinfo` returns `family: { "Whep": { "server_port": 0 } }`.
   - `bound_port_v4` and `bound_port_v6` are `Some(<port>)` after
     `start`.
   - A WHEP-capable player connecting to
     `http://<device-ip>:<bound_port>/endpoint` receives the ball
     pattern from the `gen-1 → tv-1` graph.
4. New surface area is visible to:

```bash
grep -n 'DestinationFamily::Whep\|whep_bound_port\|whep_signaller_compat' \
    senders/android/src/migration/
# → expect: protocol.rs, nodes/destination.rs
```

5. **No MVP cast-path change happens in this phase.** The existing
   screen-mirror cast loop (`Event::StartCast` → `WhepSink::new` →
   `BaseWebRTCSink`) is untouched. That handover is MVP-PHASE-6.

---

## 6. Why this matters

This phase teaches the migration runtime to do the **one thing it
couldn't do before**: speak WHEP. Combined with MVP-PHASE-4
(screen-capture source), the runtime can now construct the entire
"phone screen → TV" pipeline as a 3-node graph:

```
ScreenCapture(cap-1) ─link L1─▶ Destination::Whep(tv-1)
```

MVP-PHASE-6 then makes the cast loop *issue those four commands* on
`Event::StartCast` instead of constructing the pipeline by hand. After
MVP-PHASE-6 ships, `mcore::transmission::WhepSink` becomes a candidate
for deletion (or for being thinned down to the desktop-only
`#[cfg(not(target_os = "android"))]` paths).

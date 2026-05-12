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

## 2. Steps — split into seven per-step files

To keep each step skimmable and reviewable in isolation, the
implementation is split across seven per-step `MVP-PHASE-5-STEP-N-*.md`
files. Each file follows the same smaller five-section template
(Goal-of-this-step / Pre-flight / The change / Verification /
Next step) and is self-contained — you don't need to flip back to
this parent doc while implementing a single step.

| # | File | Scope | Net diff |
|---|---|---|---|
| 1 | [`MVP-PHASE-5-STEP-1-protocol-extension.md`](./MVP-PHASE-5-STEP-1-protocol-extension.md) | Add `Whep { server_port }` to `DestinationFamily`; add `bound_port_v4` / `bound_port_v6` to `DestinationInfo`. Backward-compatible wire format. | ~30 lines, 1 file (`protocol.rs`) |
| 2 | [`MVP-PHASE-5-STEP-2-pipeline-profile.md`](./MVP-PHASE-5-STEP-2-pipeline-profile.md) | Extend `DestinationPipelineProfile::from_family` with a `Whep` arm (diagnostic element listing). | ~15 lines, 1 file (`nodes/destination.rs`) |
| 3 | [`MVP-PHASE-5-STEP-3-destination-node-fields.md`](./MVP-PHASE-5-STEP-3-destination-node-fields.md) | Add `whep_bound_port_v4` / `whep_bound_port_v6` fields to `DestinationNode`. Plumb through `::new()` and `as_info()`. | ~15 lines, 1 file (`nodes/destination.rs`) |
| 4 | [`MVP-PHASE-5-STEP-4-build-live-pipeline.md`](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) | Wire the `Whep` arm into `DestinationNode::build_live_pipeline`. Construct `WhepServerSignaller`, wire `on-server-started` to the `Arc<Mutex<…>>` slot, instantiate `BaseWebRTCSink::with_signaller`, link the video chain. **Largest step.** | ~80 lines, 1 file (`nodes/destination.rs`) |
| 5 | [`MVP-PHASE-5-STEP-5-signaller-reexport.md`](./MVP-PHASE-5-STEP-5-signaller-reexport.md) | Flip `mod whep_signaller;` to `pub mod whep_signaller;` in `mcore::lib.rs`. Add a 1-line `whep_signaller_compat` shim in the migration crate. | 1 SDK line + 1 new shim file |
| 6 | [`MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md`](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md) | Add `whep_bound_ports: Option<Arc<Mutex<Option<(u16, u16)>>>>` to `LiveDestinationPipeline`. Extend `refresh()` to read the slot into the node's bound-port fields and reset on `Stopped`. | ~30 lines, 1 file (`nodes/destination.rs`) |
| 7 | [`MVP-PHASE-5-STEP-7-unit-tests.md`](./MVP-PHASE-5-STEP-7-unit-tests.md) | ~12 host-runnable unit tests across `protocol.rs`, `node_manager.rs`, and `nodes/destination.rs`. No GStreamer init required (optional gated tests for the `refresh()` slot read). | ~150 lines of tests across 3 files |

### Recommended landing order

```
Step 1 ──► Step 2 ──► Step 3 ──► Step 4 ──┐
                                          ├── single squash-commit
                                          ▼   (compile stays clean once
                                Step 5 ──►    Steps 4+5+6 are all in)
                                          │
                                Step 6 ───┘
                                          │
                                          ▼
                                       Step 7 (unit tests — green after Step 6)
```

**Steps 1+2+3** can land independently (each is additive and
backward-compatible).
**Step 4** depends on Step 5 (the `crate::whep_signaller_compat`
import) and Step 6 (the `whep_bound_ports` field on
`LiveDestinationPipeline`). The cleanest path is squashing Steps
4+5+6 into one commit so the tree compiles between commits.
**Step 7** is test-only and lands after the runtime changes.

---

## 2b. Why the per-step split?

The original monolithic §2 block ran to ~380 lines with seven
sub-steps interleaved. Splitting it gives:

- Per-step files small enough to review on a phone screen.
- Independent verification recipes per step (each step's §3 covers
  only that step's compile/test/grep checks).
- Step-specific pitfalls without scrolling past unrelated content.
- Easy follow-up PRs: if a reviewer asks for changes on Step 4
  only, you edit one file.

The pattern mirrors the per-step split applied to PHASE-8 in the
same PR.

---

> **Looking for inline §2.1 — §2.7?** The per-step content has
> moved into the seven `MVP-PHASE-5-STEP-N-*.md` files listed in
> the table above. Each STEP file is self-contained — Goal,
> Pre-flight, The change, Verification, and Pitfalls for that
> step alone.

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

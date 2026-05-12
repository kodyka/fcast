# DRAFT MVP — Step-by-step implementation guide

> **Audience:** developer who wants the *smallest possible* set of changes to
> get a working **Android sender → FCast receiver** screen-mirror loop —
> i.e. the user opens the app, picks a receiver, hits Cast, grants the
> MediaProjection permission, and sees their phone screen on the receiver.
>
> **Goal:** end-to-end mirroring works on a real device. Everything else is
> a follow-up.
>
> **Out of scope:** every Phase-12-through-27 UI sub-page (audio settings,
> camera settings, bitrate presets, recording, macros, debug log…) is
> *cosmetic* relative to the MVP. They make the app feel finished but they
> do not affect whether mirroring works.

This guide is the **MVP** roadmap: the shortest path from "app builds and
launches" to "screen is on the TV". It complements:

- the Phase-8 split (`PHASE-8-Section-*.md`) — Bridge / Slint surface
  completeness;
- the 19 reimplement guides (`PHASE-9` … `PHASE-27`) — page-by-page UI
  completeness;
- the existing Phase-8 strategy / spec docs (`PHASE-8-bridge-migration-plan.md`,
  `PHASE-8-rust-bridge.md`).

It is a **doc-only** guide: it tells you what to change in the existing
tree, with `file:line` citations and code snippets. It does not modify the
codebase itself.

This is the **post-Phase-8 rewrite**: Phase 8 is now landed on `master`
(15+ Bridge clusters wired), and the functional **migration runtime** in
`senders/android/src/migration/` is fully shipped as a parallel node-graph
API. The MVP framing is updated to reflect both.

---

## 0. TL;DR

After Phase 8 (Clusters F + A1–A5 + B1–B5 + C1/C2/C4/C5 + D1/D2 + E)
landed on `master`, **the live state is**:

| Surface | State | What ships today |
|---|---|---|
| **Bridge.slint globals** | wired by Phase 8 | `status-items`, `app-version`, `network-interfaces`, `recording-state` + `recording-elapsed-s`, `log-entries`, `banner-*`, `macros` + draft macro state, `quick-actions`, `history`, `presets`, `wifi-aware-enabled`, lifecycle, audio/camera/resolution/framerate selectors, panel routing. `senders/android/ui/bridge.slint` |
| **Screen-mirror cast loop** (Android → WHEP receiver) | **blocked by one Slint placeholder** | Rust state machine + JNI + MediaProjection + OpenGL YUV conversion + `appsrc` → `BaseWebRTCSink` + `WhepServerSignaller` + FCast `LoadRequest::Url` are all wired. Phase 8 did **not** touch `pages/connect_page.slint` — the device list still iterates `mock-devices` and the `clicked` handler is still a placeholder. |
| **Migration runtime** (node-graph media API) | **functional, shipped, parallel** | `start_graph_runtime()` is called during `run_event_loop`; HTTP + JNI + direct-Rust entry points all live; refresh tick runs every 100 ms; debug smoke quick-actions verify the end-to-end command flow. `senders/android/src/migration/runtime.rs` |

**The MVP is one cluster, not five.** The 757-line pre-Phase-8 draft of
this guide called out M1–M5; M2–M5 are now subsumed by Phase 8. **Only M1
remains**:

> `senders/android/ui/pages/connect_page.slint` line 86:
> ```slint
> clicked => {
>     /* placeholder: would call connect-receiver(device.address) */
> }
> ```
> Rust pushes real receivers to `Bridge.devices` (`bridge.slint:145`,
> via `update_receivers_in_ui` at `lib.rs:659-680`) and FCast SDK is
> connected through the `on_connect_receiver` callback (`lib.rs:1800`).
> But the connect-page row
> iterates `root.mock-devices` (`connect_page.slint:69, 72`) and never
> invokes `Bridge.connect-receiver(...)`. Tapping a real receiver does
> nothing. Everything downstream of that callback is already wired.

Total MVP diff: **~10 lines** in one Slint file.

The migration runtime is **not** on the screen-mirror cast path today (no
`MediaProjection` `SourceNode` variant; no WHEP `DestinationFamily`
variant) — it services a complementary surface (URL/file → mixer →
RTMP/UDP/LocalFile/LocalPlayback) and is reachable from Java via
`MainActivity.graphCommand(...)` (which calls `nativeGraphCommand` →
`migration::runtime::try_handle_command_json`) and from a debug HTTP
server gated by `MIGRATION_COMMAND_BIND`.

---

## 1. What "MVP" means

**Definition.** *One Android device casts its screen to one FCast receiver
on the local network, end-to-end, with no manual intervention beyond
granting the MediaProjection permission.*

### 1.1 What the user must be able to do

| Step | User action | Live behaviour required |
|---|---|---|
| 1 | Launches app | Connect page shows; receiver discovery starts |
| 2 | Sees their TV listed | A `ReceiverItem` row appears within ~5 s of opening the app |
| 3 | Taps the receiver | App transitions to `Connecting` → `SelectingSettings` |
| 4 | Confirms resolution / framerate | App requests MediaProjection consent |
| 5 | Grants consent | App transitions to `Casting`; phone screen renders on TV |
| 6 | Taps Stop | Cast ends cleanly; app returns to `Disconnected` |

### 1.2 What is **explicitly out of scope** for MVP

| Surface | Why deferred |
|---|---|
| Audio settings (Phase 14) | The MVP cast path uses default `audio: true, video: true`. |
| Camera capture (Phase 15) | Screen mirror is the MVP source. |
| Bitrate presets (Phase 16) | `BaseWebRTCSink` selects encoders internally. |
| Local recording (Phase 23) | Independent destination. |
| Macros (Phase 25) | UI sugar over Bridge callbacks. |
| Pairing / QR / receiver mgmt (Phase 24) | Optional; mDNS auto-discovery already works. |
| Debug log / debug video (Phases 18, 21) | Diagnostic surfaces. |
| Backup-and-reset, cast history (Phase 19, 20) | Storage surfaces. |
| **Migration runtime → WHEP unification** | Architectural follow-up (see §8). |
| **Migration runtime → MediaProjection source** | Architectural follow-up (see §8). |

### 1.3 Why the MVP is **one cluster** and not five

The pre-Phase-8 version of this guide called out five clusters M1–M5:

- **M1** — `connect_page.slint` → `Bridge.connect-receiver(...)`. **Still
  live.** Phase 8 did not touch this file.
- **M2** — push `Bridge.status-items` on `Event::CaptureStarted`. **Done by
  Phase 8 / Cluster A1.** `Bridge.status-items` is declared on
  `bridge.slint:149` and consumed by `components/status_overlay.slint`.
- **M3** — MediaProjection consent denial → state rollback. **Already wired.**
  See `senders/android/src/lib.rs:734-925`.
- **M4** — Stop button → `EndSession{disconnect:true}` → clean rollback.
  **Already wired** at `bridge.slint:237`, `lib.rs:1822`,
  `lib.rs:682` (`stop_cast`).
- **M5** — `Bridge.app-version` push from `env!("CARGO_PKG_VERSION")`. **Done
  by Phase 8 / Cluster A2.** `bridge.slint:151`.

So **the MVP guide collapses to M1** plus a verification pass over M2–M5.

---

## 2. The two functional surfaces

After Phase 8 the Android sender has **two complementary functional
surfaces**, both shipped on `master`. The MVP cluster only needs to
unblock Surface A. Surface B is already running and only needs use-case
adoption.

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Surface A — Screen-mirror cast path (MVP target)                       │
│                                                                         │
│   Slint UI ── on_connect_receiver ──▶ Application::handle_event         │
│                                       │                                 │
│                                       ▼                                 │
│                       Event::ConnectToDevice(name)                      │
│                            ↓                                            │
│                     FCast SDK device.connect()                          │
│                            ↓                                            │
│              Event::CaptureStarted → Bridge.status-items push           │
│                            ↓                                            │
│   Java MainActivity.startScreenCapture(w,h,fps)  (via JNI)              │
│                            ↓                                            │
│      MediaProjection consent → onActivityResult                         │
│                            ↓                                            │
│      VirtualDisplay + OpenGL ES (RGBA→YUV420 planar shader)             │
│                            ↓                                            │
│      nativeProcessFrame(w, h, Y, U, V)  ── JNI ──▶ Rust process_frame   │
│                            ↓                                            │
│      FRAME_PAIR (Mutex<Option<VideoFrame>> + Condvar)                   │
│                            ↓                                            │
│      appsrc.set_callbacks(need-data) → push_buffer                      │
│                            ↓                                            │
│      BaseWebRTCSink (gst-plugins-rs, internal MediaCodec enc selection) │
│                            ↓                                            │
│      WhepServerSignaller binds → Event::SignallerStarted{port_v4/v6}    │
│                            ↓                                            │
│      device.load(LoadRequest::Url{ ... }) ─── over FCast TCP protocol   │
│                            ↓                                            │
│      Receiver opens WHEP, decodes, renders on TV                        │
└─────────────────────────────────────────────────────────────────────────┘
```

```
┌─────────────────────────────────────────────────────────────────────────┐
│  Surface B — Migration runtime (parallel, post-MVP integration target)  │
│                                                                         │
│   ┌──────────────────────┐    ┌──────────────────────┐                  │
│   │  Java               │    │  Debug HTTP server   │                  │
│   │  MainActivity.      │    │  (env-gated:         │                  │
│   │  graphCommand(...)  │    │   MIGRATION_COMMAND_ │                  │
│   │   ↓                 │    │   BIND)              │                  │
│   │  nativeGraphCommand │    │   ↓                  │                  │
│   │   (JString)         │    │  POST /command       │                  │
│   └─────────┬────────────┘    └──────────┬───────────┘                  │
│             │                            │                              │
│             ├────────────┬───────────────┘                              │
│             ▼            ▼                                              │
│   migration::runtime::handle_command_json(&payload)                     │
│             ↓                                                           │
│   InboundCommand (ControllerMessage | bare Command)                     │
│             ↓                                                           │
│   GRAPH_NODE_MANAGER.lock().dispatch(command)                           │
│             ↓                                                           │
│   ┌─────────────────────────────────────────────────────────────────┐   │
│   │ NodeManager state:                                              │   │
│   │   nodes:           HashMap<String, NodeRecord>                  │   │
│   │   links:           HashMap<String, LinkRecord>                  │   │
│   │   media_bridges:   HashMap<String, StreamBridge>                │   │
│   │                                                                 │   │
│   │ NodeRecord variants:                                            │   │
│   │   Source           — fallbacksrc / uridecodebin (URL/file in)  │   │
│   │   Destination      — RTMP / UDP / LocalFile / LocalPlayback     │   │
│   │   Mixer            — compositor + audiomixer                    │   │
│   │   VideoGenerator   — videotestsrc (ball pattern)                │   │
│   └─────────────────────────────────────────────────────────────────┘   │
│             ↓                                                           │
│   Refresh thread tick() every 100 ms:                                   │
│     - refresh_nodes() — drive each node's state machine                 │
│         (Initial → Starting → Started → Stopping → Stopped)             │
│     - sync_media_links() — wire StreamBridges between sibling pipelines │
│                                                                         │
│   StreamBridge = single appsink producer → many appsrc consumers,       │
│                  with caps caching + EOS propagation + stale removal.   │
└─────────────────────────────────────────────────────────────────────────┘
```

Surface A and Surface B run in the same Rust process but do not share
GStreamer elements today. Unifying them (so the Android screen-mirror
loop also flows through the node graph as a custom `SourceNode` and a
WHEP `DestinationFamily`) is the **first architectural follow-up** after
MVP — see §8.

---

## 3. How the layers plug together

A 13-step walkthrough that maps the Android-sender architecture diagram
to the live code. Every step cites `file:line` so you can follow each
transition.

> Sources for the layer model: `senders/android/src/lib.rs`, `senders/android/ui/main.slint`, `senders/android/ui/bridge.slint`, `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java`.

### 3.1 Application startup
**Entry:** `android_main()` in `senders/android/src/lib.rs`.

1. Android logger initializes.
2. Slint Android platform initializes with the Android app context.
3. The Slint `MainWindow` is created from the `ui/*.slint` definitions.
4. Quick actions (settings, debug, codec-test, scan-qr, record, pair,
   bitrate) are seeded via `default_quick_actions()` at
   `senders/android/src/lib.rs:1070-1089`.
   In debug builds four extra quick-actions are appended:
   `migrated-server`, `test-getinfo`, `test-crossfade`, `test-smoke` —
   these drive Surface B from the UI for smoke testing.
5. The Tokio runtime spawns the `Application` task.
6. UI event loop starts via `ui.run()`.

### 3.2 Rust application initialization
**Entry:** `Application::new()` / `run_event_loop()` in `senders/android/src/lib.rs`.

1. Global event channel forwarding thread is set up.
2. `CastContext` is created for FCast device management.
3. Device map and active device tracking initialize.
4. The event loop is run by `run_event_loop()` at
   `senders/android/src/lib.rs:1025-1058`.

### 3.3 Event loop and runtime initialization
**Inside `run_event_loop()`:**

1. `tracing_gstreamer::integrate_events()` enables GStreamer tracing.
2. `ensure_gstreamer_initialized()` initialises GStreamer.
3. **`migration::runtime::start_graph_runtime()`** is called
   (`lib.rs:1035`):
   - `NodeManager` starts.
   - The 100 ms refresh thread spawns (`runtime.rs:51-58`).
   - If `MIGRATION_COMMAND_BIND` is set, the HTTP command-server thread
     spawns.
4. The main loop drains `event_rx` and dispatches into `handle_event`.
5. On shutdown, `migration::runtime::shutdown_graph_runtime()` runs
   (`lib.rs:1053`) — stops the HTTP server, stops the refresh thread,
   tears down all nodes / links / media bridges.

### 3.4 Device discovery (mDNS)
**Java side:** `FCastDiscoveryListener.java` → JNI callback.

1. Android NSD scans for FCast receivers.
2. When found, JNI callback
   `Java_org_fcast_android_sender_FCastDiscoveryListener_serviceFound`
   runs at `senders/android/src/lib.rs:2125-2225`.
3. Rust receives device info (name, addresses, port).
4. `Event::DeviceAvailable(device_info)` is sent (`lib.rs:2222`).
5. `handle_event` calls `add_or_update_device(...)` (`lib.rs:676-680`).
6. UI is updated via `update_receivers_in_ui()` (`lib.rs:659-674`) —
   pushes the device *name* into `Bridge.devices` (`bridge.slint:145`).

### 3.5 User connects to receiver — **the MVP gap**
**UI → Rust:** Slint `Bridge.connect-receiver` callback (`bridge.slint:235`).

1. User taps a receiver row on the connect page.
2. *(MVP gap)* `pages/connect_page.slint:85-88` should invoke
   `Bridge.connect-receiver(device.name)`. It currently does **not** —
   the body is a `/* placeholder ... */` comment.
3. Once fixed, `on_connect_receiver` (`lib.rs:1800-1810`) sends
   `Event::ConnectToDevice(device_name)`.
4. `handle_event` matches `Event::ConnectToDevice(...)` at `lib.rs:747`
   and calls `connect_with_device_info(...)` (`lib.rs:711`).
5. The FCast device is created, `DeviceHandler` callback is wired, and
   `active_device` is set.
6. UI state transitions to `AppState::Connecting` (`bridge.slint:36`).

### 3.6 Connection established and settings selection

1. FCast SDK handshake completes.
2. `DeviceEvent::StateChanged(Connected)` fires.
3. UI transitions to `AppState::SelectingSettings`.
4. User picks resolution / framerate (Phase-7 settings panel) — already
   wired to `Bridge.resolution-idx` and `Bridge.framerate-idx`
   (`bridge.slint:192-193`).
5. User taps "Start Casting".

### 3.7 Screen capture initiation
**Rust → Java:** `startScreenCapture()` JNI call.

1. Slint `Bridge.start-casting(w, h, fps)` callback (`bridge.slint:236`)
   sends `Event::StartCast { ... }` (`lib.rs:1813`).
2. Handler at `lib.rs:963` / `lib.rs:1011` calls Java
   `startScreenCapture(int, int, int)` via JNI
   (`senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:720-799`).
3. Android shows the MediaProjection permission dialog.
4. User grants permission.
5. `MainActivity.onActivityResult()` fires; a broadcast with
   `ACTION_MEDIA_PROJECTION_STARTED` is sent
   (`MainActivity.java:206, 280`).
6. `initializeCapture(resultCode, data)` sets up the capture pipeline
   (`MainActivity.java:815`).

### 3.8 OpenGL frame processing
**Java side:** `MainActivity.java` GL rendering loop.

1. `VirtualDisplay` is created from the MediaProjection token.
2. OpenGL ES context + EGL surface initialise.
3. Per frame:
   - Surface texture is updated from `VirtualDisplay`.
   - Fragment shader converts RGBA → YUV420 planar.
   - Y, U, V planes go into three direct `ByteBuffer`s.
4. `nativeProcessFrame(w, h, Y, U, V)` JNI callback runs per frame
   (`MainActivity.java:591`).

### 3.9 Frame hand-off to GStreamer
**JNI → Rust:** frame data transfer.

1. Rust receives the YUV plane data in
   `Java_org_fcast_android_sender_MainActivity_nativeProcessFrame`
   (`lib.rs:2437`).
2. Frames are stored in the global `FRAME_PAIR`
   (`Mutex<Option<VideoFrame<Writable>>>` + `Condvar`,
   `lib.rs:71`).
3. `CAPTURE_ACTIVE` (`lib.rs:76`) gates whether `appsrc`'s `need-data`
   callback waits or exits.
4. `FRAME_POOL` (`lib.rs:72`,
   `gst_video::VideoBufferPool`) recycles allocations.
5. The `appsrc` element pulls frames via `set_callbacks(need-data)`
   (`lib.rs:895`).

### 3.10 GStreamer pipeline & WHEP streaming
**Pipeline construction:** built in Rust when capture starts.

1. `appsrc` is configured to accept YUV frames.
2. `gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller(...)`
   handles WebRTC encoding. Internal encoder selection picks the right
   `amcvidenc-*` MediaCodec on Android — see the existing
   `senders/android/TODO.codecs.md` for the rationale on why this path
   does **not** need the migration runtime's `try_create_encoder_with_fallback`
   chain.
3. `WhepServerSignaller` creates the WHEP HTTP endpoint.
4. The signaller emits `Event::SignallerStarted { port_v4, port_v6 }`
   (`lib.rs:754`).
5. Rust sends `device.load(device::LoadRequest::Url { ... })` to the
   FCast device with the WHEP URL (`lib.rs:778`).

### 3.11 Receiver playback

1. FCast receiver receives a `PlayMessage` with the WHEP URL.
2. The receiver's WHEP client POSTs to the signaller.
3. SDP offer / answer exchange happens.
4. WebRTC media stream is established.
5. Receiver decodes and renders video / audio.

### 3.12 Migration runtime (graph system)
**Node-based processing:** `senders/android/src/migration/mod.rs`.

This is a parallel system for media-graph control. It is **not** on the
screen-mirror cast path today.

**Three entry points** (all funnel into `handle_command_json`):

| Entry | Where | Use |
|---|---|---|
| HTTP `POST /command` | `runtime.rs:200-300` | Debug / scripted, gated by `MIGRATION_COMMAND_BIND` env var |
| Java `nativeGraphCommand(String)` | `MainActivity.java:1100`, `lib.rs:2100-2120` | Production-callable from Java |
| Direct Rust `handle_command(...)` | `runtime.rs:322-324` | Internal callers (used by the in-tree smoke tests) |

**Command set** (`protocol.rs:37-106`):

| Command | Purpose |
|---|---|
| `CreateVideoGenerator { id }` | Test pattern (videotestsrc "ball") |
| `CreateSource { id, uri, audio, video }` | URL/file ingest (fallbacksrc → uridecodebin) |
| `CreateDestination { id, family, audio, video }` | RTMP / UDP / LocalFile / LocalPlayback |
| `CreateMixer { id, config, audio, video }` | compositor + audiomixer |
| `Connect { link_id, src_id, sink_id, audio, video, config }` | Add a link |
| `Disconnect { link_id }` | Remove a link |
| `Start { id, cue_time, end_time }` | Schedule a node to run |
| `Reschedule { id, cue_time, end_time }` | Move cue/end times |
| `Remove { id }` | Remove a node (cascades links) |
| `GetInfo { id }` | Read state |
| `AddControlPoint / RemoveControlPoint` | Mixer property timelines (volume, etc.) |

**NodeManager** (`senders/android/src/migration/node_manager.rs`):

```rust
struct NodeManager {
    started: bool,
    nodes:         HashMap<String, NodeRecord>,
    links:         HashMap<String, LinkRecord>,
    media_bridges: HashMap<String, StreamBridge>,
}
```

- `start()` / `tick()` / `shutdown()` (`node_manager.rs:289-314`).
- `dispatch(command)` (`node_manager.rs:316`) — central command router.
- `sync_media_links()` (`node_manager.rs:222-287`) — keeps StreamBridges
  consistent with the link graph.
- Tick interval: 100 ms (`runtime.rs:28`).

**StreamBridge** (`senders/android/src/migration/media_bridge.rs`):

- One producer `appsink` → many consumer `appsrc`s.
- Caps caching applied to late-joining consumers.
- `new_sample` callback fans buffers out.
- EOS propagated to all consumers.
- Stale consumers removed on push failure.

**State machine** (`protocol.rs:115-123`):

```
Initial → Starting → Started → Stopping → Stopped
```

**Destination pipelines** (per family, in `nodes/destination.rs`):

| Family | Video chain | Audio chain |
|---|---|---|
| `Rtmp` | `appsrc → videoconvert → timecodestamper → timeoverlay → H.264 enc → h264parse → queue → flvmux → rtmp2sink` | `appsrc → audioconvert → audioresample → AAC enc → queue → flvmux` |
| `Udp` | `appsrc → videoconvert → H.264 enc → h264parse → mpegtsmux → udpsink` | `appsrc → audioconvert → audioresample → AAC enc → mpegtsmux` |
| `LocalFile` | `appsrc → videoconvert → H.264 enc → h264parse → multiqueue → splitmuxsink` | `appsrc → audioconvert → audioresample → AAC enc → multiqueue → splitmuxsink` |
| `LocalPlayback` | `appsrc → queue → videoconvert → glimagesink / autovideosink` | `appsrc → queue → audioconvert → audioresample → openslessink / autoaudiosink` |

**Mixer pipeline** (`nodes/mixer.rs`):

- Video: `compositor` with black background + fallback videotestsrc +
  per-slot `appsrc → queue → compositor` request pads (xy/w/h/alpha/zorder).
- Audio: `audiomixer` with fallback audiotestsrc + per-slot
  `appsrc → queue → audioconvert → audioresample → capsfilter → audiomixer`
  (volume control point).

**VideoGenerator** (`nodes/video_generator.rs`):

- `videotestsrc pattern=ball flip=true is-live=true → deinterlace → appsink`.

**Smoke test path** (verifies the entire runtime from the UI):

The `test-smoke` quick-action in debug builds runs `run_graph_smoke_test()`
at `senders/android/src/lib.rs:418-481`:

1. `createvideogenerator { id }` — create a source.
2. `createmixer { id, audio:false, video:true }` — create a mixer.
3. `connect { link_id, src_id, sink_id, audio:false, video:true }` — link
   them.
4. `start { id }` for both nodes.
5. `getinfo {}` — read back the graph.
6. `remove { id }` for both nodes.

This exercises command parsing, node creation, link creation, scheduling,
StreamBridge wire-up, GStreamer pipeline building, and graceful teardown
in a single quick-action tap.

### 3.13 Stopping casting
**Cleanup flow** (Surface A):

1. User taps Stop.
2. Slint `Bridge.stop-casting()` callback (`bridge.slint:237`) → `on_stop_casting`
   handler at `lib.rs:1822`.
3. Rust sends `Event::EndSession { disconnect: true }` (`lib.rs:1826`).
4. `handle_event` matches `Event::EndSession { .. }` at `lib.rs:738`
   and calls `stop_cast(true)` (`lib.rs:682`):
   - Java `stopCapture()` is invoked via JNI
     (`MainActivity.java:801, 1113`).
   - `VirtualDisplay` is released.
   - FCast device `stop_playback()` runs.
   - Device `disconnect()` runs.
   - GStreamer pipeline is torn down.
5. UI state returns to `AppState::Disconnected`.

The migration runtime is unaffected by Surface A's stop — it has its own
lifecycle tied to `run_event_loop`'s entry / exit (`lib.rs:1035, 1053`).

---

## 4. Implementation — M1 (the only MVP cluster)

The only thing standing between you and a working Android-to-receiver
screen mirror is wiring the connect-page row taps to
`Bridge.connect-receiver(...)`. Everything downstream is already wired.

### 4.1 Step 1: replace the mock device list

**File:** `senders/android/ui/pages/connect_page.slint`

Change the iterator and the click handler.

**Before** (lines 69-88):

```slint
if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
    spacing: Theme.spacing-default;

    for device[idx] in root.mock-devices: Rectangle {
        height: Theme.row-height + 18px;

        property <bool> lp-armed: false;

        ta := TouchArea {
            changed pressed => {
                if self.pressed {
                    parent.lp-armed = true;
                } else {
                    parent.lp-armed = false;
                }
            }
            clicked => {
                /* placeholder: would call connect-receiver(device.address) */
            }
        }
        // …long-press timer, label rectangle…
    }
}
```

**After:**

```slint
if Bridge.devices.length > 0: VerticalLayout {
    spacing: Theme.spacing-default;

    for device[idx] in Bridge.devices: Rectangle {
        height: Theme.row-height + 18px;

        property <bool> lp-armed: false;

        ta := TouchArea {
            changed pressed => {
                if self.pressed {
                    parent.lp-armed = true;
                } else {
                    parent.lp-armed = false;
                }
            }
            clicked => {
                Bridge.connect-receiver(device);
            }
        }
        // …long-press timer, label rectangle…
    }
}
```

Two changes:

1. **Iterator source.** `root.mock-devices` (a page-local
   `[ReceiverItem]`) is replaced with `Bridge.devices` (the real
   `[string]` model pushed by Rust from `update_receivers_in_ui()`).
2. **Click handler.** The placeholder comment is replaced with a real
   call to `Bridge.connect-receiver(device)`. Because `Bridge.devices`
   is `[string]` today, `device` is the receiver *name* — which matches
   `on_connect_receiver`'s expected argument
   (`senders/android/src/lib.rs:1800-1810`).

Inside the row rectangle, you may need to swap `device.name` →
`device` and remove any other fields that the old `ReceiverItem` struct
exposed (`address`, `kind`, `is-default`) — those are not on
`Bridge.devices`. See §4.3 for the post-MVP fix.

The "empty state" branch on `connect_page.slint:46` keys off
`root.mock-empty || root.mock-devices.length == 0`. Switch the condition
to `Bridge.devices.length == 0` so it stays in sync.

### 4.2 Step 2: keep the long-press / context-menu intact

The long-press timer at `senders/android/ui/pages/connect_page.slint:90-101` stores
`device.id` and `device.name` into `root.context-receiver-id` /
`root.context-receiver-name`. With `Bridge.devices` returning strings,
adjust to:

```slint
triggered => {
    parent.lp-armed = false;
    root.context-receiver-id = device;
    root.context-receiver-name = device;
    root.context-menu-y = (parent.height * idx) + 100px;
    root.show-context-menu = true;
}
```

This loses the distinction between *display name* and *stable id* for
the context menu. That distinction matters for Phase 24 (rename / forget
receiver) but **not** for MVP — the disconnect button at line 209 is
already a documented no-op (`// UI-only: no-op. Phase 8 wires to
Bridge.disconnect-receiver(id).`).

### 4.3 Post-MVP cleanup (not required to ship the mirror)

The reason `Bridge.devices` is `[string]` and not `[ReceiverItem]` is
historical — see `senders/android/ui/bridge.slint:143-148`.

After MVP, promote it to `[ReceiverItem]` (already declared at
`bridge.slint:110-118`) and update both Rust's
`update_receivers_in_ui()` and the connect-page iterator to use the
rich struct. This is the **post-Phase-8 follow-up** — it does not block
the MVP demo. Track it in the existing phase docs (see Phase 24 — Pair
QR / receiver management).

### 4.4 Verification

After Step 1 + 2:

1. Build: `cargo +nightly build -p fcast-sender-android --target …`
   (or use the existing `xtask android-sender build` recipe).
2. Run on a device with a receiver on the same Wi-Fi.
3. Within ~5 s the receiver appears in the list.
4. Tap it. UI must transition Connect → Connecting → SelectingSettings.
5. Confirm resolution / framerate.
6. Grant MediaProjection consent. UI must transition →
   `Casting`. Phone screen renders on the receiver.
7. Tap Stop. UI must return to `Disconnected`. No GStreamer warnings in
   `adb logcat` other than benign ones.

If any of these steps fail, see §6 for the diagnostic recipe.

---

## 5. Verifying the Phase-8 surface that was previously M2–M5

These are "free" verifications now that Phase 8 is landed — they confirm
the work it shipped actually drives the cast UI correctly.

### 5.1 Status overlay (formerly M2)

**Expected:** When `Event::CaptureStarted` fires (`lib.rs:2253`),
`Bridge.status-items` (`bridge.slint:149`) is populated with at least
three `StatusItem`s (receiver / encoder / network). `status_overlay.slint`
reads these.

**Verify:**

```bash
grep -n 'on_capture_started\|Bridge::status_items\|status-items:' \
     senders/android/src/lib.rs senders/android/ui/*.slint
```

You should see `Bridge.status-items` declared in `bridge.slint:149` and
referenced from `components/status_overlay.slint`. The Rust push happens
in the `Event::CaptureStarted` branch of `handle_event`.

If you see *zero* badges on the casting overlay during a live mirror,
the issue is either:

- The Rust push is gated by `cfg!(debug_assertions)` or similar — check
  the `Event::CaptureStarted` handler at `lib.rs:875+`.
- The `components/status_overlay.slint` for-loop is iterating a stale
  page-local `mock-status-items` model. If so, replace with
  `Bridge.status-items` (same pattern as §4.1).

### 5.2 App version (formerly M5)

**Expected:** The About screen shows the real version. `Bridge.app-version`
(`bridge.slint:151`) is set from `env!("CARGO_PKG_VERSION")` during
startup.

**Verify:** open the About panel. The version must match
`senders/android/Cargo.toml`'s `[package].version`.

If it shows `"0.0.1-dev"` or empty, the Rust push line was not added in
the Phase-8 work for Cluster A2. Search for `app_version` /
`set_app_version` / `CARGO_PKG_VERSION` in `lib.rs` and add a one-line
push during `android_main`.

### 5.3 MediaProjection consent denial → rollback (formerly M3)

**Expected:** if the user denies the MediaProjection dialog (taps
Cancel), the app rolls back to `SelectingSettings` (or
`Disconnected` if the device disconnected). No partial cast state.

**Verify:** start a cast, then tap Cancel on the system dialog. The
event chain should be:

```
onActivityResult(resultCode != RESULT_OK)
  → Java does NOT broadcast ACTION_MEDIA_PROJECTION_STARTED
  → Rust does NOT receive Event::CaptureStarted
  → AppState rolls back to SelectingSettings via the timeout path
```

If the UI gets stuck on a black `WaitingForMedia` screen, the
`Application` timeout handler (`lib.rs:855+`) is not firing. Surface
that as a separate bug — not in scope for MVP.

### 5.4 Stop button → clean disconnect (formerly M4)

**Expected:** Tap Stop while casting → mirror ends within ~1 s; app
returns to `Disconnected`.

**Verify:**

```
adb logcat | grep -E '(EndSession|stop_cast|stopCapture|disconnect)'
```

Trace from `on_stop_casting` (`lib.rs:1822`) → `Event::EndSession` →
`stop_cast(true)` (`lib.rs:682`) → Java `stopCapture()`
(`MainActivity.java:801`).

If the UI hangs in `Casting` after Stop, check the FCast device
`disconnect()` future — most likely it's awaiting a TCP write that's
already dead. Adding a timeout to `device.disconnect()` is post-MVP.

---

## 6. Diagnostics — what to look at when the cast fails

The cast loop has six points where things commonly break. Each maps to a
specific source file with a one-line `adb logcat` filter.

| Symptom | First check | File / line |
|---|---|---|
| No receivers ever appear | mDNS scan running? Java side `FCastDiscoveryListener` started? | `MainActivity.java` (look for `NsdManager.discoverServices`) |
| Tapping receiver does nothing | **The MVP gap.** §4.1 above. | `connect_page.slint:85-88` |
| Stuck on Connecting | FCast SDK `connect()` future never resolves | `lib.rs:711-720` |
| Stuck on SelectingSettings | `Bridge.start-casting(...)` not invoked from UI | `bridge.slint:236`, look for caller in `pages/settings_*.slint` |
| MediaProjection dialog never appears | JNI `startScreenCapture` not reached | `lib.rs:1011-1020`, `MainActivity.java:720` |
| Black on receiver / no frames | YUV conversion shader bug or FRAME_PAIR contention | `MainActivity.java:591`, `lib.rs:71` |
| Receiver shows error | WHEP signaller didn't bind, or FCast `device.load` failed | `lib.rs:754, 778` |
| Migration smoke quick-action fails | NodeManager state machine / GStreamer caps | `runtime.rs`, `node_manager.rs:316` |

### 6.1 Smoke testing the migration runtime independently

Even before fixing M1, you can verify Surface B end-to-end on a debug
build:

1. Launch the app.
2. Open the quick-action bar.
3. Tap **"Smoke Graph"** (only visible in `cfg!(debug_assertions)`).
4. `Bridge.test-status` (`bridge.slint:201`) updates to
   `PASS smoke ok source=… mixer=… link=… nodes=…` or `FAIL …`.

If it returns `PASS`, the migration runtime works end-to-end from JNI
through `dispatch` through GStreamer pipeline build through teardown —
i.e. Surface B is shippable today.

If you have `MIGRATION_COMMAND_BIND=127.0.0.1:7890` set, you can hit it
directly:

```bash
curl -X POST http://127.0.0.1:7890/command \
     -d '{"createvideogenerator":{"id":"vg-1"}}'
curl -X POST http://127.0.0.1:7890/command \
     -d '{"createmixer":{"id":"mx-1","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:7890/command \
     -d '{"connect":{"link_id":"l-1","src_id":"vg-1","sink_id":"mx-1","audio":false,"video":true}}'
curl -X POST http://127.0.0.1:7890/command \
     -d '{"start":{"id":"mx-1"}}'
curl -X POST http://127.0.0.1:7890/command \
     -d '{"start":{"id":"vg-1"}}'
curl -X POST http://127.0.0.1:7890/command -d '{"getinfo":{}}' | jq .
```

The `getinfo` response is a `CommandResult::Info(Info { nodes: ... })`
listing two nodes in `state: "started"`.

---

## 7. Recommended order **after** MVP

Once §4 ships and §5 is verified, the remaining work splits into four
tiers. **Tier 1 is the architectural unification** — folding the two
functional surfaces together. Tiers 2-4 are feature breadth.

### Tier 1 — surface unification (the **first** post-MVP architectural goal)

These are the changes that fold the screen-mirror cast loop into the
migration runtime, so that there is a single source of truth for "what
is being captured, mixed, and sent where".

1. **`SourceNode::ScreenCapture` variant.** Add a new node type that
   exposes the existing JNI / OpenGL / FRAME_PAIR pipeline as a
   `NodeRecord::Source`. The implementation lives next to
   `nodes/source.rs` but feeds `appsrc` directly from `FRAME_PAIR`
   rather than building a `fallbacksrc`/`uridecodebin`. Smallest change:
   new file `nodes/screen_capture.rs` + new `NodeRecord` variant + new
   `Command::CreateScreenCaptureSource { id, width, height, fps }`.

2. **`DestinationFamily::Whep` variant.** Add a destination that wraps
   `BaseWebRTCSink` + `WhepServerSignaller` as a destination family.
   Smallest change: extend the `match family` in
   `nodes/destination.rs::build_live_pipeline` with a `Whep` arm that
   builds the existing pipeline. After this, the cast loop is:

   ```json
   {"createscreencapturesource":{"id":"cap-1","width":1280,"height":720,"fps":30}}
   {"createdestination":{"id":"whep-1","family":"Whep"}}
   {"connect":{"link_id":"l-1","src_id":"cap-1","sink_id":"whep-1"}}
   {"start":{"id":"cap-1"}}
   {"start":{"id":"whep-1"}}
   ```

3. **Replace direct `Event::StartCast` handling with graph commands.**
   The Rust state machine still owns the FCast SDK side (connect /
   disconnect / load) but the GStreamer side is fully expressed as
   migration-runtime commands. This is the largest of the three changes
   in this tier — touches `handle_event` for `StartCast` and
   `EndSession`, plus the `Event::CaptureStarted` push of
   `Bridge.status-items`.

Together, Tier 1 collapses Surface A into Surface B and removes the
"two functional surfaces" framing for good.

### Tier 2 — completeness of the screen-mirror path

- **Phase 24** — Pairing QR + receiver management. Adds the disconnect
  / rename / forget actions on top of Phase 8 / Cluster D's
  `ConfirmDialog`.
- **Phase 8 / Cluster B5** — Wi-Fi Aware toggle.
- **Phase 23** — Local recording. Trivially expressible as a second
  `DestinationFamily::LocalFile` node hanging off the same
  `ScreenCapture` source once Tier 1 is in.

### Tier 3 — feature breadth

- **Phase 15** — Camera capture (new `SourceNode::Camera` variant).
- **Phase 14** — Audio source selection.
- **Phase 16** — Bitrate presets (control points on the WHEP
  destination's encoder).
- **Phase 17 / 25** — Macros (composite commands that issue a
  graph-command sequence).
- **Phase 21 / 22** — Debug video / network detail pages (already wired
  by Phase 8 / Cluster A3-A5, just need polish).

### Tier 4 — defer indefinitely

Phases 28–48 (chat / scenes / streaming destinations / peripherals /
media player) sit downstream of architectural decisions that aren't
locked yet. Don't pull them into MVP scope.

---

## 8. Out of scope

This guide does **not** include:

1. **Architectural unification of Surfaces A and B** (§7 Tier 1). The
   MVP ships with the legacy cast path. Migration runtime is parallel.
2. **WHEP support inside the migration runtime.** Not in
   `DestinationFamily` (`protocol.rs:126-138`).
3. **MediaProjection as a `SourceNode`.** Not in `NodeRecord`
   (`node_manager.rs`).
4. **Promotion of `Bridge.devices` to `[ReceiverItem]`.** See §4.3 —
   loses display-name / id distinction but doesn't break the MVP.
5. **The 18 Phase-8 clusters that already landed.** See `PHASE-8-Section-*.md`
   for those.
6. **The 19 reimplement guides** for individual UI pages
   (`PHASE-9` … `PHASE-27`). They make the app feel finished. They
   do **not** affect whether mirroring works.
7. **Any feature not in §1.1.** Audio settings, camera, recording,
   macros, debug log, cast history, bitrate presets, pairing QR — all
   deferred.

---

## 9. Stop conditions

The MVP is "done" when **both** of the following hold:

### 9.1 Surface A (screen-mirror cast loop)

1. App launches.
2. Within 5 s, at least one receiver is listed on the connect page.
3. Tapping the receiver transitions:
   `Disconnected → Connecting → SelectingSettings`.
4. Confirming settings + granting MediaProjection consent transitions:
   `SelectingSettings → WaitingForMedia → Casting`.
5. The receiver displays the phone's screen at the selected resolution
   / framerate within 2 s of `Casting`.
6. Tapping Stop transitions `Casting → Disconnected` within 1 s.
7. `adb logcat` shows no `ERROR`-level lines from
   `org.fcast.android.sender` or `tracing_gstreamer` during the run.
8. Re-tapping the receiver works (no zombie state).

### 9.2 Surface B (migration runtime)

1. `Bridge.test-status` shows `PASS smoke ok …` after tapping the
   `Smoke Graph` debug quick-action.
2. `MIGRATION_COMMAND_BIND=127.0.0.1:7890` curl flow in §6.1 returns
   `result.info.nodes` with two entries in `state: "started"`.
3. `migration::runtime::shutdown_graph_runtime()` runs on app exit
   without leaking GStreamer pipelines (verify via
   `gst-launch-1.0 --gst-debug-no-color` traces).

If any of these fail, file a bug — do **not** mark MVP shipped.

---

## 10. Cross-reference index

| Topic | Live source | Companion phase guide |
|---|---|---|
| Bridge globals (canonical) | `senders/android/ui/bridge.slint` | `PHASE-8-Section-2-cluster-A-readonly-view-models.md` etc. |
| Application state machine | `lib.rs:1025-1058`, `lib.rs:734-925` | `PHASE-8-implementation-instructions.md` |
| Connect page (the **M1 gap**) | `pages/connect_page.slint:69-88` | `PHASE-6-receiver-list.md` |
| FRAME_PAIR / FRAME_POOL | `lib.rs:71-76` | — |
| MediaProjection / OpenGL | `MainActivity.java:206-845` | — |
| WHEP signaller | `lib.rs:754, 778` | `sdk/mirroring_core/src/transmission.rs` |
| Migration runtime entry | `lib.rs:1035, 2100, 2120` | — |
| Migration NodeManager | `migration/node_manager.rs` | — |
| Migration command protocol | `migration/protocol.rs` | — |
| Migration MediaBridge | `migration/media_bridge.rs` | — |
| Migration smoke test | `lib.rs:418-481` | — |
| Phase 8 cluster split | `PHASE-8-Section-0` … `Section-9` | — |
| UI bug review (2026-05-10) | `UI-REVIEW-2026-05-10.md` | — |

---

## Slint-doc references

These are the upstream docs that justify the patterns used in §4. All
paths verified against `draft/slint-ui/docs/astro/src/content/docs/`.

| Pattern in §4 | Slint doc |
|---|---|
| `for device[idx] in Bridge.devices` over a `[string]` model | `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx` |
| `Bridge.connect-receiver(device)` callback invocation | `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx` |
| Reading `Bridge.devices.length` in `if` | `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx` |
| `TouchArea`, `changed pressed` | `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx` (and the live `components/buttons.slint` patterns) |
| Long-press `Timer` semantics | `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx` |
| Bridge as a `global` singleton | `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` |

(Same paths previously verified during the Phase-8 split, see
`PHASE-8-Section-0-preflight.md` for the verification recipe.)

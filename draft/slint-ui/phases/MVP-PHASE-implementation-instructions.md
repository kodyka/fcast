# MVP — Step-by-step implementation guide

> **Audience:** developer who wants the *smallest possible* set of changes to
> get a working **Android sender → FCast receiver** screen-mirror loop —
> i.e. the user opens the app, picks a receiver, hits Cast, grants the
> MediaProjection permission, and sees their phone screen on the receiver.
> **Goal:** end-to-end mirroring works on a real device. Everything else is
> a follow-up.
> **Out of scope:** every Phase-12-through-27 UI sub-page (audio settings,
> camera settings, bitrate presets, recording, macros, debug log…) is
> *cosmetic* relative to the MVP. They make the app feel finished but they
> do not affect whether mirroring works.

This guide complements the Phase-8 split (`PHASE-8-Section-*.md`) and the
existing 19 reimplement guides — those are the **completeness** roadmap.
This one is the **MVP** roadmap: the shortest path from "app builds and
launches" to "screen is on the TV".

---

## 0. TL;DR — What's actually blocking MVP today

After auditing `senders/android/src/lib.rs` (1494 lines), `app/src/main/java/.../MainActivity.java`, `sdk/mirroring_core/src/transmission.rs`, `app/jni/Android.mk`, and the live Slint tree on `master`:

**Already shipped and wired in Rust + JNI** (the green path):

| Layer | Mechanism | Live in master |
|---|---|---|
| mDNS discovery | `FCastDiscoveryListener` (Java) → `Java_…_serviceFound` JNI → `Event::DeviceAvailable` | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="1153-1253" /> |
| Connect (Slint→Rust) | `Bridge.connect-receiver(name)` callback | ✅ wired in `android_main` <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="1008-1015" /> |
| Connect (Rust→FCast SDK) | `cast_ctx.create_device_from_info(…).connect(…)` | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="617-637" /> |
| MediaProjection consent | `startScreenCapture(w,h,fps)` Java method called from Rust | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="881-921" /> |
| VirtualDisplay → OES texture → GLES YUV split → 3 ByteBuffers | `MainActivity.java` GL pipeline | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java" lines="380-595" /> |
| Frame hand-off across threads | `FRAME_PAIR: (Mutex<Option<VideoFrame>>, Condvar)` + `FRAME_POOL` | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="29-37" /> |
| GStreamer pipeline | `appsrc → BaseWebRTCSink (with WhepServerSignaller)` | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/sdk/mirroring_core/src/transmission.rs" lines="476-513" /> |
| Receiver play message | After signaller binds: send `device.load(LoadRequest::Url{…})` to FCast device | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="660-700" /> |
| Stop / disconnect | `Bridge.stop-casting()` → `Event::EndSession{disconnect:true}` → `WhepSink::shutdown()` + `device.disconnect()` | ✅ <ref_snippet file="/home/ubuntu/repos/fcast/senders/android/src/lib.rs" lines="1030-1037" /> |

**The single MVP blocker** — caught during this audit:

> `pages/connect_page.slint` iterates over a page-local `mock-devices: [ReceiverItem]` (3 hardcoded entries). The `clicked => { /* placeholder */ }` handler does not call `Bridge.connect-receiver(name)`. As a result, even though Rust pushes real discovered receivers to `Bridge.devices`, the user can't actually connect to any of them — taps are no-ops.

That is **literally the only code change required** to make MVP work. Everything downstream of `Bridge.connect-receiver(name)` is already wired and tested. See [Section 4 (M1)](#41-m1--wire-the-connect-page-tap-actually-onto-bridgeconnect-receiver) below for the precise diff.

The next four MVP polish phases (M2–M5) are small, self-contained, and each adds visible UX value:

- **M2** — Casting status badges (Cluster A1; ~30 lines of Rust). Tells the user what's happening on the cast page.
- **M3** — MediaProjection-denied recovery. Today, declining the consent dialog leaves the UI stuck in WaitingForMedia. Fix the rollback.
- **M4** — Stop button confirmation. The cast page already has Stop/Cancel buttons; verify they roll back cleanly.
- **M5** — `Bridge.app-version` (Cluster A2; literally one line). About page shows `0.0.0` until this is wired.

After M1–M5, you have a shippable MVP. M6+ are optional follow-ups.

---

## 1. What "MVP" means for this app

| Capability | MVP scope | Justification |
|---|---|---|
| Discover receivers on local Wi-Fi | ✅ included | Without this, no path forward. |
| Tap → connect to a receiver | ✅ included | Currently the **blocker**. |
| Pick scale + framerate (or accept defaults) | ✅ included | The UI defaults are sensible (1080p×30). |
| Grant MediaProjection consent | ✅ included | Cannot be skipped — Android API requirement. |
| See screen on receiver | ✅ included | This is the deliverable. |
| Stop / disconnect cleanly | ✅ included | Required to free the projection token. |
| **System audio mirroring** | ❌ out | Requires API 29+ MediaProjection audio capture; never wired. Defer. |
| **Multiple cast destinations (RTMP, UDP, LocalFile)** | ❌ out | Behind `senders/android/TODO.codecs/` work — those need an `amcvidenc` encoder fallback chain. Not blocking screen mirroring over WHEP. |
| **Camera capture** | ❌ out | UI exists (Phase 15) but no JNI camera path; the front/rear/external cycler is decoration. |
| **Local recording to disk** | ❌ out | UI exists (Phase 23) but no MediaRecorder pipeline. |
| **Bitrate presets / quick-action customisation / macros** | ❌ out | UI-only Phase-12-27 work. None of it changes mirroring quality today. |
| **Localisation** | ❌ out for shipping; ✅ green-lighted in code | Phase 9 already wraps strings in `@tr()`; non-EN `.mo` files are not blockers for an English-first release. |

**MVP cluster (the 5 phases):**

- M1 — connect-page wiring (the only blocker)
- M2 — status badges on the casting page
- M3 — MediaProjection denial recovery
- M4 — verify stop / disconnect cleanly
- M5 — app version push (cosmetic but trivial)

Total estimated diff: ~120 lines across 4 files.

---

## 2. How the layers actually plug together

The published architecture overview is correct, but it leaves out the
exact line numbers / function names you need to read to understand the
state machine. Here is the layered view, grounded in the live tree:

```
┌──────────────────────────────────────────────────────────────────────┐
│ Slint UI (declarative, runs on the main thread)                      │
│ -------------------------------------------------                    │
│ pages/{connect,connecting,casting,settings,debug}_page.slint         │
│ components/{control_bar,status_overlay,buttons,…}.slint              │
│ bridge.slint   ← Bridge global: properties + callbacks               │
│ main.slint     ← Panel router; AppState dispatch                     │
└──────────────────────────────────────────────────────────────────────┘
                       │
                       │ Slint generates Rust bindings via include_modules!()
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Rust state machine (senders/android/src/lib.rs, 1494 lines)          │
│ -------------------------------------------------                    │
│ android_main()                  ← entry point per slint::android     │
│   ├── ui = MainWindow::new()                                         │
│   ├── ui.global::<Bridge>().on_connect_receiver({…})    line 1008   │
│   ├── ui.global::<Bridge>().on_start_casting({…})       line 1017   │
│   ├── ui.global::<Bridge>().on_stop_casting({…})        line 1030   │
│   ├── ui.global::<Bridge>().on_invoke_action({…})       line 1039   │
│   ├── runtime.spawn(async {                                          │
│   │       Application::new().run_event_loop(event_rx)                │
│   │   })                                                             │
│   └── ui.run()    ← blocking; runs until window closes               │
│                                                                      │
│ Application::handle_event(Event)                        line 640    │
│   • Event::ConnectToDevice(name) → connect_with_device_info(…)       │
│   • Event::SignallerStarted{…}   → device.load(LoadRequest::Url{…})  │
│   • Event::FromDevice{…}.StateChanged(Connected) → AppState::SelectingSettings │
│   • Event::StartCast{w,h,fps}    → call Java startScreenCapture(…)   │
│   • Event::CaptureStarted        → build appsrc + WhepSink           │
│   • Event::CaptureStopped        → set_capture_active(false)         │
│   • Event::CaptureCancelled      → AppState::Disconnected + stop_cast│
│   • Event::EndSession{disconnect}→ stop_cast(disconnect)             │
│                                                                      │
│ JNI callbacks (Java → Rust)                                          │
│   • Java_…_serviceFound          line 1153  ← mDNS                  │
│   • Java_…_serviceLost           line 1258                          │
│   • Java_…_nativeCaptureStarted  line 1275  ← user granted consent  │
│   • Java_…_nativeCaptureStopped  line 1289                          │
│   • Java_…_nativeCaptureCancelled line 1303 ← user denied consent   │
│   • Java_…_nativeProcessFrame    line 1465  ← per-frame YUV planes  │
│   • Java_…_nativeQrScanResult    line 1482                          │
└──────────────────────────────────────────────────────────────────────┘
                       │
                       │ JNI bridge: jni::JavaVM + JNIEnv (lib.rs:455-484)
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Java/Android (app/src/main/java/org/fcast/android/sender/…)          │
│ -------------------------------------------------                    │
│ MainActivity.java                                                    │
│   • startScreenCapture(w,h,fps) ← Rust calls into Java               │
│   • onActivityResult            ← MediaProjection consent             │
│   • initializeCapture            line 815                           │
│   • setupGles + GL rendering loop  line 380–595                     │
│   • nativeProcessFrame(w,h,Y,U,V)  line 591                         │
│   • stopCapture()                  line 801                         │
│ FCastDiscoveryListener.java     ← NSD wrapper                        │
│ ScreenCaptureService.java       ← Foreground service for projection  │
└──────────────────────────────────────────────────────────────────────┘
                       │
                       │ Frame data passed via ByteBuffers
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ GStreamer pipeline (sdk/mirroring_core/src/transmission.rs)          │
│ -------------------------------------------------                    │
│ Pipeline:                                                            │
│   appsrc (built in lib.rs:789-851)                                   │
│      │                                                               │
│      └─→  BaseWebRTCSink (gst-plugins-rs)                            │
│             │                                                        │
│             └─→ WhepServerSignaller (whep_signaller.rs)              │
│                  • Listens on auto-assigned port                     │
│                  • Emits SignallerStarted{port_v4, port_v6}          │
│                  • Provides WHEP HTTP endpoint for receiver          │
│                  • Internal MediaCodec encoder selection             │
│                    via `amcvidenc-*` factories on Android            │
└──────────────────────────────────────────────────────────────────────┘
                       │
                       │ WHEP (WebRTC-HTTP Egress Protocol)
                       ▼
┌──────────────────────────────────────────────────────────────────────┐
│ FCast receiver (TV / desktop / web)                                  │
│   1. Receives PlayMessage with content-type=video/whep + URL         │
│   2. WHEP client opens HTTP POST → SDP offer/answer exchange         │
│   3. WebRTC media flows, decoded, rendered                           │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.1 Frame pump in detail

The Android-specific frame path is the one piece that's worth reading
slowly, because it's the only place where multiple threads exchange
data without a queue:

```
┌─────────────────────────┐   1. ImageReader / VirtualDisplay produces        ┌────────────────────────────┐
│ MainActivity.java       │      OES texture frames at 30/60 Hz               │ Rust appsrc need-data CB    │
│ (MediaProjection +      │   2. GLES shader downsamples + plane-splits       │ (lib.rs:803-849)            │
│  GL renderer thread)    │      to 3 textures, glReadPixels into             │                             │
│                         │      direct ByteBuffers                           │  loop {                     │
│                         │   3. nativeProcessFrame(w,h,Y,U,V)                │    let frame = lock.lock(); │
└────────┬────────────────┘                                                   │    cvar.wait_for(100ms)     │
         │ JNI                                                                 │      until frame.is_some()  │
         ▼                                                                     │      OR !CAPTURE_ACTIVE     │
┌─────────────────────────┐                                                   │    appsrc.push_buffer(...)  │
│ process_frame()         │   4. Acquire pooled gst::Buffer (FRAME_POOL)      │  }                          │
│ (lib.rs:1315-1460)      │   5. Build VideoFrame<Writable> over the buffer   │                             │
│                         │   6. Copy 3 source planes into the writable frame │ Latency: 1 frame max         │
│ FRAME_PAIR.lock()       │   7. *frame_slot = Some(vframe);                  │ (drop-old semantics)         │
│ + cvar.notify_one()     │      cvar.notify_one()                            └────────────────────────────┘
└─────────────────────────┘
```

Key invariants:

- **`FRAME_PAIR` holds at most 1 frame.** New writes overwrite old ones if
  the consumer hasn't taken yet. This is intentional drop-old-frames
  semantics — better than queuing for a live mirror.
- **`CAPTURE_ACTIVE: AtomicBool`** is the kill switch. When the user stops
  capture, it flips to false and the consumer wakes from `cvar.wait_for`
  to call `appsrc.end_of_stream()`.
- **`FRAME_POOL: gst_video::VideoBufferPool`** reuses gst::Buffer
  allocations. On caps change (resolution/format), the pool is reconfigured.
- **No allocation on the hot path** once the pool is warm — `acquire_buffer`
  returns a pre-allocated buffer.

### 2.2 What the published architecture overview missed

- The `select_video_encoder` chain in `senders/android/src/migration/nodes/destination.rs` (P0-1 in `TODO.codecs/`) is **only** used by the migration framework's RTMP / UDP / LocalFile destinations. The MVP cast-to-FCast-receiver path uses `gst_rs_webrtc::webrtcsink::BaseWebRTCSink::with_signaller(...)` directly in `transmission.rs:476-513`, which has **its own internal encoder selection** that picks `amcvidenc-*` automatically on Android. So the TODO.codecs P0-1 work does **not** block MVP — it blocks alternative destinations nobody is using yet.

- The `migration` runtime (`crate::migration::runtime::start_graph_runtime()`, called from `run_event_loop` at lib.rs:947) is a parallel debug-only graph engine for the migration tests. It is **not** on the MVP cast path. You can read its logs without it interfering.

- The Phase-8 migration plan documents the **full** Bridge → Rust wiring for every UI page. The MVP only needs **two** of those wirings (`Bridge.connect-receiver` from the connect page, `Bridge.app-version` push). Everything else stays UI-only.

---

## 3. Smoke test that proves MVP is broken today

Before fixing anything, reproduce the broken state:

```sh
# 1. Build + install on a real Android device.
./gradlew :app:installDebug

# 2. Make sure an FCast receiver is on the same Wi-Fi network. Easiest:
#    run the Linux receiver in a sibling terminal:
cargo run -p fcast-receiver  # or whichever desktop receiver you have

# 3. On the phone, open the FCast sender app. Wait ~3 seconds.
#    Expected:  the receiver appears in the connect-page list.
#    Actual:    the connect-page list shows 3 hardcoded mock entries
#               ("Living Room TV", "Office Display", "Kitchen Chromecast")
#               and any real receivers do NOT appear.
#    Cause:     ConnectView reads `mock-devices`, not `Bridge.devices`.
#               Rust pushed `Bridge.devices` correctly (via lib.rs:572-577)
#               but the UI never reads it.

# 4. Tap any of the mock entries.
#    Expected:  state transitions to Connecting → SelectingSettings.
#    Actual:    nothing happens.
#    Cause:     `clicked => { /* placeholder */ }` in connect_page.slint:86.
#               Rust's `on_connect_receiver` handler is wired but the UI
#               never fires the callback.
```

This 30-second walkthrough is sufficient to confirm the M1 diagnosis. If
M1 fails to manifest after applying the fix below, double-check that
your APK was rebuilt and reinstalled (Slint changes require a full
rebuild — they're not hot-swappable).

---

## 4. Implementation — M1 through M5

Each section below is independently shippable. M1 is the only mandatory
one for MVP; M2–M5 are quality-of-life follow-ups.

---

### 4.1 M1 — Wire the connect page tap actually onto `Bridge.connect-receiver`

**File:** `senders/android/ui/pages/connect_page.slint`

#### Current state

```slint
// pages/connect_page.slint (lines ~17-89, ABRIDGED)
export component ConnectView inherits Rectangle {
    in-out property <[ReceiverItem]> mock-devices: [
        { id: "dev-1", name: "Living Room TV",     address: "192.168.1.50", … },
        { id: "dev-2", name: "Office Display",     address: "192.168.1.51", … },
        { id: "dev-3", name: "Kitchen Chromecast", address: "192.168.1.52", … },
    ];
    in-out property <bool> mock-empty: false;

    VerticalBox {
        // …
        if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
            for device[idx] in root.mock-devices: Rectangle {
                ta := TouchArea {
                    clicked => {
                        /* placeholder: would call connect-receiver(device.address) */
                    }
                }
                // …
            }
        }
    }
}
```

#### Target state

```slint
// pages/connect_page.slint — M1
export component ConnectView inherits Rectangle {
    // Real device names come from Rust via Bridge.devices: [string].
    // The ReceiverItem type stays for future enrichment (kind, port, etc.)
    // but Rust currently only pushes [string]; reduce to a name-only iter.
    in-out property <bool> mock-empty: false;     // dev-time toggle for empty-state QA

    VerticalBox {
        // … (header text unchanged) …

        // ── Empty state: searching ────────────────────────────────────────
        if root.mock-empty || Bridge.devices.length == 0: Rectangle {
            // … (Spinner + "Searching for receivers…" — unchanged) …
        }

        // ── Populated state: device list ──────────────────────────────────
        if !root.mock-empty && Bridge.devices.length > 0: VerticalLayout {
            spacing: Theme.spacing-default;

            for name[idx] in Bridge.devices: Rectangle {
                height: Theme.row-height + 18px;

                property <bool> lp-armed: false;

                ta := TouchArea {
                    changed pressed => {
                        if self.pressed { parent.lp-armed = true; }
                        else            { parent.lp-armed = false; }
                    }
                    clicked => {
                        Bridge.connect-receiver(name);
                    }
                }

                Timer {
                    interval: 600ms;
                    running: parent.lp-armed;
                    triggered => {
                        parent.lp-armed = false;
                        root.context-receiver-id = name;     // for now id == name
                        root.context-receiver-name = name;
                        root.context-menu-y = (parent.height * idx) + 100px;
                        root.show-context-menu = true;
                    }
                }

                Rectangle {
                    width: parent.width - 10px;
                    height: parent.height - 8px;
                    background: ta.pressed ? Theme.accent-pressed : Theme.surface-card;
                    border-radius: Theme.radius-card;

                    VerticalLayout {
                        padding-left: Theme.padding-screen;
                        padding-right: Theme.padding-screen;
                        alignment: center;
                        spacing: 2px;

                        Text {
                            text: name;       // was: device.name
                            color: Theme.text-primary;
                            font-size: Theme.font-size-body;
                            overflow: elide;
                        }
                        // No subtitle — Bridge.devices is just names today.
                        // Phase 8 (Cluster A — see PHASE-8-Section-2) can promote
                        // Bridge.devices to a [ReceiverItem] later for richer rows.
                    }
                }
            }
        }
    }

    // … (context menu + forget confirm — unchanged) …
}
```

#### Diff summary

- **Remove** `in-out property <[ReceiverItem]> mock-devices: [...]` (3 hardcoded entries).
- **Replace** `for device[idx] in root.mock-devices:` with `for name[idx] in Bridge.devices:`.
- **Replace** the placeholder `clicked => { /* … */ }` with `clicked => { Bridge.connect-receiver(name); }`.
- **Replace** every `device.name` / `device.id` reference with `name` (the iter variable).
- **Remove** any `device.kind == "fcast" ? "FCast" : "Generic"` subtitle logic — `Bridge.devices` is `[string]` today, no subtitle.

#### Why this is the only MVP blocker

After M1, the chain is complete end-to-end:

```
User taps row → Bridge.connect-receiver(name) callback fires
              → on_connect_receiver in lib.rs:1008 sends Event::ConnectToDevice(name)
              → run_event_loop dispatches to handle_event → Event::ConnectToDevice
              → connect_with_device_info → device.connect() → FCast SDK handshake
              → DeviceConnectionState::Connected → AppState::SelectingSettings
              → Slint Panel router shows pages/settings_page.slint with the Cast button
              → user taps Cast → Bridge.start-casting(w, h, fps)
              → on_start_casting in lib.rs:1017 sends Event::StartCast
              → handle_event → calls Java startScreenCapture(w, h, fps)
              → MediaProjection consent dialog
              → user grants → onActivityResult → initializeCapture → GL pipeline starts
              → nativeCaptureStarted JNI → Event::CaptureStarted
              → handle_event builds appsrc + WhepSink → AppState::Casting
              → frames pump through FRAME_PAIR → appsrc → BaseWebRTCSink
              → WhepServerSignaller binds → Event::SignallerStarted{port}
              → device.load(LoadRequest::Url{ url }) → receiver opens WHEP stream
              → MIRRORING IS LIVE
```

#### Verification

1. Apply the diff. Rebuild the APK.
2. Open the app on a real device with an FCast receiver on the same Wi-Fi.
3. The list should populate with the real receiver name(s) within ~3 seconds.
4. Tap a row. The state must transition: Disconnected → Connecting → SelectingSettings.
5. On the SelectingSettings page, tap "Cast". MediaProjection consent dialog appears.
6. Grant. State transitions: WaitingForMedia → Casting. Receiver shows the screen.

If the list stays empty: NSD discovery is not finding the receiver — check the receiver is broadcasting on `_fcast._tcp.local`. If the row fires but state stays Disconnected: the SDK `device.connect()` call is failing — check the SDK logs in `adb logcat` filtered by `tag:android-sender`.

#### Slint-doc references for M1

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — `Bridge.devices` is a global property, readable from any component.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx` — `for name[idx] in <expr>:` syntax.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx` — invoking a global callback from a touch area.

---

### 4.2 M2 — Casting status badges (Cluster A1, abbreviated)

**Goal:** when the user is casting, the casting page shows three live badges:
"Receiver: <name>", "Encoder: <selected>", "Network: <local-addr>".

This is exactly Cluster A1 from the Phase-8 split, but with two simplifications for MVP:

- Push only **once** per cast session (on `Event::CaptureStarted`), not on a 5-second poll.
- Don't bother with `BannerSeverity` (Cluster F) — keep severity inline.

**File:** `senders/android/src/lib.rs`

In the `Event::CaptureStarted` handler at line 787, after the `WhepSink::new(...)` succeeds and just before the `invoke_change_state(AppState::Casting)` call, push the badges:

```rust
// Right before: ui.global::<Bridge>().invoke_change_state(AppState::Casting);

let receiver_name = self.active_device.as_ref().map(|d| d.name()).unwrap_or_default();
let encoder_label = "Hardware (MediaCodec)".to_string();   // amcvidenc-* selected internally
let network_label = self.local_address.as_ref().map(|a| a.to_string()).unwrap_or_default();

self.ui_weak.upgrade_in_event_loop(move |ui| {
    let items: Vec<StatusItem> = vec![
        StatusItem {
            label: "Receiver".into(),
            value: receiver_name.into(),
            severity: StatusSeverity::Info,
        },
        StatusItem {
            label: "Encoder".into(),
            value: encoder_label.into(),
            severity: StatusSeverity::Info,
        },
        StatusItem {
            label: "Network".into(),
            value: network_label.into(),
            severity: StatusSeverity::Info,
        },
    ];
    let model = std::rc::Rc::new(slint::VecModel::from(items));
    ui.global::<Bridge>().set_status_items(model.into());
})?;
```

And in `Event::EndSession` / `Event::CaptureCancelled`, clear:

```rust
self.ui_weak.upgrade_in_event_loop(|ui| {
    let empty = std::rc::Rc::new(slint::VecModel::<StatusItem>::from(Vec::<StatusItem>::new()));
    ui.global::<Bridge>().set_status_items(empty.into());
    ui.global::<Bridge>().invoke_change_state(AppState::Disconnected);
})?;
```

**Bridge change** (one line):

```diff
 // bridge.slint
 export global Bridge {
     in property <[string]> devices: [];
+    in property <[StatusItem]> status-items: [];
     // …
 }
```

**Slint consumer** (in `pages/casting_page.slint`, replace the existing `mock-status-items` section):

```slint
// Was:
//   for item[idx] in root.mock-status-items: StatusPill { … }
// Becomes:
for item[idx] in Bridge.status-items: StatusPill {
    label: item.label;
    value: item.value;
    severity: item.severity;
}
```

#### Why M2 matters for MVP

Without it, the casting page renders the same mock badges (e.g. "Receiver: Living Room TV") regardless of which receiver the user actually connected to. That's confusing. M2 is ~30 lines and makes the casting page tell the truth.

---

### 4.3 M3 — Recover from MediaProjection denial

**Symptom today:** if the user taps "Cancel" or "Don't allow" on the consent dialog, the JNI callback `nativeCaptureCancelled` fires, which is handled at `lib.rs:1303`. The handler emits `Event::CaptureCancelled` which routes to `AppState::Disconnected` and stops the cast. **This already works correctly!** — but only because of the explicit handler.

**Verification:**

```sh
adb shell am start -n org.fcast.android.sender/.MainActivity
# Tap the receiver, tap Cast, tap "Cancel" on the consent dialog.
# Expected: page returns to ConnectView with the receiver still listed.
# Verify in logcat:
adb logcat android-sender:D *:S | grep -i 'capture\|state'
# You should see:
#   D android-sender: Screen capture was cancelled
#   D android-sender: Handling event: CaptureCancelled
#   …state transitions to Disconnected…
```

**If verification reveals the rollback is broken** (e.g. UI stays in WaitingForMedia), apply this defensive guard in `Event::CaptureCancelled`:

```rust
#[cfg(target_os = "android")]
Event::CaptureCancelled => {
    set_capture_active(false);
    self.ui_weak.upgrade_in_event_loop(|ui| {
        // Always go back to a deterministic state.
        let empty = std::rc::Rc::new(slint::VecModel::<StatusItem>::from(Vec::<StatusItem>::new()));
        ui.global::<Bridge>().set_status_items(empty.into());
        ui.global::<Bridge>().invoke_change_state(AppState::Disconnected);
    })?;
    self.stop_cast(false).await?;
}
```

(This is identical to the current code; if your audit shows missing rollback, the diff is empty — that means it already works and M3 is verification only.)

---

### 4.4 M4 — Verify stop / disconnect cleanly

**Goal:** ensure the user can press the cast page's Stop button and the
app returns to a clean Disconnected state, with the receiver also
returning to its idle screen.

**Already wired** at `lib.rs:1030-1037`:

```rust
ui.global::<Bridge>().on_stop_casting({
    let event_tx = event_tx.clone();
    move || {
        event_tx
            .send(Event::EndSession { disconnect: true })
            .unwrap();
    }
});
```

And the `EndSession` handler at `lib.rs:644-652` calls `stop_cast(true)`
which:

1. Calls `stopCapture()` JNI Java method (releases MediaProjection).
2. Drops the `WhepSink`, which `shutdown()`s the GStreamer pipeline.
3. Calls `device.disconnect()` to tell the receiver we're done.

**Verification:**

```sh
# While casting:
adb logcat android-sender:D *:S | grep -i 'stop\|disconnect\|end'
# Tap Stop on the cast page.
# Expected sequence:
#   D android-sender: Handling event: EndSession { disconnect: true }
#   D android-sender: Stopping playback
#   D android-sender: Disconnecting from active device
#   D android-sender: Screen capture was stopped
```

If the receiver doesn't return to idle — most common cause is that
`active_device.disconnect()` returns Ok but the FCast protocol disconnect
hasn't propagated yet. The 100ms sleep at lib.rs:600 is a workaround for
this. If you observe the receiver staying frozen, increase the sleep to
500ms.

---

### 4.5 M5 — Wire `Bridge.app-version` (Cluster A2 — one line)

**Goal:** the About page in `pages/help_page.slint` reads `Bridge.app-version` (per Phase 21's reimplement guide). Today it's never set, so it shows the default `""` (or whatever the page falls back to). One line in `android_main`:

```rust
// In android_main, right after `let ui = MainWindow::new().unwrap();`:
ui.global::<Bridge>().set_app_version(env!("CARGO_PKG_VERSION").into());
```

**Bridge change:**

```diff
 // bridge.slint
 export global Bridge {
     in property <[string]> devices: [];
+    in property <string> app-version: "";
     // …
 }
```

That's it. The about page's existing `Text { text: Bridge.app-version; }` consumer will Just Work.

---

## 5. After MVP — recommended phase order

Once M1–M5 are merged and you have a shippable MVP, here is the **fastest route** to a feature-complete app, ordered by user-perceived value vs. implementation cost. This is **separate** from the comprehensive Phase 12-27 work; it's the subset that earns its keep on the MVP.

### Tier 1 — High value, small diff (each ~1-2 days)

| Order | Phase | Why |
|---|---|---|
| 1 | **Phase 8 / Cluster A** (read-only view models) | Status badges (M2 above is the first half), app-version (M5 above), network interfaces, recording state. Pure additions to bridge + Rust pushes; no UI changes. ~150 lines. After Cluster A, the app **looks live** instead of looking like a mockup. |
| 2 | **Phase 8 / Cluster F** (banner) | Single source of truth for the success/warning/error banner that appears across multiple pages. Backup-reset, Wi-Fi-Aware toggle, Cast-history-cleared all need it. ~70 lines. Unblocks Cluster B and D. |
| 3 | **Phase 8 / Cluster B** (single-page state) | Audio settings, camera settings, recording controls, lifecycle modes, Wi-Fi-Aware all become real (Slint writes directly to `Bridge.<x>`, Rust reads at cast-start). ~250 lines. Audio settings actually shape the cast bitrate after this. |

### Tier 2 — Medium value, medium diff (~1 week each)

| Order | Phase | Why |
|---|---|---|
| 4 | **Phase 8 / Cluster C** | List mutations: bitrate presets, quick-actions unification (the **B12 fix**), macros, debug log. ~400 lines. After this, the Settings panels are interactive instead of decorative. |
| 5 | **Phase 8 / Cluster D + E** | Backup/import/reset + cast history + invariant docs. ~200 lines. After this, Phase 8 is fully done. |
| 6 | **TODO.codecs / P0-1** (encoder fallback chain) | `select_video_encoder()` rank-based amcvidenc discovery. **Required only if you want RTMP/UDP/LocalFile destinations to work.** WHEP-only flows already work without this. |

### Tier 3 — Lower priority but high-impact features

| Order | Phase | Why |
|---|---|---|
| 7 | **Phase 11 — Real platform plumbing** | Replace the placeholder `BatteryManager` / `ConnectivityManager` / `WifiAware` / `NetworkInterface` pushes Phase 8 stubs out with real JNI integrations. After this, status badges reflect reality. |
| 8 | **Camera capture (Phase 15 + Rust)** | Actual `Camera2` / `CameraX` JNI wiring. The UI already has the cycler. ~500 lines + new JNI surface. |
| 9 | **System audio mirroring** | Requires API 29+ MediaProjection audio capture path. New `AudioPlaybackCaptureConfiguration`, new `AudioRecord`, new `appsrc`. Not strictly needed for "screen mirror" but elevates the product. |
| 10 | **Local recording (Phase 23)** | `MediaRecorder` integration writing to a file alongside the WHEP stream. ~300 lines. |
| 11 | **Phases 21 / 26 (still-unbuilt UI)** | Help/about/attributions, debug log viewer. ~400 lines each. |

### Tier 4 — Speculative / out-of-scope until product direction firms up

Phases 28-48 (chat, streaming destinations, scenes, peripherals,
media-player). Defer until you have enough signal that users want these.

---

## 6. Quick-reference for "I want to add a new <X> wiring"

When you need to wire a new property or callback after MVP:

| You want to … | Pattern |
|---|---|
| Push read-only data Rust→Slint | `in property <T> X` in bridge.slint; Rust calls `set_X(...)` via `ui_weak.upgrade_in_event_loop`. See A1-A5 in [`PHASE-8-Section-2-cluster-A-readonly-view-models.md`](./PHASE-8-Section-2-cluster-A-readonly-view-models.md). |
| Two-way state where Slint mutates and Rust reads on demand | `in-out property <T> X` in bridge.slint; Slint writes directly; Rust reads via `ui.global::<Bridge>().get_X()` at action time. See B1-B2 in [`PHASE-8-Section-3-cluster-B-single-page-state.md`](./PHASE-8-Section-3-cluster-B-single-page-state.md). |
| Slint→Rust action with no return value | `callback X(...)` in bridge.slint; Rust binds via `ui.global::<Bridge>().on_X(\|args\| { ... })`. See M1 above (`Bridge.connect-receiver`). |
| Push a list and let Slint mutate via callbacks | `in property <[T]> X` + `callback set-X-Y(string, …)` etc.; Rust holds `Arc<Mutex<Vec<T>>>` + a `push` closure. See C1-C5 in [`PHASE-8-Section-4-cluster-C-list-mutations.md`](./PHASE-8-Section-4-cluster-C-list-mutations.md). |
| Add a new JNI method (Java→Rust) | Add a `pub extern "C" fn Java_<package>_<class>_<method>` block in `lib.rs` mirroring an existing one (mDNS handlers at lib.rs:1153 or capture handlers at lib.rs:1275 are good templates). Declare `native <returntype> <method>(...)` in the Java side. |
| Add a new Rust→Java method | Use `vm.get_env()?.call_method(activity, "methodName", "(II)V", &[arg.into()])` (lib.rs:898 is the canonical example). |

---

## 7. What MVP intentionally does NOT do

A list to manage expectations:

- **No multi-receiver casting.** One receiver at a time. The state machine
  enforces this — `Event::ConnectToDevice` while `active_device` is Some
  will silently overwrite, with no failure handling.
- **No reconnection on Wi-Fi loss.** If Wi-Fi drops mid-cast, the WHEP
  stream goes dead and the UI stays in `Casting` state until the user
  taps Stop. Phase 11 will add `ConnectivityManager` listening; until
  then, manual stop is the workflow.
- **No background casting.** Casting runs in the activity's lifecycle.
  If the user backgrounds the app, the foreground service keeps capture
  going (see `ScreenCaptureService.java`) but the UI thread sleeps. This
  is fine for MVP — most users keep the phone on for short casts.
- **No QR-code pairing.** The UI exposes a `scan-qr` quick action (Phase 24)
  that calls into Java's `scanQr` method and routes the result back via
  `nativeQrScanResult`. This is wired and works, but it's not on the
  critical MVP path — direct mDNS discovery is enough.
- **No audio.** As above. System audio capture requires API 29+ work
  not yet present.
- **Screen recording / DRM-protected content.** Android's MediaProjection
  blocks DRM-protected surfaces by design. Netflix etc. will appear black
  on the receiver. That's OS-level, not fixable in this app.

---

## 8. Stop conditions for MVP

You're done with MVP when **every** statement below is true:

- [ ] **M1 merged.** Connect-page list shows real receivers; tapping a row connects.
- [ ] **End-to-end live cast** verified with at least one real receiver: the user taps a receiver, taps Cast, grants consent, sees their screen on the receiver within ~5 seconds.
- [ ] **Stop works.** Tapping Stop on the cast page returns the UI to Disconnected and the receiver to idle within ~2 seconds.
- [ ] **Cancel works.** Tapping Cancel on the MediaProjection consent dialog returns the UI to ConnectView (state Disconnected), with the receiver still listed.
- [ ] **No crashes** during a 5-minute cast on a mid-range device (Pixel 4a, Galaxy S10, etc.).
- [ ] **APK builds green** with `cargo build -p android-sender --release && ./gradlew :app:assembleRelease`.
- [ ] **`adb logcat` clean** during a cast: no panics, no `error!` lines, no `tracing::warn` from the GStreamer pipeline.

M2-M5 are recommended polish; if any are deferred, document the deferral
in the release notes ("status badges show static placeholder text — wired
in next release").

---

## 9. Cross-reference index

This guide intersects with several other docs in `phases/`:

| If you want… | Read… |
|---|---|
| The full Phase-8 wiring (every property, every callback) | [`PHASE-8-implementation-instructions.md`](./PHASE-8-implementation-instructions.md) (TOC) + the `PHASE-8-Section-*.md` series. |
| The reasoning behind cluster ordering (F → A → B → C → D → E) | [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md). |
| The original Phase-8 "what we're building" spec | [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md). |
| Step-by-step UI sub-page guides (12-27) | `PHASE-{N}-reimplement-instructions.md`. |
| Codec / encoder / RTMP work (TODO.codecs) | `senders/android/TODO.codecs/README.md`. |
| Live phase-status snapshot | [`STATUS.md`](./STATUS.md). |

---

## Slint-doc references used (M1-M5)

All paths are relative to the repo root and verified against the live
tree on the current branch.

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — Bridge as global singleton.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — `in` vs `in-out` property direction.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx` — `for x in <expr>:` rendering.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx` — invoking a global callback from a TouchArea.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — `StatusItem` struct field access.

GStreamer-side and JNI-side claims are grounded in the live tree:

- `senders/android/src/lib.rs` — Rust state machine + JNI surface.
- `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java` — Java capture pipeline.
- `senders/android/app/jni/Android.mk` — GStreamer plugin bundle.
- `sdk/mirroring_core/src/transmission.rs` — WhepSink + create_webrtcsink.
- `sdk/mirroring_core/src/whep_signaller.rs` — WHEP server-side signalling.

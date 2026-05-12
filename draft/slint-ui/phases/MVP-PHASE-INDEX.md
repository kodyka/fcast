# MVP-PHASE — Index
 
> Step-by-step implementation guide for the **smallest set of changes** that
> takes the Android sender from "app builds and launches" to "screen is on
> the TV", plus the **post-MVP architectural unification** that folds the
> legacy WHEP cast loop into the migration runtime's node graph.
>
> **Doc-only.** Every file in this set tells you what to change in the
> existing tree, with `file:line` citations and concrete code snippets. No
> source-tree code is touched by this set of docs.
 
This index replaces the monolithic `MVP-PHASE-implementation-instructions.md`
when you want a *checklist-shaped* read. The monolithic doc remains as the
narrative read; both stay in sync.
 
---
 
## 0. The MVP shape (recap)
 
After Phase 8 (Clusters F + A1–A5 + B1–B5 + C1/C2/C4/C5 + D1/D2 + E) landed
on `master`, the live state is:
 
| Surface | Status |
|---|---|
| Bridge globals (15+ clusters) | wired by Phase 8 |
| Screen-mirror cast loop | **blocked by 1 Slint placeholder** (MVP-PHASE-1) |
| Migration runtime (Surface B) | functional, shipped, parallel |
 
The MVP itself is **one cluster** (Phase 1 below). The remaining phases
either *verify* the Phase-8-shipped surface or *extend* the architecture
post-MVP.
 
---
 
## 1. Phase ordering and dependency graph
 
```
                ┌────────────────────────────────────────────────┐
                │ MVP-PHASE-1                                    │
                │   connect-receiver wiring                       │  ◀── the
                │   (the only MVP-gating change, ~10 lines)       │      MVP gate
                └─────────────────┬──────────────────────────────┘
                                  │
            ┌─────────────────────┼─────────────────────┐
            ▼                     ▼                     ▼
  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
  │ MVP-PHASE-2      │  │ MVP-PHASE-3      │  │ MVP-PHASE-7      │
  │   Phase-8 verify │  │  migration smoke │  │  ReceiverItem    │
  │   (A1/A2/M3/M4)  │  │  (Surface B)     │  │  promotion       │
  └──────────────────┘  └──────────────────┘  └──────────────────┘
 
                ── MVP boundary ──────────────────────────────────
 
  ┌────────────────────────────────────────────────────────────────┐
  │ Tier 1 — surface unification (post-MVP architectural goal)      │
  │                                                                 │
  │   MVP-PHASE-4 (screen-capture SourceNode)                       │
  │           ↓                                                     │
  │   MVP-PHASE-5 (Whep DestinationFamily)                          │
  │           ↓                                                     │
  │   MVP-PHASE-6 (graph-command cast loop — final unification)     │
  └────────────────────────────────────────────────────────────────┘
 
  ┌────────────────────────────────────────────────────────────────┐
  │ Optional — protocol expansion (independent of Tier 1)           │
  │                                                                 │
  │   MVP-PHASE-8 (Srt DestinationFamily)                           │
  │     — extends DestinationFamily with Srt; mirrors the Udp arm   │
  │       in nodes/destination.rs::build_live_pipeline.             │
  └────────────────────────────────────────────────────────────────┘
```
 
- **Phase 1** is the only thing that *must* ship for MVP.
- **Phases 2, 3, 7** can run in parallel after Phase 1 (or in any order
  before it — they don't touch the cast loop).
- **Phases 4 → 5 → 6** are sequential: each one consumes the previous
  one's API surface. They are **not** in MVP scope.
- **Phase 8** is optional and independent of every other phase. It can
  ship any time after Phase 3 (which establishes the migration-runtime
  smoke infrastructure used by its on-device verification).
 
---
 
## 2. File-by-file summary
 
| # | File | What | Net diff | Risk |
|---|---|---|---|---|
| 1 | `MVP-PHASE-1-connect-receiver-wiring.md` | Replace `mock-devices` iter with `Bridge.devices` and wire `clicked => Bridge.connect-receiver(device)`. | ~10 lines, 1 Slint file | 🟢 |
| 2 | `MVP-PHASE-2-phase-8-verification.md` | Verify the Phase-8-shipped wirings that previously needed M2–M5 work (status-items, app-version, MediaProjection denial rollback, Stop button cleanup). | 0 code lines (verification only); possibly 1-line Rust push for A2 if `app-version` is empty | 🟢 |
| 3 | `MVP-PHASE-3-migration-runtime-smoke.md` | Smoke-test the migration runtime (Surface B) via the `Smoke Graph` debug quick-action and the `MIGRATION_COMMAND_BIND` HTTP server. | 0 code lines (smoke only) | 🟢 |
| 4 | `MVP-PHASE-4-screen-capture-source-node.md` (+ 6 STEP files — see below) | Add `NodeRecord::ScreenCapture` and `Command::CreateScreenCaptureSource { id, width, height, fps }`. New file `nodes/screen_capture.rs` that reads from `FRAME_PAIR` into the runtime's `appsink` model. **Post-MVP / Tier 1.1.** | ~250-400 Rust lines, 1 new file + 3 edited | 🟡 |
| 4.1 | `MVP-PHASE-4-STEP-1-protocol-extension.md` | Add `Command::CreateScreenCaptureSource { id, width, height, fps }` with serde defaults `1280 / 720 / 30`. | ~25 lines | 🟢 |
| 4.2 | `MVP-PHASE-4-STEP-2-screen-capture-node.md` | Define `ScreenCaptureNode`, `LiveScreenCapturePipeline`, `build_live_pipeline`, and the `FRAME_PAIR → appsrc` consumer. **Largest step.** | ~250 Rust lines (1 new file) | 🟡 |
| 4.3 | `MVP-PHASE-4-STEP-3-module-registration.md` | Add `pub mod screen_capture;` and `pub use screen_capture::*;` to `nodes/mod.rs`. | 2 lines | 🟢 |
| 4.4 | `MVP-PHASE-4-STEP-4-node-record.md` | Add `NodeRecord::ScreenCapture` variant; thread it through every `match self` arm in `impl NodeRecord` (~13 methods). | ~80 lines | 🟡 |
| 4.5 | `MVP-PHASE-4-STEP-5-dispatch-arm.md` | Add the `Command::CreateScreenCaptureSource` dispatch arm + `create_screen_capture_source(...)` constructor. | ~30 lines | 🟢 |
| 4.6 | `MVP-PHASE-4-STEP-6-unit-tests.md` | 8 host-runnable unit tests across `protocol.rs` and `node_manager.rs`. No GStreamer init required. | ~120 lines of tests | 🟢 |
| 5 | `MVP-PHASE-5-whep-destination-family.md` (+ 7 STEP files — see below) | Extend `DestinationFamily` with `Whep` and wire `BaseWebRTCSink` + `WhepServerSignaller` into `nodes/destination.rs::build_live_pipeline`. **Post-MVP / Tier 1.2.** | ~150-250 Rust lines, 2 edited files | 🟡 |
| 5.1 | `MVP-PHASE-5-STEP-1-protocol-extension.md` | Add `Whep { server_port }` to `DestinationFamily`; add `bound_port_v4` / `bound_port_v6` to `DestinationInfo`. | ~30 lines | 🟢 |
| 5.2 | `MVP-PHASE-5-STEP-2-pipeline-profile.md` | Extend `DestinationPipelineProfile::from_family` with a `Whep` arm. | ~15 lines | 🟢 |
| 5.3 | `MVP-PHASE-5-STEP-3-destination-node-fields.md` | Add `whep_bound_port_v4` / `whep_bound_port_v6` fields to `DestinationNode`. | ~15 lines | 🟢 |
| 5.4 | `MVP-PHASE-5-STEP-4-build-live-pipeline.md` | Wire the `Whep` arm into `DestinationNode::build_live_pipeline`. **Largest step.** | ~80 lines | 🟡 |
| 5.5 | `MVP-PHASE-5-STEP-5-signaller-reexport.md` | Flip `mod whep_signaller;` to `pub mod whep_signaller;` in `mcore::lib.rs`. Add `whep_signaller_compat` shim. | 1 SDK line + 1 shim file | 🟢 |
| 5.6 | `MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md` | Add `whep_bound_ports` field to `LiveDestinationPipeline`. Extend `refresh()` to read the slot. | ~30 lines | 🟡 |
| 5.7 | `MVP-PHASE-5-STEP-7-unit-tests.md` | ~12 host-runnable unit tests (no GStreamer init required). | ~150 lines of tests | 🟢 |
| 6 | `MVP-PHASE-6-graph-command-cast-loop.md` | Replace direct `Event::StartCast` / `Event::EndSession` handling with `migration::runtime::handle_command(...)` calls — Surface A becomes a thin orchestrator over Surface B. **Post-MVP / Tier 1.3 (the unification step).** | ~200-300 Rust lines, 1 edited file | 🟠 |
| 7 | `MVP-PHASE-7-receiver-item-promotion.md` | Promote `Bridge.devices` from `[string]` to `[ReceiverItem]` (already declared in `bridge.slint:110-118`), update `update_receivers_in_ui()` and the connect-page iterator. **Post-MVP polish / Tier 2.1.** | ~50 lines, 3 edited files | 🟢 |
| 8 | `MVP-PHASE-8-srt-destination-family.md` (+ 6 STEP files — see below) | Extend `DestinationFamily` with `Srt { uri, latency, passphrase, pbkeylen }`; mirror the `Udp` arm in `nodes/destination.rs::build_live_pipeline` with `srtsink` + `mpegtsmux`. Add `srt` to `GSTREAMER_PLUGINS` in `senders/android/app/jni/Android.mk`. SRT-as-source already works through `uridecodebin` — no `SourceNode` change. **Optional / Tier 1.4 (post-MVP protocol expansion).** | ~150 lines Rust + 1 Makefile line, 2 edited files | 🟡 |
| 8.1 | `MVP-PHASE-8-STEP-1-protocol-extension.md` | Add `Srt { uri, latency, passphrase, pbkeylen }` to `DestinationFamily`. Backward-compatible wire format. | ~30 lines | 🟢 |
| 8.2 | `MVP-PHASE-8-STEP-2-pipeline-profile.md` | Extend `DestinationPipelineProfile::from_family` with an `Srt` arm (diagnostic element listing). | ~10 lines | 🟢 |
| 8.3 | `MVP-PHASE-8-STEP-3-build-live-pipeline.md` | Wire the `Srt` arm into `DestinationNode::build_live_pipeline`. Mirror of the `Udp` branch. **Largest step.** | ~90 lines | 🟡 |
| 8.4 | `MVP-PHASE-8-STEP-4-android-makefile.md` | Add `srt` to `GSTREAMER_PLUGINS` in `senders/android/app/jni/Android.mk`. **Mandatory for any on-device test.** | 1 line | 🟢 |
| 8.5 | `MVP-PHASE-8-STEP-5-unit-tests.md` | ~12 host-runnable unit tests (no GStreamer init required). | ~150 lines of tests | 🟢 |
| 8.6 | `MVP-PHASE-8-STEP-6-source-side.md` | Documentation: SRT sources already work via `uridecodebin` + Step 4. No `SourceNode` change. | 1 test | 🟢 |
 
Risk legend: 🟢 trivial, 🟡 medium, 🟠 architectural.
 
---
 
## 3. Stop conditions
 
The **MVP** is "done" when:
 
1. Phase 1 ships and survives §9.1 of `MVP-PHASE-implementation-instructions.md`.
2. Phase 2 / §5.1–5.4 verifications all pass on a real device.
3. Phase 3 / Surface B smoke returns `PASS` via the debug quick-action.
 
**Phases 4–6 are not gates.** They are the recommended **first** post-MVP
architectural milestone. **Phase 7** is small post-MVP polish.
 
---
 
## 4. How to read each phase doc
 
Every `MVP-PHASE-N-*.md` file follows the same six-section template
(borrowed from the Phase-8 split):
 
| Section | Contents |
|---|---|
| **0. Goal** | One-paragraph statement of what changes after this phase ships. |
| **1. Pre-flight** | Live state on `master` — what's already wired, what isn't. |
| **2. Steps** | Sequential implementation steps with concrete Slint + Rust snippets. |
| **3. Verification** | `grep` recipes, `adb logcat` filters, smoke flows. |
| **4. Common pitfalls** | Failure modes specific to this phase. |
| **5. Stop conditions** | Exit criteria — when the phase is "done". |
 
---
 
## 5. Glossary
 
| Term | Defined in |
|---|---|
| **Surface A** | Legacy screen-mirror cast loop (MediaProjection → OpenGL → FRAME_PAIR → appsrc → BaseWebRTCSink → WhepServerSignaller). See `MVP-PHASE-implementation-instructions.md` §2, §3.1–3.11, §3.13. |
| **Surface B** | Migration runtime node graph (URL/file → mixer → RTMP/UDP/LocalFile/LocalPlayback). See `MVP-PHASE-implementation-instructions.md` §2, §3.12. |
| **Tier 1 unification** | Phases 4 → 5 → 6 collapse Surface A into Surface B. |
| **M1 cluster** | The one MVP-gating change. Implemented in Phase 1. |
| **`FRAME_PAIR`** | `lazy_static!` `(Mutex<Option<VideoFrame<Writable>>>, Condvar)` at `lib.rs:71`. The hand-off point from JNI's `nativeProcessFrame` to the `appsrc` `need-data` callback. |
| **`StreamBridge`** | One-producer-many-consumers `appsink → appsrc` fanout in the migration runtime. `media_bridge.rs`. |
| **`NodeRecord`** | The enum that wraps all migration runtime node types. `node_manager.rs:21-26`. |
| **`DestinationFamily`** | `protocol.rs:126-138`. Current variants: `Rtmp / Udp / LocalFile / LocalPlayback`. Phase 5 adds `Whep`; Phase 8 adds `Srt`. |
| **`Bridge.devices`** | `[string]` at `bridge.slint:145`. Promoted to `[ReceiverItem]` in Phase 7. |
 
---
 
## 6. Cross-references
 
| Topic | Live source |
|---|---|
| Application state machine | `senders/android/src/lib.rs:1025-1058`, `1734-1925` |
| Bridge globals | `senders/android/ui/bridge.slint` |
| Connect page (the M1 gap) | `senders/android/ui/pages/connect_page.slint:46, 69-101` |
| `update_receivers_in_ui()` | `senders/android/src/lib.rs:659-680` |
| FRAME_PAIR / FRAME_POOL | `senders/android/src/lib.rs:71-76` |
| MediaProjection / OpenGL | `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:206-845` |
| WHEP signaller event | `senders/android/src/lib.rs:754, 778` |
| Migration runtime entry | `senders/android/src/lib.rs:1035, 2100, 2120` |
| Migration NodeManager | `senders/android/src/migration/node_manager.rs` |
| Migration command protocol | `senders/android/src/migration/protocol.rs` |
| Migration MediaBridge | `senders/android/src/migration/media_bridge.rs` |
| Migration smoke test (Rust) | `senders/android/src/lib.rs:418-481` |

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

  ┌────────────────────────────────────────────────────────────────┐
  │ Optional — UI ↔ migration-runtime decoupling (Tier 2.2)        │
  │                                                                 │
  │   MVP-PHASE-9 (debug-bridge decoupling)                         │
  │     — routes the four debug quick-actions through new Bridge   │
  │       callbacks; makes runtime startup on-demand; gates         │
  │       the debug handlers behind #[cfg(debug_assertions)].       │
  └────────────────────────────────────────────────────────────────┘

  ┌────────────────────────────────────────────────────────────────┐
  │ Optional — repo extraction (Tier 3 architectural)              │
  │                                                                 │
  │   MVP-PHASE-10 (android-sender repo extraction)                 │
  │     — moves senders/android/ out of kodyka/fcast into a new     │
  │       standalone repo (working name fcast-android-sender);      │
  │       pulls the SDK crates as Git deps. Independent release    │
  │       cadence; smaller checkout; faster monorepo CI.            │
  │     — touches two repos; the only phase that does.              │
  │     — irreversible-ish; recommend after PHASE-9 stabilises      │
  │       the UI ↔ runtime contract.                                │
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
- **Phase 9** is post-PHASE-6 polish (depends on the graph-command
  cast loop being live, so the lazy-start ensure-call sites at
  `Event::CaptureStarted` exist). Purely structural — no behaviour
  change. Defer indefinitely if the debug surface is fine as-is.
- **Phase 10** is the only phase that **moves code between
  repositories**. PHASE-9 (the prerequisite for a clean Bridge ↔
  runtime contract) **merged to `master` on 2026-05-19** at commit
  `b394eea` (PR #46), so the new repo can pin the extraction SHA at
  or after `d8ff886` and inherit the small public contract on day
  one. Two-repo PR pair; irreversible-ish (see PHASE-10 §0.2).
  Defer indefinitely if the monorepo cadence is fine as-is.
 
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
| 6 | `MVP-PHASE-6-graph-command-cast-loop.md` (+ 9 STEP files — see below) | Replace direct `Event::StartCast` / `Event::EndSession` handling with `migration::runtime::handle_command(...)` calls — Surface A becomes a thin orchestrator over Surface B. **Post-MVP / Tier 1.3 (the unification step).** | ~200-300 Rust lines, 1 edited file | 🟠 |
| 6.1 | `MVP-PHASE-6-STEP-1-node-id-constants.md` | Add three `const &str` IDs (`CAST_SOURCE_ID`, `CAST_DESTINATION_ID`, `CAST_LINK_ID`) at the top of `lib.rs`. | ~6 lines | 🟢 |
| 6.2 | `MVP-PHASE-6-STEP-2-capturestarted-rewrite.md` | Replace `Event::CaptureStarted` body with graph commands + `tokio::spawn` poll loop. **Largest step.** | ~150 lines | 🟠 |
| 6.3 | `MVP-PHASE-6-STEP-3-signaller-started-helper.md` | Extract `mcore::transmission::build_whep_play_msg(addr, port)` helper. | ~10 SDK + ~3 lib lines | 🟢 |
| 6.4 | `MVP-PHASE-6-STEP-4-stop-cast-rewrite.md` | Replace `tx_sink.shutdown()` in `stop_cast` with `Disconnect L + Remove src + Remove dst`. | ~30 lines | 🟡 |
| 6.5 | `MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md` | Gate `tx_sink: Option<WhepSink>` field behind `#[cfg(not(target_os = "android"))]`. | ~10 lines | 🟢 |
| 6.6 | `MVP-PHASE-6-STEP-6-frame-pair-unchanged.md` | **Documentation-only checkpoint.** `FRAME_PAIR` producer untouched. | 0 source lines | 🟢 |
| 6.7 | `MVP-PHASE-6-STEP-7-set-capture-active-preservation.md` | **Preservation step.** Confirm `set_capture_active(false)` calls preserved. | 0 source lines | 🟢 |
| 6.8 | `MVP-PHASE-6-STEP-8-mod-migration-exports.md` | Cosmetic `pub use protocol::{...}` re-exports. | 1 line | 🟢 |
| 6.9 | `MVP-PHASE-6-STEP-9-optional-feature-flag.md` | **Optional, opt-in.** `FCAST_UNIFIED_CAST_GRAPH=0/1` runtime kill-switch. Conflicts with STEP-5. | ~30 lines | 🟡 |
| 7 | `MVP-PHASE-7-receiver-item-promotion.md` (+ 5 STEP files — see below) | Promote `Bridge.devices` from `[string]` to `[ReceiverItem]` (already declared in `bridge.slint:110-118`), update `update_receivers_in_ui()` and the connect-page iterator. **Post-MVP polish / Tier 2.1.** | ~50 lines, 3 edited files | 🟢 |
| 7.1 | `MVP-PHASE-7-STEP-1-bridge-property-type.md` | Change `Bridge.devices` from `[string]` to `[ReceiverItem]` (one line in `bridge.slint`). | 1 line | 🟢 |
| 7.2 | `MVP-PHASE-7-STEP-2-update-receivers-in-ui.md` | Rewrite `update_receivers_in_ui()` to construct `ReceiverItem` structs. **Largest step.** | ~40 lines | 🟢 |
| 7.3 | `MVP-PHASE-7-STEP-3-connect-page-field-reads.md` | Long-press captures `device.id` + `device.name`; row shows `device.name` + `device.address`. | ~20 Slint lines | 🟢 |
| 7.4 | `MVP-PHASE-7-STEP-4-click-handler-passes-id.md` | `Bridge.connect-receiver(device)` → `Bridge.connect-receiver(device.id)`. | 1 line | 🟢 |
| 7.5 | `MVP-PHASE-7-STEP-5-cleanup-mock-devices.md` | **Optional cleanup.** Remove `mock-devices` + `mock-empty` `in-out property`s from `ConnectView`. | ~2 lines deleted | 🟢 |
| 8 | `MVP-PHASE-8-srt-destination-family.md` (+ 6 STEP files — see below) | Extend `DestinationFamily` with `Srt { uri, latency, passphrase, pbkeylen }`; mirror the `Udp` arm in `nodes/destination.rs::build_live_pipeline` with `srtsink` + `mpegtsmux`. Add `srt` to `GSTREAMER_PLUGINS` in `senders/android/app/jni/Android.mk`. SRT-as-source already works through `uridecodebin` — no `SourceNode` change. **Optional / Tier 1.4 (post-MVP protocol expansion).** | ~150 lines Rust + 1 Makefile line, 2 edited files | 🟡 |
| 8.1 | `MVP-PHASE-8-STEP-1-protocol-extension.md` | Add `Srt { uri, latency, passphrase, pbkeylen }` to `DestinationFamily`. Backward-compatible wire format. | ~30 lines | 🟢 |
| 8.2 | `MVP-PHASE-8-STEP-2-pipeline-profile.md` | Extend `DestinationPipelineProfile::from_family` with an `Srt` arm (diagnostic element listing). | ~10 lines | 🟢 |
| 8.3 | `MVP-PHASE-8-STEP-3-build-live-pipeline.md` | Wire the `Srt` arm into `DestinationNode::build_live_pipeline`. Mirror of the `Udp` branch. **Largest step.** | ~90 lines | 🟡 |
| 8.4 | `MVP-PHASE-8-STEP-4-android-makefile.md` | Add `srt` to `GSTREAMER_PLUGINS` in `senders/android/app/jni/Android.mk`. **Mandatory for any on-device test.** | 1 line | 🟢 |
| 8.5 | `MVP-PHASE-8-STEP-5-unit-tests.md` | ~12 host-runnable unit tests (no GStreamer init required). | ~150 lines of tests | 🟢 |
| 8.6 | `MVP-PHASE-8-STEP-6-source-side.md` | Documentation: SRT sources already work via `uridecodebin` + Step 4. No `SourceNode` change. | 1 test | 🟢 |
| 9 | `MVP-PHASE-9-debug-bridge-decoupling.md` (+ 6 STEP files — see below) | Route the four debug quick-actions (`migrated-server`, `test-getinfo`, `test-crossfade`, `test-smoke`) through new `Bridge` callbacks instead of direct `migration::runtime::*` calls inside `on_invoke_action`. Make `start_graph_runtime()` on-demand instead of unconditional at `lib.rs:1110`. Optional `#[cfg(debug_assertions)]` separation. **Optional / Tier 2.2 polish.** | ~80-120 lines, 2-3 edited files | 🟢 |
| 9.1 | `MVP-PHASE-9-STEP-1-bridge-callbacks.md` | Add 3 new callbacks to `bridge.slint`: `start-migration-server(string)`, `run-migration-test(string)`, `stop-migration-server()`. | ~3 Slint lines | 🟢 |
| 9.2 | `MVP-PHASE-9-STEP-2-rust-handlers.md` | Register `on_start_migration_server` / `on_run_migration_test` / `on_stop_migration_server` handlers delegating to existing free functions. **Largest step.** | ~60 Rust lines | 🟢 |
| 9.3 | `MVP-PHASE-9-STEP-3-quick-actions-rewrite.md` | Rewrite the four `on_invoke_action` debug branches to invoke the new Bridge callbacks. | ~25 lines (~deletions) | 🟢 |
| 9.4 | `MVP-PHASE-9-STEP-4-lazy-runtime-start.md` | Delete unconditional `start_graph_runtime()` at `lib.rs:1110`; add ensure-start calls at `Event::CaptureStarted` and `nativeProcessGraphCommandJson`. | ~5 lines deleted + ~6 lines added | 🟡 |
| 9.5 | `MVP-PHASE-9-STEP-5-debug-cfg-separation.md` | **Optional.** Gate the Step-2 registrations with `#[cfg(debug_assertions)]`; optionally extract to a `mod debug_quickactions` submodule. | ~3 lines (inline) or ~70 lines (submodule) | 🟢 |
| 9.6 | `MVP-PHASE-9-STEP-6-unit-tests.md` | 6 host-runnable unit tests: dispatch-table mapping, idempotent start/shutdown, round-trip. | ~80 lines of tests | 🟢 |
| 10 | `MVP-PHASE-10-android-sender-repo-extraction.md` (+ 9 STEP files — see below) | Extract `senders/android/` from `kodyka/fcast` into a new standalone repo `fcast-android-sender`. The four SDK crates stay in the monorepo and are pulled in as Git deps with the `path` subspec. **Optional / Tier 3 architectural.** Two-repo change; irreversible-ish. | 0 functional lines; ~few-hundred file moves + Cargo.toml rewrites + CI duplication across two repos | 🟠 |
| 10.1 | `MVP-PHASE-10-STEP-1-preflight-inventory.md` | Inventory: file count, path-dep audit, Slint cross-tree audit, build.rs env audit, strategy decision (default: Git dep with subpath). **Analysis-only.** | 0 lines (doc only) | 🟢 |
| 10.2 | `MVP-PHASE-10-STEP-2-bootstrap-new-repo.md` | Create the new GitHub repo, `cp -a senders/android/.` into it, add LICENSE / .gitignore / README skeleton, first commit + tag. | ~few-hundred file moves + 3 new files | 🟢 |
| 10.3 | `MVP-PHASE-10-STEP-3-resolve-path-deps.md` | Rewrite the three `path = ...` deps (`fcast-protocol`, `fcast-sender-sdk`, `mcore`) to `{ git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/..." }`. | ~6 Cargo.toml lines | 🟡 |
| 10.4 | `MVP-PHASE-10-STEP-4-standalone-cargo-toml.md` | Inline every `workspace = true` reference with explicit version + features (14 deps + 1 build-dep). Copy `[profile.release]` from monorepo. Decide single-crate vs one-member workspace. | ~30 Cargo.toml lines | 🟡 |
| 10.5 | `MVP-PHASE-10-STEP-5-vendor-slint-helpers.md` | Vendor `sdk/mirroring_core/ui/common.slint` + `senders/ui-components/*` into the new repo's `ui/components/` tree. Rewrite two import paths at the edges. **Correction vs the research: the UI is NOT fully self-contained.** | ~5-20 Slint files + 2 import-path rewrites | 🟡 |
| 10.6 | `MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md` | Adapt GitHub Actions workflow (replace `cargo xtask android download-*` with inline `curl`/`tar`); optionally Gitlab CI; verify Dockerfile + Gradle wrapper + `ci/ui-validate.sh` paths; document NDK / GStreamer env. | ~100 CI lines + doc | 🟡 |
| 10.7 | `MVP-PHASE-10-STEP-7-first-build-verification.md` | First end-to-end build & smoke: `cargo +nightly check`, `cargo build --release`, `./gradlew assembleDebug`, install on device, exercise PHASE-9 quick-actions, end-to-end cast. **Last safety net before STEP-8.** | 0 lines (verification only) | 🟡 |
| 10.8 | `MVP-PHASE-10-STEP-8-remove-from-monorepo.md` | Delete `senders/android/` from `kodyka/fcast`, drop duplicate workspace member, drop `.gitlab-ci.yml` include, delete (or rewire) the `android-release-apk.yml` GHA workflow, update README link. **Irreversible-ish; one commit, one PR.** | ~few-hundred file deletions + 3 Cargo.toml lines + 2 CI files | 🟠 |
| 10.9 | `MVP-PHASE-10-STEP-9-cross-repo-sync.md` | Document the long-term workflow: SDK pin bump cadence (weekly), PR-pair procedure, re-vendoring Slint helpers, release-blocking regression playbook. Add `docs/cross-repo-sync.md` to the new repo. | ~250 lines of docs | 🟢 |
 
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

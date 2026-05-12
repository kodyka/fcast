# MVP-PHASE-3 — Migration runtime smoke
 
> Verify that **Surface B** (the migration runtime node graph) is
> end-to-end functional. **No new functionality.** This phase confirms
> the graph runtime, HTTP server, JNI entry, and command dispatch all
> work — so the Tier 1 unification (Phases 4–6) has a stable foundation.
 
---
 
## 0. Goal
 
After this phase, you have confirmed:
 
1. The migration runtime starts during `run_event_loop`
   (`start_graph_runtime` at `lib.rs:1035`).
2. The 100 ms refresh thread is alive
   (`runtime.rs:51-58`).
3. The full **JSON command flow** works:
   create → connect → start → getinfo → remove.
4. The optional **HTTP server** (`MIGRATION_COMMAND_BIND`) accepts
   commands and returns valid JSON responses.
5. Clean shutdown via `shutdown_graph_runtime` on app exit
   (`lib.rs:1053`).
 
You do **not** modify code in this phase. You execute the existing
debug quick-actions and optionally hit the HTTP server with `curl`.
 
---
 
## 1. Pre-flight
 
### 1.1 Three entry points to verify
 
| Entry | Wired? | Live source |
|---|---|---|
| **HTTP** (`POST /command` on `MIGRATION_COMMAND_BIND`) | optional, env-gated | `senders/android/src/lib.rs:175-232`, `senders/android/src/migration/runtime.rs:75-300` |
| **JNI** (`nativeGraphCommand(String)`) | always wired | `senders/android/src/lib.rs:2100-2120`, `senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java:875-878, 1100` |
| **Direct Rust** (`handle_command(...)`) | always wired | `senders/android/src/migration/runtime.rs:322-333` |
 
### 1.2 Debug quick-actions that exercise these entry points
 
The quick-action bar in `cfg!(debug_assertions)` builds shows four
extra items (added at `senders/android/src/lib.rs:1081-1086`):
 
| Quick action id | Title | Calls | Live source |
|---|---|---|---|
| `migrated-server` | Migrated srv | `start_migrated_command_server` (sets `MIGRATION_COMMAND_BIND=0.0.0.0:8080`, then `start_graph_runtime`) | `senders/android/src/lib.rs:180-232, 2016-2024` |
| `test-getinfo` | GetInfo | `run_legacy_http_getinfo_test` (POSTs `{"getinfo":{}}` to `0.0.0.0:8080`) | `senders/android/src/lib.rs:235-254, 2026-2033` |
| `test-crossfade` | Crossfade | `run_legacy_http_crossfade_test` (POSTs a crossfade scenario) | `senders/android/src/lib.rs:256-300, 2035-2042` |
| `test-smoke` | Smoke Graph | `run_graph_smoke_test` (in-process, exercises `handle_command` directly) | `senders/android/src/lib.rs:418-481, 2044-2051` |
 
`Bridge.test_status` (`senders/android/ui/bridge.slint:201`) is
populated with `PASS …` or `FAIL …` after each quick-action returns.
 
### 1.3 Why this matters for MVP
 
Surface B is **not** the MVP cast loop. But MVP-PHASE-4 / 5 / 6 build
on Surface B as the architecture. If Surface B's smoke is currently
broken, those phases will have a regressed foundation. Smoking it here
gates the start of the Tier 1 unification work.
 
---
 
## 2. Steps
 
### 2.1 Step 1 — verify `start_graph_runtime` runs at startup
 
```bash
adb logcat | grep -Ei 'start_graph_runtime|graph-runtime-refresh|GRAPH_REFRESH'
```
 
**Expected (within 1 s of `Application::run_event_loop` entry):**
 
```
… start_graph_runtime: started
… graph-runtime-refresh: tick (every 100 ms)
```
 
**If absent:** check `senders/android/src/lib.rs:1034-1037` is reached:
 
```rust
// senders/android/src/lib.rs:1034-1037 (read-only — do not edit in this phase)
if let Err(err) = crate::migration::runtime::start_graph_runtime() {
    warn!("Failed to start graph runtime: {err}");
}
```
 
If start logs an error, capture it. The most common failure mode is
GStreamer not being initialised before `start_graph_runtime` — the call
must come after `ensure_gstreamer_initialized()` (it does, on `master`).
 
### 2.2 Step 2 — smoke via the `Smoke Graph` quick-action (in-process JNI path)
 
This exercises the migration runtime **without** the HTTP server.
Everything runs in-process through `run_graph_command(...)` →
`migration::runtime::try_handle_command_json` (`lib.rs:210-232`).
 
**Procedure:**
 
1. Build a debug build: `cargo +nightly build -p fcast-sender-android …`.
2. Install + launch the app.
3. Open the quick-action bar (`CastControlBar`).
4. Tap **Smoke Graph** (the green-themed action with id `test-smoke`).
5. Watch `Bridge.test-status` in the casting overlay (or use
   `adb logcat | grep 'UI test completed\|UI test failed'`).
 
**Expected:**
 
```
PASS smoke ok source=slint-smoke-videogen-<millis> mixer=slint-smoke-mixer-<millis> link=slint-smoke-link-<millis> nodes=2
```
 
**What this verifies:**
 
- Command JSON serialisation: `createvideogenerator`, `createmixer`,
  `connect`, `start`, `getinfo`, `remove` (six commands).
- `NodeManager::dispatch` correctly routes each command
  (`node_manager.rs:316`).
- `SourceNode` / `MixerNode` / `VideoGeneratorNode` build their
  GStreamer pipelines (`nodes/*.rs`).
- `StreamBridge` attaches source `appsink` to mixer `appsrc`
  (`node_manager.rs:222-287`, `media_bridge.rs`).
- `getinfo` returns a populated `nodes` map with both entries in
  `state: "started"`.
- Cleanup (`remove`) tears down GStreamer pipelines without error.
 
**If FAIL:**
 
| FAIL message contains | Likely cause |
|---|---|
| `createvideogenerator … duplicate` | A previous smoke didn't clean up. Restart app. |
| `connect … capability mismatch` | `audio:false, video:true` flags didn't match. Confirm Mixer has `video:true`. |
| `Failed to build … pipeline` | A GStreamer element couldn't be created — usually missing plugin. Check `senders/android/app/jni/Android.mk:57-66`. |
| `Failed to set_state … to Playing` | `appsrc` caps mismatch. Check the mixer slot config in `nodes/mixer.rs::build_live_pipeline`. |
 
### 2.3 Step 3 — smoke the HTTP server (optional)
 
This step exercises the **HTTP entry point** of the migration runtime.
It requires that `MIGRATION_COMMAND_BIND` be set before
`start_graph_runtime` runs.
 
**3a. Set the env var via the `Migrated srv` quick-action:**
 
1. Tap **Migrated srv** (`migrated-server`). It calls
   `start_migrated_command_server("0.0.0.0:8080")` which:
   - Sets `std::env::set_var(MIGRATION_COMMAND_BIND_ENV, "0.0.0.0:8080")`
     (`lib.rs:182`).
   - Calls `migration::runtime::start_graph_runtime()` which now spawns
     the HTTP server thread (`runtime.rs:200-300`).
2. `Bridge.test-status` shows `PASS migrated server started on 0.0.0.0:8080`.
 
**3b. Forward port from the device:**
 
```bash
adb forward tcp:8080 tcp:8080
```
 
**3c. Hit the server with `curl`:**
 
```bash
# 1. Create a video generator.
curl -X POST http://127.0.0.1:8080/command \
     -H 'Content-Type: application/json' \
     -d '{"createvideogenerator":{"id":"vg-1"}}' | jq .
 
# Expected response:
# {
#   "id": null,
#   "result": "success"
# }
 
# 2. Create a mixer.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"createmixer":{"id":"mx-1","audio":false,"video":true}}' | jq .
 
# 3. Connect them.
curl -X POST http://127.0.0.1:8080/command \
     -d '{"connect":{"link_id":"l-1","src_id":"vg-1","sink_id":"mx-1","audio":false,"video":true}}' | jq .
 
# 4. Start the mixer (sink first), then the source.
curl -X POST http://127.0.0.1:8080/command -d '{"start":{"id":"mx-1"}}' | jq .
curl -X POST http://127.0.0.1:8080/command -d '{"start":{"id":"vg-1"}}' | jq .
 
# 5. Read back the graph.
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | jq .
 
# Expected response (truncated):
# {
#   "id": null,
#   "result": {
#     "info": {
#       "nodes": {
#         "vg-1": { "state": "started", "kind": "source", "audio": false, "video": true, … },
#         "mx-1": { "state": "started", "kind": "mixer",  "audio": false, "video": true, … }
#       },
#       "links": {
#         "l-1": { "src_id": "vg-1", "sink_id": "mx-1", "audio": false, "video": true }
#       },
#       …
#     }
#   }
# }
 
# 6. Tear down.
curl -X POST http://127.0.0.1:8080/command -d '{"remove":{"id":"vg-1"}}' | jq .
curl -X POST http://127.0.0.1:8080/command -d '{"remove":{"id":"mx-1"}}' | jq .
```
 
**What this verifies:**
 
- `parse_http_request` parses Method / Path / Content-Length correctly
  (`runtime.rs:92-150`).
- `handle_command_http_request` routes POSTs to
  `handle_command_json` (`runtime.rs:200-300`).
- The full command set is reachable from outside the JVM.
 
**Why bind to `0.0.0.0:8080`?** The `migrated-server` quick-action
uses that address. If you want to scope it to localhost only, set
`MIGRATION_COMMAND_BIND=127.0.0.1:7890` before app launch (via
`adb shell setprop` — but `lib.rs:182` re-overwrites this, so realistically
you'd modify the quick-action or set the env var some other way).
 
### 2.4 Step 4 — `test-getinfo` and `test-crossfade` smokes
 
These two debug quick-actions exercise more elaborate scenarios over
the HTTP server (`run_legacy_http_getinfo_test` at `lib.rs:235`,
`run_legacy_http_crossfade_test` at `lib.rs:256`).
 
**Procedure:**
 
1. With `Migrated srv` active (Step 3a), tap **GetInfo** then
   **Crossfade**.
2. `Bridge.test-status` shows `PASS …` for each.
 
**What `test-crossfade` verifies:**
 
- Mixer `AddControlPoint` / `RemoveControlPoint` commands
  (`protocol.rs:91-103`).
- Timeline-based interpolation in `evaluate_control_points` (
  `senders/android/src/migration/nodes/control.rs`).
- Mixer slot property application during refresh
  (`MixerNode::apply_control_points`).
 
### 2.5 Step 5 — verify clean shutdown
 
```bash
adb logcat | grep -Ei 'shutdown_graph_runtime|graph-runtime-refresh.*exit|graph-command-server.*exit'
```
 
**Procedure:** Press the device's back button enough times to exit
the app (or kill from Recents).
 
**Expected:**
 
```
… shutdown_graph_runtime: stop_event_loop
… graph-runtime-refresh thread exited
… graph-command-server thread exited (if started)
… NodeManager::shutdown — all nodes torn down
```
 
**If absent:** the `Application::run_event_loop` exit path didn't
reach `lib.rs:1052-1056` (`shutdown_graph_runtime`). Most common
cause is a panic earlier in the exit sequence — capture full logcat
and file as a bug.
 
---
 
## 3. Verification
 
### 3.1 Greps (read-only checks against `master`)
 
```bash
# Three entry points exist.
grep -n 'start_graph_runtime\|shutdown_graph_runtime' senders/android/src/lib.rs
#  → expect: lib.rs:1034-1037, 1052-1056, and in runtime.rs
 
grep -n 'nativeGraphCommand' senders/android/src/lib.rs \
                              senders/android/app/src/main/java/org/fcast/android/sender/MainActivity.java
#  → expect: lib.rs:~2100, MainActivity.java:876, 1100
 
grep -n 'fn handle_command\b\|fn handle_command_json\|fn try_handle_command_json' \
    senders/android/src/migration/runtime.rs
#  → expect: runtime.rs:322, 334, 358 (approx)
 
# Debug quick-actions exist.
grep -n '"test-smoke"\|"test-getinfo"\|"test-crossfade"\|"migrated-server"' \
    senders/android/src/lib.rs
#  → expect: 1082-1085 (declarations), 2016, 2026, 2035, 2044 (handlers)
```
 
### 3.2 In-UI verification matrix
 
| Test | Expected `Bridge.test-status` |
|---|---|
| `test-smoke` | `PASS smoke ok source=… mixer=… link=… nodes=2` |
| `migrated-server` | `PASS migrated server started on 0.0.0.0:8080` |
| `test-getinfo` (after `migrated-server`) | `PASS legacy getinfo …` |
| `test-crossfade` (after `migrated-server`) | `PASS legacy crossfade …` |
| Clean shutdown | logcat shows shutdown sequence in §2.5 |
 
### 3.3 Curl verification (after Step 3)
 
```bash
# All commands return HTTP 200 with `"result": "success"` or `"result": { ... }`.
# No 4xx / 5xx, no JSON parse errors, no error.
```
 
---
 
## 4. Common pitfalls
 
### P1 — `test-smoke` fails on first run, passes on retry
 
Caused by a previous smoke run not cleaning up (panic'd mid-test).
Restart the app. `run_graph_smoke_test` uses timestamp-suffixed IDs
(`lib.rs:424-426`) so consecutive normal runs don't collide.
 
### P2 — `Migrated srv` fails with `address already in use`
 
The HTTP server thread from a previous app run might still be bound.
Force-stop and restart:
 
```bash
adb shell am force-stop org.fcast.android.sender
```
 
If that doesn't help, `adb shell` and `lsof -i :8080` (if the device
has busybox) — likely a zombie thread that survived the panic.
 
### P3 — `curl` hangs after POST
 
The HTTP server reads up to `GRAPH_COMMAND_MAX_REQUEST_SIZE` (1 MB)
of body (`runtime.rs:28-30`). If your POST doesn't have a
`Content-Length` header, the parser may hang. `curl -d ...` sets it
automatically. If you're hand-crafting requests, include the header.
 
### P4 — `getinfo` returns `nodes: {}` after creating nodes
 
Either the create commands returned errors (check each response) or
you sent `start` before `connect` (the smoke ordering is intentional —
create → connect → start). Re-run with the exact sequence in §2.3.
 
### P5 — On-device `test-smoke` reports `nodes=0` despite no other errors
 
`run_graph_smoke_test` does `getinfo` *after* both `start` calls. If
`refresh_nodes` hasn't ticked yet, the state may still be `Starting`.
That's fine — `nodes=2` is the count of *registered* nodes, not the
count in `state="started"`. If `nodes < 2` you have a real bug.
 
### P6 — App exit doesn't run `shutdown_graph_runtime`
 
Slint Android lifecycle exit can short-circuit `Application::run_event_loop`
if the user kills via Recents. The shutdown is best-effort. Verify
GStreamer pipelines aren't leaked by checking `adb logcat` for
`UNREF was called on a pipeline that has children`.
 
---
 
## 5. Stop conditions
 
The phase is "done" when:
 
1. The four in-UI smoke tests in §3.2 return `PASS`.
2. The curl flow in §2.3 succeeds against the HTTP server.
3. Clean shutdown traces appear in logcat on app exit.
4. `Bridge.test-status` displays the smoke result via
   `pages/codec_test_page.slint` (or wherever the debug status is
   surfaced).
 
Once Surface B is confirmed green, you can safely start MVP-PHASE-4
(the first Tier 1 unification step).

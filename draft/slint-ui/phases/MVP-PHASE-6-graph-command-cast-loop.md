# MVP-PHASE-6 — Graph-command cast loop (Tier 1.3)

> **The surface-unification step.** After MVP-PHASE-4 added a
> screen-capture source node, and MVP-PHASE-5 added a Whep destination
> family, the migration runtime knows how to build the entire cast
> pipeline as a graph. **This phase flips the switch.** It replaces
> the bespoke `Event::StartCast` / `Event::EndSession` GStreamer
> plumbing inside `senders/android/src/lib.rs` with calls into
> `migration::runtime::handle_command(...)`. Surface A becomes a thin
> orchestrator over Surface B.

---

## 0. Goal

Today, the Android sender has **two** parallel cast paths:

| Surface | Driver | Pipeline construction site |
|---|---|---|
| A (legacy) | `Event::StartCast` / `Event::CaptureStarted` | `senders/android/src/lib.rs:875-961` (`appsrc` → `WhepSink::new`) |
| B (migrated) | HTTP `MIGRATION_COMMAND_BIND` + `Smoke Graph` quick-action | `senders/android/src/migration/runtime.rs` + `node_manager.rs` |

After this phase, **only Surface B exists**. The `Event::StartCast` /
`Event::EndSession` handlers become ~50-line adapters that:

1. Issue `CreateScreenCaptureSource` (PHASE-4 node).
2. Issue `CreateDestination { family: Whep { server_port: 0 }, … }`
   (PHASE-5 family).
3. Issue `Connect { src_id, sink_id, video: true }`.
4. Issue `Start { id: dst_id }`.
5. Poll `getinfo` until `bound_port_v4` is populated, then send the
   WHEP URL to the active FCast receiver via `device.load(...)` —
   exactly as the legacy `Event::SignallerStarted` handler did
   (`lib.rs:754-794`).

On `Event::EndSession` / `Event::CaptureStopped`, the adapter issues
`Disconnect` + `Remove` for both nodes, and `mcore::transmission::WhepSink`
becomes dead code for Android (kept only for desktop, behind
`#[cfg(not(target_os = "android"))]`).

---

## 1. Pre-flight

### 1.1 What MUST be shipped before this phase

| Prerequisite | Where |
|---|---|
| MVP-PHASE-4 (`Command::CreateScreenCaptureSource`) | `senders/android/src/migration/protocol.rs`, `nodes/screen_capture.rs` |
| MVP-PHASE-5 (`DestinationFamily::Whep` + `DestinationInfo.bound_port_v*`) | `senders/android/src/migration/protocol.rs`, `nodes/destination.rs` |
| MVP-PHASE-3 (Surface B runtime starts on app launch) | `senders/android/src/lib.rs:1035` (`start_graph_runtime()`) — already shipped pre-MVP |

If any of those is missing, this phase will compile but its smoke
test will fail at step 2 (CreateDestination returns "Unknown family"
or step 1 returns "Unknown command").

### 1.2 The five touch points in `lib.rs`

| # | Line | What it does today | What it does after this phase |
|---|---|---|---|
| 1 | `lib.rs:738-746` | `Event::EndSession` → `stop_cast(true)` (legacy WhepSink shutdown). | Issue `Disconnect L1 + Remove cap-1 + Remove tv-1` graph commands, then `stop_cast` calls become no-ops. |
| 2 | `lib.rs:754-794` | `Event::SignallerStarted` → build WHEP URL → `device.load(...)`. | Replaced by a `tokio::spawn` polling `getinfo` until `bound_port_v4` is `Some(_)`, then identical `device.load(...)`. |
| 3 | `lib.rs:875-961` | `Event::CaptureStarted` + `Event::StartCast` → `appsrc` + `WhepSink::new`. | Issue `CreateScreenCaptureSource cap-1 + CreateDestination tv-1 + Connect L1 + Start tv-1` graph commands. |
| 4 | `lib.rs:704-706` | `stop_cast` → `tx_sink.shutdown()`. | Remove the `tx_sink` field entirely on Android; the shutdown is implicit via `Remove tv-1`. |
| 5 | `lib.rs:537, 602, 943-950` | `tx_sink: Option<WhepSink>` field + `tx_sink = Some(WhepSink::new(...))`. | Delete the field on `#[cfg(target_os = "android")]`. |

Approximate scope: **~200–300 lines of Rust, all in one file
(`senders/android/src/lib.rs`)**, net –300 +200 (the cast loop body
collapses to a sequence of `handle_command` calls).

### 1.3 Why one big diff and not five small ones?

The five touch points share state (`tx_sink`, `our_source_url`,
`local_address`, `current_device_id`). Touching them piecemeal would
leave intermediate commits with broken invariants (e.g. `tx_sink =
None` but `Event::SignallerStarted` still expecting to read it). Land
this as one atomic PR, and keep the legacy paths behind
`#[cfg(not(target_os = "android"))]` so the desktop sender is
unaffected.

---

## 2. Steps — split into nine per-step files

To keep each step skimmable and reviewable in isolation, the
implementation is split across nine per-step `MVP-PHASE-6-STEP-N-*.md`
files. Each file follows the same smaller five-section template
(Goal-of-this-step / Pre-flight / The change / Verification /
Next step) and is self-contained — you don't need to flip back to
this parent doc while implementing a single step.

| # | File | Scope | Net diff |
|---|---|---|---|
| 1 | [`MVP-PHASE-6-STEP-1-node-id-constants.md`](./MVP-PHASE-6-STEP-1-node-id-constants.md) | Add three `const &str` IDs (`CAST_SOURCE_ID`, `CAST_DESTINATION_ID`, `CAST_LINK_ID`) at the top of `lib.rs`. | ~6 lines, 1 file (`lib.rs`) |
| 2 | [`MVP-PHASE-6-STEP-2-capturestarted-rewrite.md`](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) | Replace `Event::CaptureStarted` body (legacy `WhepSink::new`) with a sequence of graph commands + a `tokio::spawn` poll loop on `bound_port_v4`. **Largest step.** Adds three `last_cast_request_*` fields populated by `Event::StartCast`. | ~150 lines, 1 file (`lib.rs`) |
| 3 | [`MVP-PHASE-6-STEP-3-signaller-started-helper.md`](./MVP-PHASE-6-STEP-3-signaller-started-helper.md) | Extract `mcore::transmission::build_whep_play_msg(addr, port)` and switch `Event::SignallerStarted` to call it directly (no `tx_sink` dependency). | ~10 SDK lines + ~3 `lib.rs` lines |
| 4 | [`MVP-PHASE-6-STEP-4-stop-cast-rewrite.md`](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md) | Replace `tx_sink.take().shutdown()` inside `stop_cast(...)` with `Disconnect L + Remove src + Remove dst` graph commands. | ~30 lines, 1 file (`lib.rs`) |
| 5 | [`MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md`](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md) | Gate the `tx_sink: Option<WhepSink>` field + initialiser behind `#[cfg(not(target_os = "android"))]`. Compiler catches every remaining read. | ~10 lines, 1 file (`lib.rs`) |
| 6 | [`MVP-PHASE-6-STEP-6-frame-pair-unchanged.md`](./MVP-PHASE-6-STEP-6-frame-pair-unchanged.md) | **Documentation-only checkpoint.** Confirm the JNI-side `FRAME_PAIR` producer is untouched by PHASE-6 — only the consumer (`ScreenCaptureNode::wire_need_data`) changes. | 0 source lines |
| 7 | [`MVP-PHASE-6-STEP-7-set-capture-active-preservation.md`](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md) | **Preservation step.** Confirm `set_capture_active(false)` calls in `Event::CaptureStopped` / `Event::CaptureCancelled` are preserved (they unblock the JNI producer). | 0 source lines (or 1 optional defensive call) |
| 8 | [`MVP-PHASE-6-STEP-8-mod-migration-exports.md`](./MVP-PHASE-6-STEP-8-mod-migration-exports.md) | Add `pub use protocol::{Command, CommandResult, DestinationFamily, NodeInfo};` to `migration/mod.rs`. Optional cosmetic step. | 1 line, 1 file (`migration/mod.rs`) |
| 9 | [`MVP-PHASE-6-STEP-9-optional-feature-flag.md`](./MVP-PHASE-6-STEP-9-optional-feature-flag.md) | **Optional, opt-in.** Add `FCAST_UNIFIED_CAST_GRAPH=0/1` runtime kill-switch around Step 2 + Step 4 bodies. Recommended **only** if you need a canary rollout. | ~30 lines, 1 file (`lib.rs`) — most teams should skip this step |

### Recommended landing order

```
Step 1 ──► Step 2 ──► Step 3 ──► Step 4 ──► Step 5 ──┐
                                                     ├── single squash-commit
Step 6 (doc-only) + Step 7 (preservation)            │   (lib.rs compiles only
                                                     ▼   once Steps 1+2+3+4+5 are all in)
                                  Step 8 (cosmetic — anytime after Step 2)

                                  Step 9 (optional kill-switch — anytime,
                                          but DON'T pair with Step 5 deletion)
```

Steps 1–5 are **all required to compile**; `tx_sink` references move
around between commits 1-4, and Step 5 is what finally deletes the
field on Android. The cleanest path is squashing Steps 1+2+3+4+5
into one commit so the tree compiles between commits. Steps 6 and 7
are documentation/preservation — they don't change code. Step 8 is
cosmetic re-exports. Step 9 is opt-in and **conflicts** with Step 5
(keeping the legacy path alive at runtime means `tx_sink` must
stay).

---

## 2b. Why the per-step split?

The original monolithic §2 block ran to ~420 lines with nine
sub-steps interleaved. Splitting it gives:

- Per-step files small enough to review on a phone screen.
- Independent verification recipes per step (each step's §3 covers
  only that step's compile/test/grep checks).
- Step-specific pitfalls without scrolling past unrelated content.
- Easy follow-up PRs: if a reviewer asks for changes on Step 4
  only, you edit one file.

The pattern mirrors the per-step split applied to PHASE-4, PHASE-5,
and PHASE-8 in the same PR.

---

> **Looking for inline §2.1 — §2.9?** The per-step content has
> moved into the nine `MVP-PHASE-6-STEP-N-*.md` files listed in
> the table above. Each STEP file is self-contained — Goal,
> Pre-flight, The change, Verification, and Pitfalls for that
> step alone.

---


## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android

# Non-Android targets must still build with the legacy WhepSink:
cargo +nightly check -p fcast-sender-desktop
```

Both clean.

### 3.2 Unit tests

```bash
cargo +nightly test -p fcast-sender-android \
    migration::node_manager::tests
```

All previously-passing tests still pass. No new tests in this phase —
the change is structural, not behavioural; behaviour is covered by the
PHASE-4 and PHASE-5 tests.

### 3.3 On-device smoke

```bash
adb install -r target/aarch64-linux-android/release/apk/fcast-sender-android.apk
adb shell am force-stop org.fcast.android.sender
adb shell am start -n org.fcast.android.sender/.MainActivity

# Filter the cast-loop graph commands.
adb logcat | grep -E 'CAST_SOURCE_ID|CAST_DESTINATION_ID|CAST_LINK_ID|handle_command|SignallerStarted'
```

**Expected sequence on a successful cast:**

```
… on_connect_receiver: "Living Room TV"
… Handling event: ConnectToDevice(...)
… Handling event: FromDevice(... StateChanged(Connected ...))
… ChangeState(SelectingSettings)
… on_start_casting: 1280, 720, 30
… Handling event: StartCast { scale_width: 1280, scale_height: 720, max_framerate: 30 }
… Java method call: startScreenCapture(1280, 720, 30)
… ChangeState(WaitingForMedia)
… Handling event: CaptureStarted
… NodeManager::dispatch(CreateScreenCaptureSource { id: "cast-screen-1", ... })
… NodeManager::dispatch(CreateDestination { id: "cast-whep-1", family: Whep(...), ... })
… NodeManager::dispatch(Connect { link_id: "cast-link-1", ... })
… NodeManager::dispatch(Start { id: "cast-whep-1", ... })
… NodeManager::dispatch(Start { id: "cast-screen-1", ... })
… ChangeState(Casting)
… [tokio::spawn] getinfo … bound_port_v4: Some(39871), bound_port_v6: Some(39872)
… Handling event: SignallerStarted { bound_port_v4: 39871, bound_port_v6: 39872 }
… Sending play message: application/sdp http://192.168.1.42:39871/endpoint
```

**On stop:**

```
… on_stop_casting
… Handling event: EndSession { disconnect: true }
… ChangeState(Disconnected)
… Java method call: stopCapture()
… NodeManager::dispatch(Disconnect { link_id: "cast-link-1" })
… NodeManager::dispatch(Remove { id: "cast-screen-1" })
… NodeManager::dispatch(Remove { id: "cast-whep-1" })
… Disconnecting from active device
```

### 3.4 End-to-end cast

With a real FCast receiver on the same network:

1. Open the sender app.
2. Tap a discovered receiver row (relies on MVP-PHASE-1).
3. Confirm consent on the MediaProjection prompt.
4. The receiver displays the phone screen within ~2 s.
5. Tap **Stop** in the sender.
6. The receiver returns to its idle screen within ~1 s.

If any of these fail, check §3.3's log filter for the exact graph
command that didn't return `success`.

### 3.5 Negative test — kill the runtime mid-cast

```bash
adb logcat | grep -E 'shutdown_graph_runtime|stop_cast'
```

While casting, run:

```bash
adb shell run-as org.fcast.android.sender kill -SIGUSR2 $(adb shell pidof org.fcast.android.sender)
```

…or just background-kill the activity. On the next foreground, the
app should re-issue `start_graph_runtime()` (lib.rs:1035) and the
graph should be empty (no leftover nodes). Confirm with:

```bash
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | jq '.result.info.nodes'
# → {}
```

---

## 4. Common pitfalls

### P1 — `Event::SignallerStarted` fires twice

If both the legacy `WhepSink::new` path **and** the new graph path are
active simultaneously (e.g. you skipped Step 5's `#[cfg]` gate), you'll
get two `Event::SignallerStarted` events with different ports, and
the receiver will be told to pull from a random one of the two
servers — usually the wrong one. Symptom: the receiver shows "Cannot
connect to WHEP endpoint" or the stream is choppy.

**Fix:** ensure `self.tx_sink` is `#[cfg(not(target_os = "android"))]`
and that no Android-path code constructs `WhepSink::new`.

### P2 — `getinfo` returns `Info`, not `Success`

The `CommandResult::Info(snapshot)` variant is distinct from
`CommandResult::Success`. The §2.2 poll loop must `match` on `Info` —
matching on `Success` will silently drop the snapshot.

```rust
match crate::migration::runtime::handle_command(Command::GetInfo { id: Some(...) }) {
    CommandResult::Info(snapshot) => { /* use snapshot.nodes */ }
    CommandResult::Success | CommandResult::Error(_) => { /* unexpected */ }
}
```

### P3 — `last_cast_request_*` not populated → 0×0 capture

If `Event::CaptureStarted` fires before `Event::StartCast`'s
`last_cast_request_*` setters run, the unwrap-or-defaults at the top
of §2.2 kick in: 1280×720@30. That's a safe default but may not
match what the user selected in the settings page. To debug:

```bash
adb logcat | grep -E 'last_cast_request_scale|CreateScreenCaptureSource'
```

If you see `width: 1280, height: 720` but the user picked 1920×1080,
the event ordering is wrong. Fix by populating the `last_cast_request_*`
fields **synchronously** in the `Bridge.on_start_casting` callback
itself (lib.rs:1809-1819), not inside the async event handler.

### P4 — Removing a destination doesn't tear down its WHEP server

If the `Remove` command for `cast-whep-1` doesn't actually call
`teardown_live_pipeline()`, the WHEP TCP listener (in
`whep_signaller.rs::imp::Signaller`) stays bound until the process
exits. Symptom: a stale port stays open, and the next cast picks up
a *different* port — confusing logs, but not a correctness bug.

**Fix:** verify that `NodeManager::remove_node()` calls
`node.stop()` before dropping the `NodeRecord`. Search:

```bash
grep -n 'fn remove_node\|teardown_live_pipeline' \
    senders/android/src/migration/node_manager.rs \
    senders/android/src/migration/nodes/destination.rs
```

### P5 — The `tokio::spawn` poll loop outlives the cast

If the user starts a second cast before the first one's poll loop
times out, two poll loops race to emit `Event::SignallerStarted`.
Both will succeed (the runtime returns the latest `bound_port_v*`),
but the second `device.load(...)` may be issued before the first
WHEP server is fully torn down, leading to a brief "stream switching"
hiccup on the receiver.

**Mitigation:** include a generation counter in `Event::SignallerStarted`
or just check `self.active_device.is_some()` before issuing
`device.load(...)`. Pragmatic fix: the user can't realistically
start two casts inside the 20s timeout window — defer.

### P6 — Desktop sender still uses `WhepSink`

The diff in §2.5 puts `tx_sink` behind `#[cfg(not(target_os = "android"))]`.
The desktop sender (`senders/desktop/`) **also** uses `WhepSink` but
through a different binary entry point. The migration runtime is
Android-only for now. **Do not** propagate this change to the desktop
sender in this PR.

### P7 — `MIGRATION_COMMAND_BIND` is not required for the in-process path

`migration::runtime::handle_command(...)` is a **direct Rust call** —
it doesn't go through HTTP. So the cast loop works even when the
HTTP command server isn't bound. Don't make the cast loop depend on
`MIGRATION_COMMAND_BIND` being set; that's only for external smoke
testing.

---

## 5. Stop conditions

The phase is "done" when:

1. `cargo check` is clean across all targets in
   `senders/android/Cargo.toml`.
2. All migration-runtime unit tests still pass.
3. The on-device cast in §3.3 / §3.4 succeeds end-to-end, with the
   exact graph-command log sequence in §3.3.
4. **No call to `mcore::transmission::WhepSink::new` remains under
   `#[cfg(target_os = "android")]`:**

```bash
grep -n 'WhepSink::new' senders/android/src/lib.rs
# → expect: zero matches, OR only matches inside #[cfg(not(target_os = "android"))]
```

5. **No `self.tx_sink` read remains under `#[cfg(target_os = "android")]`:**

```bash
grep -nB1 'self\.tx_sink' senders/android/src/lib.rs | grep -v 'cfg(not'
# → expect: zero matches that aren't already cfg-gated.
```

6. The on-stop tear-down in §3.3 logs `Remove(cast-screen-1)` and
   `Remove(cast-whep-1)` in that order — both within 200ms of the
   `EndSession` event.

---

## 6. Why this matters

This is the **final unification step**. After it ships:

| Surface | Status |
|---|---|
| Surface A (legacy WHEP cast loop on Android) | **Deleted.** The `Event::StartCast` body is now a 4-command graph builder. |
| Surface B (migration runtime) | The single canonical pipeline construction site for Android. |
| `mcore::transmission::WhepSink` | Desktop-only. Android cfg-gated out. |

This brings the FCast sender into alignment with the migration
runtime's design goal: **one node-graph API for every pipeline**,
whether the user assembled it via HTTP, JNI, or the cast loop.

Downstream cleanup (out of scope, but enabled by this phase):

- Removing the `mcore::transmission::WhepSink::new` Android path
  entirely (current cfg-gated dead code).
- Reusing the runtime's `Mixer` node for picture-in-picture during
  cast (today impossible because the WHEP sink lives outside the
  graph).
- Moving the cast loop's bitrate/resolution settings to live as
  `AddControlPoint` commands on the destination, rather than
  encoder constructor args.
- Recording (PHASE-23) becomes a second `Destination::LocalFile`
  node connected to the same source — no duplicate encoder chain.

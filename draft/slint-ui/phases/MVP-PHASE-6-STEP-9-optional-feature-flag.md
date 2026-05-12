# MVP-PHASE-6 — Step 9: optional runtime feature flag for canary rollout

> Part 9 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 8 — `mod.rs` re-exports](./MVP-PHASE-6-STEP-8-mod-migration-exports.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a **runtime** feature flag —
`FCAST_UNIFIED_CAST_GRAPH=0/1` — that toggles between the
legacy `WhepSink::new`-based cast loop and the new unified
graph-command path from [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
and [Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md).

Default: `1` (= use the unified path). Setting `0` reverts to the
legacy path on a per-process basis without rebuilding.

This is an **optional, opt-out step**. The whole point of PHASE-6 is
to delete the legacy path — and the flag adds non-trivial
maintenance burden (the legacy code can't actually be deleted while
the flag exists). Most teams should **skip this step** and land the
big-bang switch directly.

When to take this step:

- You have a release shipping to thousands of users and want to ship
  a runtime kill-switch.
- Your CI test matrix can't exercise the new path on real device
  hardware before release.
- You want to A/B test latency / battery / image quality between the
  two paths.

When to **skip** this step:

- You're confident PHASE-6 has been smoke-tested on enough devices.
- You're willing to roll back the entire PR if something breaks.
- You're optimising for code-deletion (the legacy path is dead
  weight as long as the flag exists).

---

## 1. Pre-flight

### 1.1 Live state — what the flag must gate

The flag must gate everything the cast loop touches: `tx_sink`
constructor, `WhepSink::shutdown`, and the new graph-command body.
That spans **all** of Step 2 + Step 4 + Step 5.

| Site | Legacy path | Unified path |
|---|---|---|
| `Event::CaptureStarted` body | `appsrc` + `WhepSink::new(...)` | sequence of `handle_command` + `tokio::spawn` poll loop |
| `Event::SignallerStarted` URL build | `self.tx_sink.as_ref().unwrap().get_play_msg(...)` | `mcore::transmission::build_whep_play_msg(...)` (always available since Step 3) |
| `stop_cast` body | `self.tx_sink.take().shutdown()` | `Disconnect L + Remove src + Remove dst` |
| `tx_sink: Option<WhepSink>` field | Used | Unused but kept (can't `cfg` it away if the legacy path is reachable at runtime) |

### 1.2 Why **runtime** flag, not Cargo feature

A Cargo feature splits the binary into two builds. Releasing both
adds CI cost. A runtime flag (env var) lets the same binary toggle
behaviour at process start.

The cost is dead code in the binary. ~150 KB of `WhepSink` /
`mcore::transmission` glue stays linked. For comparison, the unified
path's GStreamer plugin glue is ~80 KB. Net: ~150 KB extra.

### 1.3 Why env-var (not adb setprop directly)

`std::env::var("FCAST_UNIFIED_CAST_GRAPH")` works in any Rust
context. An Android system property requires JNI + Java side
plumbing. Env-vars are also testable host-side without an emulator.

To set the env-var on-device, use a `BroadcastReceiver` or — for
debug builds — the Android Studio "Edit Configurations" → "App
launch flags" pane.

---

## 2. The change

### 2.1 Add the flag helper

**File:** `senders/android/src/lib.rs` (top, after the constants
added in [Step 1](./MVP-PHASE-6-STEP-1-node-id-constants.md)):

```rust
#[cfg(target_os = "android")]
fn use_unified_cast_graph() -> bool {
    // Default: true. Set FCAST_UNIFIED_CAST_GRAPH=0 to revert to the
    // legacy WhepSink path (PHASE-6 kill-switch).
    std::env::var("FCAST_UNIFIED_CAST_GRAPH")
        .map(|v| v.trim() != "0" && v.trim().to_ascii_lowercase() != "false")
        .unwrap_or(true)
}
```

### 2.2 Gate `Event::CaptureStarted`

**File:** `senders/android/src/lib.rs` (modifying Step 2's
rewrite):

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted => {
    set_capture_active(true);

    if use_unified_cast_graph() {
        // …Step 2's graph-command path…
    } else {
        // …Step 2's "Before" body, unchanged…
        let appsrc = gst_app::AppSrc::builder()
            .caps(/* … */)
            .is_live(true)
            /* … */
            .build();
        appsrc.set_callbacks(/* large need-data closure */);
        let source_config = SourceConfig::Video(mcore::VideoSource::Source(appsrc));
        self.tx_sink = Some(mcore::transmission::WhepSink::new(
            source_config,
            self.event_tx.clone(),
            tokio::runtime::Handle::current(),
            1920, 1080, 30,
        )?);
        // (UI signal — same in both branches; pull below the if/else.)
    }

    self.ui_weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().invoke_change_state(AppState::Casting);
    })?;
}
```

### 2.3 Gate `stop_cast`

**File:** `senders/android/src/lib.rs` (modifying Step 4's
rewrite):

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    // …existing JNI stopCapture call + active_device cleanup…

    #[cfg(target_os = "android")]
    if use_unified_cast_graph() {
        // …Step 4's graph-command path…
    } else {
        // Legacy WhepSink shutdown.
        if let Some(mut tx_sink) = self.tx_sink.take() {
            tx_sink.shutdown();
        }
    }

    #[cfg(not(target_os = "android"))]
    if let Some(mut tx_sink) = self.tx_sink.take() {
        tx_sink.shutdown();
    }

    Ok(())
}
```

### 2.4 Don't gate `Event::SignallerStarted`

The URL-builder from [Step 3](./MVP-PHASE-6-STEP-3-signaller-started-helper.md)
(`mcore::transmission::build_whep_play_msg`) is a pure function
that works in both modes (legacy reaches it via the `WhepSink`
wrapper; unified reaches it directly). No gate needed.

### 2.5 Don't gate `tx_sink` (in this step)

Because the legacy path is reachable at runtime when the flag is
`0`, you **cannot** `cfg` the `tx_sink` field as
`#[cfg(not(target_os = "android"))]` ([Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md)).
Either:

- (a) Take **all** of Steps 2 + 4 + 5 + 9 together — keep the field,
  toggle the body. The desktop sender's `#[cfg(not(target_os = "android"))]`
  is unchanged.
- (b) Take Steps 2 + 4 + 5 (no flag), defer Step 9 entirely.

(b) is cleaner — Step 5 deletes the field outright, no flag
needed. Choose (a) only if the canary use case justifies the dead
code.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Clean.

### 3.2 Runtime smoke — both paths

```bash
# Default (unified):
adb shell am start -n org.fcast.android.sender/.MainActivity
adb logcat | grep -E 'use_unified_cast_graph|handle_command'
# Expect: handle_command lines + no WhepSink::new

# Legacy fallback:
adb shell am start --es FCAST_UNIFIED_CAST_GRAPH 0 \
    -n org.fcast.android.sender/.MainActivity
adb logcat | grep -E 'WhepSink::new|tx_sink'
# Expect: WhepSink::new and tx_sink set
```

Cast should work identically in both modes (modulo the chosen
pipeline implementation).

### 3.3 Grep

```bash
grep -n 'fn use_unified_cast_graph' senders/android/src/lib.rs
# → exactly 1 match
grep -nE 'if use_unified_cast_graph\(\)' senders/android/src/lib.rs
# → exactly 2 matches (CaptureStarted + stop_cast)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to gate `stop_cast` too

`stop_cast` runs once per cast. If `Event::CaptureStarted` ran the
unified path but `stop_cast` runs the legacy path, the
`tx_sink.take()` returns `None` and the graph is never torn down.
Both must check the same flag.

### P2 — Reading the env-var twice with different results

If something flips `FCAST_UNIFIED_CAST_GRAPH` mid-cast (unlikely,
but possible via `setenv` from a child process), `use_unified_cast_graph()`
in `stop_cast` returns a different value than in
`Event::CaptureStarted`. Defence: cache the flag in
`EventLoopState` at construction:

```rust
struct EventLoopState {
    // …existing fields…
    unified_cast_graph: bool,
}
// At construction:
unified_cast_graph: use_unified_cast_graph(),
// Read site:
if self.unified_cast_graph { … } else { … }
```

### P3 — Cargo features look tempting but are wrong

A Cargo `default-features = ["unified-cast-graph"]` feature would
also work, but:

- Requires two CI build matrices.
- Can't be toggled by end users.
- Splits the binary distribution.

Runtime env-var is the right tool here.

### P4 — `setprop` vs env var

`adb shell setprop debug.fcast.unified_cast_graph 0` sets an
Android system property, not an env var. To bridge, you'd need:

```kotlin
// MainActivity.java (Kotlin)
val v = SystemProperties.get("debug.fcast.unified_cast_graph", "1")
ProcessBuilder.environment().put("FCAST_UNIFIED_CAST_GRAPH", v)
```

This is extra Java glue. For development, `am start --es` (as
shown in §3.2) is simpler.

### P5 — Mixed-mode cast — start unified, stop legacy

Don't allow this. The `tx_sink` field is empty in unified mode; a
legacy-mode `stop_cast` would no-op silently. Use the cached
`self.unified_cast_graph` from P2 to guarantee consistency.

### P6 — Feature-flag drift

If you take Step 9, plan a **deletion date** for the flag (and the
legacy code) — e.g. "remove in v1.5" or "after 30 days of clean
telemetry". Otherwise the legacy path lives forever and the
benefits of PHASE-6 are diluted.

---

## 5. Next step

**This is the last step of PHASE-6.** Run the full verification
recipe in the parent doc:

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
cargo +nightly check -p fcast-sender-desktop
```

…then proceed to [MVP-PHASE-7](./MVP-PHASE-7-receiver-item-promotion.md)
which promotes `Bridge.devices` from `[string]` to
`[ReceiverItem]` and threads `connect_page.slint` to use the
richer model.

# MVP-PHASE-6 — Step 4: replace `stop_cast` / `Event::EndSession` with graph teardown commands

> Part 4 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 3 — extract WHEP-URL helper](./MVP-PHASE-6-STEP-3-signaller-started-helper.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Replace `self.tx_sink.take().shutdown()` inside `stop_cast(...)` with
a sequence of graph teardown commands:

```
Disconnect cast-link-1
Remove     cast-source-1
Remove     cast-destination-1
```

On non-Android targets, the existing `WhepSink::shutdown()` path
remains via `#[cfg(not(target_os = "android"))]`.

After this step, the cast loop on Android is fully driven by the
migration runtime — no `WhepSink` references survive outside of
`#[cfg(not(target_os = "android"))]` blocks.

This is a **medium-sized step** (~30 Rust lines in `stop_cast`).
Pair with [Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md) which
gates the now-dead `tx_sink` field.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `stop_cast(...)` | `senders/android/src/lib.rs:682-709` |
| `Event::EndSession` handler | `lib.rs:738-746` (calls `self.stop_cast(true).await?`) |
| `self.tx_sink.take().shutdown()` | `lib.rs:702-706` (currently the legacy shutdown) |
| `Command::Disconnect` | `protocol.rs:55-58` |
| `Command::Remove` | `protocol.rs:60-63` |
| `migration::runtime::handle_command` | `senders/android/src/migration/runtime.rs` |
| `MainActivity.stopCapture` JNI call | `lib.rs:685-690` (kept — tells the JNI side to stop pushing frames) |

### 1.2 Why `Disconnect` before `Remove`

`Remove src` of a still-linked node implicitly removes its
consumer links and tears down `StreamBridge` instances. The
existing semantics of `Remove` already cover this — `Disconnect`
is technically optional. But making it explicit:

- Keeps the teardown order observable in adb logcat.
- Mirrors the `Connect` shape in [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
  (we explicitly `Connect L1`, we should explicitly `Disconnect L1`).
- Lets `Remove` calls fail gracefully if the source was never
  successfully constructed (e.g. partial `CaptureStarted` failure).

### 1.3 Why `_ = handle_command(...)` (ignore the result)

Teardown should be best-effort: if the runtime says "no such node
cast-source-1" because `Event::CaptureStarted` failed before
creating it, we log via the runtime's own diagnostics and move on.
Don't propagate errors from `stop_cast` — the caller
(`Event::EndSession`) expects an idempotent tear-down.

### 1.4 Why keep the JNI `MainActivity.stopCapture` call

That call tells the JNI side to stop pushing frames into
`FRAME_PAIR`. The migration runtime can stop reading frames
(`ScreenCaptureNode` removed), but the producer keeps running
until told to stop. See [Step 7](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md)
for the full picture.

---

## 2. The change

**File:** `senders/android/src/lib.rs`

### 2.1 Rewrite `stop_cast`

**Before** (lines 682-709):

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    let android_app = self.android_app.clone();
    self.ui_weak.upgrade_in_event_loop(move |_| {
        call_java_method_no_args(&android_app, JavaMethod::StopCapture);
    })?;

    if let Some(active_device) = self.active_device.take() {
        tokio::spawn(async move {
            if stop_playback {
                debug!("Stopping playback");
                log_err!(active_device.stop_playback(), "Failed to stop playback");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            debug!("Disconnecting from active device");
            log_err!(active_device.disconnect(), "Failed to disconnect from active device");
        });
    }

    if let Some(mut tx_sink) = self.tx_sink.take() {
        tx_sink.shutdown();
    }

    Ok(())
}
```

**After:**

```rust
async fn stop_cast(&mut self, stop_playback: bool) -> Result<()> {
    let android_app = self.android_app.clone();
    self.ui_weak.upgrade_in_event_loop(move |_| {
        call_java_method_no_args(&android_app, JavaMethod::StopCapture);
    })?;

    if let Some(active_device) = self.active_device.take() {
        tokio::spawn(async move {
            if stop_playback {
                debug!("Stopping playback");
                log_err!(active_device.stop_playback(), "Failed to stop playback");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            debug!("Disconnecting from active device");
            log_err!(active_device.disconnect(), "Failed to disconnect from active device");
        });
    }

    // NEW — tear down the unified cast graph. Removing the source
    // implicitly disconnects all of its consumer links; removing the
    // destination tears down its pipeline including the WHEP signaller.
    // Best-effort: ignore "no such node" errors from a partial cast.
    #[cfg(target_os = "android")]
    {
        use crate::migration::protocol::Command;
        for id in [CAST_LINK_ID] {
            let _ = crate::migration::runtime::handle_command(Command::Disconnect {
                link_id: id.into(),
            });
        }
        for id in [CAST_SOURCE_ID, CAST_DESTINATION_ID] {
            let _ = crate::migration::runtime::handle_command(Command::Remove {
                id: id.into(),
            });
        }
    }

    // Legacy WhepSink shutdown — kept only on non-Android targets.
    #[cfg(not(target_os = "android"))]
    if let Some(mut tx_sink) = self.tx_sink.take() {
        tx_sink.shutdown();
    }

    Ok(())
}
```

### 2.2 `Event::EndSession` stays the same

The handler at `lib.rs:738-746` already calls
`self.stop_cast(true).await?` — no change needed. The new
behaviour is entirely inside `stop_cast`.

### 2.3 Why two separate `for id in […]` loops

Disconnect first (singular `[CAST_LINK_ID]`), then remove
(`[CAST_SOURCE_ID, CAST_DESTINATION_ID]`). The split lets the
migration runtime process them in the right order — `Disconnect`
must commit before `Remove` of either endpoint sees it as
"orphaned" (which it would handle correctly, but logs noise).

### 2.4 Idempotency on second `stop_cast` call

If `stop_cast` runs twice in a row (e.g. user double-taps Stop),
the second call's `handle_command` returns `Error("no such node
cast-source-1")` for each `Remove`. We swallow that error via
`let _ = ...`, so the second call is a no-op (modulo the
`MainActivity.stopCapture` JNI call, which is itself idempotent).

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
cargo +nightly check -p fcast-sender-desktop
```

Both clean. The Android path no longer references
`self.tx_sink.take()`; the desktop path still does.

### 3.2 Smoke (on-device)

```bash
# After tapping "Stop cast":
adb logcat | grep -E '(Disconnect|Remove cast-)'
# Expect:
#   handle_command Disconnect link_id=cast-link-1 ok
#   handle_command Remove id=cast-source-1 ok
#   handle_command Remove id=cast-destination-1 ok
```

Then `getinfo` should show an empty `nodes` map (or just nodes
unrelated to this cast):

```bash
curl -X POST http://127.0.0.1:8080/command -d '{"getinfo":{}}' | \
    jq '.result.info.nodes | keys'
# → []
```

### 3.3 Grep

```bash
grep -nE 'tx_sink\.take' senders/android/src/lib.rs
# → 1 match (under #[cfg(not(target_os = "android"))])
grep -nE 'handle_command.*Disconnect|handle_command.*Remove' senders/android/src/lib.rs
# → ≥ 3 matches (the three teardown commands in stop_cast)
```

---

## 4. Pitfalls specific to this step

### P1 — Don't `await` on `handle_command`

`migration::runtime::handle_command` is synchronous. Awaiting it
gives a compile error. The teardown loop runs synchronously inside
`stop_cast`'s async body — that's fine because the runtime call is
cheap (it just locks the global `NodeManager` Mutex briefly).

### P2 — `unused variable: _` warnings

`let _ = handle_command(...)` is the canonical pattern. Don't
suppress the warning with `#[allow(unused_must_use)]` at function
scope — the per-call `let _ =` is intentional self-documenting code.

### P3 — `for id in [CAST_LINK_ID]` is a 1-element loop

Tempting to inline it as:

```rust
let _ = handle_command(Command::Disconnect { link_id: CAST_LINK_ID.into() });
```

That's fine. The loop form is just symmetric with the two-element
`[CAST_SOURCE_ID, CAST_DESTINATION_ID]` loop below it. Pick
whichever the reviewer prefers.

### P4 — Disconnecting a non-existent link is OK

The migration runtime's `Disconnect { link_id: "cast-link-1" }`
returns `Error("link cast-link-1 not found")` if the link was
never created. The `let _ =` swallow handles this. Don't add
an `unwrap()` or panic on the error — partial teardown is
expected.

### P5 — Removing the destination before the source

The order `Disconnect L → Remove src → Remove dst` is safe.
Reverse order (`Remove dst → Remove src`) is **also** safe because
`Remove` on a still-linked node implicitly disconnects its
consumer links. But the logs in adb logcat are easier to read when
the source goes first.

### P6 — Tokio `spawn` for the device-stop closure

The existing handler spawns a tokio task to stop the receiver's
playback. **Don't** move the graph teardown inside that spawn — we
want teardown to complete synchronously before `stop_cast` returns
so that a subsequent `start_cast` doesn't race with leftover
nodes still being torn down. The "stop playback" task is
intentionally async (network round-trip to receiver), but graph
teardown is local.

### P7 — `#[cfg(target_os = "android")]` block scoping

The new block must be inside the `async fn stop_cast` body, not
at the file's top level. Otherwise you can't reference
`self.event_tx` from within (which the snippet above doesn't, but
future evolution might).

---

## 5. Next step

Once this lands, [Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md)
gates the `tx_sink: Option<WhepSink>` struct field behind
`#[cfg(not(target_os = "android"))]` so it no longer exists on
the Android sender.

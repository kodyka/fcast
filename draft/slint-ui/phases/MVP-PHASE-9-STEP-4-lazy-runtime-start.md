# MVP-PHASE-9 — Step 4: lazy `migration::runtime` startup

> Part 4 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
> Previous: [Step 3 — quick-actions rewrite](./MVP-PHASE-9-STEP-3-quick-actions-rewrite.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Remove the unconditional
`crate::migration::runtime::start_graph_runtime()` call at the
top of `Application::run_event_loop()` (`lib.rs:1110-1112`). Rely
on the idempotent on-demand startup that each consuming call site
already performs (or, where it doesn't, add a one-line ensure-call).

The matching shutdown at `lib.rs:1128` **stays** — it cleans up
whatever state the on-demand calls created. Calling
`shutdown_graph_runtime()` when the runtime was never started is
already a no-op (`runtime.rs:312-320`).

After this step:

- Release builds do not bind port 8080 on launch (verified by
  `adb shell netstat -an | grep 8080` returning no match before
  the user starts a cast).
- Debug builds bind port 8080 on the first
  `start-migration-server` Bridge callback (i.e. the first time
  the user taps the `Migrated srv` quick-action, or the first
  graph command from `Event::CaptureStarted` flows through).
- The four debug tests still pass on first invocation (they each
  call `start_migrated_command_server` which calls
  `start_graph_runtime` internally — already idempotent).

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Application::run_event_loop` (function entry) | `senders/android/src/lib.rs:1099` |
| Unconditional `start_graph_runtime()` call | `senders/android/src/lib.rs:1110-1112` |
| Matching `shutdown_graph_runtime()` on loop exit | `senders/android/src/lib.rs:1128-1130` |
| `start_graph_runtime()` definition (already idempotent) | `senders/android/src/migration/runtime.rs:302-310` |
| `shutdown_graph_runtime()` (safe to call on never-started runtime) | `senders/android/src/migration/runtime.rs:312-320` |
| `start_migrated_command_server()` (calls `start_graph_runtime` internally at line 191) | `senders/android/src/lib.rs:188-198` |
| `Event::CaptureStarted` graph commands (PHASE-6) | `senders/android/src/lib.rs:985-1000` |
| `nativeProcessGraphCommandJson` JNI hook | `senders/android/src/lib.rs:2181-2184` |

### 1.2 Audit: who calls `start_graph_runtime()` today

`grep -nE 'start_graph_runtime\(\)' senders/android/src/lib.rs senders/android/src/migration/`:

```
senders/android/src/lib.rs:191    crate::migration::runtime::start_graph_runtime()
                                  // called from start_migrated_command_server
senders/android/src/lib.rs:1110   if let Err(err) = crate::migration::runtime::start_graph_runtime() {
                                  // the unconditional auto-start we want to delete
senders/android/src/migration/runtime.rs:302 pub fn start_graph_runtime() -> Result<()> {
                                  // the definition
```

So **only one** non-definition call site currently exists outside
`run_event_loop()` — `start_migrated_command_server` at line 191.

Additional consumers that **don't** call `start_graph_runtime`
directly but **do** depend on the runtime being up:

| Consumer | Location | Depends on runtime being started? |
|---|---|---|
| `Event::StopCast` / `Event::EndSession` graph commands | `lib.rs:758-770` (`Disconnect` + `Remove` for cast-link / cast-source / cast-destination) | Yes (must have been started by a prior `StartCast`). |
| `Event::CaptureStarted` graph commands | `lib.rs:985-1000` (`createscreencapturesource`, `createdestination`, `connect`, `start`) | **Yes — this is the new dependency post-MVP-PHASE-6.** |
| `nativeProcessGraphCommandJson` JNI hook | `lib.rs:2181` (`try_handle_command_json`) | Yes — the JNI hook expects the manager to be alive. |
| Step-2 `on_start_migration_server` callback handler | (new this phase) | No, it calls `start_migrated_command_server` which ensures start. |
| Step-2 `on_run_migration_test` callback handler | (new this phase) | No, all three test functions call `start_migrated_command_server` transitively. |
| Step-2 `on_stop_migration_server` callback handler | (new this phase) | No (shutdown is no-op on never-started). |

The **two consumers that don't currently ensure-start** are:

1. `Event::CaptureStarted` — relies on the unconditional auto-start.
2. `nativeProcessGraphCommandJson` (the JNI hook).

This step adds an ensure-start to both.

### 1.3 Idempotency reminder

`start_graph_runtime()` (`runtime.rs:302-310`) is **idempotent**:

```rust
pub fn start_graph_runtime() -> Result<()> {
    {
        let mut manager = GRAPH_NODE_MANAGER.lock();
        manager.start();       // no-op if already started
    }
    ensure_refresh_thread_running()?;   // checks a `static_atomic` flag; no-op if up
    ensure_command_server_running()?;   // same shape
    Ok(())
}
```

So calling it from multiple sites (1110, 191, the two new sites
in §1.2) does no harm. The cost is one mutex lock + two atomic
loads per redundant call — negligible.

### 1.4 Why not delete the call entirely (no ensure-start anywhere)

The two consumers in §1.2 (`Event::CaptureStarted` and the JNI
hook) **must** have the runtime up at the moment they fire. If
`Event::CaptureStarted` fires before any other call ensured the
runtime, the dispatch to
`crate::migration::runtime::handle_command(command)` returns
`CommandResult::Error("no manager")`-shaped errors. So we need an
ensure-start somewhere upstream of those two paths.

The minimal change is:

- **Delete** the call at `lib.rs:1110-1112`.
- **Add** an ensure-start call at the top of `Event::CaptureStarted`
  in the graph-command path (post-PHASE-6).
- **Add** an ensure-start call at the top of the JNI hook
  (`nativeProcessGraphCommandJson`).

The two new ensure-starts are each one line. They're cheap
(idempotent). They make the dependency explicit at each call site.

---

## 2. The change

### 2.1 Remove the unconditional auto-start

**File:** `senders/android/src/lib.rs`

**Before** (lines 1108-1115):

```rust
ensure_gstreamer_initialized()
    .map_err(|err| anyhow::anyhow!("Failed to initialize GStreamer: {err}"))?;
debug!("GStreamer version: {:?}", gst::version());
if let Err(err) = crate::migration::runtime::start_graph_runtime() {
    error!(?err, "Failed to start migrated graph runtime");
}

// self.add_or_update_device(fcast_sender_sdk::device::DeviceInfo::fcast(...
```

**After:**

```rust
ensure_gstreamer_initialized()
    .map_err(|err| anyhow::anyhow!("Failed to initialize GStreamer: {err}"))?;
debug!("GStreamer version: {:?}", gst::version());
// PHASE-9: migration runtime starts on-demand; see
// `MVP-PHASE-9-STEP-4-lazy-runtime-start.md`. The matching
// `shutdown_graph_runtime()` below remains and is a no-op if the
// runtime was never started.

// self.add_or_update_device(fcast_sender_sdk::device::DeviceInfo::fcast(...
```

The matching shutdown at `lib.rs:1128-1130` is **unchanged**:

```rust
debug!("Quitting event loop");
if let Err(err) = crate::migration::runtime::shutdown_graph_runtime() {
    error!(?err, "Failed to shut down migrated graph runtime");
}
```

### 2.2 Add ensure-start to `Event::CaptureStarted`

**File:** `senders/android/src/lib.rs`

**Before** (around the `Event::CaptureStarted` arm; line numbers
approximate, find the first graph-command dispatch line in the
arm body):

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted { /* … */ } => {
    self.our_source_url = None;

    // First command of the PHASE-6 graph-command sequence:
    let result = crate::migration::runtime::handle_command(
        crate::migration::Command::CreateScreenCaptureSource { /* … */ },
    );
    // …
}
```

**After:**

```rust
#[cfg(target_os = "android")]
Event::CaptureStarted { /* … */ } => {
    self.our_source_url = None;

    // PHASE-9: ensure the migration runtime is up before the
    // first graph command. Idempotent; no-op if already started.
    if let Err(err) = crate::migration::runtime::start_graph_runtime() {
        error!(?err, "Failed to start migrated graph runtime");
        return Ok(ShouldQuit::No);
    }

    let result = crate::migration::runtime::handle_command(
        crate::migration::Command::CreateScreenCaptureSource { /* … */ },
    );
    // …
}
```

### 2.3 Add ensure-start to the JNI hook

**File:** `senders/android/src/lib.rs` (around line 2178-2186,
the `nativeProcessGraphCommandJson` JNI export):

**Before:**

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_…_nativeProcessGraphCommandJson(
    /* … env, class, payload args … */
) -> jni::sys::jstring {
    let payload = match jstring_to_string(env, payload) {
        Ok(json) => crate::migration::runtime::try_handle_command_json(&json),
        Err(_) => {
            error!("Failed to convert payload to String");
            crate::migration::runtime::try_handle_command_json("")
        }
    };
    // …
}
```

**After:**

```rust
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_…_nativeProcessGraphCommandJson(
    /* … env, class, payload args … */
) -> jni::sys::jstring {
    // PHASE-9: ensure the migration runtime is up before any
    // JNI-side dispatch. Idempotent.
    if let Err(err) = crate::migration::runtime::start_graph_runtime() {
        error!(?err, "Failed to start migrated graph runtime from JNI hook");
        // Fall through to try_handle_command_json which will
        // surface a structured error in the response.
    }

    let payload = match jstring_to_string(env, payload) {
        Ok(json) => crate::migration::runtime::try_handle_command_json(&json),
        Err(_) => {
            error!("Failed to convert payload to String");
            crate::migration::runtime::try_handle_command_json("")
        }
    };
    // …
}
```

### 2.4 Why not gate the ensure-start with `cfg!`

The two new ensure-starts are in **production** paths
(`Event::CaptureStarted` is the cast loop; the JNI hook is
production tooling). Not debug-only. They must run in both debug
and release builds. The unconditional call at `lib.rs:1110`
that we delete was production-path too — it was just attached to
the wrong scope (event-loop entry instead of first-use).

### 2.5 Why preserve the shutdown call

Two reasons:

1. **Correctness.** If anything started the runtime during the
   session, the shutdown thread joins the refresh thread + closes
   the command server socket. Leaving them dangling on app exit
   leaks an FD and an OS thread.
2. **Safety on never-started.** `shutdown_graph_runtime()`'s body
   (`runtime.rs:312-320`) checks the same `static_atomic` flags
   as `start_graph_runtime()` and skips work if nothing started.
   So calling it from the loop-exit path is harmless even if no
   ensure-start ran.

---

## 3. Verification

### 3.1 Compile

```bash
cargo +nightly check -p android-sender --target aarch64-linux-android
```

Expect **clean**. If `Event::CaptureStarted` doesn't accept the
new early-return on error, refactor to match the existing
PHASE-6 STEP-2 error-handling shape (`self.stop_cast(false).await?`
followed by `return Ok(ShouldQuit::No)` — see
`MVP-PHASE-6-STEP-2-capturestarted-rewrite.md` §2.5 for the
pattern).

### 3.2 Grep

```bash
# 1. The unconditional auto-start is gone.
grep -nE 'start_graph_runtime\(\)' senders/android/src/lib.rs

# Expected matches:
#  - lib.rs:191 (inside start_migrated_command_server)
#  - lib.rs:~983 (new ensure-start in Event::CaptureStarted)
#  - lib.rs:~2178 (new ensure-start in JNI hook)
#  - NO match at lib.rs:1110 (the deletion).
```

### 3.3 Release-build smoke (the lazy-start check)

1. `cargo build --release -p android-sender --target aarch64-linux-android`
   (use whatever your debug-suffix-stripping release setup is).
2. Install on a device.
3. Launch the app.
4. **Before** any user interaction (e.g. before tapping a
   receiver), run:
   ```bash
   adb shell netstat -an | grep 8080
   ```
   Expect **no match**. The migration runtime command server is
   not bound.
5. Tap a receiver and start a cast. The first `Event::CaptureStarted`
   ensure-starts the runtime, but in **release** the HTTP command
   server only binds if `MIGRATION_COMMAND_BIND` env is set.
   Confirm:
   ```bash
   adb logcat | grep -E 'GRAPH_NODE_MANAGER start|command server listening'
   ```
   The first line should appear (manager start); the second
   should **not** unless `MIGRATION_COMMAND_BIND` is set.

### 3.4 Debug-build smoke

1. Repeat steps 1-4 with a `debug_assertions` build.
2. Tap the `Migrated srv` quick-action.
3. Confirm `Bridge.test-status` shows
   `PASS migrated server active bind=0.0.0.0:8080 health=…`
   (same as on `master`).
4. Confirm `adb shell netstat -an | grep 8080` now matches.

### 3.5 Shutdown integrity

1. Cast something, then exit the app.
2. `adb logcat | grep -E 'shutdown_graph_runtime|GRAPH_NODE_MANAGER shutdown'`
   should show the shutdown fires.
3. Subsequent `adb shell netstat -an | grep 8080` returns no match
   (port released).

---

## 4. Pitfalls specific to this step

### P1 — Forgetting one of the two ensure-start sites

`Event::CaptureStarted` and `nativeProcessGraphCommandJson` are
the two production paths that depend on the runtime being up
without going through `start_migrated_command_server`. Miss
either, and you'll get sporadic "no manager"-shaped errors when
the app exercises a cold cast loop.

If you find a third path in the future (e.g. a future debug
panel calling `crate::migration::runtime::handle_command` directly),
add a third ensure-start there. The cost is one line.

### P2 — Returning from `Event::CaptureStarted` on
ensure-start failure

The example in §2.2 returns `Ok(ShouldQuit::No)` on
`start_graph_runtime()` failure. That's right — the cast can't
proceed but the app shouldn't crash. The user sees the
`AppState::Casting` state but no stream. Match the existing
error-handling pattern in PHASE-6 STEP-2: log + `stop_cast(false)
+ return Ok(ShouldQuit::No)`. Don't propagate the error up via
`?` — that would kill the event loop.

### P3 — Calling ensure-start from too deep inside the handler

Put the ensure-start as the **first** thing in
`Event::CaptureStarted` (after the `our_source_url = None`
reset). Don't put it inside the graph-command builder closures —
multiple calls per arm is wasteful (idempotent but still locks the
manager mutex on each call).

### P4 — `start_migrated_command_server` already ensures — don't
add a third ensure inside it

`start_migrated_command_server` at `lib.rs:188-198` already
calls `start_graph_runtime()` at line 191. Don't add a second
ensure-start at the top of its body — the existing call is the
ensure.

### P5 — The "release build doesn't bind 8080" claim is conditional

The HTTP command server only binds if `MIGRATION_COMMAND_BIND` is
set (see `runtime.rs::command_endpoint_bind_from_env`,
`runtime.rs:75-80`). In debug builds, the debug callbacks call
`start_migrated_command_server` which explicitly sets the env
(`lib.rs:190`). In release builds, **nothing** sets it. So the
ensure-start in `Event::CaptureStarted` (release path) starts
the manager + the refresh thread, but **not** the HTTP command
server. That's correct — release doesn't need the HTTP server.
The lazy-start claim is about the **manager** + **refresh
thread**, not the HTTP server.

### P6 — The shutdown is still unconditional — keep it that way

Don't try to skip the shutdown based on a "was started" flag.
The runtime's shutdown function does that check internally. Adding
a second check in the caller is duplicate work and risks getting
out of sync with the runtime's internal state machine.

---

## 5. Next step

Once this lands, [Step 5](./MVP-PHASE-9-STEP-5-debug-cfg-separation.md)
(optional) gates the Step-2 callback registrations with
`#[cfg(debug_assertions)]` to match the existing
`default_quick_actions()` debug-extras gate, and optionally moves
them into a `mod debug_quickactions` submodule.

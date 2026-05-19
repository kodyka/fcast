# MVP-PHASE-9 — Step 3: rewrite `on_invoke_action` debug branches to dispatch via Bridge callbacks

> Part 3 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
> Previous: [Step 2 — Rust handlers](./MVP-PHASE-9-STEP-2-rust-handlers.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Replace the four direct-call branches in `on_invoke_action`
(`lib.rs:2091-2127`) with calls into the Bridge callbacks
registered by Step 2. After this step, the **only** caller of
`start_migrated_command_server`, `run_legacy_http_getinfo_test`,
`run_legacy_http_crossfade_test`, and `run_graph_smoke_test` is
the Step-2 callback handlers — the UI dispatcher
(`on_invoke_action`) becomes a thin redirect.

| Branch | Before | After |
|---|---|---|
| `"migrated-server"` | `let status = match start_migrated_command_server(LEGACY_COMMAND_BIND_ADDR) { … }` | `ui.global::<Bridge>().invoke_start_migration_server(LEGACY_COMMAND_BIND_ADDR.into());` |
| `"test-getinfo"` | `std::thread::spawn(move \|\| { … run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR) … })` | `ui.global::<Bridge>().invoke_run_migration_test("getinfo".into());` |
| `"test-crossfade"` | (analogous) | `ui.global::<Bridge>().invoke_run_migration_test("crossfade".into());` |
| `"test-smoke"` | (analogous, `run_graph_smoke_test()`) | `ui.global::<Bridge>().invoke_run_migration_test("smoke".into());` |

Net effect: ~25 lines deleted, ~4 lines added.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Current `on_invoke_action` handler with the four debug branches | `senders/android/src/lib.rs:2082-2129` |
| Step 1 callbacks (must have landed) | `senders/android/ui/bridge.slint` (added by Step 1) |
| Step 2 handler registrations (must have landed) | `senders/android/src/lib.rs` (added by Step 2, immediately after `on_invoke_action`) |
| `LEGACY_COMMAND_BIND_ADDR` constant | `senders/android/src/lib.rs:99` |
| `default_quick_actions()` (the `cfg!(debug_assertions)` extension that injects the four debug ids) | `senders/android/src/lib.rs:1146-1164` |

### 1.2 What still drives the four debug ids

`default_quick_actions()` continues to emit them under
`cfg!(debug_assertions)`. The user-visible behaviour is
identical: the four buttons still appear in the debug-build's
quick-action drawer, and tapping each still triggers
`invoke-action(id)`. The change is purely **how**
`on_invoke_action`'s `match id_str` arm services them.

### 1.3 Why keep `on_invoke_action` at all

Two options:

- **(a)** Keep `on_invoke_action` as a dispatcher that forwards to
  Bridge callbacks (this step's recommendation).
- **(b)** Have the Slint quick-action button **directly** invoke
  `Bridge.start-migration-server(...)` etc. without going through
  `Bridge.invoke-action(...)`.

Option (b) is cleaner conceptually, but requires changes inside
the quick-action page (`senders/android/ui/pages/debug_page.slint`
or wherever the quick-actions render) — and the existing
`invoke-action(id)` indirection is used by many other id strings
(`scan-qr`, `settings`, `debug`, `pair`, etc.). Changing the
dispatch semantics of `invoke-action` for just these four ids
breaks the abstraction.

**Choose (a).** STEP-3 keeps the existing `invoke-action(id)`
abstraction intact; it just changes what each arm does. A future
phase can refactor to (b) if quick-actions become per-callback,
but that's a Cluster F2-shaped change.

### 1.4 Why `LEGACY_COMMAND_BIND_ADDR.into()`

`invoke_start_migration_server` takes a `SharedString` (Slint's
string type). `&str → SharedString` via `.into()` is the
idiomatic conversion. `LEGACY_COMMAND_BIND_ADDR` is `&'static str`
(see `lib.rs:99`), so `.into()` is zero-cost (no heap allocation
for short ASCII strings).

---

## 2. The change

### 2.1 The current `match id_str` body

**Before** (`senders/android/src/lib.rs:2091-2127`):

```rust
"migrated-server" => {
    let status = match start_migrated_command_server(LEGACY_COMMAND_BIND_ADDR) {
        Ok(message) => format!("PASS {message}"),
        Err(err) => format!("FAIL {err}"),
    };
    log_ui_test_status("start-migrated-server", &status);
    let _ = ui_weak.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().set_test_status(status.into());
    });
}
"test-getinfo" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running legacy getinfo test...".into()));
    let ui_weak_clone = ui_weak.clone();
    std::thread::spawn(move || {
        let status = run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR);
        log_ui_test_status("legacy-getinfo", &status);
        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
    });
}
"test-crossfade" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running legacy crossfade test...".into()));
    let ui_weak_clone = ui_weak.clone();
    std::thread::spawn(move || {
        let status = run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR);
        log_ui_test_status("legacy-crossfade", &status);
        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
    });
}
"test-smoke" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| ui.global::<Bridge>().set_test_status("Running graph smoke test...".into()));
    let ui_weak_clone = ui_weak.clone();
    std::thread::spawn(move || {
        let status = run_graph_smoke_test();
        log_ui_test_status("graph-smoke", &status);
        let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| ui.global::<Bridge>().set_test_status(status.into()));
    });
}
```

### 2.2 The replacement

**After** (same location, much shorter):

```rust
"migrated-server" => {
    // PHASE-9: dispatch via Bridge callback (Step 2 handler runs the work).
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.global::<Bridge>().invoke_start_migration_server(
            LEGACY_COMMAND_BIND_ADDR.into(),
        );
    });
}
"test-getinfo" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.global::<Bridge>().invoke_run_migration_test("getinfo".into());
    });
}
"test-crossfade" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.global::<Bridge>().invoke_run_migration_test("crossfade".into());
    });
}
"test-smoke" => {
    let _ = ui_weak.upgrade_in_event_loop(|ui| {
        ui.global::<Bridge>().invoke_run_migration_test("smoke".into());
    });
}
```

### 2.3 Why `upgrade_in_event_loop` for the `invoke_*` calls

`on_invoke_action`'s closure already runs on the UI thread (Slint
delivers callbacks to the registered handler from the main
thread). In principle, `ui.global::<Bridge>().invoke_*(...)`
could be called directly without `upgrade_in_event_loop`.

However, the closure captures `ui_weak: Weak<MainWindow>`, not
`ui: Application`. To get back an `Application` handle, you have
to `upgrade_in_event_loop` — the function is the standard pattern
for "I have a `Weak`, I want a `&Application` on the UI thread".

So the wrapper is required by the lifetime model, not by the
threading model. It's identical to the existing pattern at
`lib.rs:2097-2098`.

### 2.4 What stays the same

- The match arm keys (`"migrated-server"`, `"test-getinfo"`,
  `"test-crossfade"`, `"test-smoke"`) — unchanged. Their lifecycle
  in `default_quick_actions()` is also unchanged.
- The `scan-qr` arm — unrelated to migration runtime; leave intact.
- The `_ => {}` fall-through — leave intact.
- `on_invoke_action`'s closure capture set (`app_clone`,
  `ui_weak`) — `app_clone` is no longer used by the four
  migration arms (`scan-qr` still uses it), so keep it.

---

## 3. Verification

### 3.1 Grep

```bash
# 1. Migration runtime functions are no longer called from within
#    on_invoke_action's match body.
grep -nE 'start_migrated_command_server|run_legacy_http_(getinfo|crossfade)_test|run_graph_smoke_test' \
    senders/android/src/lib.rs

# Expected matches (after STEP-3):
#  - lib.rs:188 (function definition)
#  - lib.rs:243 (function definition)
#  - lib.rs:264 (function definition)
#  - lib.rs:427 (function definition)
#  - lib.rs:~2NNN (inside the STEP-2 `on_run_migration_test` /
#                  `on_start_migration_server` handlers — the only callers)
#
# Specifically: 0 matches inside the `on_invoke_action` body
# (lines ~2082-2129 post-STEP-3).
```

### 3.2 Compile

```bash
cargo +nightly check -p android-sender --target aarch64-linux-android
```

Most likely failures:

- `error[E0599]: no method named invoke_start_migration_server` —
  Step 1 didn't land. Re-check `bridge.slint` for the three
  callback declarations.
- `error: this function takes 0 arguments but 1 argument was
  supplied` — typo (`invoke_stop_migration_server` takes no
  arguments; `invoke_run_migration_test` takes one).

### 3.3 Runtime smoke (manual)

1. `cfg!(debug_assertions)` build.
2. Open the quick-action drawer; tap each of the four ids in
   turn.
3. Expect identical behaviour to `master`: `Bridge.test-status`
   shows `Running …` then `PASS …`/`FAIL …`; logcat
   `log_ui_test_status` lines fire with the same `name` /
   `status` content as before.

### 3.4 Optional: confirm dispatch via the Bridge

Add a temporary `info!("PHASE-9 dispatch via Bridge callback id={}", test_id)`
log line inside the Step-2 `on_run_migration_test` handler.
Tap each of the three test buttons; verify the log fires. Remove
the temporary log before merge.

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to remove the worker-thread spawn from the
match arms

The "Before" snippet at §2.1 calls `std::thread::spawn(...)` for
the three test arms. The "After" snippet **must not** spawn a
thread — the spawn now happens inside the Step-2 callback handler
(see `MVP-PHASE-9-STEP-2-rust-handlers.md` §2.2). Spawning in
**both** places would run each test twice, racing on the shared
HTTP command server.

### P2 — Forgetting `LEGACY_COMMAND_BIND_ADDR.into()`

`invoke_start_migration_server` takes `SharedString`, not `&str`.
Passing `LEGACY_COMMAND_BIND_ADDR` directly will fail to compile
with `expected SharedString, found &str`. Add `.into()`.

### P3 — Changing the `id_str` for the Slint side

The four ids (`migrated-server`, `test-getinfo`, `test-crossfade`,
`test-smoke`) come from `default_quick_actions()` and are
referenced by `bar-actions` persistence (see PHASE-8 Cluster F
docs). Don't rename them as part of this step — UI persistence
keys off them.

### P4 — Dropping the `_ => {}` arm

The match still needs the catch-all because other `id_str` values
flow through `on_invoke_action` (`settings`, `debug`, `pair`,
`bitrate`, etc., all from `default_quick_actions()`'s
non-debug list). Don't replace `match` with `if id_str == "…"`
chains — the catch-all becomes implicit and you lose the
exhaustiveness signal for future readers.

### P5 — Calling `invoke_run_migration_test` from outside the UI
thread

This step's snippet calls it from inside the existing
`upgrade_in_event_loop` block, which is UI-thread. If you ever
refactor to call the Bridge callback from a different thread, you
must wrap with `upgrade_in_event_loop` first (same rule as
`set_test_status`, see STEP-2 §P3).

### P6 — Forgetting to test the catch-all id

The Step-2 `on_run_migration_test` handler has a fallback:
`other => format!("FAIL unknown migration-test id: {other}")`.
Quick-sanity that it never fires in production by temporarily
invoking `Bridge.run-migration-test("bogus")` from a debug page;
expect `Bridge.test-status == "FAIL unknown migration-test id:
bogus"`. STEP-6 adds an automated unit test for this.

---

## 5. Next step

Once this lands, [Step 4](./MVP-PHASE-9-STEP-4-lazy-runtime-start.md)
removes the unconditional `start_graph_runtime()` from the top of
`run_event_loop()` and relies on idempotent on-demand startup
from the (now Bridge-routed) callbacks.

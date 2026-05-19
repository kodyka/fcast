# MVP-PHASE-9 — Step 2: register `on_start_migration_server` / `on_run_migration_test` / `on_stop_migration_server` handlers

> Part 2 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
> Previous: [Step 1 — Bridge callback declarations](./MVP-PHASE-9-STEP-1-bridge-callbacks.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Wire the three new `Bridge` callbacks (declared in Step 1) to
their Rust handlers. Each handler delegates to the existing
debug-test free functions in `senders/android/src/lib.rs`:

| Slint callback | Rust handler delegates to |
|---|---|
| `Bridge.start-migration-server(bind_addr)` | `start_migrated_command_server(&bind_addr)` (`lib.rs:188`) |
| `Bridge.run-migration-test("getinfo")` | `run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR)` (`lib.rs:243`) |
| `Bridge.run-migration-test("crossfade")` | `run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR)` (`lib.rs:264`) |
| `Bridge.run-migration-test("smoke")` | `run_graph_smoke_test()` (`lib.rs:427`) |
| `Bridge.stop-migration-server()` | `migration::runtime::shutdown_graph_runtime()` (`runtime.rs:312`) |

Each handler writes the result string to `Bridge.test-status` via
`upgrade_in_event_loop`, mirroring the existing pattern at
`lib.rs:2095-2126`.

After this step paired with Step 1, the callbacks have working
handlers but **nobody invokes them yet** — the four debug
quick-actions still call the migration runtime directly. Step 3
flips those callers over.

This is the **largest step in PHASE-9** (~60 Rust lines net). It
exercises the existing Slint Rust glue, the existing free
functions, and the existing `set_test_status` write path. No new
external dependencies.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `Application::run_event_loop` (where callbacks are registered today) | `senders/android/src/lib.rs:1099-1135` (function entry to top of event loop) |
| `on_invoke_action` registration (style reference + match arms to mirror) | `senders/android/src/lib.rs:2082-2129` |
| `on_connect_receiver` (style reference for a simple Slint → event_tx callback) | `senders/android/src/lib.rs:1875-1881` |
| `start_migrated_command_server(bind_addr)` | `senders/android/src/lib.rs:188-198` |
| `run_legacy_http_getinfo_test(bind_addr)` | `senders/android/src/lib.rs:243-261` |
| `run_legacy_http_crossfade_test(bind_addr)` | `senders/android/src/lib.rs:264-425` |
| `run_graph_smoke_test()` | `senders/android/src/lib.rs:427-487` |
| `log_ui_test_status(name, status)` | `senders/android/src/lib.rs:492-499` |
| `Bridge::set_test_status` (generated) | inside `ui.global::<Bridge>()` |
| `LEGACY_COMMAND_BIND_ADDR` | `senders/android/src/lib.rs:99` |

### 1.2 Where to put the new registrations

Three options:

- **(a)** Inline next to `on_invoke_action` (`lib.rs:2082`). Same
  file, same function scope, same `ui_weak` / `app_clone` clones
  available. Simplest.
- **(b)** A new helper `fn register_migration_callbacks(ui:
  &Application, ui_weak: Weak<MainWindow>)` called from the same
  setup function. Keeps the registrations together as a unit,
  easier to gate with `#[cfg(debug_assertions)]` in Step 5.
- **(c)** A new submodule `mod debug_quickactions` with all four
  test helpers + the three callback registrations. Most isolated;
  invasive to set up.

**Recommendation: (a) for STEP-2** — match the existing pattern.
STEP-5 (optional) can lift the block into a helper or submodule.

### 1.3 Threading model

The three test functions take 100ms-10s to run (network I/O via
`send_http_request`). The existing pattern in `on_invoke_action`
spawns a worker thread per test:

```rust
std::thread::spawn(move || {
    let status = run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR);
    log_ui_test_status("legacy-getinfo", &status);
    let _ = ui_weak_clone.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().set_test_status(status.into())
    });
});
```

Mirror this pattern in `on_run_migration_test`. Don't switch to
`tokio::spawn` — the test functions are blocking
(`send_http_request` uses `std::net::TcpStream`), so they'd block
a runtime worker.

### 1.4 `start-migration-server` vs `run-migration-test`

`start_migrated_command_server` is **synchronous** (binds the
HTTP server, polls `/health`, returns). It can run on the
event-loop thread without a worker spawn. The test functions
**transitively** call it, but the cost is amortised.

For consistency with the test handlers, you can spawn a worker for
`start-migration-server` too. Slightly heavier, but uniform.
Pick one and stick with it; STEP-2 recommends inline for the
start/stop callbacks and worker-spawn for the test callbacks.

---

## 2. The change

### 2.1 Find the registration site

The existing `on_invoke_action` registration is at:

```rust
// senders/android/src/lib.rs:2082-2129
ui.global::<Bridge>().on_invoke_action({
    let app_clone = app_clone.clone();
    let ui_weak = ui.as_weak();
    move |id| {
        let id_str = id.as_str();
        match id_str {
            "scan-qr" => { /* … */ }
            "migrated-server" => { /* delegates to start_migrated_command_server */ }
            "test-getinfo" => { /* delegates to run_legacy_http_getinfo_test */ }
            "test-crossfade" => { /* delegates to run_legacy_http_crossfade_test */ }
            "test-smoke" => { /* delegates to run_graph_smoke_test */ }
            _ => {}
        }
    }
});
```

Add the three new registrations **immediately after** this block.

### 2.2 The three handlers

**File:** `senders/android/src/lib.rs` (immediately after
`on_invoke_action` registration, around line 2130):

```rust
// ── MVP-PHASE-9: migration-runtime callbacks ────────────────────────
//
// Bridge.start-migration-server(bind-addr) → start_migrated_command_server
ui.global::<Bridge>().on_start_migration_server({
    let ui_weak = ui.as_weak();
    move |bind_addr: SharedString| {
        let status = match start_migrated_command_server(bind_addr.as_str()) {
            Ok(message) => format!("PASS {message}"),
            Err(err) => format!("FAIL {err}"),
        };
        log_ui_test_status("start-migration-server", &status);
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_test_status(status.into());
        });
    }
});

// Bridge.run-migration-test(test-id) → run_legacy_http_*_test / run_graph_smoke_test
ui.global::<Bridge>().on_run_migration_test({
    let ui_weak = ui.as_weak();
    move |test_id: SharedString| {
        let test_id = test_id.to_string();
        let _ = ui_weak.upgrade_in_event_loop({
            let test_id = test_id.clone();
            move |ui| {
                ui.global::<Bridge>().set_test_status(
                    format!("Running migration test '{test_id}'…").into(),
                );
            }
        });
        let ui_weak_inner = ui_weak.clone();
        std::thread::spawn(move || {
            let status = match test_id.as_str() {
                "getinfo"   => run_legacy_http_getinfo_test(LEGACY_COMMAND_BIND_ADDR),
                "crossfade" => run_legacy_http_crossfade_test(LEGACY_COMMAND_BIND_ADDR),
                "smoke"     => run_graph_smoke_test(),
                other       => format!("FAIL unknown migration-test id: {other}"),
            };
            log_ui_test_status(
                match test_id.as_str() {
                    "getinfo"   => "legacy-getinfo",
                    "crossfade" => "legacy-crossfade",
                    "smoke"     => "graph-smoke",
                    _           => "unknown",
                },
                &status,
            );
            let _ = ui_weak_inner.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_test_status(status.into());
            });
        });
    }
});

// Bridge.stop-migration-server() → migration::runtime::shutdown_graph_runtime
ui.global::<Bridge>().on_stop_migration_server({
    let ui_weak = ui.as_weak();
    move || {
        let status = match crate::migration::runtime::shutdown_graph_runtime() {
            Ok(()) => "PASS migration server stopped".to_string(),
            Err(err) => format!("FAIL migration server stop: {err}"),
        };
        log_ui_test_status("stop-migration-server", &status);
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_test_status(status.into());
        });
    }
});
```

### 2.3 Required imports

The handlers reference `SharedString`. If the file doesn't already
have `use slint::SharedString;` near the existing imports, add it
or use the fully qualified name (`slint::SharedString`). Most of
the existing handlers use `SharedString` unqualified — confirm
with:

```bash
grep -n 'use slint::SharedString\|use slint::\*\|slint::SharedString' \
    senders/android/src/lib.rs | head -5
```

`crate::migration::runtime::shutdown_graph_runtime` is already
called at `lib.rs:1128`, so the path is in scope without a new
`use`.

### 2.4 Why log_ui_test_status names are reused

The existing handlers use the log names
`start-migrated-server` / `legacy-getinfo` / `legacy-crossfade` /
`graph-smoke`. Reusing them preserves logcat search continuity
for anyone with existing greps. The only new name is
`stop-migration-server` (new functionality).

### 2.5 Why the "Running migration test '<id>'…" prelude

Mirrors the existing per-test prelude
(`"Running legacy getinfo test..."` etc., `lib.rs:2102/2111/2120`).
A single generic message replaces three test-specific messages —
less code, equivalent UX.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p android-sender --target aarch64-linux-android
```

Expect **clean**. Most likely failures:

- `error[E0599]: no method named on_start_migration_server` — Step 1
  didn't land or `slint-build` didn't re-run. Force-rebuild with
  `cargo clean -p android-sender && cargo build -p android-sender`.
- `error[E0277]: SharedString cannot be …` — missing `use
  slint::SharedString` import. See §2.3.
- `error[E0599]: no function named log_ui_test_status` — typo or
  scoping issue. The function is module-private at `lib.rs:492`;
  the call site is in the same file.

### 3.2 Unit-test the dispatch table (preview of STEP-6)

```rust
#[test]
fn migration_test_id_dispatch_table() {
    assert_eq!(map_test_id_to_log_name("getinfo"),   "legacy-getinfo");
    assert_eq!(map_test_id_to_log_name("crossfade"), "legacy-crossfade");
    assert_eq!(map_test_id_to_log_name("smoke"),     "graph-smoke");
    assert_eq!(map_test_id_to_log_name("bogus"),     "unknown");
}
```

(If you extract the test-id → log-name mapping into a small
helper, this unit test fits cleanly; STEP-6 spells it out.)

### 3.3 Runtime smoke

Invoke each callback manually (after STEP-3 lands) and verify:

- `adb logcat | grep log_ui_test_status` shows the same log line
  shapes as on `master`.
- `Bridge.test-status` (visible on the debug page) shows the
  same `PASS …` / `FAIL …` content as before.

Until STEP-3 lands, nothing invokes the new callbacks at runtime
— `cargo check` passing is the only verification possible.

### 3.4 Grep

```bash
grep -nE 'on_start_migration_server|on_run_migration_test|on_stop_migration_server' \
    senders/android/src/lib.rs
# → exactly 3 matches.
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting `ui_weak.clone()` inside the spawned thread

`run-migration-test` needs the `ui_weak` captured both **before**
the spawn (for the "Running test…" prelude) **and** inside the
spawn (for the final status write). The example in §2.2 calls
`.clone()` on the outer `ui_weak`; missing this clone causes a
move-of-borrowed-value error.

### P2 — `bind_addr.as_str()` lifetime

`SharedString::as_str()` returns `&str` with the lifetime of the
`SharedString`. Inside `move |bind_addr: SharedString| { … }`, the
`bind_addr` is owned by the closure, so `bind_addr.as_str()` is
fine to use synchronously. Don't try to stash it across a
`std::thread::spawn` — clone to `String` first (see the
`run-migration-test` handler for the `test_id.to_string()`
pattern).

### P3 — `set_test_status` from the worker thread

The example uses `upgrade_in_event_loop` correctly — Slint's
generated glue requires UI mutations on the main thread. Calling
`set_test_status` directly from the spawn'd thread will panic
with `slint_interpreter::ApiError::NotMainThread` (or similar).
Always go through `upgrade_in_event_loop`.

### P4 — Calling `shutdown_graph_runtime` while a test is running

`stop-migration-server` can fire while a `run-migration-test`
worker thread is mid-HTTP-call. The shutdown stops the command
server's listener thread; the in-flight HTTP request will fail
with a connection error and the test handler will return
`FAIL connection refused`. This is the **right** behaviour
(matches what would happen if the server crashed mid-test), but
worth noting — the user might see a `FAIL` immediately after
hitting "stop".

If you want to be defensive, add a "test currently running"
mutex (out of scope for this phase; flag for a follow-up if
needed).

### P5 — Bypassing the Bridge callback in `on_invoke_action`

This step does **not** modify `on_invoke_action` yet — that's
STEP-3. After STEP-2 lands and before STEP-3 lands, the four
debug quick-actions still call the migration runtime directly via
the existing `match id_str` ladder. That's expected and correct
for the intermediate state.

### P6 — `SharedString::as_str()` vs `to_string()` for `bind_addr`

`start_migrated_command_server` takes `&str`. `as_str()` is
zero-copy. Don't convert to `String` (no need to own the value;
the function returns before the closure scope exits). The
`run-migration-test` handler uses `to_string()` for `test_id`
specifically because it crosses the `std::thread::spawn`
boundary, which `&str` cannot.

---

## 5. Next step

Once this lands, [Step 3](./MVP-PHASE-9-STEP-3-quick-actions-rewrite.md)
rewrites the four `migrated-server` / `test-getinfo` /
`test-crossfade` / `test-smoke` branches inside `on_invoke_action`
to invoke the new Bridge callbacks via
`ui.global::<Bridge>().invoke_start_migration_server(...)` /
`invoke_run_migration_test(...)` instead of calling the migration
runtime functions directly.

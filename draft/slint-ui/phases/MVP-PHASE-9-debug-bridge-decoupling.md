# MVP-PHASE-9 — UI ↔ migration-runtime decoupling (Tier 2.2 polish)

> **Small post-MVP polish.** The Android sender UI (Slint) and the
> migration runtime (Surface B) currently share an internal coupling
> through four debug quick-actions whose Rust handlers call
> `migration::runtime::*` free functions directly. This phase routes
> those calls through `Bridge` callbacks so the UI layer is agnostic
> to the migration-runtime implementation and so the runtime can be
> treated as a pluggable backend (or replaced under test).
>
> A second concern is that `start_graph_runtime()` runs
> unconditionally at the top of `run_event_loop()` even when nothing
> downstream needs it. This phase converts that startup to be
> on-demand (driven by the first Bridge callback that needs the
> runtime) while keeping the existing shutdown semantics.
>
> **Doc-only.** Every file in this set tells you what to change in
> the existing tree, with `file:line` citations and concrete code
> snippets. No source-tree code is touched by this set of docs.

---

## 0. Goal

After this phase:

- `Bridge.start-migration-server(string)` /
  `Bridge.run-migration-test(string)` /
  `Bridge.stop-migration-server()` callbacks exist as the **only**
  entry point through which the UI talks to the migration runtime.
- The four debug quick-actions (`migrated-server`, `test-getinfo`,
  `test-crossfade`, `test-smoke`) dispatch via those callbacks
  instead of calling `migration::runtime::*` directly inside
  `on_invoke_action`'s `match id_str` ladder.
- The unconditional `start_graph_runtime()` at
  `senders/android/src/lib.rs:1110` becomes **on-demand** — the
  runtime starts the first time a Bridge callback that needs it
  fires (or, equivalently, when the first cast event hits a graph
  command path). The matching `shutdown_graph_runtime()` at
  `lib.rs:1128` still runs on event-loop exit.
- The debug quick-action plumbing remains `#[cfg(debug_assertions)]`
  gated — both at the `default_quick_actions()` extension site
  (`lib.rs:1156-1162`) and at the callback registration site (new
  in this phase). Optionally the debug-only Rust glue is moved into
  a dedicated `debug_quickactions` submodule.
- The user-visible test-status display (`Bridge.test-status` at
  `bridge.slint:202`) is unchanged — same property, same setter
  pattern via `upgrade_in_event_loop` from inside the new callback
  handlers.

This phase is **purely structural**. No new behaviour ships; no
existing behaviour changes (the four debug quick-actions still
trigger the same migration-runtime functions, with the same
results, displayed the same way).

---

## 1. Pre-flight

### 1.1 What already exists (do not re-create)

| Component | Location |
|---|---|
| `Bridge.test-status: string` (the test-status display property) | `senders/android/ui/bridge.slint:202` |
| `Bridge.invoke-action(string)` callback | `senders/android/ui/bridge.slint:239` |
| `Bridge.quick-actions: [QuickAction]` list | `senders/android/ui/bridge.slint:198` |
| `default_quick_actions()` (debug extras list) | `senders/android/src/lib.rs:1147-1164` |
| `on_invoke_action` registration with the four debug branches | `senders/android/src/lib.rs:2082-2129` |
| `start_migrated_command_server(bind_addr)` | `senders/android/src/lib.rs:188-198` |
| `run_legacy_http_getinfo_test(bind_addr)` | `senders/android/src/lib.rs:243-261` |
| `run_legacy_http_crossfade_test(bind_addr)` | `senders/android/src/lib.rs:264-425` |
| `run_graph_smoke_test()` | `senders/android/src/lib.rs:427-487` |
| `migration::runtime::start_graph_runtime()` | `senders/android/src/migration/runtime.rs:302-310` |
| `migration::runtime::shutdown_graph_runtime()` | `senders/android/src/migration/runtime.rs:312-320` |
| Unconditional auto-start inside `run_event_loop()` | `senders/android/src/lib.rs:1110` |
| Matching `shutdown_graph_runtime()` on loop exit | `senders/android/src/lib.rs:1128` |
| `LEGACY_COMMAND_BIND_ADDR: &str = "0.0.0.0:8080"` | `senders/android/src/lib.rs:99` |
| `log_ui_test_status(name, status)` (existing logger) | `senders/android/src/lib.rs:492` |

### 1.2 What needs to change

| File | Edit |
|---|---|
| `senders/android/ui/bridge.slint` | Add 3 new callback declarations: `start-migration-server(string)`, `run-migration-test(string)`, `stop-migration-server()`. |
| `senders/android/src/lib.rs` | (a) Register `on_start_migration_server` / `on_run_migration_test` / `on_stop_migration_server` handlers alongside `on_invoke_action`. (b) Rewrite the four `migrated-server` / `test-getinfo` / `test-crossfade` / `test-smoke` branches in `on_invoke_action` to invoke the new callbacks. (c) Remove the unconditional `start_graph_runtime()` from `run_event_loop()` and replace it with on-demand startup driven by the new callbacks (and by existing graph-command call sites). |
| `senders/android/src/lib.rs` (optional) | Extract the four debug callbacks into a `mod debug_quickactions` submodule and call `debug_quickactions::register(&ui, ...)` once per `cfg!(debug_assertions)` build. |

Approximate scope: **~80-120 lines across 2-3 files**.

### 1.3 Why route through Bridge callbacks at all?

Three reasons:

1. **Testability.** `bridge.slint`'s callback surface is the contract
   between Slint and Rust. Everything that crosses that boundary is
   easy to reason about and easy to mock from a Slint test harness
   (or replace under a feature flag). Direct `crate::migration::*`
   calls from inside an `on_invoke_action` `match` arm sit on the
   Rust side of the line and need extra plumbing to swap out.

2. **Pluggability.** A future on-device debug panel (or even a
   non-debug build's quick-actions that need migration-runtime
   features) can invoke the same `start-migration-server(...)` /
   `run-migration-test(...)` callbacks without each page needing to
   re-wire to the Rust free functions. Today, every new caller has
   to grow a new `on_invoke_action` branch — that doesn't scale.

3. **Lazy-start.** Once the runtime entry points are explicit
   Bridge callbacks, it becomes trivially correct to start the
   runtime **inside** the callback registration (or first-use of
   any callback that needs it). The current unconditional start at
   `lib.rs:1110` is a workaround for "we don't know who calls it"
   — Bridge-callback dispatch makes the answer "every caller is
   one of these three callbacks", which closes the question.

### 1.4 Why three callbacks (not one)

The user's research suggests three: `start-migration-server`,
`run-migration-test`, `stop-migration-server`. The split mirrors
the lifecycle:

- `start-migration-server(bind_addr: string)` — bring up the HTTP
  command server, idempotent (`start_graph_runtime()` is already
  idempotent). Takes the bind address explicitly so the callback
  signature documents the contract rather than reading it from
  `LEGACY_COMMAND_BIND_ADDR` implicitly.
- `run-migration-test(test_id: string)` — dispatch one of the four
  test runs (`"getinfo"`, `"crossfade"`, `"smoke"`, or future
  additions). The string namespace mirrors the existing
  `Bridge.invoke-action(...)` quick-action id space.
- `stop-migration-server()` — explicit teardown. Used by debug
  panels that want to force a clean restart, or by future logic
  that wants to free the GStreamer state on background.

A single `migration-command(action, arg)` callback would also
work, but two-string callbacks are awkward to read and the
lifecycle distinction (start / run / stop) is meaningful.

### 1.5 Why on-demand runtime start

`migration::runtime::start_graph_runtime()` currently runs
unconditionally at the top of `run_event_loop()` (`lib.rs:1110`).
That's fine for debug builds where the runtime is always
exercised, but in release builds (where the debug quick-actions
don't appear) the runtime spins up a refresh thread and an HTTP
command server that nobody talks to.

The four call sites that **do** need the runtime today are:

- The four debug callbacks introduced by this phase
  (`start-migration-server` calls it explicitly; the test
  callbacks call it transitively through
  `start_migrated_command_server`).
- The graph-command cast loop in `Event::CaptureStarted` (post
  MVP-PHASE-6) — `lib.rs:985-1000` dispatches commands via
  `crate::migration::runtime::handle_command(...)`.
- The `nativeProcessGraphCommandJson` JNI hook
  (`lib.rs:2181`).

Each of those call sites can ensure the runtime is started before
use. The startup is **already idempotent** (re-entrant; second
call is a no-op past the manager state guard) — see
`runtime.rs:302-310`. So the change is mechanical: delete the
unconditional call at `lib.rs:1110` and rely on each call site to
ensure-then-use. STEP-4 spells out exactly how.

---

## 2. Steps — split into six per-step files

To keep each step skimmable and reviewable in isolation, the
implementation is split across six per-step
`MVP-PHASE-9-STEP-N-*.md` files. Each file follows the smaller
five-section template (Goal-of-this-step / Pre-flight / The change
/ Verification / Next step) and is self-contained.

| # | File | Scope | Net diff |
|---|---|---|---|
| 1 | [`MVP-PHASE-9-STEP-1-bridge-callbacks.md`](./MVP-PHASE-9-STEP-1-bridge-callbacks.md) | Add 3 new callback declarations to `bridge.slint`. Type-only Slint change. | ~3 lines, 1 file |
| 2 | [`MVP-PHASE-9-STEP-2-rust-handlers.md`](./MVP-PHASE-9-STEP-2-rust-handlers.md) | Register `on_start_migration_server` / `on_run_migration_test` / `on_stop_migration_server` in `run_event_loop()`. Each handler delegates to the existing free functions (`start_migrated_command_server`, `run_legacy_http_*_test`, `run_graph_smoke_test`, `shutdown_graph_runtime`). **Largest step.** | ~60 Rust lines, 1 file |
| 3 | [`MVP-PHASE-9-STEP-3-quick-actions-rewrite.md`](./MVP-PHASE-9-STEP-3-quick-actions-rewrite.md) | Rewrite the four `on_invoke_action` debug branches to forward to the new Bridge callbacks via `ui.global::<Bridge>().invoke_*`. | ~25 lines, 1 file |
| 4 | [`MVP-PHASE-9-STEP-4-lazy-runtime-start.md`](./MVP-PHASE-9-STEP-4-lazy-runtime-start.md) | Remove the unconditional `start_graph_runtime()` at `lib.rs:1110`; rely on idempotent on-demand startup from the four call sites that actually need it. Keep the shutdown at `lib.rs:1128`. | ~5 lines deleted + ~3 lines added |
| 5 | [`MVP-PHASE-9-STEP-5-debug-cfg-separation.md`](./MVP-PHASE-9-STEP-5-debug-cfg-separation.md) | **Optional.** Gate STEP-2's callback registrations with `#[cfg(debug_assertions)]` (matching the `default_quick_actions()` debug extras gate), and optionally extract them to a `mod debug_quickactions` submodule. | ~10 lines or new file |
| 6 | [`MVP-PHASE-9-STEP-6-unit-tests.md`](./MVP-PHASE-9-STEP-6-unit-tests.md) | Host-runnable unit tests: callback dispatch table, lazy-init guard, `Bridge.test-status` write path. | ~80 lines of tests |

---

## 3. Verification (phase-level)

### 3.1 Grep recipes

```bash
# 1. The three new Bridge callbacks exist
grep -n 'callback start-migration-server\|callback run-migration-test\|callback stop-migration-server' \
    senders/android/ui/bridge.slint
# → exactly 3 matches

# 2. The four `on_invoke_action` debug branches no longer call
#    migration runtime functions directly
grep -nE 'on_invoke_action|migrated-server|test-getinfo|test-crossfade|test-smoke' \
    senders/android/src/lib.rs | grep -v 'invoke-action\|callback\|test-status'
# → the four branches should ONLY invoke `ui.global::<Bridge>().invoke_*`,
#   not call `start_migrated_command_server` / `run_legacy_http_*` directly.

# 3. The unconditional auto-start at lib.rs:1110 is gone
grep -nE 'start_graph_runtime\(\)' senders/android/src/lib.rs
# → 0 unconditional call sites in `run_event_loop()`.
#   (`start_migrated_command_server` may still call it inside its body.)
```

### 3.2 On-device smoke

1. Build a `debug_assertions` APK.
2. Launch on a device or emulator.
3. Open the quick-action drawer; verify the four extras
   (`Migrated srv`, `GetInfo`, `Crossfade`, `Smoke Graph`) are
   present.
4. Tap each one in turn and verify `Bridge.test-status` updates
   with the same `PASS …` / `FAIL …` content as on `master`.
5. Watch `adb logcat | grep -E 'log_ui_test_status|GraphCommand'`
   — the log line shapes should be unchanged.

### 3.3 Negative smoke (lazy-start verification)

1. Build a **release** APK (`cfg!(debug_assertions)` is `false`).
2. Launch on a device.
3. Verify that the migration-runtime command server does **not**
   bind port 8080 (`adb shell netstat -an | grep 8080`).
4. Trigger a cast via the connect-receiver flow. Verify the
   runtime starts on the first graph command issued by
   `Event::CaptureStarted` (visible in logcat: `GRAPH_NODE_MANAGER
   start` / `command server listening on …`).
5. Stop the cast and exit the app. Verify the shutdown still runs
   (logcat: `GRAPH_NODE_MANAGER shutdown`).

---

## 4. Common pitfalls

### P1 — `Bridge.invoke_start_migration_server` not the right call shape

Slint's generated Rust glue exposes callbacks with **`invoke_`**
prefix when called **from Rust** (and **`on_`** prefix when
registering a handler). The Slint side calls them as
`Bridge.start-migration-server(...)`. Don't mix them up — the
compiler error if you do is clear, but the read-direction
confusion is the most common pitfall when adding new callbacks.

### P2 — Removing `start_graph_runtime()` before all call sites are
covered

The unconditional auto-start at `lib.rs:1110` masks any call site
that **assumes** the runtime is up. Before removing it (STEP-4),
audit the four call sites listed in §1.5 and confirm each calls
`start_graph_runtime()` itself (idempotent, cheap) or has an
upstream caller that does. STEP-4's pre-flight section walks
through this.

### P3 — `test-status` race when two debug callbacks fire in quick
succession

The current handlers spawn a `std::thread` for each test, which
can interleave with another test's `set_test_status("Running …")`
prelude. The new callback handlers preserve the same threading
model — so the race is identical, neither better nor worse. Don't
"fix" it as part of this phase; it's pre-existing behaviour. A
future Phase can add a debounce or "currently running" flag if
needed.

### P4 — Forgetting that the `Bridge.test-status` writers run on
the UI thread

All `set_test_status(...)` calls go through
`upgrade_in_event_loop(|ui| ...)`. Don't call `set_test_status`
directly from the spawned worker thread — the Slint generated
glue panics if called off-thread. The existing pattern is correct;
copy it exactly into the new callback handlers.

### P5 — `debug_assertions` cfg leak

If you forget to gate the **callback registrations** (STEP-2) with
`#[cfg(debug_assertions)]`, release builds will register handlers
that are never invoked (the debug quick-actions never reach
release builds because of the existing
`default_quick_actions()` gate at `lib.rs:1156-1162`). Harmless,
but technically dead code. STEP-5 cleans this up.

### P6 — The `LEGACY_COMMAND_BIND_ADDR` constant is now a parameter

After STEP-2, the bind address is a callback argument
(`string`). The constant at `lib.rs:99` still has one remaining
caller (STEP-3 passes it to
`ui.global::<Bridge>().invoke_start_migration_server(LEGACY_COMMAND_BIND_ADDR.into())`).
Don't delete the constant. If you want to delete it later
(post-PHASE-9), make sure every `invoke_start_migration_server`
caller has a bind address argument first.

---

## 5. Stop conditions

This phase is **done** when:

1. The three new Bridge callbacks exist with the documented
   signatures (verified by `grep` in §3.1).
2. The four `on_invoke_action` debug branches forward via Bridge
   callbacks (verified by §3.1 and §3.2).
3. Release builds do not bind port 8080 on launch (verified by
   §3.3).
4. Debug builds behave **identically** to `master` for all four
   debug quick-actions (verified by §3.2).
5. Unit tests in STEP-6 pass under `cargo test -p android-sender
   phase9_` on host.
6. The `nativeProcessGraphCommandJson` JNI hook still works
   end-to-end (it ensures-starts the runtime itself, so the
   STEP-4 removal does not affect it).

---

## 6. Out of scope

- **Renaming `Bridge.test-status`** to e.g. `migration-test-status`
  is tempting (it's now more specific) but breaks any page that
  reads the property today. Defer.
- **Adding new debug quick-actions** (e.g. a "reset migration
  runtime" action) — out of scope; this phase is structural only.
  Once the Bridge callbacks exist, adding more actions is a
  one-line `Bridge.run-migration-test("…")` invocation.
- **Renaming `LEGACY_COMMAND_BIND_ADDR`** — out of scope. The name
  is a relic of the pre-graph-command HTTP server; the constant
  itself is still the right value.
- **Replacing `std::thread::spawn` with `tokio::spawn`** inside
  the new callback handlers — out of scope. The existing pattern
  uses `std::thread` because `run_legacy_http_*_test` is blocking
  via `send_http_request`. A future phase can move it.

---

## 7. Glossary

| Term | Defined in |
|---|---|
| **Migration runtime / Surface B** | `senders/android/src/migration/runtime.rs`. The node-graph backend introduced by MVP-PHASE-3 / 4 / 5 / 6. |
| **`Bridge.test-status`** | `bridge.slint:202`. UI display string for the latest debug-test result. Written by Rust callback handlers; read by the debug page. |
| **`LEGACY_COMMAND_BIND_ADDR`** | `lib.rs:99`. The fixed bind address (`"0.0.0.0:8080"`) used by all debug quick-actions today. |
| **`start_graph_runtime()`** | `runtime.rs:302`. Idempotent runtime startup — starts the `GRAPH_NODE_MANAGER`, the refresh thread, and the HTTP command server. |
| **`debug_assertions`** | Standard Rust cfg flag. `true` in debug builds, `false` in release builds. |

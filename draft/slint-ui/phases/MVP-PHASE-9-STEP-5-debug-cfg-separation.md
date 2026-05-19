# MVP-PHASE-9 — Step 5 (optional): `#[cfg(debug_assertions)]` separation + optional `debug_quickactions` submodule

> Part 5 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
> Previous: [Step 4 — lazy runtime start](./MVP-PHASE-9-STEP-4-lazy-runtime-start.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Gate the Step-2 callback registrations with
`#[cfg(debug_assertions)]` so release builds don't carry handlers
for callbacks that release builds never invoke. This matches the
existing gate at `default_quick_actions()` (`lib.rs:1156-1162`),
which only emits the four debug-quick-action ids under
`cfg!(debug_assertions)`.

**Optionally** extract the three handlers into a dedicated
`mod debug_quickactions` submodule for cleaner separation. The
optional refactor is recommended when the file grows another
debug-only block (e.g. a future "reset migration runtime" callback
or a "dump graph state" handler).

This is the **most optional step** in PHASE-9. Without it the
behaviour is identical; you just carry ~50 lines of dead-in-release
code. Land it when you want the codebase to read cleanly, not when
you need a functional change.

---

## 0.1 When to skip this step

- The Step-2 callback handlers are short (~60 lines total).
  Carrying them in release builds adds ~2 KB to the binary.
- They never fire in release builds (no UI code path invokes
  them).
- If you ship a doc-fix or follow-up PR before STEP-5 is ready,
  the system works correctly without it.

Skip STEP-5 if you have no other debug-only callbacks to add in
the near future. Defer to a later PHASE if a dedicated
`debug_quickactions` module becomes useful.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `default_quick_actions()` debug-extras gate | `senders/android/src/lib.rs:1156-1162` (`if cfg!(debug_assertions) { … }`) |
| Step-2 callback registrations (added by STEP-2; ungated) | `senders/android/src/lib.rs` (immediately after `on_invoke_action` at `lib.rs:2082-2129`) |
| `cfg_attr` style examples in the file | search the file for `#[cfg(debug_assertions)]` |
| The four free functions invoked by the handlers (compile-only in debug, see §1.2) | `senders/android/src/lib.rs:188, 243, 264, 427` |

### 1.2 The four free functions are **already** `#[cfg(target_os = "android")]`

`start_migrated_command_server`, `run_legacy_http_getinfo_test`,
`run_legacy_http_crossfade_test`, `run_graph_smoke_test` are all
`#[cfg(target_os = "android")]`. They compile on Android only —
no need to also gate them with `cfg(debug_assertions)`.

The STEP-2 registrations are inside an `#[cfg(target_os = "android")]`
function (`run_event_loop`'s Android body) — so they already only
compile on Android. The remaining question is whether they should
also be `cfg(debug_assertions)` to skip registering handlers in
release-Android builds.

Recommendation: yes, gate them. The release-Android build doesn't
expose the quick-actions and shouldn't carry the wiring.

### 1.3 Two scope-levels for the gate

**Scope A — per-registration**: gate each
`ui.global::<Bridge>().on_start_migration_server(...)` block
individually with `#[cfg(debug_assertions)]`. Three gates.

**Scope B — block**: gate a containing module / function /
`if cfg!(debug_assertions) { … }` block once. One gate.

Scope B is cleaner and matches `default_quick_actions()`. Use Scope B.

---

## 2. The change

### 2.1 Inline `cfg!(debug_assertions)` gate (Scope B, minimal)

**File:** `senders/android/src/lib.rs` (just after the STEP-2
registrations from `on_invoke_action`):

**Before** (STEP-2 wired the three handlers unconditionally):

```rust
ui.global::<Bridge>().on_invoke_action({ /* … */ });

// ── MVP-PHASE-9: migration-runtime callbacks ──
ui.global::<Bridge>().on_start_migration_server({ /* … */ });
ui.global::<Bridge>().on_run_migration_test({ /* … */ });
ui.global::<Bridge>().on_stop_migration_server({ /* … */ });
```

**After (Scope B, minimal):**

```rust
ui.global::<Bridge>().on_invoke_action({ /* … */ });

// ── MVP-PHASE-9: migration-runtime callbacks (debug builds only) ──
#[cfg(debug_assertions)]
{
    ui.global::<Bridge>().on_start_migration_server({ /* … */ });
    ui.global::<Bridge>().on_run_migration_test({ /* … */ });
    ui.global::<Bridge>().on_stop_migration_server({ /* … */ });
}
```

That's the entire scope-B change. Release-Android builds compile
without the three registrations; debug-Android builds compile
with them.

### 2.2 Optional: extract to a `debug_quickactions` submodule

**File:** `senders/android/src/debug_quickactions.rs` (**new**):

```rust
//! Debug-only quick-action callback registrations.
//!
//! These wire the three Bridge callbacks declared by
//! `MVP-PHASE-9-STEP-1-bridge-callbacks.md` to the existing
//! `start_migrated_command_server` / `run_legacy_http_*_test` /
//! `run_graph_smoke_test` / `shutdown_graph_runtime` free
//! functions in `lib.rs`.
//!
//! Only present in `#[cfg(debug_assertions)]` builds.

#![cfg(all(debug_assertions, target_os = "android"))]

use slint::{ComponentHandle, SharedString, Weak};

use crate::{
    log_ui_test_status, run_graph_smoke_test, run_legacy_http_crossfade_test,
    run_legacy_http_getinfo_test, start_migrated_command_server, Bridge, MainWindow,
    LEGACY_COMMAND_BIND_ADDR,
};

pub(crate) fn register(ui_weak: Weak<MainWindow>, ui: &MainWindow) {
    let weak_for_start = ui_weak.clone();
    ui.global::<Bridge>().on_start_migration_server(move |bind_addr: SharedString| {
        let status = match start_migrated_command_server(bind_addr.as_str()) {
            Ok(message) => format!("PASS {message}"),
            Err(err) => format!("FAIL {err}"),
        };
        log_ui_test_status("start-migration-server", &status);
        let _ = weak_for_start.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_test_status(status.into());
        });
    });

    let weak_for_test = ui_weak.clone();
    ui.global::<Bridge>().on_run_migration_test(move |test_id: SharedString| {
        let test_id = test_id.to_string();
        let weak_for_prelude = weak_for_test.clone();
        let _ = weak_for_prelude.upgrade_in_event_loop({
            let test_id = test_id.clone();
            move |ui| {
                ui.global::<Bridge>().set_test_status(
                    format!("Running migration test '{test_id}'…").into(),
                );
            }
        });
        let weak_for_worker = weak_for_test.clone();
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
            let _ = weak_for_worker.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_test_status(status.into());
            });
        });
    });

    let weak_for_stop = ui_weak.clone();
    ui.global::<Bridge>().on_stop_migration_server(move || {
        let status = match crate::migration::runtime::shutdown_graph_runtime() {
            Ok(()) => "PASS migration server stopped".to_string(),
            Err(err) => format!("FAIL migration server stop: {err}"),
        };
        log_ui_test_status("stop-migration-server", &status);
        let _ = weak_for_stop.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_test_status(status.into());
        });
    });
}
```

**File:** `senders/android/src/lib.rs`:

```rust
// Near the existing `mod migration;` etc. declarations:
#[cfg(all(debug_assertions, target_os = "android"))]
mod debug_quickactions;

// And in `run_event_loop`, in place of the inline registrations:
ui.global::<Bridge>().on_invoke_action({ /* … */ });

#[cfg(all(debug_assertions, target_os = "android"))]
debug_quickactions::register(ui.as_weak(), &ui);
```

### 2.3 Which scope to choose

| Goal | Choose |
|---|---|
| Smallest diff | **§2.1** (inline `cfg!` block, ~3 lines) |
| Cleanest read | **§2.2** (submodule, ~70 lines moved + 3 lines of module decl) |
| Easy to add more debug callbacks later | **§2.2** (drop new fns into the submodule) |
| Easy to revert | **§2.1** (one block to delete) |

The submodule extraction (§2.2) is recommended **only** if you
plan to add more debug callbacks in the near future. Otherwise
§2.1 is the right cost/benefit balance.

### 2.4 Why the imports in `debug_quickactions.rs` are non-trivial

The submodule needs to reach back into the parent `lib.rs` for:

- `Bridge` (Slint-generated; only visible through `slint::include_modules!()`)
- `MainWindow` (Slint-generated)
- `LEGACY_COMMAND_BIND_ADDR` (constant)
- `start_migrated_command_server`, `run_legacy_http_getinfo_test`,
  `run_legacy_http_crossfade_test`, `run_graph_smoke_test`,
  `log_ui_test_status` (functions)
- `crate::migration::runtime::shutdown_graph_runtime` (used by
  the `on_stop_migration_server` handler)

The four free functions and `log_ui_test_status` are currently
not `pub` — they're module-private at `lib.rs`. To `use crate::*`
them from a submodule, mark them `pub(crate)`:

```rust
#[cfg(target_os = "android")]
pub(crate) fn start_migrated_command_server(bind_addr: &str)
    -> std::result::Result<String, String>
{ /* … */ }
```

Apply this to all five functions referenced in §2.2. The
visibility upgrade is harmless — they remain crate-private.

### 2.5 Why not gate the whole `mod migration` with `cfg(debug_assertions)`

Tempting, since the migration runtime is currently exercised only
by debug builds. But PHASE-6 made the cast loop dispatch graph
commands via `crate::migration::runtime::handle_command` —
that's a **production** code path (release builds also cast). So
the migration runtime is not debug-only at the **module** level
anymore. Only the **quick-action wiring** is debug-only.

This is why STEP-5's gate is on the callback **registrations**,
not on the module declaration.

---

## 3. Verification

### 3.1 Release build skips the registrations (Scope B inline)

After §2.1 lands:

```bash
cargo +nightly build --release -p android-sender --target aarch64-linux-android 2>&1 \
    | grep -E 'on_start_migration_server|on_run_migration_test|on_stop_migration_server'
# Expected: 0 references in the compiled crate.
```

(Inspecting the Mach-O/ELF symbol table would be more rigorous —
`objdump -T` — but at the build-log level, the `cfg`'d-out code
generates no symbols.)

### 3.2 Debug build still works

```bash
cargo +nightly build -p android-sender --target aarch64-linux-android
adb install -r path/to/debug.apk
# Launch, tap each of the 4 debug quick-actions, verify Bridge.test-status updates.
```

Identical to STEP-3 §3.3.

### 3.3 Optional grep for the submodule path (after §2.2)

```bash
grep -n 'mod debug_quickactions\|debug_quickactions::register' \
    senders/android/src/lib.rs
# → 2 matches (declaration + call).
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to make the helper functions `pub(crate)`

If you extract to a submodule (§2.2) and the helper functions are
still module-private, the submodule won't compile:

```
error[E0603]: function `start_migrated_command_server` is private
```

Either upgrade them to `pub(crate)` (correct) or paste the
helpers inside `debug_quickactions.rs` (duplicative; rejected).

### P2 — `#[cfg(debug_assertions)]` and `cfg!(debug_assertions)`
are different

- `#[cfg(debug_assertions)] fn foo() { … }` — attribute; the
  whole `fn` disappears in release.
- `if cfg!(debug_assertions) { … }` — expression; both branches
  always compile, the runtime check is constant-folded at
  compile time but the code is still type-checked in both
  configs.

For the Scope-B inline gate (§2.1), use the `#[cfg(debug_assertions)]`
**attribute** on the block, not the `cfg!(...)` macro. The
attribute fully omits the block in release. The macro keeps it.

### P3 — Submodule and JNI exports

`nativeProcessGraphCommandJson` (`lib.rs:2178`) and other JNI
exports must stay in the crate root (or at least in a known
location). Don't move them into `debug_quickactions` — the
runtime is **not** debug-only (see §2.5), and JNI symbol mangling
expects them at the documented path.

### P4 — Linker errors in release

If you only mark some references behind `#[cfg(debug_assertions)]`
and miss others, the release linker will fail with "undefined
reference to start_migrated_command_server". The function itself
is `#[cfg(target_os = "android")]`, not gated on debug — so it
exists in both. The references in `on_invoke_action` (STEP-3) are
already gone post-STEP-3. The only remaining release-build
reference would be the Step-2 registrations, which are exactly
what we're gating here. Confirm via release build (§3.1).

### P5 — Don't move the four debug ids out of
`default_quick_actions()`

`default_quick_actions()` at `lib.rs:1156-1162` is the **other**
debug gate (it controls which quick-action **buttons** appear).
This step gates the **callback handlers**. Both gates need to
agree (debug builds: buttons + handlers; release builds:
neither). Don't accidentally upgrade only one — that creates the
broken state of "buttons that do nothing" or "handlers that
nothing invokes" (the latter is harmless; the former is a UX
bug).

---

## 5. Next step

Once this lands, [Step 6](./MVP-PHASE-9-STEP-6-unit-tests.md)
adds host-runnable unit tests covering the callback dispatch
table, the lazy-start ensure-pattern, and the test-id → log-name
mapping.

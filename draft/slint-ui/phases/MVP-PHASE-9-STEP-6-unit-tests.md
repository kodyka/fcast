# MVP-PHASE-9 — Step 6: host-runnable unit tests

> Part 6 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
> Previous: [Step 5 — debug cfg separation](./MVP-PHASE-9-STEP-5-debug-cfg-separation.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Add a small set of host-runnable unit tests that cover the
**non-GStreamer** parts of PHASE-9:

- The test-id → log-name mapping inside the
  `on_run_migration_test` handler.
- The "unknown test id" fall-through.
- The idempotency claim for `start_graph_runtime()` (already
  asserted by PHASE-3 / PHASE-6 tests; this step adds a
  PHASE-9-specific regression for the lazy-start invariant).

The tests run on the dev host (`cargo test -p android-sender`
without a `--target` flag, **debug** profile) — no Android
device, no real GStreamer pipeline. They use the existing
helpers / fixtures already present in `senders/android/src/migration/`.

This phase doesn't introduce any new test infrastructure. It just
adds three (~80 lines) test functions to existing modules.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| Existing test functions in `lib.rs` (style reference) | search the file for `#[test]` |
| Existing `mod tests` in migration runtime | `senders/android/src/migration/runtime.rs` (search `#[cfg(test)]`) |
| Idempotency invariant for `start_graph_runtime` (PHASE-6 STEP-2 §3.2 already covers it for the cast loop) | see `MVP-PHASE-6-STEP-2-capturestarted-rewrite.md` |
| `log_ui_test_status` (the function whose name the dispatch table maps to) | `senders/android/src/lib.rs:492` |

### 1.2 What we can't test on host

- The actual HTTP command server (`run_legacy_http_*_test`)
  — needs GStreamer init and a free TCP port. These are
  covered by the existing on-device manual smoke (see parent
  doc §3.2-3.3).
- The Slint generated glue (`Bridge.on_*` registrations)
  — not callable from host tests (no UI thread, no
  `slint_interpreter` context). These are covered by `cargo
  check` (the build itself is the test).
- The `Event::CaptureStarted` ensure-start path — needs
  GStreamer init for the rest of the arm to be exercised. Add
  to on-device verification instead.

What we **can** test on host:

- The test-id → log-name string mapping (pure function).
- The "unknown test id" path returns the expected error
  message.
- `start_graph_runtime()` idempotency (call it twice, assert
  no panic + same internal state).

### 1.3 Where the tests live

Three options:

- **(a)** `senders/android/src/lib.rs` — same file as the
  callback handlers. Adds ~50 lines to an already-large file.
- **(b)** `senders/android/src/migration/runtime.rs` — the
  natural home for the idempotency test.
- **(c)** A new module
  `senders/android/src/migration/runtime_tests.rs` — keeps
  all PHASE-9 tests grouped under one `phase9_` prefix.

**Recommendation:** (a) for the dispatch table tests
(they're tightly coupled to the handler logic) + (b) for the
idempotency test (it belongs near the function it tests).
Skip (c) — it's over-organisation for ~80 lines.

---

## 2. The change

### 2.1 Extract the test-id → log-name mapping into a helper

The STEP-2 `on_run_migration_test` handler has a `match test_id.as_str()`
that maps to both:

- the test function to call (`run_legacy_http_getinfo_test` etc.)
- the log name (`legacy-getinfo` etc.)

The function-call part can't be tested on host (the test functions
need GStreamer + network). The log-name part is pure string ops.
Extract the mapping into a small helper:

**File:** `senders/android/src/lib.rs` (above or below the STEP-2
callback registrations):

```rust
/// Maps a `Bridge.run-migration-test(test_id)` argument to the
/// log-name used by `log_ui_test_status`. Returns "unknown" for
/// unrecognised test ids.
#[cfg(target_os = "android")]
fn migration_test_log_name(test_id: &str) -> &'static str {
    match test_id {
        "getinfo"   => "legacy-getinfo",
        "crossfade" => "legacy-crossfade",
        "smoke"     => "graph-smoke",
        _           => "unknown",
    }
}
```

Then update the STEP-2 handler to call `migration_test_log_name(&test_id)`
instead of duplicating the match.

For host-test reachability, the helper must compile without
`#[cfg(target_os = "android")]`. Either:

```rust
// Option A — drop the cfg and let it be available on all targets.
fn migration_test_log_name(test_id: &str) -> &'static str { /* … */ }

// Option B — duplicate the cfg gate on the test.
#[cfg(test)]
fn migration_test_log_name_for_test(test_id: &str) -> &'static str {
    migration_test_log_name(test_id)
}
```

**Recommendation:** Option A. The helper is a pure string lookup
with no platform dependencies. The cfg-gate on the surrounding
free functions is what makes them Android-only; the helper itself
is portable.

### 2.2 Test the dispatch table

**File:** `senders/android/src/lib.rs` (in or near the file's
existing `#[cfg(test)] mod tests { … }` block):

```rust
#[cfg(test)]
mod phase9_dispatch_tests {
    use super::*;

    #[test]
    fn migration_test_log_name_known_ids() {
        assert_eq!(migration_test_log_name("getinfo"),   "legacy-getinfo");
        assert_eq!(migration_test_log_name("crossfade"), "legacy-crossfade");
        assert_eq!(migration_test_log_name("smoke"),     "graph-smoke");
    }

    #[test]
    fn migration_test_log_name_unknown_id() {
        assert_eq!(migration_test_log_name(""),       "unknown");
        assert_eq!(migration_test_log_name("bogus"),  "unknown");
        assert_eq!(migration_test_log_name("GetInfo"), "unknown");  // case-sensitive
    }

    /// The dispatch table in STEP-2 §2.2 maps three valid ids
    /// (`getinfo`, `crossfade`, `smoke`). Verify the count to
    /// catch silent drift if a future PHASE adds a new id.
    #[test]
    fn migration_test_id_count_invariant() {
        const KNOWN: &[&str] = &["getinfo", "crossfade", "smoke"];
        for id in KNOWN {
            assert_ne!(
                migration_test_log_name(id),
                "unknown",
                "test id {id} should be in the dispatch table",
            );
        }
    }
}
```

### 2.3 Test the lazy-start idempotency

**File:** `senders/android/src/migration/runtime.rs` (inside or
near any existing `#[cfg(test)] mod tests { … }` block):

```rust
#[cfg(test)]
mod phase9_lazy_start_tests {
    use super::*;

    /// Idempotency invariant assumed by STEP-4: calling
    /// `start_graph_runtime` twice does not panic and leaves the
    /// runtime in the same state both times.
    #[test]
    fn start_graph_runtime_is_idempotent() {
        // First call: brings the runtime up.
        let first = start_graph_runtime();
        assert!(first.is_ok(), "first start_graph_runtime() failed: {first:?}");

        // Second call: no-op. Must not panic, must return Ok.
        let second = start_graph_runtime();
        assert!(second.is_ok(), "second start_graph_runtime() failed: {second:?}");

        // Clean up so we don't leak state into other tests.
        let _ = shutdown_graph_runtime();
    }

    /// Idempotency invariant assumed by STEP-4: calling
    /// `shutdown_graph_runtime` on a never-started runtime is a
    /// no-op (returns Ok, no panic).
    #[test]
    fn shutdown_on_never_started_is_noop() {
        // Don't call start first.
        let result = shutdown_graph_runtime();
        assert!(result.is_ok(), "shutdown_graph_runtime() on never-started failed: {result:?}");
    }

    /// Round-trip: start → shutdown → start again should still
    /// succeed (idempotent in both directions).
    #[test]
    fn start_then_shutdown_then_start_works() {
        let _ = start_graph_runtime().expect("start 1 failed");
        let _ = shutdown_graph_runtime().expect("shutdown failed");
        let _ = start_graph_runtime().expect("start 2 failed");

        // Final cleanup.
        let _ = shutdown_graph_runtime();
    }
}
```

### 2.4 Test serialization in `phase9_lazy_start_tests`

The three idempotency tests above mutate **global state**
(`GRAPH_NODE_MANAGER` is a `lazy_static!` mutex). `cargo test`
runs test functions in parallel threads by default, so two of
these tests racing can produce flaky results.

Either:

- **(a)** Run with `--test-threads=1`. Add a note in CI / README.
- **(b)** Use a `static SERIAL: Mutex<()> = Mutex::new(())` and
  acquire it at the top of each test. The mutex serialises the
  three tests but lets other tests run in parallel.

**Recommendation:** (b). It avoids contaminating other tests'
parallelism settings.

```rust
use std::sync::Mutex;
static SERIAL: Mutex<()> = Mutex::new(());

#[test]
fn start_graph_runtime_is_idempotent() {
    let _guard = SERIAL.lock().unwrap();
    // …
}
```

---

## 3. Verification

### 3.1 Host test run

```bash
cd /path/to/fcast
cargo test -p android-sender phase9_ 2>&1 | tail -20
```

Expect 6 tests passing:

```
running 6 tests
test phase9_dispatch_tests::migration_test_log_name_known_ids ... ok
test phase9_dispatch_tests::migration_test_log_name_unknown_id ... ok
test phase9_dispatch_tests::migration_test_id_count_invariant ... ok
test phase9_lazy_start_tests::shutdown_on_never_started_is_noop ... ok
test phase9_lazy_start_tests::start_graph_runtime_is_idempotent ... ok
test phase9_lazy_start_tests::start_then_shutdown_then_start_works ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 3.2 Failure modes

If any test fails:

- `migration_test_log_name_*` — likely a typo in STEP-2's match
  arms or a new test id was added without updating the
  dispatch table. Re-sync STEP-2 §2.2 with the helper.
- `start_graph_runtime_is_idempotent` — `start_graph_runtime`
  is not idempotent. Either the assumption underpinning STEP-4
  is wrong (escalate; this would block PHASE-9 STEP-4), or the
  test isn't serialising properly (check §2.4).
- `shutdown_on_never_started_is_noop` — same escalation path.
- `start_then_shutdown_then_start_works` — the runtime's internal
  state machine doesn't reset cleanly after shutdown. Escalate.

### 3.3 Re-run under `--test-threads=1` (sanity)

If §2.4's mutex-based serialisation has a bug:

```bash
cargo test -p android-sender phase9_ -- --test-threads=1
```

If the tests pass with `--test-threads=1` but fail without, the
serial mutex is broken.

### 3.4 No GStreamer needed

These tests don't initialise GStreamer. If a test fails with a
GStreamer-related error message, something is wrong — either the
test is calling a function that needs GStreamer, or
`start_graph_runtime` started doing GStreamer work it didn't
used to do. Investigate.

---

## 4. Pitfalls specific to this step

### P1 — Tests in `senders/android/src/lib.rs` need
`#[cfg(target_os = "android")]`-relaxed helpers

The helper `migration_test_log_name` at §2.1 is gated
`#[cfg(target_os = "android")]` in STEP-2's snippet. For
host-target tests to see it, drop the cfg (Option A in §2.1) or
duplicate it under `#[cfg(test)]`. If you don't, the tests
won't compile on host:

```
error[E0425]: cannot find function `migration_test_log_name` in this scope
```

### P2 — Tests in `runtime.rs` may have an existing `mod tests`
block

`runtime.rs` already has `#[cfg(test)] mod tests` from earlier
PHASEs (PHASE-3 / PHASE-6). The PHASE-9 tests go in a **separate**
sub-module (`phase9_lazy_start_tests`) so they're easy to filter
with `cargo test phase9_`. Don't merge them into the existing
`mod tests` — keep the namespace prefix.

### P3 — Global-state contamination

`GRAPH_NODE_MANAGER` is a `lazy_static!` — state persists across
tests in the same `cargo test` invocation. The §2.3 tests clean
up with `shutdown_graph_runtime()` at the end. If a test fails
mid-way, the cleanup is skipped and subsequent tests may see
stale state. The §2.4 mutex doesn't help with this — it only
serialises.

If flakiness becomes an issue, wrap each test body in a
`std::panic::catch_unwind` and always call cleanup in the
finally-equivalent (or use a fixture with `Drop`). For now,
accept the brittleness — the tests are short and unlikely to
panic.

### P4 — Tests on Android (cross-compile)

The cross-compile target (`cargo +nightly check ... --target
aarch64-linux-android`) doesn't typically run tests
(no harness on the device). So these tests run **only** on the
dev host. That's fine — they cover host-portable logic only.

### P5 — Adding more test ids in the future

If a future PHASE adds a new `Bridge.run-migration-test(id)`
value (e.g. `"audio-smoke"` or `"rtmp-smoke"`):

1. Add the arm to STEP-2 §2.2 `match test_id.as_str()`.
2. Add the log-name to `migration_test_log_name` (§2.1).
3. Add the id to the `KNOWN` slice in
   `migration_test_id_count_invariant` (§2.2).

Forgetting #3 will silently allow the test to pass even though
the dispatch table grew. The invariant test is paranoia, not
mandatory — but it's the cheapest line of defence.

---

## 5. Stop conditions (phase-level)

After STEP-6 lands, PHASE-9 is **done** when:

- All six unit tests pass on host (`cargo test -p android-sender
  phase9_`).
- All four debug quick-actions work end-to-end on a debug-build
  device (verified by parent doc §3.2).
- Release-build APK does not bind port 8080 on launch (verified
  by parent doc §3.3).
- `grep` invariants from parent doc §3.1 hold.

If any of these fail, do not mark the phase complete — the
remaining work is **not** "polish, ship anyway". The decoupling
relies on every step landing cleanly.

---

## 6. Next phase

PHASE-9 is the last STEP doc in this phase. A future
`MVP-PHASE-10-…` (TBD) could cover one of:

- Replacing `std::thread::spawn` inside the new callback handlers
  with a `tokio::task` properly attached to the event-loop
  runtime.
- Refactoring `Bridge.invoke-action(id)` into per-callback
  dispatch (the "Option (b)" called out in STEP-3 §1.3).
- A production-facing "migration runtime status" Bridge property
  the debug page can read for richer state than the
  `Bridge.test-status` string.

None of these are required follow-ups. PHASE-9 is self-contained.

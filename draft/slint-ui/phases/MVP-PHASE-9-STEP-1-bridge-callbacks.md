# MVP-PHASE-9 — Step 1: add `start-migration-server` / `run-migration-test` / `stop-migration-server` callbacks to `Bridge`

> Part 1 of 6. Parent doc: [`MVP-PHASE-9-debug-bridge-decoupling.md`](./MVP-PHASE-9-debug-bridge-decoupling.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Declare three new callbacks on the `Bridge` global in
`senders/android/ui/bridge.slint`. These are the entry points
through which the UI will dispatch to the migration runtime
(replacing the current direct `migration::runtime::*` calls
buried inside `on_invoke_action`):

- `start-migration-server(bind_addr: string)` — bring the
  migration runtime HTTP command server up at the given address.
  Idempotent.
- `run-migration-test(test_id: string)` — run one of the named
  debug tests (`"getinfo"`, `"crossfade"`, `"smoke"`).
- `stop-migration-server()` — explicit teardown of the migration
  runtime.

After this step, the Rust side compiles cleanly only if
[Step 2](./MVP-PHASE-9-STEP-2-rust-handlers.md) also lands in the
same commit (callbacks without handlers compile but emit a Slint
warning the first time they fire — see §3.1 below).

This is a **trivial Slint-only step** (~3 lines added). The work
happens in the dependent steps. This file exists to keep the diff
scoped.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `export global Bridge { … }` declaration | `senders/android/ui/bridge.slint:143-…` |
| `Bridge.invoke-action(string)` callback (the existing dispatcher) | `senders/android/ui/bridge.slint:239` |
| `Bridge.test-status: string` (already present — the display sink for the test handlers) | `senders/android/ui/bridge.slint:202` |
| Other Slint → Rust callback declarations (style reference) | `senders/android/ui/bridge.slint:236-247` |

### 1.2 Why three callbacks, not one

See parent doc §1.4. Short version: the lifecycle (start / run /
stop) is meaningful, and a single `migration-command(action, arg)`
two-string callback is harder to read at call sites.

### 1.3 Why `start-migration-server` takes the bind address

Today the bind address is read from `LEGACY_COMMAND_BIND_ADDR`
implicitly (`lib.rs:99`). Making it an explicit callback argument
documents the contract at the boundary — debug panels that want
to bind a different port (e.g. `"127.0.0.1:0"` for an
ephemeral-port test) can pass it directly without touching the
constant.

### 1.4 Naming convention

`bridge.slint` uses kebab-case (`callback start-casting`,
`callback stop-casting`, etc.). Match it. The Rust glue generates
snake_case method names automatically (e.g.
`on_start_migration_server`, `invoke_start_migration_server`).

---

## 2. The change

**File:** `senders/android/ui/bridge.slint`

**Before** (around line 235-247, "Callbacks (Slint → Rust)"
section):

```slint
// ── Callbacks (Slint → Rust) ─────────────────────────────────────────
callback connect-receiver(string);
callback start-casting(scale-width: int, scale-height: int, max-framerate: int);
callback stop-casting();
callback invoke-action(string);
callback start-recording();
callback pause-recording();
callback resume-recording();
callback stop-recording();
callback engage-lock();
callback engage-stealth();
```

**After:**

```slint
// ── Callbacks (Slint → Rust) ─────────────────────────────────────────
callback connect-receiver(string);
callback start-casting(scale-width: int, scale-height: int, max-framerate: int);
callback stop-casting();
callback invoke-action(string);
callback start-recording();
callback pause-recording();
callback resume-recording();
callback stop-recording();
callback engage-lock();
callback engage-stealth();

// ── Migration-runtime callbacks (MVP-PHASE-9) ───────────────────────
//
// Routed through the Bridge so the UI layer remains agnostic to the
// `migration::runtime::*` Rust API. See `MVP-PHASE-9-debug-bridge-
// decoupling.md` §1.3 for the rationale.
callback start-migration-server(string);     // (bind-addr)
callback run-migration-test(string);         // (test-id: "getinfo" | "crossfade" | "smoke")
callback stop-migration-server();
```

### 2.1 Why a separate section comment

`bridge.slint` already groups callbacks by domain (`Backup / reset`,
`Cast history`, `Bitrate presets`, `Callbacks (Slint → Rust)`).
Adding a dedicated `Migration-runtime callbacks (MVP-PHASE-9)`
section matches that convention and gives future readers a single
place to add more callbacks (e.g. `restart-migration-server()`,
`get-migration-runtime-status() -> string`).

### 2.2 Why the parameter-name annotations in comments

The Slint callback-declaration syntax doesn't carry parameter
names (`callback foo(string);` is equivalent to
`callback foo(name: string);`). The inline comment makes the
expected value space explicit at the declaration site.

### 2.3 Why no `out` callbacks (e.g. returning a result)

Slint callbacks can have return types
(`callback foo(string) -> string`). The current four debug tests
deliver their results **asynchronously** via `set_test_status` —
the result is not in the callback's return value. Keep that
asynchronous pattern; a synchronous return type would conflict
with the `std::thread::spawn` model used by
`run_legacy_http_*_test`.

---

## 3. Verification

### 3.1 Slint compile

```bash
cargo +nightly check -p android-sender --target aarch64-linux-android
```

This step **does not compile** without
[Step 2](./MVP-PHASE-9-STEP-2-rust-handlers.md), because the Rust
side will fail with:

```
warning: Bridge callback `start-migration-server` has no handler
warning: Bridge callback `run-migration-test` has no handler
warning: Bridge callback `stop-migration-server` has no handler
```

That's expected — the warnings become errors only if the
callbacks are invoked at runtime without handlers. The combined
Step 1 + Step 2 commit is what gets a clean build. Don't ship
Step 1 in isolation.

### 3.2 Grep

```bash
grep -nE 'callback start-migration-server|callback run-migration-test|callback stop-migration-server' \
    senders/android/ui/bridge.slint
# → exactly 3 matches.

grep -n 'callback start-casting' senders/android/ui/bridge.slint
# → still 1 match (unchanged — sanity that the existing section is intact).
```

### 3.3 Slint generated glue

After Step 1 lands, `slint-build`'s generated Rust glue gains
three new methods on the `Bridge` generated type:

| Slint side | Generated Rust method | Used in Step |
|---|---|---|
| `Bridge.start-migration-server("0.0.0.0:8080")` | `Bridge::invoke_start_migration_server(self, bind_addr: SharedString)` | STEP-3 (UI → callback dispatch) |
| (handler registration) | `Bridge::on_start_migration_server(self, handler: impl Fn(SharedString) + 'static)` | STEP-2 (Rust handler) |
| `Bridge.run-migration-test("getinfo")` | `Bridge::invoke_run_migration_test(self, test_id: SharedString)` | STEP-3 |
| (handler registration) | `Bridge::on_run_migration_test(self, handler: impl Fn(SharedString) + 'static)` | STEP-2 |
| `Bridge.stop-migration-server()` | `Bridge::invoke_stop_migration_server(self)` | STEP-3 |
| (handler registration) | `Bridge::on_stop_migration_server(self, handler: impl Fn() + 'static)` | STEP-2 |

These methods exist after the next `cargo build` pulls in the
regenerated glue. Verify with:

```bash
# Re-run slint-build, then look for the generated methods.
cargo build -p android-sender --target aarch64-linux-android
grep -rn 'invoke_start_migration_server\|on_start_migration_server' \
    target/aarch64-linux-android/debug/build/*/out/*.rs | head -6
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting to update the Rust side

If you land Step 1 alone, the build emits three "no handler" Slint
warnings and the callbacks become no-ops at runtime. The four
debug quick-actions in STEP-3 will silently do nothing. Cluster
Steps 1 + 2 + 3 together — or land Steps 1 + 2 first and STEP-3
second.

### P2 — Wrong section in `bridge.slint`

`bridge.slint` is ~250 lines and has ~10 callback-declaration
sections grouped by domain (Phase 8 / Cluster X). Don't paste the
new declarations into the wrong section — search for the
`// ── Callbacks (Slint → Rust) ────` header (around line 235)
and add the new section **immediately after** the last callback
in that block. The grep in §3.2 will confirm placement.

### P3 — Renaming the callbacks

Tempting to rename to e.g. `migration-server-start` /
`migration-server-stop` (matching the noun-verb order of
`save-bar-actions`). Don't — the parent doc and the user research
both use `start-migration-server` / `stop-migration-server`. Pick
one convention and stick with it across the doc set.

### P4 — Adding an extra `out` parameter for the test result

```slint
// Don't:
callback run-migration-test(string) -> string;
```

The handler runs asynchronously on a worker thread (see parent
doc §1 / pitfall P3). Returning a `string` synchronously from a
Slint callback that fires `std::thread::spawn` requires
`tokio::sync::oneshot` plumbing across the Slint → Rust
boundary, which is not worth it. Keep the
`set_test_status(...)`-based async-delivery model.

### P5 — Forgetting that callbacks have no Slint defaults

Properties have defaults (`in property <string> foo: "";`).
Callbacks do not — they're either invoked or unregistered. So the
"no handler" warning from §3.1 is purely advisory — it does not
prevent compilation. Don't be misled into thinking the build is
broken when you see those warnings on a transient Step 1-only
commit.

---

## 5. Next step

Once this lands, [Step 2](./MVP-PHASE-9-STEP-2-rust-handlers.md)
registers the matching Rust handlers via
`ui.global::<Bridge>().on_start_migration_server(...)` etc.,
each one delegating to the existing free functions
(`start_migrated_command_server`, `run_legacy_http_*_test`,
`run_graph_smoke_test`, `shutdown_graph_runtime`).

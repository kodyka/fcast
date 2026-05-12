# MVP-PHASE-6 — Step 1: define cast-graph node IDs in one place

> Part 1 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Declare three `const &str` IDs at the top of
`senders/android/src/lib.rs` that name the screen-capture source, the
WHEP destination, and the link between them. Subsequent steps refer
to these constants by name instead of repeating string literals
across handlers.

After this step, the Android sender has three new `const` items but
no other behavioural change.

This is a **trivial, zero-risk step** — one of the smallest in
PHASE-6. The intent is to keep [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
and [Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md) readable.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| The `lazy_static!` block (insertion point) | `senders/android/src/lib.rs:65-77` |
| `Event::StartCast` / `Event::CaptureStarted` handlers (which will read the IDs) | `lib.rs:875-961, 963-1018` |
| `stop_cast` (which will read the IDs) | `lib.rs:682-709` |
| `Event::SignallerStarted` (which will read the destination ID) | `lib.rs:754-794` |

### 1.2 Why hard-coded IDs (not a `UuidV4`)

The Android sender supports **one active cast at a time** — this is
already implicit in the existing code via `Option<WhepSink>` and
`Option<active_device>`. Hard-coding the three IDs:

- Keeps the smoke test reproducible (`curl /command -d '{"getinfo":{"id":"cast-whep-1"}}'`).
- Avoids threading a UUID through three handler bodies.
- Lets `Remove cast-source-1` in `stop_cast` be unconditional —
  if the previous cast wasn't fully torn down, we re-claim the
  same ID and the runtime returns a friendly "already exists"
  error on the next `CreateScreenCaptureSource`, which we treat
  as a no-op.

### 1.3 Why `#[cfg(target_os = "android")]` on every const

The migration runtime is currently Android-only in MVP scope. The
desktop sender keeps the legacy `mcore::transmission::WhepSink` path
(see [Step 5](./MVP-PHASE-6-STEP-5-tx-sink-cfg-gate.md)). Gating the
constants means desktop targets don't carry dead symbols and the
diff stays minimal.

---

## 2. The change

**File:** `senders/android/src/lib.rs`

Insert after the `lazy_static!` block (around line 77):

```rust
// senders/android/src/lib.rs

// Graph node IDs for the unified cast loop (MVP-PHASE-6).
// One source, one destination, one link — the entire cast loop.
#[cfg(target_os = "android")]
const CAST_SOURCE_ID: &str = "cast-screen-1";
#[cfg(target_os = "android")]
const CAST_DESTINATION_ID: &str = "cast-whep-1";
#[cfg(target_os = "android")]
const CAST_LINK_ID: &str = "cast-link-1";
```

### 2.1 Naming convention

- `cast-` prefix groups all three so they don't collide with future
  graph nodes spawned by the smoke quick-action.
- Numeric suffix (`-1`) reserves room for a future
  multi-active-cast feature without breaking the wire format.
- All lowercase, hyphen-separated — matches the smoke-test
  fixtures in PHASE-3.

### 2.2 Alternative: a single struct

```rust
#[cfg(target_os = "android")]
struct CastIds {
    source: &'static str,
    destination: &'static str,
    link: &'static str,
}

#[cfg(target_os = "android")]
const CAST_IDS: CastIds = CastIds {
    source: "cast-screen-1",
    destination: "cast-whep-1",
    link: "cast-link-1",
};
```

Marginally more "typed", but the call-site syntax
(`CAST_IDS.source` vs `CAST_SOURCE_ID`) doesn't read any better.
Stick with the flat constants — matches the existing
`COMMAND_BIND_ENV` etc. style in the same file.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. The constants are unused at this point, so you'll
see three `warning: unused constant` warnings — that's correct.
**Don't** silence them with `#[allow(dead_code)]`; they clear on
their own once [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
references them.

### 3.2 Grep

```bash
grep -nE 'const CAST_(SOURCE|DESTINATION|LINK)_ID' senders/android/src/lib.rs
# → exactly 3 matches
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting `#[cfg(target_os = "android")]`

If you skip the `cfg` gate, the desktop sender will see three
unused constants forever. Worse, if any future desktop-side code
accidentally references one, the compile will succeed but the
behaviour won't match (desktop has no migration runtime). Always
gate.

### P2 — Don't pull IDs from a JSON config

Tempting (less hardcoding, more flexible). But IDs change rarely
and the smoke tests would need a way to query them at runtime.
Hardcoded constants are simpler and CI-friendlier.

### P3 — `&str` vs `String`

Use `&'static str`. The runtime's `Command::Create*` variants take
`String`, so the call sites in [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
do `CAST_SOURCE_ID.into()` (one allocation per cast start — fine).
Using `String` would require `lazy_static!` and force allocations
in the consts themselves, which Rust doesn't support directly.

---

## 5. Next step

Once this lands, [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
replaces the body of `Event::CaptureStarted` (the legacy
`appsrc` + `WhepSink::new` construction) with a sequence of
`handle_command` calls using these IDs, plus a `tokio::spawn`
poll loop that watches `getinfo` until the WHEP bound port is
populated.

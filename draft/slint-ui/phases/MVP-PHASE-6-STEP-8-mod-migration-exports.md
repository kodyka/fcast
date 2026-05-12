# MVP-PHASE-6 — Step 8: shorten import paths via `migration/mod.rs` re-exports

> Part 8 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 7 — preserve `set_capture_active(false)`](./MVP-PHASE-6-STEP-7-set-capture-active-preservation.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

After [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) and
[Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md), the
`lib.rs` cast loop references types and functions from three
sub-modules of `migration/`:

- `migration::protocol::{Command, CommandResult, DestinationFamily, NodeInfo}`
- `migration::runtime::handle_command`
- `migration::nodes::{...}` (transitively, via the runtime)

This step re-exports the most common items at
`migration/mod.rs` so the call sites can drop a level of
qualification — `migration::Command` instead of
`migration::protocol::Command`, etc.

This is an **optional, cosmetic step** (~7 lines added in
`mod.rs`). Strictly speaking the cast loop compiles fine without
it. Skip if you prefer verbose paths.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `senders/android/src/migration/mod.rs` | currently lists `pub mod` declarations only |
| `protocol::Command`, `protocol::CommandResult`, etc. | `senders/android/src/migration/protocol.rs` |
| `runtime::handle_command` | `senders/android/src/migration/runtime.rs` |

Verify the current contents:

```bash
cat senders/android/src/migration/mod.rs
```

Expected (approximate):

```rust
pub mod media_bridge;
pub mod messages;
pub mod node_manager;
pub mod nodes;
pub mod protocol;
pub mod runtime;
```

### 1.2 Why re-exports help (and where they hurt)

**Help:**

- `migration::Command` reads better than `migration::protocol::Command`.
- Auto-import via rust-analyser is shorter (one fewer module hop).
- Convention matches `senders/desktop/src/migration/mod.rs` (if/when
  it exists).

**Hurt:**

- A future reader looking for `Command`'s definition has to search
  for `pub enum Command` inside `protocol.rs` instead of seeing it
  in the import path.
- Two sources of truth: `migration::Command` and
  `migration::protocol::Command` both work, so codebases can drift.

The tradeoff is mild. The PHASE-6 doc recommends *(a)* doing the
re-exports because the call sites in [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md)
and [Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md) are already
long.

### 1.3 What to re-export — the minimum set

Just the four items the cast loop actually uses:

```
Command          (the enum that drives the runtime)
CommandResult    (Success / Error / Info)
DestinationFamily (Whep variant in particular)
NodeInfo         (matched against ::Destination in the poll loop)
```

Add more later if other callers emerge. Don't over-export.

---

## 2. The change

**File:** `senders/android/src/migration/mod.rs`

**Before:**

```rust
pub mod media_bridge;
pub mod messages;
pub mod node_manager;
pub mod nodes;
pub mod protocol;
pub mod runtime;
```

**After:**

```rust
pub mod media_bridge;
pub mod messages;
pub mod node_manager;
pub mod nodes;
pub mod protocol;
pub mod runtime;

// PHASE-6 — re-exports for the common types used by the unified
// cast loop in `senders/android/src/lib.rs`. Keep narrow; expand
// only when new call sites need it.
pub use protocol::{Command, CommandResult, DestinationFamily, NodeInfo};
```

### 2.1 Don't re-export `handle_command`

`runtime::handle_command` is invoked by name in Step 2 / Step 4 —
re-exporting at `migration::handle_command` makes the call shorter
(`migration::handle_command(...)`) but loses the `runtime::`
breadcrumb. The benefit is small; leave `runtime::handle_command`
as-is.

### 2.2 Don't add `pub use nodes::*`

`nodes` exposes many types (`SourceNode`, `DestinationNode`,
`MixerNode`, `VideoGeneratorNode`, `ScreenCaptureNode`, plus
internal helpers). Glob-re-exporting them at `migration::*` would
flood the namespace. Stick with the explicit four items in §2.

### 2.3 (Optional) Update the call sites in Step 2 / Step 4

After this re-export lands, the snippets in
[Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) can be
tightened:

```rust
// Before (verbose, but works without Step 8):
use crate::migration::protocol::{Command, CommandResult, DestinationFamily};

// After (with Step 8 re-exports):
use crate::migration::{Command, CommandResult, DestinationFamily};
```

This is a strictly aesthetic edit. If you prefer the verbose path,
skip the re-write.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Clean. The new `pub use` line is benign — it just makes existing
items reachable via a shorter path.

### 3.2 Grep

```bash
grep -nE '^pub use protocol::' senders/android/src/migration/mod.rs
# → exactly 1 match
```

### 3.3 Optional: check call-site simplifications

```bash
# How many lib.rs call sites use the longer path?
grep -cE 'crate::migration::protocol::' senders/android/src/lib.rs
# Compare to:
grep -cE 'crate::migration::(Command|CommandResult|DestinationFamily|NodeInfo)\b' senders/android/src/lib.rs
```

Either count is fine; the re-export just lets you choose.

---

## 4. Pitfalls specific to this step

### P1 — `glob` re-exports leak internal types

Don't do `pub use protocol::*` — that exposes internal helpers
(serde defaults, private structs the protocol crate uses) into
`migration::*`. Use the explicit four-item list.

### P2 — Forgetting to keep `protocol::Command` reachable

The old `crate::migration::protocol::Command` path still works
(because `pub mod protocol;` re-exports the module). The new
`pub use` is *additive*. Don't replace the `pub mod protocol;`
with `mod protocol;` — that breaks the long-form path that other
modules (and tests) may rely on.

### P3 — Name collisions

If `protocol::Command` collides with something else in
`migration::`, the compiler errors with `the name Command is
defined multiple times`. There are no current collisions, but
adding `pub use nodes::Source` would conflict with
`protocol::Source` if it existed. Audit the four chosen names
before re-exporting.

### P4 — Don't add `#[doc(hidden)]`

Tempting if you're worried about exposing too much surface area.
But these are internal to `senders/android` — there is no public
docs surface. Leave the re-exports unmarked.

---

## 5. Next step

Once this lands, [Step 9](./MVP-PHASE-6-STEP-9-optional-feature-flag.md)
adds an **optional** runtime feature flag
(`FCAST_UNIFIED_CAST_GRAPH=0/1`) so you can canary the unified path
against the legacy path in production builds. Strictly optional —
skip if you prefer a big-bang switch.

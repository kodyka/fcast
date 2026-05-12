# MVP-PHASE-5 — Step 5: re-export `WhepServerSignaller` into the migration crate

> Part 5 of 7. Parent doc: [`MVP-PHASE-5-whep-destination-family.md`](./MVP-PHASE-5-whep-destination-family.md).
> Previous: [Step 4 — wire the Whep arm in `build_live_pipeline`](./MVP-PHASE-5-STEP-4-build-live-pipeline.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Make `WhepServerSignaller` and `ON_SERVER_STARTED_SIGNAL_NAME`
visible to the migration crate so that
[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md)'s
`build_live_pipeline` arm can import them.

Today, `whep_signaller.rs` is a **private** module of `mcore`
(`sdk/mirroring_core/src/whep_signaller.rs`), consumed only by
`mcore::transmission::WhepSink`. The migration crate currently has
no access path.

This step is **the only SDK-crate change in PHASE-5.** Keep it
minimum-diff — a single `pub mod` re-export. Larger refactors (e.g.
moving the signaller into a standalone `crates/whep-signaller/`
crate) are deferred until after MVP-PHASE-6, when the legacy
`WhepSink` call site is removed.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `whep_signaller.rs` (the actual signaller impl) | `sdk/mirroring_core/src/whep_signaller.rs:1-575` |
| `ON_SERVER_STARTED_SIGNAL_NAME` constant | `sdk/mirroring_core/src/whep_signaller.rs:7` |
| `mcore` crate root | `sdk/mirroring_core/src/lib.rs` |
| Current `mod whep_signaller;` declaration in `lib.rs` | search for `mod whep_signaller` in `sdk/mirroring_core/src/lib.rs` |
| Migration crate Cargo deps | `senders/android/Cargo.toml` (search for `mcore` or `mirroring_core`) |

### 1.2 Two options for exposing the signaller

| Option | Diff size | When to choose |
|---|---|---|
| (a) `pub mod whep_signaller;` in `mcore::lib.rs` — re-export from the existing private module | 1 line | **Preferred for PHASE-5.** Minimum diff, no file moves. |
| (b) Move `whep_signaller.rs` into a new `crates/whep-signaller/` crate. `mcore` and the migration crate both depend on it. | ~150 lines (Cargo.toml + path updates + new crate skeleton) | The right end-state once MVP-PHASE-6 removes the `mcore::transmission::WhepSink` call site and `mcore` no longer needs to bundle the signaller. |

This step does (a). Defer (b) to post-PHASE-6.

### 1.3 Why a `pub mod` is sufficient (not `pub use ...::*`)

`pub mod whep_signaller;` exposes the whole module — including
`WhepServerSignaller`, `ON_SERVER_STARTED_SIGNAL_NAME`, and any
private items the impl uses internally. A more surgical
`pub use whep_signaller::{WhepServerSignaller, ON_SERVER_STARTED_SIGNAL_NAME};`
at the crate root would expose only the two needed names — but the
migration crate's `use mcore::whep_signaller::*;` is more
readable and matches the existing convention (other `mcore`
modules are exposed as `pub mod`).

**Pick `pub mod` for minimum diff.**

### 1.4 Why the migration crate uses a `whep_signaller_compat` shim

[Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) §2 imports the
signaller as:

```rust
use crate::whep_signaller_compat::WhepServerSignaller;
```

The `whep_signaller_compat` shim is a one-line re-export in the
migration crate that maps to whichever path Step 5 ends up using.
This indirection means that if PHASE-5 lands with option (a) and a
later refactor moves the signaller to a separate crate (option (b)),
**only the shim file changes** — `build_live_pipeline` is untouched.

```rust
// senders/android/src/whep_signaller_compat.rs (new file)
pub use mcore::whep_signaller::*;
```

Or, if the signaller later moves to its own crate:

```rust
pub use whep_signaller::*;
```

The shim is a 1-line file. It's the kind of indirection that pays
for itself the first time you refactor.

---

## 2. The change

### 2.1 Edit `mcore::lib.rs` — flip `mod` to `pub mod`

**File:** `sdk/mirroring_core/src/lib.rs`

Find the existing declaration:

```rust
mod whep_signaller;
```

…and change it to:

```rust
pub mod whep_signaller;
```

That's the **entire** SDK-crate change for PHASE-5. One word added.

If the declaration doesn't exist (i.e. `whep_signaller.rs` is
referenced only via `use crate::whep_signaller::*;` inside
`transmission.rs`), add the declaration explicitly:

```rust
pub mod whep_signaller;
```

…and remove any duplicate `mod whep_signaller;` from inside
`transmission.rs`.

### 2.2 Add the compat shim in the migration crate

**File:** `senders/android/src/whep_signaller_compat.rs` (new file,
1 line):

```rust
// senders/android/src/whep_signaller_compat.rs

//! Migration-crate-local re-export of mcore's WhepServerSignaller.
//!
//! This indirection lets PHASE-5 use a stable
//! `crate::whep_signaller_compat::WhepServerSignaller` import path,
//! regardless of whether the signaller lives in `mcore` (current,
//! PHASE-5) or a standalone `whep-signaller` crate (post-PHASE-6).

pub use mcore::whep_signaller::*;
```

### 2.3 Register the shim in `senders/android/src/lib.rs`

**File:** `senders/android/src/lib.rs` (top, alongside other
`mod` declarations)

```rust
mod whep_signaller_compat;
```

If `lib.rs` has a `pub mod` for other internal shims, follow that
convention; otherwise plain `mod` keeps it crate-private.

### 2.4 (Optional) Re-export bitrate constants

If [Step 4](./MVP-PHASE-5-STEP-4-build-live-pipeline.md) §1.3
option (a) was chosen (re-export the bitrate constants instead of
duplicating them), add to `mcore::lib.rs`:

```rust
pub use crate::transmission::{
    WHEP_MIN_BITRATE,
    WHEP_START_BITRATE,
    WHEP_MAX_BITRATE,
};
```

Then in the migration arm, replace
`crate::migration::constants::WHEP_*` with `mcore::WHEP_*`.

If option (b) (duplicate) was chosen, no constant re-export is
needed — the migration crate owns its own constants module.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p mirroring_core
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

After Step 5 lands, both crates compile. The migration crate's
import path `crate::whep_signaller_compat::WhepServerSignaller`
resolves; the SDK crate's existing `mcore::transmission::WhepSink`
still works (the `pub mod` change is additive — the legacy call
site uses `use crate::whep_signaller::*;` which still works inside
`mcore`).

### 3.2 Test that the migration crate can construct the signaller

Drop into the `#[cfg(test)] mod tests` block in
`senders/android/src/migration/nodes/destination.rs`:

```rust
#[test]
fn migration_crate_can_import_whep_signaller() {
    // Compile-time check that the re-export and shim are wired up.
    // This test doesn't actually run the signaller (which would
    // require a Glib main loop) — it just exercises the import.
    let _signaller: Option<crate::whep_signaller_compat::WhepServerSignaller> = None;

    // Same for the signal-name constant.
    assert!(!crate::whep_signaller_compat::ON_SERVER_STARTED_SIGNAL_NAME.is_empty());
}
```

The test compiles and runs on the host; no Android emulator needed.

### 3.3 Grep recipe

```bash
grep -n 'pub mod whep_signaller' sdk/mirroring_core/src/lib.rs
# → expect: 1 match (the line edited in Step 5 §2.1).

grep -rn 'whep_signaller_compat' senders/android/src/
# → expect: matches in lib.rs (the mod declaration) and
#   migration/nodes/destination.rs (the import in Step 4).

grep -n 'mcore::transmission::WhepSink' senders/android/src/
# → expect: the legacy call site in lib.rs:943-950 — **unchanged**
#   by this step. PHASE-6 removes it.
```

---

## 4. Pitfalls specific to this step

### S5-P1 — Changing `mod` to `pub mod` in the wrong file

The signaller is consumed inside `mcore::transmission` via
`use crate::whep_signaller::*;`. The `mod whep_signaller;`
declaration **must** be at `sdk/mirroring_core/src/lib.rs` — the
crate root.

If the declaration currently lives elsewhere (e.g. as a sub-module
of `transmission.rs`), moving it to the crate root is a slightly
larger change but still small. **Don't add a duplicate
`pub mod` in `lib.rs` while leaving the original `mod` in
`transmission.rs`** — that creates two parallel modules with the
same name and `cargo check` complains.

### S5-P2 — Re-exporting more than needed

```rust
pub use whep_signaller::*;  // ❌ exposes everything publicly
```

The `pub mod whep_signaller;` version exposes the module name; the
items inside it are public by virtue of their existing `pub` on the
type / constant declarations. Re-exporting with a wildcard at the
crate root **also** works, but pollutes the crate's top-level
namespace.

Prefer:

```rust
pub mod whep_signaller;
```

…and let consumers reach in via `mcore::whep_signaller::*`.

### S5-P3 — Forgetting to add the migration-crate shim's `mod` declaration

```rust
// senders/android/src/lib.rs

// (no `mod whep_signaller_compat;` here)
```

Without the `mod` declaration, the file
`senders/android/src/whep_signaller_compat.rs` is just an
unreferenced file on disk; the Rust compiler doesn't pick it up
automatically. `crate::whep_signaller_compat::*` fails with
`unresolved module 'whep_signaller_compat'`.

### S5-P4 — Skipping the shim entirely

```rust
// build_live_pipeline arm — direct mcore import:
use mcore::whep_signaller::{WhepServerSignaller, ON_SERVER_STARTED_SIGNAL_NAME};
```

This works **today**. But it couples PHASE-5's pipeline code to the
current `mcore::whep_signaller` location. When PHASE-6 / future
refactors move the signaller to its own crate, the migration arm
breaks.

The shim costs one new file (1 line) and decouples PHASE-5 from any
future signaller home. **Keep the shim.** (If you really want to
skip it, document the coupling in a `// TODO(phase-6):` comment so
the next maintainer knows where to look.)

### S5-P5 — Re-exporting via `pub use` at the crate root

```rust
// sdk/mirroring_core/src/lib.rs
pub use whep_signaller::WhepServerSignaller;
pub use whep_signaller::ON_SERVER_STARTED_SIGNAL_NAME;
```

Works, but the migration-crate import becomes
`mcore::WhepServerSignaller` (no `whep_signaller::` segment),
flattening the namespace. That's fine — but inconsistent with the
"keep the module name in the path" convention used elsewhere in
`mcore`.

Default to `pub mod` (the entire-module re-export) for namespace
consistency.

### S5-P6 — Touching `transmission.rs`

This step **only** edits `sdk/mirroring_core/src/lib.rs`. Do NOT
touch `transmission.rs` — the legacy `WhepSink` call site continues
to work unchanged via the same `mcore::whep_signaller::*` import
path. Editing `transmission.rs` is in MVP-PHASE-6's scope, not
PHASE-5's.

---

## 5. Next step

After this lands, [Step 6 — extend `LiveDestinationPipeline` to carry the port handle](./MVP-PHASE-5-STEP-6-live-pipeline-port-handle.md)
adds the `whep_bound_ports: Option<Arc<Mutex<Option<(u16, u16)>>>>`
field on `LiveDestinationPipeline` and wires `refresh()` to read it
back into the `DestinationNode.whep_bound_port_*` fields that Step 3
added.

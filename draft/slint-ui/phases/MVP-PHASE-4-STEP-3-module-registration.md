# MVP-PHASE-4 — Step 3: register the `screen_capture` module

> Part 3 of 6. Parent doc: [`MVP-PHASE-4-screen-capture-source-node.md`](./MVP-PHASE-4-screen-capture-source-node.md).
> Previous: [Step 2 — define `ScreenCaptureNode`](./MVP-PHASE-4-STEP-2-screen-capture-node.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Make the new `screen_capture` module reachable from
`node_manager.rs` by:

- Adding `pub mod screen_capture;` to
  `senders/android/src/migration/nodes/mod.rs`.
- Adding `pub use screen_capture::*;` so callers can write
  `use crate::migration::nodes::ScreenCaptureNode;`.

This is a **single-file, two-line change.** The whole point of this
step is to keep the diff diff-able and reviewable on its own — the
real work is in Steps 2, 4, and 5.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `nodes/mod.rs` (the module aggregator) | `senders/android/src/migration/nodes/mod.rs` |
| Existing children | `nodes/control.rs`, `nodes/destination.rs`, `nodes/mixer.rs`, `nodes/source.rs`, `nodes/video_generator.rs` |
| The new file (created in Step 2) | `senders/android/src/migration/nodes/screen_capture.rs` |

### 1.2 Why we need both `pub mod` and `pub use`

| Statement | What it does |
|---|---|
| `pub mod screen_capture;` | Makes the file visible to the compiler. |
| `pub use screen_capture::*;` | Makes the items inside re-exportable as `crate::migration::nodes::ScreenCaptureNode` (the path used in [Step 4](./MVP-PHASE-4-STEP-4-node-record.md) and [Step 5](./MVP-PHASE-4-STEP-5-dispatch-arm.md)). |

Without the `pub use`, callers would have to write
`crate::migration::nodes::screen_capture::ScreenCaptureNode` which is
verbose and inconsistent with the existing siblings.

### 1.3 The existing convention

`nodes/mod.rs` already follows a strict pattern: one `pub mod` line
per file, then one `pub use child::*;` line per child. PHASE-4 just
adds the same two lines for the new file.

---

## 2. The change

**File:** `senders/android/src/migration/nodes/mod.rs`

```rust
// senders/android/src/migration/nodes/mod.rs

pub mod control;
pub mod destination;
pub mod mixer;
pub mod source;
pub mod video_generator;

// NEW —
pub mod screen_capture;

pub use control::*;
pub use destination::*;
pub use mixer::*;
pub use source::*;
pub use video_generator::*;

// NEW —
pub use screen_capture::*;
```

### 2.1 Why alphabetical order matters here

The existing file groups the `pub mod` declarations alphabetically
and the `pub use` re-exports the same way. Keeping `screen_capture`
in alphabetical order (between `mixer` and `source`) is more readable
but **deliberately not** what the snippet above does — it puts the
new lines at the end so the diff stays small (two added lines, zero
moved lines).

Pick whichever your reviewers prefer. The compiler is indifferent.

### 2.2 Why `pub mod` and not `pub(crate) mod`

`pub(crate)` would work and would be marginally tighter, but it
breaks the existing convention. All sibling modules are `pub mod` —
matches the rest of `nodes/mod.rs`.

---

## 3. Verification

### 3.1 Compile check

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean** — the new file from Step 2 now sees the rest of the
crate, and downstream `use crate::migration::nodes::ScreenCaptureNode;`
imports in Steps 4 and 5 will resolve.

Most likely failures:

- `error[E0583]: file not found for module screen_capture` — Step 2
  was not committed. Re-check that
  `senders/android/src/migration/nodes/screen_capture.rs` exists.
- `unused import: …` warnings on the `pub use screen_capture::*;` —
  expected until Step 4 and Step 5 land. Don't suppress with
  `#[allow(unused_imports)]`; just let the warning land for now.

### 3.2 Grep

```bash
grep -n 'pub mod screen_capture' senders/android/src/migration/nodes/mod.rs
# → exactly 1 match
grep -n 'pub use screen_capture' senders/android/src/migration/nodes/mod.rs
# → exactly 1 match
grep -nE '(pub mod|pub use) (control|destination|mixer|source|video_generator|screen_capture)' \
    senders/android/src/migration/nodes/mod.rs
# → exactly 12 matches (6 pub mod + 6 pub use)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting the `pub use`

If you only add `pub mod screen_capture;` (without
`pub use screen_capture::*;`), the type still exists at
`crate::migration::nodes::screen_capture::ScreenCaptureNode` but not
at the shorter `crate::migration::nodes::ScreenCaptureNode` path the
rest of the codebase uses. Steps 4 and 5 will then fail with
`cannot find type ScreenCaptureNode in this scope`, which is a
confusing error if you don't know to look at this file.

### P2 — Re-exporting an unused module produces a warning

Until Steps 4 and 5 land, the `pub use screen_capture::*;` line will
trigger `warning: unused import`. **Don't** silence it with
`#[allow(unused_imports)]` — the warning correctly tells the reader
that the registration is dormant. It clears on its own once Step 4 lands.

### P3 — Don't reorder existing entries

Tempting to alphabetise the existing five entries while you're in
the file. Don't — it inflates the diff. Add the two new lines at the
end (or just before `pub use control::*;` if you really want
alphabetical) and leave the rest.

### P4 — Test gating

If you wrap the new module in `#[cfg(target_os = "android")]`,
the Step 6 unit tests (which run host-side) will fail to find
`ScreenCaptureNode`. The struct itself doesn't depend on Android-
specific APIs (only `crate::FRAME_PAIR` does, and that's a `pub`
global on all targets thanks to `#[cfg(target_os = "android")]`
already wrapping its actual initialisation). Keep the module
declaration unconditional and let the GStreamer code in the
pipeline body do any target-specific gating if needed.

---

## 5. Next step

Once this lands, [Step 4](./MVP-PHASE-4-STEP-4-node-record.md)
extends `NodeRecord` in
`senders/android/src/migration/node_manager.rs` with a new
`ScreenCapture(ScreenCaptureNode)` variant and threads it through
every `match self` arm in `impl NodeRecord`.

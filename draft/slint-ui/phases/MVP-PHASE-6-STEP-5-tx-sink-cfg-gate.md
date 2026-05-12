# MVP-PHASE-6 — Step 5: gate the `tx_sink` field behind `cfg(not(target_os = "android"))`

> Part 5 of 9. Parent doc: [`MVP-PHASE-6-graph-command-cast-loop.md`](./MVP-PHASE-6-graph-command-cast-loop.md).
> Previous: [Step 4 — replace `stop_cast`](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

After Steps 2, 3, and 4, no Android code path reads or writes
`self.tx_sink`. This step deletes the field from the Android build
by adding `#[cfg(not(target_os = "android"))]` to the field
declaration and its initialiser, and to every remaining
read site.

After this step, `mcore::transmission::WhepSink` is **dead code on
Android** — the Android binary no longer carries the symbol.

This is a **small, mechanical step** (~10 lines). The compiler will
catch every remaining `self.tx_sink` read after Steps 2 and 4 — they
all need a matching `cfg` gate or removal.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `tx_sink: Option<WhepSink>` field | `senders/android/src/lib.rs:537` |
| `tx_sink: None` initialiser | `lib.rs:602` |
| `tx_sink = Some(WhepSink::new(...))` (legacy assignment) | `lib.rs:943-950` — already removed by [Step 2](./MVP-PHASE-6-STEP-2-capturestarted-rewrite.md) |
| `tx_sink.as_ref().unwrap()` | `lib.rs:767-771` — already replaced by [Step 3](./MVP-PHASE-6-STEP-3-signaller-started-helper.md) |
| `tx_sink.take()` | `lib.rs:702-706` — already cfg'd by [Step 4](./MVP-PHASE-6-STEP-4-stop-cast-rewrite.md) |
| `use mcore::transmission::WhepSink;` (top of file) | `lib.rs:30` (approx) |

### 1.2 Why this step is mostly compile-driven

After Steps 2 + 3 + 4, the only Android references to `tx_sink` are
the declaration and initialiser. Once those are gated, the compiler
catches anything we missed via `error[E0609]: no field tx_sink on
type Self`.

Run `cargo +nightly check -p fcast-sender-android --target
aarch64-linux-android` between each cfg change — if it stays green,
the change is complete.

### 1.3 Why we don't `pub(crate)` the new gates

The `tx_sink` field is private. Its `cfg` gate stays at the field
declaration. No re-exports change.

---

## 2. The change

### 2.1 Gate the struct field

**File:** `senders/android/src/lib.rs` (around line 537):

```rust
struct EventLoopState {
    // …existing fields…

    // Before:
    tx_sink: Option<WhepSink>,

    // After:
    #[cfg(not(target_os = "android"))]
    tx_sink: Option<mcore::transmission::WhepSink>,

    // …existing fields…
}
```

### 2.2 Gate the initialiser

**File:** `senders/android/src/lib.rs` (around line 602):

```rust
EventLoopState {
    // …existing field initialisers…

    // Before:
    tx_sink: None,

    // After:
    #[cfg(not(target_os = "android"))]
    tx_sink: None,

    // …existing field initialisers…
}
```

### 2.3 Gate the import (optional but tidy)

If `use mcore::transmission::WhepSink;` is at file scope, it's
unused on Android. Either:

- (a) Wrap it: `#[cfg(not(target_os = "android"))] use
  mcore::transmission::WhepSink;`
- (b) Inline the full path at the field declaration (as shown in
  §2.1) and remove the top-of-file `use` entirely.

(b) is slightly cleaner because the import disappears from
Android-side rust-analyser hints.

### 2.4 Walk every remaining read

Run:

```bash
grep -nE 'self\.tx_sink' senders/android/src/lib.rs
```

Expected output **before** this step:

```
602:    tx_sink: None,
767:    let (content_type, url) = self.tx_sink.as_ref().unwrap().get_play_msg(...)
702:    if let Some(mut tx_sink) = self.tx_sink.take() { tx_sink.shutdown(); }
943:    self.tx_sink = Some(WhepSink::new(...))?;
```

Expected output **after** Steps 2-5 are all in:

```
602:    #[cfg(not(target_os = "android"))]
602:    tx_sink: None,
702:    #[cfg(not(target_os = "android"))]
702:    if let Some(mut tx_sink) = self.tx_sink.take() { tx_sink.shutdown(); }
```

`lib.rs:767` and `lib.rs:943` should be **gone** entirely (replaced
by Step 2/3's graph commands and free-function call). If `grep`
still finds a `self.tx_sink` read outside a `#[cfg(not(target_os =
"android"))]` block, you missed it — wrap it.

---

## 3. Verification

### 3.1 Compile check (both targets)

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
cargo +nightly check -p fcast-sender-desktop
```

Both clean. The Android target no longer has `tx_sink` anywhere;
the desktop target retains the legacy path.

### 3.2 Symbol check (optional, sanity)

```bash
cargo +nightly rustc -p fcast-sender-android --target aarch64-linux-android -- --emit=metadata
nm target/aarch64-linux-android/debug/deps/libfcast_sender_android-*.rmeta | \
    grep -i 'WhepSink' || echo "no WhepSink references (expected)"
```

A clean Android build genuinely has no `WhepSink` symbols.

### 3.3 Grep

```bash
grep -nE 'tx_sink' senders/android/src/lib.rs
# → exactly 2 matches (the declaration and the stop_cast use),
#   both inside #[cfg(not(target_os = "android"))] blocks.
```

---

## 4. Pitfalls specific to this step

### P1 — `WhepSink` import gated incorrectly

If you gate the `use mcore::transmission::WhepSink;` import but not
the field declaration, the compiler will fail with `cannot find
type WhepSink in scope` on desktop. Either gate **both** to
`#[cfg(not(target_os = "android"))]`, or inline the full path
(§2.3 option b).

### P2 — `Option<WhepSink>` on desktop becomes `dead_code`

On desktop, `self.tx_sink` is still read in `stop_cast` (after
Step 4 cfg'd that block to non-Android). So it's not dead — but the
compiler may warn about `Some(WhepSink::new(...))` not being
assigned anywhere if `Event::CaptureStarted` on desktop never
runs that code path (because Android-only). Check the desktop
`Event::CaptureStarted` handler; if it doesn't construct
`WhepSink::new(...)`, the desktop binary's behaviour is unchanged
because the legacy cast loop has been gone for a while there too.
**This phase is Android-focused** — don't deepen the desktop
investigation here.

### P3 — `unused field tx_sink` warning on desktop

If desktop already doesn't use `tx_sink`, you'll see this warning.
Don't suppress with `#[allow(dead_code)]` — instead, look at
whether the desktop cast loop should be **removed** in a separate
follow-up PR. PHASE-6 is Android-only; desktop refactor is
out of scope.

### P4 — Missing cfg gate on a remaining read

Symptoms: `error[E0609]: no field tx_sink on type Self`. Solution:
go back to the cited `self.tx_sink` location, wrap it in a
`#[cfg(not(target_os = "android"))]` block, or — better — *delete*
it entirely (if it was only relevant to the Android cast loop).

### P5 — `Sync` bounds on `WhepSink`

`WhepSink: !Sync` (it holds a `gst::Pipeline`). Stripping it from
the Android `EventLoopState` may *remove* a `!Sync` constraint that
other code accidentally relied on. Cross-check with `cargo
+nightly check --all-features` to flush any auto-traits that
shifted.

### P6 — `unused import: mcore::transmission::WhepSink`

If you forget to cfg the top-of-file `use`, you'll see this warning
on Android. It's harmless but ugly. Adopt §2.3 option (b) to
eliminate it.

---

## 5. Next step

Once this lands, [Step 6](./MVP-PHASE-6-STEP-6-frame-pair-unchanged.md)
documents — but does **not** modify — the
`FRAME_PAIR` / `nativeProcessFrame` producer-side. It's a
documentation-only checkpoint that calls out *why* the JNI side
is untouched.

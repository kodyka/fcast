# Phase 8 — Section 8: Common pitfalls

> Section 8 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-7-verification.md`](./PHASE-8-Section-7-verification.md) first.

These are the recurring traps you'll hit during execution. Each links to a specific risk in the migration plan's **R**egister ([`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) Section 10) and includes a one-line preventive grep.

| # | Pitfall | Risk ID | Preventive grep |
|---|---|---|---|
| 8.1 | Forgetting to remove the `mock-*` initialiser | R4 | `grep -n 'mock-' senders/android/ui/pages/<migrated-page>.slint` |
| 8.2 | Reactivity loop via `<=>` two-way binding | R1 | `grep -nE 'mock-[a-z-]+ +<=> +Bridge' senders/android/ui/` |
| 8.3 | `upgrade_in_event_loop` panics if UI is gone | R2 | `grep -n 'upgrade_in_event_loop' senders/android/src/lib.rs \| grep -v 'let _ ='` |
| 8.4 | `VecModel::from(...)` allocates a fresh model on every push | R3 | (inspect by hand) |
| 8.5 | `Bridge.active-panel` race conditions | R5 | `grep -n 'set_active_panel' senders/android/src/lib.rs \| grep -v open_panel` |
| 8.6 | Slint enum naming: `-` vs `_` vs PascalCase | — | (inspect compile errors) |
| 8.7 | Stub initialiser flicker for a Rust-pushed property | R4 | (inspect by hand) |
| 8.8 | In-place struct-field mutation on a list element | — | `grep -nE '\\[[0-9]+\\]\\.[a-z-]+ *=' senders/android/ui/` |
| 8.9 | Holding a `Mutex` lock across `await` | R2 | `grep -n 'await' senders/android/src/lib.rs` (manual inspection of surrounding code) |
| 8.10 | Forgetting to push the **whole** list after a single-row mutation | — | (manual code review) |
| 8.11 | Mismatched `RecordingTickerState` between handlers and ticker | — | (manual code review) |
| 8.12 | Slint `pure` requirement for functions called from bindings | — | (compile error surfaces it) |
| 8.13 | Reentrant tracing deadlock inside `LogRing` (Cluster A5) | — | (manual code review; filter own target) |
| 8.14 | Assuming Slint auto-generates `on_<prop>_changed` callbacks | — | `grep -nE 'on_[a-z_]+_changed' senders/android/src/lib.rs` — every match must have a matching `changed` handler in `bridge.slint` |

---

## 8.1 — Forgetting to remove the `mock-*` initialiser (R4)

**Symptom:** the page renders the Rust-pushed value briefly, then visibly snaps back to the page-local default for one frame, then re-renders.

**Cause:** if you add `Bridge.x` and the page binding `for item in Bridge.x:` but **leave** `in-out property <T> mock-x: <stub>;` in the page **and** keep a stray reference like `for item in mock-x:`, Slint silently shadows the binding with the stub on first frame.

**Fix:** always remove the stub initialiser in the same commit as the wiring:

```sh
# After every cluster migration:
grep -n 'mock-' senders/android/ui/pages/<migrated-page>.slint
# Should be 0.
```

**Better fix:** use the `for x in Bridge.<list>:` form everywhere and grep for `for x in root.mock-`:

```sh
grep -rnE 'for [a-zA-Z_]+ in root\.mock-' senders/android/ui/
# Expected: 0 matches after Cluster D.
```

---

## 8.2 — Reactivity loop via `<=>` two-way binding (R1)

**Symptom:** infinite re-render, CPU pegged, "binding loop detected" warning in `slint-viewer` console.

**Cause:** if both Slint and Rust write to the same `in-out` property, you can get an oscillation. Especially common when a page does:

```slint
in-out property <int> page-source-idx <=> Bridge.audio-source-idx;
// …
clicked => { root.page-source-idx = (root.page-source-idx + 1); }
```

The `<=>` operator collapses the two properties into one identity at compile time — so the assignment goes back through Bridge, which propagates to the page, which… etc.

**Phase 8 discipline:**

- Use `in property <T>` for Rust-pushed values (Slint reads, Rust writes via `set_<x>`). Examples: `presets`, `history`, `log-entries`, `network-interfaces`.
- Use `in-out property <T>` only when Slint *also* writes (cycler increments, slider drags, text fields). Examples: `audio-source-idx`, `audio-input-gain`, `selected-receiver-id`.
- **Never** use `<=>` to bind a `Bridge` property to a page-local property of the same name — collapses to a single property at compile time, and Rust's `set_<x>` writes also propagate up to the page, defeating the abstraction.

**Detect:**

```sh
grep -rnE 'mock-[a-z-]+ +<=> +Bridge' senders/android/ui/
# Expected: 0 matches.
```

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — section "Two-way bindings" warns about this.

---

## 8.3 — `upgrade_in_event_loop` panics if UI is gone (R2)

**Symptom:** rare crash log on app background → reopen → some destructive action: `EventLoopError::EventLoopTerminated`.

**Cause:** when the user backgrounds the app and Rust still has work to do (a tokio task in flight pushing to the UI), the upgrade fails. Calling `.unwrap()` panics.

**Fix:** always handle the `Result`:

```rust
let _ = ui_weak.upgrade_in_event_loop(|ui| { /* … */ });
```

Discarding with `let _ = ...` is the correct fire-and-forget shape.

**Detect:**

```sh
# Find any upgrade_in_event_loop call that ISN'T discarded with `let _ =`.
grep -n 'upgrade_in_event_loop' senders/android/src/lib.rs | grep -v 'let _ ='
# Expected: 0 matches.
```

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/guide/backends-and-renderers/backends_and_renderers.mdx`. The Slint Rust API docs at `https://slint.dev/releases/latest/docs/rust/slint/struct.Weak.html#method.upgrade_in_event_loop` are the authoritative reference for `upgrade_in_event_loop`'s `Result` return.

---

## 8.4 — `VecModel::from(...)` allocates a fresh model on every push (R3)

**Symptom:** for high-volume lists (log entries at high tracing volume), GC churn / dropped frames.

**Cause:** every `set_X(Rc::new(VecModel::from(snapshot)).into())` call allocates a fresh `Rc<VecModel<T>>`. For 4-row presets this is invisible. For 1024-row log entries pushed every 10 ms, it's measurable.

**Fix for low-volume lists:** the simple `Rc::new(VecModel::from(snapshot)).into()` push pattern is fine. Do nothing.

**Fix for high-volume lists:** reuse the `Rc<VecModel<T>>` and call `model.set_vec(snapshot)` to mutate in place:

```rust
// Instead of allocating a new VecModel on every push:
let model_rc: Rc<slint::VecModel<LogEntry>> = Rc::new(VecModel::from(Vec::<LogEntry>::new()));
ui.global::<Bridge>().set_log_entries(model_rc.clone().into());

// Then on each push:
model_rc.set_vec(snapshot);
// No reassignment needed — Slint observes the model's internal change signal.
```

**Phase 8 trigger:** apply this to `Bridge.log-entries` if tracing volume exceeds ~50 entries / second. For everything else, the simple pattern is fine.

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx` — the Rust tab covers `VecModel::set_vec`.

---

## 8.5 — `Bridge.active-panel` race conditions (R5)

**Symptom:** user taps a panel-opening button on the bar, the panel briefly opens, then immediately closes (or vice versa).

**Cause:** Slint *and* Rust both write to `Bridge.active-panel` in the same tick. The visible panel may not match what either side expects.

**Fix:** Cluster E's invariant — "Slint writes via clicks; Rust writes via a single `open_panel(p)` dispatcher" — keeps this safe. See [Section 6.2](./PHASE-8-Section-6-cluster-E-overlay-invariants.md#62--invariant-a-bridgeactive-panel-has-a-single-rust-dispatcher).

**Detect:**

```sh
# Check that Rust writes set_active_panel only inside the open_panel dispatcher.
grep -n 'set_active_panel' senders/android/src/lib.rs | grep -v 'fn open_panel'
# Expected: 0 matches outside open_panel.
```

---

## 8.6 — Slint enum naming (`-` vs `_` vs PascalCase)

**Symptom:** Rust compile error: `cannot find variant 'lock-screen' in enum LifecycleMode`.

**Cause:** Slint enums use kebab-case in `.slint` files (`Panel.cast-history`, `RecordingState.recording`, `LifecycleMode.lock-screen`). The Rust generated bindings use PascalCase (`Panel::CastHistory`, `RecordingState::Recording`, `LifecycleMode::LockScreen`).

**Fix:** use the Rust naming on the Rust side. The compiler tells you which one to use — don't fight it.

**Cheat sheet for this codebase:**

| Slint identifier | Rust identifier |
|---|---|
| `Panel.audio` | `Panel::Audio` |
| `Panel.bitrate-presets` | `Panel::BitratePresets` |
| `Panel.cast-history-detail` | `Panel::CastHistoryDetail` |
| `RecordingState.idle` | `RecordingState::Idle` |
| `RecordingState.recording` | `RecordingState::Recording` |
| `RecordingState.paused` | `RecordingState::Paused` |
| `RecordingState.finalizing` | `RecordingState::Finalizing` |
| `LifecycleMode.normal` | `LifecycleMode::Normal` |
| `LifecycleMode.lock-screen` | `LifecycleMode::LockScreen` |
| `LifecycleMode.stealth` | `LifecycleMode::Stealth` |
| `LifecycleMode.snapshot-countdown` | `LifecycleMode::SnapshotCountdown` |
| `BannerSeverity.info` | `BannerSeverity::Info` |
| `BannerSeverity.success` | `BannerSeverity::Success` |
| `BannerSeverity.warning` | `BannerSeverity::Warning` |
| `BannerSeverity.error` | `BannerSeverity::Error` |
| `LogLevel.trace` | `LogLevel::Trace` |
| `LogLevel.debug` | `LogLevel::Debug` |
| `LogLevel.info` | `LogLevel::Info` |
| `LogLevel.warning` | `LogLevel::Warning` |
| `LogLevel.error` | `LogLevel::Error` |
| `StatusSeverity.info` | `StatusSeverity::Info` |
| `StatusSeverity.warning` | `StatusSeverity::Warning` |
| `StatusSeverity.error` | `StatusSeverity::Error` |

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — naming conventions.

---

## 8.7 — Stub initialiser flicker for a Rust-pushed property (R4)

**Symptom:** the page renders empty for one frame on launch, then the Rust-pushed values appear, causing a visible "pop" / layout shift.

**Cause:** the `: []` initialiser is the value Slint sees **before** Rust's first `set_X` call. If your page's `for x in Bridge.X:` paints something jarring on the empty list (e.g. an empty status badge row that takes 28 px of vertical space), the layout will flicker on the next push.

**Fix options:**

1. **Make the empty state visually invisible.** A status badge row that is `0px tall` when empty doesn't flicker. Status badges via `for item in Bridge.status-items: Badge { … }` is naturally empty-friendly (the HorizontalLayout just shrinks to 0 width).

2. **Provide a non-empty stub.** If you must have placeholder content (e.g. a "Loading…" text), declare `Bridge.<x>: [<stub>];` with a single placeholder entry that's safe to render. **But** then make sure Rust's first push happens within ~50 ms of UI creation so the placeholder is replaced fast enough that the user doesn't notice.

3. **Provide an `is-loading: bool`.** Rust pushes `is-loading: true` initially and `is-loading: false` once the real data lands. Slint shows a spinner while loading, the list once loaded. Slightly more boilerplate but visually nicer.

**Phase 8 default:** option 1 wherever possible. Option 2 only for `Bridge.app-version` (a non-empty stub like `"…"` is fine because Rust pushes within ~10 ms of init).

---

## 8.8 — In-place struct-field mutation on a list element (Slint reactivity caveat)

**Symptom:** `presets[2].active = true;` runs without error, but the UI doesn't update.

**Cause:** Slint's reactivity tracks **whole-array reassignment**, not field-level writes. In-place mutations of struct fields inside an array element are not observed.

**Fix:** rebuild the entire array, or (post-Phase-8) move the mutation Rust-side via a callback. After Phase 8, the canonical pattern is "user taps row → callback → Rust mutates Vec → push back".

**Detect:**

```sh
grep -rnE '\[[0-9]+\]\.[a-z-]+ *=' senders/android/ui/
# Should be 0 matches after Cluster C.
```

**Reference:** Phase-16 reimplement guide gotcha 28 ("In-place struct field mutation on a list element"). Phase 8 retires this trap by moving every list mutation Rust-side.

---

## 8.9 — Holding a `Mutex` lock across `await` (R2 corollary)

**Symptom:** deadlock or panic ("cannot block waiting on `await` while holding a Mutex lock").

**Cause:** code like:

```rust
let mut guard = state.lock().unwrap();
guard.frobnicate();
some_async_thing().await;        // ← BAD: lock held across await
guard.something_else();
```

If `some_async_thing()` takes any meaningful time, you've blocked every other task that needs `state`. If you're using `tokio::sync::Mutex` instead of `std::sync::Mutex`, it works but starves other tasks.

**Fix:** drop the lock before the `await`:

```rust
let mut guard = state.lock().unwrap();
guard.frobnicate();
drop(guard);                     // ← release before async work
some_async_thing().await;

let mut guard = state.lock().unwrap();
guard.something_else();
drop(guard);
```

Or factor the async work into a separate function that takes a snapshot and pushes a result back:

```rust
let snapshot = state.lock().unwrap().clone();
let result = some_async_thing(snapshot).await;
let mut guard = state.lock().unwrap();
guard.apply(result);
```

The Cluster C handlers in Section 4 follow this discipline — every callback pulls a snapshot via `Mutex::lock`, mutates inside the closure, drops the guard with `drop(g);`, then calls `push();` (which is sync and just `upgrade_in_event_loop`s a clone).

---

## 8.10 — Forgetting to push the **whole** list after a single-row mutation

**Symptom:** the user toggles one row's enabled flag, the row visually toggles, but the list count doesn't change. Then they reload the panel — and the change is gone.

**Cause:** the Rust handler mutated `Vec<T>` in-place but didn't call `push();` afterwards. Slint kept showing the previously-pushed model.

**Fix:** every callback that mutates `Arc<Mutex<Vec<T>>>` MUST end with `push();` — i.e. push the rebuilt VecModel back. The Cluster C / D handlers in Sections 4-5 follow this convention strictly:

```rust
ui.global::<Bridge>().on_set_bar_action_enabled({
    let bar_actions = bar_actions.clone();
    let push        = push_bar.clone();
    move |idx, enabled| {
        let mut g = bar_actions.lock().unwrap();
        if let Ok(i) = usize::try_from(idx) {
            if let Some(a) = g.get_mut(i) { a.enabled = enabled; }
        }
        drop(g);
        push();      // ← REQUIRED
    }
});
```

**Detect:** code review only. There is no automated grep for this.

---

## 8.11 — Mismatched `RecordingTickerState` between handlers and ticker

**Symptom:** the user taps Record, state goes Recording, but elapsed counter stays at 0.

**Cause:** Cluster A4's `spawn_recording_ticker` and Cluster B3's `on_start_recording` both refer to a `recorder_state: Arc<Mutex<RecordingTickerState>>`. If you initialise A4 with `Default::default()` and B3 with a *different* `Arc::new(Mutex::new(Default::default()))`, the ticker reads from a different lock than the handlers mutate.

**Fix:** the `recorder_state` value declared in Cluster B3 (Section 3.3) should be passed to `spawn_recording_ticker`:

```rust
// Earlier in init_ui:
let recorder_state = Arc::new(Mutex::new(RecordingTickerState::default()));

// Cluster B3's handlers all clone recorder_state.
// Cluster A4's ticker takes the SAME Arc:
spawn_recording_ticker(ui.as_weak(), recorder_state.clone());
```

If you ship A4 and B3 in separate PRs, fix this in B3's PR — A4 alone with a `Default::default()` ticker is OK (it just reads idle) until B3 lands.

---

## 8.12 — Slint `pure` requirement for functions called from bindings

**Symptom:** `error: side-effect free function 'foo' called from a property binding`.

**Cause:** Slint requires functions called from `property <X>: foo(...)` bindings to be marked `pure`. Helper functions like `preset-by-id(...)` from Section 4.1 must be declared:

```slint
pure function preset-by-id(list: [BitratePreset], id: string) -> BitratePreset {
    for p in list { if p.id == id { return p; } }
    return { id: "", name: "", bitrate-kbps: 0, active: false };
}
```

Without `pure`, binding-context calls fail at compile time. With `pure`, the function may not have side effects (no callbacks, no property writes).

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.

---

## 8.13 — Reentrant tracing deadlock inside `LogRing` (Cluster A5)

**Symptom:** the app freezes the first time the debug log subscriber processes
an event whose handler — directly or transitively — emits another `tracing`
event. Stacks show one thread parked on `Mutex::lock` inside `LogRing::on_event`
with the same thread already owning the mutex one frame up.

**Cause:** `std::sync::Mutex` is **not reentrant**. The Cluster A5 sketch
([Section 2.5](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#25--a5--debug-log-entries))
locks `self.entries` inside `Layer::on_event`. If anything between
`q.lock().unwrap()` and `drop(q);` emits a `tracing` event — an instrumented
allocator, a panic-on-overflow path, a `tracing::trace!` added inside
`LogEventVisitor`, even a future inside `chrono::Local::now().format(...)` — the
subscriber re-enters `on_event` on the same thread and self-deadlocks. Phase
8.9 covered "lock across `await`"; this is the orthogonal "lock across
re-entry" trap.

**Fix (pick one):**

1. **Filter the subscriber so it ignores its own target.** Cheapest and
   robust — events emitted from inside the ring are dropped before the
   subscriber tries to lock anything:
   ```rust
   fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
       if event.metadata().target().starts_with("fcast::log_ring") { return; }
       // … rest of body …
   }
   ```
   Combine with a `tracing_subscriber::filter::Targets` rule in `init_ui` to
   exclude the same target from the global registry as well.

2. **Use `try_lock` and silently drop on contention.** Losing a reentrant event
   is preferable to a deadlock for a debug-only ring:
   ```rust
   let Ok(mut q) = self.entries.try_lock() else { return; };
   ```

3. **Use `parking_lot::ReentrantMutex`.** Last resort. Adds a dependency and
   papers over the bug rather than fixing it; future refactors may still hit
   subtler ordering issues.

**Detect:** code review only. There's no clean grep for "tracing event emitted
under a lock" — the event source is often transitive. Run a stress test under
`tracing_subscriber::filter::LevelFilter::TRACE` with a tracing-instrumented
allocator (e.g. `tracing-allocations`) and watch for hangs.

**Slint doc reference:** none — this is a Rust / `tracing` interaction. The
authoritative reference is the [`tracing-subscriber` Layer
docs](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html)
which note that `on_event` may be called recursively.

---

## 8.14 — Assuming Slint auto-generates `on_<prop>_changed` callbacks

**Symptom:** Rust compile error like `no method named on_selected_history_id_changed found for global 'Bridge'`. Or — worse — the code compiles because of an unrelated handler with a similar name, and the change observer silently never fires.

**Cause:** Slint's Rust binding generator emits `set_<prop>` / `get_<prop>` for properties and `on_<callback>` for callbacks, but **does not** synthesize `on_<prop>_changed` callbacks for `in` / `in-out` properties. There is no automatic property-change observer in the public API. (Internal `PropertyTracker` / `ChangeTracker` types exist but aren't part of the generated global surface.)

**Fix.** Declare an explicit callback in the global and wire it from a `changed` handler:

```slint
export global Bridge {
    in-out property <string> selected-history-id: "";
    callback selected-history-id-changed(string);
    changed selected-history-id => {
        Bridge.selected-history-id-changed(Bridge.selected-history-id);
    }
}
```

Then on the Rust side, bind to the explicit callback:

```rust
ui.global::<Bridge>().on_selected_history_id_changed(|id: slint::SharedString| {
    // … react to the new value …
});
```

This is the canonical pattern; Cluster D2's `selected-history-entry` plumbing in [Section 5.2 Step 1](./PHASE-8-Section-5-cluster-D-destructive-flows.md#step-1-extend-bridgeslint-1) uses it.

**Detect:**

```sh
# Every on_<x>_changed binding in lib.rs must have a matching `changed <x>`
# handler in bridge.slint that re-emits the corresponding callback.
grep -nE 'on_[a-z_]+_changed' senders/android/src/lib.rs
grep -nE 'changed [a-z-]+ =>' senders/android/ui/bridge.slint
# Pair them up by hand. A name in lib.rs without a counterpart in bridge.slint
# is a bug.
```

**Slint doc reference:** `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — the `changed` handler section.

---

## Exit criteria for Section 8

This section is a reference, not a checklist. Treat it as a debugging aid when something goes wrong during execution. The corresponding fixes are spread across Sections 1-6 — each pitfall above links back to where the canonical pattern is documented.

You can now move to **Section 9 — stop conditions** at [`PHASE-8-Section-9-stop-conditions.md`](./PHASE-8-Section-9-stop-conditions.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/backends-and-renderers/backends_and_renderers.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx`

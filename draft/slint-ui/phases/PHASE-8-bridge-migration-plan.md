# Phase 8 — Rust Bridge Reactivation Migration Plan

**Audience:** developer reactivating the deferred [`PHASE-8-rust-bridge.md`][spec] phase, after some subset of UI Phases 5/6/7 + 12–27 has shipped to `master`.
**Goal:** wire every UI-only stub property declared by the prior phases to a real Rust producer/consumer in `senders/android/src/lib.rs`. Promote page-local `mock-*` properties to `Bridge.*` properties; replace `mock-*` initialisers with Rust-driven setters; promote `clicked => { ... }` Slint-side handlers that should round-trip through Rust to `Bridge.<callback>(...)` invocations.
**Scope:** mostly Rust changes, with surgical Slint edits to swap bindings. Each consumer phase migrates **independently** — the order here is a recommendation, not a hard sequence.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-8-rust-bridge.md

> **Read every shipped UI-phase reimplement guide first.** This plan is the consolidated index of their "When Phase 8 reactivates" sections. The guides are the source of truth for what a phase's stub model contains; this plan is the index of what to wire.

---

## Why this guide exists

The UI-only roadmap was deliberately permissive: every page declares its own `in-out property <T> mock-*: <stub>` and exposes the right shape without touching Rust. That worked beautifully for design iteration. But it leaves a substantial wiring backlog. This document:

1. **Lists every deferred wiring point** ("Bridge promotions" — the `mock-*` → `Bridge.*` moves).
2. **Lists every callback that should round-trip Rust** ("Callback promotions" — the Slint-side direct mutations that should become Rust-handled).
3. **Sequences migrations** by dependency + risk.
4. **Documents what's already wired** so you don't accidentally re-wire it.

Every checklist item below cites the originating UI-phase guide and the file/property/struct in question.

---

## Section 0 — What's already wired (do not touch)

These bindings predate the UI-only roadmap and are already plumbed end-to-end in `senders/android/src/lib.rs`. Leave them alone:

| Bridge property / callback | Producer / handler in lib.rs |
|---|---|
| `Bridge.devices: [string]` | `set_devices` from mDNS discovery |
| `Bridge.app-state: AppState` | `invoke_change_state(...)` — driven by connect/cast lifecycle |
| `Bridge.show-debug: bool` | `set_show_debug` — driven by Phase 4 debug toggle (still wired) |
| `Bridge.test-status: string` | `set_test_status` — driven by codec test runner |
| `Bridge.quick-actions: [QuickAction]` | `set_quick_actions` — populated at startup |
| `Bridge.connect-receiver(string)` | `on_connect_receiver` |
| `Bridge.start-casting(scale-w, scale-h, max-fps)` | `on_start_casting` |
| `Bridge.stop-casting()` | `on_stop_casting` |
| `Bridge.invoke-action(string)` | `on_invoke_action` (already routes non-panel ids to Rust) |
| `Bridge.change-state(AppState)` | Slint-side public function (state machine — Rust calls into it) |

Everything else introduced by the UI phases is **deferred** and lives below.

---

## Section 1 — Migration sequence

The migrations cluster naturally by data shape. Recommended order (each cluster ~ 1 small PR):

1. **Cluster A — read-only view models** (lowest risk; no callback round-trips).
2. **Cluster B — single-page state with one or two callbacks** (audio / camera / recording).
3. **Cluster C — list pages with mutations** (bitrate presets / quick actions / macros / network).
4. **Cluster D — destructive flows** (backup-reset / cast history / debug log clear).
5. **Cluster E — overlay state** (lifecycle modes / panels — already wired but document the contract).
6. **Cluster F — promote shared utils** (Theme tokens, IconAndText raster icons).

Each cluster's items below are **independent** — split clusters into PRs by item if a subset is enough for the user-visible win.

---

## Section 2 — Cluster A: read-only view models

### A1. Phase 13 status overlay → `Bridge.status-items: [StatusItem]`

**From:** `PHASE-13-*-reimplement-instructions.md` and the `mock-status-items` initialiser on `StatusOverlay`.

```rust
// In lib.rs, alongside the existing set_devices producer:
ui.global::<Bridge>().set_status_items(
    std::rc::Rc::new(slint::VecModel::from(vec![
        // produced by encoder pipeline metrics
    ])).into(),
);
```

**Slint side:**

```diff
-    in-out property <[StatusItem]> mock-status-items: [
-        ...stub entries...
-    ];
+    in property <[StatusItem]> status-items <=> Bridge.status-items;
```

Then bind the page's `for item in root.status-items:` directly.

### A2. Phase 21 about / version-history / attributions → `Bridge.app-version: string`, `Bridge.version-history: [...]`, `Bridge.attributions: [...]`

**From:** `PHASE-21-reimplement-instructions.md`.

`app-version` already noted as deferred in [`PHASE-8-rust-bridge.md`][spec]. Wire from Cargo's `env!("CARGO_PKG_VERSION")`:

```rust
ui.global::<Bridge>().set_app_version(env!("CARGO_PKG_VERSION").into());
```

Version history and attributions can stay inline (these are static across builds, not user-facing data — Slint stub initialisers are fine). Promote only if you want translation infrastructure to reach them via `@tr`.

### A3. Phase 22 network interfaces → `Bridge.network-interfaces: [NetworkInterface]`

**From:** `PHASE-22-reimplement-instructions.md`.

```rust
ui.global::<Bridge>().set_network_interfaces(
    enumerate_interfaces()
        .into_iter()
        .map(|iface| NetworkInterface {
            name: iface.name.into(),
            ipv4: iface.ipv4.into(),
            ipv6: iface.ipv6.into(),
            kind: iface.kind.into(),
            connected: iface.is_connected,
        })
        .collect::<slint::VecModel<_>>()
        .into(),
);
```

The page's per-row tap-to-expand state (`expanded: bool` inside `NetworkInterfaceRow`) stays Slint-side — it's pure UI, not data.

### A4. Phase 23 recording elapsed counter → `Bridge.recording-state: RecordingState`, `Bridge.recording-elapsed-s: int`

**From:** `PHASE-23-reimplement-instructions.md`.

The page-local `mock-state` and `mock-elapsed-s` move to Bridge. Rust drives both — when a recording starts, Rust starts an interval-spawned task that increments `recording-elapsed-s` every second and pushes via `upgrade_in_event_loop`. The Slint Timer that ticks `mock-elapsed-s` is removed.

```rust
// On recording start:
let ui_weak = self.ui_weak.clone();
tokio::spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            let cur = ui.global::<Bridge>().get_recording_elapsed_s();
            ui.global::<Bridge>().set_recording_elapsed_s(cur + 1);
        });
        // Break when state transitions to idle/finalizing.
    }
});
```

### A5. Phase 26 debug log entries → `Bridge.log-entries: [LogEntry]`

**From:** `PHASE-26-reimplement-instructions.md`.

`tracing` subscriber appends entries to a bounded ring buffer; on each append, Rust pushes the truncated buffer:

```rust
ui.global::<Bridge>().set_log_entries(
    std::rc::Rc::new(slint::VecModel::from(ring_buffer.snapshot()))
        .into(),
);
```

The level-filter chip stays Slint-side (`mock-min-level-idx` remains a page-local property).

---

## Section 3 — Cluster B: single-page state with one or two callbacks

### B1. Phase 14 audio settings → `Bridge.audio-source-idx`, `Bridge.audio-muted`, `Bridge.audio-input-gain`, `Bridge.audio-bitrate-idx`

**From:** `PHASE-14-reimplement-instructions.md`.

Promote each `mock-*` to a Bridge property. **Slint-side direct mutations** (cycler `clicked => { Math.mod(idx + 1, N); }`, slider `changed(v) => idx = v;`) **stay Slint-side** — these are stateful UI knobs that don't need Rust round-trip until the Rust pipeline consumes them. Rust *reads* the current values when starting a cast.

If you want Rust to react to changes (e.g. live-update a running cast), expose callbacks:

```rust
ui.global::<Bridge>().on_audio_source_changed({
    let pipeline = self.pipeline.clone();
    move |idx| {
        let _ = pipeline.set_audio_source(idx as usize);
    }
});
```

### B2. Phase 15 camera settings → analogous to B1

**From:** `PHASE-15-reimplement-instructions.md`. Same pattern, more properties (source / resolution / framerate / mirror / stabilization / tap-to-focus / zoom).

### B3. Phase 23 recording controls → `Bridge.start-recording()`, `Bridge.pause-recording()`, `Bridge.stop-recording()`

**From:** `PHASE-23-reimplement-instructions.md`. The state-machine button's `clicked => { mock-state = ... }` becomes `clicked => { Bridge.start-recording(); }` etc. Rust transitions `recording-state` based on real progress.

### B4. Phase 18 lifecycle modes → `Bridge.engage-lock()`, `Bridge.engage-stealth()`, `Bridge.start-snapshot-countdown(int)`

**From:** `PHASE-18-reimplement-instructions.md`. The settings PRIVACY rows' `clicked => { Bridge.lifecycle = ...; }` becomes `clicked => { Bridge.engage-lock(); }` etc. Rust calls Android `KeyguardManager` / sets `FLAG_SECURE` / starts a real countdown task.

### B5. Phase 22 Wi-Fi Aware toggle → `Bridge.set-wifi-aware(bool)`

**From:** `PHASE-22-reimplement-instructions.md`. The toggle's `toggled(v) => { mock-wifi-aware = v; banner-visible = true; }` becomes `toggled(v) => { Bridge.set-wifi-aware(v); }`. Rust runs the actual Wi-Fi Aware enablement and either confirms or pushes a banner-visible flag (consumes the new `Bridge.banner-message`/`banner-visible` from Phase 27's `InfoBanner` migration).

---

## Section 4 — Cluster C: list pages with mutations

The pattern across all of these: page-local `mock-list: [T]` becomes `Bridge.list: [T]`; Slint-side rebuild helpers (the hardcoded N-row swap pattern from Phase 16/17/22/25) become Rust-side callbacks that mutate and push back.

### C1. Phase 16 bitrate presets → `Bridge.presets`, `Bridge.save-preset(...)`, `Bridge.delete-preset(string)`, `Bridge.set-active-preset(string)`

**From:** `PHASE-16-reimplement-instructions.md`. Promote `mock-presets` to Bridge. Replace the hardcoded `set-active(id)` rebuild helper with `Bridge.set-active-preset(id)`. The edit page's Save handler calls `Bridge.save-preset(id, name, kbps)`.

### C2. Phase 17 quick-action customisation → `Bridge.bar-actions`, `Bridge.move-bar-action(int, int)`, `Bridge.set-bar-action-enabled(int, bool)`, `Bridge.save-bar-actions()`

**From:** `PHASE-17-reimplement-instructions.md`. Promote `mock-bar-actions` to Bridge. The hardcoded 5-row `swap` and `set-enabled` helpers go away.

**Important:** the live `CastControlBar`'s `mock-quick-actions` is currently a separate copy. Phase 8 unifies these — the bar reads from the same `Bridge.bar-actions`. The "Save" semantics in the customisation page call `Bridge.save-bar-actions()` to persist; without an explicit save, edits don't propagate to the bar.

### C3. Phase 22 Wi-Fi Aware (already covered by B5)

### C4. Phase 25 macros → `Bridge.macros`, `Bridge.save-macro(...)`, `Bridge.delete-macro(string)`, `Bridge.move-step(string, int, int)`, `Bridge.add-step(string, string)`, `Bridge.remove-step(string, int)`, `Bridge.run-macro(string)`

**From:** `PHASE-25-reimplement-instructions.md`. The largest callback surface in this plan. The macro execution engine itself is Rust-side; the UI just dispatches.

The control-bar tweak (`id.starts-with("macro:")` ▶ glyph) stays Slint-side — purely visual.

### C5. Phase 26 debug log filter / clear (already covered by A5; clear callback below)

```rust
ui.global::<Bridge>().on_clear_log_entries({
    let buffer = self.log_buffer.clone();
    move || { buffer.clear(); /* + push empty list */ }
});
```

---

## Section 5 — Cluster D: destructive flows

### D1. Phase 19 backup / reset → `Bridge.export-settings()`, `Bridge.import-settings()`, `Bridge.reset-settings()`, `Bridge.clear-cast-history()`, `Bridge.clear-known-receivers()`, plus Bridge banner

**From:** `PHASE-19-reimplement-instructions.md`.

```rust
ui.global::<Bridge>().on_export_settings({
    let ui_weak = self.ui_weak.clone();
    move || {
        // Launch ACTION_CREATE_DOCUMENT via JNI; on success:
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>()
                .set_banner_message("Exported settings to <path>".into());
            ui.global::<Bridge>().set_banner_visible(true);
        });
    }
});
```

The page's `pending-action: string` + `on-confirm()` helper stays Slint-side — the dispatch is pure UI flow control. The action's *effect* is the Rust callback.

### D2. Phase 20 cast history → `Bridge.history`, `Bridge.clear-history()`, `Bridge.delete-history-entry(string)`, `Bridge.recast(string)`, plus `Bridge.selected-history-entry: CastHistoryEntry` derived property

**From:** `PHASE-20-reimplement-instructions.md`.

The detail page's `find-entry(id)` lookup is replaced by Rust pushing a derived `selected-history-entry` whenever `selected-history-id` changes — eliminates the hardcoded N-row search.

```rust
ui.global::<Bridge>()
    .on_set_selected_history_id({
        let ui_weak = self.ui_weak.clone();
        let history = self.history.clone();
        move |id| {
            if let Some(entry) = history.find(&id) {
                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                    ui.global::<Bridge>().set_selected_history_entry(entry);
                });
            }
        }
    });
```

`recast(id)` resolves the receiver and starts a cast.

### D3. Phase 22 ADVANCED reset (no-op currently) — defer until a phase introduces a destructive ADVANCED row.

---

## Section 6 — Cluster E: overlay state

The `Panel` enum and `LifecycleMode` enum are already in Bridge; their orthogonal-overlay layering in `main.slint` is correct and needs no Rust change. Cluster E is mostly **document the invariants**:

- `Bridge.active-panel = Panel.<x>` is **always** mutated from Slint (a button click sets it). Rust **may** mutate it when an external trigger needs to surface a panel (e.g. Phase 8's "Connection lost" auto-routes to a status panel) — but currently no such trigger exists.
- `Bridge.lifecycle = LifecycleMode.<x>` is mutated from both sides: Slint-side from settings PRIVACY rows; Rust-side from real lock-engagement / inactivity / countdown completion.
- The conditional `if Bridge.active-panel == Panel.x:` chains in `main.slint` are read-only in Rust's view; do not introduce Rust-side panel routing — keep all `Panel.<x>` mutations in either Slint or in a single dispatcher function (`open_panel(p: Panel)`).

---

## Section 7 — Cluster F: shared utils Bridge integration

These come out of `PHASE-27-reimplement-instructions.md`:

### F1. `InfoBanner` → `Bridge.banner-message`, `Bridge.banner-visible`

```diff
 export global Bridge {
+    in property <string>     banner-message;
+    in-out property <bool>   banner-visible: false;
+    in property <BannerSeverity> banner-severity: BannerSeverity.info;
 }
```

The util binds to these Bridge properties when used in a top-level `MainWindow` overlay; per-page consumers continue to bind to local flags. This makes "show banner from anywhere" a one-liner from Rust.

### F2. `IconAndText` → bundled raster icon assets

`Theme.icon-*` properties resolve to `image` literals via Slint's asset embedding (`@image-url("../assets/icons/...png")`). Rust doesn't see these — they're Slint compile-time assets.

### F3. `Theme.success`, `Theme.warning`, `Theme.error` tokens

```diff
 export global Theme {
+    out property <color> success: #2e7d32;
+    out property <color> warning: #ed6c02;
+    out property <color> error:   #c62828;
 }
```

Replace inline severity hex in `info_banner.slint` and `cast_history_page.slint`'s `status-color` helper.

---

## Section 8 — Per-phase checklist (consolidated index)

Use this table to confirm you didn't miss a wiring point. Each row maps a UI phase to its Phase-8 work items. Rows in **bold** are user-visible; non-bold are internal.

| Phase | Bridge promotions (UI-only `mock-*` → `Bridge.*`) | Callback promotions (Slint mutation → Rust call) |
|---|---|---|
| 13 | `status-items` | — (read-only) |
| 14 | `audio-source-idx`, `audio-muted`, `audio-input-gain`, `audio-bitrate-idx` | optional `on_audio_*_changed` |
| 15 | analogous camera properties | optional `on_camera_*_changed` |
| 16 | `presets`, `selected-preset-id` | `save-preset`, `delete-preset`, `set-active-preset` |
| 17 | `bar-actions` (unify with `quick-actions`) | `move-bar-action`, `set-bar-action-enabled`, `save-bar-actions` |
| 18 | `lifecycle`, `mock-snapshot-secs` (now `snapshot-secs`) | `engage-lock`, `engage-stealth`, `start-snapshot-countdown(int)` |
| 19 | `pending-action` (stays Slint), `banner-*` (promote per F1) | `export-settings`, `import-settings`, `reset-settings`, `clear-cast-history`, `clear-known-receivers` |
| 20 | `history`, `selected-history-id`, `selected-history-entry` (derived) | `clear-history`, `delete-history-entry`, `recast` |
| 21 | `app-version` | — (mostly static content) |
| 22 | `network-interfaces`, `wifi-aware` | `set-wifi-aware` |
| 23 | `recording-state`, `recording-elapsed-s` | `start-recording`, `pause-recording`, `resume-recording`, `stop-recording` |
| 25 | `macros`, `mock-macro-edit-id` (rename to `macro-edit-id`) | `save-macro`, `delete-macro`, `move-step`, `add-step`, `remove-step`, `run-macro` |
| 26 | `log-entries` | `clear-log-entries` |
| 27 | (utils — see Section 7) | (utils — see Section 7) |

Phases 12, 24, 28+ aren't in this index because they didn't ship a UI-only reimplement guide.

---

## Section 9 — Migration patterns (cookbook)

### P1. Promoting `mock-*` to `Bridge.*` without breaking the page

Three steps per property:

1. **Add the Bridge property** in `bridge.slint`. Initial value can mirror the page's current stub.
2. **Change the page's binding** from `in-out property <T> mock-x: <stub>;` to `in property <T> x <=> Bridge.x;`. (Or `in property <T> x: Bridge.x;` for one-way reads.)
3. **Remove the stub initialiser** from the page. Replace consumers (`root.mock-x` → `root.x`).

Before you commit, set the Bridge property's initial value identically to the page's old stub so the visual state doesn't change. After Rust wiring lands, drop the stub from `bridge.slint` (Rust now owns the value).

### P2. Promoting a Slint-side mutation to a Rust callback

Three steps:

1. **Declare the callback** in `bridge.slint`: `callback <name>(<args>);`.
2. **Replace the Slint-side mutation** with the callback invocation: `clicked => { root.mock-x = newval; }` → `clicked => { Bridge.<name>(newval); }`.
3. **Wire the handler** in `lib.rs`: `ui.global::<Bridge>().on_<name>({ ... mutate Rust state, push back via set_<x> ... });`.

The Rust handler typically (a) updates Rust-owned state, (b) calls a side effect (file I/O, network, JNI), (c) pushes the new state back via `set_<x>` inside `upgrade_in_event_loop`.

### P3. Adding a derived property pushed by Rust

Slint can declare `in property <T> name;` — Rust holds the source of truth and calls `set_name(...)` whenever the underlying source changes. Used for `Bridge.selected-history-entry` (Phase 20) and `Bridge.recording-elapsed-s` (Phase 23). Avoid declaring it `in-out` unless the Slint side also writes — a leaky abstraction otherwise.

### P4. Two-way bindings between page-local and Bridge

If a page wants to bind a checkbox state both to its own model and to a Bridge property, declare both as `in-out` and use `<=>`. See Phase 27's `InfoBanner.visible <=> root.banner-visible` pattern.

### P5. Promoting Slint-only enums to Rust-visible enums

Slint enums become Rust enums via `slint_build`. Once the enum is in `bridge.slint`, Rust sees `enum BannerSeverity { Info, Success, Warning, Error }` (capitalised in Rust) and can construct values directly. No extra mapping layer needed.

---

## Section 10 — Risk register

### R1. Reactivity loops via `<=>` two-way bindings

If both Slint and Rust write to the same `in-out` property in the same tick, you get an oscillation. Rust should prefer one-way `set_<x>` to a Slint-`in` property; reserve `in-out` for cases where Slint's UI input drives the value (text fields, toggles, sliders). Audit all `<=>` bindings in `bridge.slint` after each migration.

### R2. `upgrade_in_event_loop` panics if the UI is gone

Always handle the `Result` returned by `ui_weak.upgrade_in_event_loop(...)`. Existing handlers in `lib.rs` use `let _ = ...` — fine for fire-and-forget; bad if the next operation depends on the closure executing. Don't change the discipline mid-migration.

### R3. `VecModel::from(...)` allocates a fresh model on every push

For `log-entries` (Phase 26) at high tracing volume, allocating + pushing on every event saturates the event loop. Use `slint::ModelRc<T>` with a custom `Model` impl backed by the ring buffer; push only on flush boundaries.

### R4. Slint stub initialisers persist across `set_<x>` calls

If you forget to remove a `mock-*` initialiser, Slint's first frame renders the stub before Rust's first `set_<x>` lands. Result: a visible flicker. Always remove stub initialisers in the same PR as the wiring.

### R5. `Bridge.active-panel` mutated from both Slint and Rust

If Rust ever writes `Bridge.active-panel = Panel.x`, ensure no concurrent Slint click is also writing. Cluster E's discipline ("either Slint mutates, or a single Rust dispatcher mutates — never both directly") prevents this.

---

## Section 11 — Verification checklist

Run after **each** cluster migrates. Don't batch — catch regressions per cluster.

```sh
# 1. No remaining mock-* properties on the migrated page (ensure they were
#    removed, not just shadowed):
grep -rn 'mock-' senders/android/ui/pages/<migrated_page>.slint
# Expected: 0 matches.

# 2. No Slint-side direct mutations of Bridge properties that should be
#    Rust-driven:
grep -n 'Bridge\.<promoted_property> *=' senders/android/ui/
# Expected: 0 matches (all writes go through callbacks now).

# 3. Bridge property is declared in bridge.slint:
grep -n '<promoted_property>' senders/android/ui/bridge.slint
# Expected: at least 1 match.

# 4. Rust handler exists in lib.rs:
grep -n 'set_<promoted_property>\|on_<promoted_callback>' senders/android/src/lib.rs
# Expected: matching number per the cluster.

# 5. cargo build + lint:
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings

# 6. Smoke test in slint-viewer for the migrated page:
slint-viewer senders/android/ui/pages/<migrated_page>.slint
# Verify the page renders without the stub initialiser values.
```

The Phase 8 audit script from [`PHASE-8-rust-bridge.md`][spec] should be inverted at the end:

```sh
# After each cluster, the inverse of the placeholder audit holds:
grep -RnE 'Bridge\.(<list of newly wired properties>)' senders/android/ui/ \
  && echo "OK: live bindings present" \
  || echo "FAIL: cluster claimed wired but no Bridge bindings detected"
```

---

## Section 12 — Stop conditions / when Phase 8 is done

Phase 8 is "done" — the placeholder audit can be removed — when **all** of:

1. Every UI phase that shipped to `master` has its corresponding cluster items wired.
2. Every `mock-*` property on a `pages/*.slint` file is gone OR is documented in this plan as "intentionally page-local" (e.g. `pending-action` in Phase 19).
3. Every `Bridge.*` property declared in `bridge.slint` has either a producer (Rust → UI) or a consumer (UI → Rust callback) — no orphaned properties.
4. `cargo build` + `cargo clippy --all-targets -- -D warnings` are clean.
5. The Phase 8 placeholder gate audit (`grep -RnE 'Bridge\.(status-items|quick-actions|app-version)' senders/android/ui/`) reports the expected number of matches per the migration table.

Updates to `PHASE-8-rust-bridge.md`:
- Replace the "explicitly **deferred**" header with "Reactivated <date>; see this migration plan for execution log".
- Move the placeholder audit to a "regression guard" section that triggers on **new** UI phases (Phases 28+) — same discipline, but only the as-yet-unwired ones.

---

## Slint-doc references used

- **`global Bridge { ... }` declaration** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx`.
- **`callback name(args);` declaration** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **`in` vs `out` vs `in-out` property semantics** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`<=>` two-way binding** — same.
- **`slint::VecModel<T>` and `ModelRc<T>` for collection properties** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Enum mapping Slint ↔ Rust** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **`upgrade_in_event_loop` discipline** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx` (cross-reference) and Slint's Rust API docs (out-of-tree).
- Per-phase guide cross-references:
  - [`PHASE-13-reimplement-instructions.md`](./PHASE-13-reimplement-instructions.md) (if/when written)
  - [`PHASE-14-reimplement-instructions.md`](./PHASE-14-reimplement-instructions.md)
  - [`PHASE-15-reimplement-instructions.md`](./PHASE-15-reimplement-instructions.md)
  - [`PHASE-16-reimplement-instructions.md`](./PHASE-16-reimplement-instructions.md)
  - [`PHASE-17-reimplement-instructions.md`](./PHASE-17-reimplement-instructions.md)
  - [`PHASE-18-reimplement-instructions.md`](./PHASE-18-reimplement-instructions.md)
  - [`PHASE-19-reimplement-instructions.md`](./PHASE-19-reimplement-instructions.md)
  - [`PHASE-20-reimplement-instructions.md`](./PHASE-20-reimplement-instructions.md)
  - [`PHASE-21-reimplement-instructions.md`](./PHASE-21-reimplement-instructions.md)
  - [`PHASE-22-reimplement-instructions.md`](./PHASE-22-reimplement-instructions.md)
  - [`PHASE-23-reimplement-instructions.md`](./PHASE-23-reimplement-instructions.md)
  - [`PHASE-25-reimplement-instructions.md`](./PHASE-25-reimplement-instructions.md)
  - [`PHASE-26-reimplement-instructions.md`](./PHASE-26-reimplement-instructions.md)
  - [`PHASE-27-reimplement-instructions.md`](./PHASE-27-reimplement-instructions.md)

---

## What's NOT in this plan

- **New UI phases that didn't ship a UI-only reimplement guide.** Phases 12, 24, 28+ aren't in the migration index because their patterns aren't established yet.
- **Rust-side architectural choices** (where the cast history persists, which crate handles JNI, log buffer ring size). These are decisions for the implementer, not migration scope.
- **Performance tuning** for high-volume properties (debug log push frequency, status-items polling rate). Address per-property as needed.
- **Removal of the placeholder audit** in `PHASE-8-rust-bridge.md`. Do that as the last commit of Phase 8 — confirms the gate has held until completion.
- **`@tr(...)` localisation sweep.** Phase 9.
- **Integration tests for the wired bindings.** Phase 10.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-8-rust-bridge.md

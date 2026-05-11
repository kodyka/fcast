# Phase 8 — Section 6: Cluster E — overlay invariants (documentation only)

> Section 6 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-5-cluster-D-destructive-flows.md`](./PHASE-8-Section-5-cluster-D-destructive-flows.md) first.

**Cluster E is the only cluster with no code changes.** It writes down three invariants — `Bridge.active-panel`, `Bridge.lifecycle`, and `Bridge.app-state` — so subsequent phases don't accidentally break them.

If you've already done Clusters F + A + B + C + D, you've already implicitly enforced these invariants. Cluster E is the documentation that makes them explicit and stops Phase 9-48 from regressing them.

| Item | What we add | Files touched |
|---|---|---|
| E1 | Top-of-file comment on `senders/android/ui/main.slint` listing the 3 invariants | 1 |
| E2 | `phases/PANEL-INVARIANTS.md` (optional dedicated doc) | 1 (new) |

**Net new code:** ~30 lines of comments. Zero Slint or Rust functional changes.

**Risk:** zero — pure documentation.

---

## 6.1 — Why these invariants matter

By the end of Cluster D, `Bridge.active-panel`, `Bridge.lifecycle`, and `Bridge.app-state` each have **multiple writers**:

| Property | Writer 1 | Writer 2 |
|---|---|---|
| `active-panel: Panel` | Slint pages (settings rows, control bar buttons, route-back buttons) | Rust may need to surface a panel from a `Bridge.connect-receiver` failure or "Wi-Fi dropped" path |
| `lifecycle: LifecycleMode` | Slint settings PRIVACY rows write via `engage-lock` / `engage-stealth` callbacks (Cluster B4) | Rust handlers `on_engage_lock` / `on_engage_stealth` / `on_start_snapshot_countdown` write `set_lifecycle` |
| `app-state: AppState` | Slint connect/casting pages call `Bridge.change-state(AppState.x)` | Rust `invoke_change_state` is the device-event-driven authoritative writer |

In each case **multiple writers is fine**, but only when each writer goes through a single chokepoint. The invariants below name those chokepoints.

---

## 6.2 — INVARIANT A: `Bridge.active-panel` has a single Rust dispatcher

```text
INVARIANT — Bridge.active-panel
  - Mutated from Slint by panel-opening clicks (settings row, control-bar
    button, page-internal navigation). These are direct writes:
        Bridge.active-panel = Panel.audio;
    Slint owning panel routing is correct — opening a panel is a pure UI
    state change.

  - Mutated from Rust ONLY through a single dispatcher: open_panel(p: Panel).
    Do not write Bridge.active-panel from multiple Rust call sites; if you
    need to surface a panel from a Rust path (e.g. "connection lost" auto-
    routes to a status panel), call open_panel.

  - The chain of `if Bridge.active-panel == Panel.x:` blocks in main.slint
    is read-only from the implementation's perspective. New panels add a
    single `if` branch; the old branches stay in the same order.
```

**Recommended Rust dispatcher:**

```rust
// senders/android/src/lib.rs

fn open_panel(ui_handle: slint::Weak<MainWindow>, panel: Panel) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().set_active_panel(panel);
    });
}

// Use it everywhere instead of inline `set_active_panel(...)` calls:
//   open_panel(ui_handle.clone(), Panel::Settings);
//   open_panel(ui_handle.clone(), Panel::DebugVideo);
```

**Why a single dispatcher:**

- Future hooks (logging, analytics, panel-stack state) attach in one place.
- A grep for `set_active_panel` outside of the dispatcher is a code-review red flag.
- Slint's panel chain in `main.slint` doesn't care who opens the panel — it just renders whatever's in `Bridge.active-panel`. The dispatcher is purely a Rust convention.

---

## 6.3 — INVARIANT B: `Bridge.lifecycle` is Rust-authoritative

```text
INVARIANT — Bridge.lifecycle (LifecycleMode)
  - Mutated from BOTH Slint (settings PRIVACY rows: lock / stealth /
    countdown) AND Rust (real lock-engagement / inactivity / countdown
    completion). Dual writers are fine HERE because the rows mutate via
    callbacks (engage-lock / engage-stealth / start-snapshot-countdown)
    that go through Rust. Rust is the only writer that calls set_lifecycle.

  - LockOverlay / StealthOverlay / SnapshotCountdown overlays live above
    the Panel layer in main.slint. They directly write Bridge.lifecycle
    when the user dismisses (long-press unlock, tap-to-wake, countdown
    finish). This is OK because:
      a. Each overlay has exactly one exit path.
      b. The exit path is the *only* Slint-side write.
      c. Rust observes (via property-changed) but doesn't fight back.
```

**Why direct Slint writes from overlays are an exception:**

- The lock/stealth/countdown overlays *are* the lifecycle. When the user dismisses Lock, the only thing that needs to happen is `Bridge.lifecycle = LifecycleMode.normal;`. Round-tripping through Rust is meaningless because Rust isn't holding any extra state — it just reads `lifecycle` to decide whether to gate features.
- This is consistent with `engage-lock` / `engage-stealth` going *through* Rust on the way *in* (because Rust may want to react with system-level work like disabling gesture nav). The exit path doesn't need that; Slint can just flip the property.

---

## 6.4 — INVARIANT C: `Bridge.app-state` goes through `change-state(...)` only

```text
INVARIANT — Bridge.app-state (AppState)
  - Mutated from Slint via the public function Bridge.change-state(to).
    The function is declared in bridge.slint and contains the only Slint-
    side write to Bridge.app-state.
  - Rust calls Bridge.change-state(...) to drive lifecycle transitions.
    Never mutate Bridge.app-state directly; always go through change-state
    so future hooks (e.g. logging, analytics) attach in one place.
```

**Why a Slint-side `function` instead of an `in-out property`:**

- The `change-state(to: AppState)` function is the chokepoint. All Slint→Rust state pushes look identical:
  ```slint
  Bridge.change-state(AppState.Connected);
  ```
- Rust's `invoke_change_state(...)` is the same shape on the Rust side. They funnel.
- This is already in master since Phase 5 — Cluster E just affirms it.

---

## 6.5 — Recommended placement: top-of-file comment on `main.slint`

```slint
// senders/android/ui/main.slint
//
// =====================================================================
// Bridge invariants (Phase 8 / Cluster E)
// =====================================================================
//
// 1. Bridge.active-panel — Slint pages may write directly. Rust writes
//    only through `open_panel(panel: Panel)` in lib.rs.
//
// 2. Bridge.lifecycle — Rust is the authoritative writer (set_lifecycle).
//    Slint writes directly only from inside the LockOverlay /
//    StealthOverlay / SnapshotCountdown exit paths. Settings rows that
//    *engage* a lifecycle mode go through Bridge.engage-lock /
//    engage-stealth / start-snapshot-countdown callbacks.
//
// 3. Bridge.app-state — Slint writes ONLY via Bridge.change-state(to).
//    No Slint-side direct writes (`Bridge.app-state = ...`). Rust writes
//    only via invoke_change_state, which calls change-state internally.
//
// Breaking any of these invariants will produce subtle bugs that race
// with the UI thread. Treat changes to these properties' producers /
// consumers as cross-cutting and review them as a unit.
// =====================================================================

import { Bridge, Panel, AppState, LifecycleMode } from "bridge.slint";
// …
```

---

## 6.6 — Optional: dedicated `phases/PANEL-INVARIANTS.md`

If you prefer the invariants to live in a dedicated doc rather than an inline header (because Phase 28+ adds many more panels and the comment will grow), create `draft/slint-ui/phases/PANEL-INVARIANTS.md` with:

````markdown
# Bridge invariants (post-Phase 8)

This file is the canonical reference for `Bridge.active-panel`,
`Bridge.lifecycle`, and `Bridge.app-state` writers. Any PR that adds a
new writer to one of these properties must update this file too.

## Bridge.active-panel
- Slint writers: …
- Rust writer: `open_panel(p)` in `lib.rs:<line>`.
- Reader chain: `main.slint:<line>` (`if Bridge.active-panel == Panel.x: …`).

## Bridge.lifecycle
- Slint writers: …
- Rust writers: …

## Bridge.app-state
- Slint writers: …
- Rust writers: …
````

This is optional — pick whichever ergonomics you prefer.

---

## 6.7 Cluster E verification

```sh
# 1. The invariant comment is present on main.slint (or PANEL-INVARIANTS.md).
grep -nE 'INVARIANT|Bridge invariants' senders/android/ui/main.slint draft/slint-ui/phases/PANEL-INVARIANTS.md 2>/dev/null

# 2. Rust has exactly ONE call to set_active_panel outside of the open_panel helper.
#    (Or zero — perfectly fine.)
grep -n 'set_active_panel' senders/android/src/lib.rs
# Expected: 1 match (inside open_panel) or 0.

# 3. Rust has set_lifecycle calls only inside the engage-lock / engage-stealth /
#    start-snapshot-countdown / exit-lifecycle handlers (Cluster B4).
grep -n 'set_lifecycle' senders/android/src/lib.rs
# Expected: 4 matches (one per lifecycle handler).

# 4. Slint writers of Bridge.app-state (other than the change-state function body)
#    are zero.
grep -n 'Bridge.app-state *=' senders/android/ui/
# Expected: 0 matches outside bridge.slint's change-state function.
```

---

## 6.8 Commit message

```
docs(slint): document Panel / LifecycleMode / AppState invariants (Phase 8 / Cluster E)
```

---

## 6.9 Exit criteria for Section 6

- [x] Top-of-file comment on `main.slint` (or dedicated `PANEL-INVARIANTS.md`) names the three invariants
- [x] `set_active_panel` calls in `lib.rs` go through `open_panel(...)` (or there are zero such calls)
- [x] `set_lifecycle` calls match exactly the lifecycle callback handler set
- [x] No Slint-side direct writes to `Bridge.app-state` outside the `change-state(...)` function body
- [x] No code-side regressions (Cluster E is documentation only)

You can now move to **Section 7 — verification** at [`PHASE-8-Section-7-verification.md`](./PHASE-8-Section-7-verification.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — global property semantics underpin all three invariants.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — `in` / `in-out` / `out` direction discipline.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx` — `Bridge.change-state(...)` chokepoint pattern.

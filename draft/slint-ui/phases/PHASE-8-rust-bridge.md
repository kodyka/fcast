# Phase 8 — Rust Bridge Wiring (deferred to `futures/`)

> **Original intent:** wire all new Slint structs (`StatusItem`, `ReceiverItem`,
> `QuickAction`, `Panel`, `app-version`) into Rust handlers in `lib.rs`.
> **New scope (UI-only roadmap):** explicitly **deferred**. All UI phases (5–7
> and 12–27) consume inline stub data, never `Bridge.*` setters. Rust wiring
> lands as a single hardening pass once the UI surface is signed off.

**Status:** `[ ] Deferred — placeholder phase`
**Blocking:** Nothing (UI phases stand on their own with stub data).
**Will be unblocked by:** UI sign-off across Phases 5–7 and any subset of 12–27 that ships.

---

## Why this phase is empty for now

The phase plan was rewritten so every UI surface — receiver list, settings,
status overlay, capture preview, audio/camera placeholders, etc. — uses inline
`in-out` stub properties on the page or component. That choice keeps the
visual scaffold working without dragging the Rust bridge into every iteration.
It also means there's nothing to wire here yet: there's no live `status-items`
producer to bind, no `ReceiverItem` mapper, no `Panel` callback to register,
because the UI side already writes those properties directly from Slint where
needed (e.g. `Bridge.active-panel = Panel.none` in the settings Done button).

When the UI is signed off, this doc gets re-expanded into the original wiring
checklist (see `futures/RUST-WIRING.md`, to be authored when promotion happens).

---

## What lives in `futures/` until promotion

The full Rust integration backlog — including `UiState` helper module, `VecModel`
construction, `upgrade_in_event_loop` dispatch, `on_invoke_action` handler,
`open_panel` / `close_panel`, `app-version` setter, `mdns-enabled` setter, and
the `Bridge.devices` migration from `[string]` to `[ReceiverItem]` — is
captured at high level in `draft/slint-ui/futures/README.md` under
"Rust wiring (post-UI)" and will be expanded into a phase doc when the UI is
ready to consume real data.

---

## Stop conditions for the placeholder

This phase remains a placeholder until **all of**:

1. Phases 5, 6, 7 ship as UI-only and survive a visual review.
2. At least one of the new UI phases (12–27) ships and exposes a stub model
   that maps cleanly onto a Rust producer.
3. The team explicitly opts to add live data (rather than continuing to ship
   placeholder builds for design review).

Until those conditions hold, do **not** add `UiState`, `VecModel`,
`on_invoke_action`, or any `set_*` calls in `lib.rs`. They will conflict with
inline stub properties used by the UI.

---

## Audit

Run periodically to confirm the placeholder discipline is holding:

```sh
# UI files must not reference new Rust-driven Bridge properties.
grep -RnE 'Bridge\.(status-items|quick-actions|app-version)' senders/android/ui/ \
  && echo "FAIL: live Bridge property bound while still in UI-only mode" \
  || echo "OK"
```

Existing legacy bindings (`Bridge.devices: [string]`, `Bridge.app-state`,
`Bridge.show-debug`, `Bridge.test-status`) are explicitly allowed — they predate
the UI-only roadmap and continue to work as before.

---

## When this phase is reactivated

The original Phase 8 content (UiState, VecModel wiring, `upgrade_in_event_loop`
discipline, `on_invoke_action` dispatch, `Panel` callbacks, `app-version`
setter) will be reinstated here. Until then, this doc serves only as a
gate / reminder that UI changes do **not** require Rust changes.

---

## Slint best practices applied here

- **Defer wiring.** Building the UI on stub data first is a recognised
  Slint workflow — the upstream
  [SKILL.md](https://github.com/slint-ui/slint/blob/master/docs/skills/slint/SKILL.md)
  promotes "design-time previews" using inline static models so the Rust side
  doesn't have to exist yet.
- **`in-out` properties on the page itself** are the canonical way to drive
  stub state without a global. Promoting one to a `Bridge.<name>` setter later
  is a one-line change at the call site.
- **Single source of truth.** When real Rust data lands, the UI only changes
  one binding (`items: root.mock-status-items;` → `items: Bridge.status-items;`).
  No component rewrites.

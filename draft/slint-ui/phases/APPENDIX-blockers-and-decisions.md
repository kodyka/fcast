# Appendix — Codec Blockers and Open Decisions

> Companion to the phase files. Tracks the cross-cutting concerns that affect multiple phases
> but do not belong in any single phase file.

---

## Codec blocker dependency map

The casting path (Phases 5, 6, and 10) requires working GStreamer pipelines on Android.
Track these items in `senders/android/TODO.codecs.md`. Reproduced here for UI planning context.

| Codec item | Priority | UI work blocked |
|---|---|---|
| P0-1: H.264 encoder selection (`amcvidenc`) | P0 — Critical | Phase 5 encoder name in `StatusOverlay`; Phase 10 end-to-end cast test |
| P0-2: `rtmp2sink` / `rtmpsink` fallback | P0 — Critical | Phase 7 RTMP destination row (deferred until codec works) |
| P0-3: `fallbacksrc` hardening | P0 — Critical | Phase 10-I end-to-end cast test; `WaitingForMedia` UX stability |
| P1-5: Startup element validation | P1 — High | Phase 5 "Encoder" status pill (shows unknown until this lands) |
| P1-4: `videoconvert` color space | P1 — High | Not directly UI-visible; affects cast success rate |

**Rule:** Do not invest in casting-page UI polish (Phases 5, 6, 10-I) until P0-1 and P0-3
are resolved. Phases 1–4 are pure UI restructuring and are never blocked by codec status.

---

## Open architectural decisions

These decisions affect multiple phases and must be resolved before or during Phase 1.

### Decision A — Where do production `.slint` files live?

| Option | Pros | Cons |
|---|---|---|
| **A1: `senders/android/ui/`** (recommended) | Real files, built by CI, tested on device | Draft sketches must be kept separate |
| A2: `draft/slint-ui/ui/` then promote | Low-risk iteration sandbox | Risk of double-maintenance; draft files can diverge |

**Recommendation:** Option A1. Keep `draft/slint-ui/` as a read-only design reference.
All changes that affect the app go directly into `senders/android/ui/`.

**Decision:** `[ ] Not decided` / `[ ] A1` / `[ ] A2`

---

### Decision B — Should `migration-skeleton.slint` be promoted?

`draft/slint-ui/ui/migration-skeleton.slint` contains a near-complete prototype of the
proposed architecture (`Panel`, `QuickAction`, `StatusItem`, etc.).

| Option | When to do it |
|---|---|
| **B1: Promote to `senders/android/ui/`** (recommended) | After Phase 1 module split is complete and passes CI |
| B2: Use as design reference only, rewrite from scratch | If the skeleton has diverged significantly from Phase 1 output |

**Recommendation:** Option B1 — the skeleton is a valid starting point and will save time.
Diff it against the Phase 1 output and merge the useful parts.

**Decision:** `[ ] Not decided` / `[ ] B1` / `[ ] B2`

---

### Decision C — `Panel` routing: overlay or page replacement?

Two possible architectures for panel navigation:

| Option | Description |
|---|---|
| **C1: Panel overlay layer** (recommended) | `Panel` components rendered on top of the `AppState` page stack using `if Bridge.active-panel == ...`. The underlying page stays mounted. |
| C2: Panel replaces AppState page | When a Panel is active, the entire `AppState` page is replaced by the Panel page. |

**Recommendation:** Option C1 (overlay layer). Cleaner separation between background
application state and foreground navigation panels. Matches Moblin's sheet-based presentation.

**Decision:** `[ ] Not decided` / `[ ] C1` / `[ ] C2`

---

### Decision D — `AppState.SelectingSettings` lifetime

Currently `SelectingSettingsView` (renamed `SettingsPageView`) is shown before casting begins
to let the user pick resolution and framerate. After Phase 7, there is also `FullSettingsPage`.

| Option | Description |
|---|---|
| **D1: Keep both** (recommended) | `SettingsPageView` remains as the pre-cast "Start" flow. `FullSettingsPage` is the in-app settings opened from the control bar. |
| D2: Merge into FullSettingsPage | `FullSettingsPage` gains a "Start Casting" button at the top; `AppState.SelectingSettings` is removed. |

**Recommendation:** Option D1 for simplicity. The pre-cast selection is a natural checkpoint.

**Decision:** `[ ] Not decided` / `[ ] D1` / `[ ] D2`

---

### Decision E — Android back button handling

Slint on Android may or may not intercept the system back button depending on the version.

- [ ] Test after Phase 7 whether tapping Android back while a `Panel` is open closes it.
- [ ] If not handled automatically, add a `FocusScope` (or `key-pressed` callback on
  the window itself) and return an `EventResult` value from the handler:

  ```
  FocusScope {
      key-pressed(event) => {
          if event.text == Key.Escape && Bridge.active-panel != Panel.none {
              Bridge.close-panel();
              return EventResult.accept;
          }
          return EventResult.reject;
      }
      // ... rest of MainWindow content as children ...
  }
  ```

  > Slint's keyboard event API uses the `EventResult` enum (`accept` / `reject`) and
  > the `Key` enum from `std-widgets.slint`. Returning a bare `accept` /  `reject`
  > identifier does not compile. Reference:
  > [`reference/elements/focusscope.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/focusscope.mdx).

- [ ] Document the chosen approach in `senders/android/README.md`.

**Decision:** `[ ] Not investigated yet`

---

## Phase dependency graph

```
Phase 0 (baseline)
    └─► Phase 1 (module split)
            ├─► Phase 2 (theme)
            │       └─► Phase 3 (components)
            │               ├─► Phase 4 (control bar)
            │               │       └─► Phase 5 (status overlay) ← [codec P0 dependency]
            │               ├─► Phase 6 (receiver list UX)
            │               └─► Phase 7 (settings pages)
            │                       ├─► Phase 8 (Rust bridge)
            │                       └─► Phase 9 (localization)
            └─► Phase 10 (testing) ← runs after EVERY phase
                                   ← [codec P0 dependency for 10-I]

Phase 11 (source tracking) — updated after each phase, not a gate
```

---

## Phases that can run in parallel

Once Phase 1 is complete and the module layout exists:

| Parallel track A | Parallel track B |
|---|---|
| Phase 2 → Phase 3 → Phase 4 | Phase 6 (receiver list) |
| Phase 5 (status overlay) | Phase 7 (settings pages) |

Phase 8 (Rust bridge) waits for Phases 4–7 to define the full Bridge API.
Phase 9 (localization) waits for Phase 7 to finalize all strings.

---

## Known risks and mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Slint `compat-1-2` feature hides cross-file `export struct` | Low | High | Test in Phase 0-A; if blocked keep structs in `bridge.slint` only |
| `slint::include_modules!()` generates wrong Rust type names after rename | Low | High | Keep component names stable; only `SelectingSettingsView` → `SettingsPageView` planned |
| `ListView` performance with 50+ receivers | Medium | Medium | Cap mDNS list at 20 items in Rust if needed |
| Android back-button not intercepted by Slint | Medium | Low | Wire `key-pressed` handler in Phase 10-H |
| Codec P0 blockers unresolved when Phase 10-I is attempted | High | Medium | Gate 10-I explicitly on codec resolution; other tests proceed |
| `@tr` not available in pinned Slint version | Low | Low | Defer Phase 9; strings still work unwrapped |
| `draft/slint-ui/ui/migration-skeleton.slint` diverges from Phase 1 output | Medium | Low | Compare before promoting; take the better version |

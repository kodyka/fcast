# FCast Slint UI — Phase Files

Each file in this directory tracks one phase of the Slint UI evolution plan.
Start with `PHASE-0` and work forward. See `../TODO.md` for the high-level overview.

## Phase index

| File | Phase | One-line summary | Depends on |
|---|---|---|---|
| `PHASE-0-baseline-audit.md` | 0 | Confirm Slint version, build, Bridge contract, codec status | — |
| `PHASE-1-split-modules.md` | 1 | Split `main.slint` into `bridge/theme/pages/components` files | 0 |
| `PHASE-2-theme-tokens.md` | 2 | Replace all hardcoded colors/sizes with `Theme` tokens | 1 |
| `PHASE-3-components.md` | 3 | Build `PrimaryButton`, `SettingsValueRow`, etc. | 1, 2 |
| `PHASE-4-control-bar.md` | 4 | `CastControlBar` + model-driven `QuickAction` buttons | 1, 2, 3 |
| `PHASE-5-status-overlay.md` | 5 | `StatusOverlay` pills on casting screen | 1, 2, 4 |
| `PHASE-6-receiver-list.md` | 6 | `ReceiverItem` model, name+address rows, spinner empty state | 1, 2, 3 |
| `PHASE-7-settings-pages.md` | 7 | `FullSettingsPage` with FCast-specific sections + `Panel` routing | 1, 2, 3, 4 |
| `PHASE-8-rust-bridge.md` | 8 | Wire all new structs/callbacks into `lib.rs` + `ui_state.rs` | 4, 5, 6, 7 |
| `PHASE-9-localization.md` | 9 | Wrap all strings with `@tr("...")`, generate `.pot` template | 7 |
| `PHASE-10-testing.md` | 10 | Build gate + on-device validation checklist (runs each phase) | each phase |
| `PHASE-11-source-tracking.md` | 11 | Moblin → Slint group completeness reference table | reference |
| `APPENDIX-blockers-and-decisions.md` | — | Codec blockers, arch decisions, dependency graph, risks | — |

## Quick-start order

```
Phase 0  →  Phase 1  →  Phase 2  →  Phase 3
                                         ├──→  Phase 4  →  Phase 5
                                         ├──→  Phase 6
                                         └──→  Phase 7  →  Phase 8
                                                        →  Phase 9
Phase 10 runs alongside every phase above.
Phase 11 is a living reference, updated as phases complete.
```

## Status key

Each phase file has a `**Status:**` line at the top:

| Status | Meaning |
|---|---|
| `[ ] Not started` | No work begun |
| `[ ] In progress` | Some tasks checked off |
| `[x] Complete` | All exit criteria met |
| `[ ] Blocked` | Waiting on a dependency (codec or decision) |
| `Defer` | Intentionally excluded from current scope |

## What changed vs. the original `TODO.md`

The original `draft/slint-ui/TODO.md` (Phases 0–9) was speculative and did not reference
actual codebase paths. These phase files are grounded in the real project:

- Phase 0 contains the actual `Bridge` contract table extracted from `main.slint`.
- Phases 1–3 include exact file paths and build-check steps.
- Phases 4–7 include concrete Slint code snippets.
- Phases 8 includes concrete Rust code snippets for `lib.rs`.
- The Appendix captures the dependency graph and codec blockers from `TODO.codecs.md`.

# FCast Slint UI migration draft

This folder is a planning workspace for migrating ideas from Moblin's SwiftUI UI (`draft/moblin-ui`) into FCast Android sender's Slint UI.

> **Where am I in the plan?** See [`phases/STATUS.md`](phases/STATUS.md).
> That file is the canonical, evidence-grounded snapshot of which phases are
> shipped in `senders/android/ui/` today and which are still on paper.
> [`phases/README.md`](phases/README.md) has the full roadmap shape and per-phase index.

## Contents

- [`phases/`](phases/) — **the active roadmap.** One markdown file per phase
  (`PHASE-0-baseline-audit.md` … `PHASE-48-other-broadcast-deferrals.md`),
  plus per-phase **reimplement guides** for Phases 5-10 and 12-27 with
  step-by-step Slint snippets, plus `PHASE-8-bridge-migration-plan.md` for
  the Rust reactivation plan, plus `STATUS.md` for the live audit and
  `APPENDIX-blockers-and-decisions.md` for cross-cutting decisions. **Start here.**
- `TODO.md` — original speculative migration checklist (Phases 0–9 only).
  Superseded by `phases/`; kept for historical reference.
- `docs/swiftui-to-slint-guide.md` — concept mapping, architecture notes, Slint patterns, and risks.
- `docs/` — copy of upstream Slint documentation (`astro/src/content/docs/`)
  used as the citation source for every reimplement guide.
- `analysis/summary.md` — generated inventory summary of the copied Moblin SwiftUI files.
- `analysis/moblin-swiftui-inventory.csv` — per-file SwiftUI pattern counts.
- `source-inventory/moblin-view-files.md` — copied list of all 279 Moblin `View/**/*.swift` files.
- `source-inventory/moblin-ui-info.md` — copied Moblin UI architecture notes from the previous draft.
- `ui/migration-skeleton.slint` — draft-only Slint skeleton showing proposed globals, components, control bar, status overlay, and settings page shape.
- `futures/NOT-APPLICABLE.md` — per-Moblin-file applicability triage that the speculative phases (28–48) are derived from.

## Important conclusion

Moblin UI cannot be copied directly into FCast Android sender. Moblin is SwiftUI/iOS, while FCast Android sender is Rust + Slint. The correct path is to reimplement the layouts and interaction concepts in `.slint`, with Rust providing state and callbacks through Slint globals/models.

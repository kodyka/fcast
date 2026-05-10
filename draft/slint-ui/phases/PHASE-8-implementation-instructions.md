# Phase 8 — Rust Bridge Reactivation: Step-by-step Implementation Guide (TOC)

**Audience:** developer ready to *execute* Phase 8 — wire the deferred `mock-*` properties on every shipped UI page to real Rust producers/consumers in `senders/android/src/lib.rs`.
**Goal:** by the end of this guide, every `pages/*.slint` file is free of `mock-*` initialisers, every promoted property is a `Bridge.*` property, every promoted mutation goes through a Slint→Rust callback, and `cargo build && cargo clippy --all-targets -- -D warnings` is clean.
**Out of scope:** any new UI work; `@tr()` audits ([Phase 9](./PHASE-9-reimplement-instructions.md)); UI-only validation tooling ([Phase 10](./PHASE-10-reimplement-instructions.md)); chat / streaming / scenes phases (28+).
**Constraint of this *document*:** guide-only, with full Slint and Rust snippets. The actual migration happens in a separate PR.

---

## How this guide is structured

Originally a single ~1500-line document, the guide has been split into 10 self-contained sections under `draft/slint-ui/phases/`. Each section is a complete reading unit — read top-to-bottom — with concrete before/after Slint diffs, Rust handler skeletons, verification commands, and citations to upstream Slint docs.

| # | File | Cluster | Topic |
|---|---|---|---|
| 0 | [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) | — | Audit `mock-*` inventory, capture already-wired bindings, branch off master |
| 1 | [`PHASE-8-Section-1-cluster-F-shared-tokens.md`](./PHASE-8-Section-1-cluster-F-shared-tokens.md) | F | `Bridge.banner-*` + `BannerSeverity` enum + `flash_banner` helper |
| 2 | [`PHASE-8-Section-2-cluster-A-readonly-view-models.md`](./PHASE-8-Section-2-cluster-A-readonly-view-models.md) | A | A1: status badges, A2: app-version, A3: network interfaces, A4: recording state+elapsed, A5: log entries |
| 3 | [`PHASE-8-Section-3-cluster-B-single-page-state.md`](./PHASE-8-Section-3-cluster-B-single-page-state.md) | B | B1: audio settings, B2: camera, B3: recording controls, B4: lifecycle, B5: Wi-Fi Aware |
| 4 | [`PHASE-8-Section-4-cluster-C-list-mutations.md`](./PHASE-8-Section-4-cluster-C-list-mutations.md) | C | C1: bitrate presets, C2: quick-actions unification (**B12 fix**), C4: macros, C5: log clear |
| 5 | [`PHASE-8-Section-5-cluster-D-destructive-flows.md`](./PHASE-8-Section-5-cluster-D-destructive-flows.md) | D | D1: backup/import/reset, D2: cast history |
| 6 | [`PHASE-8-Section-6-cluster-E-overlay-invariants.md`](./PHASE-8-Section-6-cluster-E-overlay-invariants.md) | E | `active-panel` / `lifecycle` / `app-state` invariants — documentation only |
| 7 | [`PHASE-8-Section-7-verification.md`](./PHASE-8-Section-7-verification.md) | — | Per-cluster verification recipes; on-device walkthrough |
| 8 | [`PHASE-8-Section-8-pitfalls.md`](./PHASE-8-Section-8-pitfalls.md) | — | Common pitfalls (R1–R5 + 8 more) with preventive greps |
| 9 | [`PHASE-8-Section-9-stop-conditions.md`](./PHASE-8-Section-9-stop-conditions.md) | — | Completion criteria; doc-sync checklist |

---

## Suggested execution order

Read the sections sequentially. Then implement in the order:

1. **Section 0** — Pre-flight (run today)
2. **Section 1** — Cluster F (smallest; unblocks the rest)
3. **Section 2** — Cluster A (read-only view models; pure additions)
4. **Section 3** — Cluster B (single-page state; per-page chunks)
5. **Section 4** — Cluster C (list mutations; the biggest section + the B12 fix)
6. **Section 5** — Cluster D (destructive flows; depends on Cluster F's banner)
7. **Section 6** — Cluster E (documentation only; can land in any PR)
8. **Section 7** — verification before each commit, full walkthrough at the end
9. **Section 8** — debugging reference (read once; consult when something breaks)
10. **Section 9** — final exit criteria; declare Phase 8 complete

**Recommended PR cadence:** one PR per cluster (6 PRs total). Smaller PRs are easier to review, revert, and CI-validate than one mega-PR. See [Section 0.6](./PHASE-8-Section-0-preflight.md#06-branch) for the typical PR sizing.

---

## Companion documents

- [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md) — original Phase 8 spec (the "what").
- [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) — strategy / risk register / per-cluster index (the "why").
- This file (TOC) + the `PHASE-8-Section-*.md` files (the "how").

Read the migration plan first if you haven't. The guide assumes its terminology (Cluster A / B / C / D / E / F) and its per-phase index table.

---

## Per-phase quick-reference

If you'd rather migrate phase-by-phase (e.g. "I'm doing Phase 22 today; what does Phase 8 require for it?"), the following table maps each shipped UI phase to its Phase-8 cluster items:

| UI Phase | Slint property/properties to promote | Section |
|---|---|---|
| 5 (status overlay) | `Bridge.status-items: [StatusItem]` | Section 2.1 (A1) |
| 6 (receiver list) | (already wired — `Bridge.devices`, `connect-receiver`) | n/a |
| 7 (settings chassis) | (no list state to migrate; just routing) | n/a |
| 13 (status badges) | `Bridge.status-items` consumer in `components/status_badges.slint` | Section 2.1 (A1) |
| 14 (audio settings) | `Bridge.audio-source-idx`, `audio-muted`, `audio-input-gain`, `audio-bitrate-idx` | Section 3.1 (B1) |
| 15 (camera) | `Bridge.camera-idx`, `resolution-idx`, `framerate-idx`, `camera-mirror-front`, `camera-stabilization`, `camera-tap-to-focus`, `camera-zoom-level` | Section 3.2 (B2) |
| 16 (bitrate presets) | `Bridge.presets: [BitratePreset]` + 3 callbacks | Section 4.1 (C1) |
| 17 (quick actions) | `Bridge.quick-actions: [QuickAction]` (already declared); `move-bar-action`, `set-bar-action-enabled`, `save-bar-actions` callbacks | Section 4.2 (C2 — fixes B12) |
| 18 (lifecycle overlays) | `Bridge.lifecycle: LifecycleMode` (already declared); `engage-lock`, `engage-stealth`, `start-snapshot-countdown`, `exit-lifecycle` | Section 3.4 (B4) |
| 19 (backup/reset) | 5 callbacks (`export-settings`, `import-settings`, `reset-settings`, `clear-cast-history`, `clear-known-receivers`) | Section 5.1 (D1) |
| 20 (cast history) | `Bridge.history: [CastHistoryEntry]`, `selected-history-entry`; `clear-history`, `delete-history-entry`, `recast` | Section 5.2 (D2) |
| 21 (about / help) | `Bridge.app-version: string` | Section 2.2 (A2) |
| 22 (network) | `Bridge.network-interfaces: [NetworkInterface]` + `set-interface-enabled`; `Bridge.wifi-aware-enabled` + `set-wifi-aware` | Sections 2.3 (A3) + 3.5 (B5) |
| 23 (recording) | `Bridge.recording-state: RecordingState` + `recording-elapsed-s: int`; `start-recording`, `pause-recording`, `resume-recording`, `stop-recording` | Sections 2.4 (A4) + 3.3 (B3) |
| 24 (pairing) | (mostly already wired — `Bridge.connect-receiver`); `selected-receiver-id`, `selected-receiver-name` (already exists) | n/a (covered by existing handlers) |
| 25 (macros) | `Bridge.macros: [Macro]`, `macro-edit-id`; `save-macro`, `delete-macro`, `move-step`, `add-step`, `remove-step`, `run-macro` | Section 4.3 (C4) |
| 26 (debug log) | `Bridge.log-entries: [LogEntry]`; `clear-log-entries` | Sections 2.5 (A5) + 4.4 (C5) |
| 27 (utils) | (no Phase 8 work — Cluster F already promoted the banner severity vocabulary) | n/a |

---

## Quick start

If you're ready to begin and prefer minimal preamble:

```sh
# 1. Pre-flight (Section 0)
git checkout master && git pull
ls senders/android/ui/pages/    # confirm 22 files
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l
cargo build -p android-sender   # must be green
git checkout -b devin/$(date +%s)-phase-8-bridge-reactivation

# 2. Open Section 1 (Cluster F)
$EDITOR draft/slint-ui/phases/PHASE-8-Section-1-cluster-F-shared-tokens.md
```

Then work through Sections 1 → 6 in order, running Section 7's verification after each cluster. When all 6 cluster checkboxes in [Section 9.1](./PHASE-8-Section-9-stop-conditions.md#91--per-cluster-completeness) are checked, Phase 8 is complete.

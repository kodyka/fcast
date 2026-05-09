# FCast Slint UI — Phase Implementation Status

> **Canonical answer to "what's actually shipped vs. what's still on paper?"**
> All claims below are grounded in the live `senders/android/ui/` tree at the
> commit this file is being checked in against. If a row says "complete", the
> referenced files exist and meet the corresponding phase's exit criteria;
> if a row says "not started", the referenced files do not exist (no stubs,
> no scaffolding).

---

## How to read this document

Each row records four things:

1. **Phase** — short name + link to the phase spec.
2. **Declared status** — what the per-phase `**Status:**` line currently says.
3. **Actual status** — what is actually present in `senders/android/ui/`,
   verified by direct file inspection.
4. **Evidence** — file paths (and where useful, line numbers) that justify
   the actual status. For "not started" rows the evidence is the absence
   of any matching file.

The **Reimplement guide** column tracks the separate per-phase
`PHASE-N-reimplement-instructions.md` (or the consolidated
`PHASE-8-bridge-migration-plan.md`) artefacts shipped in PR #1. A guide is
not the same as an implementation — it is a step-by-step instruction
document an implementer can follow to land the phase. The presence of a
guide is independent of the actual code state.

Status taxonomy (matches `phases/README.md`):

| Symbol | Meaning |
|---|---|
| `[x] Complete` | All exit criteria met; UI present and importable. |
| `[x] Complete (UI-only)` | UI shipped as planned; Rust wiring deliberately deferred to Phase 8 — this is the **intended** end state for Phases 5–7 and 12–48 until Phase 8 reactivates. |
| `[~] Ongoing` | Recurring work (e.g. testing, source tracking); never has a single completion date. |
| `[ ] Deferred` | Intentionally parked behind another gate (Phase 8 waits on UI sign-off). |
| `[ ] Not started` | No work begun. |

---

## Foundation phases (0–11)

| Phase | Declared | Actual | Reimplement guide | Evidence |
|---|---|---|---|---|
| **0** Baseline audit | `[x] Complete` | `[x] Complete` | — | `phases/PHASE-0-baseline-audit.md` (doc-only phase). |
| **1** Split modules | `[x] Complete` | `[x] Complete` | — | Tree split as planned: `senders/android/ui/{bridge.slint, theme.slint, components/, pages/}`. |
| **2** Theme tokens | `[x] Completed` | `[x] Complete` | — | `senders/android/ui/theme.slint` exports the full surface / text / accent / severity / typography / spacing / radius token set (lines 7–44). |
| **3** Components | `[x] Complete` | `[x] Complete` | — | `components/buttons.slint` exports `PrimaryButton`, `TextButton`, `DestructiveButton`, `LoadingView`. `components/settings_rows.slint` exports `SettingsTextRow`, `SettingsValueRow`, `SettingsToggleRow`, `SettingsSliderRow`, `SettingsSection`. |
| **4** Control bar | `[ ] Not started` | **`[x] Complete`** ⚠️ | — | `components/control_bar.slint` exports `CastControlBar` + `QuickActionButton`; `bridge.slint` defines `QuickAction` struct + `invoke-action` callback; `main.slint` mounts the bar at the bottom of `MainWindow`; the bar routes the canonical `settings` / `debug` / `codec-test` ids to `Bridge.active-panel` and falls back to `Bridge.invoke-action(id)` for everything else. **The declared status line is stale and is updated by this PR.** |
| **5** Status overlay | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only)` | `PHASE-5-reimplement-instructions.md` | `components/status_overlay.slint` exports `StatusOverlay` + internal `StatusPill`; severity-keyed background driven by `StatusSeverity` enum; `pages/casting_page.slint` embeds the overlay with `mock-status-items` (3 entries) and a `mock-status-items-error` flip-stub for severity coverage screenshots. |
| **6** Receiver list | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only)` | `PHASE-6-reimplement-instructions.md` | `pages/connect_page.slint` defines `mock-devices: [ReceiverItem]` (3 stub entries), `mock-empty: bool`, and renders the empty-state spinner card or the populated list per the toggle. `bridge.slint` exports the `ReceiverItem` struct. |
| **7** Settings pages | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only)` | `PHASE-7-reimplement-instructions.md` | `pages/settings_page.slint` exports `FullSettingsPage` (header + scroll body + RECEIVER / VIDEO QUALITY / CODEC & DEBUG / ABOUT sections); `bridge.slint` defines `Panel` enum + `active-panel` property; `main.slint` mounts the panel layer with `if Bridge.active-panel == Panel.settings: FullSettingsPage { }`. |
| **8** Rust bridge | `[ ] Deferred — placeholder phase` | `[ ] Deferred` | `PHASE-8-bridge-migration-plan.md` | `senders/android/src/lib.rs` already wires `connect-receiver`, `start-casting`, `stop-casting`, `invoke-action`, `set_devices`, and `change-state`. The full Phase 8 ambition (replacing every page-local `mock-*` with `Bridge.*` properties) is intentionally parked until the UI layer stabilises. |
| **9** Localization | `[x] Complete` | `[x] Complete` | `PHASE-9-reimplement-instructions.md` | `senders/android/ui/i18n/messages.pot` exists, `@tr` used in all slint files. |
| **10** Testing | `[~] Ongoing — run after each UI phase merge` | `[~] Ongoing` | `PHASE-10-reimplement-instructions.md` | No automated UI-regression suite yet; `cargo build -p android-sender` is the build gate. The Phase 10 guide ships a runnable `senders/android/ci/ui-validate.sh` skeleton. |
| **11** Source tracking | `[~] Reference — update as phases ship` | `[~] Ongoing reference` | — | `phases/PHASE-11-source-tracking.md` is a living per-Moblin-group completeness map. |

⚠️ Phase 4 is the one outright stale status flag. The declared line said
"Not started" but every exit criterion was met before this audit. The
status line is updated alongside this document.

---

## Phase-7-dependent UI phases (12–27)

All phases in this band sit downstream of the Phase 7 `Panel` chassis.
None of them have any code in `senders/android/ui/` today — the
"reimplement guides" column tracks the step-by-step recipes that PR #1
landed.

| Phase | Declared | Actual | Reimplement guide | Evidence |
|---|---|---|---|---|
| **12** Capture preview | `[ ] Not started` | `[ ] Not started` | `PHASE-12-reimplement-instructions.md` | No `CapturePreview` component, no `mock-source-label` / `mock-active` properties on `pages/casting_page.slint`. |
| **13** Status badges row | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-13-reimplement-instructions.md` | `components/status_badges.slint` exports `StatusBadgesRow` + internal `Badge`; `main.slint` instantiates it above `CastControlBar`. |
| **14** Audio capture controls | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-14-reimplement-instructions.md` | `pages/audio_page.slint`; `Panel` enum has `audio` variant. |
| **15** Camera capture controls | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-15-reimplement-instructions.md` | `pages/camera_page.slint` exists, `bridge.slint` has `Panel.camera`. |
| **16** Bitrate quality presets | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-16-reimplement-instructions.md` | `pages/bitrate_presets_page.slint` and `pages/bitrate_preset_edit_page.slint` implemented. |
| **17** Quick-action customisation | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-17-reimplement-instructions.md` | `pages/quick_actions_page.slint` exists and implements `QuickActionsPage`, added variant to `bridge.slint` enum `Panel`. |
| **18** Privacy / lifecycle modes | `[ ] Not started` | `[ ] Not started` | `PHASE-18-reimplement-instructions.md` | No `LockOverlay` / `StealthOverlay` / `SnapshotCountdown` siblings in `main.slint`. |
| **19** Settings backup / reset | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-19-reimplement-instructions.md` | `pages/backup_reset_page.slint`, shared `ConfirmDialog` component. |
| **20** Cast history | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-20-reimplement-instructions.md` | `pages/cast_history_page.slint` and `pages/cast_history_detail_page.slint` exist. |
| **21** Help & support | `[ ] Not started` | `[ ] Not started` | `PHASE-21-reimplement-instructions.md` | No `pages/about_page.slint` / `version_history_page.slint` / `attributions_page.slint` / `help_page.slint`; the inline `ABOUT` section in `FullSettingsPage` is still the only About surface. |
| **22** Network interfaces / Wi-Fi Aware | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-22-reimplement-instructions.md` | `pages/network_page.slint` + `NetworkInterface` struct in `bridge.slint` |
| **23** Local recording | `[ ] Not started — blocked by Rust recording capability for live data, but UI placeholder is unblocked` | `[ ] Not started` | `PHASE-23-reimplement-instructions.md` | No `pages/recording_page.slint`. |
| **24** Pairing QR + receiver management | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-24-reimplement-instructions.md` | `pages/pairing_page.slint` and `pages/receiver_rename_page.slint` exist; QR placeholder and context menu components exist. |
| **25** Macros / action chains | `[x] Complete (UI-only)` | `[x] Complete (UI-only)` | `PHASE-25-reimplement-instructions.md` | `pages/macros_page.slint` / `macro_edit_page.slint` exist; `Macro` / `MacroStep` structs in `bridge.slint`. |
| **26** Debug log viewer | `[ ] Not started` | `[ ] Not started` | `PHASE-26-reimplement-instructions.md` | No `pages/debug_log_page.slint` / `debug_video_page.slint`; the legacy `Show debug panel` toggle in `FullSettingsPage` is still the only debug surface. |
| **27** Utility components backlog | `[~] Ongoing — pull from this list when a downstream phase needs a util` | `[~] Ongoing` | `PHASE-27-reimplement-instructions.md` | No utilities extracted yet; the guide ships `IconAndText` + `InfoBanner` + `ValueEditChip` as pre-defined extraction targets. |

---

## Speculative phases (28–48)

These ship spec docs only — no reimplement guides, no code. Most sit
downstream of broadcast / streaming / scenes architectural decisions
that have not been settled, so guides for them would be premature.

| Phases | Declared | Actual | Notes |
|---|---|---|---|
| **28** Chat overlay | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **29** Streaming destinations | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **30** Streams configuration / wizards | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **31** Streaming protocols | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **32** Ingests / servers | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **33–38** Scenes / widgets / effects / wizards / local overlays | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **39** Right-side broadcast HUD | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **40** Replay buffer | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **41** Streaming navigation overlay | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **42–43** Peripheral hardware | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **44–46** Apple targets / IAP / Moblin internals | `[ ] Not started` | `[ ] Not started` | Reference-only; live under `pages/_apple/` and never imported by `main.slint`. The `_apple/` directory does not yet exist on disk — guides are placeholders. |
| **47** Media player + browser | `[ ] Not started` | `[ ] Not started` | Spec only. |
| **48** Other broadcast deferrals | `[ ] Not started` | `[ ] Not started` | Spec only. |

---

## Summary

- **Phases 0–7 are functionally complete** in `senders/android/ui/`. Phases 5–7
  are the intended-end-state UI placeholders; Rust wiring is parked behind
  Phase 8.
- **Phase 8 is intentionally deferred.** The migration plan
  (`PHASE-8-bridge-migration-plan.md`) consolidates the per-phase "When
  Phase 8 reactivates" sections from every UI guide.
- **Phases 9, 10, 11** are ongoing meta-phases. None of them ships a single
  on/off implementation — they wrap or run alongside other phases.
- **Phases 13–17, 19, 22, 24, 25** are landed as UI-only (see the table
  above for the exact files). **Phases 12, 18, 20, 21, 23, 26** still
  have only their step-by-step reimplement guides (PR #1) and no code in
  `senders/android/ui/` — a reader can pick any of those remaining phases
  and follow the guide to land them.
- **Phases 28–48** ship spec only. Guides for these are deferred until the
  upstream architectural decisions land.

### Diff vs. previously declared status

| File | Old status line | New status line | Why |
|---|---|---|---|
| `PHASE-4-control-bar.md` | `[ ] Not started` | `[x] Complete` | Every exit criterion is met by `components/control_bar.slint` + `bridge.slint` + `main.slint`. The status line was never updated when the work merged. |
| `PHASE-5-status-overlay.md` | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only — Rust wiring deferred to Phase 8)` | Wording aligned with the canonical taxonomy. The substance is unchanged. |
| `PHASE-6-receiver-list.md` | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only — Rust wiring deferred to Phase 8)` | Same. |
| `PHASE-7-settings-pages.md` | `[ ] UI placeholder — no functionality` | `[x] Complete (UI-only — Rust wiring deferred to Phase 8)` | Same. |

No other per-phase status lines change in this PR — every other line is
already accurate.

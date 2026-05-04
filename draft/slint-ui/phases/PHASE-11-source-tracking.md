# Phase 11 — Source Tracking (reference)

> Cross-reference Moblin SwiftUI source groups against their FCast Slint
> equivalents. **Reference doc only — not implementation.** Use this as a
> completeness check at the end of each milestone, not as a per-file porting guide.

**Status:** `[ ] Reference — update as phases complete`
**Purpose:** Verify nothing was missed and confirm deliberate defers.
**Authoritative source:** `draft/slint-ui/futures/NOT-APPLICABLE.md`. This doc is
a higher-level rollup, kept in sync by hand whenever a phase ships.

**Related files:**
- `draft/slint-ui/analysis/summary.md` — file counts and SwiftUI pattern counts by group
- `draft/slint-ui/analysis/moblin-swiftui-inventory.csv` — per-file pattern breakdown
- `draft/slint-ui/futures/NOT-APPLICABLE.md` — per-file applicability triage (279 entries)

---

## Roadmap rollup

| Phase | What it builds | Moblin source group(s) | UI-only? |
|---|---|---|---|
| 0 | Audit + analysis tooling | (all) | n/a (research) |
| 1 | Module split (`main.slint` → pages/) | `MainView.swift` | n/a (refactor) |
| 2 | Theme tokens (`theme.slint`) | `Common/`, scattered colors | n/a (refactor) |
| 3 | Reusable components (buttons / settings rows) | `View/Utils/` (36 files) | yes |
| 4 | Control bar + quick actions | `View/ControlBar/` (23 files) | yes |
| 5 | Status overlay | `View/Stream/` (subset) | yes |
| 6 | Receiver list + discovery UX | `View/Settings/Streams/` (subset) | yes |
| 7 | Settings root + panel routing | `View/Settings/` (FCast-relevant subset) | yes |
| 8 | Rust bridge wiring | (cross-cutting) | **deferred** |
| 9 | Localization | (all UI files) | yes |
| 10 | UI validation (build + visual) | n/a | yes |
| 11 | This document | n/a | n/a |
| 12 | Capture preview placeholder | `View/Stream/StreamView` | yes |
| 13 | Status badges row (battery / thermal) | `View/ControlBar/Battery,Thermal*` | yes |
| 14 | Audio capture controls placeholder | `View/Settings/Audio/`, `QuickButtonMicView` | yes |
| 15 | Camera capture controls placeholder | `View/Settings/Camera/` (8 files) | yes |
| 16 | Bitrate & quality presets placeholder | `View/Settings/BitratePresets/`, `QuickButtonBitrateView` | yes |
| 17 | Quick-action customization placeholder | `View/Settings/Display/QuickButtons/` | yes |
| 18 | Privacy & lifecycle modes | `View/Main/{Lock,Stealth,SnapshotCountdown}` | yes |
| 19 | Settings backup & reset | `View/Settings/{ImportExport,Reset}/` | yes |
| 20 | Cast history | `View/Settings/StreamingHistory/` | yes |
| 21 | Help & support | `View/Settings/{About,HelpAndSupport}/` | yes |
| 22 | Network interface & Wi-Fi Aware | `Display/NetworkInterfaceNames/`, `WiFiAware/` | yes |
| 23 | Local recording placeholder | `View/Settings/Recordings/` | yes |
| 24 | Pairing QR & receiver management | `Utils/{QrCodeImage,ContextMenu,SwipeLeftTo,NameEdit}` | yes |
| 25 | Macros & action chains placeholder | `View/Settings/Macros/`, `QuickButtonMacrosView` | yes |
| 26 | Debug log viewer placeholder | `View/Settings/Debug/{Log,Video}` | yes |
| 27 | Reusable utils backlog | `View/Utils/` (remaining) | yes |

> **Pattern counts in `analysis/summary.md` are SwiftUI matches in the Moblin
> source** (e.g. `Button ×721`, `Form ×300`). They quantify the size of the
> original codebase, not the Slint port. Moblin-style counts do not apply to
> Slint, where model-driven `for` loops and shared global tokens collapse
> hundreds of repeated SwiftUI patterns into a handful of components.

---

## Deliberate omissions

The following Moblin source groups are **explicitly excluded** (per user direction
and `futures/NOT-APPLICABLE.md`):

| Excluded category | File count | Reason |
|---|---|---|
| Chat / Twitch / Kick / YouTube | 21 | No FCast chat surface |
| Navigation widgets | 1 | No FCast navigation overlay |
| Replay buffer | 2 | No FCast replay surface |
| Right-side overlays | 13 | No FCast right-side overlay |
| Streaming-platform configs (RTMP/SRT/RIST/WHIP/SRTLA/Ingest) | 59 | Wrong protocol — FCast uses its own TCP protocol |
| Scene / widget editor | 52 | No FCast scene system |
| Peripheral hardware (DJI/GoPro/Tesla/cat printers/cameras) | 22 | Out of FCast hardware scope |
| iOS / watchOS / Mac targets | 13 | Android-only |
| StoreKit / IAP | 1 | No paid features |
| Moblink / OpenAI / TTS | 4 | Moblin-internal services |

Total excluded: **~188 files (67%)**. See `futures/NOT-APPLICABLE.md` for per-file detail.

---

## Status by group (update as phases ship)

| Moblin group | Files | Slint target | Phase | Status |
|---|---|---|---|---|
| `View/MainView.swift` | 1 | `MainWindow` in `main.slint` | 1 | [x] Done |
| `View/Utils/` | 36 | components + Phase 27 backlog | 3, 27 | [x] Phase 3 done |
| `View/ControlBar/` | 23 | `cast_control_bar.slint` | 4 | [x] Done (UI-only) |
| `View/Stream/` | 23 | `status_overlay.slint`, Phase 12 capture preview | 5, 12 | [ ] |
| `View/Settings/` | 190 | `settings_page.slint` + Phases 14–26 | 7, 14–26 | [ ] |
| `View/Main/` | 4 | Phase 18 (lifecycle modes) | 18 | [ ] |
| `View/WebBrowser/` | 1 | Excluded | — | N/A |
| `View/ExternalDisplay/` | 1 | Excluded (no Android multi-display target) | — | N/A |

---

## Maintenance

When a phase ships:

1. Tick its row in the roadmap rollup above.
2. Update the corresponding rows in `futures/NOT-APPLICABLE.md` from
   `applicable-future` → `applicable`.
3. If a phase is split or merged, update both the rollup and the per-phase
   files; do not let this doc and `NOT-APPLICABLE.md` drift.

---

## Slint best practices applied here

- **A roadmap rollup is not a porting checklist.** Use this for completeness
  audits at milestone boundaries; use the per-phase docs for actual implementation.
- **Excluded groups stay excluded.** When a Moblin file looks tempting (e.g.
  `BitratePresetsView`), check `futures/NOT-APPLICABLE.md` first — many
  Moblin views are bait, because they reference RTMP/SRT plumbing that doesn't
  exist in FCast.

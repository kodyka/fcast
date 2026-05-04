# FCast Slint UI — Futures (applicability of Moblin UI)

This directory triages **every Moblin SwiftUI view file** in
`draft/moblin-ui/Moblin/View/` (279 files + `MainView.swift`) for whether it
applies to the FCast Android sender Slint UI.

The companion `phases/` directory contains the **executable plan**
(Phases 0–48) for porting the applicable subset, surfacing every
applicable-future / deferred feature as a UI-only placeholder, **and**
(per the user's expanded directive) surfacing the previously-excluded
Moblin categories as UI-only placeholders too. This `futures/` directory
is the **reference / triage record** that documents the original
applicability tagging, why each category was tagged that way, and how each
entry now maps to a phase — so future contributors don't ask "did you miss
`XYZView.swift`?".

## Files

| File | Purpose |
|---|---|
| [`README.md`](README.md) | This file — overview and exclusion taxonomy. |
| [`NOT-APPLICABLE.md`](NOT-APPLICABLE.md) | Per-file table marking each Moblin view as applicable / not applicable / deferred, with a one-line reason and a target phase reference. |

## Applicability legend

Each Moblin view file is tagged with one of:

| Tag | Meaning |
|---|---|
| **applicable** | Has an FCast sender equivalent and is already covered in `phases/` (Phase 0–4). |
| **applicable → Phase N** | Promoted into a UI-only placeholder phase (5–48). The placeholder ships **UI without functionality**; Rust wiring is parked in Phase 8. |
| **not-applicable — chat** | Streamer chat (Twitch/Kick/YouTube IRC, chat overlays, chat bots, TTS). FCast is a media-cast protocol with no chat surface. |
| **not-applicable — navigation** | Streamer-broadcasting navigation overlay (location/route HUD). Not relevant to a cast remote. |
| **not-applicable — replay** | Live-stream instant replay (rewind during broadcast). Not applicable to a one-way cast sender. |
| **not-applicable — right-overlay** | Moblin's right-side broadcast HUD layout (beauty/face/pixellate/whirlpool/scene selector). The layout is excluded; *individual* generic widgets inside (e.g. `AudioLevelView`, `SegmentedPicker`) are tagged separately. |
| **not-applicable — streaming-platform** | RTMP / SRT / RIST / WHIP / RTSP / SRTLA / Belabox / Twitch / Kick / YouTube / Soop / OBS-remote ingest, server, and platform configuration. FCast uses its own binary protocol; these are unrelated. |
| **not-applicable — scene-widget** | Moblin scenes / widgets / overlays editor (alerts, scoreboards, slideshows, VTubers, PNGTubers, wheel-of-luck, bingo, browser-source widgets, video-source widgets, effects). Editorial broadcast tooling, not applicable to a cast remote. |
| **not-applicable — peripheral-hardware** | External hardware peripherals: DJI gimbals/devices, GoPro cameras, Tesla, BlackShark phone coolers, Cat printers, gimbals, selfie sticks, workout devices, game controllers. Out of scope for FCast sender. |
| **not-applicable — ios-target** | iOS/watchOS/macOS-specific targets: Apple Watch app, Mac key-press, external display, iOS deep-link creator. Android sender doesn't have these targets. |
| **not-applicable — iap** | In-app purchase / store. FCast Android sender does not monetize. |
| **not-applicable — moblin-internal** | Moblin-specific feature (Moblink relay, OpenAI integration, MetalPetal effects, etc.) with no FCast analogue. |
| **deferred → Phase N** | Has potential FCast applicability but blocked on Rust media-graph capability that doesn't exist yet (camera capture, local recording, Wi-Fi Aware peer discovery). Promoted as a **UI-only placeholder** in the listed phase; functionality lands when Rust capability ships. |

## UI-only roadmap (Phases 5–48)

The user direction *"create a UI without functionality — placeholders for all
applicable-future / deferred entries"* has been turned into a 44-phase plan
(Phases 5–48). Every entry in `NOT-APPLICABLE.md` is now mapped to a target
phase under `phases/`, regardless of original applicability tag.

- `applicable-future` and `deferred — needs-rust-capability` map to
  Phases 12–27.
- Previously-excluded categories (chat, navigation, replay, right-side
  overlays, streaming-platform, scene-widget, peripheral-hardware,
  ios-target, iap, moblin-internal, mediaplayer, broadcast deferrals)
  map to Phases 28–48. Reference-only Apple-target / IAP / Moblink /
  OpenAI surfaces live under `senders/android/ui/pages/_apple/` and are
  never imported by `main.slint`.

The mapping is in `phases/PHASE-11-source-tracking.md` and the per-phase
files. Each placeholder phase ships:

- A `.slint` page or component (or, for reference-only phases, a
  `_apple/...` placeholder).
- Inline `in-out property <[T]> mock-...` stub data.
- Local-only state mutations (no Rust callbacks).
- A `Panel` enum variant (where it's a sub-page).
- Documented "What's NOT in this phase" section parking real wiring in
  Phase 8 (deferred).
- A Moblin SwiftUI source excerpt + Slint primitive mapping section
  (Phases 12–48).

## Exclusion history (now relaxed for UI-only placeholders)

The original v1 scope explicitly excluded **four** Moblin UI categories
from the FCast port:

1. **chat** — all chat surfaces, including TTS / Talkback
2. **navigation** — Moblin streaming-navigation overlays
3. **replay** — live-stream instant replay
4. **right-side overlays** — Moblin's broadcast right HUD layout

The v2 user direction ("add Moblin categories chat / navigation / replay /
right-side overlays and etc to phases. We need to create a UI without
functionality.") **lifts those exclusions for the placeholder UI** while
preserving the no-functionality discipline. Phases 28–48 now ship visible
placeholders for every previously-excluded category. The `NOT-APPLICABLE.md`
table still records the original tagging plus a phase mapping column.

Functionality remains permanently parked — these placeholders are visual
only. Reactivating any of them as functional features still requires a
separate scope-change decision.

## Rust wiring (post-UI)

Phase 8 is parked as a deferred placeholder. When the team is ready to
graduate Phases 5–7 + 12–48 from "UI-only" to "wired", Phase 8 will be
re-expanded with:

- `UiState` helper (`Weak<MainWindow>` only — no `Rc<VecModel>` in shared state).
- `upgrade_in_event_loop` discipline for cross-thread updates.
- `on_invoke_action(id)` → `match id` dispatch in `lib.rs`.
- `Bridge.devices: [string]` → `[ReceiverItem]` migration.
- Per-page model promotions: each `mock-*` inline `in-out property` becomes a
  `Bridge.<name>` property, set from Rust.

## How to use this directory

- **When triaging a new feature request**, check `NOT-APPLICABLE.md` first to
  see whether the relevant Moblin view was excluded by category. If it was,
  the request is either out of scope or requires a scope-change discussion.
- **When promoting a `deferred → Phase N` entry to functional**, the UI
  placeholder already exists; the work is reactivating Phase 8 for that
  specific feature and migrating the inline stub to a `Bridge.<name>`
  property.
- **Do not delete excluded entries** when scope changes. Mark them with a
  history note in `NOT-APPLICABLE.md` so the audit trail is preserved.

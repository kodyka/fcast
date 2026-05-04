# Phase 44 — iOS / watchOS / Mac Targets Placeholder

> **REFERENCE-ONLY UI.** Apple Watch / Mac key-press / external display /
> deep-link / shortcut surfaces — rendered for visual completeness so the
> design language is documented, but **never instantiated** by the Android
> `MainWindow`. Apple-platform code paths simply don't run on FCast Android.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3
**Functional integration:** **Never wired.** These pages exist as design
artifacts only. They live in `senders/android/ui/pages/_apple/` (the
underscore prefix marks "do not import from MainWindow").

**Moblin source analogues** (13 files):
- `View/Watch/{ChatView,Padel,Snapshot,WatchView,SettingsView,…}.swift`
- `View/Mac/MacKeyPressView.swift`
- `View/ExternalDisplay/ExternalDisplayChatView.swift`
- `View/Settings/DeepLinkCreator/{DeepLinkCreatorSettingsView,DeepLinkCreatorStreamSettingsView,…}.swift`
- `View/Settings/Keyboard/KeyboardSettingsView.swift`
- `View/Settings/AppleSettings/{AppleSettingsTabView,AppleSettingsView}.swift`

**Files to add (under `senders/android/ui/pages/_apple/`):**
- `apple_watch_root_page.slint`
- `apple_watch_chat_page.slint`
- `apple_watch_settings_page.slint`
- `apple_watch_padel_page.slint`
- `apple_watch_snapshot_page.slint`
- `mac_keypress_page.slint`
- `external_display_chat_page.slint`
- `deep_link_creator_root_page.slint`
- `deep_link_creator_stream_page.slint`
- `keyboard_settings_page.slint`
- `apple_settings_tab_page.slint`

---

## Goal

Render the Apple-platform surfaces as visible Slint files so a future
"unify the design language across platforms" review can find them — but
keep them off the Android `MainWindow` import graph.

---

## Why include these at all?

The user's directive is *"UI without functionality"*. Even when there's no
Android equivalent, having a placeholder lets a designer compare layouts
across platforms and lets a future audit catch missing translations
(Phase 9). Marking them with the `_apple/` prefix keeps the Android build
exactly as it is today — none of these `.slint` files are imported by
`main.slint`.

---

## Tasks

### 44-A — Directory + README

- [ ] Create `senders/android/ui/pages/_apple/`.
- [ ] Add `senders/android/ui/pages/_apple/README.md` explaining:
  - These are reference-only; do not import from `main.slint`.
  - They survive the build because nothing imports them.
  - Phase 8 audit greps must include
    `! grep -r "from \"pages/_apple/" senders/android/ui/`.

### 44-B — `apple_watch_root_page.slint`

- [ ] Apple Watch sized shell (180×220 logical px). Renders four
  `Rectangle`s in a 2×2 grid for: Cast / Chat / Replay / Padel.

### 44-C — `apple_watch_chat_page.slint`

- [ ] Mini chat list — same `ChatMessage` struct as Phase 28, smaller
  font sizes.

### 44-D — `apple_watch_settings_page.slint`

- [ ] Toggle list (mute, send haptics).

### 44-E — `apple_watch_padel_page.slint`

- [ ] Score-buttons grid (Player 1 ▲▼, Player 2 ▲▼, period cycler).

### 44-F — `apple_watch_snapshot_page.slint`

- [ ] One big "Take snapshot" button + 3 thumbnails of recent.

### 44-G — `mac_keypress_page.slint`

- [ ] List of keyboard mapping rows (`mock-mappings: [{ key, action }]`).

### 44-H — `external_display_chat_page.slint`

- [ ] Same chat list as Phase 28, scaled up for an external monitor.

### 44-I — `deep_link_creator_root_page.slint`

- [ ] Form for building `moblin://...` URIs:
  - Stream URL field
  - Stream key field
  - Auto-start toggle
  - Generated URI preview (computed via string interpolation)

### 44-J — `deep_link_creator_stream_page.slint`

- [ ] Per-stream sub-form: name, protocol picker, advanced toggles.

### 44-K — `keyboard_settings_page.slint`

- [ ] Hardware keyboard shortcut mapping list.

### 44-L — `apple_settings_tab_page.slint`

- [ ] iOS-style tab bar with 5 tabs: Stream / Settings / Help / About /
  Pro.

---

## Exit criteria

1. All 11 files exist under `senders/android/ui/pages/_apple/`.
2. `senders/android/ui/main.slint` imports **none** of them.
3. Audit grep `grep -rE 'from "pages/_apple/' senders/android/ui/`
   returns 0 matches.
4. The files compile **standalone** in `slint-viewer` (so they're not
   broken; just unwired).

---

## What's NOT in this phase

- Any Android instantiation of these surfaces.
- Real WatchOS / iOS / Mac functionality.

---

## Slint best practices applied here

- **Reference-only files in a prefixed directory** is the standard way
  to keep design artifacts in-tree without polluting the runtime
  graph. Phase 8 audit gates this with a grep.
- **`@tr("...")` is OPT-IN here** — these strings will be skipped by
  Phase 9's localization sweep because they live under `_apple/`.

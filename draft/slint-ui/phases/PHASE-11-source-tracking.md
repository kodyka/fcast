# Phase 11 — Source Tracking by Moblin Group

> Cross-reference every Moblin SwiftUI source group against its FCast Slint equivalent.
> Use this as a completeness check at the end of the migration, not as a per-file porting guide.
> Reference data: `draft/slint-ui/analysis/summary.md` (279 files, 8 groups).

**Status:** `[ ] Reference — update as phases complete`
**Purpose:** Verify nothing was missed and confirm deliberate defers.
**Related files:**
- `draft/slint-ui/analysis/summary.md` — file counts and SwiftUI pattern counts by group
- `draft/slint-ui/analysis/moblin-swiftui-inventory.csv` — per-file pattern breakdown

---

## Group mapping table

| Moblin group | Files | FCast Slint target | Phase | Status |
|---|---|---|---|---|
| `View/Utils` | 36 | `ui/components/buttons.slint`, `ui/components/settings_rows.slint` | 3 | [ ] |
| `View/ControlBar` | 23 | `ui/components/control_bar.slint`; `CastButton` lives in `casting_page.slint` (Phase 4-E) | 4 | [ ] |
| `View/Stream` | 23 | `ui/components/status_overlay.slint`, `casting_page.slint` | 5 | [ ] |
| `View/Settings` | 190 | `ui/pages/settings_page.slint` — FCast subset only | 7 | [ ] |
| `View/MainView.swift` | 1 | `main.slint` `MainWindow` routing | 1 | [ ] |
| `View/Main` | 4 | Deferred — no FCast equivalent | — | Defer |
| `View/WebBrowser` | 1 | Deferred — no FCast browser target | — | Defer |
| `View/ExternalDisplay` | 1 | Deferred — no Android multi-display target | — | Defer |

> **Pattern counts in this document are SwiftUI matches in the Moblin source** (e.g.
> `Button ×721`, `Form ×300`). They quantify the size of the original codebase, not
> the size of the Slint port. The FCast port re-implements the equivalent behaviours
> with far fewer Slint elements thanks to model-driven `for` loops, `@children`-based
> container components, and shared global tokens.

---

## Group detail: `View/Utils` (36 files → Phase 3)

Top Moblin patterns: `Button` ×40, `Picker` ×20, `HStack` ×20, `Form` ×15, `TextField` ×12.

| Moblin file | FCast Slint equivalent | Status |
|---|---|---|
| `ButtonView.swift` | `PrimaryButton` in `buttons.slint` | [ ] |
| `BorderlessButtonView.swift` | `TextButton` in `buttons.slint` | [ ] |
| `AddButtonView.swift` | `PrimaryButton` (reuse, no separate add button) | [ ] |
| `CreateButtonView.swift` | `PrimaryButton` (reuse) | [ ] |
| `CloseToolbarView.swift` | `TextButton { label: "Done"; }` in panel headers | [ ] |
| `TextItemView.swift` | `SettingsTextRow` in `settings_rows.slint` | [ ] |
| `TextValueView.swift` | `SettingsValueRow` in `settings_rows.slint` | [ ] |
| `TextEditView.swift` | `LineEdit` from std-widgets (Phase 6-E manual IP row) | [ ] |
| `ValueEditView.swift` | Not needed yet — defer until numeric input is required | Defer |
| `NameEditView.swift` | Not needed yet — defer | Defer |
| `SliderView.swift` | `SettingsSliderRow` in `settings_rows.slint` | [ ] |
| `InlinePickerView.swift` | `ComboBox` from std-widgets or SDK pickers | [ ] |
| `IconAndTextView.swift` | Inline `HorizontalLayout` with `Image` + `Text` — no separate component needed | N/A |
| `HCenter.swift` | `HorizontalLayout { alignment: center; }` — no separate component needed | N/A |
| `StrokeModifier.swift` | `border-width` + `border-color` in `Rectangle` — no separate component needed | N/A |
| `QrCodeImageView.swift` | Defer — Rust generates QR image or uses Android intent; no Slint component needed | Defer |
| `VoicesView.swift` | Defer — TTS not in FCast scope | Defer |
| Swipe-left helpers | Not applicable — use explicit action buttons in Slint | N/A |

---

## Group detail: `View/ControlBar` (23 files → Phase 4)

Top Moblin patterns: `Button` ×721, `@ObservedObject` ×117, `HStack` ×55, `ForEach` ×45.

| Moblin concept | FCast Slint equivalent | Status |
|---|---|---|
| `ControlBarPortraitView.swift` | `CastControlBar` (portrait, bottom-pinned) | [ ] |
| `ControlBarLandscapeView.swift` | Deferred — landscape variant | Defer |
| `StreamButton.swift` | `CastButton` state machine in `casting_page.slint` (Phase 4-E) | [ ] |
| `QuickButtonsView.swift` | `for action in Bridge.quick-actions: QuickActionButton` loop | [ ] |
| `BatteryView.swift` | `StatusItem` pill in `StatusOverlay` (Phase 5) | [ ] |
| `ThermalStateSheetView.swift` | `StatusItem` severity pill — "thermal" severity | Defer |
| `QuickButton/**` panels | `Panel` enum routing → `FullSettingsPage` sections (Phase 7) | [ ] |

---

## Group detail: `View/Stream` (23 files → Phase 5)

Top Moblin patterns: `@ObservedObject` ×147, `Button` ×52, `HStack` ×46.

| Moblin concept | FCast Slint equivalent | Status |
|---|---|---|
| `StreamView.swift` | Not applicable — FCast does not show camera preview | N/A |
| `StreamOverlayView.swift` | `StatusOverlay` component | [ ] |
| `StreamGridView.swift` | Not applicable | N/A |
| `CameraLevelView.swift` | Not applicable — no camera level indicator | N/A |
| `DrawOnStreamView.swift` | Not applicable | N/A |
| `Overlay/LeftOverlayView.swift` | Left-side `StatusOverlay` items | [ ] |
| `Overlay/RightOverlayView.swift` | Deferred — right side status not needed yet | Defer |
| `Overlay/ChatView.swift` | Not applicable — no chat in FCast | N/A |
| `Overlay/NavigationOverlayView.swift` | Not applicable | N/A |

---

## Group detail: `View/Settings` (190 files → Phase 7)

Top Moblin patterns: `Form` ×300, `Picker` ×296, `NavigationLink` ×293, `Toggle` ×271.

**FCast porting strategy:** Port sections by topic. Port only topics FCast supports.

| Moblin settings topic | FCast support? | FCast Slint target | Status |
|---|---|---|---|
| Stream destinations (RTMP/SRT/RIST/WHIP) | No | Omit | N/A |
| Scenes and widgets | No | Omit | N/A |
| Chat (Twitch/Kick/YouTube) | No | Omit | N/A |
| Camera settings | No (FCast captures screen, not camera) | Omit | N/A |
| Audio settings | Partial future | Defer | Defer |
| Video resolution / framerate | Yes | `SettingsSection` in `FullSettingsPage` Phase 7-F | [ ] |
| Receivers / discovery | Yes | `SettingsSection` in `FullSettingsPage` Phase 7-E | [ ] |
| Codec / encoder | Yes (debug) | `SettingsSection` in `FullSettingsPage` Phase 7-G | [ ] |
| About / version | Yes | `SettingsSection` in `FullSettingsPage` Phase 7-H | [ ] |
| Debug | Yes | `DebugPage` / `CodecTestPage` Phase 7-G | [ ] |
| GoPro / DJI / Tesla / IoT | No | Omit | N/A |
| Remote control | No | Omit | N/A |
| Watch / widget targets | No | Omit | N/A |
| Location | No | Omit | N/A |
| Store / purchases | No | Omit | N/A |

---

## Group detail: `View/Main` (4 files → Deferred)

| Moblin file | FCast need? | Decision |
|---|---|---|
| `LockView.swift` | No — Android handles device lock | N/A |
| `StealthModeView.swift` | No | N/A |
| `SnapshotView.swift` | No — no photo capture in FCast sender | N/A |
| Gesture handling | Partial — Android back button handled via Slint | Phase 10-H |

---

## Group detail: `View/WebBrowser` (1 file → Deferred)

| Moblin file | FCast need? | Decision |
|---|---|---|
| `WebBrowserView.swift` | No — FCast sender does not embed a browser | Omit |

---

## Group detail: `View/ExternalDisplay` (1 file → Deferred)

| Moblin file | FCast need? | Decision |
|---|---|---|
| `ExternalDisplayView.swift` | No — Android sender has no second display target | Omit |

---

## Completion checklist

Mark each group complete when its corresponding phase is fully done:

- [ ] `View/Utils` — Phase 3 complete
- [ ] `View/ControlBar` — Phase 4 complete
- [ ] `View/Stream` — Phase 5 complete
- [ ] `View/Settings` (FCast subset) — Phase 7 complete
- [ ] `View/MainView.swift` — Phase 1 complete
- [x] `View/Main` — **Deferred by design**
- [x] `View/WebBrowser` — **Omitted by design**
- [x] `View/ExternalDisplay` — **Omitted by design**

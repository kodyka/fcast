# TODO: migrate Moblin UI reference to FCast Slint UI

Goal: use `draft/moblin-ui/` as reference material and create Slint UI components under a future `slint-ui`/`senders/android/ui` structure. This is a reimplementation plan, not a direct SwiftUI copy.

## Phase 0 — Baseline and safety

- [ ] Keep `draft/moblin/`, `draft/moblin-ui/`, and `draft/slint/` as research-only folders unless explicitly deciding to commit docs.
- [ ] Keep FCast Android sender UI source of truth at `senders/android/ui/main.slint` until a migration branch is created.
- [ ] Confirm target Slint version from workspace lockfile before using new features such as MCP or experimental layouts.
- [ ] Decide whether final folder should be `draft/slint-ui` only, or committed app files under `senders/android/ui/`.

## Phase 1 — Slint UI foundation

- [ ] Split current `senders/android/ui/main.slint` into modules:
  - [ ] `ui/main.slint`
  - [ ] `ui/theme.slint`
  - [ ] `ui/components/buttons.slint`
  - [ ] `ui/components/settings_rows.slint`
  - [ ] `ui/components/control_bar.slint`
  - [ ] `ui/components/status_overlay.slint`
  - [ ] `ui/pages/connect_page.slint`
  - [ ] `ui/pages/casting_page.slint`
  - [ ] `ui/pages/settings_page.slint`
  - [ ] `ui/pages/debug_page.slint`
- [ ] Preserve existing `Bridge` callbacks and properties so Rust code keeps compiling.
- [ ] Add enums/structs for UI routing and data models:
  - [ ] `Panel`
  - [ ] `QuickAction`
  - [ ] `StatusItem`
  - [ ] `ReceiverItem`
  - [ ] `SettingsRow`
- [ ] Replace hard-coded repeated debug buttons with model-backed quick actions only after existing behavior is covered.

## Phase 2 — Port reusable Moblin `View/Utils` patterns

Source: `draft/moblin-ui/Moblin/View/Utils`.

- [ ] Port button styles:
  - [ ] `ButtonView.swift` → `PrimaryButton`
  - [ ] `BorderlessButtonView.swift` → `TextButton`
  - [ ] `AddButtonView.swift` / `CreateButtonView.swift` → `CreateActionButton`
  - [ ] `CloseToolbarView.swift` → `PanelHeader`
- [ ] Port settings rows/editors:
  - [ ] `TextItemView.swift` → `SettingsTextRow`
  - [ ] `TextValueView.swift` → `SettingsValueRow`
  - [ ] `TextEditView.swift` → `TextEditPanel`
  - [ ] `ValueEditView.swift` → `NumberEditPanel`
  - [ ] `NameEditView.swift` → `NameEditPanel`
  - [ ] `SliderView.swift` → `SettingsSliderRow`
  - [ ] `InlinePickerView.swift` → `SegmentedChoiceRow`
- [ ] Port layout helpers:
  - [ ] `IconAndTextView.swift` → `IconText`
  - [ ] `HCenter.swift` → use Slint `HorizontalLayout` alignment or helper component
  - [ ] `StrokeModifier.swift` → border properties in reusable rectangles
- [ ] Decide replacements for unsupported/feature-specific utilities:
  - [ ] `QrCodeImageView.swift` → Rust-generated image or asset binding
  - [ ] `VoicesView.swift` → defer until text-to-speech exists
  - [ ] swipe-left helpers → replace with explicit action buttons or Slint gesture/touch pattern

## Phase 3 — Create FCast settings pages inspired by Moblin

Source: `draft/moblin-ui/Moblin/View/Settings`.

- [ ] Create `SettingsPage` with section rows and page routing.
- [ ] Create first FCast-specific settings sections:
  - [ ] Receiver / discovery settings
  - [ ] Video quality settings: resolution, max framerate, bitrate when supported
  - [ ] Codec settings/debug: H.264 MediaCodec test, future codec probes
  - [ ] Network settings/status
  - [ ] About/debug logs
- [ ] Keep unsupported Moblin settings as backlog, not UI stubs:
  - [ ] chat/Twitch/Kick/YouTube
  - [ ] scenes/widgets
  - [ ] ingests/RTMP/SRT/RIST/WHIP server configs
  - [ ] GoPro/DJI/Tesla/cat printers/workout devices
  - [ ] watch/widget targets
- [ ] Convert Moblin `NavigationLink` structure into explicit Slint `Panel` enum routing.
- [ ] Convert each SwiftUI `Form`/`Section` into `SettingsSection` + `SettingsRow` components.

## Phase 4 — Port control bar concept

Source: `draft/moblin-ui/Moblin/View/ControlBar`.

- [ ] Create `CastControlBar` with portrait/landscape variants.
- [ ] Create `CastButton` from Moblin `StreamButton` concept:
  - [ ] disconnected → connect/scan action
  - [ ] ready → start casting
  - [ ] casting → stop casting
  - [ ] waiting/connecting → cancel
- [ ] Create `QuickActionButton` model for actions:
  - [ ] scan QR
  - [ ] receiver list
  - [ ] debug panel
  - [ ] H.264 encoder test
  - [ ] migrated server tests
  - [ ] settings
- [ ] Add status badges inspired by `BatteryView` and overlay status views:
  - [ ] receiver count
  - [ ] app state
  - [ ] encoder result
  - [ ] network/address

## Phase 5 — Port stream/status overlay concept

Source: `draft/moblin-ui/Moblin/View/Stream`.

- [ ] Create `StatusOverlay` component for casting screen.
- [ ] Show overlay items from a Rust-provided `StatusItem` model.
- [ ] Add debug log/status text scroll area using `ScrollView`.
- [ ] Add optional grid/camera-level equivalents only if FCast gets preview/camera data.
- [ ] Avoid porting chat/navigation/replay overlays until backend support exists.

## Phase 6 — Rust bridge and state model

- [ ] Create a small Rust `ui_state` module to map backend state to Slint globals/models.
- [ ] Keep Slint globals shallow; do not recreate Moblin's monolithic `Model`.
- [ ] Feed arrays through Slint model types for receivers, quick actions, and statuses.
- [ ] Wire new callbacks in `src/lib.rs` using weak handles to avoid callback ownership cycles.
- [ ] Persist only settings that FCast actually supports.

## Phase 7 — Localization and assets

- [ ] Replace hard-coded new strings with `@tr("...")`.
- [ ] Use `slint-tr-extractor` later if localization becomes required.
- [ ] Create `ui/images/` for icons/assets; avoid embedding base64.
- [ ] Decide whether Moblin's `Common/Localizable.xcstrings` is only reference material or source for translated copy.

## Phase 8 — Testing and debugging

- [ ] Run existing Android sender build/CI after each functional slice.
- [ ] Test UI on Android device/emulator for touch target sizes and orientation.
- [ ] Use `ListView` for long models to avoid performance issues.
- [ ] Enable Slint MCP only when runtime UI inspection is needed and the project Slint version supports it.
- [ ] Verify no binding loops or missing layout sizes from Slint compiler warnings.

## Phase 9 — Migration tracking by source group

- [ ] `Moblin/View/Utils` → reusable Slint components.
- [ ] `Moblin/View/MainView.swift` → extend `MainWindow` and router.
- [ ] `Moblin/View/ControlBar` → `CastControlBar`, `QuickActionButton`, `CastButton`.
- [ ] `Moblin/View/Stream` → `StatusOverlay` and casting page overlays.
- [ ] `Moblin/View/Settings` → selective FCast settings pages.
- [ ] `Moblin/View/Main` → lock/stealth/snapshot concepts only if FCast needs them.
- [ ] `Moblin/View/WebBrowser` → defer; not currently core to FCast sender.
- [ ] `Moblin/View/ExternalDisplay` → defer; Android sender has no matching target.

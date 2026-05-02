# Moblin SwiftUI UI draft

Source repository: `https://github.com/eerimoq/moblin`

This draft copies the Moblin main-app UI layer only, for study and possible migration into another app. The full cloned source is in `draft/moblin/`; the copied UI-only subset is in `draft/moblin-ui/`.

## What was copied

- `Moblin/View/` — 279 SwiftUI view files.
- `Common/Localizable.xcstrings` — localization strings used by UI text.
- `Moblin/InfoPlist.xcstrings` — main app Info.plist localization strings.
- `VIEW_FILES.md` — generated list of every copied `Moblin/View/**/*.swift` file.

This intentionally does not copy Moblin media backends, integrations, watch/widget targets, build settings, or Xcode project files.

## UI architecture summary

Moblin's UI is a SwiftUI tree with a single central `Model` object injected through SwiftUI environment.

### App launch

`Moblin/MoblinApp.swift` creates the global model and injects it into `MainView`:

```swift
@main
struct MoblinApp: App {
    @StateObject var model: Model

    init() {
        MoblinApp.globalModel = Model()
        _model = StateObject(wrappedValue: MoblinApp.globalModel!)
    }

    var body: some Scene {
        WindowGroup {
            MainView(...)
                .environmentObject(model)
        }
    }
}
```

Important source lines in the clone:

- `draft/moblin/Moblin/MoblinApp.swift:3-31`
- `draft/moblin/Moblin/View/MainView.swift:115-160`
- `draft/moblin/Moblin/View/Settings/SettingsView.swift:5-90`

### Root UI

`Moblin/View/MainView.swift` is the main coordinator. It owns or renders:

- live stream/camera preview area;
- settings and quick-button panels;
- control bars for portrait/landscape;
- stream overlays;
- web browser view;
- lock screen, stealth mode, snapshot countdown, and gesture handling.

`MainView` reads `@EnvironmentObject var model: Model` in many nested views and switches on model state such as `showingPanel`, `panelHidden`, `showDrawOnStream`, stream orientation, and live state.

### Settings UI

`Moblin/View/Settings/SettingsView.swift` is the root settings form. It depends on both:

```swift
@EnvironmentObject var model: Model
@ObservedObject var database: Database
```

It branches into feature-specific settings modules such as streams, scenes, chat, display, camera, audio, location, ingests, remote control, recordings, store, watch, and debug.

### Control bar and quick buttons

`Moblin/View/ControlBar/` contains the live control bar and action buttons:

- `ControlBarPortraitView.swift`
- `ControlBarLandscapeView.swift`
- `QuickButtonsView.swift`
- `StreamButton.swift`
- `BatteryView.swift`
- `ThermalStateSheetView.swift`
- `QuickButton/**` feature panels

These views are tightly coupled to live stream state, quick-button database settings, mic state, OBS state, LUTs, chat, scene widgets, GoPro/DJI, and stream switching.

### Stream overlays

`Moblin/View/Stream/` contains the live video display UI:

- `StreamView.swift`
- `StreamOverlayView.swift`
- `StreamGridView.swift`
- `CameraLevelView.swift`
- `DrawOnStreamView.swift`
- `Overlay/**` left/right/chat/debug/navigation overlays

These views depend heavily on `Model`, `Database`, chat providers, stream state, camera state, location, recording, widgets, navigation, replay, and status providers.

### Reusable UI components

`Moblin/View/Utils/` contains reusable SwiftUI controls: buttons, editors, sliders, QR code image view, text/value rows, position/size editors, URL list UI, swipe helpers, voice selector, and similar generic components. These are the most reusable pieces if migrating incrementally.

## Key non-UI dependencies required to compile copied views

The copied UI does not compile standalone. The main missing dependencies are:

1. `Model`
   - Source: `draft/moblin/Moblin/Various/Model/Model.swift`
   - Central state manager: `final class Model: NSObject, ObservableObject, @unchecked Sendable`.
   - Provides UI state, stream state, camera controls, overlays, web browser controller, database, store, chat, widgets, recording, quick-button state, and many actions.

2. `Database`
   - Source: `draft/moblin/Moblin/Various/Settings/Settings.swift`
   - Defined as `class Database: Codable, ObservableObject`.
   - Holds streams, scenes, widgets, chat settings, display settings, quick buttons, camera/audio settings, ingests, watch settings, debug settings, and many feature-specific settings models.

3. Supporting settings and state models
   - Many `Settings*` types in `Moblin/Various/Settings/Settings.swift` and related files.
   - Camera, stream, chat, overlay, scene, widget, ingest, OBS, GoPro, DJI, store, and media player types.

4. Apple frameworks
   - SwiftUI, AVFoundation, WebKit, UIKit, CoreLocation, Photos, StoreKit, and other iOS frameworks appear across the UI.

5. Swift package dependencies used by UI
   - `WrappingHStack` — wrapping tag/button layouts.
   - `SDWebImageSwiftUI` — async images/emotes/chat assets.
   - `Collections` — used by some chat/overlay views.
   - `MetalPetal` — used by LUT/video effect related UI and previews.

6. Localization
   - `Common/Localizable.xcstrings` is required for localized `String(localized:)` and SwiftUI text labels.

## Migration TODO

### Phase 1 — Decide scope

- Choose one target UI slice first. Recommended order:
  1. `View/Utils` reusable controls.
  2. Simple `Settings` pages that only edit plain settings data.
  3. Control bar shell.
  4. Stream overlay shell.
  5. Full `MainView` last.
- Do not try to compile all 279 files at once unless porting most of Moblin's model/backend too.

### Phase 2 — Create a compatibility state layer

- Create an adapter or stub equivalent for `Model` with only properties/methods needed by the chosen views.
- Create a slim `Database` or settings store with the required `@Published` fields.
- Keep UI-facing state separate from media/streaming backend internals so the UI can be reused in the Android sender conceptually.

### Phase 3 — Untangle feature-specific views

For each copied SwiftUI file:

- List missing symbols from compiler errors.
- Classify each missing symbol as:
  - pure UI helper;
  - settings model;
  - app state;
  - media/backend integration;
  - third-party package dependency.
- Port pure UI helpers first.
- Replace backend integrations with protocols or no-op adapters until the Android sender has matching functionality.

### Phase 4 — Map Moblin UI concepts to FCast Android sender

- `MainView` preview/stream area → FCast sender preview/capture status.
- `ControlBar` → start/stop casting, scan QR, receiver selection, debug/test buttons.
- `SettingsView` → FCast sender settings/debug panels.
- `StreamOverlay` → optional status overlay for connection, bitrate, encoder, battery, logs.
- Moblin stream destination settings → FCast receiver/cast destination settings.

### Phase 5 — Reimplement in target UI framework

The FCast Android sender currently uses Slint, not SwiftUI. Direct SwiftUI files cannot be used in the Rust/Android sender without an iOS SwiftUI host. For FCast Android, treat this copied UI as design/reference material and port layouts/components into `senders/android/ui/main.slint` and Rust bridge callbacks.

Suggested Slint port order:

1. Control bar layout inspired by `ControlBarPortraitView.swift` / `ControlBarLandscapeView.swift`.
2. Reusable button styles inspired by `View/Utils/ButtonView.swift` and `BorderlessButtonView.swift`.
3. Settings navigation inspired by `SettingsView.swift`.
4. Status overlays inspired by `StreamOverlayView.swift` and `Stream/Overlay/*`.
5. Debug/codec test panels for Android-specific GStreamer/MediaCodec tests.

## File inventory

See `VIEW_FILES.md` for the complete generated list of copied UI files.

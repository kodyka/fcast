# Current FCast Android Slint notes

## Existing files

- UI entry point: `senders/android/ui/main.slint`
- Rust bridge: `senders/android/src/lib.rs`
- Build script: `senders/android/build.rs`
- Android sender crate: `senders/android/Cargo.toml`

## Existing Slint app shape

`main.slint` currently imports standard widgets plus shared mirroring UI helpers:

```slint
import { VerticalBox, Button, ListView, ScrollView, Spinner } from "std-widgets.slint";
import { Utils, VideoResolutionPicker, FrameratePicker } from "../../../sdk/mirroring_core/ui/common.slint";
```

The UI is state-driven by:

```slint
enum AppState {
    Disconnected,
    Connecting,
    SelectingSettings,
    WaitingForMedia,
    Casting,
}
```

The backend API is exposed through `export global Bridge` with properties and callbacks. This is the correct integration point for migrated UI concepts.

## Existing pages/components

- `ConnectView`
  - receiver list through `Bridge.devices`
  - QR scanning button
  - debug/migration/codec buttons under `Bridge.show-debug`
- `ConnectingView`
- `SelectingSettingsView`
  - video resolution and framerate pickers
- `WaitingForMediaView`
- `CastingView`
- `MainWindow`
  - conditional rendering based on `Bridge.app-state`

## Workspace Slint dependency

Root `Cargo.toml` currently pins Slint to a custom git fork:

```toml
slint = { git = "https://gitlab.futo.org/videostreaming/fcast-slint.git", rev = "f976d8c7958bddbba14a9f4632b8d3302cea96f6", default-features = false }
slint-build = { git = "https://gitlab.futo.org/videostreaming/fcast-slint.git", rev = "f976d8c7958bddbba14a9f4632b8d3302cea96f6" }
```

Before using newer Slint features from upstream docs, confirm the fork revision supports them. Keep initial migration to stable/basic Slint features already used in the app: globals, callbacks, properties, `VerticalBox`, `Button`, `ListView`, `ScrollView`, `Spinner`, `TouchArea`, conditional components, and standard widgets.

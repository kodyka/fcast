# Moblin UI — Applicability Triage

This document tags every file under `draft/moblin-ui/Moblin/View/` (279 files +
`MainView.swift`) for whether it applies to the FCast Android sender Slint UI.

See [`README.md`](README.md) for the legend and the four user-excluded
categories (chat / navigation / replay / right-side overlays).

> **Format**: each section corresponds to a top-level `Moblin/View/<Section>/`
> directory. Within a section, files are listed alphabetically, with a tag and
> a one-line reason. Tags use the legend in `README.md`.
>
> **Goal**: when someone asks "what about Moblin's `XYZView.swift`?" the answer
> should be one `grep` away.

---

## Promotion mapping (Phases 12–48)

Per the user's expanded direction (*"add Moblin categories chat / navigation /
replay / right-side overlays and etc to phases"* — v2), **every entry** in the
per-section tables below has been mapped to a UI-only placeholder phase under
`phases/`, regardless of original applicability tag. Reference-only Apple-target
/ IAP / Moblink / OpenAI surfaces live under
`senders/android/ui/pages/_apple/` and are never imported by `main.slint`.

### Applicable-future / deferred (Phases 12–27)

| Cluster | Source files (representative) | Target phase |
|---|---|---|
| Capture preview | `Stream/StreamView` | `PHASE-12-capture-preview.md` |
| Status badges (battery / thermal / network) | `ControlBar/{Battery,ThermalStateSheet}View` | `PHASE-13-status-badges-row.md` |
| Audio capture controls | `Settings/Audio/`, `ControlBar/QuickButton/QuickButtonMicView` | `PHASE-14-audio-capture-controls.md` |
| Camera capture controls | `Settings/Camera/{CameraSettings,CameraControls,MirrorFrontCamera,TapScreenToFocus,VideoStabilization,Zoom}` (8 files) | `PHASE-15-camera-capture-controls.md` |
| Bitrate & quality presets | `Settings/BitratePresets/`, `ControlBar/QuickButton/QuickButtonBitrateView` | `PHASE-16-bitrate-quality-presets.md` |
| Quick-action customization | `Settings/Display/QuickButtons/{QuickButtonsSettings,QuickButtonsButtonSettings}` | `PHASE-17-quick-action-customization.md` |
| Privacy & lifecycle modes | `Main/{LockScreen,StealthMode,SnapshotCountdown}View` | `PHASE-18-privacy-lifecycle-modes.md` |
| Settings backup & reset | `Settings/{ImportExport,Reset}/` | `PHASE-19-settings-backup-reset.md` |
| Cast history | `Settings/StreamingHistory/` | `PHASE-20-cast-history.md` |
| Help & support | `Settings/{About,HelpAndSupport}/` | `PHASE-21-help-and-support.md` |
| Network interface & Wi-Fi Aware | `Settings/Display/NetworkInterfaceNames/`, `Settings/WiFiAware/` | `PHASE-22-network-interface-wifi-aware.md` |
| Local recording | `Settings/Recordings/` | `PHASE-23-local-recording.md` |
| Pairing QR & receiver management | `Utils/{QrCodeImage,ContextMenu,SwipeLeftTo,NameEdit}` | `PHASE-24-pairing-qr-receiver-management.md` |
| Macros & action chains | `Settings/Macros/`, `ControlBar/QuickButton/QuickButtonMacrosView` | `PHASE-25-macros-action-chains.md` |
| Debug log viewer | `Settings/Debug/{Log,Video}` | `PHASE-26-debug-log-viewer.md` |
| Reusable utils backlog | `Utils/*` (remaining un-ported files) | `PHASE-27-utils-backlog.md` |

A row tagged `applicable-future` or `deferred — needs-rust-capability` in the
per-section tables below is now read as **"UI placeholder lives in the phase
above; real functionality lives in Phase 8 (deferred)"**. The original tags
are preserved so the audit trail of *why* the file was deferred remains
intact.

### Previously-excluded categories (Phases 28–48)

The v1 scope excluded chat / navigation / replay / right-overlay; v2 lifts
those exclusions for **UI placeholders only** and additionally adds
placeholders for streaming-platform, scene-widget, peripheral-hardware,
ios-target, iap, moblin-internal, mediaplayer and broadcast-deferral
categories.

| Original `not-applicable` tag | Source files (representative) | Target phase |
|---|---|---|
| chat | `Settings/Chat/*`, `View/Chat*`, `ControlBar/QuickButton/Chat/*` (21 files) | `PHASE-28-chat-overlay.md` |
| streaming-platform (destinations) | `Settings/Streams/Stream/{Twitch,Kick,YouTube,Soop,Obs,RealtimeIrl,Osp}/*` | `PHASE-29-streaming-destinations.md` |
| streaming-platform (streams root + wizards) | `Settings/Streams/{Streams,Stream,Wizard}/*` | `PHASE-30-streams-configuration-wizards.md` |
| streaming-platform (protocols) | `Settings/Streams/Stream/{Rtmp,Srt,Rist,Whip,Srtla,Rtsp,Belabox}/*` | `PHASE-31-streaming-protocols.md` |
| streaming-platform (ingests / servers) | `Settings/Ingests/*`, `Settings/{Rtmp,Rist,Srtla,Whip,Whep,Rtsp}Server/*` | `PHASE-32-ingests-servers.md` |
| scene-widget (scenes editor) | `Settings/Scenes/*` | `PHASE-33-scenes-widgets-root.md` |
| scene-widget (alerts) | `Settings/Widgets/Alerts/*` | `PHASE-34-widget-alerts.md` |
| scene-widget (content widgets) | `Settings/Widgets/{Text,Image,Slideshow,Snapshot,QrCode,Map,Bingo,Wheel,Scoreboard,VTuber,PNGTuber,VideoSource,BrowserSource,Scene,Crop}/*` | `PHASE-35-widget-content.md` |
| scene-widget (effects) | `Settings/Widgets/{Lut,Anamorphic,Dewarp,Opacity,Shape,RemoveBackground}/*` | `PHASE-36-widget-effects.md` |
| scene-widget (wizards) | `Settings/Widgets/Wizard*` | `PHASE-37-widget-wizards.md` |
| scene-widget (local overlays + position editors) | `Settings/Widgets/LocalOverlays/*`, `Utils/{Position,Size}EditView` | `PHASE-38-local-overlays.md` |
| right-overlay | `View/Stream/Overlay/Right/*` (13 files) | `PHASE-39-right-side-broadcast-hud.md` |
| replay | `Settings/Replay/*`, `View/Stream/Replay/*` (2 files) | `PHASE-40-replay-buffer.md` |
| navigation | `View/Stream/Overlay/StreamOverlayNavigationView` (1 file) | `PHASE-41-streaming-navigation-overlay.md` |
| peripheral-hardware (camera / Bluetooth) | `Settings/{DjiDevices,GoPro,PhoneCoolers,BlackSharkCoolers,CatPrinters,Gimbal,SelfieStick}/*` | `PHASE-42-camera-bluetooth-peripherals.md` |
| peripheral-hardware (workout / controller / Tesla / external camera) | `Settings/{WorkoutDevices,GameControllers,Tesla,Camera/ExternalCamera*}/*` | `PHASE-43-workout-controller-tesla-peripherals.md` |
| ios-target | `View/Watch/*`, `View/Mac/*`, `View/ExternalDisplay/*`, `Settings/{DeepLinkCreator,Keyboard,AppleSettings}/*` (13 files) | `PHASE-44-ios-watchos-mac-targets.md` (reference-only) |
| iap | `Settings/Cosmetics/*` (1 file) | `PHASE-45-in-app-purchase.md` (reference-only) |
| moblin-internal | `Settings/{Moblink,OpenAi,RemoteControl}/*` (4–5 files) | `PHASE-46-moblin-internal-integrations.md` (reference-only) |
| mediaplayer / browser | `Settings/MediaPlayer/*`, `View/WebBrowser/*` (4 files) | `PHASE-47-mediaplayer-and-browser.md` |
| broadcast deferrals (other) | `View/Stream/Overlay/{CameraLevel,DrawOnStream,FixedHorizon,Location,Left}*`, `View/Stream/StreamGridView`, `Settings/CommandCopy/*`, `Utils/PlaceholderView`, `View/ExternalDisplay/*` (~12 files) | `PHASE-48-other-broadcast-deferrals.md` |

Tagging rule: a row remains `not-applicable` for the *functional port* but
is concurrently marked **"placeholder phase: NN"** when a Slint placeholder
exists. Reactivating any placeholder as functional remains a separate
scope-change decision — these phases produce static UI only.

---

## ControlBar (24 files)

| File | Tag | Reason |
|---|---|---|
| `BatteryView.swift` | applicable-future | Battery % pill; could land in a future status-badge row above `CastControlBar`. Needs Rust telemetry source first. |
| `ControlBarLandscapeView.swift` | applicable | Landscape control-bar layout. Concept covered by `CastControlBar` (Phase 4); orientation-specific layouts are not in scope for v1. |
| `ControlBarPortraitView.swift` | applicable | Portrait control-bar layout. Already covered by `CastControlBar` (Phase 4). |
| `ControlBarUtils.swift` | applicable | Helpers absorbed into `CastControlBar` / `QuickActionButton` (Phase 4). No standalone port. |
| `QuickButton/Chat/QuickButtonChatChatterInfoView.swift` | not-applicable — chat | Per-chatter info popup. |
| `QuickButton/Chat/QuickButtonChatModerationView.swift` | not-applicable — chat | Chat moderation actions (ban/timeout). |
| `QuickButton/Chat/QuickButtonChatUrlView.swift` | not-applicable — chat | Chat URL preview. |
| `QuickButton/Chat/QuickButtonChatView.swift` | not-applicable — chat | Quick-button chat panel. |
| `QuickButton/QuickButtonAutoSceneSwitcherView.swift` | not-applicable — scene-widget | Auto-scene switcher quick button. |
| `QuickButton/QuickButtonBitrateView.swift` | applicable-future | Bitrate quick-button. Could become a future bitrate-presets phase if FCast adds adaptive bitrate. |
| `QuickButton/QuickButtonDjiDevicesView.swift` | not-applicable — peripheral-hardware | DJI device control. |
| `QuickButton/QuickButtonGoProView.swift` | not-applicable — peripheral-hardware | GoPro camera control. |
| `QuickButton/QuickButtonLiveView.swift` | not-applicable — streaming-platform | "Go live" toggle for streaming platforms. |
| `QuickButton/QuickButtonLutsView.swift` | not-applicable — scene-widget | LUT (color-grading) overlay selector. |
| `QuickButton/QuickButtonMacrosView.swift` | applicable-future | User-defined action chains. Could become a future macros phase after Phase 8 stabilizes. |
| `QuickButton/QuickButtonMicView.swift` | applicable-future | Mic mute quick-button. Maps to a future audio-controls phase. |
| `QuickButton/QuickButtonObsView.swift` | not-applicable — streaming-platform | OBS remote-control panel. |
| `QuickButton/QuickButtonSceneWidgetsView.swift` | not-applicable — scene-widget | Scene-widget toggle list. |
| `QuickButton/QuickButtonStreamSwitcherView.swift` | not-applicable — streaming-platform | Multi-stream profile switcher. |
| `QuickButtonsView.swift` | applicable | Generic quick-button container; concept ported as `for action in Bridge.quick-actions` in Phase 4. |
| `RemoteControlAssistant/ControlBarRemoteControlAssistantView.swift` | not-applicable — moblin-internal | Moblin's remote-control assistant pairing UI. |
| `StreamButton.swift` | applicable | Big start/stop stream button; concept ported as `CastButton` in Phase 4. |
| `ThermalStateSheetView.swift` | applicable-future | Device thermal state sheet. Could land in a future status-badge row. Needs `PowerManager.getCurrentThermalStatus` plumbing in Rust. |

---

## ExternalDisplay (1 file)

| File | Tag | Reason |
|---|---|---|
| `ExternalDisplayView.swift` | not-applicable — ios-target | iOS dual-display (e.g. Stage Manager / external monitor) UI. The Android sender treats the *receiver* as the external display by definition; no in-app secondary screen. |

---

## Main (4 files)

| File | Tag | Reason |
|---|---|---|
| `LockScreenView.swift` | applicable-future | UI lock to prevent accidental taps during a cast. Future privacy/lifecycle phase. |
| `MacKeyPressView.swift` | not-applicable — ios-target | macOS key-press capture for the Mac Catalyst target. |
| `SnapshotCountdownView.swift` | applicable-future | 3-2-1 countdown overlay before snapshot/start. Future privacy/lifecycle phase. |
| `StealthModeView.swift` | applicable-future | Hide preview / dim screen mode. Future privacy/lifecycle phase. |

---

## MainView (1 file)

| File | Tag | Reason |
|---|---|---|
| `MainView.swift` | applicable | Root coordinator. Concept already covered by `MainWindow` in `senders/android/ui/main.slint` (Phase 1) with `AppState` page stack and `Panel` overlay (Phase 7). |

---

## Settings — About (3 files)

| File | Tag | Reason |
|---|---|---|
| `About/AboutAttributionsSettingsView.swift` | applicable-future | OSS license / attribution screen. Future help-and-support phase. |
| `About/AboutSettingsView.swift` | applicable | App version, FCast protocol version. Covered by `FullSettingsPage` "ABOUT" section in Phase 7-H. |
| `About/AboutVersionHistorySettingsView.swift` | applicable-future | Changelog viewer. Future help-and-support phase. |

---

## Settings — Audio (1 file)

| File | Tag | Reason |
|---|---|---|
| `Audio/AudioSettingsView.swift` | applicable-future | Mic / system-audio source picker, gain slider. Future audio-controls phase. |

---

## Settings — BitratePresets (2 files)

| File | Tag | Reason |
|---|---|---|
| `BitratePresets/BitratePresetsPresetSettingsView.swift` | applicable-future | Edit a single bitrate preset. Future bitrate-presets phase, gated on FCast adding adaptive bitrate. |
| `BitratePresets/BitratePresetsSettingsView.swift` | applicable-future | List of presets. Same gate as above. |

---

## Settings — BlackSharkCoolers (3 files)

| File | Tag | Reason |
|---|---|---|
| `BlackSharkCoolers/BlackSharkCoolerDeviceScannerSettingsView.swift` | not-applicable — peripheral-hardware | Phone-cooler accessory scanner. |
| `BlackSharkCoolers/BlackSharkCoolerDeviceSettingsView.swift` | not-applicable — peripheral-hardware | Phone-cooler accessory settings. |
| `BlackSharkCoolers/BlackSharkCoolerDevicesSettingsView.swift` | not-applicable — peripheral-hardware | Phone-cooler device list. |

---

## Settings — Camera (8 files)

| File | Tag | Reason |
|---|---|---|
| `Camera/CameraControls/CameraControlsView.swift` | deferred — needs-rust-capability | Camera control panel. Blocked on FCast adding camera-capture source in Rust. |
| `Camera/CameraSettingsView.swift` | deferred — needs-rust-capability | Camera settings root. Blocked on Rust camera source. |
| `Camera/FixedHorizon/FixedHorizonView.swift` | not-applicable — defer | Horizon-leveling overlay. Niche broadcast feature; defer indefinitely even if camera lands. |
| `Camera/MirrorFrontCamera/MirrorFrontCameraView.swift` | deferred — needs-rust-capability | Front-camera mirror toggle. Blocked on camera source. |
| `Camera/TapScreenToFocus/TapScreenToFocusSettingsView.swift` | deferred — needs-rust-capability | Tap-to-focus settings. Blocked on camera source + preview phase. |
| `Camera/VideoStabilization/VideoStabilizationSettingsView.swift` | deferred — needs-rust-capability | Stabilization mode picker. Blocked on camera source. |
| `Camera/Zoom/ZoomPresetSettingsView.swift` | deferred — needs-rust-capability | Zoom preset list. Blocked on camera source. |
| `Camera/Zoom/ZoomSettingsView.swift` | deferred — needs-rust-capability | Zoom slider settings. Blocked on camera source. |
| `Camera/Zoom/ZoomSwitchToSettingsView.swift` | deferred — needs-rust-capability | Switch lens (1x → 5x) settings. Blocked on camera source. |

---

## Settings — CatPrinters (3 files)

| File | Tag | Reason |
|---|---|---|
| `CatPrinters/CatPrinterScannerSettingsView.swift` | not-applicable — peripheral-hardware | Bluetooth thermal-printer scanner. |
| `CatPrinters/CatPrinterSettingsView.swift` | not-applicable — peripheral-hardware | Thermal printer settings. |
| `CatPrinters/CatPrintersSettingsView.swift` | not-applicable — peripheral-hardware | Thermal printer list. |

---

## Settings — Chat (7 files)

| File | Tag | Reason |
|---|---|---|
| `Chat/ChatBotSettingsView.swift` | not-applicable — chat | Chat-bot configuration. |
| `Chat/ChatFiltersSettingsView.swift` | not-applicable — chat | Chat filter/blocklist. |
| `Chat/ChatNicknamesSettingsView.swift` | not-applicable — chat | Per-user nickname overrides. |
| `Chat/ChatSettingsAppearanceView.swift` | not-applicable — chat | Chat appearance. |
| `Chat/ChatSettingsLayoutView.swift` | not-applicable — chat | Chat layout. |
| `Chat/ChatSettingsView.swift` | not-applicable — chat | Chat root. |
| `Chat/ChatTextToSpeechSettingsView.swift` | not-applicable — chat | TTS for chat messages. |

---

## Settings — Debug (3 files)

| File | Tag | Reason |
|---|---|---|
| `Debug/DebugLogSettingsView.swift` | applicable-future | Log viewer with filter. Could become a future debug-logging phase. |
| `Debug/DebugSettingsView.swift` | applicable | Debug toggles. Concept covered by `DebugPage` (Phase 1-G) and Phase 7-G "CODEC & DEBUG" section. |
| `Debug/DebugVideoSettingsView.swift` | applicable-future | Video-pipeline debug toggles. Could become a future codec-debug page (extends Phase 7-G). |

---

## Settings — DeepLinkCreator (5 files)

| File | Tag | Reason |
|---|---|---|
| `DeepLinkCreator/DeepLinkCreatorQuickButtonsSettingsView.swift` | not-applicable — ios-target | iOS URL-scheme deep-link builder. |
| `DeepLinkCreator/DeepLinkCreatorSettingsView.swift` | not-applicable — ios-target | Same. |
| `DeepLinkCreator/DeepLinkCreatorStreamSettingsView.swift` | not-applicable — ios-target | Same. |
| `DeepLinkCreator/DeepLinkCreatorStreamsSettingsView.swift` | not-applicable — ios-target | Same. |
| `DeepLinkCreator/DeepLinkCreatorWebBrowserSettingsView.swift` | not-applicable — ios-target | Same. |

---

## Settings — Display (6 files)

| File | Tag | Reason |
|---|---|---|
| `Display/DisplaySettingsView.swift` | applicable | Display-section root. Replaced by `FullSettingsPage` sections in Phase 7. |
| `Display/LocalOverlays/LocalOverlaysSettingsView.swift` | not-applicable — scene-widget | Streamer overlay editor (image/text/clock placement). |
| `Display/NetworkInterfaceNames/LocalOverlaysNetworkInterfaceNamesSettingsView.swift` | applicable-future | Friendly names for network interfaces (Wi-Fi/cellular/hotspot). Future network-interface phase. |
| `Display/QuickButtons/QuickButtonsButtonSettingsView.swift` | applicable-future | Edit one quick-button. Future quick-action customization phase. |
| `Display/QuickButtons/QuickButtonsSettingsView.swift` | applicable-future | Reorder/enable quick-buttons. Future quick-action customization phase. |
| `Display/StreamButton/StreamButtonsSettingsView.swift` | applicable | "Stream button" appearance — concept covered by the single `CastButton` in Phase 4. |

---

## Settings — DjiDevices (3 files)

| File | Tag | Reason |
|---|---|---|
| `DjiDevices/DjiDeviceScannerSettingsView.swift` | not-applicable — peripheral-hardware | DJI BLE scanner. |
| `DjiDevices/DjiDeviceSettingsView.swift` | not-applicable — peripheral-hardware | One DJI device. |
| `DjiDevices/DjiDevicesSettingsView.swift` | not-applicable — peripheral-hardware | DJI device list. |

---

## Settings — GameControllers (4 files)

| File | Tag | Reason |
|---|---|---|
| `GameControllers/GameControllersControllerButtonSettingsView.swift` | not-applicable — peripheral-hardware | Controller button mapping. |
| `GameControllers/GameControllersControllerSettingsView.swift` | not-applicable — peripheral-hardware | Per-controller settings. |
| `GameControllers/GameControllersControllerThumbStickSettingsView.swift` | not-applicable — peripheral-hardware | Thumb-stick mapping. |
| `GameControllers/GameControllersSettingsView.swift` | not-applicable — peripheral-hardware | Controller list. |

---

## Settings — Gimbal / GoPro / Tesla / SelfieStick / WorkoutDevices (8 files)

| File | Tag | Reason |
|---|---|---|
| `Gimbal/GimbalSettingsView.swift` | not-applicable — peripheral-hardware | Camera gimbal control. |
| `GoPro/GoProSettingsView.swift` | not-applicable — peripheral-hardware | GoPro camera control. |
| `Tesla/TeslaSettingsView.swift` | not-applicable — peripheral-hardware | Tesla vehicle integration. |
| `SelfieStick/SelfieStickSettingsView.swift` | not-applicable — peripheral-hardware | BT selfie-stick remote. |
| `WorkoutDevices/WorkoutDeviceScannerSettingsView.swift` | not-applicable — peripheral-hardware | BLE workout sensor scanner. |
| `WorkoutDevices/WorkoutDeviceSettingsView.swift` | not-applicable — peripheral-hardware | One workout sensor. |
| `WorkoutDevices/WorkoutDevicesSettingsView.swift` | not-applicable — peripheral-hardware | Workout sensor list. |

---

## Settings — HelpAndSupport (1 file)

| File | Tag | Reason |
|---|---|---|
| `HelpAndSupport/HelpAndSupportSettingsView.swift` | applicable-future | Help/FAQ/links page. Future help-and-support phase. |

---

## Settings — ImportExport (3 files)

| File | Tag | Reason |
|---|---|---|
| `ImportExport/ExportSettingsView.swift` | applicable-future | Export FCast settings to JSON. Future backup phase. |
| `ImportExport/ImportExportSettingsView.swift` | applicable-future | Import/export root. Future backup phase. |
| `ImportExport/ImportSettingsView.swift` | applicable-future | Import FCast settings from JSON. Future backup phase. |

---

## Settings — Ingests (12 files)

All `Ingests/*` views configure RTMP / SRT / RIST / WHIP / RTSP / SRTLA servers
and clients. The FCast sender uses the FCast binary protocol over TCP — none of
these protocols apply.

| File | Tag | Reason |
|---|---|---|
| `Ingests/IngestsSettingsView.swift` | not-applicable — streaming-platform | Ingest root. |
| `Ingests/RistServer/RistServerSettingsView.swift` | not-applicable — streaming-platform | RIST server. |
| `Ingests/RistServer/RistServerStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream RIST settings. |
| `Ingests/RtmpServer/RtmpServerSettingsView.swift` | not-applicable — streaming-platform | RTMP server. |
| `Ingests/RtmpServer/RtmpServerStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream RTMP settings. |
| `Ingests/RtspClient/RtspClientSettingsView.swift` | not-applicable — streaming-platform | RTSP client. |
| `Ingests/RtspClient/RtspClientStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream RTSP settings. |
| `Ingests/SrtlaServer/SrtlaServerSettingsView.swift` | not-applicable — streaming-platform | SRTLA server. |
| `Ingests/SrtlaServer/SrtlaServerStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream SRTLA settings. |
| `Ingests/WhepClient/WhepClientSettingsView.swift` | not-applicable — streaming-platform | WHEP client. |
| `Ingests/WhepClient/WhepClientStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream WHEP settings. |
| `Ingests/WhipServer/WhipServerSettingsView.swift` | not-applicable — streaming-platform | WHIP server. |
| `Ingests/WhipServer/WhipServerStreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream WHIP settings. |

---

## Settings — Keyboard / Location (3 files)

| File | Tag | Reason |
|---|---|---|
| `Keyboard/KeyboardKeySettingsView.swift` | not-applicable — ios-target | iOS hardware-keyboard shortcut binding. |
| `Keyboard/KeyboardSettingsView.swift` | not-applicable — ios-target | iOS hardware-keyboard root. |
| `Location/LocationSettingsView.swift` | not-applicable — defer | GPS/location overlay. Tied to the navigation-overlay category — excluded by user. |

---

## Settings — Macros (1 file)

| File | Tag | Reason |
|---|---|---|
| `Macros/MacrosSettingsView.swift` | applicable-future | User-defined action chains. Future macros phase, depends on Phase 4 quick-actions and Phase 8 Rust dispatcher. |

---

## Settings — MediaPlayer (3 files)

| File | Tag | Reason |
|---|---|---|
| `MediaPlayer/MediaPlayerFileSettingsView.swift` | not-applicable — defer | Per-file source settings for an in-app media player. FCast sends to a *receiver*; the in-app player is a Moblin streaming-source feature, not a cast-sender feature. |
| `MediaPlayer/MediaPlayerSettingsView.swift` | not-applicable — defer | Same as above. |
| `MediaPlayer/MediaPlayersSettingsView.swift` | not-applicable — defer | Same as above. |

---

## Settings — Moblink (1 file)

| File | Tag | Reason |
|---|---|---|
| `Moblink/MoblinkSettingsView.swift` | not-applicable — moblin-internal | Moblin-specific peer-to-peer relay (multiple phones combine bandwidth). Has no FCast analogue. |

---

## Settings — Recordings (1 file)

| File | Tag | Reason |
|---|---|---|
| `Recordings/RecordingsSettingsView.swift` | deferred — needs-rust-capability | Local recording of cast session. Blocked on Rust media graph adding a recording sink (file mux). |

---

## Settings — RemoteControl (1 file)

| File | Tag | Reason |
|---|---|---|
| `RemoteControl/RemoteControlSettingsView.swift` | not-applicable — moblin-internal | Moblin's "phone B controls phone A" pairing. FCast sender does not act as both controller and remote. |

---

## Settings — Reset (1 file)

| File | Tag | Reason |
|---|---|---|
| `Reset/ResetSettingsView.swift` | applicable-future | Factory reset / clear settings. Future settings-management phase. |

---

## Settings — Scenes (51 files)

All `Scenes/**` views are Moblin's scene/widget editor — alerts, scoreboards,
slideshows, VTubers, PNGTubers, wheel-of-luck, bingo, browser-source widgets,
text widgets, image widgets, video-source widgets, effects (LUTs, dewarp,
opacity, shape, anamorphic, remove-background), wizards. None apply to a
cast remote.

| File | Tag | Reason |
|---|---|---|
| `Scenes/AutoSwitchers/AutoSwitchersSettingsView.swift` | not-applicable — scene-widget | Scene auto-switching rules. |
| `Scenes/DisconnectProtection/DisconnectProtectionSettingsView.swift` | not-applicable — scene-widget | "Hide stream on disconnect" overlay logic. |
| `Scenes/Scene/SceneSettingsView.swift` | not-applicable — scene-widget | One scene definition. |
| `Scenes/Scene/SceneWidgetSettingsView.swift` | not-applicable — scene-widget | Per-widget placement in a scene. |
| `Scenes/ScenesSettingsView.swift` | not-applicable — scene-widget | Scene list. |
| `Scenes/Widgets/WidgetsSettingsView.swift` | not-applicable — scene-widget | Widget library. |
| `Scenes/Widgets/Widget/WidgetSettingsView.swift` | not-applicable — scene-widget | Per-widget root. |
| `Scenes/Widgets/Widget/WidgetWizardSettingsView.swift` | not-applicable — scene-widget | Widget creation wizard. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsChatBotSettingsView.swift` | not-applicable — chat | Chat-bot alert. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsImageSettingsView.swift` | not-applicable — scene-widget | Alert image. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsKickSettingsView.swift` | not-applicable — chat | Kick chat alert. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsSettingsView.swift` | not-applicable — scene-widget | Alert root. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsSoundSettingsView.swift` | not-applicable — scene-widget | Alert sound. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsSpeechToTextSettingsView.swift` | not-applicable — chat | TTS alert. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsTextSettingsView.swift` | not-applicable — scene-widget | Alert text. |
| `Scenes/Widgets/Widget/Alerts/WidgetAlertsTwitchSettingsView.swift` | not-applicable — chat | Twitch alert. |
| `Scenes/Widgets/Widget/BingoCard/WidgetBingoCardSettingsView.swift` | not-applicable — scene-widget | Bingo widget. |
| `Scenes/Widgets/Widget/Browser/WidgetBrowserSettingsView.swift` | not-applicable — scene-widget | Browser-source widget. |
| `Scenes/Widgets/Widget/Chat/WidgetChatSettingsView.swift` | not-applicable — chat | Chat widget. |
| `Scenes/Widgets/Widget/Crop/WidgetCropSettingsView.swift` | not-applicable — scene-widget | Crop widget. |
| `Scenes/Widgets/Widget/Effects/AnamorphicLensEffectView.swift` | not-applicable — scene-widget | Anamorphic effect. |
| `Scenes/Widgets/Widget/Effects/Dewarp360EffectView.swift` | not-applicable — scene-widget | 360° dewarp. |
| `Scenes/Widgets/Widget/Effects/LutEffectView.swift` | not-applicable — scene-widget | LUT color effect. |
| `Scenes/Widgets/Widget/Effects/OpacityEffectView.swift` | not-applicable — scene-widget | Opacity effect. |
| `Scenes/Widgets/Widget/Effects/RemoveBackgroundEffectView.swift` | not-applicable — scene-widget | Background removal. |
| `Scenes/Widgets/Widget/Effects/ShapeEffectView.swift` | not-applicable — scene-widget | Shape mask effect. |
| `Scenes/Widgets/Widget/Effects/WidgetEffectsView.swift` | not-applicable — scene-widget | Effects root. |
| `Scenes/Widgets/Widget/Image/WidgetImageSettingsView.swift` | not-applicable — scene-widget | Image widget. |
| `Scenes/Widgets/Widget/Map/WidgetMapSettingsView.swift` | not-applicable — scene-widget | Map widget. |
| `Scenes/Widgets/Widget/PngTuber/WidgetPngTuberSettingsView.swift` | not-applicable — scene-widget | PNGTuber avatar. |
| `Scenes/Widgets/Widget/QrCode/WidgetQrCodeSettingsView.swift` | not-applicable — scene-widget | QR-code widget for stream overlay. |
| `Scenes/Widgets/Widget/Scene/WidgetSceneSettingsView.swift` | not-applicable — scene-widget | Sub-scene widget. |
| `Scenes/Widgets/Widget/Scoreboard/WidgetScoreboardGenericSettingsView.swift` | not-applicable — scene-widget | Generic scoreboard. |
| `Scenes/Widgets/Widget/Scoreboard/WidgetScoreboardModularSettingsView.swift` | not-applicable — scene-widget | Modular scoreboard. |
| `Scenes/Widgets/Widget/Scoreboard/WidgetScoreboardPadelSettingsView.swift` | not-applicable — scene-widget | Padel scoreboard. |
| `Scenes/Widgets/Widget/Scoreboard/WidgetScoreboardSettingsView.swift` | not-applicable — scene-widget | Scoreboard root. |
| `Scenes/Widgets/Widget/Slideshow/WidgetSlideshowSettingsView.swift` | not-applicable — scene-widget | Slideshow widget. |
| `Scenes/Widgets/Widget/Snapshot/WidgetSnapshotSettingsView.swift` | not-applicable — scene-widget | Snapshot widget. |
| `Scenes/Widgets/Widget/Text/WidgetTextSettingsView.swift` | not-applicable — scene-widget | Text widget. |
| `Scenes/Widgets/Widget/VTuber/WidgetVTuberSettingsView.swift` | not-applicable — scene-widget | VTuber avatar. |
| `Scenes/Widgets/Widget/VideoSource/WidgetVideoSourceSettingsView.swift` | not-applicable — scene-widget | Video-source widget. |
| `Scenes/Widgets/Widget/WheelOfLuck/WidgetWheelOfLuckSettingsView.swift` | not-applicable — scene-widget | Wheel-of-luck widget. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardBingoCardSettingsView.swift` | not-applicable — scene-widget | Bingo wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardBrowserSettingsView.swift` | not-applicable — scene-widget | Browser wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardImageSettingsView.swift` | not-applicable — scene-widget | Image wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardPngTuberSettingsView.swift` | not-applicable — scene-widget | PNGTuber wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardSlideshowSettingsView.swift` | not-applicable — scene-widget | Slideshow wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardTextSettingsView.swift` | not-applicable — scene-widget | Text wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardVTuberSettingsView.swift` | not-applicable — scene-widget | VTuber wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardVideoSourceSettingsView.swift` | not-applicable — scene-widget | Video-source wizard. |
| `Scenes/Widgets/Widget/Wizard/WidgetWizardWheelOfLuckSettingsView.swift` | not-applicable — scene-widget | Wheel-of-luck wizard. |

---

## Settings — Store / SettingsView (2 files)

| File | Tag | Reason |
|---|---|---|
| `Store/StoreSettingsView.swift` | not-applicable — iap | StoreKit subscription / purchase. |
| `SettingsView.swift` | applicable | Settings root. Replaced by `FullSettingsPage` in Phase 7. |

---

## Settings — StreamingHistory (2 files)

| File | Tag | Reason |
|---|---|---|
| `StreamingHistory/StreamingHistorySettingsView.swift` | applicable-future | Cast-history list. Future cast-history phase. |
| `StreamingHistory/StreamingHistoryStreamSettingsView.swift` | applicable-future | Per-cast detail. Future cast-history phase. |

---

## Settings — Streams (44 files)

All `Streams/**` views configure outgoing streaming destinations (Twitch,
YouTube, Kick, Soop, OBS-remote, OpenStreamingPlatform), per-platform
authentication/wizards, and per-protocol bitrate/connection settings (RTMP,
SRT, RIST, WHIP). None apply to FCast.

| File | Tag | Reason |
|---|---|---|
| `Streams/StreamsSettingsView.swift` | not-applicable — streaming-platform | Streams list root. |
| `Streams/Stream/StreamSettingsView.swift` | not-applicable — streaming-platform | Per-stream root. |
| `Streams/Stream/StreamWizardSettingsView.swift` | not-applicable — streaming-platform | Stream-creation wizard. |
| `Streams/Stream/Audio/StreamAudioSettingsView.swift` | not-applicable — streaming-platform | Per-stream audio settings. |
| `Streams/Stream/Chat/StreamEmotesSettingsView.swift` | not-applicable — chat | Emote settings. |
| `Streams/Stream/GoLiveNotification/GoLiveNotificationSettingsView.swift` | not-applicable — streaming-platform | "Going live" push notification. |
| `Streams/Stream/Kick/StreamKickSettingsView.swift` | not-applicable — streaming-platform | Kick platform settings. |
| `Streams/Stream/MultiStreaming/StreamMultiStreamingSettingsView.swift` | not-applicable — streaming-platform | Multi-platform simulcast. |
| `Streams/Stream/ObsRemoteControl/StreamObsRemoteControlSettingsView.swift` | not-applicable — streaming-platform | OBS WebSocket remote-control. |
| `Streams/Stream/OpenStreamingPlatform/StreamOpenStreamingPlatformSettingsView.swift` | not-applicable — streaming-platform | OpenStreamingPlatform. |
| `Streams/Stream/RealtimeIrl/StreamRealtimeIrlSettingsView.swift` | not-applicable — streaming-platform | RealtimeIRL location ping. |
| `Streams/Stream/Recording/StreamRecordingAudioSettingsView.swift` | not-applicable — streaming-platform | Per-stream recording audio. |
| `Streams/Stream/Recording/StreamRecordingSettingsView.swift` | not-applicable — streaming-platform | Per-stream recording. |
| `Streams/Stream/Replay/StreamReplaySettingsView.swift` | not-applicable — replay | Per-stream replay buffer. |
| `Streams/Stream/Rist/StreamRistSettingsView.swift` | not-applicable — streaming-platform | RIST settings. |
| `Streams/Stream/Rtmp/StreamRtmpSettingsView.swift` | not-applicable — streaming-platform | RTMP settings. |
| `Streams/Stream/Snapshot/StreamSnapshotSettingsView.swift` | not-applicable — streaming-platform | Per-stream snapshot. |
| `Streams/Stream/Soop/StreamSoopSettingsView.swift` | not-applicable — streaming-platform | Soop platform. |
| `Streams/Stream/Srt/StreamSrtAdaptiveBitrateSettingsView.swift` | not-applicable — streaming-platform | SRT adaptive bitrate. |
| `Streams/Stream/Srt/StreamSrtConnectionPriority2View.swift` | not-applicable — streaming-platform | SRT priority. |
| `Streams/Stream/Srt/StreamSrtSettingsView.swift` | not-applicable — streaming-platform | SRT settings. |
| `Streams/Stream/Twitch/StreamTwitchSettingsView.swift` | not-applicable — streaming-platform | Twitch platform. |
| `Streams/Stream/Url/StreamUrlSettingsView.swift` | not-applicable — streaming-platform | Stream URL editor. |
| `Streams/Stream/Video/StreamVideoSettingsView.swift` | not-applicable — streaming-platform | Per-stream video settings. |
| `Streams/Stream/Whip/StreamWhipSettingsView.swift` | not-applicable — streaming-platform | WHIP settings. |
| `Streams/Stream/Wizard/Custom/StreamWizardCustomRistSettingsView.swift` | not-applicable — streaming-platform | Custom RIST wizard. |
| `Streams/Stream/Wizard/Custom/StreamWizardCustomRtmpSettingsView.swift` | not-applicable — streaming-platform | Custom RTMP wizard. |
| `Streams/Stream/Wizard/Custom/StreamWizardCustomSettingsView.swift` | not-applicable — streaming-platform | Custom wizard root. |
| `Streams/Stream/Wizard/Custom/StreamWizardCustomSrtSettingsView.swift` | not-applicable — streaming-platform | Custom SRT wizard. |
| `Streams/Stream/Wizard/Custom/StreamWizardCustomWhipSettingsView.swift` | not-applicable — streaming-platform | Custom WHIP wizard. |
| `Streams/Stream/Wizard/NetworkSetup/MyServers/StreamWizardNetworkSetupMyServersRtmpSettingsView.swift` | not-applicable — streaming-platform | "My servers" RTMP wizard. |
| `Streams/Stream/Wizard/NetworkSetup/MyServers/StreamWizardNetworkSetupMyServersSettingsView.swift` | not-applicable — streaming-platform | "My servers" wizard. |
| `Streams/Stream/Wizard/NetworkSetup/MyServers/StreamWizardNetworkSetupMyServersSrtSettingsView.swift` | not-applicable — streaming-platform | "My servers" SRT wizard. |
| `Streams/Stream/Wizard/NetworkSetup/StreamWizardNetworkSetupBelaboxSettingsView.swift` | not-applicable — streaming-platform | Belabox cloud wizard. |
| `Streams/Stream/Wizard/NetworkSetup/StreamWizardNetworkSetupDirectSettingsView.swift` | not-applicable — streaming-platform | Direct connect wizard. |
| `Streams/Stream/Wizard/NetworkSetup/StreamWizardNetworkSetupObsSettingsView.swift` | not-applicable — streaming-platform | OBS network wizard. |
| `Streams/Stream/Wizard/NetworkSetup/StreamWizardNetworkSetupSettingsView.swift` | not-applicable — streaming-platform | Network wizard root. |
| `Streams/Stream/Wizard/Platform/StreamWizardKickSettingsView.swift` | not-applicable — streaming-platform | Kick wizard. |
| `Streams/Stream/Wizard/Platform/StreamWizardObsSettingsView.swift` | not-applicable — streaming-platform | OBS wizard. |
| `Streams/Stream/Wizard/Platform/StreamWizardSoopSettingsView.swift` | not-applicable — streaming-platform | Soop wizard. |
| `Streams/Stream/Wizard/Platform/StreamWizardTwitchSettingsView.swift` | not-applicable — streaming-platform | Twitch wizard. |
| `Streams/Stream/Wizard/Platform/StreamWizardYouTubeSettingsView.swift` | not-applicable — streaming-platform | YouTube wizard. |
| `Streams/Stream/Wizard/StreamWizardGeneralSettingsView.swift` | not-applicable — streaming-platform | General wizard step. |
| `Streams/Stream/Wizard/StreamWizardObsRemoteControlSettingsView.swift` | not-applicable — streaming-platform | OBS-remote wizard step. |
| `Streams/Stream/YouTube/StreamYouTubeSettingsView.swift` | not-applicable — streaming-platform | YouTube settings. |

---

## Settings — Talkback / Watch / WiFiAware (5 files)

| File | Tag | Reason |
|---|---|---|
| `Talkback/TalkbackSettingsView.swift` | not-applicable — chat | Two-way audio (chat-adjacent). |
| `Watch/Chat/WatchChatSettingsView.swift` | not-applicable — chat | Apple Watch chat. |
| `Watch/Display/LocalOverlays/WatchLocalOverlaysSettingsView.swift` | not-applicable — ios-target | Apple Watch overlays. |
| `Watch/Display/WatchDisplaySettingsView.swift` | not-applicable — ios-target | Apple Watch display. |
| `Watch/WatchSettingsView.swift` | not-applicable — ios-target | Apple Watch root. |
| `WiFiAware/WiFiAwareSettingsView.swift` | deferred — needs-rust-capability | Wi-Fi Aware peer-to-peer discovery. Blocked on Rust mDNS/Wi-Fi-Aware integration. |

---

## Stream (23 files)

| File | Tag | Reason |
|---|---|---|
| `CameraLevelView.swift` | not-applicable — defer | Camera horizon-level indicator. No camera level concept in FCast sender. |
| `DrawOnStreamView.swift` | not-applicable — defer | Live annotation drawing on broadcast. Not applicable to a cast remote; defer. |
| `Overlay/Right/AudioLevelView.swift` | not-applicable — right-overlay | Right-side audio meter. *Component* (the level meter widget) could be extracted as part of an audio-controls phase, but the right-overlay layout is excluded. |
| `Overlay/Right/CameraSettingsControlView.swift` | not-applicable — right-overlay | Right-overlay camera controls panel. |
| `Overlay/Right/MediaPlayerControlsView.swift` | not-applicable — right-overlay | Right-overlay media-player transport controls (in-app player, not cast-target controls). |
| `Overlay/Right/ReplayView.swift` | not-applicable — replay | Live-stream replay UI. |
| `Overlay/Right/SceneSelectorView.swift` | not-applicable — right-overlay | Right-overlay scene selector. |
| `Overlay/Right/SegmentedPicker.swift` | not-applicable — right-overlay | Right-overlay segmented picker. *Component* could be reused if extracted; the right-overlay layout is excluded. |
| `Overlay/Right/StreamOverlayRightBeautyView.swift` | not-applicable — right-overlay | Beauty filter overlay. |
| `Overlay/Right/StreamOverlayRightFaceView.swift` | not-applicable — right-overlay | Face filter overlay. |
| `Overlay/Right/StreamOverlayRightPinchView.swift` | not-applicable — right-overlay | Pinch effect overlay. |
| `Overlay/Right/StreamOverlayRightPixellateView.swift` | not-applicable — right-overlay | Pixellate effect overlay. |
| `Overlay/Right/StreamOverlayRightWhirlpoolView.swift` | not-applicable — right-overlay | Whirlpool effect overlay. |
| `Overlay/Right/VideoPreviewView.swift` | not-applicable — right-overlay | Right-overlay video preview thumbnail. |
| `Overlay/Right/ZoomPresetSelctorView.swift` | not-applicable — right-overlay | Right-overlay zoom preset chips. |
| `Overlay/StreamOverlayChatView.swift` | not-applicable — chat | Chat overlay. |
| `Overlay/StreamOverlayDebugView.swift` | applicable | Debug overlay. Concept already covered by `DebugPage` (Phase 1-G) and the Phase 5 `StatusOverlay`. |
| `Overlay/StreamOverlayLeftView.swift` | not-applicable — defer | Left-side broadcast HUD layout (battery/thermal/uptime). The user's exclusion was right-side; left-side is technically not excluded, but FCast does not have a streamer broadcast layout, so the *layout* is not applicable. Individual badges (battery, thermal) tracked under `ControlBar/BatteryView.swift` and `ThermalStateSheetView.swift`. |
| `Overlay/StreamOverlayNavigationView.swift` | not-applicable — navigation | Navigation overlay. |
| `Overlay/StreamOverlayRightView.swift` | not-applicable — right-overlay | Right-side HUD root. |
| `StreamGridView.swift` | not-applicable — defer | Multi-camera grid composition. FCast captures one source. |
| `StreamOverlayView.swift` | not-applicable — defer | Top-level overlay coordinator. The `StatusOverlay` layer (Phase 5) is the FCast equivalent, but the broadcast-overlay coordinator concept does not apply. |
| `StreamView.swift` | applicable-future | Live capture preview window. Future capture-preview phase. |

---

## Utils (37 files)

Reusable SwiftUI controls. Most have FCast equivalents already in
`senders/android/ui/components/buttons.slint` and `settings_rows.slint`
(Phase 3). The remainder are tagged `applicable-future` for a "reusable Utils
backlog" phase if/when the surface needs them.

| File | Tag | Reason |
|---|---|---|
| `AddButtonView.swift` | applicable-future | "+" toolbar button. Future utils phase (toolbars). |
| `BorderlessButtonView.swift` | applicable | Concept covered by `TextButton` in Phase 3. |
| `ButtonView.swift` | applicable | Concept covered by `PrimaryButton` / `DestructiveButton` in Phase 3. |
| `CloseToolbarView.swift` | applicable-future | "X" close-toolbar button. Future utils phase. |
| `CommandCopyView.swift` | not-applicable — defer | Copyable shell-command snippet (Moblin debug help). Niche. |
| `ContextMenuDeleteButtonView.swift` | applicable-future | Context-menu delete action. Future utils phase (context menus). |
| `ContextMenuDuplicateButtonView.swift` | applicable-future | Context-menu duplicate action. Future utils phase. |
| `CreateButtonView.swift` | applicable-future | "Create" button. Future utils phase. |
| `DraggableItemPrefixView.swift` | applicable-future | Drag-handle prefix for reorderable lists. Future utils phase. |
| `FormFieldError.swift` | applicable-future | Inline form-field error label. Future utils phase. |
| `HCenter.swift` | applicable | Trivial horizontal-center helper; covered by `HorizontalLayout { alignment: center; }` in Slint. |
| `IconAndTextView.swift` | applicable-future | Icon + text row helper. Future utils phase. |
| `InfoBannerView.swift` | applicable-future | Info banner (separate from status-overlay pills). Future utils phase. |
| `InlinePickerView.swift` | applicable-future | Segmented picker. Future utils phase (referenced by FUTURE-camera, FUTURE-audio). |
| `MultiLineTextFieldView.swift` | applicable-future | Multi-line text field (e.g. notes). Future utils phase. |
| `NameEditView.swift` | applicable-future | Rename modal. Future receiver-management phase. |
| `OpenAiSettingsView.swift` | not-applicable — moblin-internal | OpenAI API key entry. |
| `PositionEditView.swift` | not-applicable — scene-widget | Overlay position editor (X/Y placement). |
| `QrCodeImageView.swift` | applicable-future | QR-code image renderer. Future pairing-QR phase (display this device's QR). |
| `ShortcutView.swift` | not-applicable — ios-target | iOS keyboard-shortcut picker. |
| `SizeEditView.swift` | not-applicable — scene-widget | Overlay size editor. |
| `SliderView.swift` | applicable | Concept covered by `SettingsSliderRow` in Phase 3. |
| `StrokeModifier.swift` | applicable-future | SwiftUI text-stroke helper. Future utils phase if needed. |
| `SwipeLeftToDeleteButtonView.swift` | applicable-future | Swipe-to-delete row action. Future utils phase. |
| `SwipeLeftToDeleteHelpView.swift` | applicable-future | First-time-use hint for swipe-to-delete. Future utils phase. |
| `SwipeLeftToDuplicateButtonView.swift` | applicable-future | Swipe-to-duplicate. Future utils phase. |
| `SwipeLeftToDuplicateOrDeleteHelpView.swift` | applicable-future | Hint for swipe-duplicate/delete. Future utils phase. |
| `SwipeLeftToRemoveHelpView.swift` | applicable-future | Hint for swipe-remove. Future utils phase. |
| `TextEditNavigationView.swift` | applicable-future | Push-style text-edit modal. Future utils phase. |
| `TextEditView.swift` | applicable-future | Plain text-edit modal. Future utils phase. |
| `TextItemView.swift` | applicable | Concept covered by `SettingsTextRow` in Phase 3. |
| `TextValueView.swift` | applicable | Concept covered by `SettingsValueRow` in Phase 3. |
| `UrlsView.swift` | applicable-future | List of URLs (multiple receivers / endpoints). Future utils phase. |
| `ValueEditView.swift` | applicable-future | Numeric-value-edit modal. Future utils phase. |
| `VideoSourceRotationView.swift` | deferred — needs-rust-capability | Source-rotation lock (0/90/180/270/auto). Needs Rust capture-rotation control. |
| `VoicesView.swift` | not-applicable — chat | TTS voice picker. |

---

## WebBrowser (1 file)

| File | Tag | Reason |
|---|---|---|
| `WebBrowser/WebBrowserView.swift` | not-applicable — defer | In-app web browser (Moblin uses it as a browser-source widget). FCast sender does not host an in-app browser. |

---

## Tally

Exact counts across all 279 Moblin view files (matches `wc -l draft/moblin-ui/VIEW_FILES.md`
minus section headers; `MainView.swift` is row 1 of the "MainView" section above):

| Bucket | Count |
|---|---|
| applicable (covered in Phases 0–11) | 18 |
| applicable-future (could become a future phase) | 50 |
| deferred — needs-rust-capability | 11 |
| not-applicable — chat | 21 |
| not-applicable — navigation | 1 |
| not-applicable — replay | 2 |
| not-applicable — right-overlay | 13 |
| not-applicable — streaming-platform | 59 |
| not-applicable — scene-widget | 52 |
| not-applicable — peripheral-hardware | 22 |
| not-applicable — ios-target | 13 |
| not-applicable — iap | 1 |
| not-applicable — moblin-internal | 4 |
| not-applicable — defer (other) | 12 |
| **Total** | **279** |

About **75%** of Moblin's UI surface (~210 / 279 files) is **not applicable**
to a cast sender — Moblin is fundamentally a livestream-broadcast app with a
heavy scene/widget editor and many hardware integrations. The applicable subset
(~18 files = 6.5%) is already covered by Phases 0–11. The remaining
~50 `applicable-future` and ~11 `deferred` files are the realistic post-v1
backlog.

---

## Maintenance

When a new Moblin upstream release adds files:

1. Re-run the inventory script that produced `draft/moblin-ui/VIEW_FILES.md`
   (see `draft/moblin-ui/INFO.md`).
2. `diff` the new file list against the table above.
3. Add a row for each new file with the appropriate tag.
4. If a new Moblin feature falls outside the existing tag legend in
   [`README.md`](README.md), add the tag to both files.

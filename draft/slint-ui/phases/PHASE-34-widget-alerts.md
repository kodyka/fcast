# Phase 34 — Widget Alerts Placeholder

> **UI-only.** Per-platform alert widget editors (Twitch / Kick / Chat-bot /
> TTS / Image / Sound / Text). **No real EventSub / WebSocket / TTS engine.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 33
**Functional integration:** Permanently deferred.

**Moblin source analogues** (8 files):
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsSettingsView.swift` — root tabs
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsTwitchSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsKickSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsChatBotSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsSpeechToTextSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsImageSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsSoundSettingsView.swift`
- `View/Settings/Scenes/Widgets/Widget/Alerts/WidgetAlertsTextSettingsView.swift`

**Files to add:**
- `senders/android/ui/pages/widget_alerts_settings_page.slint` — TabWidget root
- `senders/android/ui/pages/widget_alerts_<kind>_settings_page.slint` — content panes (twitch/kick/chat-bot/tts/image/sound/text)

---

## Goal

Render Moblin's alert sub-widgets as a tabbed editor. Each tab routes one
event type (follow, raid, sub, donation, etc.) to a media response (image,
sound, text overlay, TTS).

---

## Moblin source pattern

```swift
// WidgetAlertsTwitchSettingsView.swift (excerpt)
List {
    NavigationLink { /* alert config */ } label: {
        HStack { Toggle(alert.event.toString(), isOn: $alert.enabled) }
    }
}
```

A flat list of toggleable events, each with a deep configuration page.

---

## Tasks

### 34-A — `WidgetAlertsSettingsPage` (TabWidget root)

```slint
import { TabWidget } from "std-widgets.slint";

export component WidgetAlertsSettingsPage inherits Rectangle {
    in-out property <int> mock-platform-tab: 0;

    background: Theme.surface-primary;
    VerticalLayout {
        // Header (title + Done) ...
        TabWidget {
            current-index <=> root.mock-platform-tab;
            Tab { title: "Twitch";    WidgetAlertsTwitchPane    {} }
            Tab { title: "Kick";      WidgetAlertsKickPane      {} }
            Tab { title: "Chat-bot";  WidgetAlertsChatBotPane   {} }
            Tab { title: "TTS";       WidgetAlertsTtsPane       {} }
            Tab { title: "Image";     WidgetAlertsImagePane     {} }
            Tab { title: "Sound";     WidgetAlertsSoundPane     {} }
            Tab { title: "Text";      WidgetAlertsTextPane      {} }
        }
    }
}
```

### 34-B — Per-kind panes

Each pane is its own `.slint` file. They share the structure:

```slint
export struct AlertEvent {
    id: string,
    label: string,            // "Follow", "Raid", "Sub", "Bits", "Donation"
    enabled: bool,
    media-name: string,       // "horn.mp3" / "sub_alert.png"
    duration-ms: int,
}

export component WidgetAlertsTwitchPane inherits Rectangle {
    in-out property <[AlertEvent]> mock-events: [
        { id: "follow",   label: "Follow",   enabled: true,
          media-name: "follow.mp3",   duration-ms: 3000 },
        { id: "raid",     label: "Raid",     enabled: true,
          media-name: "raid_horn.mp3", duration-ms: 5000 },
        { id: "sub",      label: "Sub",      enabled: false,
          media-name: "sub_alert.png", duration-ms: 4000 },
        { id: "bits",     label: "Bits",     enabled: false,
          media-name: "coin.mp3",    duration-ms: 2000 },
        { id: "follow_msg", label: "Follow message", enabled: false,
          media-name: "(none)",        duration-ms: 0 },
    ];

    VerticalLayout {
        for e in root.mock-events: SettingsValueRow {
            label: e.label;
            value: (e.enabled ? "On" : "Off") + " · " + e.media-name;
            clicked => { Bridge.active-panel = Panel.widget-alert-detail; }
        }
    }
}
```

### 34-C — Per-event detail page

- [ ] `widget_alert_detail_page.slint`: a single event with:
  - Toggle "Enabled"
  - Media file picker (`SettingsValueRow` cycler over 5 stub options)
  - Duration slider (500 ms — 10 s)
  - Volume slider
  - Test button (no-op)

### 34-D — Pane variants

- **Twitch / Kick** — events list (follow/raid/sub/bits/donation/host).
- **Chat-bot** — trigger-text → response-text rules list.
- **TTS** — voice picker, rate/pitch sliders, "preview" button.
- **Image / Sound / Text** — single-asset variants of the detail page.

---

## Exit criteria

1. `WidgetAlertsSettingsPage` opens with 7 tabs.
2. Each tab renders its mock event list / media settings.
3. Per-event detail page allows tweaking and is reactive.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real Twitch EventSub / Kick WebSocket subscriptions.
- Real chat-bot rule engine.
- Real TTS audio synthesis.
- Real audio playback / image rendering on alert trigger.

---

## Slint best practices applied here

- **`TabWidget` from `std-widgets.slint`** is the right primitive for the
  per-platform alert editor — see [TabWidget reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/tabwidget.mdx).
- **`current-index <=> root.mock-platform-tab;`** sets up two-way binding
  so the page's selected tab survives navigation (until the user
  closes/re-opens the panel).
- **Inline `[...]` mock data per pane** keeps tabs independent — no
  shared state means each pane can be `slint-viewer`-tested in
  isolation.

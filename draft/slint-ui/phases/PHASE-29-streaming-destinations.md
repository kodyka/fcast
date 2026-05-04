# Phase 29 — Streaming Destinations Placeholder

> **UI-only.** Placeholder pages for Twitch / Kick / YouTube / Soop /
> OBS-remote / RealtimeIRL / OpenStreamingPlatform configuration. **No real
> OAuth, no platform API calls, no stream key submission.** FCast does not
> stream to any of these platforms — these pages exist purely to keep the
> design surface complete.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Permanently deferred — FCast has no platform
authentication or RTMP/SRT/WHIP egress.

**Moblin source analogues** (~10 files):
- `View/Settings/Streams/Stream/Twitch/StreamTwitchSettingsView.swift`
- `View/Settings/Streams/Stream/Kick/StreamKickSettingsView.swift`
- `View/Settings/Streams/Stream/YouTube/StreamYouTubeSettingsView.swift`
- `View/Settings/Streams/Stream/Soop/StreamSoopSettingsView.swift`
- `View/Settings/Streams/Stream/ObsRemoteControl/StreamObsRemoteControlSettingsView.swift`
- `View/Settings/Streams/Stream/RealtimeIrl/StreamRealtimeIrlSettingsView.swift`
- `View/Settings/Streams/Stream/OpenStreamingPlatform/StreamOpenStreamingPlatformSettingsView.swift`
- `View/Settings/Streams/Stream/MultiStreaming/StreamMultiStreamingSettingsView.swift`
- `View/Settings/Streams/Stream/GoLiveNotification/GoLiveNotificationSettingsView.swift`
- `View/ControlBar/QuickButton/QuickButtonLiveView.swift`

**Files to add:**
- `senders/android/ui/pages/destinations_settings_page.slint` — root
- `senders/android/ui/pages/destination_twitch_settings_page.slint`
- `senders/android/ui/pages/destination_kick_settings_page.slint`
- `senders/android/ui/pages/destination_youtube_settings_page.slint`
- `senders/android/ui/pages/destination_soop_settings_page.slint`
- `senders/android/ui/pages/destination_obs_remote_settings_page.slint`
- `senders/android/ui/pages/destination_realtime_irl_settings_page.slint`
- `senders/android/ui/pages/destination_osp_settings_page.slint`
- `senders/android/ui/pages/multi_streaming_settings_page.slint`
- `senders/android/ui/pages/go_live_notification_settings_page.slint`

---

## Goal

Surface a destinations picker + per-destination configuration forms with
realistic field shapes (channel, OAuth status, bandwidth budget) so the UI
language stays consistent across "real" FCast pages and these speculative
placeholders.

---

## Moblin source pattern (SwiftUI → Slint mapping)

Moblin's destination pages are nearly identical: a `Form` with sections for
*Account* (OAuth) and *Channel* (stream key, broadcaster ID, etc.).

```swift
// Twitch settings (excerpt) — Moblin/View/Settings/Streams/Stream/Twitch/StreamTwitchSettingsView.swift
Form {
    Section { Text("Channel name"); TextEditNavigationView(value: $stream.twitchChannelName) }
    Section { Toggle("Logged in", isOn: $stream.twitchLoggedIn) }
}
```

In Slint this is a stack of `SettingsTextRow` / `SettingsValueRow` / a
"Sign in" button that flips a local `mock-logged-in` bool. No real OAuth
flow.

---

## Tasks

### 29-A — Add Panel variants

- [ ] In `bridge.slint`, add to the `Panel` enum:
  `destinations`, `destination-twitch`, `destination-kick`,
  `destination-youtube`, `destination-soop`, `destination-obs-remote`,
  `destination-realtime-irl`, `destination-osp`, `multi-streaming`,
  `go-live-notification`.

### 29-B — `DestinationsSettingsPage` (root list)

- [ ] Create `senders/android/ui/pages/destinations_settings_page.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { Bridge, Panel } from "../bridge.slint";
  import { TextButton } from "../components/buttons.slint";
  import { SettingsValueRow, SettingsToggleRow, SettingsSection }
      from "../components/settings_rows.slint";

  export struct Destination {
      id: string,
      label: string,
      sub-label: string,         // "Connected as <user>" / "Not configured"
      enabled: bool,
      panel: Panel,              // route to per-destination page
  }

  export component DestinationsSettingsPage inherits Rectangle {
      in-out property <[Destination]> mock-destinations: [
          { id: "twitch",        label: "Twitch",     sub-label: "Not signed in",
            enabled: false, panel: Panel.destination-twitch },
          { id: "kick",          label: "Kick",       sub-label: "Not signed in",
            enabled: false, panel: Panel.destination-kick },
          { id: "youtube",       label: "YouTube",    sub-label: "Not signed in",
            enabled: false, panel: Panel.destination-youtube },
          { id: "soop",          label: "Soop",       sub-label: "Not signed in",
            enabled: false, panel: Panel.destination-soop },
          { id: "obs-remote",    label: "OBS remote control",
            sub-label: "Localhost:4455", enabled: false,
            panel: Panel.destination-obs-remote },
          { id: "realtime-irl",  label: "RealtimeIRL", sub-label: "No pull key",
            enabled: false, panel: Panel.destination-realtime-irl },
          { id: "osp",           label: "OpenStreamingPlatform",
            sub-label: "Not configured", enabled: false,
            panel: Panel.destination-osp },
      ];

      background: Theme.surface-primary;
      VerticalLayout {
          padding: Theme.padding-screen; spacing: Theme.spacing-md;

          HorizontalLayout {
              alignment: space-between;
              Text { text: "Destinations"; color: Theme.text-primary;
                     font-size: Theme.font-size-title;
                     font-weight: FontWeight.semi-bold; }
              TextButton { label: "Done";
                  clicked => { Bridge.active-panel = Panel.none; } }
          }

          SettingsSection { title: "PLATFORMS";
              for d in root.mock-destinations: SettingsValueRow {
                  label: d.label; value: d.sub-label;
                  clicked => { Bridge.active-panel = d.panel; }
              }
          }

          SettingsSection { title: "AGGREGATE";
              SettingsValueRow { label: "Multi-streaming"; value: "0 active";
                  clicked => { Bridge.active-panel = Panel.multi-streaming; } }
              SettingsValueRow { label: "Go-live notification"; value: "Off";
                  clicked => { Bridge.active-panel = Panel.go-live-notification; } }
          }
      }
  }
  ```

### 29-C — Per-destination page template

- [ ] Each destination page follows the same template. Example for
  Twitch (`destination_twitch_settings_page.slint`):

  ```slint
  import { Theme } from "../theme.slint";
  import { Bridge, Panel } from "../bridge.slint";
  import { TextButton, PrimaryButton, DestructiveButton } from "../components/buttons.slint";
  import { SettingsTextRow, SettingsValueRow, SettingsToggleRow, SettingsSection }
      from "../components/settings_rows.slint";

  export component DestinationTwitchSettingsPage inherits Rectangle {
      in-out property <bool>   mock-logged-in: false;
      in-out property <string> mock-channel-name: "";
      in-out property <string> mock-broadcaster-id: "";
      in-out property <bool>   mock-enable-emotes: true;

      background: Theme.surface-primary;
      VerticalLayout {
          padding: Theme.padding-screen; spacing: Theme.spacing-md;

          HorizontalLayout {
              alignment: space-between;
              Text { text: "Twitch"; color: Theme.text-primary;
                     font-size: Theme.font-size-title;
                     font-weight: FontWeight.semi-bold; }
              TextButton { label: "Back";
                  clicked => { Bridge.active-panel = Panel.destinations; } }
          }

          SettingsSection { title: "ACCOUNT";
              SettingsValueRow { label: "Status";
                  value: root.mock-logged-in ? "Signed in" : "Not signed in"; }
              if !root.mock-logged-in:
              PrimaryButton { label: "Sign in with Twitch";
                  clicked => { root.mock-logged-in = true; } }
              if root.mock-logged-in:
              DestructiveButton { label: "Sign out";
                  clicked => { root.mock-logged-in = false; } }
          }

          SettingsSection { title: "CHANNEL";
              SettingsTextRow { label: "Channel name"; placeholder: "your_channel";
                                text: root.mock-channel-name;
                                edited(s) => { root.mock-channel-name = s; } }
              SettingsTextRow { label: "Broadcaster ID"; placeholder: "(auto)";
                                text: root.mock-broadcaster-id;
                                edited(s) => { root.mock-broadcaster-id = s; } }
              SettingsToggleRow { label: "Enable emotes";
                                  checked: root.mock-enable-emotes;
                                  toggled => { root.mock-enable-emotes = !root.mock-enable-emotes; } }
          }
      }
  }
  ```

  > `SettingsTextRow` is a new component that Phase 27 (utils backlog) will
  > pull from on demand — wrap a `LineEdit` with the `SettingsRow` chrome.

### 29-D — Kick / YouTube / Soop pages

- [ ] Repeat the 29-C template with platform-specific fields:
  - **Kick:** username, slug, "uses Kick PusherJS WS" footer text.
  - **YouTube:** channel ID, video ID picker placeholder ("auto-detect
    upcoming livestream" — a `SettingsValueRow` cycler with
    `["Latest", "Manual ID"]`).
  - **Soop:** username, password (LineEdit `password` mode), captcha
    placeholder banner.

### 29-E — `DestinationObsRemoteSettingsPage`

- [ ] OBS-WebSocket fields: host, port, password (LineEdit `input-type:
  password`), connect button (toggles `mock-connected`), polling
  interval slider.

### 29-F — `DestinationRealtimeIrlSettingsPage`

- [ ] Pull key text field, push interval slider (5–60 seconds), `mock-pushes-sent` counter (read-only).

### 29-G — `DestinationOspSettingsPage`

- [ ] Server URL field, channel field, "Test connection" button (no-op).

### 29-H — `MultiStreamingSettingsPage`

- [ ] List of destinations with per-destination toggle. Bandwidth
  estimate footer text ("Estimated upload: 7.5 Mbps").

### 29-I — `GoLiveNotificationSettingsPage`

- [ ] Toggle "Send push when going live", recipient text field
  (placeholder), template multi-line text field.

---

## Exit criteria

1. `DestinationsSettingsPage` lists 7 platforms; each routes to its own
   page via `Bridge.active-panel`.
2. "Sign in with Twitch" toggles local `mock-logged-in` boolean and
   updates the status label without any Rust callback.
3. All text fields persist their value across the page (using
   `in-out property <string>` + 2-way binding).
4. `cargo build -p android-sender` passes.
5. No `Bridge.<destination>-*` properties exist.

---

## What's NOT in this phase

- Any actual OAuth / token storage.
- Any RTMP / SRT / RIST / WHIP / WebSocket client.
- Real Twitch Helix / Kick / YouTube Live API calls.
- Real OBS-WebSocket protocol handshake.
- Real RealtimeIRL HTTP push.
- Validation of stream keys, channel IDs, captcha solving.

---

## Slint best practices applied here

- **One sub-page per destination** — keeps each Panel small and
  reviewable in `slint-viewer`. Routing centralises in the `Panel` enum
  on `Bridge`.
- **`SettingsTextRow`** pattern (LineEdit-in-row) avoids each page
  re-implementing text input chrome.
- **`if root.mock-logged-in:` / `if !root.mock-logged-in:`** uses
  Slint's conditional element instantiation rather than a `visible:`
  property — the unused branch is never created, matching what real
  conditional sign-in flows would do.

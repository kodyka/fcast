# Phase 30 — Streams Configuration & Wizards Placeholder

> **UI-only.** Streams root list, per-stream configuration root, and the
> first-run "create a stream" wizard. **No real protocol clients, no actual
> stream lifecycle.** Pure UI scaffolding.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 29 (per-platform pages), 31 (per-protocol pages)
**Functional integration:** Permanently deferred — FCast does not author RTMP/SRT/RIST/WHIP streams.

**Moblin source analogues** (~30 files, including wizards):
- `View/Settings/Streams/StreamsSettingsView.swift` — root list
- `View/Settings/Streams/Stream/StreamSettingsView.swift` — per-stream root
- `View/Settings/Streams/Stream/StreamWizardSettingsView.swift`
- `View/Settings/Streams/Stream/Audio/StreamAudioSettingsView.swift`
- `View/Settings/Streams/Stream/Video/StreamVideoSettingsView.swift`
- `View/Settings/Streams/Stream/Url/StreamUrlSettingsView.swift`
- `View/Settings/Streams/Stream/Recording/{StreamRecordingSettingsView,StreamRecordingAudioSettingsView}.swift`
- `View/Settings/Streams/Stream/Snapshot/StreamSnapshotSettingsView.swift`
- `View/Settings/Streams/Stream/Wizard/Custom/{StreamWizardCustomSettingsView,StreamWizardCustomRtmpSettingsView,StreamWizardCustomSrtSettingsView,StreamWizardCustomRistSettingsView,StreamWizardCustomWhipSettingsView}.swift`
- `View/Settings/Streams/Stream/Wizard/NetworkSetup/{StreamWizardNetworkSetupSettingsView,…BelaboxSettingsView,…DirectSettingsView,…ObsSettingsView}.swift`
- `View/Settings/Streams/Stream/Wizard/NetworkSetup/MyServers/{StreamWizardNetworkSetupMyServersSettingsView,…RtmpSettingsView,…SrtSettingsView}.swift`
- `View/Settings/Streams/Stream/Wizard/Platform/{StreamWizardKickSettingsView,StreamWizardObsSettingsView,StreamWizardSoopSettingsView,StreamWizardTwitchSettingsView,StreamWizardYouTubeSettingsView}.swift`
- `View/Settings/Streams/Stream/Wizard/{StreamWizardGeneralSettingsView,StreamWizardObsRemoteControlSettingsView}.swift`

**Files to add:**
- `senders/android/ui/pages/streams_settings_page.slint` — root list (with reorder + duplicate)
- `senders/android/ui/pages/stream_settings_page.slint` — per-stream root
- `senders/android/ui/pages/stream_audio_settings_page.slint`
- `senders/android/ui/pages/stream_video_settings_page.slint`
- `senders/android/ui/pages/stream_url_settings_page.slint`
- `senders/android/ui/pages/stream_recording_settings_page.slint`
- `senders/android/ui/pages/stream_snapshot_settings_page.slint`
- `senders/android/ui/pages/stream_wizard_settings_page.slint` — multi-step
- `senders/android/ui/components/wizard_step_chrome.slint` — Back / Next chrome

---

## Goal

Render the streams list (with reorder/duplicate/delete) plus the per-stream
configuration tree, then implement a multi-step wizard chrome that lets the
user click through 4–5 stub steps without committing anything.

---

## Moblin source pattern (SwiftUI → Slint mapping)

The streams list uses SwiftUI's `.swipeActions` + `.contextMenu` (delete,
duplicate, reorder via `DraggableItemPrefixView`). In Slint, swipe actions
do not exist as a built-in primitive (covered by Phase 27); the placeholder
renders the same actions as a long-press `Popup` instead.

```swift
// View/Settings/Streams/StreamsSettingsView.swift (excerpt)
NavigationLink { StreamSettingsView(...) } label: {
    HStack {
        DraggableItemPrefixView()
        Toggle(stream.name, isOn: ...)
    }
}
.swipeActions(edge: .trailing, allowsFullSwipe: false) {
    SwipeLeftToDeleteButtonView { ... }
    SwipeLeftToDuplicateButtonView { ... }
}
```

In Slint, the same row uses `HorizontalLayout { handle / label / toggle / ⋯ }`
with a long-press `Popup` exposing duplicate / delete / reorder. The
reorder is via ▲/▼ buttons (Phase 17 already established this pattern).

---

## Tasks

### 30-A — Add Panel variants

- [ ] In `bridge.slint`'s `Panel` enum: `streams`, `stream`, `stream-audio`,
  `stream-video`, `stream-url`, `stream-recording`, `stream-snapshot`,
  `stream-wizard`.

### 30-B — `StreamsSettingsPage` (root list)

- [ ] Inline mock model:

  ```slint
  export struct StreamConfig {
      id: int,
      name: string,
      url-protocol: string,        // "rtmp://" / "srt://" / etc.
      enabled: bool,
      is-current: bool,
  }

  export component StreamsSettingsPage inherits Rectangle {
      in-out property <[StreamConfig]> mock-streams: [
          { id: 1, name: "Kick (RTMP)",   url-protocol: "rtmp://",  enabled: true,  is-current: true  },
          { id: 2, name: "Twitch (RTMP)", url-protocol: "rtmp://",  enabled: false, is-current: false },
          { id: 3, name: "Belabox (SRT)", url-protocol: "srt://",   enabled: false, is-current: false },
      ];
      // ... ListView with rows + "Add stream" button at bottom
  }
  ```

- [ ] Add stream → opens `StreamWizardSettingsPage` (a brand new
  `mock-streams` row is appended after the wizard's "Save" button).

### 30-C — `StreamSettingsPage` (per-stream root)

- [ ] Sections:
  - "GENERAL": name, enabled
  - "URL": current URL value (read-only summary), tap → `stream-url`
  - "VIDEO": resolution + framerate + bitrate summary, tap → `stream-video`
  - "AUDIO": bitrate + channels summary, tap → `stream-audio`
  - "RECORDING": "Off" / "Saving to <folder>", tap → `stream-recording`
  - "SNAPSHOT": "Off" / "X-second pre-buffer", tap → `stream-snapshot`
  - "DELETE": `DestructiveButton` "Delete stream" → opens `ConfirmDialog`
    (Phase 19) → on Yes, removes from `mock-streams`.

### 30-D — `StreamUrlSettingsPage`

- [ ] Single multi-line `TextEdit` field showing the encoded URL,
  protocol picker (RTMP/SRT/RIST/WHIP/SRTLA), "Reveal stream key"
  toggle. URL is computed as
  `"\{root.mock-protocol}://\{root.mock-host}/live/\{root.mock-stream-key}"`
  via Slint's binding system.

### 30-E — `StreamVideoSettingsPage`

- [ ] Resolution picker (`SettingsValueRow` cycler over
  `["1080p60", "1080p30", "720p60", "720p30", "480p30"]`), bitrate
  slider (1000–12 000 kbps), keyframe interval picker (1/2/4 s),
  codec picker (`["H.264", "H.265", "AV1"]` — the *picker* is shown,
  no real codec capability switch).

### 30-F — `StreamAudioSettingsPage`

- [ ] Codec picker (`["AAC-LC", "Opus"]`), bitrate slider
  (32–320 kbps), channel picker (Mono / Stereo / 5.1).

### 30-G — `StreamRecordingSettingsPage` + `StreamRecordingAudioSettingsPage`

- [ ] Toggle "Record while streaming", folder picker stub
  (`SettingsValueRow` "Selected folder: …" — no real folder
  picker, just a static string), separate audio bitrate/codec
  selectors.

### 30-H — `StreamSnapshotSettingsPage`

- [ ] Toggle "Capture snapshot every N seconds", duration slider,
  output folder stub.

### 30-I — `StreamWizardSettingsPage` (multi-step chrome)

- [ ] Steps as a numeric `mock-step: int` (0..4) with `Back` / `Next`
  / `Cancel` chrome:

  ```slint
  export component WizardStepChrome inherits Rectangle {
      in property <int> step;
      in property <int> total-steps;
      callback back();
      callback next();
      callback cancel();
      callback finish();

      VerticalLayout {
          // Stepper indicator
          HorizontalLayout {
              alignment: center; spacing: 8px;
              for i in total-steps: Rectangle {
                  width: 12px; height: 12px; border-radius: 6px;
                  background: i == step ? Theme.accent
                                         : (i < step ? Theme.success : Theme.surface-overlay);
              }
          }

          @children

          HorizontalLayout {
              alignment: space-between; spacing: 8px;
              TextButton { label: "Cancel"; clicked => { root.cancel(); } }
              if step > 0: TextButton { label: "Back"; clicked => { root.back(); } }
              if step < total-steps - 1:
                  PrimaryButton { label: "Next"; clicked => { root.next(); } }
              if step == total-steps - 1:
                  PrimaryButton { label: "Finish"; clicked => { root.finish(); } }
          }
      }
  }
  ```

  Step content rendered with `if root.mock-step == 0: ...; if == 1: ...`
  branches, each step pulling from one of the existing pages
  (`StreamUrlSettingsPage`, `StreamVideoSettingsPage`, etc.) or a
  wizard-specific subpanel.

- [ ] Step ordering for the *General* wizard:
  1. Pick platform (Twitch / Kick / YouTube / Soop / OBS / Custom)
  2. (if Custom) Pick protocol (RTMP / SRT / RIST / WHIP)
  3. Network setup branch: Direct / Belabox / OBS / "My servers"
  4. Per-protocol settings (delegate to `StreamRtmp/Srt/Rist/Whip` from Phase 31)
  5. Confirm / save

### 30-J — Wizard Custom branches

- [ ] `stream_wizard_custom_settings_page.slint`: protocol picker.
- [ ] `stream_wizard_custom_rtmp.slint`, `…srt.slint`, `…rist.slint`,
  `…whip.slint`: each shows the same form as the per-protocol pages
  in Phase 31 but inside the wizard chrome.

### 30-K — Wizard NetworkSetup branches

- [ ] `stream_wizard_network_direct.slint`: server URL.
- [ ] `stream_wizard_network_belabox.slint`: Belabox cloud picker
  (region select + ingest URL preview).
- [ ] `stream_wizard_network_obs.slint`: localhost / LAN OBS server.
- [ ] `stream_wizard_network_my_servers_*.slint`: bookmark a custom
  server set; renders a list of `mock-servers: [{ name, url }]`.

### 30-L — Wizard Platform branches

- [ ] `stream_wizard_twitch.slint`, `…kick.slint`, `…youtube.slint`,
  `…soop.slint`: 2-step OAuth-flavored flow (Step 1: "Sign in" button
  flips `mock-logged-in`, Step 2: confirm channel name).

### 30-M — Wizard General + ObsRemoteControl

- [ ] `stream_wizard_general.slint`: name + per-stream description text.
- [ ] `stream_wizard_obs_remote.slint`: OBS-WebSocket host/port/password.

---

## Exit criteria

1. `StreamsSettingsPage` lists 3 mock streams; "Add stream" opens the
   wizard.
2. Wizard cycles through 5 steps; "Finish" appends a new mock row.
3. Each per-stream subsection renders.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real RTMP / SRT / RIST / WHIP / SRTLA client implementations.
- Real Belabox cloud reservation API.
- Real OBS-WebSocket auth.
- Recording or snapshot file output.
- Codec capability discovery (the codec picker is a stub).

---

## Slint best practices applied here

- **`@children` slot in `WizardStepChrome`** allows each step's body to
  be inlined inside the chrome without a `Component<T>` generic system.
  See [Component slot docs](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/component.mdx).
- **`if i < step:`** instead of imperative state mutation for the
  stepper indicator — Slint re-evaluates conditional elements when
  bindings change.
- **String interpolation `"\{root.mock-protocol}://\{root.mock-host}/…"`**
  computes the URL preview reactively — no manual string concatenation
  in callbacks.

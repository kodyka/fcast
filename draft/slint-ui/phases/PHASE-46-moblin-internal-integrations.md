# Phase 46 — Moblin-internal Integrations Placeholder (Moblink / OpenAI / RemoteControl)

> **REFERENCE-ONLY UI.** Moblink relay configuration, OpenAI API integration,
> and Remote-Control assistant pages. **Never instantiated** by Android
> `MainWindow`. These are Moblin-specific protocols / cloud APIs FCast
> sender does not author.

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3
**Functional integration:** **Never wired.** Reference-only files under
`senders/android/ui/pages/_apple/` (path overloaded as "non-Android
reference").

**Moblin source analogues** (4–5 files):
- `View/Settings/Moblink/{MoblinkRelaySettingsView,MoblinkStreamerSettingsView}.swift`
- `View/Settings/OpenAi/OpenAiSettingsView.swift`
- `View/Settings/RemoteControl/{RemoteControlAssistantSettingsView,RemoteControlStreamerSettingsView,RemoteControlAssistantPreviewView,RemoteControlStreamerStatusView}.swift`

**Files to add (under `_apple/` reference path):**
- `_apple/moblink_relay_page.slint`
- `_apple/moblink_streamer_page.slint`
- `_apple/openai_settings_page.slint`
- `_apple/remote_control_assistant_page.slint`
- `_apple/remote_control_streamer_page.slint`
- `_apple/remote_control_assistant_preview_page.slint`
- `_apple/remote_control_streamer_status_page.slint`

---

## Goal

Surface Moblink (Moblin's bonded-bandwidth relay) and OpenAI / Remote-Control
configuration as reference Slint files. Pure design completeness.

---

## Moblin source pattern

```swift
// MoblinkRelaySettingsView.swift (excerpt)
Form {
    Section {
        Toggle("Enabled", isOn: $relay.enabled)
        TextEditNavigationView(value: $relay.streamerUrl)
        TextEditNavigationView(value: $relay.password)
    }
    Section { Picker("Network priority", selection: $relay.priority) {...} }
}
```

```swift
// OpenAiSettingsView.swift (excerpt)
Form {
    Section { TextField("API key", text: $openAi.apiKey) }
    Section { Picker("Model", selection: $openAi.model) {...} }
}
```

---

## Tasks

### 46-A — `MoblinkRelayPage`

- [ ] Toggle "Enable as relay node", streamer URL field, password
  LineEdit (`password` mode), priority picker (`["High", "Normal",
  "Background"]`), bandwidth-budget cycler, status panel
  ("Connected to streamer:port • 4.2 Mbps").

### 46-B — `MoblinkStreamerPage`

- [ ] Toggle "Accept Moblink relays", port field, password field,
  list of `mock-connected-relays: [{ name: string, bandwidth-mbps:
  float, latency-ms: int, ip: string }]`.

### 46-C — `OpenAiSettingsPage`

- [ ] API key field (`password` mode), base URL field (default
  "https://api.openai.com/v1"), model cycler
  (`["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"]`), system-prompt
  multi-line text field, "use as chat-bot" toggle.

### 46-D — `RemoteControlAssistantPage`

- [ ] Server URL, password, "Connect" button, status panel.

### 46-E — `RemoteControlStreamerPage`

- [ ] Same fields as Assistant, mirror direction.

### 46-F — `RemoteControlAssistantPreviewPage`

- [ ] Embedded preview placeholder (large rectangle with "Streamer view"
  label) + per-action buttons (zoom in/out, focus area, panic stop).

### 46-G — `RemoteControlStreamerStatusPage`

- [ ] Read-only status grid: bitrate / dropped frames / temperature /
  battery / network — each as a `SettingsValueRow` with a static
  value.

---

## Exit criteria

1. All 7 files exist under `_apple/`.
2. None imported by `main.slint`.
3. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Any Moblink protocol / WebSocket implementation.
- Any OpenAI HTTP client.
- Any Remote-Control assistant / streamer wire protocol.

---

## Slint best practices applied here

- **`LineEdit { input-type: password; }`** for the OpenAI API key
  follows the same convention as Phase 31's stream-key fields.
- **Read-only status grids** built from `SettingsValueRow` (no
  `clicked` callback) — the unified row chrome works equally for
  read-only and editable surfaces.

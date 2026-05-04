# Phase 32 — Ingests Servers Placeholder

> **UI-only.** Per-server (RTMP / RIST / SRTLA / WHEP / WHIP / RTSP) inbound
> ingest configuration pages. **No real listening sockets, no actual
> stream demuxing.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7
**Functional integration:** Permanently deferred — FCast is a sender, not an ingest server.

**Moblin source analogues** (13 files):
- `View/Settings/Ingests/IngestsSettingsView.swift` — root
- `View/Settings/Ingests/RtmpServer/{RtmpServerSettingsView,RtmpServerStreamSettingsView}.swift`
- `View/Settings/Ingests/RistServer/{RistServerSettingsView,RistServerStreamSettingsView}.swift`
- `View/Settings/Ingests/SrtlaServer/{SrtlaServerSettingsView,SrtlaServerStreamSettingsView}.swift`
- `View/Settings/Ingests/WhipServer/{WhipServerSettingsView,WhipServerStreamSettingsView}.swift`
- `View/Settings/Ingests/WhepClient/{WhepClientSettingsView,WhepClientStreamSettingsView}.swift`
- `View/Settings/Ingests/RtspClient/{RtspClientSettingsView,RtspClientStreamSettingsView}.swift`

**Files to add:**
- `senders/android/ui/pages/ingests_settings_page.slint` — root
- `senders/android/ui/pages/ingest_<server>_settings_page.slint` × 6 (rtmp, rist, srtla, whip, whep, rtsp)
- `senders/android/ui/pages/ingest_<server>_stream_settings_page.slint` × 6

---

## Goal

Render an "Ingests" panel where the user can configure one or more inbound
servers, each with its own per-stream sub-list. Pure placeholder — no real
TCP/UDP listeners.

---

## Moblin source pattern

```swift
// View/Settings/Ingests/RtmpServer/RtmpServerSettingsView.swift (excerpt)
Form {
    Section {
        Toggle("Enabled", isOn: $rtmpServer.enabled)
        TextField("Port", text: $rtmpServer.port)
    }
    Section("Streams") {
        ForEach(rtmpServer.streams) { stream in
            NavigationLink { RtmpServerStreamSettingsView(stream: stream) } label: {
                Text(stream.name)
            }
        }
    }
}
```

A two-level pattern (server → list of streams → per-stream).

---

## Tasks

### 32-A — Add Panel variants

- [ ] `Panel.ingests`, `Panel.ingest-rtmp`, `Panel.ingest-rist`,
  `Panel.ingest-srtla`, `Panel.ingest-whip`, `Panel.ingest-whep`,
  `Panel.ingest-rtsp`, plus `Panel.ingest-<x>-stream` for each.

### 32-B — `IngestsSettingsPage`

- [ ] Inline mock model:

  ```slint
  export struct IngestServer {
      id: string,                 // "rtmp" / "rist" / "srtla" / ...
      label: string,
      enabled: bool,
      port: int,
      stream-count: int,
      panel: Panel,
  }

  export component IngestsSettingsPage inherits Rectangle {
      in-out property <[IngestServer]> mock-servers: [
          { id: "rtmp",  label: "RTMP server",   enabled: false, port: 1935, stream-count: 0, panel: Panel.ingest-rtmp },
          { id: "rist",  label: "RIST server",   enabled: false, port: 1968, stream-count: 0, panel: Panel.ingest-rist },
          { id: "srtla", label: "SRTLA server",  enabled: false, port: 5000, stream-count: 0, panel: Panel.ingest-srtla },
          { id: "whip",  label: "WHIP server",   enabled: false, port: 8080, stream-count: 0, panel: Panel.ingest-whip },
          { id: "whep",  label: "WHEP client",   enabled: false, port: 0,    stream-count: 1, panel: Panel.ingest-whep  },
          { id: "rtsp",  label: "RTSP client",   enabled: false, port: 0,    stream-count: 0, panel: Panel.ingest-rtsp  },
      ];
      // ListView with status pills (port + stream count + enabled)
  }
  ```

### 32-C — Per-server pages

Each `ingest_<server>_settings_page.slint` follows the same template:

```slint
SettingsSection { title: "GENERAL";
    SettingsToggleRow { label: "Enabled";
                        checked: root.mock-enabled;
                        toggled => { root.mock-enabled = !root.mock-enabled; } }
    SettingsValueRow  { label: "Port"; value: "\{root.mock-port}";
                        clicked => { root.mock-port = root.mock-port == 1935 ? 1936 : 1935; } }
}
SettingsSection { title: "STREAMS";
    for s in root.mock-streams: SettingsValueRow { label: s.name; value: "\{s.bitrate-kbps} kbps";
        clicked => { Bridge.active-panel = Panel.ingest-rtmp-stream; } }
    PrimaryButton { label: "Add stream";
        clicked => {
            root.mock-streams = [
                ...root.mock-streams,
                { id: root.mock-streams.length + 1, name: "Stream \{root.mock-streams.length + 1}", bitrate-kbps: 4000 }
            ];
        } }
}
```

- **RTMP / RIST / SRTLA / WHIP** — `port`, `stream-count`, list of streams.
- **WHEP / RTSP** — these are *clients*, so the page surfaces a *URL*
  field instead of a port, and the streams list is replaced with a
  single stream summary.

### 32-D — Per-stream sub-pages

- [ ] Each `ingest_<server>_stream_settings_page.slint` exposes:
  - Name (`SettingsTextRow`)
  - Stream key / publish URL (LineEdit `password` mode)
  - Per-stream stats (read-only: connection state, bitrate, RTT)
  - Delete button → removes from `mock-streams` of the parent.

---

## Exit criteria

1. `IngestsSettingsPage` lists 6 ingest servers/clients with status pills.
2. Tapping a server opens its page; "Add stream" appends a row.
3. Per-stream sub-pages render and Delete removes correctly.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real RTMP / RIST / SRTLA / WHIP listening sockets.
- Real WHEP / RTSP client connections.
- Real per-stream bitrate / RTT measurement.
- Authentication / ACL on inbound streams.

---

## Slint best practices applied here

- **`...root.mock-streams` spread syntax** is the only safe way to
  mutate a `[T]` property in Slint — assignment to a copy with one new
  row appended. Direct push is not supported.
- **One `Panel` per server type** keeps the routing flat — enum stays
  legible up to ~30 panels before it should be split into namespaced
  enums.

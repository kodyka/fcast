# Phase 26 — Debug Log Viewer Placeholder

> A scrolling log viewer for in-app debug events, plus a video-debug overlay
> toggle. **UI-only.** Log entries come from inline mock data; no real
> `tracing` / `logcat` capture.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — would tail Rust `tracing` events
asynchronously, requires `Bridge.log-events` model + `tracing-subscriber` layer.
**Moblin source analogues:**
- `Settings/Debug/DebugLogSettingsView.swift`
- `Settings/Debug/DebugVideoSettingsView.swift`

**Files:**
- `senders/android/ui/pages/debug_log_page.slint` — new
- `senders/android/ui/pages/debug_video_page.slint` — new
- `senders/android/ui/bridge.slint` — `LogLevel` enum + `LogEntry` struct + `Panel.debug-log`, `Panel.debug-video`

---

## Tasks

### 26-A — `LogEntry` struct

- [ ] In `bridge.slint`:

  ```slint
  export enum LogLevel { trace, debug, info, warning, error }

  export struct LogEntry {
      level:     LogLevel,
      timestamp: string,
      target:    string,
      message:   string,
  }
  ```

---

### 26-B — `DebugLogPage`

- [ ] Inline mock model with a representative spread of levels:

  ```slint
  in-out property <[LogEntry]> mock-log: [
      { level: LogLevel.info,    timestamp: "12:34:56.012", target: "fcast::discovery", message: "mDNS scan started" },
      { level: LogLevel.debug,   timestamp: "12:34:56.087", target: "fcast::discovery", message: "Resolved Living Room TV at 192.168.1.50" },
      { level: LogLevel.warning, timestamp: "12:34:56.220", target: "fcast::net",       message: "Reconnect attempt 1/3" },
      { level: LogLevel.error,   timestamp: "12:34:56.510", target: "fcast::encoder",   message: "Encoder negotiation failed: H264 not advertised" },
      { level: LogLevel.trace,   timestamp: "12:34:56.612", target: "slint",            message: "Layout pass 423 (12ms)" },
  ];
  in-out property <int> mock-min-level-idx: 1; // 0=trace .. 4=error
  ```

- [ ] Header: filter chips for each level (Trace / Debug / Info / Warning / Error).
  Tapping a chip sets `mock-min-level-idx`. Show only entries whose level
  index ≥ `mock-min-level-idx`.
- [ ] Body: `ListView` (NOT `ScrollView` — let Slint virtualize). Each row:
  level color stripe (4px wide), timestamp (mono font), target (dimmed),
  message (full width, `wrap: word-wrap`).
- [ ] Bottom toolbar: "Clear" + "Copy all" (no-op in UI-only build).

---

### 26-C — `DebugVideoPage`

- [ ] Sections:

  ```
  PIPELINE OVERLAY
    Show element graph        toggle
    Show buffer timestamps    toggle
    Show negotiated caps      toggle
    Show keyframe markers     toggle

  PIPELINE STATE
    Element                  State (read-only mock)
    src                      PLAYING
    videoconvert             PLAYING
    x264enc                  PLAYING
    rtph264pay               PLAYING
    udpsink                  PLAYING
  ```

- [ ] **Build check.**

---

### 26-D — Bridge + linking

- [ ] Extend `Panel`: `debug-log`, `debug-video`.
- [ ] Route in `main.slint`.
- [ ] In `FullSettingsPage` "CODEC & DEBUG" section, replace the existing
  "Show debug panel" toggle with two value rows: "Debug log" → `debug-log`,
  "Video pipeline" → `debug-video`. Keep the old toggle if you want backward
  compatibility — the new pages are additive.

---

## Exit criteria

1. Debug log page renders 5 stub entries with level color stripes.
2. Filter chips reduce visible entries by min level.
3. ListView virtualizes (test by inflating `mock-log` to 200 entries; scroll
   stays smooth).
4. Debug video page renders 4 toggles + element-state read-only table.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Live `tracing` event capture.
- Log file export / sharing.
- Real GStreamer pipeline introspection.
- Search / regex filtering of log messages.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Debug/DebugLogSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Debug/DebugLogSettingsView.swift (excerpt)
Form {
    Section {
        ForEach(model.debugLog, id: \.self) { line in
            Text(line).font(.system(size: 11, design: .monospaced))
        }
    }
    .listRowBackground(Color.black)
    .foregroundColor(.green)
}
```

**Mapping notes:**

A monospace log viewer. In Slint a `ListView` (for virtualization since
logs can be 10k+ lines) of `Text { font-family: "monospace"; }` rows.
Severity color comes from a regex match against the line content
(parsed in the placeholder by inspecting the first character: `E` →
red, `W` → orange, default → green).

**Relevant Slint docs:**
- [ListView](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx)
- [font-family](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/text.mdx)

## Slint best practices applied here

- **`ListView` for the log, not `ScrollView`** — log content can grow without
  bound and virtualization matters. Reference:
  [`reference/std-widgets/views/listview.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx).
- **Level filter as `min-idx` int rather than per-level booleans** keeps
  filter logic to a single comparison per row. Cheaper than 5 boolean checks.
- **4px color stripe instead of full-row tint** preserves readability —
  full-row tint on warnings/errors makes long messages painful to read.

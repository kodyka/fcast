# Phase 23 — Local Recording Placeholder

> Settings sub-page + quick-action surface for local recording (record the
> cast to a local file). **UI-only — no real `MediaRecorder` integration.**

**Status:** `[ ] Not started — blocked by Rust recording capability for live data, but UI placeholder is unblocked`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — Android `MediaRecorder` /
GStreamer file sink requires storage permission + capture pipeline
muxer changes.
**Moblin source analogues:**
- `Settings/Recordings/RecordingsSettingsView.swift`

**Files:**
- `senders/android/ui/pages/recording_page.slint` — new
- `senders/android/ui/bridge.slint` — `RecordingState` enum + `Panel.recording`

---

## Tasks

### 23-A — `RecordingState` enum

- [ ] In `bridge.slint`:

  ```slint
  export enum RecordingState {
      idle,
      recording,
      paused,
      finalizing,
  }
  ```

---

### 23-B — `RecordingPage`

- [ ] Inline stub state:

  ```slint
  in-out property <RecordingState> mock-state: RecordingState.idle;
  in-out property <int>             mock-elapsed-s: 0;
  in-out property <int>             mock-format-idx: 0;     // MP4 / MKV / WebM
  in-out property <int>             mock-folder-idx: 0;     // App / Movies / Custom
  in-out property <bool>            mock-record-audio: true;
  in-out property <int>             mock-disk-free-mb: 12480;
  ```

- [ ] Sections:

  ```
  RECORDING
    Big record button (red dot when idle, square when recording, slim chevron when paused)
    Below: elapsed time HH:MM:SS (running counter when state == recording)
    Two-button row: Pause / Resume + Stop

  OUTPUT
    Format        MP4 / MKV / WebM   (cycle)
    Folder        App / Movies / Custom (cycle)
    Record audio  toggle
    Disk free     "12.2 GB" (read-only)
  ```

- [ ] Implement the elapsed counter using a Slint `Timer`:

  ```slint
  Timer {
      interval: 1s;
      running: root.mock-state == RecordingState.recording;
      triggered => { root.mock-elapsed-s += 1; }
  }
  ```

- [ ] Record button click cycles state:
  `idle → recording → paused → recording → ... ; long-press while recording → finalizing → idle`.
  In UI-only build, finalize is instant.

---

### 23-C — Bridge + linking

- [ ] Extend `Panel` with `recording`.
- [ ] Route in `main.slint`.
- [ ] Link from `FullSettingsPage` "ADVANCED" section.
- [ ] Add `quick-action` entry id `"record"` in the bar stub model. On click in
  Slint: `Bridge.active-panel = Panel.recording;`.

---

## Exit criteria

1. Page opens; record button cycles through visual states.
2. Elapsed counter ticks 1s when state == recording.
3. Format / folder cycle on click; audio toggle flips.
4. Disk-free read-only row shows formatted value.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real `MediaRecorder` start/stop.
- Real disk-free probe via `StatFs`.
- Real file output / Storage Access Framework integration.
- Trim / clip post-processing.
- Background-recording service.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Streams/Stream/Recording/StreamRecordingSettingsView.swift`
- `View/Settings/Recordings/RecordingsSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Streams/Stream/Recording/StreamRecordingSettingsView.swift (excerpt)
Form {
    Section { Toggle("Enabled", isOn: $stream.recording.enabled) }
    Section {
        Picker("Codec", selection: $stream.recording.videoCodec) {...}
        Slider(value: $stream.recording.videoBitrate, in: 1...20)
    }
    Section { TextField("Folder", text: $stream.recording.folder) }
}
```

**Mapping notes:**

A toggle + codec picker + bitrate slider + folder text field on a single
page. The folder picker is stubbed (no native dialog) — the placeholder
just stores the typed string. The recordings *browser* sits on a sibling
page (RecordingsSettingsView) with a `ListView` of past recordings.

**Relevant Slint docs:**
- [Slider](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx)

## Slint best practices applied here

- **`Timer { running: state == X }`** is the clean way to gate periodic
  updates by an enum value. Slint reactively starts/stops the timer.
- **Visual state cycling on a single button** (idle → recording → paused) is
  simpler than three separate buttons and matches typical recording-app UX.

# Phase 14 — Audio Capture Controls Placeholder

> Settings sub-page + quick-action surface for audio capture (mic source,
> output gain, mute). **UI-only.** All controls flip inline `in-out` properties.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7 (settings root)
**Functional integration:** Deferred — Android `AudioRecord` / GStreamer audio source not wired.
**Moblin source analogues:**
- `Settings/Audio/AudioSettingsView.swift`
- `ControlBar/QuickButton/QuickButtonMicView.swift`

**Files:**
- `senders/android/ui/pages/audio_page.slint` — new (panel-style page)
- `senders/android/ui/pages/settings_page.slint` — link from settings root

---

## Tasks

### 14-A — `AudioPage` panel

- [ ] Create `senders/android/ui/pages/audio_page.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { Bridge, Panel } from "../bridge.slint";
  import { TextButton } from "../components/buttons.slint";
  import {
      SettingsSection, SettingsToggleRow, SettingsValueRow, SettingsSliderRow
  } from "../components/settings_rows.slint";

  export component AudioPage inherits Rectangle {
      // UI-only stub state.
      in-out property <bool>  mock-muted:         false;
      in-out property <int>   mock-source-idx:    0;   // Mic / System / Both
      in-out property <float> mock-input-gain:    0.7; // 0..1
      in-out property <int>   mock-bitrate-idx:   1;   // 64 / 128 / 192 / 256 kbps

      background: Theme.surface-primary;

      VerticalLayout {
          // Header
          Rectangle {
              height: 56px;
              background: Theme.surface-card;
              HorizontalLayout {
                  padding: Theme.padding-screen;
                  Text {
                      text: "Audio";
                      color: Theme.text-primary;
                      font-size: Theme.font-size-heading;
                      vertical-alignment: center;
                      horizontal-stretch: 1;
                  }
                  TextButton { label: "Done"; clicked => { Bridge.active-panel = Panel.none; } }
              }
          }

          ScrollView {
              VerticalLayout {
                  spacing: Theme.spacing-section;
                  padding: Theme.padding-screen;

                  SettingsSection {
                      title: "INPUT";
                      SettingsValueRow {
                          label: "Source";
                          value: ["Microphone", "System audio", "Both"][root.mock-source-idx];
                          clicked => { root.mock-source-idx = (root.mock-source-idx + 1) mod 3; }
                      }
                      SettingsToggleRow {
                          label: "Mute";
                          checked: root.mock-muted;
                          toggled => { root.mock-muted = !root.mock-muted; }
                      }
                      SettingsSliderRow {
                          label: "Input gain";
                          value: root.mock-input-gain * 100;
                          minimum: 0; maximum: 100; unit: "%";
                          changed(v) => { root.mock-input-gain = v / 100; }
                      }
                  }

                  SettingsSection {
                      title: "ENCODING";
                      SettingsValueRow {
                          label: "Bitrate";
                          value: ["64 kbps", "128 kbps", "192 kbps", "256 kbps"][root.mock-bitrate-idx];
                          clicked => { root.mock-bitrate-idx = (root.mock-bitrate-idx + 1) mod 4; }
                      }
                      SettingsValueRow {
                          label: "Codec";
                          value: "AAC-LC";
                      }
                  }
              }
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 14-B — Add `Panel.audio` enum variant + main.slint route

- [ ] In `bridge.slint`, extend `Panel`: `none, settings, debug, codec-test, audio`.
- [ ] In `main.slint`, add `if Bridge.active-panel == Panel.audio: AudioPage { }`.
- [ ] **Build check.**

---

### 14-C — Link from `FullSettingsPage`

- [ ] In `settings_page.slint` `FullSettingsPage`, add a row in a new
  "AUDIO & VIDEO" section:

  ```slint
  SettingsValueRow {
      label: "Audio";
      value: "Open";
      clicked => { Bridge.active-panel = Panel.audio; }
  }
  ```

---

## Exit criteria

1. `AudioPage` opens from settings root and closes via Done button.
2. All four controls (source, mute toggle, gain slider, bitrate) flip stub state.
3. Slider drags update the percentage label live.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real `AudioRecord` source selection.
- Real GStreamer audio capsfilter / encoder bitrate change.
- VU meter / live level monitoring.
- Per-app audio source filtering (Android 10+ `AudioPlaybackCaptureConfiguration`).

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/Audio/AudioSettingsView.swift`
- `View/ControlBar/QuickButton/QuickButtonMicView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/Audio/AudioSettingsView.swift (excerpt)
Form {
    Section {
        NavigationLink { QuickButtonMicView(...) } label: {
            HStack { Text("Mic"); Spacer(); GrayTextView(text: mic.current.name) }
        }
        Toggle("Auto", isOn: $audio.autoSelect)
    }
    Section {
        Picker("Bitrate (kbps)", selection: $stream.audioBitrate) {
            ForEach([32, 64, 96, 128, 160, 192, 256, 320], id: \.self) { Text("\($0)") }
        }
        Picker("Codec", selection: $stream.audioCodec) {
            ForEach(SettingsAudioCodec.allCases, id: \.self) { Text($0.toString()) }
        }
    }
}
```

**Mapping notes:**

The `Form` → `Section` → `Picker` shape lines up with Slint's
`SettingsSection` → `SettingsValueRow` cycler pattern. Mic source picker
becomes a row whose `value` reads from `mock-mic-source` and whose `clicked`
cycles through the option array. Bitrate picker uses the same cycler over a
`property <[int]> bitrate-options-kbps: [32, 64, ...];`.

**Relevant Slint docs:**
- [ComboBox (alternative)](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/combobox.mdx)
- [Switch / Toggle](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/switch.mdx)

## Slint best practices applied here

- **`SettingsSliderRow` with `changed(v) => ...`** is the right pattern for
  continuous controls. Avoid binding `value` directly to a stub property
  without `changed`, or the slider becomes read-only.
- **Indexed-string value pickers** (`["A", "B", "C"][idx]`) keep the stub
  logic in one line per row.

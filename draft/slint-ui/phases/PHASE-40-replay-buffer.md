# Phase 40 — Replay Buffer Placeholder

> **UI-only.** Replay buffer settings page + per-replay viewer panel.
> **No real ring buffer, no MediaProjection capture loop.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 39 (HUD button)
**Functional integration:** Permanently deferred.

**Moblin source analogues** (2 files):
- `View/Settings/Replay/ReplaySettingsView.swift`
- `View/Stream/Replay/StreamReplayView.swift`

**Files to add:**
- `senders/android/ui/pages/replay_settings_page.slint`
- `senders/android/ui/pages/replay_viewer_page.slint`

---

## Goal

Render Moblin's replay UI: a settings page for buffer length / quality and
a viewer that lets the user scrub through saved replays.

---

## Moblin source pattern

```swift
// ReplaySettingsView.swift (excerpt)
Form {
    Section { Toggle("Enabled", isOn: $replay.enabled) }
    Section { Picker("Buffer length", selection: $replay.bufferLength) {...} }
    Section("Saved replays") {
        ForEach(replay.saved) { item in
            NavigationLink { ReplayViewerView(item: item) } label: {
                HStack { Text(item.label); Spacer(); Text(item.duration) }
            }
        }
    }
}
```

The viewer is a video scrubber + Save / Discard / Share buttons — in
Slint the video preview is replaced with a labelled placeholder
rectangle.

---

## Tasks

### 40-A — Add Panel variants

- [ ] `Panel.replay-settings`, `Panel.replay-viewer`.

### 40-B — `ReplaySettingsPage`

```slint
export struct ReplayItem {
    id: int,
    label: string,
    duration: string,         // "1m12s"
    saved-at: string,         // "12:34"
}

export component ReplaySettingsPage inherits Rectangle {
    in-out property <bool>          mock-enabled: false;
    in-out property <int>           mock-buffer-len-idx: 1;
    in-out property <[ReplayItem]>  mock-saved: [
        { id: 1, label: "First impressive moment", duration: "0m23s", saved-at: "12:08" },
        { id: 2, label: "Goal!",                   duration: "0m18s", saved-at: "12:32" },
    ];
    property <[string]> buffer-options: ["10s", "30s", "60s", "120s", "300s"];

    VerticalLayout {
        SettingsToggleRow { label: "Replay buffer enabled";
                            checked: root.mock-enabled;
                            toggled => { root.mock-enabled = !root.mock-enabled; } }
        SettingsValueRow  { label: "Buffer length";
                            value: buffer-options[root.mock-buffer-len-idx];
                            clicked => {
                                root.mock-buffer-len-idx =
                                    mod(root.mock-buffer-len-idx + 1, buffer-options.length);
                            } }
        SettingsSection { title: "SAVED REPLAYS";
            for r in root.mock-saved: SettingsValueRow {
                label: r.label; value: r.duration + " · " + r.saved-at;
                clicked => { Bridge.active-panel = Panel.replay-viewer; }
            }
        }
    }
}
```

### 40-C — `ReplayViewerPage`

```slint
in-out property <float> mock-scrubber-pos: 0.5;       // 0..1
in-out property <bool>  mock-playing: false;

VerticalLayout {
    // Video placeholder
    Rectangle {
        height: 220px; background: black; border-radius: Theme.radius-card;
        Text { text: root.mock-playing ? "▶ Playing" : "⏸ Paused";
               color: white; font-size: Theme.font-size-title;
               horizontal-alignment: center; vertical-alignment: center; }
    }

    // Scrubber
    Slider {
        minimum: 0; maximum: 100;
        value: root.mock-scrubber-pos * 100;
        changed(v) => { root.mock-scrubber-pos = v / 100; }
    }

    HorizontalLayout {
        spacing: 8px; alignment: space-between;
        TextButton { label: root.mock-playing ? "Pause" : "Play";
            clicked => { root.mock-playing = !root.mock-playing; } }
        TextButton { label: "Save"; }
        DestructiveButton { label: "Discard"; }
    }
}
```

---

## Exit criteria

1. Settings page lists 2 mock saved replays with toggleable buffer.
2. Viewer renders a placeholder + scrubber + play/save/discard buttons.
3. Cycler walks through 5 buffer-length options.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real ring buffer over MediaProjection / camera frames.
- Real video playback.
- Real save-to-file / share.

---

## Slint best practices applied here

- **`Slider` from `std-widgets.slint`** — see [Slider reference](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx) — exposes
  `changed(value: float)`; this matches the corrected
  Phase 3-H signature from the [CRIT] / [BUG] sweep.
- **`property <[string]> buffer-options: [...]`** is a static option
  list — declared at the page level and reused by the cycler.

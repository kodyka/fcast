# Phase 47 — Media Player & In-app Browser Placeholder

> **UI-only.** A standalone media player (audio/video file picker + scrubber)
> and a basic in-app browser surface. **No real playback, no real WebView.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 7, 40 (replay scrubber pattern)
**Functional integration:** Permanently deferred — FCast Android sender does
not need standalone media playback or a browser.

**Moblin source analogues** (~4 files):
- `View/Settings/MediaPlayer/{MediaPlayerSettingsView,MediaPlayerPlayerSettingsView,MediaPlayerPlayerFileSettingsView}.swift`
- `View/WebBrowser/WebBrowserView.swift`

**Files to add:**
- `senders/android/ui/pages/media_player_settings_page.slint` — root list
- `senders/android/ui/pages/media_player_player_page.slint`
- `senders/android/ui/pages/media_player_file_page.slint`
- `senders/android/ui/pages/web_browser_page.slint`

---

## Goal

Render Moblin's standalone playback / browse surfaces as placeholders so
the design language stays consistent.

---

## Moblin source pattern

```swift
// MediaPlayerSettingsView.swift (excerpt)
List {
    ForEach(database.mediaPlayers) { player in
        NavigationLink { MediaPlayerPlayerSettingsView(player: player) } label: {
            Text(player.name)
        }
    }
}
```

```swift
// MediaPlayerPlayerSettingsView.swift (excerpt)
List {
    Section("Files") {
        ForEach(player.files) { file in
            NavigationLink { MediaPlayerPlayerFileSettingsView(file: file) } label: {
                HStack { Text(file.name); Spacer(); Text(file.duration) }
            }
        }
    }
    Section { Toggle("Loop", isOn: $player.loop) }
}
```

---

## Tasks

### 47-A — Add Panel variants

- [ ] `Panel.media-player`, `Panel.media-player-instance`,
  `Panel.media-player-file`, `Panel.web-browser`.

### 47-B — `MediaPlayerSettingsPage`

```slint
export struct MediaPlayer {
    id: int,
    name: string,
    file-count: int,
    looping: bool,
}

in-out property <[MediaPlayer]> mock-players: [
    { id: 1, name: "BGM playlist",   file-count: 5, looping: true  },
    { id: 2, name: "Intro stinger",  file-count: 1, looping: false },
];

VerticalLayout {
    for p in root.mock-players: SettingsValueRow {
        label: p.name;
        value: "\{p.file-count} files" + (p.looping ? " · loop" : "");
        clicked => { Bridge.active-panel = Panel.media-player-instance; }
    }
    PrimaryButton { label: "Create player";
        clicked => { /* append to mock-players */ } }
}
```

### 47-C — `MediaPlayerPlayerPage`

- [ ] Sections: GENERAL (name + loop toggle), FILES list (`mock-files:
  [{ id, name, duration }]`), per-file row → `media-player-file`,
  CONTROLS (Play / Pause / Stop / Next placeholder buttons).

### 47-D — `MediaPlayerFilePage`

- [ ] File path display (read-only stub), trim-start / trim-end
  cyclers, gain slider.

### 47-E — `WebBrowserPage`

```slint
in-out property <string> mock-url: "https://fcast.org";
in-out property <bool>   mock-loading: false;

VerticalLayout {
    // Address bar
    HorizontalLayout {
        spacing: 4px;
        TextButton { label: "←"; }
        TextButton { label: "→"; }
        SettingsTextRow { label: ""; placeholder: "Enter URL";
                          text: root.mock-url;
                          edited(s) => { root.mock-url = s; } }
        TextButton { label: "Reload"; }
    }

    // Browser viewport placeholder
    Rectangle {
        background: Theme.surface-card;
        height: 320px;
        if root.mock-loading: LoadingView {}
        if !root.mock-loading: Text {
            text: "(WebView placeholder — \{root.mock-url})";
            color: Theme.text-secondary;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

---

## Exit criteria

1. `MediaPlayerSettingsPage` lists 2 stub players.
2. Per-player page lists files; per-file page shows trim/gain controls.
3. `WebBrowserPage` renders an address bar + viewport placeholder.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real audio/video playback (no AVPlayer / ExoPlayer).
- Real `WebView` (no Android `WebView` / Slint web component).
- File picker dialogs.

---

## Slint best practices applied here

- **`LoadingView` reuse** from Phase 3 keeps the loading shimmer
  consistent.
- **In-page address bar = `HorizontalLayout` of `TextButton`s plus a
  `SettingsTextRow`** — no custom address-bar component is necessary
  for the placeholder.

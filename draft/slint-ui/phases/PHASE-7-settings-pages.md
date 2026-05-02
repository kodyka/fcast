# Phase 7 — Settings Navigation and FCast-Specific Pages

> Build a real settings root page with FCast-relevant sections, replacing the current
> inline `SelectingSettingsView`. Inspired by Moblin `View/Settings` (190 files);
> port only what FCast Android sender actually supports — omit the rest entirely.
> Reference: `draft/moblin-ui/Moblin/View/Settings/`

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 3 (settings row components), Phase 4 (Panel routing)
**Unlocks:** Phase 8 (Rust provides `app-version`, `Panel` callbacks), Phase 9 (strings to `@tr`)
**Related files:**
- `senders/android/ui/bridge.slint` — `Panel` enum + routing callbacks added here
- `senders/android/ui/pages/settings_page.slint` — rebuilt here
- `senders/android/ui/main.slint` — `Panel` routing added to `MainWindow`
- `senders/android/src/lib.rs` — `open_panel` / `close_panel` callbacks wired

**Moblin settings omitted by design (not stubs, not TODOs — simply excluded):**
- Chat / Twitch / Kick / YouTube
- Scenes / widgets / overlays
- RTMP / SRT / RIST / WHIP server configs
- GoPro / DJI / Tesla / workout devices
- watchOS / widget targets

---

## Tasks

### 7-A — Add `Panel` enum to `bridge.slint`

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```
  export enum Panel {
      none,
      settings,
      debug,
      codec-test,
  }
  ```

- [ ] Add to `Bridge`:

  ```
      in-out property <Panel>  active-panel: Panel.none;
      in     property <string> app-version:  "";
      callback open-panel(Panel);
      callback close-panel();
  ```

- [ ] **Build check** — `lib.rs` needs `open_panel` / `close_panel` handlers (add stubs now, full
  wiring in Phase 8).

---

### 7-B — Add `Panel` routing to `main.slint`

- [ ] Import `Panel` from `bridge.slint`.
- [ ] Add a panel overlay layer in `MainWindow` that sits above the `AppState` page stack:

  ```
  export component MainWindow inherits Window {
      // AppState page stack (existing):
      if Bridge.app-state == AppState.Disconnected:      ConnectView { }
      if Bridge.app-state == AppState.Connecting:        ConnectingView { }
      if Bridge.app-state == AppState.SelectingSettings: SettingsPageView { }
      if Bridge.app-state == AppState.WaitingForMedia:   WaitingForMediaView { }
      if Bridge.app-state == AppState.Casting:           CastingView { }

      // Panel overlay (on top):
      if Bridge.active-panel == Panel.settings:   FullSettingsPage { }
      if Bridge.active-panel == Panel.debug:      DebugPage { }
      if Bridge.active-panel == Panel.codec-test: CodecTestPage { }   // Phase 7-G

      CastControlBar { y: parent.height - self.height; width: parent.width; }
  }
  ```

- [ ] `FullSettingsPage` is the new component built in 7-D below.
- [ ] **Build check.**

---

### 7-C — Add settings quick action to control bar

- [ ] In Rust `lib.rs`, add a "Settings" quick action to the `quick-actions` model:

  ```rust
  QuickAction { id: "settings", title: "Settings".into(), enabled: true, active: false }
  ```

- [ ] In the `invoke-action` handler, add:

  ```rust
  "settings" => { bridge.set_active_panel(Panel::Settings); }
  ```

- [ ] **Build check.**

---

### 7-D — Build `FullSettingsPage` root

- [ ] Open `senders/android/ui/pages/settings_page.slint`.
- [ ] The file currently contains `SettingsPageView` (the old `SelectingSettingsView`). Keep it
  for now; add `FullSettingsPage` as a second component in the same file.
- [ ] Import `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow` from components.
- [ ] Import `Theme`, `Bridge`, `Panel`.
- [ ] Implement `export component FullSettingsPage`:

  ```
  export component FullSettingsPage inherits Rectangle {
      background: Theme.surface-primary;

      VerticalLayout {
          // Header
          Rectangle {
              height: 56px;
              background: Theme.surface-card;
              HorizontalLayout {
                  padding: Theme.padding-screen;
                  Text {
                      text: "Settings";
                      color: Theme.text-primary;
                      font-size: Theme.font-size-heading;
                      vertical-alignment: center;
                      horizontal-stretch: 1;
                  }
                  TextButton {
                      label: "Done";
                      clicked => Bridge.close-panel();
                  }
              }
          }

          ScrollView {
              VerticalLayout {
                  padding: Theme.padding-screen;
                  spacing: Theme.spacing-default;

                  // Section: Receiver
                  SettingsSection { title: "RECEIVER"; /* rows added 7-E */ }

                  // Section: Video Quality
                  SettingsSection { title: "VIDEO QUALITY"; /* rows added 7-F */ }

                  // Section: Codec / Debug
                  SettingsSection { title: "CODEC & DEBUG"; /* rows added 7-G */ }

                  // Section: About
                  SettingsSection { title: "ABOUT"; /* rows added 7-H */ }
              }
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 7-E — Receiver / Discovery section rows

- [ ] Add inside the "RECEIVER" `SettingsSection`:

  ```
  SettingsValueRow {
      title: "Discovered receivers";
      value: Bridge.devices.length + " found";
      show-chevron: false;
  }
  SettingsToggleRow {
      title: "mDNS discovery";
      checked <=> Bridge.mdns-enabled;    // new Bridge property — add in 7-E step
      toggled(on) => Bridge.set-mdns(on); // new Bridge callback — add in 7-E step
  }
  ```

- [ ] Add to `bridge.slint` `Bridge`:

  ```
      in-out property <bool>   mdns-enabled: true;
      callback set-mdns(bool);
  ```

- [ ] Wire `set-mdns` in `lib.rs` stub (full implementation is outside UI scope).
- [ ] **Build check.**

---

### 7-F — Video Quality section rows

The existing `SettingsPageView` (old `SelectingSettingsView`) already has resolution and
framerate pickers from the SDK. Migrate them into `FullSettingsPage`:

- [ ] Import `VideoResolutionPicker`, `FrameratePicker`, `Utils` from SDK common.
- [ ] Add inside "VIDEO QUALITY" `SettingsSection`:

  ```
  property <int> resolution-idx: 2;
  property <int> framerate-idx:  2;

  SettingsTextRow { title: "Max resolution"; }
  VideoResolutionPicker { current-index <=> resolution-idx; }

  SettingsTextRow { title: "Max framerate"; }
  FrameratePicker { current-index <=> framerate-idx; }
  ```

- [ ] Wire "Start" button in the existing flow to read values from `resolution-idx` and
  `framerate-idx` on `FullSettingsPage`. The simplest approach: keep `SettingsPageView`
  for the pre-cast selection flow; use `FullSettingsPage` only for the in-session settings.
- [ ] **Build check.**

---

### 7-G — Codec / Debug section rows

- [ ] Add inside "CODEC & DEBUG" `SettingsSection`:

  ```
  SettingsValueRow {
      title: "H.264 encoder test";
      value: "Tap to run";
      clicked => Bridge.open-panel(Panel.codec-test);
  }
  SettingsToggleRow {
      title: "Show debug panel";
      checked <=> Bridge.show-debug;
  }
  ```

- [ ] Create a stub `export component CodecTestPage` in a new file
  `senders/android/ui/pages/codec_test_page.slint`:
  - [ ] Header with "Codec Test" title and "Done" close button.
  - [ ] `DebugPage` component embedded below (reuse from Phase 1-G).
  - [ ] This is the page opened by `Bridge.open-panel(Panel.codec-test)`.
- [ ] **Build check.**

---

### 7-H — About section rows

- [ ] Add inside "ABOUT" `SettingsSection`:

  ```
  SettingsValueRow {
      title: "App version";
      value: Bridge.app-version;
      show-chevron: false;
  }
  SettingsValueRow {
      title: "FCast protocol";
      value: "v2";
      show-chevron: false;
  }
  ```

- [ ] `Bridge.app-version` is populated from Rust in Phase 8-F.
- [ ] **Build check.**

---

### 7-I — Transition from `SettingsPageView` to `FullSettingsPage`

Once `FullSettingsPage` is complete:

- [ ] Decide whether `AppState.SelectingSettings` page should be replaced by a `Panel.settings`
  open, or kept as a separate cast-start flow.
  - Recommendation: keep `SettingsPageView` for the pre-cast resolution/framerate selection
    only; open `FullSettingsPage` via the settings quick action for all other settings.
- [ ] If `SettingsPageView` is retained, trim it to only contain resolution + framerate + Start/Disconnect.
- [ ] **Build check.**

---

## Exit criteria

Phase 7 is complete when:

1. `bridge.slint` defines `Panel` enum and `active-panel`, `open-panel`, `close-panel` in `Bridge`.
2. `MainWindow` routes to `FullSettingsPage`, `DebugPage`, and `CodecTestPage` via `Panel`.
3. `FullSettingsPage` shows four sections: Receiver, Video Quality, Codec/Debug, About.
4. "Settings" quick action in `CastControlBar` opens the settings panel.
5. "Done" button closes the panel by setting `Bridge.active-panel = Panel.none`.
6. No unsupported Moblin settings are stubbed — they simply do not appear.
7. `cargo build -p android-sender` passes.

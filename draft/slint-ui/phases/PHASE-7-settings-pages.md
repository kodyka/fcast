# Phase 7 — Settings Navigation and FCast-Specific Pages (UI-only placeholder)

> Build a real settings root page with FCast-relevant sections, replacing the current
> inline `SelectingSettingsView`. **UI placeholder only — no Rust wiring.**
> All values come from inline `in-out` stub properties. Inspired by Moblin
> `View/Settings/` (190 files); port only what FCast Android sender actually
> supports — omit the rest entirely (see `futures/NOT-APPLICABLE.md`).

**Status:** `[ ] UI placeholder — no functionality`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 3 (settings row components), Phase 4 (control bar)
**Functional integration:** Deferred — `open-panel` / `close-panel` / `app-version` / per-row Rust handlers all parked in `futures/`.
**Related files:**
- `senders/android/ui/bridge.slint` — `Panel` enum added here (no setter wiring)
- `senders/android/ui/pages/settings_page.slint` — rebuilt here
- `senders/android/ui/main.slint` — `Panel` overlay layer added to `MainWindow`

**Moblin settings omitted by design** (see `futures/NOT-APPLICABLE.md`):
- Chat / Twitch / Kick / YouTube / Afreeca / Soop
- Scenes / widgets / overlays
- RTMP / SRT / RIST / WHIP / SRTLA server configs
- GoPro / DJI / Tesla / cat printers / workout devices / Wi-Fi cameras
- watchOS / widget targets / Mac key-press relay

---

## Goal

Stand up the settings root page and panel routing chassis. All controls render
from inline `in-out` properties on `FullSettingsPage` so a developer (or
designer) can flip them and see the UI react — but flipping nothing actually
changes app behaviour. The Rust bridge is untouched.

---

## Tasks

### 7-A — Add `Panel` enum to `bridge.slint`

Type definition + a single in-out property for routing. **No callbacks** — the
panel close button writes the property directly from Slint.

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```slint
  export enum Panel {
      none,
      settings,
      debug,
      codec-test,
  }
  ```

- [ ] Add to `Bridge`:

  ```slint
      in-out property <Panel> active-panel: Panel.none;
  ```

- [ ] **Do not** add `open-panel` / `close-panel` callbacks — Slint reads/writes
  `active-panel` directly. Wiring those callbacks is parked in `futures/`.
- [ ] **Build check.**

---

### 7-B — Add `Panel` overlay layer to `main.slint`

- [ ] Import `Panel` from `bridge.slint`.
- [ ] Add the panel overlay above the `AppState` page stack:

  ```slint
  if Bridge.active-panel == Panel.settings:   FullSettingsPage { }
  if Bridge.active-panel == Panel.debug:      DebugPage { }
  if Bridge.active-panel == Panel.codec-test: CodecTestPage { }
  ```

- [ ] **Build check.**

---

### 7-C — Add a "Settings" quick action to the existing control-bar stub

- [ ] In whichever `.slint` file declares the bar's quick-action stub model
  (Phase 4-G's inline `mock-quick-actions`), append:

  ```slint
  { id: "settings", title: "Settings", enabled: true, active: false },
  ```

- [ ] In the bar's `clicked` handler for the action, switch on `id` and write the
  panel directly from Slint (no Rust callback):

  ```slint
  clicked => {
      if (action.id == "settings")    { Bridge.active-panel = Panel.settings; }
      if (action.id == "debug")       { Bridge.active-panel = Panel.debug; }
      if (action.id == "codec-test")  { Bridge.active-panel = Panel.codec-test; }
  }
  ```

- [ ] **Build check.**

---

### 7-D — Build `FullSettingsPage` shell with inline stub state

- [ ] Open `senders/android/ui/pages/settings_page.slint`.
- [ ] Keep existing `SettingsPageView` for now; add `FullSettingsPage` as a second
  exported component.
- [ ] Import `SettingsSection`, `SettingsValueRow`, `SettingsToggleRow`,
  `SettingsTextRow`, `SettingsSliderRow` from `../components/settings_rows.slint`.
- [ ] Import `TextButton` from `../components/buttons.slint`.
- [ ] Import `Theme` from `../theme.slint`, and `Bridge`, `Panel` from `../bridge.slint`.
- [ ] Implement `export component FullSettingsPage`:

  ```slint
  export component FullSettingsPage inherits Rectangle {
      // UI-only stub state — all flips live in this component.
      in-out property <int>    resolution-idx: 2;
      in-out property <int>    framerate-idx:  2;
      in-out property <bool>   mdns-enabled:   true;
      in-out property <bool>   debug-panel:    false;
      in-out property <string> mock-app-version: "0.0.1-dev";

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
                      clicked => { Bridge.active-panel = Panel.none; }
                  }
              }
          }

          // Body
          ScrollView {
              VerticalLayout {
                  spacing: Theme.spacing-section;
                  padding: Theme.padding-screen;

                  // (sections inserted in 7-E … 7-H)
              }
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 7-E — Section: RECEIVER

- [ ] Inside the body `VerticalLayout`:

  ```slint
  SettingsSection {
      title: "RECEIVER";
      SettingsValueRow {
          label: "Discovered receivers";
          value: "3 found";
          // UI-only — clicking would open the connect page.
      }
      SettingsToggleRow {
          label: "mDNS discovery";
          checked: root.mdns-enabled;
          toggled => { root.mdns-enabled = !root.mdns-enabled; }
      }
  }
  ```

---

### 7-F — Section: VIDEO QUALITY

- [ ] Two value-pickers driven by `resolution-idx` / `framerate-idx`:

  ```slint
  SettingsSection {
      title: "VIDEO QUALITY";
      SettingsValueRow {
          label: "Max resolution";
          value: ["480p", "720p", "1080p", "1440p"][root.resolution-idx];
          // UI-only — would open a picker panel.
          clicked => { root.resolution-idx = (root.resolution-idx + 1) mod 4; }
      }
      SettingsValueRow {
          label: "Max framerate";
          value: ["24 fps", "30 fps", "60 fps"][root.framerate-idx];
          clicked => { root.framerate-idx = (root.framerate-idx + 1) mod 3; }
      }
  }
  ```

  Note: cycling on click is a placeholder UX. Real picker panels land in `futures/`.

---

### 7-G — Section: CODEC & DEBUG

- [ ] Add the codec test launcher and debug toggle:

  ```slint
  SettingsSection {
      title: "CODEC & DEBUG";
      SettingsValueRow {
          label: "H.264 encoder test";
          value: "Open";
          clicked => { Bridge.active-panel = Panel.codec-test; }
      }
      SettingsToggleRow {
          label: "Show debug panel";
          checked: root.debug-panel;
          toggled => { root.debug-panel = !root.debug-panel; }
      }
  }
  ```

---

### 7-H — Section: ABOUT

- [ ] Last section with version + protocol info:

  ```slint
  SettingsSection {
      title: "ABOUT";
      SettingsTextRow {
          label: "App version";
          value: root.mock-app-version;
      }
      SettingsTextRow {
          label: "FCast protocol";
          value: "v3";
      }
  }
  ```

---

### 7-I — Standalone `CodecTestPage` placeholder

- [ ] Create `senders/android/ui/pages/codec_test_page.slint`.
- [ ] Build a minimal panel: header (title + Done button writing
  `Bridge.active-panel = Panel.none`), a `PrimaryButton` labelled "Run encoder test",
  and a multiline `Text` showing a hard-coded mock log.
- [ ] Register in `main.slint` (already done in 7-B).
- [ ] **Build check.**

---

## Exit criteria

1. `bridge.slint` exposes `Panel` enum + `active-panel` (no callbacks).
2. `main.slint` shows `FullSettingsPage` / `DebugPage` / `CodecTestPage` based on `active-panel`.
3. The control bar's "Settings" stub action sets `Bridge.active-panel = Panel.settings`.
4. `FullSettingsPage` renders the four sections (RECEIVER / VIDEO QUALITY / CODEC & DEBUG / ABOUT)
   with inline stub state.
5. Toggle rows flip their stub property when tapped (no Rust round-trip).
6. Done button closes the panel from Slint.
7. `cargo build -p android-sender` passes.

---

## What's NOT in this phase (deferred)

- Persisted settings (writing `mdns-enabled` / `resolution-idx` etc. to disk).
- Real receiver count populated from Rust discovery.
- Actual app-version string from Cargo / Android metadata.
- Real H.264 encoder test launcher.
- Picker panels for resolution/framerate (cycle-on-click is a placeholder).
- `@tr(...)` wrapping (Phase 9).
- Per-section sub-pages (audio/camera/bitrate/etc.) — those each get their own
  UI-only phase later (Phases 13–27).

---

## Slint best practices applied here

- **`in-out` properties on the page are ideal for placeholder state.** They let
  the UI react to taps locally without round-tripping through Rust.
- **Reading/writing `Bridge.active-panel` directly from Slint** avoids needing
  callbacks. This is the simplest possible routing chassis — no Rust changes.
- **Indexed string lookup `["a", "b", "c"][idx]`** is the idiomatic Slint pattern
  for cycling through a small enum-like value set without defining a separate
  enum or model.

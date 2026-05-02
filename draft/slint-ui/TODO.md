# TODO: FCast Android Sender — Slint UI Evolution

**Goal:** Evolve `senders/android/ui/main.slint` from a functional-but-flat single-file UI into a
modular, Moblin-inspired component system, using `draft/moblin-ui/` as reference design material only.
This is a **Slint reimplementation**, not a SwiftUI copy.

**Related trackers:**

- `senders/android/TODO.md` — protocol/graph migration (Phases 0–6, mostly complete)
- `senders/android/TODO.codecs.md` — GStreamer Android codec blockers (P0–P3)
- `draft/slint-ui/docs/swiftui-to-slint-guide.md` — concept mapping reference
- `draft/slint-ui/analysis/summary.md` — Moblin SwiftUI file inventory (279 files)

---

## Phase 0 — Baseline audit (do this first, before any UI changes)

> Confirm the ground truth so every later task has a stable reference point.

- [ ] **Confirm Slint version** in `Cargo.lock` — check `slint` and `slint-build` entries before
      using any features listed in migration skeleton (e.g. `@tr`, `ListView`, `ScrollView`).
- [ ] **Confirm current `main.slint` compiles** end-to-end with `cargo build -p android-sender`
      (or equivalent Nix/CI path).
- [ ] **Confirm existing Bridge contract** — document every `in`, `in-out`, and `callback` property
      currently in `export global Bridge` in `main.slint` so nothing is silently dropped during split.

  Current Bridge surface (as of last read):
  | Property / Callback | Direction | Type | Notes |
  |---|---|---|---|
  | `devices` | `in` | `[string]` | mDNS discovered receivers |
  | `app-state` | `in-out` | `AppState` | Disconnected → Casting enum |
  | `show-debug` | `in-out` | `bool` | Toggles debug panel |
  | `test-status` | `in` | `string` | Debug log text |
  | `connect-receiver` | callback | `(string)` | User taps a device |
  | `start-casting` | callback | `(int,int,int)` | width, height, framerate |
  | `stop-casting` | callback | `()` | Stop / cancel |
  | `scan-qr` | callback | `()` | Open QR scanner |
  | `start-migrated-server` | callback | `()` | Debug |
  | `test-legacy-getinfo` | callback | `()` | Debug |
  | `test-legacy-crossfade` | callback | `()` | Debug |
  | `test-smoke-graph` | callback | `()` | Debug |

- [ ] **Map current views** — note which Slint components exist today and which states they cover:
  - `ConnectView` → `AppState.Disconnected`
  - `ConnectingView` → `AppState.Connecting`
  - `SelectingSettingsView` → `AppState.SelectingSettings`
  - `WaitingForMediaView` → `AppState.WaitingForMedia`
  - `CastingView` → `AppState.Casting`
- [ ] **Decide migration branch strategy** — create a separate git branch before Phase 1 destructive
      splits, so `main` stays stable.
- [ ] **Check codec blockers** in `TODO.codecs.md` — at least P0 items (issues 1–3) should be resolved
      or have a fallback before investing heavily in casting UI polish.

---

## Phase 1 — Split `main.slint` into modules

> Current single-file `senders/android/ui/main.slint` (~230 lines) will become a directory of modules.
> Every split step must keep `cargo build` green.

Proposed target layout:

```
senders/android/ui/
├── main.slint               ← thin root: imports + MainWindow only
├── theme.slint              ← colors, font sizes, radii, spacing constants
├── bridge.slint             ← export global Bridge (single source of truth)
├── components/
│   ├── buttons.slint        ← PrimaryButton, TextButton, CreateActionButton
│   ├── settings_rows.slint  ← SettingsTextRow, SettingsValueRow, SettingsSliderRow
│   ├── control_bar.slint    ← CastControlBar, QuickActionButton
│   └── status_overlay.slint ← StatusOverlay, StatusItem rendering
└── pages/
    ├── connect_page.slint   ← ConnectView (current ConnectView)
    ├── connecting_page.slint← ConnectingView
    ├── settings_page.slint  ← SelectingSettingsView → renamed/extended
    ├── casting_page.slint   ← WaitingForMediaView + CastingView merged
    └── debug_page.slint     ← current inline debug block extracted
```

- [ ] Create `ui/theme.slint`:
  - [ ] Export color palette constants matching current hardcoded values (`#222633`, `#B8BECD`, `steelblue`, `lightsteelblue`).
  - [ ] Export font-size tokens (`font-size-body`, `font-size-label`, `font-size-heading`).
  - [ ] Export spacing/radius tokens (`border-radius-card`, `padding-screen`).
- [ ] Create `ui/bridge.slint`:
  - [ ] Move `export global Bridge` out of `main.slint`.
  - [ ] Add new properties from Phase 2+ gradually (do not break existing Rust handle calls).
- [ ] Create `ui/pages/connect_page.slint`:
  - [ ] Extract current `ConnectView` verbatim.
  - [ ] Import `Bridge` from `bridge.slint`.
- [ ] Create `ui/pages/connecting_page.slint`:
  - [ ] Extract `ConnectingView`.
- [ ] Create `ui/pages/settings_page.slint`:
  - [ ] Extract `SelectingSettingsView`.
  - [ ] Rename to `SettingsPageView` to avoid collision with future settings navigation.
- [ ] Create `ui/pages/casting_page.slint`:
  - [ ] Extract `WaitingForMediaView` and `CastingView`.
- [ ] Create `ui/pages/debug_page.slint`:
  - [ ] Extract the `if Bridge.show-debug: VerticalBox { ... }` block from `ConnectView`.
  - [ ] Make it a standalone `DebugPage` component.
- [ ] Update `main.slint` to import all modules and keep only `MainWindow`.
- [ ] Verify `build.rs` `slint_build::compile` path still resolves to `main.slint`.
- [ ] Confirm `lib.rs` `slint::include_modules!()` bridge handles still compile after split.

---

## Phase 2 — Theme and design tokens

> Replace all hardcoded hex values and magic numbers with named tokens from `theme.slint`.

- [ ] Audit every color literal in the new module files.
- [ ] Define semantic tokens:
  - [ ] `Theme.surface-primary` (`#0b1020`)
  - [ ] `Theme.surface-card` (`#222633`)
  - [ ] `Theme.text-primary` (white)
  - [ ] `Theme.text-secondary` (`#B8BECD`)
  - [ ] `Theme.accent` (`steelblue` → replace with a proper hex)
  - [ ] `Theme.accent-pressed` (pressed state darken)
  - [ ] `Theme.error` (`#7f1d1d`)
  - [ ] `Theme.warning` (`#78350f`)
- [ ] Replace `if device_ta.pressed ? steelblue : lightsteelblue` pattern with theme token in all touch areas.
- [ ] Set `background` on `MainWindow` to `Theme.surface-primary`.

---

## Phase 3 — Reusable button and row components

> Inspired by Moblin `View/Utils` patterns; implement as pure Slint with no SwiftUI dependency.
> Reference: `draft/moblin-ui/Moblin/View/Utils/`.

- [ ] Create `ui/components/buttons.slint`:
  - [ ] `PrimaryButton` — full-width accent button (replaces raw `Button` in pages).
  - [ ] `TextButton` — borderless text-only button.
  - [ ] `DestructiveButton` — red-tinted stop/cancel button.
  - [ ] `IconTextButton` — icon + label horizontal pair (for control bar actions).
  - [ ] All buttons use `Theme` tokens for colors and radii.
  - [ ] All buttons have `enabled` property with `opacity: enabled ? 1 : 0.45`.
- [ ] Create `ui/components/settings_rows.slint`:
  - [ ] `SettingsTextRow` — label-only row (section headers, info rows).
  - [ ] `SettingsValueRow` — label + trailing value text + optional chevron.
  - [ ] `SettingsToggleRow` — label + `CheckBox` on trailing edge.
  - [ ] `SettingsSliderRow` — label + `Slider` + value readout.
  - [ ] `SettingsSection` — titled group container (card-style background).
  - [ ] All rows `height: 44px` for touch target compliance.
- [ ] Replace `Button` usages in `connect_page.slint` with `PrimaryButton`.
- [ ] Replace `Button` usages in `settings_page.slint` with `PrimaryButton`.
- [ ] Replace spinner+text patterns with a shared `LoadingView` component.

---

## Phase 4 — Control bar and quick actions

> Inspired by Moblin `View/ControlBar`. Reference: `draft/moblin-ui/Moblin/View/ControlBar/`.

- [ ] Add `QuickAction` struct to `bridge.slint`:
  ```
  export struct QuickAction { id: string, title: string, enabled: bool, active: bool }
  ```
- [ ] Add `quick-actions: [QuickAction]` and `invoke-action(string)` callback to `Bridge`.
- [ ] Create `ui/components/control_bar.slint`:
  - [ ] `QuickActionButton` component (badge-style, active/inactive states).
  - [ ] `CastControlBar` — horizontal scrollable action strip, `height: 72px`, pinned to bottom.
- [ ] Create `ui/pages/casting_page.slint` `CastButton` state machine:
  - [ ] `disconnected` → "Connect / Scan" action.
  - [ ] `connecting` → "Cancel" with spinner.
  - [ ] `waiting-for-media` → "Cancel" with spinner.
  - [ ] `casting` → "Stop Casting" destructive button.
- [ ] Wire quick actions for existing debug buttons:
  - [ ] `{ id: "scan-qr", title: "Scan QR" }`
  - [ ] `{ id: "migrated-server", title: "Start Server" }` — only when `show-debug: true`.
  - [ ] `{ id: "test-getinfo", title: "GetInfo" }` — debug only.
  - [ ] `{ id: "test-crossfade", title: "Crossfade" }` — debug only.
  - [ ] `{ id: "test-smoke", title: "Smoke Graph" }` — debug only.
- [ ] Update Rust `lib.rs` to populate `Bridge.quick-actions` slice from Rust state.

---

## Phase 5 — Status overlay

> Inspired by Moblin `View/Stream/StreamOverlayView`. Reference: `draft/moblin-ui/Moblin/View/Stream/`.

- [ ] Add `StatusItem` struct to `bridge.slint`:
  ```
  export struct StatusItem { label: string, value: string, severity: string }
  ```
- [ ] Add `status-items: [StatusItem]` to `Bridge`.
- [ ] Create `ui/components/status_overlay.slint`:
  - [ ] `StatusOverlay` — transparent overlay, positioned top-left of casting page.
  - [ ] Renders `for item in Bridge.status-items` with severity-colored pill per item.
  - [ ] Severity values: `"info"` (neutral), `"warning"` (amber), `"error"` (red).
- [ ] Add overlay to `casting_page.slint` using `ZStack` pattern.
- [ ] Populate from Rust with: receiver name, bitrate (if available), encoder name, network address.
- [ ] Keep overlay hidden when `status-items` is empty.
- [ ] Add debug log `ScrollView` to `debug_page.slint` (currently inline `ScrollView` in `ConnectView`).

---

## Phase 6 — Receiver list and discovery UX

> Improve the current flat device list in `connect_page.slint`.

- [ ] Add `ReceiverItem` struct to `bridge.slint`:
  ```
  export struct ReceiverItem { name: string, address: string }
  ```
- [ ] Replace `[string]` devices with `[ReceiverItem]` in `Bridge`.
- [ ] Update `connect_page.slint` `ListView` to show name + address sub-text.
- [ ] Show animated "Searching…" placeholder with `Spinner` when list is empty.
- [ ] Add swipe-to-dismiss or explicit "Forget" action on known receivers (Phase 6b — defer).
- [ ] Update Rust `lib.rs` `Bridge.devices` mapping to produce `ReceiverItem` values.

---

## Phase 7 — Settings navigation and FCast-specific pages

> Inspired by Moblin `View/Settings`. Port only settings that FCast Android sender actually supports.
> Reference: `draft/moblin-ui/Moblin/View/Settings/`.

- [ ] Add `Panel` enum to `bridge.slint`:
  ```
  export enum Panel { none, settings, debug, codec-test }
  ```
- [ ] Add `active-panel: Panel` + `open-panel(Panel)` / `close-panel()` to `Bridge`.
- [ ] Create `ui/pages/settings_page.slint` settings navigation root:
  - [ ] `SettingsSection` for **Receiver / Discovery**:
    - [ ] Manual IP entry (`LineEdit` + connect button).
    - [ ] mDNS toggle.
  - [ ] `SettingsSection` for **Video Quality**:
    - [ ] Resolution picker (reuse existing `VideoResolutionPicker` from SDK).
    - [ ] Framerate picker (reuse existing `FrameratePicker` from SDK).
    - [ ] Bitrate cap input (future — once Rust side supports it).
  - [ ] `SettingsSection` for **Codec / Debug**:
    - [ ] H.264 MediaCodec test row → opens `debug_page.slint`.
    - [ ] Encoder name readout.
  - [ ] `SettingsSection` for **About**:
    - [ ] App version string (from Bridge property).
    - [ ] Protocol version.
- [ ] Wire settings panel open/close through `Panel` enum routing in `MainWindow`.
- [ ] Do **not** stub unsupported Moblin settings (chat, RTMP/SRT/RIST, scenes, GoPro, watch) — omit entirely.

---

## Phase 8 — Rust bridge hardening

> Ensure `lib.rs` and the new Slint modules stay in sync as the Bridge grows.

- [ ] Add `ReceiverItem` Slint model binding in `lib.rs` (replacing raw `String` vec).
- [ ] Add `QuickAction` model population from Rust (conditional on `show_debug` flag).
- [ ] Add `StatusItem` model population from Rust cast state.
- [ ] Add `Panel` routing callbacks (`open_panel`, `close_panel`) in Rust.
- [ ] Keep all Slint handle callbacks using `Weak<MainWindow>` pattern to avoid ownership cycles.
- [ ] Add `Bridge.app-version` string property populated at startup from `env!("CARGO_PKG_VERSION")`.
- [ ] Confirm no binding loops introduced by new two-way (`in-out`) properties (check Slint compiler warnings).

---

## Phase 9 — Localization preparation

> FCast Android sender currently has zero i18n. Lay the groundwork without full translation work.

- [ ] Replace all user-visible string literals in `.slint` files with `@tr("...")`.
- [ ] Verify `slint-tr-extractor` is available in the workspace tooling path.
- [ ] Run `slint-tr-extractor` to generate a `.pot` template — commit as `ui/i18n/messages.pot`.
- [ ] Do **not** copy Moblin `Common/Localizable.xcstrings` — it is iOS/macOS format only.
- [ ] Decide whether English-only shipping is acceptable for initial release (likely yes).

---

## Phase 10 — Testing and Android validation

> Each functional slice must be verified on device, not just desktop Slint preview.

- [ ] Run `cargo build -p android-sender` after each phase merge.
- [ ] Run existing unit tests in `senders/android/src/migration/` — confirm no regressions.
- [ ] Test on a physical Android device for:
  - [ ] Touch target sizes (minimum 44dp per row/button).
  - [ ] Portrait and landscape layout (control bar must pin to correct edge).
  - [ ] `ListView` scroll performance with 10+ receivers.
  - [ ] Spinner animation during `Connecting` / `WaitingForMedia` states.
- [ ] Use Slint's desktop preview (`slint-viewer`) during development to iterate quickly before deploying to device.
- [ ] Verify no `ERROR: missing layout size` or binding loop warnings in Slint compiler output.
- [ ] Address P0 codec blockers (`TODO.codecs.md` items 1–3) before testing casting flow end-to-end.

---

## Phase 11 — Source tracking by Moblin group

> Cross-reference to `draft/slint-ui/analysis/summary.md` group counts for completeness review.

| Moblin source group             | FCast equivalent target                                            | Priority | Status  |
| ------------------------------- | ------------------------------------------------------------------ | -------- | ------- |
| `View/Utils` (36 files)         | `ui/components/buttons.slint`, `ui/components/settings_rows.slint` | High     | Phase 3 |
| `View/ControlBar` (23 files)    | `ui/components/control_bar.slint`, `CastButton`                    | High     | Phase 4 |
| `View/Stream` (23 files)        | `ui/components/status_overlay.slint`, casting page                 | Medium   | Phase 5 |
| `View/Settings` (190 files)     | `ui/pages/settings_page.slint` — FCast subset only                 | Medium   | Phase 7 |
| `View/MainView.swift` (1 file)  | `main.slint` `MainWindow` routing                                  | Low      | Phase 1 |
| `View/Main` (4 files)           | Lock/stealth/snapshot — defer, not in FCast scope                  | Defer    | —       |
| `View/WebBrowser` (1 file)      | Defer — no FCast browser target                                    | Defer    | —       |
| `View/ExternalDisplay` (1 file) | Defer — no Android multi-display target                            | Defer    | —       |

---

## Codec blocker dependency

> UI phases 5, 6, and 10 depend on a working casting path. Confirm these `TODO.codecs.md` items
> before investing in casting-page polish:

| Codec item                                     | Blocks UI work                               |
| ---------------------------------------------- | -------------------------------------------- |
| **P0-1** H.264 encoder selection (`amcvidenc`) | Casting page, status overlay encoder readout |
| **P0-2** `rtmp2sink` / `rtmpsink` fallback     | RTMP destination row in settings             |
| **P0-3** `fallbacksrc` hardening               | Source node stability for WaitingForMedia UX |
| **P1-5** Startup element validation            | Health status item in status overlay         |

---

## Decisions still open

- [ ] Should `draft/slint-ui/` become the final home for module files, or should all `.slint` files
      land directly under `senders/android/ui/`? (Recommendation: `senders/android/ui/` — keep draft
      as design reference only.)
- [ ] Should `migration-skeleton.slint` (`draft/slint-ui/ui/`) be promoted to a living prototype
      in `senders/android/ui/` or left as a design artifact? (Recommendation: promote once Phase 1
      split is complete.)
- [ ] Should the `Panel` enum routing replace the current `if Bridge.app-state ==` chain, or live
      alongside it as an overlay layer? (Recommendation: sidebar/overlay `Panel` layer on top of the
      existing `AppState` page stack — cleaner separation.)

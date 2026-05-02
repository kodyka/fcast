# Phase 10 — Testing and Android Validation

> Verify every UI change works on a real Android device, not just in desktop preview.
> Run after each phase merge — this is a recurring checklist, not a one-time task.

**Status:** `[ ] Ongoing — run after each phase`
**Blocked by (for casting flow tests):** `TODO.codecs.md` P0-1, P0-2, P0-3
**Related files:**
- `senders/android/src/migration/` — unit tests that must not regress
- `senders/android/TODO.codecs.md` — P0 blockers for end-to-end casting test

---

## 10-A — Build gate (run after every phase)

- [ ] `cargo build -p android-sender` — zero errors.
- [ ] `cargo test  -p android-sender` — all tests pass.
- [ ] Slint compiler output: zero new warnings compared to Phase 0 baseline.
  - Especially watch for: `ERROR: missing layout size` and `Binding loop detected`.
- [ ] For Android cross-compile (if CI available): build `aarch64-linux-android` target.

---

## 10-B — Desktop Slint preview check (fast iteration)

Use `slint-viewer` or `cargo run` with a desktop backend for rapid visual checks before
deploying to device:

- [ ] Install or confirm `slint-viewer` is available.
- [ ] After Phase 1 split: open `main.slint` in viewer and confirm all five `AppState` views render.
- [ ] After Phase 2 theme: confirm colors match expected dark palette.
- [ ] After Phase 3 components: confirm button styles and row types render correctly.
- [ ] After Phase 4 control bar: confirm bar is pinned to bottom and quick action buttons appear.
- [ ] After Phase 5 overlay: confirm pills appear/disappear with mock `status-items` data.
- [ ] After Phase 7 settings: confirm `FullSettingsPage` sections render with correct row types.

---

## 10-C — Touch target size validation (on device)

Android minimum recommended touch target: **48dp** (Android material guidelines).

- [ ] Connect a physical Android device (API 26+) or start emulator.
- [ ] Deploy the APK.
- [ ] Tap every interactive element and confirm it is comfortably tappable:
  - [ ] Receiver list rows (`Theme.row-height` — currently `44px`; may need bump to `48px`).
  - [ ] Scan QR button.
  - [ ] Quick action buttons in `CastControlBar` (`height: 48px` — should be fine).
  - [ ] Settings rows in `FullSettingsPage`.
  - [ ] Toggle switches in settings.
  - [ ] "Stop Casting" / "Cancel" / "Start" buttons.
  - [ ] "Done" close button in settings header.
- [ ] If rows feel too small, update `Theme.row-height` to `48px` in `theme.slint`.
  This change propagates everywhere automatically.

---

## 10-D — Portrait layout validation

- [ ] Launch app in portrait orientation.
- [ ] Confirm `CastControlBar` is pinned to bottom and does not overlap content.
- [ ] Confirm `ConnectView` device list scrolls when there are many receivers (use mock data).
- [ ] Confirm `FullSettingsPage` `ScrollView` scrolls all sections.
- [ ] Confirm `StatusOverlay` in casting page does not block "Stop Casting" button.
- [ ] Confirm `DebugPage` `ScrollView` for test status log scrolls correctly.

---

## 10-E — Landscape layout validation

- [ ] Rotate device to landscape.
- [ ] Confirm `CastControlBar` still pins to screen bottom (not side).
  > Note: Moblin has a separate landscape bar layout. For FCast, bottom-pinned is
  > acceptable unless the app is locked to portrait.
- [ ] Confirm no layout overflow or clipped text in landscape.
- [ ] If landscape is broken, add a `preferred-height` and `preferred-width` guard on
  `MainWindow` or lock screen orientation in `AndroidManifest.xml`.

---

## 10-F — `ListView` scroll performance

- [ ] Populate `Bridge.devices` with 20+ mock `ReceiverItem` entries.
- [ ] Scroll the list rapidly — confirm no janky frame drops.
- [ ] Confirm `ListView` handles 50+ items without significant slowdown (Slint `ListView`
  does not virtualize by default — monitor this limit).
- [ ] If performance degrades: investigate `ListView` virtualization options for the Slint
  version in use, or add a result cap in the Rust discovery handler.

---

## 10-G — Spinner animation validation

- [ ] Manually set `AppState.Connecting` from debug UI.
- [ ] Confirm `Spinner` animates smoothly (indeterminate mode).
- [ ] Repeat for `AppState.WaitingForMedia`.
- [ ] Confirm spinner stops / disappears when state transitions to `Casting` or `Disconnected`.

---

## 10-H — Panel routing validation

- [ ] From `CastControlBar`, tap "Settings" quick action.
- [ ] Confirm `FullSettingsPage` slides / appears over the current page.
- [ ] Tap "Done" — confirm settings page disappears and underlying page is restored.
- [ ] Open "Codec Test" from the settings codec row.
- [ ] Confirm `CodecTestPage` appears with `DebugPage` embedded.
- [ ] Tap "Done" on codec test page — confirm routing returns to settings.
- [ ] Test back-button behavior: Android system back should close panels.
  - If Slint handles back button, wire it to `Bridge.close-panel()`.

---

## 10-I — Casting flow end-to-end (requires codec P0 resolution)

> **Blocked:** This test requires `TODO.codecs.md` P0-1 (H.264 encoder) and P0-3
> (`fallbacksrc` / `uridecodebin`) to be resolved first.

- [ ] Confirm `TODO.codecs.md` P0-1 is resolved.
- [ ] Connect to a real FCast receiver on the same network.
- [ ] Start casting — confirm `AppState` transitions through:
  `Disconnected → Connecting → SelectingSettings → WaitingForMedia → Casting`.
- [ ] Confirm `StatusOverlay` shows "Receiver" and "Encoder" pills in `Casting` state.
- [ ] Stop casting — confirm `AppState` returns to `Disconnected`.
- [ ] Confirm `StatusOverlay` is hidden after casting stops.

---

## 10-J — Slint compiler warning audit

- [ ] After all phases are merged, run a full build and capture compiler output.
- [ ] Resolve any warnings of these types:
  - `Binding loop detected` — rework the binding chain.
  - `ERROR: missing layout size` — add explicit `height` or `width` to the affected component.
  - `Property X is never read` — remove dead properties from Bridge.
  - `Unused import` — clean up unused `import` statements.

---

## 10-K — Memory and lifecycle check

- [ ] Launch app, start casting, stop casting, restart casting — repeat 5 times.
- [ ] Confirm app does not crash or grow in memory (use Android Studio profiler or `adb logcat`).
- [ ] Confirm `VecModel` instances in `ui_state.rs` are not accumulating stale entries
  after repeated cast sessions.

---

## Recurring test matrix

Run this matrix at the end of each phase:

| Test | Phase 1 | Phase 2 | Phase 3 | Phase 4 | Phase 5 | Phase 6 | Phase 7 | Phase 8 |
|---|---|---|---|---|---|---|---|---|
| `cargo build` passes | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |
| `cargo test` passes | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |
| Desktop preview renders | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |
| On-device portrait layout | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |
| No new Slint warnings | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] | [ ] |

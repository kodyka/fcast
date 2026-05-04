# Phase 10 — UI Validation (build + visual checks only)

> Verify every UI placeholder builds, renders, and looks right on a real
> Android device. **Build-only validation** — no end-to-end functional testing,
> because Phases 5–7 and 12–27 ship UI without functionality.

**Status:** `[ ] Ongoing — run after each UI phase merge`
**Functional integration:** None. Functional/casting validation is parked in
`futures/` until the Rust wiring placeholder (Phase 8) is reactivated.
**Related files:**
- `senders/android/src/migration/` — unit tests that must not regress
- `senders/android/TODO.codecs.md` — P0 blockers tracked separately, not in scope here

---

## 10-A — Build gate (run after every phase)

- [ ] `cargo build -p android-sender` — zero errors.
- [ ] `cargo test  -p android-sender` — all tests pass.
- [ ] Slint compiler output: zero new warnings compared to Phase 0 baseline.
  - Especially watch for: `ERROR: missing layout size` and `Binding loop detected`.
- [ ] For Android cross-compile (when CI is available): build `aarch64-linux-android` target.

---

## 10-B — Desktop Slint preview check (fast iteration)

`slint-viewer` lets you preview a `.slint` file with all its inline stub data
without compiling Rust. This is the fastest visual loop for UI-only phases.

- [ ] Install/confirm `slint-viewer` is available (matching the futo Slint fork version).
- [ ] After any UI phase, open the touched page in the viewer:

  ```sh
  slint-viewer senders/android/ui/pages/connect_page.slint
  ```

- [ ] Confirm stub data renders as expected. Try the "alt mocks" if a phase
  ships variants (e.g. Phase 5-E `mock-status-items-error`, Phase 6-D `mock-empty`).

---

## 10-C — Touch target size validation (on device)

Android minimum recommended touch target: **48dp** (Material guidelines).

- [ ] Connect a physical Android device (API 26+) or start emulator.
- [ ] Deploy the APK.
- [ ] Tap every interactive element and confirm it is comfortably tappable:
  - [ ] Receiver list rows (Phase 6).
  - [ ] Quick action buttons in `CastControlBar` (Phase 4).
  - [ ] Settings rows in `FullSettingsPage` (Phase 7).
  - [ ] Toggle switches in settings.
  - [ ] "Done" close button in panel headers.
  - [ ] Any new placeholder rows from Phases 12–27 that have shipped.
- [ ] If rows feel too small, update `Theme.row-height` to `48px` in `theme.slint`.

---

## 10-D — Portrait layout validation

- [ ] Launch app in portrait orientation.
- [ ] Confirm `CastControlBar` is pinned to bottom and does not overlap content.
- [ ] Confirm `ConnectView` device list scrolls when stub `mock-devices` has 20+ entries.
- [ ] Confirm `FullSettingsPage` `ScrollView` scrolls all sections.
- [ ] Confirm `StatusOverlay` in casting page does not block the Stop Casting button.

---

## 10-E — Landscape layout validation

- [ ] Rotate device to landscape.
- [ ] Confirm `CastControlBar` still pins to screen bottom.
- [ ] Confirm no layout overflow or clipped text in landscape.
- [ ] If landscape is broken, lock screen orientation in `AndroidManifest.xml` for now.

---

## 10-F — `ListView` scroll performance (stub-driven)

- [ ] Temporarily expand `mock-devices` (Phase 6) to 50+ entries.
- [ ] Scroll the list rapidly — confirm no janky frame drops.
- [ ] Slint's `ListView` virtualizes by default — only visible rows are
  instantiated. Reference:
  [`reference/std-widgets/views/listview.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx)
  — _"Elements are only instantiated if they are visible"_.
- [ ] If performance degrades, the bottleneck is per-row cost (image decoding,
  expensive bindings, deep nesting). Profile and simplify the row component.
- [ ] **Do not wrap `ListView` inside another `ScrollView`.** That disables
  virtualization.

---

## 10-G — Spinner / animation validation

- [ ] Set `mock-empty: true` in `ConnectView` and confirm spinner animates.
- [ ] Manually flip `Bridge.app-state = AppState.Connecting` from a debug build
  and confirm `ConnectingView` spinner animates smoothly.

---

## 10-H — Panel routing validation (UI-only)

- [ ] From `CastControlBar`, tap the "Settings" stub action.
- [ ] Confirm `FullSettingsPage` appears (it's `Bridge.active-panel = Panel.settings`).
- [ ] Tap "Done" — confirm the panel closes (`Bridge.active-panel = Panel.none`).
- [ ] Repeat for "Codec Test" panel from inside `FullSettingsPage`.

---

## 10-I — Per-phase visual regression checklist

For each shipped UI placeholder phase, capture a reference screenshot and
attach it to the PR. Cross-check on subsequent merges that nothing visually
regresses.

- [ ] Phase 5: `CastingView` with overlay (info pills) + alt mock with error pill.
- [ ] Phase 6: `ConnectView` populated + empty state.
- [ ] Phase 7: `FullSettingsPage` (all four sections).
- [ ] Each phase from 12–27: at least one screenshot of the new placeholder.

---

## 10-J — Functional smoke (deferred)

End-to-end functional smoke (start cast, transmit media, stop cast, settings
persist, etc.) is **not in scope** for the UI-only roadmap. Once Phase 8 is
reactivated, this section will be rewritten with real device/casting checks.

---

## Exit criteria (per UI phase)

A UI phase is "validated" when:

1. `cargo build -p android-sender` is clean.
2. The phase's stub UI renders correctly in `slint-viewer`.
3. The phase's stub UI renders correctly on a physical Android device in both
   portrait and landscape.
4. A reference screenshot is attached to the PR.
5. No unrelated UI surfaces visually regressed.

---

## What's NOT in this phase

- Functional / casting / discovery / settings persistence tests — all deferred
  with Phase 8.
- Automated UI tests (Espresso, screenshot-diff) — out of scope until UI is sealed.

---

## Slint best practices applied here

- **Preview first, deploy second.** `slint-viewer` against a single `.slint`
  file is orders of magnitude faster than rebuilding the APK for every UI tweak.
- **Stub data scaling.** Temporarily expanding a `mock-*` array to 50+ entries
  to stress-test the layout (then reverting before commit) is the standard
  Slint workflow for catching layout/scroll bugs early.

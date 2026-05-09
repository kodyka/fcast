# Phase 10 — UI Validation reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-10-testing.md`][spec] to the current `senders/android` tree.
**Goal:** establish a repeatable validation routine that runs after every UI phase merges. Build gate → desktop preview check → on-device touch / portrait / landscape / scroll-perf checks → per-phase visual regression checklist. **Build-only validation** — no end-to-end functional testing while Phase 8 is deferred.
**Scope:** mostly tooling and procedure. **No code changes** — this guide is a checklist + a CI-friendly script that audits the merged UI.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-10-testing.md

> **Run this guide after every UI phase merges.** Like Phase 9, Phase 10 is **ongoing**. Each merge triggers a fresh sweep through the relevant sections — not a re-execution of every section. Use the "trigger" column in Section 3 to scope the validation to what the phase touched.

---

## Why this guide exists

Phase 10 is small in scope but high in cadence — it runs after every UI phase merge. The risk of skipping it: visual regressions accumulate silently because the UI surface has no automated test coverage. This guide:

1. **Consolidates the four validation modes** (build / desktop preview / on-device touch / on-device layout) into one runnable routine.
2. **Maps each shipped UI phase to its specific visual-regression risk** so a per-merge sweep targets the right surfaces.
3. **Documents the four most common Slint validation failures** (binding loops, missing layout sizes, ListView wrapped in ScrollView, touch targets below 48dp) and how to spot them.
4. **Establishes the screenshot-attachment-to-PR convention** as the visual diff baseline (until automated screenshot-diff lands post-Phase 8).

Phase 10 has **no implementation steps** in the conventional sense — there's no Slint code to write. Everything below is procedure + audit greps + checklists.

---

## Section 0 — Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only

# Validate the toolchain.
cargo --version
cargo build -p android-sender    # baseline; should be clean

# Install slint-viewer (matching the futo Slint fork version).
# The version must align with senders/android/Cargo.lock's slint entry.
cargo install slint-viewer --locked

# For on-device validation: connect a physical Android device or start an emulator.
adb devices                       # expect at least one device listed.
```

If `slint-viewer --version` doesn't match `slint` from `Cargo.lock`, version-mismatched stubs may render incorrectly. Either pin via `cargo install slint-viewer --version <X>` or accept that visual fidelity may diverge from the on-device build.

---

## Section 1 — Build gate (every UI phase)

### 1.1 Cargo build + tests

```sh
cargo build -p android-sender 2>&1 | tee /tmp/build.log

# Look for new warnings vs. baseline.
diff <(grep -E '^warning:' /tmp/build.log) <(git show master:.tmp/build-baseline.log 2>/dev/null) || true

# Expected: zero new warnings. Some pre-existing baseline noise is OK as
# long as the count doesn't grow. If it does, the new phase introduced a
# warning — fix before merge.
```

```sh
cargo test -p android-sender
# Expected: all tests pass. The migration/ unit tests must not regress.
```

### 1.2 Slint compiler watch list

The Slint compiler emits warnings inline during `cargo build`. Two specific warnings catch UI-only phase mistakes:

```sh
grep -E "ERROR: missing layout size|Binding loop detected" /tmp/build.log
# Expected: 0 matches.
```

| Warning | Cause | Fix |
|---|---|---|
| `ERROR: missing layout size` | A child element of a `*Layout` has neither implicit size nor explicit `width`/`height`. | Set `width:` / `height:` explicitly, or wrap in a `Rectangle` with size. Common around `if cond: Component { ... }` branches that don't size their content. |
| `Binding loop detected: <prop>` | Two `<=>` two-way bindings or a circular `property: expr` chain. | Identify the cycle (`grep -n '<=>' senders/android/ui/`); break it by demoting one side to one-way `in` / `out`. |
| `Element <X> has no children` | Empty container after a refactor. | Remove the container, or add a fallback child (`Text { text: ""; }` is fine). |
| `Property <X> not used` | Stub property declared but never read. | Wire it to a binding, or drop it. |

### 1.3 Android cross-compile (if CI runner has the Android NDK)

```sh
cargo ndk -t arm64-v8a -p 26 build -p android-sender --release
# Or the project's existing cross-compile target. The error surface differs
# from desktop builds — Android-only Slint backends (linuxkms / skia-android)
# can fail on stubs that work fine on desktop.
```

---

## Section 2 — Desktop Slint preview check (fast iteration)

### 2.1 Per-phase preview targets

Each UI phase's reimplement guide lists the page(s) it touches. After the phase merges, open each touched page in `slint-viewer`:

```sh
# Phase 14 — audio settings
slint-viewer senders/android/ui/pages/audio_page.slint

# Phase 15 — camera settings
slint-viewer senders/android/ui/pages/camera_page.slint

# Phase 16 — bitrate presets
slint-viewer senders/android/ui/pages/bitrate_presets_page.slint
slint-viewer senders/android/ui/pages/bitrate_preset_edit_page.slint

# ... and so on for each phase.
```

Confirm the stub data renders as expected. Click through interactive elements (cyclers, toggles, sliders) and verify visual feedback.

### 2.2 Alt-mocks

Some phases ship multiple stub data sets to test edge cases. To preview an alt-mock, temporarily edit the page's `mock-*` initialiser, run `slint-viewer`, then revert:

```diff
-    in-out property <[CastHistoryEntry]> mock-history: [
-        { id: "h1", ... }, ..., { id: "h5", ... },
-    ];
+    in-out property <[CastHistoryEntry]> mock-history: [];   // empty state
```

Open in `slint-viewer`. Confirm the empty-state card renders ("No casts yet."). Revert.

Phases with documented alt-mocks:
- **Phase 5** — `mock-status-items-error` (severity-error variant).
- **Phase 6** — `mock-empty: true` (no devices).
- **Phase 17** — empty `bar-actions` (overflow banner test).
- **Phase 20** — empty `mock-history` (empty-state).
- **Phase 25** — empty `mock-macros` (empty-state).
- **Phase 26** — empty `log-entries` (empty log).

### 2.3 Component-only previews

Reusable components (`components/*.slint`) can be previewed standalone. The component file must have at least one `export component Name { ... }` and ideally a default-property initialiser so `slint-viewer` shows something:

```sh
slint-viewer senders/android/ui/components/confirm_dialog.slint
slint-viewer senders/android/ui/components/info_banner.slint
slint-viewer senders/android/ui/components/capture_preview.slint
slint-viewer senders/android/ui/components/icon_and_text.slint
```

If a component renders empty, add stub `in property <T> name: <stub>;` defaults so the standalone preview is non-empty.

---

## Section 3 — On-device validation matrix

For each shipped UI phase, the on-device check runs the relevant subset of Sections 4–8. Use this matrix to scope:

| Phase | 4 Touch | 5 Portrait | 6 Landscape | 7 Scroll | 8 Animation |
|---|---|---|---|---|---|
| 5 status overlay | — | ✓ | ✓ | — | — |
| 6 receiver list | ✓ | ✓ | ✓ | ✓ | ✓ (spinner) |
| 7 settings shell | ✓ | ✓ | ✓ | ✓ | — |
| 12 capture preview | — | ✓ | ✓ | — | — |
| 13 status items | — | ✓ | — | — | — |
| 14 audio settings | ✓ | ✓ | ✓ | — | — (slider) |
| 15 camera settings | ✓ | ✓ | ✓ | — | — (slider) |
| 16 bitrate presets | ✓ | ✓ | ✓ | ✓ | — |
| 17 quick-action customisation | ✓ | ✓ | — | — | — |
| 18 lifecycle overlays | ✓ | ✓ | ✓ | — | ✓ (countdown anim) |
| 19 backup / reset | ✓ | ✓ | — | — | ✓ (banner) |
| 20 cast history | ✓ | ✓ | — | ✓ | — |
| 21 help / about | — | ✓ | ✓ | ✓ | — |
| 22 network | ✓ | ✓ | — | ✓ | — |
| 23 recording | ✓ | ✓ | — | — | ✓ (state-machine button + Timer-driven counter) |
| 25 macros | ✓ | ✓ | — | ✓ | — |
| 26 debug log | — | ✓ | — | ✓ | — |
| 27 utils (per consumer migrated) | — | — | — | — | ✓ (banner) |

A "✓" means **run this section against the merged page**. A "—" means the phase doesn't introduce new risk in that dimension; skip unless visual review flags something.

---

## Section 4 — Touch-target validation (48dp minimum)

### 4.1 Procedure

1. Deploy the APK to the device.
2. Tap each interactive element in the merged phase's pages.
3. Record any element that feels "too small to hit reliably".

### 4.2 Audit grep

```sh
# Find rows / buttons / TouchAreas declared with explicit small sizes.
grep -REn '(height|min-height): *(1[0-9]|2[0-9]|3[0-9]|4[0-7])px' \
    senders/android/ui/ --include='*.slint'
# Any match below 48px is suspect — verify the element isn't interactive.
```

Spot the false-positives: `height: 24px` for status pills (text-only, not tappable) is fine. `height: 32px` for `ValueEditChip` step buttons (tappable, < 48px) is **a problem** if shipped — consider widening.

### 4.3 Fix

Update `Theme.row-height` to `48px` if rows feel too small system-wide. For specific buttons, set `min-height: 48px` directly.

---

## Section 5 — Portrait layout validation

### 5.1 Procedure

1. Launch the APK in portrait.
2. Open each merged phase's page.
3. Confirm layout flows correctly:
   - No horizontal scroll on the main view.
   - `CastControlBar` pinned to the bottom; doesn't overlap content.
   - `StatusOverlay` doesn't block the Stop Casting button.
4. If a list page: scroll through the stub data (or expand `mock-*` to 50+ entries — see Section 7).

### 5.2 Audit grep — pages without a `ScrollView` wrapping their main content

```sh
# Pages with multiple SettingsSection / SettingsValueRow elements MUST be
# inside a ScrollView, otherwise content overflows on small screens.
for page in senders/android/ui/pages/*.slint; do
    sections=$(grep -c 'SettingsSection\|SettingsValueRow\|SettingsToggleRow' "$page" 2>/dev/null)
    has_scroll=$(grep -c 'ScrollView' "$page" 2>/dev/null)
    if [ "$sections" -gt 3 ] && [ "$has_scroll" -eq 0 ]; then
        echo "WARN: $page has $sections rows but no ScrollView"
    fi
done
```

---

## Section 6 — Landscape layout validation

### 6.1 Procedure

1. Rotate device to landscape.
2. Open each merged phase's page.
3. Confirm:
   - `CastControlBar` still pins to the bottom.
   - No clipped text.
   - No horizontal overflow.

### 6.2 Common landscape failures

- **Fixed-height pages** that don't have a ScrollView — content clips off the bottom.
- **Hard-coded widths** that exceed landscape narrow-edge dimension.
- **`HorizontalLayout` without `horizontal-stretch`** on the main flex child — causes pinned-to-left rendering with empty trailing space.

### 6.3 Fallback

If landscape is broken in a way that's not trivially fixable in the current phase, lock orientation to portrait in `senders/android/AndroidManifest.xml` (`android:screenOrientation="portrait"`) and document the deferral. Don't ship a broken landscape mode.

---

## Section 7 — `ListView` scroll performance

### 7.1 Procedure

1. Temporarily expand `mock-devices` (Phase 6) or any `for` model to 50+ entries.
2. Scroll rapidly with finger / trackpad.
3. Watch for visible jank (frame drops, lag).

### 7.2 What to expect

Slint's `ListView` virtualises by default — only visible rows are instantiated. Performance should be smooth at 60fps regardless of model size. See [listview.mdx][listview] § "Elements are only instantiated if they are visible".

### 7.3 Common scroll-perf failures

```sh
# CRITICAL: ListView wrapped inside ScrollView disables virtualisation.
grep -B5 'ListView' senders/android/ui/ -r --include='*.slint' \
  | grep -B1 'ScrollView' \
  && echo "WARN: ListView possibly wrapped in ScrollView — disables virtualisation" \
  || echo "OK: no ListView-in-ScrollView nesting detected"
```

If the grep flags a real nesting, restructure: `ListView` is itself a scrolling viewport; double-wrapping breaks both the inner virtualisation and the outer scroll behaviour.

### 7.4 Per-row cost reduction

If perf is bad even with virtualisation, the per-row cost is the bottleneck:

- Image decoding: defer to scroll-idle, or use lower-resolution sources.
- Deep nesting: flatten; each row should be ~5 elements deep, not 15.
- Expensive bindings: `pure function` calls on every row binding can be expensive — hoist constants.

---

## Section 8 — Animation validation

### 8.1 Procedure

For each phase that ships an animation (see Section 3 matrix column 8):

1. Deploy the APK.
2. Trigger the animation (load page, click button, change state).
3. Confirm:
   - Animation plays end-to-end (not stuck at start or end).
   - No visible flicker during transition.
   - Timing feels appropriate (not too slow, not too fast).

### 8.2 Common animation failures

- **`animate <prop> { duration: 0; }` left in by mistake** — animation is instant.
- **Property write that breaks the animation binding** — Phase 18 § gotcha 31. Once the consumer writes the property imperatively, the `animate` directive no longer applies.
- **Timer with `running: false`** — the animation never triggers because the Timer driving it was never enabled. Audit `Timer { running: ...; }` expressions.

### 8.3 Audit grep

```sh
# Find every animate directive — verify each has a non-zero duration.
grep -REn 'animate +[a-z-]+ *\{ *duration: *0' senders/android/ui/ --include='*.slint'
# Expected: 0 matches.
```

---

## Section 9 — Panel routing validation

### 9.1 Procedure (smoke test for any phase that adds a Panel variant)

1. From `CastControlBar`, tap the "Settings" stub action.
2. Confirm `FullSettingsPage` opens (`Bridge.active-panel = Panel.settings`).
3. Tap into each new panel from the relevant SettingsSection:
   - Phase 14 `AUDIO & VIDEO → Audio` → AudioPage.
   - Phase 15 `AUDIO & VIDEO → Camera` → CameraPage.
   - Phase 16 `AUDIO & VIDEO → Bitrate presets` → BitratePresetsPage.
   - Phase 17 `DISPLAY → Quick actions` → QuickActionsPage.
   - Phase 18 `PRIVACY → ...` → lifecycle overlays.
   - Phase 19 `DATA → Backup & reset` → BackupResetPage.
   - Phase 20 `DATA → Cast history` → CastHistoryPage.
   - Phase 21 `ABOUT → About / Version history / Attributions` → respective page.
   - Phase 22 `ADVANCED → Network` → NetworkPage.
   - Phase 23 `AUDIO & VIDEO → Local recording` → RecordingPage.
   - Phase 25 `AUTOMATION → Macros` → MacrosPage.
   - Phase 26 `ADVANCED → Debug log / Video pipeline` → respective page.
4. From each new panel, tap "Done". Confirm the back-stack invariant:
   - From a top-level settings sub-page → returns to `Panel.settings`.
   - From a leaf sub-page (`MacroEditPage`, `BitratePresetEditPage`, `CastHistoryDetailPage`) → returns to its parent list panel, **not** `Panel.none`.

### 9.2 Audit grep

```sh
# Every panel routed in main.slint must also be reachable from
# settings_page.slint or another navigation surface.
grep -hoE 'Panel\.[a-z-]+' senders/android/ui/main.slint | sort -u > /tmp/routed.txt
grep -rohE 'Panel\.[a-z-]+ *=|Panel\.[a-z-]+' senders/android/ui/ \
    --include='*.slint' \
    --exclude='main.slint' --exclude='bridge.slint' \
  | sort -u > /tmp/referenced.txt

comm -23 /tmp/routed.txt /tmp/referenced.txt
# Expected: empty. Any output means a panel is routed in main.slint but
# never written to from anywhere — orphaned panel, likely dead code.
```

---

## Section 10 — Per-phase visual regression checklist

For each merged UI phase, capture a reference screenshot and attach it to the PR. The screenshot is the visual diff baseline.

### 10.1 Procedure

1. After deploying to device, take a screenshot of:
   - The phase's main page (default stub state).
   - One alt-mock variant (e.g. empty state, error state).
2. Attach both to the PR description.
3. Reviewer compares against any prior screenshot of the same surface (from a previous merge into the same area) — flag visual regressions.

### 10.2 Per-phase screenshot list

- **Phase 5** `CastingView` with overlay (info pills) + alt mock with error pill.
- **Phase 6** `ConnectView` populated + empty state.
- **Phase 7** `FullSettingsPage` (all four sections initially: AUDIO & VIDEO, ABOUT, PROTOCOL, ADVANCED — sections multiply across phases).
- **Phase 12** `CastingPage` with `CapturePreview` + StatusOverlay z-order.
- **Phase 14** `AudioPage` default state.
- **Phase 15** `CameraPage` default state.
- **Phase 16** `BitratePresetsPage` list + `BitratePresetEditPage`.
- **Phase 17** `QuickActionsPage` with reorderable rows + overflow banner state.
- **Phase 18** Lock overlay + Stealth overlay + SnapshotCountdown.
- **Phase 19** `BackupResetPage` + ConfirmDialog over each destructive row.
- **Phase 20** `CastHistoryPage` with status pills + detail page.
- **Phase 21** AboutPage + VersionHistoryPage + AttributionsPage + HelpPage.
- **Phase 22** `NetworkPage` with expanded interface row.
- **Phase 23** `RecordingPage` in idle / recording / paused / finalizing.
- **Phase 25** `MacrosPage` + `MacroEditPage` (with steps populated + add-step picker open).
- **Phase 26** `DebugLogPage` (filtered) + `DebugVideoPage`.

---

## Section 11 — Functional smoke tests (deferred)

End-to-end functional smoke is **not in scope** for the UI-only roadmap. Once Phase 8 reactivates, this section will be rewritten to include:

- Real device discovery → connect → cast → stop.
- Settings persistence across app restart.
- Macro execution end-to-end.
- Recording lifecycle (start → write file → finalise → playback).
- Cast history population from real events.

Until then, every interaction is a no-op and "smoke" reduces to "the UI doesn't crash". This is verified by Sections 1 (build), 2 (desktop preview), and 9 (panel routing).

---

## Section 12 — Continuous validation script

A single shell script that wraps Sections 1, 4 audit, 5 audit, 7 audit, and 9 audit. Run before every PR:

```sh
#!/usr/bin/env bash
# senders/android/ci/ui-validate.sh — Phase 10 audit script.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

echo "=== Build gate ==="
cargo build -p android-sender 2>&1 | tee /tmp/build.log

NEW_WARNINGS=$(grep -c '^warning:' /tmp/build.log || true)
if [ "$NEW_WARNINGS" -gt 0 ]; then
    echo "WARN: $NEW_WARNINGS build warnings"
fi

if grep -E "ERROR: missing layout size|Binding loop detected" /tmp/build.log; then
    echo "FAIL: Slint compiler errors"; exit 1
fi

cargo test -p android-sender

echo "=== Touch target audit ==="
SMALL_INTERACTIVE=$(grep -REn '(height|min-height): *(1[0-9]|2[0-9]|3[0-9]|4[0-7])px' \
                       senders/android/ui/ --include='*.slint' || true)
if [ -n "$SMALL_INTERACTIVE" ]; then
    echo "REVIEW: small heights detected"
    echo "$SMALL_INTERACTIVE"
fi

echo "=== ListView nesting audit ==="
if grep -B5 'ListView' senders/android/ui/ -r --include='*.slint' \
     | grep -B1 'ScrollView' > /tmp/listview-nest.txt; then
    if [ -s /tmp/listview-nest.txt ]; then
        echo "WARN: possible ListView-in-ScrollView nesting"
        head -10 /tmp/listview-nest.txt
    fi
fi

echo "=== Panel routing audit ==="
grep -hoE 'Panel\.[a-z-]+' senders/android/ui/main.slint | sort -u > /tmp/routed.txt
grep -rohE 'Panel\.[a-z-]+' senders/android/ui/ \
    --include='*.slint' \
    --exclude='main.slint' --exclude='bridge.slint' \
  | sort -u > /tmp/referenced.txt

ORPHANS=$(comm -23 /tmp/routed.txt /tmp/referenced.txt)
if [ -n "$ORPHANS" ]; then
    echo "WARN: orphan panels routed in main.slint:"
    echo "$ORPHANS"
fi

echo "=== animate-with-zero-duration audit ==="
grep -REn 'animate +[a-z-]+ *\{ *duration: *0' senders/android/ui/ \
    --include='*.slint' \
  && { echo "FAIL: animate with duration: 0"; exit 1; } \
  || echo "OK: no zero-duration animations"

echo "=== ALL UI VALIDATION CHECKS PASSED ==="
```

Save as `senders/android/ci/ui-validate.sh`, make executable (`chmod +x`), and add to CI as a required check.

---

## Section 13 — Gotchas (Phase 10 specific)

### Gotcha 61 — `slint-viewer` version drift hides bugs

**Symptom:** a page renders fine in `slint-viewer` but crashes or looks different on device.

**Cause:** `slint-viewer` was installed from crates.io and is a different version than the futo Slint fork that compiles into the app.

**Fix:** install `slint-viewer` from the same source as the app's Slint dependency. If the futo fork doesn't publish a viewer binary, build it locally:

```sh
git clone <futo-slint-fork-url> /tmp/slint-fork
cd /tmp/slint-fork && cargo build --release --package slint-viewer
ln -sf /tmp/slint-fork/target/release/slint-viewer ~/.cargo/bin/slint-viewer
```

Document the install path in the repo README so other contributors don't drift.

### Gotcha 62 — Empty alt-mock initialiser breaks the `for` element

**Symptom:** previewing an alt-mock with `mock-history: []` crashes `slint-viewer` or shows "for empty model" warning.

**Cause:** Slint compiler optimises empty arrays at compile time; some versions emit warnings for dead `for` bodies.

**Fix:** the `for` body should be guarded by an `if` for the empty-state UI anyway (Phase 20 pattern: `if mock-history.length == 0: <empty state> ... for entry in mock-history: <row>`). If the warning persists in `slint-viewer`, ignore it — the on-device behaviour is correct.

### Gotcha 63 — On-device touch tests don't catch all 48dp violations

**Symptom:** small buttons feel fine in development on a tablet, but users with smaller phones complain.

**Cause:** developer's device is large; 32px feels OK on a 6.7" display. On a 5.5" display, 32px is a struggle.

**Fix:** validate on the smallest target screen size (typically 5.5" 720x1280 emulator). The 48dp rule is screen-size-independent in dp units; if the actual pixels don't hit that, the device-specific scaling is wrong somewhere — review `Theme.row-height` in real `dp` rather than raw `px`.

### Gotcha 64 — `cargo test` doesn't run UI tests; the Slint test harness is separate

**Symptom:** `cargo test -p android-sender` passes but a UI bug ships.

**Cause:** Slint UI doesn't have unit tests in the conventional Rust sense — there's no `#[test] fn it_renders_correctly()`. UI tests would require a separate harness (Slint's `slint-build` test fixtures, or screenshot diff via image hashing).

**Fix:** acknowledge that `cargo test` covers Rust code only. Until automated UI tests land (post-Phase 8), `slint-viewer` + on-device + screenshot-attached-to-PR is the validation surface.

### Gotcha 65 — Stub data scaling test must be reverted before commit

**Symptom:** developer expands `mock-devices` to 100 entries to test scroll perf, then accidentally commits the bloated stub data.

**Cause:** quick edit, forgot to revert. The bloat then sits in master.

**Fix:** use `git stash` before the test:

```sh
git stash push -- senders/android/ui/pages/connect_page.slint
# Edit mock-devices to 100 entries.
slint-viewer senders/android/ui/pages/connect_page.slint
# Test perf, take notes.
git stash pop   # revert the stub data.
```

The stash protects against accidental commits.

---

## Section 14 — Exit criteria checklist (per UI phase)

- [ ] `cargo build -p android-sender` is clean — no new warnings, no errors.
- [ ] `cargo test -p android-sender` passes — migration tests do not regress.
- [ ] `slint-viewer` opens every page touched by the phase without errors.
- [ ] At least one alt-mock (empty state, error state, edge case) verified in `slint-viewer` or on-device.
- [ ] On-device touch test passed for every interactive row added by the phase (per Section 3 matrix).
- [ ] On-device portrait layout verified for every page added (per Section 3 matrix).
- [ ] On-device landscape layout verified, OR orientation locked to portrait with documented rationale.
- [ ] If the phase ships a list-view page, scroll perf tested with 50+ stub entries (per Section 7).
- [ ] If the phase ships an animation, animation plays end-to-end on device.
- [ ] If the phase adds a Panel variant, the back-stack returns to the correct parent panel (Section 9).
- [ ] Reference screenshot attached to the PR per Section 10.
- [ ] CI script (Section 12) passes locally before merge.

---

## Section 15 — When Phase 8 reactivates

Phase 10 expands to include:

- **End-to-end functional smoke** — start cast, transmit, stop cast, verify all real flows work.
- **Settings persistence** — quit + restart app; settings persist via Rust-driven storage.
- **Cast history population** — perform a cast, verify it appears in `CastHistoryPage`.
- **Macro execution** — define a macro, trigger via quick action, verify execution.
- **Recording lifecycle** — start, write file, stop, verify playback (in another media player).
- **Lifecycle overlay engagement** — engage lock, verify keyguard interaction; engage stealth, verify FLAG_SECURE.
- **Network interface enumeration** — verify the listed interfaces match `ip addr` output.
- **Wi-Fi Aware** — verify enablement on devices that support it; graceful failure on devices that don't.
- **Backup / import / reset** — round-trip a JSON export/import, verify settings restore correctly.

This list is the inverse of the "What's NOT in this phase" section in the original spec — once Phase 8 reactivates, those deferrals dissolve into real test cases.

---

## Slint-doc references used

- **`ListView` virtualisation** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx`.
- **`ScrollView`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx`.
- **`slint-viewer` desktop preview** — `draft/slint-ui/docs/astro/src/content/docs/guide/tooling/live-preview.mdx`.
- **Layout sizing requirements** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **`animate <prop> { duration, easing }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/animation.mdx`.
- **Property binding loops** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **Best practices (general)** — `draft/slint-ui/docs/astro/src/content/docs/guide/development/best-practices.mdx`.
- **Debugging techniques** — `draft/slint-ui/docs/astro/src/content/docs/guide/development/debugging_techniques.mdx`.
- **48dp touch target rule** — Material Design 3 (external; not a Slint doc, but a UI standard).

---

## What's NOT in this guide

- **Functional / casting / discovery tests** — deferred with Phase 8.
- **Automated screenshot-diff** — manual reference screenshots only until tooling lands.
- **Espresso / instrumented Android tests** — out of scope while UI is unstable.
- **Performance profiling** beyond visible jank — Slint has no built-in profiler in the futo fork.
- **Accessibility audit** (TalkBack, font scaling) — separate concern; revisit when accessibility tokens are added to `theme.slint`.
- **`@tr(...)` / localisation** validation — Phase 9.
- **Bridge wiring** validation — Phase 8.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-10-testing.md
[listview]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx

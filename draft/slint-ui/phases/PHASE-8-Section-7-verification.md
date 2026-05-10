# Phase 8 — Section 7: Per-cluster verification

> Section 7 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-6-cluster-E-overlay-invariants.md`](./PHASE-8-Section-6-cluster-E-overlay-invariants.md) first.

**Run all of the following after each cluster.** Don't batch — catching regressions per cluster is easier than untangling a 1500-line PR.

| Check | Command | Where it lives |
|---|---|---|
| 7.1 | No remaining `mock-*` on the migrated surface | `grep` |
| 7.2 | The promoted Bridge property is declared | `grep` |
| 7.3 | The promoted callback has a Rust handler | `grep` |
| 7.4 | No Slint-side direct mutations of Rust-driven properties | `grep` |
| 7.5 | Build + lint | `cargo build` + `cargo clippy` |
| 7.6 | Smoke test | `slint-viewer` |
| 7.7 | On-device sanity (after all clusters) | `adb` |

---

## 7.1 — No remaining `mock-*` on the migrated surface

Run this **per cluster** with the cluster's file set:

```sh
# Per-cluster — replace <paths-touched-this-cluster> with the actual files.
grep -n 'mock-' senders/android/ui/<paths-touched-this-cluster>
```

**Expected per cluster:**

| Cluster | Files | Expected `mock-*` count after migration |
|---|---|---|
| F | `bridge.slint`, `components/info_banner.slint` | 0 (Cluster F adds — doesn't touch mocks) |
| A | `components/status_badges.slint`, `pages/recording_page.slint`, `pages/network_page.slint`, `pages/debug_log_page.slint`, `about/version pages` | 0 in those files for the migrated subset (`mock-min-level-idx` may remain in debug_log_page — page-local UI filter, not a Rust signal) |
| B | `pages/audio_page.slint`, `pages/camera_page.slint`, `pages/recording_page.slint`, `pages/network_page.slint` | 0 for audio/camera/wifi-aware/recording-state/recording-elapsed; `mock-format-idx`, `mock-folder-idx`, `mock-record-audio`, `mock-disk-free-mb` may remain (page-local UI choices) |
| C | `pages/bitrate_presets_page.slint`, `pages/bitrate_preset_edit_page.slint`, `pages/quick_actions_page.slint`, `components/control_bar.slint`, `pages/macros_page.slint`, `pages/macro_edit_page.slint` | 0 for `mock-presets`, `mock-quick-actions`, `mock-bar-actions`, `mock-macros`, `mock-macro-edit-id` |
| D | `pages/backup_reset_page.slint`, `pages/cast_history_page.slint`, `pages/cast_history_detail_page.slint` | 0 for `mock-history`. Page-local `pending-action` and `confirm-visible` stay |
| E | (no code changes) | unchanged |

**Acceptable remaining `mock-*` properties** as of post-D:

```
pages/debug_log_page.slint:N: in-out property <int> mock-min-level-idx: 1;       # filter UI state
pages/recording_page.slint:N: in-out property <int>  mock-format-idx: 0;          # page-local choice
pages/recording_page.slint:N: in-out property <int>  mock-folder-idx: 0;          # page-local choice
pages/recording_page.slint:N: in-out property <bool> mock-record-audio: true;     # page-local choice
pages/recording_page.slint:N: in-out property <int>  mock-disk-free-mb: 12480;    # placeholder until Phase 11
```

These are **intentionally page-local** — document each in a one-line comment so future readers don't migrate them by mistake.

The full inventory (Section 0.2) should drop to <5 entries by end of Cluster D.

---

## 7.2 — The promoted Bridge property is declared

For each item migrated this cluster:

```sh
grep -n '<promoted_property>' senders/android/ui/bridge.slint
# Expected: at least 1 match.
```

Examples:

```sh
# Cluster F:
grep -nE 'banner-(message|visible|severity)|BannerSeverity' senders/android/ui/bridge.slint
# Expected: 4+ matches (3 properties + 1 enum declaration).

# Cluster A:
grep -nE 'status-items|app-version|network-interfaces|recording-state|recording-elapsed-s|log-entries' senders/android/ui/bridge.slint
# Expected: 6+ matches.

# Cluster B:
grep -nE 'audio-source-idx|audio-muted|audio-input-gain|audio-bitrate-idx|camera-idx|camera-mirror-front|camera-stabilization|camera-tap-to-focus|camera-zoom-level|resolution-idx|framerate-idx|wifi-aware-enabled|snapshot-secs|engage-lock|engage-stealth|start-snapshot-countdown|exit-lifecycle|set-wifi-aware|start-recording|pause-recording|resume-recording|stop-recording' senders/android/ui/bridge.slint
# Expected: 22+ matches.

# Cluster C:
grep -nE 'presets|macros|macro-edit-id|move-bar-action|set-bar-action-enabled|save-bar-actions|save-preset|delete-preset|set-active-preset|save-macro|delete-macro|move-step|add-step|remove-step|run-macro|clear-log-entries' senders/android/ui/bridge.slint
# Expected: 16+ matches.

# Cluster D:
grep -nE 'history|selected-history-entry|export-settings|import-settings|reset-settings|clear-cast-history|clear-known-receivers|clear-history|delete-history-entry|recast' senders/android/ui/bridge.slint
# Expected: 10+ matches.
```

---

## 7.3 — The promoted callback has a Rust handler

For each item migrated this cluster:

```sh
grep -n 'on_<promoted_callback>\|set_<promoted_property>' senders/android/src/lib.rs
# Expected: 1+ matches per item in the cluster.
```

Examples:

```sh
# Cluster A:
grep -nE 'set_status_items|set_app_version|set_network_interfaces|set_recording_state|set_recording_elapsed_s|set_log_entries' senders/android/src/lib.rs
# Expected: 6+ matches (one per setter).

# Cluster B:
grep -nE 'on_(start|pause|resume|stop)_recording|on_engage_(lock|stealth)|on_start_snapshot_countdown|on_exit_lifecycle|on_set_wifi_aware' senders/android/src/lib.rs
# Expected: 9 matches.

# Cluster C:
grep -nE 'on_(save|delete|set_active)_preset|on_(move_bar_action|set_bar_action_enabled|save_bar_actions)|on_(save|delete|run)_macro|on_(move|add|remove)_step|on_clear_log_entries' senders/android/src/lib.rs
# Expected: 13+ matches.

# Cluster D:
grep -nE 'on_(export|import|reset)_settings|on_clear_(cast_history|known_receivers|history)|on_delete_history_entry|on_recast' senders/android/src/lib.rs
# Expected: 8 matches.
```

---

## 7.4 — No Slint-side direct mutations of Rust-driven properties

```sh
# For every `in property <T> X` in bridge.slint (Rust-driven only — not in-out),
# Slint must NOT have any direct writes. If a write is found, it's a bug.

grep -nE 'Bridge\.(presets|macros|history|status-items|app-version|network-interfaces|log-entries|recording-state|recording-elapsed-s|selected-history-entry) *=' senders/android/ui/
# Expected: 0 matches.
```

**Exception:** `Bridge.lifecycle` direct mutations stay in `LockOverlay` / `StealthOverlay` / `SnapshotCountdown` for state-transition exits — that's by design per Cluster E (Section 6.3).

```sh
grep -nE 'Bridge\.lifecycle *=' senders/android/ui/
# Expected: ~3 matches (one per overlay's exit path).
```

---

## 7.5 — Build + lint

```sh
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

**Common failures** by error message:

| Error | Likely cause | Fix |
|---|---|---|
| `cannot find type Macro / BitratePreset / StatusItem in this scope` | The struct is exported from `bridge.slint` but Rust hasn't run `slint::include_modules!()`. | Add `slint::include_modules!();` near the top of `lib.rs`. |
| `the trait bound 'X: Into<SharedString>' is not satisfied` | Rust passed `&str` to a `set_X(...)` setter but `String` is required. | `.into()` the string. |
| `cannot move out of dereference` on a `Bridge` getter | `bridge.get_*()` returns a `SharedString`/`ModelRc` reference. | `.to_string()` the SharedString; `.iter().collect()` the model. |
| `binding loop detected on Bridge.recording-elapsed-s` | A Slint binding writes back to the same property it reads. | Cluster A4: removed the `Timer { triggered => mock-elapsed-s += 1 }` block in Slint; ticker lives in Rust now. |
| `non-exhaustive match` on `Panel::*` | Cluster E referenced a panel that doesn't exist in the Rust binding. | Match the Slint enum names exactly (PascalCase). Slint generates one variant per enum value. |

---

## 7.6 — Smoke test in `slint-viewer`

```sh
slint-viewer senders/android/ui/main.slint
```

**What to look for:**

- Each page renders.
- No "binding loop detected" or "missing layout size" warnings in the console.
- Without Rust, properties show their declared default (`[]`, `""`, `false`, `0`). That's expected — verify there are no compile errors and no obvious layout regressions.

For per-page testing:

```sh
slint-viewer senders/android/ui/pages/audio_page.slint
slint-viewer senders/android/ui/pages/recording_page.slint
slint-viewer senders/android/ui/pages/cast_history_page.slint
# … etc.
```

Each page has its own `export component`, so `slint-viewer` will pick the first one and render it standalone.

**Skipping smoke test on CI:**

If `slint-viewer` is not available (headless container, locked-down CI), skip 7.6. The on-device walkthrough (7.7) still applies.

---

## 7.7 — On-device sanity (only after all clusters)

```sh
cargo build -p android-sender --release
# Install + open the app. Walk through:
```

| # | Step | Expected behavior |
|---|---|---|
| 1 | Connect page — receivers populate | mDNS scan starts on launch; a real receiver appears within ~3s |
| 2 | Settings → Audio | Tap "Source" cycles Mic → System → Both. Mute toggle persists across panel close/reopen |
| 3 | Settings → Camera | Tap "Camera" cycles Front → Back → External. Zoom slider drags smoothly |
| 4 | Settings → Bitrate | 4 default presets; tap a row → pill highlights; "Add preset" → editor opens with empty draft; Save → returns; new row visible |
| 5 | Settings → Recording | Idle. Tap red dot → Recording. Counter ticks 00:00:01, 00:00:02, ... Tap Pause → counter freezes. Resume → counter resumes from where it paused. Stop → goes to Finalizing briefly then Idle |
| 6 | Settings → Privacy → Lock screen | Lock overlay appears. Long-press the dot to unlock → returns to last panel |
| 7 | Settings → Privacy → Stealth | Black overlay. Tap → returns to last panel |
| 8 | Settings → Privacy → Cast with countdown (5s) | Countdown 5 → 4 → 3 → 2 → 1 → starts cast |
| 9 | Settings → Backup & reset → Reset → confirm | Banner: "Settings reset to defaults" (severity=success). Bitrate presets list shows only the 4 defaults (any custom presets gone) |
| 10 | Cast history → tap a row | Detail page renders. Recast button → banner: "Recasting to <name>". Delete entry → routes back, row gone |
| 11 | Cast history → Clear all → confirm | List empty. Banner: "Cast history cleared" |
| 12 | Quick action bar | 7 actions visible (release) or 11 actions visible (debug). Tap each panel-opening action → corresponding page opens. Tap codec-test action → codec-test panel opens; result string updates within ~2s |
| 13 | Pair via QR | Page renders with QR placeholder + "Done" button (real QR comes in Phase 24's follow-up) |

If **any** check above fails, the corresponding cluster has a regression. Re-read that cluster's section, run its dedicated verification, and fix.

---

## 7.8 — Cluster verification cheat-sheet

For copy-paste convenience:

```sh
# Run after every commit:
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l
cargo build -p android-sender 2>&1 | tail -10
cargo clippy -p android-sender --all-targets -- -D warnings 2>&1 | tail -10
```

Three lines, ~30 seconds. If any of them surface unexpected output, stop and reconcile before continuing.

---

## 7.9 — Reporting cluster status in the PR description

If you split Phase 8 into 6 PRs (one per cluster), each PR description should include:

```markdown
## Cluster <X> verification

- [x] mock-* count delta: 31 → 24 (matches Section 0.2 expectation)
- [x] Bridge.<promoted-property-set> declared
- [x] lib.rs has set_<x> / on_<x> for every listed item
- [x] cargo build green
- [x] cargo clippy --all-targets -- -D warnings green
- [x] slint-viewer renders without warnings
- [ ] On-device walkthrough (deferred to all-clusters PR)
```

The on-device walkthrough is most efficient as a single pass after **all** clusters land — the per-cluster PRs can defer it.

---

## 7.10 Exit criteria for Section 7

- [ ] Per-cluster `grep` checks in 7.1–7.4 pass
- [ ] `cargo build -p android-sender` green at HEAD
- [ ] `cargo clippy -p android-sender --all-targets -- -D warnings` green at HEAD
- [ ] `slint-viewer senders/android/ui/main.slint` renders without warnings
- [ ] On-device walkthrough (7.7) executed in full and all 13 steps pass

You can now move to **Section 8 — pitfalls** at [`PHASE-8-Section-8-pitfalls.md`](./PHASE-8-Section-8-pitfalls.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/quickstart.mdx` — `slint-viewer` invocation.

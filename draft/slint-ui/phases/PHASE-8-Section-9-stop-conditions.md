# Phase 8 — Section 9: Stop conditions

> Section 9 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-8-pitfalls.md`](./PHASE-8-Section-8-pitfalls.md) first.

**Phase 8 is "done" — and the placeholder gate in [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md) can be removed — when ALL of the conditions below hold.**

This section is the canonical exit criteria. If any item is unchecked, Phase 8 is incomplete; do not flip `STATUS.md` or close the migration PR.

---

## 9.1 — Per-cluster completeness

| Cluster | Done? | Section ref |
|---|---|---|
| F | [ ] All shared banner tokens declared in `bridge.slint`; `flash_banner` helper in `lib.rs` | [Section 1](./PHASE-8-Section-1-cluster-F-shared-tokens.md) |
| A1 | [ ] `Bridge.status-items` driven from Rust periodic poll | [Section 2.1](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#21--a1--status-overlay-items) |
| A2 | [ ] `Bridge.app-version` pushed once at startup | [Section 2.2](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#22--a2--app-version) |
| A3 | [ ] `Bridge.network-interfaces` + `set-interface-enabled` | [Section 2.3](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#23--a3--network-interfaces) |
| A4 | [ ] `Bridge.recording-state` + `recording-elapsed-s` ticker | [Section 2.4](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#24--a4--recording-elapsed-counter) |
| A5 | [ ] `Bridge.log-entries` + `LogRing` tracing-subscriber layer | [Section 2.5](./PHASE-8-Section-2-cluster-A-readonly-view-models.md#25--a5--debug-log-entries) |
| B1 | [ ] Audio settings via `Bridge.audio-*` | [Section 3.1](./PHASE-8-Section-3-cluster-B-single-page-state.md#31--b1--audio-settings) |
| B2 | [ ] Camera settings via `Bridge.camera-*` etc. | [Section 3.2](./PHASE-8-Section-3-cluster-B-single-page-state.md#32--b2--camera-settings) |
| B3 | [ ] Recording state-machine callbacks (`start`/`pause`/`resume`/`stop`) | [Section 3.3](./PHASE-8-Section-3-cluster-B-single-page-state.md#33--b3--recording-controls-state-machine-writes) |
| B4 | [ ] Lifecycle callbacks (`engage-lock`, `engage-stealth`, `start-snapshot-countdown`, `exit-lifecycle`); `mock-snapshot-secs` renamed | [Section 3.4](./PHASE-8-Section-3-cluster-B-single-page-state.md#34--b4--lifecycle-modes--snapshot-countdown) |
| B5 | [ ] Wi-Fi Aware via `set-wifi-aware`; auto-hide banner via `flash_banner` | [Section 3.5](./PHASE-8-Section-3-cluster-B-single-page-state.md#35--b5--wi-fi-aware-toggle) |
| C1 | [ ] Bitrate presets via `Bridge.presets` + 3 callbacks | [Section 4.1](./PHASE-8-Section-4-cluster-C-list-mutations.md#41--c1--bitrate-presets) |
| C2 | [ ] **B12 fix landed:** `CastControlBar` reads from `Bridge.quick-actions`; customisation page mutates the same model | [Section 4.2](./PHASE-8-Section-4-cluster-C-list-mutations.md#42--c2--quick-action-customisation-and-the-live-castcontrolbar-unification--fixes-b12) |
| C4 | [ ] Macros via `Bridge.macros` + 6 callbacks | [Section 4.3](./PHASE-8-Section-4-cluster-C-list-mutations.md#43--c4--macros) |
| C5 | [ ] Debug log clear via `Bridge.clear-log-entries` | [Section 4.4](./PHASE-8-Section-4-cluster-C-list-mutations.md#44--c5--debug-log-clear) |
| D1 | [ ] Backup/reset 5 callbacks; banner via Cluster F | [Section 5.1](./PHASE-8-Section-5-cluster-D-destructive-flows.md#51--d1--backup--reset) |
| D2 | [ ] Cast history via `Bridge.history` + `clear-history` / `delete-history-entry` / `recast` | [Section 5.2](./PHASE-8-Section-5-cluster-D-destructive-flows.md#52--d2--cast-history) |
| E | [ ] `Bridge.active-panel` / `lifecycle` / `app-state` invariants documented | [Section 6](./PHASE-8-Section-6-cluster-E-overlay-invariants.md) |

---

## 9.2 — Cross-cutting checks

1. **No `mock-*` properties on `pages/*.slint` or `components/*.slint` for migrated surfaces.**

   ```sh
   grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/
   ```

   The remaining matches must be on the **explicitly page-local exception list**, each with a one-line comment explaining why:

   - `pages/debug_log_page.slint` — `mock-min-level-idx` (filter UI state)
   - `pages/recording_page.slint` — `mock-format-idx`, `mock-folder-idx`, `mock-record-audio`, `mock-disk-free-mb` (page-local UI choices, not Rust signals)
   - any others documented as "intentionally page-local" in their cluster's section

2. **Every `Bridge.*` declaration in `bridge.slint` has either a producer or a consumer in Rust** (no orphan declarations):

   ```sh
   # For each property X declared in bridge.slint:
   grep -nE 'set_<x>|get_<x>|on_<x>' senders/android/src/lib.rs
   # Should be ≥ 1 match per property.
   ```

3. **Build green:**

   ```sh
   cargo build -p android-sender
   cargo clippy -p android-sender --all-targets -- -D warnings
   ```

4. **Cluster verification re-run.** Each cluster's verification (Section 7) should be re-runnable on HEAD without surfacing new failures. If a re-run flags a new mock-* or a new direct-write to a Rust-driven property, that's a regression — fix before declaring Phase 8 complete.

5. **On-device walkthrough.** All 13 steps of [Section 7.7](./PHASE-8-Section-7-verification.md#77--on-device-sanity-only-after-all-clusters) execute end-to-end without surfacing a stale page-local model. Specifically:

   - The bar's actions match the customisation page's order (B12 fix).
   - Reset → confirm clears every list (presets, macros, history) back to defaults.
   - Recording start → pause → resume → stop transitions cleanly through `RecordingState` enum.

---

## 9.3 — Documentation updates required at "done"

When all 9.1 + 9.2 checks pass, update the following docs in the same PR (or a follow-up doc PR):

### 9.3.1 `STATUS.md`

Flip Phase 8 from `[~] Migration plan only` to `[x] Complete`:

```diff
-| 8  | Rust bridge reactivation       | [~] Migration plan only (PR #1) | bridge migration plan shipped; execution deferred. |
+| 8  | Rust bridge reactivation       | [x] Complete                    | All clusters migrated; see commits <range> and PR #<n>. |
```

### 9.3.2 `PHASE-8-rust-bridge.md` (the original spec)

Replace the "explicitly **deferred**" header with a "Reactivated YYYY-MM-DD" header pointing at this guide:

```diff
-> **Status:** UI-only build. The Rust bridge described below is **deferred**
-> until the UI work in Phases 5-27 lands.
+> **Status:** Reactivated YYYY-MM-DD. See
+> [`PHASE-8-implementation-instructions.md`](./PHASE-8-implementation-instructions.md)
+> and the `PHASE-8-Section-*.md` series for the execution log.
```

### 9.3.3 `phases/README.md`

Phase 8 row gets the same treatment as 9.3.1 — flip from "[~] Migration plan only" to "[x] Complete" with a commit-range link.

### 9.3.4 (optional) `PANEL-INVARIANTS.md`

If you chose to create this dedicated doc instead of inline comments in `main.slint` (Cluster E option B), make sure it links back to all six cluster section files for context.

---

## 9.4 — When NOT to declare Phase 8 done

- ❌ "Most clusters are wired." — All 6 clusters (F, A, B, C, D, E) must be complete. Half-done is half-broken.
- ❌ "B12 isn't fixed yet but the bar still works." — B12 is the most important migration in Cluster C. If the bar reads from `mock-quick-actions`, Phase 8 is incomplete and the user-facing bug is the canonical demonstration.
- ❌ "On-device walkthrough was skipped." — `cargo build` green is necessary but not sufficient. Phase 8's whole point is bridging UI ⇄ Rust; only the on-device test exercises that bridge.
- ❌ "Some `mock-*` properties remain." — UNLESS each remaining match is on the explicit exception list (Section 9.2.1), Phase 8 is incomplete.

---

## 9.5 — Follow-up phases unblocked by Phase 8 completion

Once Phase 8 lands and `STATUS.md` shows `[x] Complete`, the following are unblocked:

- **Phase 11** — peripherals & lifecycle (real BatteryManager, ConnectivityManager, WifiAwareManager, NetworkInterface enumeration). Phase 11's diff against Phase 8 is just replacing the placeholder pushes with real JNI calls; the Bridge surface stays identical.
- **Phase 28+** — chat / streaming / scenes / peripheral / media-player. Each of those phases brings its own producers and consumers, but they all sit on top of the Bridge contract Phase 8 enforces.
- **Production cast pipeline.** With audio/camera/recording settings reachable from Rust, the actual `MediaRecorder` / `MediaProjection` plumbing can land without UI churn.

---

## 9.6 — Final commit message convention

After all clusters land, the closing PR description should look something like:

```markdown
# Phase 8 — Rust bridge reactivation (complete)

This PR closes Phase 8 per the spec in `PHASE-8-rust-bridge.md` and the
execution guide in `PHASE-8-implementation-instructions.md` (TOC) /
`PHASE-8-Section-*.md` (per-cluster details).

## Summary

- Cluster F shipped in <commit-or-PR>
- Cluster A shipped in <commit-or-PR>
- Cluster B shipped in <commit-or-PR>
- Cluster C shipped in <commit-or-PR>  ← includes the B12 fix
- Cluster D shipped in <commit-or-PR>
- Cluster E shipped in <commit-or-PR>

## mock-* inventory delta

```
Master baseline:  31 lines
Phase 8 final:     5 lines (page-local UI state, documented per-row)
```

## Verification

- [x] `cargo build -p android-sender` green
- [x] `cargo clippy -p android-sender --all-targets -- -D warnings` green
- [x] On-device walkthrough (PHASE-8-Section-7.7) passes all 13 steps
- [x] B12 visually verified (bar matches customisation order)

## Docs synced

- `STATUS.md` — Phase 8 → Complete
- `PHASE-8-rust-bridge.md` — header updated
- `phases/README.md` — Phase 8 → Complete
```

---

## 9.7 — What's NOT in Phase 8

These items are explicitly **out of scope** for Phase 8. They are listed here so future contributors don't conflate them with Phase 8:

- **`@tr()` localisation sweep over the new strings.** That's [Phase 9](./PHASE-9-reimplement-instructions.md). Do it as a follow-up — every `set_*` call site that pushes user-visible text needs `@tr` wrapping in the Slint consumer, but the Rust side stays English-only (Slint's translator extractor walks `.slint` only).

- **Slint-viewer screenshot regression suite.** That's [Phase 10](./PHASE-10-reimplement-instructions.md). Phase 8 verifies via on-device walkthrough; Phase 10 will add an automated visual diff CI job.

- **Real platform plumbing for the placeholders.** Phase 8 produces non-trivial stubs (e.g. status-items pushed every 5s with hardcoded values; `flash_banner` after a 500ms `tokio::time::sleep`). Phase 11 replaces the stubs with real JNI calls, but the Bridge surface stays identical — Phase 8 already declared the right shape.

- **Chat / streaming / scenes / peripherals.** Phases 28+. Phase 8's Bridge declaration grows when those phases land, but the existing Phase 8 work doesn't get rewritten — it's a stable foundation.

---

## 9.8 — Exit criteria for Section 9 (and for Phase 8 as a whole)

Phase 8 is **DONE** when every checkbox in 9.1 and 9.2 is checked, every doc in 9.3 is updated, and the closing PR description (9.6) accurately reflects the work.

Until all of those are true: Phase 8 is in progress. Continue.

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

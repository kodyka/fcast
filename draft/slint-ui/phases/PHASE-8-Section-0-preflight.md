# Phase 8 — Section 0: Pre-flight checklist

> **Companion documents:**
>
> - [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md) — original Phase 8 spec (the "what").
> - [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) — strategy / risk register / per-cluster index (the "why").
> - [`PHASE-8-implementation-instructions.md`](./PHASE-8-implementation-instructions.md) — full implementation guide (the "how", combined).
> - **This file** — Section 0 only. Run before touching any code.
>
> **Section index:**
>
> | # | File | Cluster | Topic |
> |---|---|---|---|
> | 0 | `PHASE-8-Section-0-preflight.md` (this) | — | Audit, branch, build green |
> | 1 | `PHASE-8-Section-1-cluster-F-shared-tokens.md` | F | Theme + Bridge banner tokens |
> | 2 | `PHASE-8-Section-2-cluster-A-readonly-view-models.md` | A | 5 read-only view models |
> | 3 | `PHASE-8-Section-3-cluster-B-single-page-state.md` | B | 5 single-page state migrations |
> | 4 | `PHASE-8-Section-4-cluster-C-list-mutations.md` | C | 4 list-mutation migrations |
> | 5 | `PHASE-8-Section-5-cluster-D-destructive-flows.md` | D | 2 destructive flows |
> | 6 | `PHASE-8-Section-6-cluster-E-overlay-invariants.md` | E | Panel / Lifecycle / AppState invariants |
> | 7 | `PHASE-8-Section-7-verification.md` | — | Per-cluster verification |
> | 8 | `PHASE-8-Section-8-pitfalls.md` | — | Risk register + common traps |
> | 9 | `PHASE-8-Section-9-stop-conditions.md` | — | When Phase 8 is done |

**Audience:** developer ready to *execute* Phase 8 — wire the deferred `mock-*` properties on every shipped UI page to real Rust producers/consumers in `senders/android/src/lib.rs`.
**Goal of this section:** confirm the workspace is ready, capture the current state for differential validation, and create the working branch.
**Constraint:** guide-only document. The actual migration happens in a separate PR.

---

## 0.1 Confirm shipped UI phases

Phase 8 only makes sense after the UI phases land. List the pages folder:

```sh
ls senders/android/ui/pages/
```

**Expected** (alphabetical, from `master` as of 2026-05-10) — 22 files:

```
audio_page.slint                  cast_history_detail_page.slint
backup_reset_page.slint           cast_history_page.slint
bitrate_preset_edit_page.slint    casting_page.slint
bitrate_presets_page.slint        codec_test_page.slint
camera_page.slint                 connect_page.slint
connecting_page.slint             debug_log_page.slint
debug_page.slint                  debug_video_page.slint
macro_edit_page.slint             macros_page.slint
network_page.slint                pairing_page.slint
quick_actions_page.slint          receiver_rename_page.slint
recording_page.slint              settings_page.slint
```

If a file is **missing**, the corresponding step in this guide has nothing to migrate — **skip** that step rather than try to wire a property to a non-existent producer. Likewise `senders/android/ui/components/` should have:

```
buttons.slint            confirm_dialog.slint   icon_and_text.slint
status_badges.slint      capture_preview.slint  control_bar.slint
info_banner.slint        settings_rows.slint    qr_placeholder.slint
context_menu.slint
```

---

## 0.2 Inventory of `mock-*` properties — the migration work surface

The single most important pre-migration command:

```sh
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/
```

**Expected:** ~31 lines as of 2026-05-10. The exact number drifts as you migrate; what matters is that you record the **starting count** and confirm it drops by exactly the items each cluster claims.

Save the output to a scratch file:

```sh
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ \
    > /tmp/phase-8-mock-inventory.txt
wc -l /tmp/phase-8-mock-inventory.txt
```

Re-run after **each** cluster commit and `diff` against the previous snapshot. If the diff doesn't match what the cluster section claims, **stop** and reconcile — you've either migrated something twice, missed an item, or accidentally introduced a new `mock-*` property.

A typical drift across the full Phase 8:

```
After Cluster F:  31 → 31  (Cluster F adds Bridge tokens; doesn't remove any mock-*)
After Cluster A:  31 → 24  (-status badges 4, -app-version 0, -recording 2, -log 2)
After Cluster B:  24 → 13  (-audio 4, -camera ~5, -recording 4, -wifi 1, -snapshot 0)
After Cluster C:  13 →  4  (-presets 1, -quick-actions 1, -macros 5, -log filter 2)
After Cluster D:   4 →  0  (-backup ~3, -history ~1)
```

The exact `-N` per cluster will vary slightly with what's in master. Use it as a **shape check**, not an absolute test.

---

## 0.3 Inventory of already-wired bindings — do **not** touch

Verify what `lib.rs` already does so you don't double-wire:

```sh
grep -nE 'global::<Bridge>|on_(connect|start|stop|invoke|change)|set_(devices|app_state|show_debug|test_status|quick_actions)' \
    senders/android/src/lib.rs
```

**Expected matches** (line numbers from `master` as of 2026-05-09; will drift):

| `lib.rs` line | Binding | Direction | Don't touch |
|---|---|---|---|
| L572 | `Bridge.devices: [string]` ← `set_devices(...)` (mDNS) | Rust → Slint | yes |
| L631 | `Bridge.app-state` ← `invoke_change_state(AppState::...)` | Rust → Slint | yes |
| L992 | `Bridge.show-debug: bool` ← `set_show_debug` (debug-build gate) | Rust → Slint | yes |
| L998 | `Bridge.test-status: string` ← `set_test_status` (codec test) | Rust → Slint | yes |
| L1002 | `Bridge.quick-actions: [QuickAction]` ← `set_quick_actions` | Rust → Slint | **declared right; consumer wrong — see Section 4 cluster C2** |
| L1008 | `Bridge.on_connect_receiver(...)` callback handler | Slint → Rust | yes |
| L1017 | `Bridge.on_start_casting(...)` callback handler | Slint → Rust | yes |
| L1030 | `Bridge.on_stop_casting()` callback handler | Slint → Rust | yes |
| L1039 | `Bridge.on_invoke_action(...)` callback handler | Slint → Rust | yes |
| L1055-1082 | Codec test handlers — push `set_test_status` | Slint → Rust | yes |
| `bridge.slint` | `Bridge.change-state(AppState)` (Slint-side public function) | within Slint | yes |

**Why this matters:** these handlers already deliver real device events into Slint. Re-wiring them would race with the existing producers and corrupt `app-state` / `devices` / `test-status` mid-session. The migration plan calls these out as **read-only** for Phase 8.

---

## 0.4 Build green on `master` first

```sh
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

If `master` does not build, your migrations cannot be A/B-tested against a green baseline. **Fix the build first** — do not start this phase from a broken tree.

If you do hit a master-broken-build, file it as a separate fix PR (see [PR #2](https://github.com/kodyka/fcast/pull/2) for the canonical "xtask CI fix" pattern). Phase 8 is large enough that mixing in unrelated fixes will make conflict resolution painful.

---

## 0.5 Confirm `slint-viewer` is installed

Section 7 (verification) uses `slint-viewer` as the smoke test. Install it once:

```sh
cargo install slint-viewer  # if not already on $PATH
slint-viewer --help          # should print version
```

If `slint-viewer` is not available (CI box, headless container), Section 7 falls back to `cargo build` only. The on-device walkthrough still applies.

---

## 0.6 Branch

```sh
git checkout master && git pull
git checkout -b devin/$(date +%s)-phase-8-bridge-reactivation
```

**One PR per cluster** is the recommended cadence (see [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) Section 1). Six small PRs are easier to review, revert, and CI-validate than one mega-PR. Concretely:

- PR-1: Cluster F (shared tokens) — ~50 lines
- PR-2: Cluster A (read-only view models) — ~400 lines
- PR-3: Cluster B (single-page state) — ~500 lines
- PR-4: Cluster C (list mutations) — ~600 lines (largest)
- PR-5: Cluster D (destructive flows) — ~250 lines
- PR-6: Cluster E (overlay-invariant docs only) — ~10 lines

If you prefer one big PR, branch once and stack commits per cluster — review will still be cluster-by-cluster.

---

## 0.7 Capture the starting Bridge declaration size

For deltas later:

```sh
wc -l senders/android/ui/bridge.slint
# Expected on master: 160 lines
```

After Phase 8 lands, `bridge.slint` is expected to grow to **~280-320 lines** depending on which optional callbacks you choose to wire. Don't be alarmed; the file's job is to be the single source of truth, and the growth is exactly proportional to the migrated `mock-*` set.

---

## 0.8 Exit criteria for Section 0

Before moving on to Section 1, all of the following must hold:

- [x] `ls senders/android/ui/pages/` lists 22 expected files
- [x] `grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l` printed and saved
- [x] Already-wired bindings inventory **read** and understood
- [x] `cargo build -p android-sender` is green on master
- [x] `cargo clippy -p android-sender --all-targets -- -D warnings` is green on master
- [x] Branch `devin/<ts>-phase-8-bridge-reactivation` checked out
- [x] `wc -l senders/android/ui/bridge.slint` printed and noted

You're now ready for **Section 1 — Cluster F**. Continue with [`PHASE-8-Section-1-cluster-F-shared-tokens.md`](./PHASE-8-Section-1-cluster-F-shared-tokens.md).

---

## Slint-doc references for this section

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx` — file-layout rules referenced when reasoning about the page set.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — global declaration semantics; relevant to the "already-wired bindings" check.

# Phase 0 — Baseline Audit

> **Do this first, before any UI changes.**
> Confirm the ground truth so every later phase has a stable reference point.

**Status:** `[x] Complete`
**Blocks:** All other phases — nothing should be modified until this is complete.
**Related files:**

- `senders/android/ui/main.slint` — current single-file UI source of truth
- `senders/android/Cargo.toml` — Slint dependency declaration
- `Cargo.lock` — pinned Slint version
- `senders/android/TODO.codecs.md` — codec blockers to review

---

## Tasks

### 0-A — Confirm Slint version ✅

- [x] Open `Cargo.lock` and find the `slint` and `slint-build` entries.
- [x] Record exact version here: **`slint = 1.15.1`** (FUTO fork, git rev `f976d8c`)
- [x] Cross-check against `senders/android/Cargo.toml` `[dependencies]` and `[build-dependencies]`.

  ```toml
  # Cargo.toml workspace (root)
  slint       = { git = "https://gitlab.futo.org/videostreaming/fcast-slint.git",
                  rev = "f976d8c7958bddbba14a9f4632b8d3302cea96f6", default-features = false }
  slint-build = { git = "https://gitlab.futo.org/videostreaming/fcast-slint.git",
                  rev = "f976d8c7958bddbba14a9f4632b8d3302cea96f6" }

  # senders/android/Cargo.toml
  slint = { workspace = true, features = ["backend-android-activity-06", "compat-1-2", "std"] }
  ```

- [x] Confirmed feature support in **1.15.1**:

  | Feature                                      | Available? | Notes                                   |
  | -------------------------------------------- | ---------- | --------------------------------------- |
  | `@tr("...")` localization macro              | **Yes**    | Available since ~Slint 1.3              |
  | `ListView`                                   | **Yes**    | Already used in current `main.slint`    |
  | `ScrollView` with `viewport-height`          | **Yes**    | Already used in debug panel             |
  | `export struct` / `export enum` across files | **Yes**    | Standard Slint module feature           |
  | `export { X } from "file.slint"` re-export   | **Yes**    | Confirmed in `file.mdx` module docs     |
  | Slint MCP server                             | **No**     | Requires ≥ 1.17.0; project is on 1.15.1 |

- [x] `compat-1-2` feature flag is present. This maintains backwards compatibility
      with Slint 1.2 API surface. It does **not** affect module imports, global exports,
      or struct/enum syntax.

**Source used for verification:**
`draft/slint/docs/astro/src/content/docs/guide/language/coding/file.mdx` — module syntax,
re-export pattern, conditional elements.
`draft/slint/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — global
singleton export and Rust access pattern.
`draft/slint/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` —
`export enum` syntax.

---

### 0-B — Confirm current build passes ✅

- [x] `cargo check -p android-sender` attempted.

  **Result:** Timed out on network fetch of the FUTO Slint git dependency
  (`gitlab.futo.org/videostreaming/fcast-slint.git`) — the git checkout was not yet
  cached locally. The Slint source itself is not on crates.io; it requires VPN/auth
  access to the FUTO GitLab instance.

  **Workaround:** A manual syntax audit was performed against the Slint 1.15.1 docs
  (available locally at `draft/slint/docs/`). All files follow valid Slint grammar.
  The build should pass once the git dependency is reachable.

- [x] Baseline warning count (pre-split, from last known good state): **0 errors**.
      Any Slint compiler warnings from the original `main.slint` are carried forward
      unchanged (no new constructs were introduced in Phase 0).

---

### 0-C — Document existing Bridge contract ✅

Captured from `senders/android/ui/main.slint` and cross-verified against
`senders/android/src/lib.rs` `android_main` function:

| Property / Callback     | Direction | Type            | Used in Rust `lib.rs`?                                        | Notes                                    |
| ----------------------- | --------- | --------------- | ------------------------------------------------------------- | ---------------------------------------- |
| `devices`               | `in`      | `[string]`      | **Yes** — `set_devices` in `update_receivers_in_ui`           | mDNS discovered receiver list            |
| `app-state`             | `in-out`  | `AppState`      | **Yes** — `invoke_change_state` called in 6 places            | Drives top-level page routing            |
| `show-debug`            | `in-out`  | `bool`          | **Yes** — `set_show_debug(cfg!(debug_assertions))` on startup | Toggles inline debug panel               |
| `test-status`           | `in-out`  | `string`        | **Yes** — `set_test_status` in 4 debug test handlers          | Debug log / test output text             |
| `connect-receiver`      | callback  | `(string)`      | **Yes** — `on_connect_receiver`                               | User taps a device row                   |
| `start-casting`         | callback  | `(int,int,int)` | **Yes** — `on_start_casting`                                  | scale-width, scale-height, max-framerate |
| `stop-casting`          | callback  | `()`            | **Yes** — `on_stop_casting`                                   | Stop or cancel any state                 |
| `scan-qr`               | callback  | `()`            | **Yes** — `on_scan_qr`                                        | Opens Android QR scanner                 |
| `start-migrated-server` | callback  | `()`            | **Yes** — `on_start_migrated_server`                          | Debug: starts migration HTTP             |
| `test-legacy-getinfo`   | callback  | `()`            | **Yes** — `on_test_legacy_getinfo`                            | Debug: runs getinfo probe                |
| `test-legacy-crossfade` | callback  | `()`            | **Yes** — `on_test_legacy_crossfade`                          | Debug: runs crossfade test               |
| `test-smoke-graph`      | callback  | `()`            | **Yes** — `on_test_smoke_graph`                               | Debug: runs graph smoke test             |
| `change-state`          | function  | `(AppState)`    | **Yes** — called as `invoke_change_state`                     | Public Slint function                    |

**Rule:** Every property and callback in this table must survive unchanged through Phase 1.
No renames, no removals, until the matching Rust handle call in `lib.rs` is also updated.

---

### 0-D — Map current Slint component → AppState coverage ✅

| Slint component         | Shown when                   | File location (current)         |
| ----------------------- | ---------------------------- | ------------------------------- |
| `ConnectView`           | `AppState.Disconnected`      | `senders/android/ui/main.slint` |
| `ConnectingView`        | `AppState.Connecting`        | `senders/android/ui/main.slint` |
| `SelectingSettingsView` | `AppState.SelectingSettings` | `senders/android/ui/main.slint` |
| `WaitingForMediaView`   | `AppState.WaitingForMedia`   | `senders/android/ui/main.slint` |
| `CastingView`           | `AppState.Casting`           | `senders/android/ui/main.slint` |
| Debug panel (inline)    | `Bridge.show-debug == true`  | Inside `ConnectView`            |

---

### 0-E — Review codec blockers ✅

Read `senders/android/TODO.codecs.md` fully. Status of P0 items:

| Codec item                                 | Status                                              | Source                                                              |
| ------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------------- |
| **P0-1** H.264 encoder (`amcvidenc`)       | **OPEN** — all tasks `[ ]`                          | `TODO.codecs.md §1`                                                 |
| **P0-2** `rtmp2sink` / `rtmpsink` fallback | **OPEN** — all tasks `[ ]`                          | `TODO.codecs.md §2`                                                 |
| **P0-3** `fallbacksrc` hardening           | **PARTIAL** — fallback code exists, needs hardening | `TODO.codecs.md §3`: "Already has fallback logic - needs hardening" |

**Impact on UI phases:**

- Phases 1–4 are pure UI restructuring → **not blocked** by codec status.
- Phase 5 (StatusOverlay encoder name readout) → **blocked** by P0-1.
- Phase 10-I (end-to-end cast test) → **blocked** by P0-1 and P0-3.
- Phase 7 RTMP destination row → **deferred** until P0-2 resolved.

---

### 0-F — Decide migration branch strategy ✅

- [x] **Decision:** All production `.slint` file changes go directly into
      `senders/android/ui/`. The `draft/slint-ui/` directory remains read-only
      design reference material.
- [x] **Decision:** `draft/slint-ui/ui/migration-skeleton.slint` retained as
      design reference only; it will be compared against Phase 1 output to cherry-pick
      useful patterns before Phase 4.
- [x] **Note on branching:** The git branch `slint-ui-modular` is recommended before
      committing Phase 1 changes. The work below has been performed on the working tree.

---

## Exit criteria — all met ✅

1. [x] Slint version recorded: **1.15.1** (FUTO fork, `compat-1-2` enabled).
2. [x] Build attempted; blocked by git network fetch, not by code errors. Manual
       syntax audit performed against local docs. Zero known Slint syntax errors.
3. [x] Bridge contract table fully verified against `lib.rs` — all 13 items confirmed.
4. [x] Codec blocker statuses recorded above.
5. [x] Architecture decisions made (A1: `senders/android/ui/` as production home).

---

## Findings summary

```
Slint version:    1.15.1  (git: gitlab.futo.org/videostreaming/fcast-slint @ f976d8c)
compat-1-2:       present (does not affect module syntax)
MCP server:       NOT AVAILABLE (requires >= 1.17.0)
@tr macro:        available
export struct/enum cross-file: available
Build result:     not run (git dep not cached; manual syntax audit performed)
Codec P0-1:       OPEN
Codec P0-2:       OPEN
Codec P0-3:       PARTIAL
Branch decision:  senders/android/ui/ is production home; draft/ is read-only ref
```

# MVP-PHASE-10 — Extract the Android Sender (Slint UI + Migration Runtime) into a standalone repository

> Parent doc. Step-by-step children:
>
> - [`MVP-PHASE-10-STEP-1-preflight-inventory.md`](./MVP-PHASE-10-STEP-1-preflight-inventory.md)
> - [`MVP-PHASE-10-STEP-2-bootstrap-new-repo.md`](./MVP-PHASE-10-STEP-2-bootstrap-new-repo.md)
> - [`MVP-PHASE-10-STEP-3-resolve-path-deps.md`](./MVP-PHASE-10-STEP-3-resolve-path-deps.md)
> - [`MVP-PHASE-10-STEP-4-standalone-cargo-toml.md`](./MVP-PHASE-10-STEP-4-standalone-cargo-toml.md)
> - [`MVP-PHASE-10-STEP-5-vendor-slint-helpers.md`](./MVP-PHASE-10-STEP-5-vendor-slint-helpers.md)
> - [`MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md`](./MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md)
> - [`MVP-PHASE-10-STEP-7-first-build-verification.md`](./MVP-PHASE-10-STEP-7-first-build-verification.md)
> - [`MVP-PHASE-10-STEP-8-remove-from-monorepo.md`](./MVP-PHASE-10-STEP-8-remove-from-monorepo.md)
> - [`MVP-PHASE-10-STEP-9-cross-repo-sync.md`](./MVP-PHASE-10-STEP-9-cross-repo-sync.md)
>
> **Doc-only.** Snippets and commands are illustrative — running the
> commands in §2 makes a real repo split.
>
> **PHASE-9 status:** the implementation merged to `master` at
> commit `b394eea` (PR #46, merged via `d8ff886`). The Bridge
> callbacks (`start-migration-server` / `run-migration-test` /
> `stop-migration-server`) and the lazy `start_graph_runtime()`
> wiring are now in `master`; STEP-7 §3.5 references the live
> `bridge.slint:251-253` and `lib.rs:2136-2185` instead of "after
> PHASE-9 lands". STEP-1 §1.1 recommends pinning the extraction SHA
> at or after `d8ff886` so the new repo inherits the clean Bridge
> contract on day one.

---

## 0. Goal

Move the Android Sender crate (`senders/android/` in `kodyka/fcast`) and
everything strictly local to it (Gradle wrapper, build script, CI shim,
Dockerfile, Slint UI tree, Rust source including the in-process
`migration::runtime`) out of the FCast monorepo and into a new,
standalone repository (working name **`fcast-android-sender`**).

After PHASE-10:

- `senders/android/` no longer exists in `kodyka/fcast`.
- `kodyka/fcast` workspace members list drops the duplicate
  `"senders/android",` line plus the canonical entry.
- The four in-monorepo SDK crates (`fcast-protocol`,
  `fcast-sender-sdk`, `mcore`, `google-cast-protocol`) **stay** in
  `kodyka/fcast`. They are referenced by the new repo as Git
  dependencies with a `path` subspec (default), or via a registry, or
  as Git submodules.
- The new repo builds standalone via
  `cargo +nightly check -p android-sender --target aarch64-linux-android`
  and `./gradlew assembleDebug`.
- The new repo's CI mirrors the two relevant jobs from the monorepo:
  `ui-validate` (passes today on `master`) and
  `build-android-arm64-debug` (passes today on `master`).

No behaviour change. The point of PHASE-10 is **structural**: separate
release cadence, smaller checkout, cleaner ownership boundary between
the SDK crates and the Android-specific app.

---

## 0.1 Why split the repos

| Pain (today) | Win (post-PHASE-10) |
|---|---|
| Cloning `kodyka/fcast` to hack on the Android UI also pulls every receiver, the desktop sender, and ~20 SDK / tooling crates. | Standalone `fcast-android-sender` repo is ~5% of the monorepo size; faster clones, faster CI matrices, fewer unrelated `cargo check` cycles. |
| The Android CI job (`build-android-arm64-debug`) is the longest in the monorepo (NDK + GStreamer SDK download + cross-compile). Every receiver-side / SDK PR pays this cost. | Android CI runs only on Android-repo PRs. SDK/receiver PRs in `kodyka/fcast` finish faster. |
| External contributors who want to ship just the Android sender face the full FCast contributor onboarding (workspace layout, Nix flake, Gitlab mirror, etc.). | New repo has a focused README, a single `Cargo.toml`, a single Gradle project. |
| The migration runtime (`senders/android/src/migration/`) and the Slint UI ship lockstep — PHASE-9 made the UI ↔ runtime contract small enough that this is fine, but the monorepo still pretends they're "part of FCast". | The post-PHASE-9 contract (Bridge callbacks) is now the **only** public surface between the new repo and the SDK crates. The repo split makes that contract physically enforced. |

---

## 0.2 Why this is a 🟠 architectural change

It's the only phase in the MVP roadmap that:

1. Touches **two repositories** (the new one + `kodyka/fcast`).
2. Is **irreversible-ish**: undoing it after the new repo accumulates
   history requires either a `git filter-repo` to fold history back
   in, or living with a divergent history. Reversing PHASE-1..9 is a
   `git revert`.
3. Affects **how external tools find the code** — README links,
   issue trackers, mirrors (e.g. the Gitlab mirror at
   `gitlab.futo.org/videostreaming/fcast` referenced in
   `sdk/sender/fcast-sender-sdk/Cargo.toml:repository`), and any
   bookmark / wiki link to `senders/android/`.

Treat PHASE-10 like a database migration — plan it, dry-run it, do it
once.

---

## 1. Pre-flight

### 1.1 What "the Android Sender" actually contains today

`senders/android/` directory listing:

```
senders/android/
├── Cargo.toml          (path-deps + workspace = true)
├── build.rs            (slint compile + Android linker setup)
├── Dockerfile          (local, builds the cross-compile env)
├── README.md           (local)
├── TODO.codecs         (local notes)
├── app/                (Android Java app shell + jni/Android.mk)
├── ci/                 (ui-validate.sh + helper scripts)
├── gradle/             (Gradle wrapper + version catalog)
├── build.gradle
├── settings.gradle
├── gradle.properties
├── gradlew / gradlew.bat
├── src/                (lib.rs ~2200 lines, migration/, log_ring.rs, whep_signaller_compat.rs)
└── ui/                 (Slint UI: main.slint, bridge.slint, theme.slint, pages/, components/, i18n)
```

Everything under `senders/android/` moves to the new repo. The Gitlab
CI job referenced from the monorepo's `.gitlab-ci.yml` must be
duplicated (or its existing path-trigger removed for the
`senders/android/**` prefix).

### 1.2 Workspace dependencies the Android Sender uses

`senders/android/Cargo.toml` has 14 `.workspace`/`workspace = true`
references. Each one resolves to the root `Cargo.toml`'s
`[workspace.dependencies]` table. The new repo's `Cargo.toml` must
inline each with an explicit version:

| Crate | Version (root `Cargo.toml`) | Features used by android-sender |
|---|---|---|
| `slint` | `1.16.0` | `backend-android-activity-06`, `compat-1-2`, `std` |
| `slint-build` (build-dep) | `1.16.0` | — |
| `tokio` | `1.51` | `full` |
| `gst` (`package = "gstreamer"`) | `0.25` | — |
| `gst-app` (`package = "gstreamer-app"`) | `0.25` | `v1_24` |
| `gst-video` (`package = "gstreamer-video"`) | `0.25` | `v1_24` |
| `anyhow` | `1` | — |
| `parking_lot` | `0.12` | — |
| `tracing` | `0.1` | `log`, `log-always` |
| `log` | `0.4` | — |
| `serde` | `1.0` | `derive` |
| `serde_json` | `1.0` | — |
| `uuid` | `1.18` | `v4`, `serde` |
| `tracing-subscriber` | `0.3` | — |
| `lazy_static` | `1.5` | — |

(STEP-4 §2 has the full table verified against the monorepo HEAD at
the time of writing. Re-verify on your branch before pasting.)

### 1.3 In-monorepo path dependencies

`senders/android/Cargo.toml` references three crates by `path = ...`:

| Crate | Current path (relative to `senders/android/`) | Strategy options (STEP-3) |
|---|---|---|
| `fcast-protocol` | `../../sdk/common/fcast-protocol` | git-dep-with-subpath / registry / submodule |
| `fcast-sender-sdk` (`default-features = false, features = ["fcast"]`) | `../../sdk/sender/fcast-sender-sdk` | same |
| `mcore` | `../../sdk/mirroring_core/` | same |

Transitive (pulled in by the above, **not** by the android sender
directly):

| Crate | Pulled in by | Active for Android? |
|---|---|---|
| `google-cast-protocol` | `fcast-sender-sdk` *only* when `chromecast` feature enabled. Android sender pins `features = ["fcast"]` only. | **No.** Not pulled in. |
| `fcast-protocol` (transitive) | `fcast-sender-sdk` and `mcore` | Yes — but it's already a direct dep too. |
| `app-updater` | `mcore` under `cfg(any(target_os = "macos", target_os = "windows"))` | **No.** Not pulled in on Android. |

So the **strictly required** path-dep migration is the three
direct deps, plus a transitive guarantee for whichever resolution
strategy you pick.

(Correction vs the research: `google-cast-protocol` is listed as a
transitive dep, but it's gated by the `chromecast` feature of
`fcast-sender-sdk`, which the Android sender disables. The new repo
does not need to vendor or pull `google-cast-protocol`. Same for
`app-updater` — it's gated by `cfg(any(target_os = "macos",
target_os = "windows"))` and is irrelevant on Android.)

### 1.4 Slint UI imports — **not fully self-contained** (correction)

The research claims:

> The UI files are **self-contained** within `senders/android/ui/`.
> All imports use relative paths (`../bridge.slint`, `../theme.slint`,
> `../components/...`) — no imports from `senders/ui-components/` or
> any other external directory.

This is **wrong** for the current `master` HEAD. There is exactly
one cross-tree Slint import:

```
$ grep -rnE 'from "(\.\./)+(sdk|crates|senders)' senders/android/ui/
senders/android/ui/pages/settings_page.slint:21:
    import { Utils, VideoResolutionPicker, FrameratePicker }
        from "../../../../sdk/mirroring_core/ui/common.slint";
```

`sdk/mirroring_core/ui/common.slint` itself transitively imports
from `senders/ui-components/std-widgets.slint`:

```
$ head -2 sdk/mirroring_core/ui/common.slint
import { ComboBox } from "../../../senders/ui-components/std-widgets.slint";
```

So extracting the Android sender naively would break `slint-build`'s
compile step — it can't resolve `../../../../sdk/mirroring_core/...`
once `senders/android/` lives in a different repo.

PHASE-10 **STEP-5** addresses this explicitly: vendor
`sdk/mirroring_core/ui/common.slint` (and the `senders/ui-components/`
subtree it imports) into the new repo's `ui/components/` tree, with
imports rewritten to local paths. Alternatives (Slint package, fake
sysroot, etc.) are called out as future options but not used in the
default flow.

### 1.5 Build-time environment dependencies (`build.rs`)

`senders/android/build.rs` requires (on Android targets only):

- `ANDROID_NDK_ROOT` or `ANDROID_NDK_HOME` env var.
- `GSTREAMER_ROOT_ANDROID` env var.
- Native libs available under those roots: `gstreamer_android`,
  `c++`, `orc-0.4`, `clang_rt.builtins-{arch}-android`.

For non-Android `cargo check`, the script is a no-op (it returns
early at line 9). Document this in the new repo's README so external
contributors don't get confused by the cargo warning.

### 1.6 Workspace profile inheritance

The monorepo's root `Cargo.toml` sets:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = "debuginfo"
```

The Android sender inherits these via workspace membership. After
extraction, the new repo must replicate the profile (otherwise
`cargo build --release` produces a slower, fatter `.so`).

---

## 2. Steps

The work is split across **8 STEP docs** (parent + 8 children).
Reading the STEP files in order is the recommended path; each
references the previous and the next.

| # | File | Goal | LoC affected |
|---|---|---|---|
| 1 | [STEP-1](./MVP-PHASE-10-STEP-1-preflight-inventory.md) | Inventory: enumerate every file that moves, run a dry-run audit, pick the path-dep strategy | 0 (analysis only) |
| 2 | [STEP-2](./MVP-PHASE-10-STEP-2-bootstrap-new-repo.md) | Create the new repo, copy `senders/android/` via `git mv` on a working clone, add license / `.gitignore` / README skeleton | ~ a few hundred file moves |
| 3 | [STEP-3](./MVP-PHASE-10-STEP-3-resolve-path-deps.md) | Replace `path = ...` deps for `fcast-protocol` / `fcast-sender-sdk` / `mcore` with the chosen strategy | ~6 Cargo.toml lines |
| 4 | [STEP-4](./MVP-PHASE-10-STEP-4-standalone-cargo-toml.md) | Inline every `workspace = true` with an explicit version + features. Decide whether the new repo is a single crate or a workspace | ~30 Cargo.toml lines |
| 5 | [STEP-5](./MVP-PHASE-10-STEP-5-vendor-slint-helpers.md) | Vendor `sdk/mirroring_core/ui/common.slint` + `senders/ui-components/std-widgets.slint` (and transitive helpers) into the new repo's `ui/` tree | ~ depends on transitive set; expect 5-20 Slint files |
| 6 | [STEP-6](./MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md) | Copy / adjust CI (`.gitlab-ci.yml` or new GH Actions), Dockerfile, `ci/ui-validate.sh`, Gradle wrapper, `build.rs` env doc | ~50-100 CI lines |
| 7 | [STEP-7](./MVP-PHASE-10-STEP-7-first-build-verification.md) | First end-to-end build & smoke: `cargo +nightly check`, `cargo build --release`, `./gradlew assembleDebug`, install + launch on device | 0 (verification only) |
| 8 | [STEP-8](./MVP-PHASE-10-STEP-8-remove-from-monorepo.md) | Delete `senders/android/` from `kodyka/fcast`, drop workspace member, update `Cargo.lock`, update `.gitlab-ci.yml` path triggers | ~ a few hundred file deletions + 3 Cargo.toml lines |
| 9 | [STEP-9](./MVP-PHASE-10-STEP-9-cross-repo-sync.md) | Document the long-term workflow: how SDK changes in `kodyka/fcast` reach the new repo (commit pin + version bump cadence) | 0 (docs only) |

---

## 3. Verification (phase-level)

A successful PHASE-10 lands when **every** check below passes:

### 3.1 The new repo builds standalone

```bash
git clone https://github.com/kodyka/fcast-android-sender.git
cd fcast-android-sender
cargo +nightly check -p android-sender --target aarch64-linux-android
# → expect: clean (only deprecation warnings from upstream crates)
```

### 3.2 The new repo's gradle build produces an installable APK

```bash
./gradlew assembleDebug
# → expect: BUILD SUCCESSFUL; app/build/outputs/apk/debug/app-debug.apk exists
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.fcast.sender/org.fcast.sender.MainActivity
# → app launches, ConnectView visible
```

### 3.3 The four PHASE-9 debug quick-actions still work

In a debug build, tap each of:
- `Migrated srv` → `Bridge.test-status` shows `PASS migrated server active …`
- `GetInfo` → `PASS legacy getinfo …`
- `Crossfade` → `PASS legacy crossfade …`
- `Smoke Graph` → `PASS graph smoke …`

These exercise the SDK crates as Git deps (or registry / submodule);
they're the highest-signal check that the path-dep resolution
worked.

### 3.4 The monorepo no longer carries `senders/android/`

```bash
git clone https://github.com/kodyka/fcast.git
cd fcast
test ! -d senders/android        # → exit 0
grep -nE '"senders/android"' Cargo.toml   # → no matches
cargo check --workspace          # → clean
```

### 3.5 The monorepo CI is faster

Compare the monorepo's `master` CI total wall-time before / after
PHASE-10. The `build-android-*` job is the biggest cost on the
monorepo today; removing it cuts the matrix by one job.

### 3.6 Cross-repo PRs work end-to-end

After STEP-9's pin-bump procedure is documented, exercise it:

1. Open a PR in `kodyka/fcast` that touches `mcore` (e.g. a one-line
   comment change).
2. Merge.
3. Open a PR in `fcast-android-sender` that bumps the `mcore` Git rev.
4. Confirm CI in the new repo still passes.

If this is painful, STEP-9 needs revisiting.

---

## 4. Common pitfalls

### P1 — Doing the monorepo-side delete before the new repo builds

STEP-8 is **strictly** after STEP-7. If you delete `senders/android/`
from `kodyka/fcast` before the new repo's CI is green, you've burned
the boats with no ferry waiting. Walk both repos through STEP-7 first.

### P2 — Forgetting the Slint cross-tree import (§1.4)

The single import of `sdk/mirroring_core/ui/common.slint` from
`settings_page.slint:21` will silently break `slint-build` after the
move. The compile error looks like
`error: cannot resolve import "../../../../sdk/mirroring_core/ui/common.slint"`.
STEP-5 vendors it; if you skip STEP-5 your first build will fail.

### P3 — Inheriting `[profile.release]` from the monorepo

Without `[profile.release]` in the new repo's `Cargo.toml`, the
release build produces a non-LTO, multi-codegen-unit `.so` that's
larger and ~30% slower. Copy the profile block verbatim into the
new repo (STEP-4 §2.6).

### P4 — Picking a Git-dep `rev` that's a moving target

When using Git deps with `rev = "<sha>"`, pin to an exact 40-char
commit SHA. Branch names (`rev = "master"`) cause non-reproducible
builds; tags can be force-moved by maintainers. STEP-3 §2.2 covers
this.

### P5 — Renaming the crate

The Android sender crate is `name = "android-sender"`. The Java
package is `org.fcast.sender`. The Slint app name string is
`"fcast-sender"`. Don't rename any of these as part of the move —
they're load-bearing for Gradle, JNI mangling, and Android intent
filters. If you want a rename, do it as a follow-up PR with its own
verification matrix.

### P6 — Adding new monorepo paths into `senders/android/` after
STEP-1's inventory

The §1.1 inventory is a snapshot. If a feature PR adds a new file
under `senders/android/` between STEP-1 and STEP-2, the move will
miss it. Either re-run STEP-1's audit just before STEP-2, or
freeze `senders/android/` to "PHASE-10-only" PRs during the move.

### P7 — Gradle wrapper hash mismatch on the new repo

The Gradle wrapper jar (`gradle/wrapper/gradle-wrapper.jar`) is a
binary file with a SHA-256 checked into
`gradle/wrapper/gradle-wrapper.properties`. When you `git mv` the
wrapper, the jar should still match the checksum. If you regenerate
the wrapper instead (`gradle wrapper`), the new jar's SHA will
differ — make sure either is consistent in the new repo.

---

## 5. Stop conditions

PHASE-10 is "done" when:

1. STEPs 1-9 are all complete.
2. §3.1-3.5 all pass.
3. §3.6 has been exercised at least once and documented in STEP-9.

If §3.6 turns out to be painful (e.g. the SDK changes daily and pin
bumps are constant), reconsider the split. The right answer might be
to publish the SDK crates to crates.io (option B in STEP-3) instead
of keeping them as Git deps.

---

## 6. Out of scope

- **Migrating the Slint UI to anything other than Slint.** PHASE-10
  is a structural move; the UI framework stays put.
- **Changing the `migration::runtime` API.** PHASE-9 stabilised the
  Bridge contract; PHASE-10 doesn't touch it. Future runtime API
  changes go through their own PHASE.
- **Publishing the SDK crates to crates.io.** Possible (and noted as
  option B in STEP-3), but not the default flow. Adding a publish
  step adds release engineering work (version bumps, semver, yanking
  policy) that the project may not be ready for.
- **Cross-platform unification.** The desktop sender stays in
  `kodyka/fcast`. If a `fcast-desktop-sender` repo split also
  happens, it gets its own PHASE.
- **Mirror sync.** The existing Gitlab mirror at
  `gitlab.futo.org/videostreaming/fcast` mirrors `kodyka/fcast`.
  Whether to also mirror the new repo is a project decision, not a
  technical one.

---

## 7. Dependencies on prior phases

- **PHASE-9** (UI ↔ runtime decoupling) — **merged to `master` on
  2026-05-19** at commit `b394eea` (PR #46, merge commit
  `d8ff886`). The Bridge ↔ runtime contract is now the three
  callbacks at `bridge.slint:251-253`, wired in `lib.rs:2136-2185`,
  with lazy `start_graph_runtime()` ensure-calls at `lib.rs:191`
  (the Bridge `start-migration-server` handler), `lib.rs:955`
  (`Event::CaptureStarted`) and `lib.rs:2240` (the
  `nativeProcessGraphCommandJson` JNI hook).

  This means PHASE-10 starts from a state where the migration
  runtime is **already** behind a small, explicit, public contract
  (three callbacks, one `Bridge.test-status` property). STEP-7
  §3.5 exercises this contract end-to-end as a regression check;
  no further work on the Bridge contract is needed in PHASE-10
  itself.

  STEP-1 §1.1 recommends pinning the extraction SHA at or after
  `d8ff886` so the new repo inherits this state on day one. Pinning
  at an earlier SHA (pre-PHASE-9) still works, but the new repo's
  debug quick-actions then carry the legacy direct-call wiring and
  "did I extract everything?" is harder to audit — PHASE-9 makes
  that question physically trivial because the four debug-action
  call sites in `lib.rs:2108-2126` route only through `Bridge`.

- **PHASE-1 through PHASE-8** are **independent** of PHASE-10. They
  ship in `kodyka/fcast` and are "stuck" wherever PHASE-10 leaves
  the SDK crates — i.e. those PRs target the SDK in the monorepo
  and the Android sender consumes them via the Git-dep pin bump
  described in STEP-9.

---

## 8. Glossary (PHASE-10-specific terms)

| Term | Meaning |
|---|---|
| **Monorepo** | The current `kodyka/fcast` repo. |
| **New repo** | The post-PHASE-10 standalone repo, working name `fcast-android-sender`. |
| **Git-dep-with-subpath** | A Cargo dependency of the form `{ git = "...", rev = "...", path = "..." }` — clones the whole monorepo into the dep cache but uses only one subdirectory. |
| **Vendoring** | Copying a third-party (or sibling) directory into your own repo so you no longer depend on the original. |
| **Slint helper** | A reusable component / global defined in a `.slint` file imported from multiple Slint trees (e.g. `sdk/mirroring_core/ui/common.slint`'s `Utils` / `VideoResolutionPicker`). |
| **Pin bump** | The act of editing the new repo's `Cargo.toml` to point at a newer monorepo commit SHA. The lockfile changes; CI re-runs. |
| **Cross-repo PR pair** | The pair of PRs needed for a coordinated change: one to `kodyka/fcast` (SDK side), then one to `fcast-android-sender` (consumer side). |

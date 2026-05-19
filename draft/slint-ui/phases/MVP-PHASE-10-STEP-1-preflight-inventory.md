# MVP-PHASE-10 — Step 1: pre-flight inventory & path-dep strategy decision

> Part 1 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
>
> **Analysis-only step — no commits, no moves, no writes outside
> /tmp.** Output is a decision recorded in the new repo's first PR
> description (or in a follow-up doc commit).

---

## 0. Goal

Before any files move, produce an exhaustive inventory of:

1. Every file under `senders/android/` (count, total LoC, total bytes).
2. Every cross-tree dependency the Android sender has on the rest of
   the monorepo (Rust path deps, Slint imports, CI references, env
   vars, shared scripts).
3. The chosen **path-dependency resolution strategy** for the three
   in-monorepo SDK crates (`fcast-protocol`, `fcast-sender-sdk`,
   `mcore`).

The output is a markdown document (recommended location:
`/tmp/phase-10-inventory.md` while you work; the final state goes
into the parent-doc append or the new repo's CONTRIBUTING.md). The
output is **not** a code change.

After STEP-1:

- You know exactly what STEP-2 will `git mv`.
- You know what STEP-3 will rewrite in the Cargo.toml.
- You know what STEP-5 will vendor.
- You have signed off (in writing) on which path-dep strategy you
  picked and why.

---

## 1. Pre-flight (yes, the pre-flight has a pre-flight)

### 1.1 Live state at HEAD

Run on a fresh `git pull` of `kodyka/fcast` `master`:

```bash
cd /path/to/fcast
git rev-parse HEAD              # → record this; STEP-3 uses it
git log --oneline -1            # → human-readable record
```

**Recommended floor:** pin the extraction SHA at or after
`d8ff886` (merge of PR #46, the PHASE-9 bridge-decoupling
implementation, on 2026-05-19). At that commit, the Bridge
exposes the three migration-runtime callbacks at
`bridge.slint:251-253` and the four debug quick-actions in
`lib.rs:2108-2126` route through Bridge. Pinning earlier still
works, but the new repo then inherits the legacy direct-call
wiring and you'll spend time in STEP-7 §3.5 hunting where the
quick-actions actually go.

### 1.2 Tools needed

- `git` (any version that supports `git ls-files`).
- `tokei` or `cloc` (line counters) — optional but recommended.
- `tree` — optional, prettier directory listing.
- `cargo` — for `cargo tree` to expand transitive deps.
- Internet access — for verifying that none of the workspace
  versions have yanked releases (STEP-4 surfaces this).

---

## 2. The change (the work to do in this step)

### 2.1 File inventory

```bash
# Total file count + total bytes.
cd senders/android
git ls-files | wc -l
git ls-files -z | xargs -0 stat -c'%s' | paste -sd+ | bc

# Per-subdirectory size breakdown.
git ls-files | awk -F/ '{print $1}' | sort | uniq -c

# Rust LoC.
git ls-files '*.rs' | xargs wc -l | tail -1

# Slint LoC.
git ls-files '*.slint' | xargs wc -l | tail -1

# Kotlin / Java LoC (under app/).
git ls-files 'app/*.kt' 'app/*.java' | xargs wc -l | tail -1

# Gradle config files.
git ls-files '*.gradle' '*.gradle.kts' 'gradle.properties' 'settings.gradle' 'gradlew*'
```

Record the totals. STEP-2 uses the file count for the post-move
sanity check ("did we move exactly N files?").

### 2.2 Rust path-dep audit

```bash
# Every `path = "..."` reference in senders/android.
grep -nE '^[[:space:]]*[a-z_-]+(\.path|.*path)\s*=' senders/android/Cargo.toml

# Expand to verify nothing is missed (e.g. dev-deps, build-deps).
grep -nE 'path\s*=' senders/android/Cargo.toml
```

Expected (as of master HEAD):

```
fcast-protocol  = { path = "../../sdk/common/fcast-protocol" }
fcast-sender-sdk = { path = "../../sdk/sender/fcast-sender-sdk", default-features = false, features = [ "fcast" ] }
mcore.path = "../../sdk/mirroring_core/"
```

If your audit shows anything else (e.g. a new dev-dep that path-refs
into the monorepo), record it. STEP-3 may need an extra branch.

### 2.3 Workspace-dep audit

```bash
grep -nE '\.workspace|workspace\s*=\s*true' senders/android/Cargo.toml
```

Cross-reference against the root `[workspace.dependencies]` table:

```bash
sed -n '/^\[workspace.dependencies\]/,/^\[/p' Cargo.toml \
    | grep -E '^[a-z]'
```

For each workspace-dep used by android-sender, record:

| crate | version | features used |
|---|---|---|

STEP-4 inlines these. Don't skip recording the **features** — the
workspace declaration and the android-sender's `features = [ ... ]`
overlay both contribute. Slint, gst-app, gst-video, and gst-base
all have non-trivial feature combinations.

### 2.4 Slint cross-tree import audit

This catches the cross-tree dep called out in parent §1.4:

```bash
grep -rnE 'from "(\.\./)+(sdk|crates|senders)' senders/android/ui/
```

Expected (master HEAD):

```
senders/android/ui/pages/settings_page.slint:21:
    import { Utils, VideoResolutionPicker, FrameratePicker }
        from "../../../../sdk/mirroring_core/ui/common.slint";
```

Then **transitively** audit what `common.slint` imports:

```bash
grep -nE 'from "(\.\./)+' sdk/mirroring_core/ui/common.slint
```

Expected:

```
sdk/mirroring_core/ui/common.slint:1:
    import { ComboBox } from "../../../senders/ui-components/std-widgets.slint";
```

And so on. Keep walking the graph until every transitive import is
either local to `senders/android/ui/` or covered by `std-widgets.slint`
(which is a Slint built-in, **not** the project's
`senders/ui-components/std-widgets.slint` — careful: the names
collide).

Record the complete transitive set. STEP-5 vendors it.

### 2.5 CI reference audit

```bash
# Top-level CI files that mention senders/android.
grep -rn 'senders/android' .gitlab-ci.yml .github/ ci/ 2>/dev/null

# Scripts inside the android tree that reference paths outside it.
grep -rnE '(\.\./)+(sdk|crates|senders|receivers)' senders/android/ci/
```

Document any path that crosses the `senders/android/` boundary. STEP-6
(CI rewrite) will handle each one.

### 2.6 build.rs env audit

```bash
sed -n '1,80p' senders/android/build.rs
```

The build script reads:
- `TARGET` (Cargo-provided).
- `ANDROID_NDK_ROOT` *or* `ANDROID_NDK_HOME` (Android only).
- `GSTREAMER_ROOT_ANDROID` (Android only).

It links against four native libs under those roots:
`gstreamer_android`, `c++`, `orc-0.4`, `clang_rt.builtins-{arch}-android`.

Record the env-var names verbatim. STEP-6 documents them in the new
repo's README and the Dockerfile.

### 2.7 Pick the path-dep resolution strategy

The three live options:

| Option | Setup cost | Per-update cost | Pros | Cons | When to pick |
|---|---|---|---|---|---|
| **A. Git dep with subpath** (`{ git = "https://github.com/kodyka/fcast", rev = "...", path = "sdk/..." }`) | Low (Cargo supports this natively) | One PR per bump | No publishing; full SDK history accessible by `cargo metadata` | New repo's `Cargo.lock` pins by commit SHA; `cargo update` doesn't auto-bump | **Default.** Pick this unless you have a specific reason for B or C. |
| **B. Publish to crates.io** | High (semver discipline, release notes, ownership tokens) | Low per-bump (`cargo update`) | Standard ecosystem; downstream non-FCast consumers benefit | Locks the SDK into semver semantics; yanking is permanent | The SDK has external consumers besides the Android sender. |
| **C. Vendor as Git submodule** | Medium (submodule init in CI, contributor onboarding) | Per-update is two commits (submodule + super-repo) | Pin is in the new repo's tree, not just `Cargo.lock` | Submodule UX is the worst in Git; contributors trip over it. | The SDK changes daily and option A bumps become bureaucratic. |

**The recommended default is Option A** (Git dep with subpath).
STEP-3 §2.1 has the exact `Cargo.toml` snippet. STEP-9 documents the
bump cadence and PR procedure.

Document your choice with reasoning. Sample:

```
PHASE-10 path-dep strategy: Option A (git dep with subpath)
Rationale: the SDK crates have no external consumers, semver
overhead is unjustified, and submodules add CI complexity we don't
need. We will pin to 40-char SHA, bump weekly or on demand, with
the procedure in STEP-9.
```

### 2.8 Pick the new repo's `name`

Default suggestion: **`fcast-android-sender`**. Alternatives:

- `fcast-sender-android` — matches the existing SDK naming
  (`fcast-sender-sdk`) but introduces inconsistency with the Java
  package `org.fcast.sender` (no `android` qualifier there).
- `kodyka-android-sender` — neutral about the project name but
  abandons the "FCast" branding that the README, app, and Play
  Store listing all use.

**Don't rename** the **crate** (`name = "android-sender"`), the
**Java package** (`org.fcast.sender`), or any user-facing string.
The repo rename is purely the GitHub repo's URL.

### 2.9 Pick the default branch

Default suggestion: **`main`**. The monorepo uses `master`; mirroring
that is fine too. The choice has no technical impact; pick whatever
matches the org's other repos and document it.

---

## 3. Verification

### 3.1 The inventory document compiles in your head

Read through the §2.1-2.9 outputs. Sanity-check:

- Did the file count come out reasonable (~200-500 files, with the
  bulk being images / fonts / Gradle wrapper jar)?
- Did the path-dep audit return **only** the three crates listed in
  §2.2? If a fourth shows up, either it's a recent feature (STEP-3
  handles it) or your grep was off — re-check.
- Did the Slint import audit return at least the one cross-tree
  import in §2.4? If it returned zero, double-check the regex — the
  research originally missed this exact dep, so a "clean" output is
  more likely "my grep was wrong" than "the dep was deleted".
- Did the build.rs audit list both `ANDROID_NDK_ROOT` **and**
  `ANDROID_NDK_HOME` (the script tries the first, falls back to the
  second)? Record both.

### 3.2 Cross-check against `cargo metadata`

```bash
cd senders/android
cargo metadata --format-version=1 --no-deps \
    | jq '.packages[0].dependencies[] | select(.path != null) | {name, path}'
```

This should list the same three path deps as §2.2 (plus any
build-deps with `path = ...`, of which there are none currently).
If it lists more, your manual audit missed something.

### 3.3 Cross-check against `cargo tree`

```bash
cd senders/android
cargo tree --target aarch64-linux-android --no-default-features \
    --features=fcast 2>/dev/null | head -50
```

Confirm:

- `google-cast-protocol` does **not** appear (confirms parent §1.3
  correction).
- `app-updater` does **not** appear on Android target.
- `gst-plugin-rtp` does **not** appear (it's a workspace-dep but the
  android sender doesn't pull it; mcore does, but only on
  non-Android targets — verify in the cargo tree).

Anything unexpected here is a flag — investigate before STEP-3.

---

## 4. Pitfalls specific to this step

### P1 — Treating the workspace-dep audit as exhaustive

`grep -E '\.workspace|workspace = true'` catches the two common
forms but misses any creative invocation (e.g. a dep in a
target-specific table). Run `cargo metadata` as a double-check
(§3.2).

### P2 — Confusing the two `std-widgets.slint`

There are **two** files called `std-widgets.slint` in the codebase:

1. `senders/ui-components/std-widgets.slint` — the project's
   wrapper / re-export shim.
2. Slint's built-in `std-widgets.slint`, imported via
   `import { … } from "std-widgets.slint"` (no path, resolved by
   slint-build).

Don't confuse them when auditing §2.4. Only the **project's** copy
crosses the repo boundary; the **built-in** is fine post-move.

### P3 — Forgetting binary files in the count

Images, fonts, the Gradle wrapper jar, and any vendored AAR live
under `senders/android/`. `git ls-files` finds them but `wc -l` on
them is meaningless. Use `git ls-files -z | xargs -0 stat -c'%s'`
for byte size on binaries.

### P4 — Not recording the source commit SHA

§1.1 records `git rev-parse HEAD`. If you skip this, STEP-3's Git
dep can't pin to a known-good revision. The whole new repo's first
build depends on this SHA — record it twice.

### P5 — Picking Option B (publish) without a release plan

If you pick crates.io, you need (a) crate ownership tokens, (b) a
semver policy, (c) a yanking policy, (d) a CHANGELOG, (e) a release
PR template. None of this exists today. Don't pick Option B "to be
ideologically pure" without doing the release-engineering work
first.

### P6 — Picking Option C (submodule) and forgetting CI

Submodules need `git clone --recurse-submodules` (or a follow-up
`git submodule update --init --recursive`) in **every** CI step
that clones the repo. Forgetting this in even one CI job produces
"file not found" errors with confusing messages. If you pick C,
STEP-6 has more work.

---

## 5. Next step

[Step 2 — bootstrap the new repo and move the source tree](./MVP-PHASE-10-STEP-2-bootstrap-new-repo.md).

Bring the §2.1 file inventory, the §2.7 strategy choice, and the
§1.1 source SHA with you — STEP-2 starts by creating the new repo
and STEP-3 uses the SHA.

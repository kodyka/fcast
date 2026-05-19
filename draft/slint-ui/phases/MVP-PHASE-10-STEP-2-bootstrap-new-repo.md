# MVP-PHASE-10 — Step 2: bootstrap the new repo and move the source tree

> Part 2 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 1 — pre-flight inventory](./MVP-PHASE-10-STEP-1-preflight-inventory.md).
>
> **First step that writes to disk** (but to a clone of the new
> repo, not to the monorepo). The monorepo is read-only through the
> end of STEP-7.

---

## 0. Goal

Create an empty GitHub repo named `fcast-android-sender` (per
STEP-1 §2.8 decision), clone it locally, copy `senders/android/`
into it via `git mv` on a working monorepo clone, and commit the
first snapshot.

After STEP-2:

- The new repo exists on GitHub with at least one commit on its
  default branch.
- The new repo's filesystem matches `senders/android/` in the
  monorepo, **but rooted at the repo root** (no `senders/android/`
  prefix).
- The new repo has a license file, a `.gitignore`, and a README
  skeleton.
- The new repo's `Cargo.toml` and `build.rs` are still broken (path
  deps still reference `../../sdk/...` paths that don't exist in
  the new repo). STEP-3 / STEP-4 / STEP-5 / STEP-6 fix this.

This step does **not** make the new repo build. That's STEP-7.

---

## 1. Pre-flight

### 1.1 Decisions carried from STEP-1

| Decision | Value (from STEP-1) |
|---|---|
| Repo name | `fcast-android-sender` (default) |
| Default branch | `main` (default) |
| Path-dep strategy | Option A (Git dep with subpath) |
| Source commit SHA | (recorded in STEP-1 §1.1) |
| File inventory count | (recorded in STEP-1 §2.1) |

### 1.2 Tools needed

- GitHub permissions to create a repo in the chosen org.
- A local clone of `kodyka/fcast` at the SHA recorded in STEP-1.
- `git` 2.34+ (for `git mv` on directories with subpaths).

### 1.3 Two valid copy strategies

| Strategy | What it does | When to pick |
|---|---|---|
| **Plain copy** (`cp -r senders/android/ /path/to/new-repo/`) | New repo has no history of the old code. | Default. Cleanest. History stays in the monorepo until STEP-8 deletes it. |
| **`git filter-repo`** (subdirectory extraction) | New repo carries every commit that ever touched `senders/android/`. | If "git blame" history for the Android sender code matters to maintainers, and the cost of `git filter-repo` (rewriting history, force-pushing) is acceptable. |

**Default: plain copy.** History is preserved in the monorepo (we
don't delete monorepo history in STEP-8 — we just `rm -r` the
directory in a new commit). Anyone needing pre-PHASE-10 history can
clone the monorepo at any commit before STEP-8's PR.

The rest of this doc assumes plain-copy. If you pick
`git filter-repo`, swap §2.3 for the filter-repo recipe and add
extra verification (§3 §3.3) that the rewritten history compiles.

---

## 2. The change

### 2.1 Create the empty GitHub repo

Via the GitHub UI **or** the `gh` CLI:

```bash
gh repo create kodyka/fcast-android-sender \
    --description "Standalone FCast Android sender (extracted from kodyka/fcast in MVP-PHASE-10)" \
    --public \
    --add-readme=false   # we add our own README in §2.4
```

If the org is different (e.g. you're forking to `myorg`), substitute
accordingly. The default branch will be `main`.

### 2.2 Clone the new repo locally

```bash
git clone https://github.com/kodyka/fcast-android-sender.git /tmp/new-repo
cd /tmp/new-repo
git status   # → expect "no commits yet" or single README commit
```

### 2.3 Copy the source tree

Working from a clean clone of `kodyka/fcast` checked out at the
SHA recorded in STEP-1 §1.1:

```bash
# In the monorepo clone:
cd /path/to/kodyka-fcast
git checkout <STEP-1-§1.1-SHA>
git status   # must be clean

# Copy everything from senders/android/ to the new repo root.
# Use cp -a to preserve permissions (the gradlew script must keep
# its +x bit). Don't use mv — leave the monorepo intact.
cp -a senders/android/. /tmp/new-repo/

# Sanity check: file count matches STEP-1 §2.1.
cd /tmp/new-repo
git status --short -uall | wc -l
# → should be the same as the count recorded in STEP-1 §2.1.
```

`cp -a` preserves mode bits (gradlew needs `+x`), timestamps, and
symlinks. Don't use plain `cp -r`.

### 2.4 Add the license, .gitignore, and README skeleton

#### License

Match the monorepo license (`MIT`, per `kodyka/fcast`'s root
`Cargo.toml` `[workspace.package] license = "MIT"`):

```bash
cd /tmp/new-repo
curl -sLo LICENSE https://raw.githubusercontent.com/kodyka/fcast/master/LICENSE
# verify the file is sensible
head -5 LICENSE
```

If the monorepo doesn't have a top-level `LICENSE` file (the
workspace package declaration alone doesn't guarantee one), write a
fresh `MIT` LICENSE pointing at the project's copyright holders.

#### .gitignore

The monorepo has a top-level `.gitignore` that covers Rust + Slint +
Gradle + Android. The new repo can lift the Android-relevant
sections. Minimum acceptable:

```gitignore
# Rust
target/
**/*.rs.bk
Cargo.lock           # keep this only if the repo is a library; remove
                     # this line for the android-sender (it's an app —
                     # check Cargo.lock in)

# Slint
*.slint.swp

# Android / Gradle
.gradle/
build/
local.properties
captures/
.idea/
*.iml
.cxx/
app/build/
app/.cxx/
**/build/intermediates/
**/build/outputs/

# IDE
.vscode/
.DS_Store

# OS
Thumbs.db
```

The line about `Cargo.lock`: **the android-sender is an app, not a
library** — check `Cargo.lock` in. Same convention as the monorepo.

#### README skeleton

```markdown
# fcast-android-sender

Standalone Android sender app for the [FCast protocol](https://github.com/kodyka/fcast).

Extracted from `kodyka/fcast` at commit `<STEP-1-§1.1-SHA>` via
[MVP-PHASE-10](https://github.com/kodyka/fcast/blob/master/draft/slint-ui/phases/MVP-PHASE-10-android-sender-repo-extraction.md).

## Building

Required environment:

- Rust nightly toolchain (the build uses `cargo +nightly`).
- `ANDROID_NDK_ROOT` (or `ANDROID_NDK_HOME`) pointing at NDK r25c
  or later.
- `GSTREAMER_ROOT_ANDROID` pointing at the GStreamer Android SDK
  (`gstreamer-1.0-android-universal-1.28.0` or compatible).
- Gradle (provided via `./gradlew`).

```bash
# Cross-compile check (any host):
cargo +nightly check --target aarch64-linux-android

# Full APK build (requires the env above):
./gradlew assembleDebug
```

## Repository layout

- `Cargo.toml`, `build.rs`, `src/` — the `android-sender` Rust crate
  (cdylib that the Android app loads at runtime).
- `ui/` — Slint UI tree (`main.slint`, `bridge.slint`, `theme.slint`,
  `pages/`, `components/`).
- `app/` — Android Java app shell, `jni/Android.mk`, resources.
- `gradle/`, `gradlew*`, `build.gradle`, `settings.gradle` — Gradle.
- `ci/` — `ui-validate.sh` and other CI helpers.
- `Dockerfile` — builds the Android+Rust+GStreamer cross-compile
  environment.

## SDK dependencies

The Rust crate depends on three SDK crates that live in
[`kodyka/fcast`](https://github.com/kodyka/fcast). They are pulled
in as Git dependencies with the `path` subspec; see
`Cargo.toml`.

To bump the SDK pin, see
[`docs/cross-repo-sync.md`](docs/cross-repo-sync.md) (added in
STEP-9).

## License

MIT
```

### 2.5 First commit

```bash
cd /tmp/new-repo
git add -A
git status                # eyeball: should be the whole tree + LICENSE/README/.gitignore
git -c user.name="<your name>" -c user.email="<your email>" \
    commit -m "Initial extraction from kodyka/fcast@<STEP-1-§1.1-SHA>

Extracted senders/android/ from https://github.com/kodyka/fcast
at commit <SHA> per MVP-PHASE-10 STEP-2. SDK path deps still point
at ../../sdk/... and will fail to resolve until STEP-3 rewrites
them.

Cargo, Gradle, and slint-build do not yet succeed in this commit;
see MVP-PHASE-10 STEPs 3-7."
```

### 2.6 Push to GitHub

```bash
cd /tmp/new-repo
git push origin main
```

After this push, the new repo's GitHub page shows the directory
listing. CI is **not** set up yet (STEP-6); GitHub Actions / Gitlab
CI will be silent until then.

### 2.7 Tag the extraction point

Mark the commit so STEP-9's pin-bump procedure has a known anchor:

```bash
git tag -a phase-10-extraction -m "MVP-PHASE-10 extraction point. Sibling of kodyka/fcast@<SHA>."
git push origin phase-10-extraction
```

The tag is purely a label; it doesn't change behaviour.

---

## 3. Verification

### 3.1 File count parity

```bash
# In the monorepo:
cd /path/to/kodyka-fcast
git ls-files senders/android | wc -l   # → record N

# In the new repo:
cd /tmp/new-repo
git ls-files | wc -l   # → expect N + 3 (LICENSE, README.md, .gitignore)
                       # — but watch for any of those that already
                       #   existed in senders/android/; subtract
                       #   accordingly.
```

If the count differs by more than the 3 new files, something was
skipped (likely a dotfile or symlink). `cp -a` should not skip
anything; double-check `git status` from §2.3.

### 3.2 Mode bits preserved

```bash
cd /tmp/new-repo
ls -l gradlew gradlew.bat
# → gradlew should have +x set. If not, `chmod +x gradlew` and
#   commit a fix.
```

### 3.3 Tree shape matches expected

```bash
cd /tmp/new-repo
ls -la
```

Expected top-level entries:

```
Cargo.toml
Dockerfile
LICENSE             (new in STEP-2 §2.4)
README.md           (new in STEP-2 §2.4)
TODO.codecs
app/
build.gradle
build.rs
ci/
gradle/
gradle.properties
gradlew
gradlew.bat
settings.gradle
src/
ui/
```

If `senders/` shows up at the top level, you accidentally copied the
directory wrapper too — restart from §2.3 with the `senders/android/.`
(note the trailing dot) form.

### 3.4 No build attempts yet

Do **not** run `cargo check` in the new repo at this stage. It will
fail (path deps unresolved). Failing builds in the first commit are
fine and expected. STEP-7 is the first build attempt.

---

## 4. Pitfalls specific to this step

### P1 — Using `git mv` instead of `cp -a`

`git mv senders/android /tmp/new-repo/` works but commits a deletion
in the monorepo as a side effect. STEP-8 owns the monorepo-side
delete, not STEP-2. Use `cp -a` (or `rsync -a`) to keep the
monorepo intact.

### P2 — Forgetting `cp -a`'s trailing dot

`cp -a senders/android /tmp/new-repo/` creates
`/tmp/new-repo/android/...` (wrong).
`cp -a senders/android/. /tmp/new-repo/` copies the **contents** to
the new repo root (right). The trailing `/.` is load-bearing.

### P3 — Committing as the wrong author

The first commit's author is preserved forever. If you're running
the move on a shared machine, double-check `git config user.name`
and `user.email` before the §2.5 commit. Use
`git commit --author="Name <email>"` if you need to override.

### P4 — Pushing to `master` instead of `main`

GitHub's default for new repos is `main`. The monorepo uses
`master`. Match whichever you decided in STEP-1 §2.9 and push to
the matching ref.

### P5 — Including the Gradle wrapper jar's SHA in the .gitignore

`gradle/wrapper/gradle-wrapper.jar` is a checked-in binary. Do
**not** add `*.jar` to `.gitignore` — it would un-track the
wrapper jar and the next `./gradlew` invocation would re-download
a (potentially different-version) jar.

### P6 — Forgetting the existing `senders/android/README.md`

The directory already contains a `README.md`. The §2.4 README
**overwrites** it. If the original README has content worth
keeping (mostly historical at this point), move its content into
a new file (e.g. `docs/legacy-readme.md`) before overwriting. STEP-1
§2.1 listed all files including the old README — check it.

---

## 5. Next step

[Step 3 — resolve path dependencies (apply chosen strategy)](./MVP-PHASE-10-STEP-3-resolve-path-deps.md).

The new repo now exists. STEP-3 rewrites the three `path = ...`
deps to point at `kodyka/fcast` via Git, registry, or submodule.

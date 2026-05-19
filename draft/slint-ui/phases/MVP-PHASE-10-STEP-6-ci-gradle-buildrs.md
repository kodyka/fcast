# MVP-PHASE-10 — Step 6: CI, Gradle, Dockerfile, and the `build.rs` env

> Part 6 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 5 — vendor Slint helpers](./MVP-PHASE-10-STEP-5-vendor-slint-helpers.md).

---

## 0. Goal

Make the new repo's CI work. There are three CI surfaces in the
monorepo to handle:

1. **GitHub Actions** (`.github/workflows/android-release-apk.yml`)
   — `ui-validate` + `build-android-arm64-debug` jobs run on every
   push.
2. **Gitlab CI** (`senders/android/.gitlab-ci.yml`, included from
   the root `.gitlab-ci.yml`) — the legacy mirror's CI that
   produces release APKs. Optional to mirror, depending on whether
   the new repo also gets mirrored to Gitlab.
3. **Local Docker** (`senders/android/Dockerfile`) — the
   cross-compile environment for the Gitlab job.

Also in scope:

- The Gradle wrapper (already moved via `cp -a` in STEP-2; verify).
- The `ci/ui-validate.sh` script (also already moved; verify and
  update paths if needed).
- The `build.rs` env vars (`ANDROID_NDK_ROOT`, `GSTREAMER_ROOT_ANDROID`)
  documented in the README and embedded in the CI workflows.

After STEP-6:

- The new repo has working GitHub Actions for `ui-validate` and
  `build-android-arm64-debug`.
- Optionally, the new repo has a `.gitlab-ci.yml` for the Gitlab
  mirror.
- The Dockerfile, Gradle wrapper, and `ui-validate.sh` work in the
  new repo's tree without monorepo paths.

---

## 1. Pre-flight

### 1.1 Live state in the monorepo

These files live in the monorepo as of master HEAD and need
attention:

| File | Action |
|---|---|
| `.github/workflows/android-release-apk.yml` | Adapt + copy into new repo's `.github/workflows/`. |
| `senders/android/.gitlab-ci.yml` | Adapt + copy if mirroring to Gitlab. |
| `senders/android/Dockerfile` | Already at new repo root (STEP-2); rebuild & push to new registry path. |
| `senders/android/ci/ui-validate.sh` | Already at `ci/` in new repo (STEP-2); audit paths. |
| `senders/android/gradlew*` | Already at new repo root; no change. |
| `xtask/` (workspace member) | **Not copied.** GHA workflow uses `cargo xtask android download-{sdk,ndk,gstreamer}`; we need a replacement. See §1.2. |

### 1.2 The `xtask` dependency problem

The current GHA workflow at
`.github/workflows/android-release-apk.yml` runs:

```yaml
- name: Download Android SDK, NDK, and GStreamer
  run: |
    cargo xtask android download-sdk
    cargo xtask android download-ndk
    cargo xtask android download-gstreamer
```

`xtask` is a workspace member in `kodyka/fcast` that, among many
other things, downloads the thirdparty toolchains. It depends on
`receiver-resources` (`crates/receiver-resources`) on macOS/Windows
targets and pulls in 10+ other crates. **Vendoring all of `xtask`
into the new repo is excessive** — it brings receiver-side code
that's irrelevant to the Android sender.

Two viable approaches:

| Approach | Cost | Notes |
|---|---|---|
| **A. Inline shell** — replace `cargo xtask` calls with `wget` / `curl` / `tar` directly in the workflow. | Low (~30 lines of shell in the workflow). | Brittle: any download URL change in `xtask/src/android.rs` upstream goes undetected. |
| **B. Mini-xtask** — write a small Cargo bin crate at `xtask-android/` in the new repo that does only the Android downloads. | Medium (one new crate, ~200 lines). | Mirrors the monorepo's `xtask/src/android.rs` subset; easy to keep in sync. |

**Default: A (inline shell).** The new repo doesn't have other CI
needs that justify a full xtask. STEP-6 §2.3 gives the exact
workflow snippet.

### 1.3 The Gitlab mirror question

The monorepo is mirrored to `gitlab.futo.org/videostreaming/fcast`
(referenced in `sdk/sender/fcast-sender-sdk/Cargo.toml:repository`).
The Gitlab CI is the **release** CI — it produces signed APKs and
pushes to the registry.

If the new repo also gets mirrored to Gitlab, copy
`senders/android/.gitlab-ci.yml` (the per-sender include) into the
new repo's root as `.gitlab-ci.yml`, rewrite paths
(`senders/android/...` → `./`), and rebuild + push the Dockerfile
to the new repo's registry namespace.

If the new repo stays GitHub-only, **skip the Gitlab work**. The
release flow needs a separate decision (probably a GitHub Actions
release workflow that signs and uploads to the Play Store / release
page); that's out of scope for PHASE-10 and gets its own follow-up.

---

## 2. The change

### 2.1 Audit `ci/ui-validate.sh`

```bash
cd /tmp/new-repo
grep -nE '(\.\./)+|sender/|senders/android' ci/ui-validate.sh
# → expect 0 matches if the script was already path-agnostic
#   (it shells out to `cargo build -p android-sender`, which works
#   from the crate root).
```

If matches show up (e.g. the script does `cd senders/android`),
patch them. Most likely no patch is needed — the script is already
intended to run with the working directory being the crate root.

Sanity check:

```bash
cd /tmp/new-repo
bash ci/ui-validate.sh --help    # → should print the help text from
                                  #   lines 2-34.
```

### 2.2 Update `Dockerfile` paths (if any)

```bash
cd /tmp/new-repo
grep -nE 'COPY .*(senders/android|sdk|crates)' Dockerfile
```

If the Dockerfile has `COPY senders/android/...` lines (it
shouldn't — Docker builds from the build context, which is the
directory the `docker build` command was run from), patch them.

Build the image locally to confirm:

```bash
docker build -t fcast-android-sender-dev:latest .
```

This produces an image that can be used as the `image:` in the
Gitlab CI job (§2.4) or pushed to a new registry (replacing
`$CI_REGISTRY/videostreaming/fcast/android-sender-dev:latest`).

### 2.3 New GitHub Actions workflow

Create `.github/workflows/android-release-apk.yml` in the new repo.
The file is a mostly-faithful copy of the monorepo's same-named
workflow, with two changes:

1. The `cargo xtask android download-*` lines replaced with inline
   downloads.
2. The `ui-validate` job's script path (was
   `senders/android/ci/ui-validate.sh`) becomes `ci/ui-validate.sh`.

Full workflow (snippet, fill in download URLs from xtask source):

```yaml
name: Android APK Release

on:
  push:
  release:
    types: [published]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  ui-validate:
    runs-on: ubuntu-24.04
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Run UI audit (audit-only mode)
        run: ci/ui-validate.sh --no-build

  build-android-arm64-debug:
    runs-on: ubuntu-24.04
    env:
      CARGO_TERM_COLOR: always

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up Java
        uses: actions/setup-java@v4
        with:
          distribution: temurin
          java-version: '21'

      - name: Set up Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-linux-android

      - name: Install Linux build dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            build-essential cmake pkg-config automake autoconf libtool \
            libprotobuf-dev protobuf-compiler unzip xz-utils wget curl \
            ninja-build libclang-dev

      - name: Install cargo-ndk
        run: cargo install cargo-ndk --locked

      # ─────────── inline replacement for `cargo xtask android download-*` ──
      - name: Download Android SDK
        run: |
          mkdir -p thirdparty/Android/Sdk/cmdline-tools
          # URL & version cross-referenced from kodyka/fcast xtask/src/android.rs
          # at SHA <STEP-1-§1.1-SHA>. Bump when the upstream xtask bumps.
          curl -fsSL https://dl.google.com/android/repository/commandlinetools-linux-<VER>_latest.zip \
              -o /tmp/cmdline-tools.zip
          unzip -q /tmp/cmdline-tools.zip -d thirdparty/Android/Sdk/cmdline-tools
          mv thirdparty/Android/Sdk/cmdline-tools/cmdline-tools thirdparty/Android/Sdk/cmdline-tools/latest
          yes | thirdparty/Android/Sdk/cmdline-tools/latest/bin/sdkmanager --licenses
          thirdparty/Android/Sdk/cmdline-tools/latest/bin/sdkmanager "platform-tools" "platforms;android-34"

      - name: Download Android NDK (r25c)
        run: |
          mkdir -p thirdparty
          curl -fsSL https://dl.google.com/android/repository/android-ndk-r25c-linux.zip \
              -o /tmp/ndk.zip
          unzip -q /tmp/ndk.zip -d thirdparty

      - name: Download GStreamer Android SDK
        run: |
          mkdir -p thirdparty/gstreamer-1.0-android-universal-1.28.0
          # mirror the monorepo's pinned version; update on bump
          curl -fsSL --retry 5 \
              https://gstreamer.freedesktop.org/pkg/android/1.28.0/gstreamer-1.0-android-universal-1.28.0.tar.xz \
              -o /tmp/gst.tar.xz
          tar -xf /tmp/gst.tar.xz -C thirdparty/gstreamer-1.0-android-universal-1.28.0

      - name: Export Android toolchain paths
        run: |
          echo "ANDROID_HOME=$GITHUB_WORKSPACE/thirdparty/Android/Sdk" >> "$GITHUB_ENV"
          echo "ANDROID_SDK_ROOT=$GITHUB_WORKSPACE/thirdparty/Android/Sdk" >> "$GITHUB_ENV"
          echo "ANDROID_NDK_ROOT=$GITHUB_WORKSPACE/thirdparty/android-ndk-r25c" >> "$GITHUB_ENV"
          echo "GSTREAMER_ROOT_ANDROID=$GITHUB_WORKSPACE/thirdparty/gstreamer-1.0-android-universal-1.28.0" >> "$GITHUB_ENV"

      - name: cargo +nightly check
        run: cargo check --target aarch64-linux-android

      - name: ./gradlew assembleDebug
        run: ./gradlew assembleDebug
```

**Notes:**

- The exact `<VER>` for cmdline-tools and the exact NDK / GStreamer
  versions must match what `xtask/src/android.rs` downloads in the
  monorepo at the STEP-1 SHA. Look at `xtask/src/android.rs` for
  the constants.
- The GStreamer download has historically been flaky (the
  `gstreamer.freedesktop.org` mirror sometimes returns HTTP errors
  — see PR #44's CI failure in the audit log). `--retry 5` and a
  `curl` instead of `wget` gives a slightly better signal.
- Caching: consider adding `actions/cache` for `~/.cargo` and
  `thirdparty/`. The current monorepo workflow doesn't cache, so
  matching it is fine for STEP-6 — but this is an obvious
  follow-up optimisation.

### 2.4 Optional: Gitlab CI

If the new repo is also mirrored to Gitlab, create `.gitlab-ci.yml`
at the new repo root by lifting the **content** of
`senders/android/.gitlab-ci.yml` from the monorepo and rewriting:

- `cd senders/android` → remove (already at repo root).
- `$CI_REGISTRY/videostreaming/fcast/android-sender-dev:latest` →
  `$CI_REGISTRY/<new-org>/fcast-android-sender/android-sender-dev:latest`.
- Artifact paths `senders/android/$ANDROID_VERSION_CODE/...` → just
  `$ANDROID_VERSION_CODE/...`.

If no Gitlab mirror, skip.

### 2.5 README env-vars section

Update the new repo's README (created in STEP-2 §2.4) to document
the env vars `build.rs` needs:

````markdown
## Required environment

`build.rs` is a no-op on non-Android targets. On Android targets
(`aarch64-linux-android`, `armv7-linux-androideabi`,
`x86_64-linux-android`, `i686-linux-android`), the following env
vars must be set:

| Var | Purpose | Example value |
|---|---|---|
| `ANDROID_NDK_ROOT` (or `ANDROID_NDK_HOME` fallback) | Path to the Android NDK | `~/Android/Sdk/ndk/25.2.9519653` |
| `GSTREAMER_ROOT_ANDROID` | Path to the GStreamer Android SDK | `~/gstreamer-1.0-android-universal-1.28.0` |

Without these, the build script prints
`cargo:warning=Skipping Android linker setup` and the link step
fails at the final stage. See `build.rs:15-37` for the exact
behaviour.

For Gradle builds (`./gradlew assembleDebug`), additionally export
`ANDROID_HOME` (or `ANDROID_SDK_ROOT`):

| Var | Purpose | Example value |
|---|---|---|
| `ANDROID_HOME` | Path to the Android SDK | `~/Android/Sdk` |
| `ANDROID_SDK_ROOT` | (alias for ANDROID_HOME) | `~/Android/Sdk` |
````

This text is **doc-only** — STEP-6 doesn't touch `build.rs` itself
(STEP-2's `cp -a` already moved it).

### 2.6 Gradle wrapper version

Verify the Gradle wrapper version matches a known-good (the
monorepo's) version:

```bash
cat gradle/wrapper/gradle-wrapper.properties
```

Compare with the monorepo's. If they differ, the new repo's wrapper
might be staler than expected — but **don't regenerate** with
`gradle wrapper` casually; that changes the wrapper jar's SHA and
needs a verification commit on its own.

---

## 3. Verification

### 3.1 Local Docker build

```bash
cd /tmp/new-repo
docker build -t fcast-android-sender-dev:latest .
```

Should succeed. If it fails on a `COPY` line, audit §2.2.

### 3.2 `ci/ui-validate.sh --no-build` runs to completion

```bash
cd /tmp/new-repo
bash ci/ui-validate.sh --no-build
echo $?
# → 0 (or 1 if there are pre-existing UI audit warnings — those
#   are content issues, not STEP-6 issues).
```

### 3.3 GitHub Actions on the new repo

After §2.3 + push to the new repo's default branch (STEP-2 §2.6),
GitHub Actions kicks off:

```bash
# Once the workflow file is committed:
git add .github/workflows/android-release-apk.yml
git commit -m "ci: add android-release-apk workflow"
git push origin main
```

Watch the Actions tab. Expected outcomes:

- `ui-validate`: **passes** (it's a fast static audit; no
  toolchain).
- `build-android-arm64-debug`: **passes** if the inline downloads
  succeed. Common failures:
  - GStreamer download HTTP error (retry with `--retry 5` already
    in the workflow; otherwise re-run).
  - Wrong NDK version (the cargo-ndk install might bring r28+ while
    `build.rs` expects r25c paths). Pin cargo-ndk version
    explicitly if you hit this.

### 3.4 Local `cargo check`

```bash
cd /tmp/new-repo
export ANDROID_NDK_ROOT=/path/to/ndk
export GSTREAMER_ROOT_ANDROID=/path/to/gst
cargo +nightly check --target aarch64-linux-android
echo $?    # → 0 on a clean STEP-5-done tree.
```

If this still fails, STEP-5 or STEP-3 / STEP-4 wasn't fully
applied. Don't proceed to STEP-7 with a failing local check.

---

## 4. Pitfalls specific to this step

### P1 — Translating `cargo xtask` calls one-by-one with curl URLs that are stale

The xtask source is the **source of truth** for what version of
each toolchain is expected. Pinning URLs in the workflow without a
mechanism to keep them in sync with `xtask/src/android.rs` is a
maintenance debt. Either (a) write a small Rust bin in the new
repo that imports from `xtask`-style code, (b) accept the debt and
add a calendar reminder to re-verify quarterly, or (c) automate via
a Dependabot-style update PR for download URLs.

### P2 — Forgetting `chmod +x ci/ui-validate.sh` after `cp -a`

If the source has `+x` and `cp -a` preserves it, you're fine. If
not, GitHub Actions fails with "permission denied". Re-`chmod` and
commit. Same for `gradlew`.

### P3 — Caching `~/.cargo` without invalidation

The monorepo workflow doesn't cache `~/.cargo`. If you add caching,
the cache key must include the SHA of `Cargo.lock` (otherwise stale
deps end up being used). Use
`hashFiles('**/Cargo.lock')` in the cache key.

### P4 — Mirroring the Gitlab CI half-way

If you copy `senders/android/.gitlab-ci.yml` but forget to update
`$CI_REGISTRY` paths, the job tries to push the dev container to
the **monorepo's** registry — which fails with auth errors. Either
do it all (§2.4) or skip it entirely.

### P5 — `dtolnay/rust-toolchain@stable` vs nightly

The monorepo's workflow uses `dtolnay/rust-toolchain@stable` and
the build still works (the project compiles on stable; the docs
sometimes say "nightly" as a habit). Match the monorepo. If you
want nightly, change to `@nightly` and also `cargo +nightly` in
the build step.

### P6 — `actions/setup-java@v4 java-version: '21'` mismatch

The monorepo uses Java 21. If you bump (e.g. to 24), the Android
Gradle Plugin may not be ready for that version. Match the
monorepo unless you have a reason to bump.

---

## 5. Next step

[Step 7 — first build + verification](./MVP-PHASE-10-STEP-7-first-build-verification.md).

CI is configured. STEP-7 is the moment of truth: does the new repo
actually build and produce a working APK?

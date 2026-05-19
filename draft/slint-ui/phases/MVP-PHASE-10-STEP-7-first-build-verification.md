# MVP-PHASE-10 — Step 7: first build + verification

> Part 7 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 6 — CI / Gradle / build.rs env](./MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md).
>
> **The "are we there yet" step.** No code change; instead, run
> every verification matrix below and only proceed to STEP-8 once
> every check is green.

---

## 0. Goal

Prove that the new repo, as it stands at the end of STEP-6, can:

1. Pass `cargo +nightly check --target aarch64-linux-android`
   locally.
2. Pass `cargo +nightly build --release --target aarch64-linux-android`
   locally.
3. Produce an installable debug APK via `./gradlew assembleDebug`.
4. Install on a real (or emulated) Android device and reach the
   ConnectView.
5. Exercise the four PHASE-9 debug quick-actions: `migrated-server`,
   `test-getinfo`, `test-crossfade`, `test-smoke` — each must
   succeed.
6. Cast end-to-end to a real FCast receiver (rcore / Electron /
   webOS).
7. Pass both CI jobs on GitHub Actions: `ui-validate` and
   `build-android-arm64-debug`.

After STEP-7:

- The new repo is a viable replacement for the monorepo's
  `senders/android/` directory.
- STEP-8 (delete from monorepo) is safe to start.

If any of §0.1-0.7 fails, **stop**. Fix the underlying step
(STEP-3/4/5/6) before proceeding.

---

## 1. Pre-flight

### 1.1 Environment

| Item | Value (per the monorepo's setup) |
|---|---|
| Rust toolchain | nightly (or stable; match STEP-6 §2.3 choice) |
| Java | 21 |
| Android NDK | r25c |
| Android SDK platform | android-34 |
| GStreamer Android SDK | 1.28.0 |
| Slint | 1.16.0 |

If your local env differs, `cargo check` may produce different
warnings than CI; that's fine. The CI run is the canonical signal.

### 1.2 Test device

For §0.4-0.6, you need either:

- A physical Android phone with USB debugging on, or
- An emulator running an arm64 / x86_64 Android 14 (API 34) image
  with the FCast receiver app running on a separate machine, or
- An Android emulator with rcore running on localhost (loop back
  routing required — google "adb reverse" for the receiver port).

For §0.6, the receiver also needs to be running. Easiest: rcore on
the same desktop, then `adb reverse tcp:46899 tcp:46899` so the
phone can discover it.

---

## 2. The change

This step is **verification only** — no commits, no edits.

---

## 3. Verification

### 3.1 Cargo check passes locally

```bash
cd /tmp/new-repo
export ANDROID_NDK_ROOT=/path/to/ndk
export GSTREAMER_ROOT_ANDROID=/path/to/gst
cargo +nightly check --target aarch64-linux-android 2>&1 | tee /tmp/check.log
echo "exit: $?"
```

**Expected:** exit code 0. Warnings are fine. Errors are not.

If `slint-build` errors with "cannot resolve import":
- Audit STEP-5 §3.1.

If Cargo errors with "workspace dependency not found":
- Audit STEP-4 §3.1.

If Cargo errors with "no matching package found in registry":
- A workspace dep was inlined with a version that doesn't exist on
  crates.io. Re-check STEP-4 §1.2 versions.

### 3.2 Cargo build (release) produces a `.so`

```bash
cargo +nightly build --release --target aarch64-linux-android 2>&1 | tail -20
ls -lh target/aarch64-linux-android/release/libfcastsender.so
```

**Expected:** `libfcastsender.so` exists, size in the
30-80 MB range. If it's < 1 MB, something is wrong (likely
`[profile.release]` wasn't copied from STEP-4 §2.3, or LTO
silently failed).

If `cargo build` fails at the link stage:
- Confirm `GSTREAMER_ROOT_ANDROID` is set and the lib path exists:
  ```bash
  ls "$GSTREAMER_ROOT_ANDROID/arm64/lib/" | head
  ```
- Confirm the NDK has the right host-toolchain path:
  ```bash
  ls "$ANDROID_NDK_ROOT/toolchains/llvm/prebuilt/" | head
  ```
- Re-read `build.rs:42-52` (the search-path construction).

### 3.3 Gradle build produces a debug APK

```bash
cd /tmp/new-repo
export ANDROID_HOME=/path/to/sdk
./gradlew assembleDebug 2>&1 | tail -30
ls -lh app/build/outputs/apk/debug/app-debug.apk
```

**Expected:** APK file, size in the 50-150 MB range (it bundles
the .so + GStreamer plugins + Slint backend).

If Gradle fails:
- Check `app/build.gradle` for any path that crosses the
  `senders/android/` boundary. (STEP-2's `cp -a` moved the
  `app/` directory verbatim; paths inside it should already be
  repo-local.)
- Check `gradle/libs.versions.toml` for any reference to a
  monorepo path. Same — should be self-contained.

### 3.4 Install and launch on device

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n org.fcast.sender/org.fcast.sender.MainActivity
adb logcat -d | grep -E '(fcastsender|panicked at|FATAL)' | tail -50
```

**Expected:**

- `adb install` returns "Success".
- `am start` launches the app; the device screen shows the
  `ConnectView`.
- `adb logcat` shows no `panicked at` or `FATAL` lines for the
  `org.fcast.sender` process.

If the app crashes on launch, capture the full logcat
(`adb logcat -d > /tmp/crash.log`) and inspect for Rust panics. The
most common reason at this stage is a Slint import path that was
rewritten incorrectly in STEP-5.

### 3.5 PHASE-9 debug quick-actions

In a debug build (the `assembleDebug` APK), the four debug
quick-actions are visible in `QuickActionsPage`. They route
through the Bridge callbacks landed by PHASE-9 (merged to
`master` at `b394eea` / `d8ff886`):

- Quick-action call sites: `lib.rs:2108-2126`
  (`invoke_start_migration_server` / `invoke_run_migration_test`).
- Bridge handlers: `lib.rs:2136-2185`
  (`on_start_migration_server` / `on_run_migration_test` /
  `on_stop_migration_server`).
- Lazy `start_graph_runtime()` ensure-calls: `lib.rs:191` (Bridge
  handler entry), `lib.rs:955` (`Event::CaptureStarted`),
  `lib.rs:2240` (`nativeProcessGraphCommandJson` JNI hook).
- Bridge callback declarations: `bridge.slint:251-253`.

Tap each and check the displayed `Bridge.test-status` value:

| Quick action | Expected `Bridge.test-status` value |
|---|---|
| `Migrated srv` | `PASS migrated server active on 0.0.0.0:8080 (graph runtime up)` |
| `GetInfo` | `PASS legacy getinfo HTTP …` |
| `Crossfade` | `PASS legacy crossfade HTTP …` |
| `Smoke Graph` | `PASS graph smoke …` |

If any FAIL, the Bridge → migration runtime wiring is broken; this
is the PHASE-9 contract being exercised end-to-end. Likely cause
is the SDK Git-dep pin (STEP-3) pointing at a commit where the
runtime API has drifted from what the new repo's `lib.rs` calls.

### 3.6 End-to-end cast

With an rcore receiver running on the same network:

1. Open the app.
2. Discover the receiver in the ConnectView.
3. Tap the receiver → connect.
4. Tap "Start mirroring" → grant `MediaProjection`.
5. On the receiver, confirm the cast stream is rendering.
6. Tap "Stop" in the app → confirm receiver stops rendering.

This is the highest-signal check that PHASE-1..8 still work in the
new repo (the cast loop, the WHEP signaller, the graph-command
dispatch — all SDK-side and going through the Git-dep).

### 3.7 CI on the new repo

After STEP-6's workflow file is pushed:

```bash
gh run list --repo kodyka/fcast-android-sender --limit 5
gh run view <RUN_ID> --log
```

Both `ui-validate` and `build-android-arm64-debug` must be green.

If `build-android-arm64-debug` fails on the GStreamer download with
HTTP 503 / 8, re-run the job. It's a known mirror flake.

### 3.8 Comparison with the monorepo's APK

Build the **same** Android APK from the monorepo (at the STEP-1
SHA) and from the new repo. Diff key properties:

```bash
# In the monorepo:
cd /path/to/kodyka-fcast/senders/android
./gradlew assembleDebug
mv app/build/outputs/apk/debug/app-debug.apk /tmp/monorepo-debug.apk

# In the new repo:
cd /tmp/new-repo
./gradlew assembleDebug
cp app/build/outputs/apk/debug/app-debug.apk /tmp/newrepo-debug.apk

# Compare.
ls -l /tmp/monorepo-debug.apk /tmp/newrepo-debug.apk
unzip -l /tmp/monorepo-debug.apk | sort > /tmp/mono-files.txt
unzip -l /tmp/newrepo-debug.apk | sort > /tmp/new-files.txt
diff /tmp/mono-files.txt /tmp/new-files.txt | head -30
```

**Expected:** the file lists are functionally identical. Differences
in `META-INF/` signing data and timestamps are fine.
Differences in actual bundled files (e.g. an icon missing, a Slint
binary differs in size by more than ~10%) are warnings to
investigate before proceeding.

---

## 4. Pitfalls specific to this step

### P1 — Treating "it builds" as enough

`cargo build` succeeding is the **first** signal, not the last.
§3.4-3.6 are runtime / behaviour checks. A crate that builds but
crashes on launch (because a Slint import resolved at compile time
but the relative path is wrong for resources at runtime) is a
worse outcome than a build failure — it ships broken to users.

### P2 — Forgetting to set the env vars before `cargo check`

`cargo check` without `ANDROID_NDK_ROOT` and
`GSTREAMER_ROOT_ANDROID` set still succeeds with a
`cargo:warning=...` — but `cargo build` fails. Run `build` not just
`check`, at least once.

### P3 — Testing only on the emulator

The emulator's x86_64 build path is **different** from the device's
arm64 path. STEP-6's CI runs only arm64-debug; STEP-7's local
runs should too. If you test only on x86_64 and ship arm64, you
may discover ABI-only issues (e.g. `ndk-context` JNI mangling)
post-merge.

### P4 — Comparing APK sizes naively

The new repo's APK may be a few hundred KB different from the
monorepo's even with identical code, due to (a) timestamps embedded
in `MANIFEST.MF`, (b) signing data, (c) build-time random seeds
in `protobuf`/`prost` if any are used. A few-hundred-KB difference
is fine; a 10MB+ difference is a flag.

### P5 — Skipping the CI run "because it just passed locally"

The CI environment differs from local in significant ways: clean
`~/.cargo`, fresh Cargo registry index, different network paths to
the GStreamer mirror, etc. **Always** wait for green CI before
moving on to STEP-8. STEP-8 is the irreversible-ish step; STEP-7's
CI is the last safety net.

### P6 — Marking §3.6 (end-to-end cast) as untested

If the end-to-end cast test is hard to run (no receiver
infrastructure, no spare device), record it as "untested" and
document the risk. **Do not** mark it as "passed" by faith. A
PHASE-10 that ships with §3.6 untested is a PHASE-10 that may
break casting silently for users on day one of the new repo.

### P7 — The first APK install showing "App not installed"

On Android, "App not installed" usually means a signing-key
mismatch with the previously-installed version. Either uninstall
the old one (`adb uninstall org.fcast.sender`) and reinstall, or
sign the debug APK with the same key. Don't conclude the new repo's
APK is broken — it's the install-side that's confused.

---

## 5. Next step

[Step 8 — remove `senders/android/` from the monorepo](./MVP-PHASE-10-STEP-8-remove-from-monorepo.md).

The new repo is verified. STEP-8 deletes the directory from
`kodyka/fcast` and removes the workspace entry. This is the
"point of no return".

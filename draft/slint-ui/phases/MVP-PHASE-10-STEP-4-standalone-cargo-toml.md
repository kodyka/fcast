# MVP-PHASE-10 — Step 4: standalone `Cargo.toml` (inline workspace deps + `[profile.release]`)

> Part 4 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 3 — resolve path deps](./MVP-PHASE-10-STEP-3-resolve-path-deps.md).

---

## 0. Goal

Make the new repo's `Cargo.toml` compile **without** a parent
`[workspace]` to inherit from. Two sub-goals:

1. Inline every `workspace = true` reference with an explicit
   `version = "..."` (and the feature flags the workspace declared).
2. Replicate the monorepo's `[profile.release]` settings (LTO,
   single codegen unit, strip) so release builds don't get
   accidentally larger / slower.

After STEP-4:

- The new repo's `Cargo.toml` has no `workspace = true` references.
- `cargo +nightly metadata` resolves cleanly.
- The new repo is **either** a single-crate repo (no `[workspace]`
  table — default; see §2.5) **or** a one-member workspace
  (`[workspace]` table referencing only the single crate; see §2.6 for
  why you'd choose this).
- A `cargo check` would still fail on the Slint cross-tree import
  (STEP-5) and / or the build.rs env (STEP-6 documents) — but the
  `Cargo.toml` itself is no longer the bottleneck.

---

## 1. Pre-flight

### 1.1 Inputs from previous steps

| Input | Source |
|---|---|
| Workspace dep version table | STEP-1 §2.3 |
| The 14 `workspace = true` references | STEP-1 §2.3 |
| Build profile from monorepo | STEP-1 §2.6 / parent §1.6 |

### 1.2 Where each version comes from

The monorepo's root `Cargo.toml` `[workspace.dependencies]` table is
the source of truth. Versions verified against monorepo HEAD at the
time of writing:

| Crate | Workspace declaration | Notes |
|---|---|---|
| `slint` | `version = "1.16.0"` (no features in workspace) | features applied by android-sender |
| `slint-build` | `version = "1.16.0"` | build-dep |
| `tokio` | `version = "1.51"`, features `["full"]` | full activates everything |
| `gst` (`package = "gstreamer"`) | `version = "0.25"` | rename via `package` |
| `gst-app` (`package = "gstreamer-app"`) | `version = "0.25"`, features `["v1_24"]` | |
| `gst-video` (`package = "gstreamer-video"`) | `version = "0.25"`, features `["v1_24"]` | |
| `anyhow` | `version = "1"` | |
| `parking_lot` | `version = "0.12"` | |
| `tracing` | `version = "0.1"` | |
| `log` | `version = "0.4"` | |
| `serde` | `version = "1.0"`, features `["derive"]` | |
| `serde_json` | `version = "1.0"` | |
| `uuid` | `version = "1.18"`, features `["v4", "serde"]` | confirm with current root Cargo.toml; the research said 1.18.1 |
| `tracing-subscriber` | `version = "0.3"` | |
| `lazy_static` | `version = "1.5"` | |

**Always re-verify** these against the monorepo at the SHA recorded
in STEP-1 §1.1. The versions drift; the snapshot above will go
stale.

### 1.3 Features applied per-crate

Two layers of features apply to each workspace-dep:

1. The features declared in the **workspace** table (e.g.
   `tokio = { version = "1.51", features = ["full"] }`).
2. The features the android-sender adds on top (e.g.
   `slint = { workspace = true, features = ["backend-android-activity-06", "compat-1-2", "std"] }`).

When inlined, the explicit dep declaration is the **union** of both:

```toml
# Before (workspace = true with android-sender features):
slint = { workspace = true, features = ["backend-android-activity-06", "compat-1-2", "std"] }

# After (inlined):
slint = { version = "1.16.0", features = ["backend-android-activity-06", "compat-1-2", "std"] }
```

Note: the workspace declaration of `slint` doesn't add any features
of its own, so the inlined form just carries the android-sender's
list. For `tokio` (workspace adds `full`) and `serde` (workspace
adds `derive`), confirm whether the android-sender adds anything on
top:

```bash
# In the new repo (or in the monorepo's senders/android/Cargo.toml).
grep -nE '^(tokio|serde|serde_json|gst-app|gst-video|gst|uuid|tracing|slint)\s*=' Cargo.toml
```

For the current monorepo HEAD, the android sender adds:

- `slint`: `["backend-android-activity-06", "compat-1-2", "std"]`
- `tracing`: `["log", "log-always"]`
- `uuid`: `["serde"]` (workspace already has `["v4"]`; final inlined
  list: `["v4", "serde"]`)

Every other workspace-dep is taken verbatim.

---

## 2. The change

### 2.1 Inline every `workspace = true`

**File:** `/tmp/new-repo/Cargo.toml`.

Replace each `workspace = true` (or `.workspace = true`) declaration
with an explicit `version = "..."`. Carry the workspace's features
*plus* any android-sender-specific features.

**Before:**

```toml
[dependencies]
android_logger = "0.15.0"
ndk-context = "0.1.1"
jni = "0.21.1"
slint = { workspace = true, features = ["backend-android-activity-06", "compat-1-2", "std"] }
async-channel = "2.3.1"
lazy_static.workspace = true
futures-core = "0.3.31"
tokio.workspace = true
gst-video.workspace = true
gst-app.workspace = true
anyhow.workspace = true
crossbeam-channel = "0.5.15"
gst.workspace = true
fcast-protocol = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/common/fcast-protocol" }
fcast-sender-sdk = { git = "...", rev = "<SHA>", path = "sdk/sender/fcast-sender-sdk", default-features = false, features = ["fcast"] }
mcore = { git = "...", rev = "<SHA>", path = "sdk/mirroring_core" }
parking_lot.workspace = true
tracing-gstreamer = "0.9.0"
tracing = { workspace = true, features = ["log", "log-always"] }
log.workspace = true
serde.workspace = true
serde_json.workspace = true
uuid = { workspace = true, features = ["serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing-subscriber.workspace = true
gst_rs_webrtc = { package = "gst-plugin-webrtc", version = "0.15", default-features = false, features = ["static"] }

[build-dependencies]
slint-build.workspace = true
```

**After:**

```toml
[dependencies]
# External crates (already pinned, no change).
android_logger = "0.15.0"
ndk-context = "0.1.1"
jni = "0.21.1"
async-channel = "2.3.1"
futures-core = "0.3.31"
crossbeam-channel = "0.5.15"
chrono = { version = "0.4", features = ["serde"] }
tracing-gstreamer = "0.9.0"
gst_rs_webrtc = { package = "gst-plugin-webrtc", version = "0.15", default-features = false, features = ["static"] }

# Inlined from monorepo [workspace.dependencies].
slint = { version = "1.16.0", features = ["backend-android-activity-06", "compat-1-2", "std"] }
tokio = { version = "1.51", features = ["full"] }
gst = { package = "gstreamer", version = "0.25" }
gst-app = { package = "gstreamer-app", version = "0.25", features = ["v1_24"] }
gst-video = { package = "gstreamer-video", version = "0.25", features = ["v1_24"] }
anyhow = "1"
parking_lot = "0.12"
tracing = { version = "0.1", features = ["log", "log-always"] }
log = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.18", features = ["v4", "serde"] }
tracing-subscriber = "0.3"
lazy_static = "1.5"

# Git-dep SDK crates (from STEP-3).
fcast-protocol = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/common/fcast-protocol" }
fcast-sender-sdk = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/sender/fcast-sender-sdk", default-features = false, features = ["fcast"] }
mcore = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/mirroring_core" }

[build-dependencies]
slint-build = "1.16.0"
```

The reorganisation groups deps logically (external vs inlined vs
SDK). The original `Cargo.toml` was alphabetical-ish; either is
fine, but grouping helps the reviewer of this PR see what
changed.

### 2.2 Inline the build-dep

`slint-build.workspace = true` → `slint-build = "1.16.0"`.

There's only the one build-dep. Don't forget it; without it the
build.rs (still calling `slint_build::compile(...)`) fails to
resolve the dep.

### 2.3 Replicate `[profile.release]`

Add at the **bottom** of `Cargo.toml`:

```toml
# ─── Release profile (replicated from kodyka/fcast root Cargo.toml) ───
[profile.release]
lto = "fat"
codegen-units = 1
strip = "debuginfo"
```

Without this, the new repo's release build is:

- Not LTO'd → larger .so, slower startup.
- Multi-codegen-unit → larger .so.
- Debug info not stripped → ~20-30% larger .so.

The monorepo sets these intentionally; copy them.

If the monorepo has other profile customisations (`[profile.dev]`,
`[profile.bench]`, etc.), check the root `Cargo.toml` and copy any
that apply to the android-sender crate.

### 2.4 Confirm `[package.metadata.android]` is preserved

The android-sender's `Cargo.toml` already has its own
`[package.metadata.android]` block (visible at the bottom of the
file). This is **not** workspace-inherited; it lives on the
android-sender crate. STEP-2's `cp -a` preserved it. Verify:

```bash
grep -A20 '^\[package.metadata.android\]' Cargo.toml
```

Expected (excerpt):

```toml
[package.metadata.android]
package = "org.fcast.sender"
build_targets = [ "aarch64-linux-android", "armv7-linux-androideabi", "x86_64-linux-android", "i686-linux-android" ]

[package.metadata.android.sdk]
min_sdk_version = 26
target_sdk_version = 34
```

Don't touch this. The Gradle build reads it.

### 2.5 Default: single-crate repo (no `[workspace]`)

The simplest, most idiomatic choice. The new repo has one crate
(`android-sender`); no `[workspace]` table is needed. This is the
default flow.

A side benefit: `cargo` commands work without `-p`. `cargo check`,
`cargo build`, `cargo test` all operate on the single crate.

### 2.6 Alternative: one-member workspace

Add this **only** if STEP-3 used Option C (submodule) and the
submodule contains crates you want Cargo to be aware of as
workspace members:

```toml
[workspace]
members = ["."]
exclude = ["vendor/fcast"]   # don't pull in monorepo's workspace
```

The `exclude` line keeps the submodule's `kodyka/fcast` workspace
from being confused with the new repo's workspace (otherwise Cargo
walks into `vendor/fcast/Cargo.toml` and tries to interpret its
`[workspace]` block).

This is **not** the default. Skip §2.6 if you picked Option A or B.

### 2.7 Update the `[package]` block for cleanliness

The `[package]` block in the post-STEP-2 commit may still have:

```toml
[package]
name = "android-sender"
version = "0.1.0"
edition = "2021"
```

Add a few quality-of-life fields:

```toml
[package]
name = "android-sender"
version = "0.1.0"
edition = "2021"
description = "FCast Android sender app (Slint UI + GStreamer + WHEP signaller)."
license = "MIT"
repository = "https://github.com/kodyka/fcast-android-sender"
readme = "README.md"
publish = false                   # not on crates.io
```

`publish = false` is important for Option A — without it, an
accidental `cargo publish` would fail anyway (Cargo rejects Git
deps in published crates) but the failure is at the very end of
the publish flow. Explicit is better.

---

## 3. Verification

### 3.1 No remaining `workspace = true`

```bash
grep -nE '\.workspace|workspace\s*=\s*true' Cargo.toml
# → expect 0 matches.
```

### 3.2 `cargo metadata` resolves

```bash
cd /tmp/new-repo
cargo +nightly metadata --format-version=1 > /tmp/metadata.json
echo $?   # → 0
jq '.workspace_members' /tmp/metadata.json
# → ["android-sender 0.1.0 (path+file:///tmp/new-repo)"]
```

If you get errors about "workspace dependency not found", a
`workspace = true` slipped through. Repeat §3.1.

### 3.3 `cargo +nightly check --target aarch64-linux-android` progresses past dep resolution

```bash
cd /tmp/new-repo
cargo +nightly check --target aarch64-linux-android 2>&1 | tail -30
```

The command will **still fail** — STEP-5 hasn't vendored the Slint
helper, so `slint-build` errors at the cross-tree import. But the
error should be a **slint-build error**, not a Cargo dep error. If
you see "no matching package found in workspace", revisit §3.1.

### 3.4 `[profile.release]` is in place

```bash
grep -A4 '\[profile.release\]' Cargo.toml
```

Expected:

```toml
[profile.release]
lto = "fat"
codegen-units = 1
strip = "debuginfo"
```

### 3.5 Cargo.lock checked in

```bash
ls -l Cargo.lock          # → exists
grep -c 'name = ' Cargo.lock  # → expect ~150-300 entries
```

If `Cargo.lock` was previously gitignored (STEP-2 §2.4 has a guard
comment), un-ignore it and commit it.

---

## 4. Pitfalls specific to this step

### P1 — Inlining only the version, dropping features

```toml
# Wrong:
tokio = "1.51"
# Right:
tokio = { version = "1.51", features = ["full"] }
```

Without the workspace's `features = ["full"]`, `tokio` exposes only
its default features. The android-sender uses
`tokio::sync::Mutex`, `tokio::net::TcpListener`, etc. — all behind
features. Drop the feature flag and compilation explodes.

### P2 — Carrying android-sender features only

For workspace-deps where both the workspace **and** the android-
sender add features (e.g. `uuid`, `tracing`), the inlined form must
include the **union**:

```toml
# Workspace adds ["v4"]; android-sender adds ["serde"].
# Wrong (loses v4):
uuid = { version = "1.18", features = ["serde"] }
# Right:
uuid = { version = "1.18", features = ["v4", "serde"] }
```

Always check the workspace declaration and merge both lists.

### P3 — `package = "..."` rename forgotten

`gst = { package = "gstreamer", version = "0.25" }` — the `package`
key renames the crate locally from `gst` (the alias) to `gstreamer`
(the actual crate). Without `package = ...`, Cargo looks for a
crate literally named `gst` and fails to find it. Same for
`gst-app`, `gst-video`, `gst-base`, `gst_rs_webrtc`.

### P4 — `gst_rs_webrtc` is already explicit; don't double-inline

`gst_rs_webrtc` in the original `Cargo.toml` is already version-
pinned (`version = "0.15"`); it's not a workspace dep. Don't
"convert" it. Leave it as-is.

### P5 — Forgetting the build-dep

`slint-build` is in `[build-dependencies]`, not `[dependencies]`.
The version block goes in the right section:

```toml
[build-dependencies]
slint-build = "1.16.0"
```

### P6 — Adding a `[workspace.dependencies]` of your own

A new repo with a `[workspace]` table can declare
`[workspace.dependencies]` and re-introduce the workspace = true
pattern. **Don't** for a single-crate repo. It adds indirection
without a benefit. The android-sender's flat `[dependencies]` is
shorter and easier to audit.

### P7 — `Cargo.lock` not committed

If STEP-2's `.gitignore` had a default-Rust template that ignores
`Cargo.lock`, this step is the last chance to fix it. Commit the
lock; ship reproducible builds.

### P8 — `edition = "2024"` requirement leaking through

`mcore`'s `Cargo.toml` has `edition = "2024"`. The android-sender
crate has `edition = "2021"`. The lockfile entries for the SDK
crates carry their own edition; this is fine. Don't "fix" the
android-sender's edition to match `mcore`'s — STEP-4 keeps the
android-sender on whatever edition it was on (`2021` per current
master).

---

## 5. Next step

[Step 5 — vendor the Slint cross-tree helper(s)](./MVP-PHASE-10-STEP-5-vendor-slint-helpers.md).

`Cargo.toml` is now standalone, but `slint-build` still fails on
`settings_page.slint:21`. STEP-5 vendors
`sdk/mirroring_core/ui/common.slint` and its transitive imports
into the new repo's `ui/` tree.

# MVP-PHASE-10 — Step 5: vendor the cross-tree Slint helpers

> Part 5 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 4 — standalone Cargo.toml](./MVP-PHASE-10-STEP-4-standalone-cargo-toml.md).
>
> **This is the step the user's research missed.** See parent §1.4
> for the correction.

---

## 0. Goal

Vendor (copy) the Slint files outside `senders/android/ui/` that the
Android sender imports, then rewrite `settings_page.slint:21`'s
import path to point at the local copy. After STEP-5,
`slint-build`'s `compile()` invocation finds every transitive
import without crossing the repo boundary.

After STEP-5:

- `cargo +nightly check --target aarch64-linux-android` no longer
  errors out on `slint-build` (it may still fail at link stage if
  `GSTREAMER_ROOT_ANDROID` is unset — that's documented, not a
  STEP-5 concern).
- The Slint UI compiles cleanly; running `slint-viewer` or the
  `ui-validate.sh` script also succeeds (when the script is in
  place; see STEP-6).
- The new repo contains a `ui/components/mcore/` directory
  containing the vendored helpers.

---

## 1. Pre-flight

### 1.1 What needs vendoring

From STEP-1 §2.4 audit. The transitive Slint import graph rooted at
`senders/android/ui/`:

```
senders/android/ui/pages/settings_page.slint
   └── (line 21) sdk/mirroring_core/ui/common.slint
                    └── (line 1) senders/ui-components/std-widgets.slint
                                     └── (re-exports from various sibling
                                          .slint files in
                                          senders/ui-components/)
```

The minimum vendor set is **two files**: `common.slint` (the file
directly imported by settings_page) and `std-widgets.slint` (the
re-export hub it imports).

But `senders/ui-components/std-widgets.slint` re-exports from many
sibling files. Let's enumerate:

```bash
grep -nE '^(export|import)' senders/ui-components/std-widgets.slint
```

Likely output (verify on master HEAD):

```
1: imports from std-widgets-impl.slint
4: re-exports from std-widgets-impl.slint   # StyleMetrics, ScrollView, Button, Palette
6-13: re-exports from sibling files         # checkbox, combobox, lineedit, …
```

So the **full** transitive vendor set is **every** file in
`senders/ui-components/` that's reachable from the entry point. Run
the audit:

```bash
grep -rnE 'from "[^"]+\.slint"' senders/ui-components/ \
    | grep -oE '"[^"]+\.slint"' | sort -u
```

For the current monorepo HEAD, expect ~15-20 files. The icons
folder (`senders/ui-components/icons/`) is also referenced
indirectly via `@image-url("icons/...")` — those resource paths are
resolved at slint-build time relative to the importing file. Vendor
the entire `icons/` folder too.

The fonts folder (`senders/ui-components/fonts/`) — check whether
any Slint file references it via `@font-face` / `@font-files` (a
Slint feature). If yes, vendor.

### 1.2 Choosing the vendor target path

Two viable target paths inside the new repo:

| Target | Pros | Cons |
|---|---|---|
| `ui/components/mcore/` | Mirrors the source layout (`sdk/mirroring_core/ui/`); easy to mentally map. | Confusing name (the dir is from `mcore` SDK, but the SDK isn't called `mcore` everywhere). |
| `ui/vendor/` | Generic name; clear "this isn't ours" signal. | Doesn't preserve the source structure. |

**Default: `ui/components/mcore/`** (mirror-the-source). Keep the
filename `common.slint` for the file from
`sdk/mirroring_core/ui/common.slint`. Place
`senders/ui-components/*` into `ui/components/std/` (the "std" hint
matches the file's name `std-widgets.slint`).

### 1.3 Tools needed

- `cp -a` for the copy.
- `sed` (or your editor) for the import-path rewrite.
- `slint-viewer` (optional) for a quick local preview.

---

## 2. The change

### 2.1 Vendor `sdk/mirroring_core/ui/`

```bash
cd /tmp/new-repo
mkdir -p ui/components/mcore
cp -a /path/to/kodyka-fcast/sdk/mirroring_core/ui/common.slint \
    ui/components/mcore/common.slint
git add ui/components/mcore/common.slint
```

### 2.2 Vendor `senders/ui-components/`

```bash
cd /tmp/new-repo
mkdir -p ui/components/std
cp -a /path/to/kodyka-fcast/senders/ui-components/. \
    ui/components/std/
git add ui/components/std/
```

Use `cp -a` (preserves perms, timestamps, follows the trailing dot
convention from STEP-2 §2.3).

### 2.3 Rewrite the import paths

The vendored files now live at different relative locations. Two
edits:

#### Edit 1: `ui/pages/settings_page.slint:21`

**Before:**

```slint
import { Utils, VideoResolutionPicker, FrameratePicker }
    from "../../../../sdk/mirroring_core/ui/common.slint";
```

**After:**

```slint
import { Utils, VideoResolutionPicker, FrameratePicker }
    from "../components/mcore/common.slint";
```

#### Edit 2: `ui/components/mcore/common.slint:1`

**Before:**

```slint
import { ComboBox } from "../../../senders/ui-components/std-widgets.slint";
```

**After:**

```slint
import { ComboBox } from "../std/std-widgets.slint";
```

#### Edit 3+ (only if needed): inside `ui/components/std/*.slint`

If any of the vendored `std/` files import from
`senders/ui-components/` siblings (which they will, internally),
the relative paths between them are **already correct** because
they were preserved by `cp -a`. The only files that need import
rewrites are the **entry points** of the vendored graph (Edit 1 and
Edit 2 above).

To be safe, scan the vendored tree:

```bash
grep -rnE 'from "[^"]+(sdk|crates|senders)' ui/components/
# → expect 0 matches after Edit 2.
```

If non-zero matches remain, rewrite each to point at a vendored
sibling.

### 2.4 Watch for `@image-url` resource paths

The vendored Slint files may use `@image-url("icons/tv.svg")`. Slint
resolves these relative to the .slint file's own location. Since
the icons subfolder was vendored alongside (`ui/components/std/icons/`),
the paths still resolve correctly **provided** `icons/` is in the
right relative position. Verify:

```bash
ls ui/components/std/icons/ | head    # → expected: tv.svg, down.svg, etc.
```

If `cp -a` flattened the folder or skipped it, redo §2.2.

### 2.5 Don't forget `@font-face` / font files

`std-widgets.slint` may declare custom fonts via `@font-face` or
`@font-files`. Check:

```bash
grep -rnE '@font-(face|files)' ui/components/std/
```

Each declaration is a path relative to the .slint file. Vendor the
referenced `.ttf` / `.otf` / `.woff` files into a sibling folder if
they aren't already in `ui/components/std/fonts/`.

### 2.6 Document the vendor source

Add a `ui/components/mcore/README.md` and `ui/components/std/README.md`
(or one combined `ui/components/VENDORING.md`) so the source is
discoverable:

```markdown
# Vendored Slint helpers

These are copies of Slint files from the FCast monorepo
(`kodyka/fcast`). They are vendored because Slint imports must be
resolved at compile time and Slint doesn't support importing from
a Cargo dependency.

| File / dir | Source (in kodyka/fcast at SHA <STEP-1-§1.1-SHA>) |
|---|---|
| `mcore/common.slint` | `sdk/mirroring_core/ui/common.slint` |
| `std/` | `senders/ui-components/` |

To re-vendor (after a Slint helper change in the monorepo):

```bash
cp /path/to/kodyka-fcast/sdk/mirroring_core/ui/common.slint \
    ui/components/mcore/common.slint
cp -a /path/to/kodyka-fcast/senders/ui-components/. \
    ui/components/std/
```

Then re-apply the local edits noted in
[`MVP-PHASE-10-STEP-5-vendor-slint-helpers.md`](https://github.com/kodyka/fcast/blob/master/draft/slint-ui/phases/MVP-PHASE-10-STEP-5-vendor-slint-helpers.md) §2.3.
```

STEP-9 details the re-vendor cadence.

---

## 3. Verification

### 3.1 No cross-tree imports remain

```bash
cd /tmp/new-repo
grep -rnE 'from "(\.\./)+(sdk|crates|senders)' ui/
# → expect 0 matches.
```

### 3.2 `cargo check` no longer hits a slint-build path error

```bash
cargo +nightly check --target aarch64-linux-android 2>&1 | tail -10
```

Expected:

- No `error: cannot resolve import "../../../../sdk/..."` line.
- The build may still fail later (linker can't find
  `gstreamer_android` because `GSTREAMER_ROOT_ANDROID` isn't set,
  for example). That's a STEP-6 / STEP-7 concern — Slint is happy.

### 3.3 Live preview (optional)

If `slint-viewer` is installed:

```bash
slint-viewer ui/main.slint
```

Watch for errors mentioning missing imports. If the viewer renders
something (or fails with a runtime error like "Bridge global not
set" — fine, that's expected from running outside the host app), the
import graph is healthy.

### 3.4 Run `ci/ui-validate.sh` (if it doesn't require Cargo)

```bash
cd /tmp/new-repo
bash ci/ui-validate.sh
```

If the script just runs `slint-build` or `slint-viewer --print-stats`,
this is the highest-signal check. If it does more (compile the
whole crate), expect it to fail until STEP-6/7 — but a slint-only
subset should pass.

---

## 4. Pitfalls specific to this step

### P1 — Vendoring `common.slint` but forgetting `std-widgets.slint`

`common.slint` imports `ComboBox` from `std-widgets.slint`. If you
vendor only `common.slint` and leave `std-widgets.slint` as a
cross-tree import, you've moved the bug, not fixed it.

Always walk the **full** transitive set; STEP-1 §2.4 enumerates it.

### P2 — `cp -a` skipping dot files

`cp -a senders/ui-components/.` (with trailing dot) copies dotfiles
too. `cp -a senders/ui-components/*` (with glob) typically does not
in bash, depending on `dotglob` setting. Use the trailing-dot form.

### P3 — Forgetting `icons/` or `fonts/` subfolders

The icons and fonts referenced via `@image-url` and `@font-face`
are loaded at slint-build time relative to the importing file.
Skipping them means runtime "image not found" errors with
confusing messages. Verify via §2.4.

### P4 — Rewriting too much / too little

Edit 1 (settings_page.slint) and Edit 2 (common.slint) are the
only **required** rewrites. Don't rewrite anything inside
`ui/components/std/` — those files reference each other via
relative paths that were already correct when `cp -a` preserved
them.

If you "helpfully" rewrite every import to add `components/std/`
prefix, you'll break the internal references. Run §3.1 to catch.

### P5 — Updating Slint version-with-features without updating vendor

If a future PR bumps the Slint version (say to 1.17) in the new
repo's `Cargo.toml`, the vendored Slint helpers may need updating
too (Slint sometimes deprecates / renames built-in widgets between
minor versions). After a Slint bump:

```bash
cargo +nightly check --target aarch64-linux-android 2>&1 | grep -E 'deprecated|error'
```

If errors appear inside vendored files, re-vendor from the
monorepo's matching Slint-bumped commit.

### P6 — Treating the vendored copies as "stable forever"

The vendored Slint files diverge from the monorepo over time. STEP-9
documents the cadence; in the worst case the divergence accumulates
and a future re-vendor is painful. Mitigate by re-vendoring on
every monorepo SDK PR that touches `sdk/mirroring_core/ui/` or
`senders/ui-components/`. Add a `CODEOWNERS` rule or doc to make
this discoverable.

---

## 5. Next step

[Step 6 — copy CI, Gradle wrapper, Dockerfile, and document build.rs env](./MVP-PHASE-10-STEP-6-ci-gradle-buildrs.md).

`cargo` and `slint-build` are now happy (modulo native libs).
STEP-6 deals with the CI, Gradle, and the NDK / GStreamer env
documentation that the next maintainer will need.

# MVP-PHASE-10 — Step 3: resolve in-monorepo path dependencies

> Part 3 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 2 — bootstrap & move](./MVP-PHASE-10-STEP-2-bootstrap-new-repo.md).
> Default strategy: **Option A — Git dep with subpath**. Alternatives (B publish, C submodule) covered in §2.5-2.6.

---

## 0. Goal

Rewrite the three `path = ...` dependencies in the new repo's
`Cargo.toml`:

| Crate | Before (post-STEP-2) | After (Option A) |
|---|---|---|
| `fcast-protocol` | `{ path = "../../sdk/common/fcast-protocol" }` | `{ git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/common/fcast-protocol" }` |
| `fcast-sender-sdk` | `{ path = "../../sdk/sender/fcast-sender-sdk", default-features = false, features = ["fcast"] }` | `{ git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/sender/fcast-sender-sdk", default-features = false, features = ["fcast"] }` |
| `mcore` | `{ path = "../../sdk/mirroring_core/" }` | `{ git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/mirroring_core" }` |

After STEP-3:

- `cargo metadata` resolves the three deps to commits on
  `kodyka/fcast`.
- The first `cargo +nightly fetch` populates the Cargo cache with
  a full clone of `kodyka/fcast` (Cargo does not support shallow Git
  deps — see pitfall P3).
- The new repo's `Cargo.lock` has new entries referencing the Git
  source.

This step does **not** yet make the new repo build (the
workspace-deps still need inlining in STEP-4, and the Slint
cross-tree import still needs vendoring in STEP-5). It only fixes
the three path deps.

---

## 1. Pre-flight

### 1.1 Inputs from previous steps

| Input | Source |
|---|---|
| Source commit SHA | STEP-1 §1.1 |
| Chosen strategy | STEP-1 §2.7 (default: A) |
| Working tree | The new repo on disk (post-STEP-2 commit) |

### 1.2 Verify the SHA is reachable

```bash
git ls-remote https://github.com/kodyka/fcast <SHA>
# → expected: 1 line of output. If 0 lines, the SHA was force-pushed
#   away from a branch; pick the next commit.
```

The SHA must be reachable from a branch or tag on the remote (Cargo
clones the full reachable graph; an orphaned commit fails to
resolve).

### 1.3 Authentication

`kodyka/fcast` is a public repo. Anonymous `git clone https://...`
works. If you switched the monorepo to private at some point, see
§2.4 ("Private SDK repo authentication").

---

## 2. The change

### 2.1 Option A — Git dep with subpath (default)

**File:** `Cargo.toml` (new repo root).

**Before** (post-STEP-2):

```toml
fcast-protocol = { path = "../../sdk/common/fcast-protocol" }
fcast-sender-sdk = { path = "../../sdk/sender/fcast-sender-sdk", default-features = false, features = [ "fcast" ] }
mcore.path = "../../sdk/mirroring_core/"
```

**After:**

```toml
fcast-protocol = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/common/fcast-protocol" }
fcast-sender-sdk = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/sender/fcast-sender-sdk", default-features = false, features = [ "fcast" ] }
mcore = { git = "https://github.com/kodyka/fcast", rev = "<SHA>", path = "sdk/mirroring_core" }
```

Notes:

- The `path = "..."` inside the Git table is a Cargo feature
  (`path` within a `git` source) that selects a sub-directory of
  the cloned repo. **All three deps must have the same `git` URL +
  `rev`** so Cargo treats them as a single Git source and clones
  the repo exactly once.
- Replace `<SHA>` with the 40-char commit SHA from STEP-1 §1.1.
  Do **not** use a branch name; see pitfall P4.
- Switch `mcore.path = "..."` (inline-key form) to the table form
  so all three look identical. The semantics are the same.

### 2.2 First `cargo fetch`

```bash
cd /tmp/new-repo
cargo +nightly fetch
```

Expected output (excerpt):

```
Updating git repository `https://github.com/kodyka/fcast`
Updating crates.io index
   Downloaded ...
```

This populates `~/.cargo/git/db/kodyka-fcast-<hash>/` with a bare
clone of the monorepo. On subsequent fetches, Cargo updates the
clone instead of re-cloning.

`cargo fetch` does **not** yet compile anything. It just resolves
the dependency graph and downloads sources. If it fails, the
likely causes are:

- Network unreachable (firewall).
- The SHA is not reachable (see §1.2).
- A workspace-dep is still `workspace = true` (STEP-4 fixes this —
  STEP-3 may need to run after STEP-4 if `cargo fetch` is unhappy).
  Easiest: run STEP-4 first if you hit "workspace dependency not
  found" errors.

### 2.3 Lockfile update

```bash
cat Cargo.lock | grep -A1 'name = "fcast-protocol"'
# → expect "source" line referencing git+https://github.com/kodyka/fcast?rev=<SHA>
```

This is the canonical record of the pin. STEP-9's bump procedure
edits `Cargo.toml`'s `rev = "<SHA>"`, then `cargo update -p
fcast-protocol -p fcast-sender-sdk -p mcore` rewrites the lockfile.

### 2.4 Private SDK repo authentication (skip if monorepo is public)

If `kodyka/fcast` became private after PHASE-10 starts:

#### Option (a) — HTTPS with a personal access token

```bash
# Once, on the dev machine:
git config --global url."https://oauth2:${GITHUB_TOKEN}@github.com/".insteadOf "https://github.com/"
```

Cargo respects `~/.gitconfig`. CI needs the same configuration
applied via secrets (GitHub Actions: `${{ secrets.GITHUB_TOKEN }}`;
Gitlab CI: `CI_JOB_TOKEN`).

#### Option (b) — SSH

```bash
# Once, on the dev machine + CI:
git config --global url."ssh://git@github.com/".insteadOf "https://github.com/"
ssh-add ~/.ssh/id_ed25519
```

Both options leave `Cargo.toml` unchanged — Cargo always resolves
`https://github.com/...` URLs; the `insteadOf` config rewrites
them at clone time.

### 2.5 Option B — Publish to crates.io (alternative)

This is **not** the default. Only do this if you have a release
plan (STEP-1 §2.7 pitfall P5).

If you do go this way:

1. In `kodyka/fcast`, bump each SDK crate's version and run
   `cargo publish -p fcast-protocol`, `cargo publish -p
   fcast-sender-sdk`, `cargo publish -p mcore`. Order matters:
   `fcast-protocol` first (no deps), then `fcast-sender-sdk` and
   `google-cast-protocol`, then `mcore`.
2. In the new repo's `Cargo.toml`:
   ```toml
   fcast-protocol = "0.1.3"
   fcast-sender-sdk = { version = "0.1.3", default-features = false, features = ["fcast"] }
   mcore = "0.1.0"
   ```
3. `cargo update`.

The crates.io ecosystem now expects semver — bumping
`fcast-protocol` to `0.2.0` is a breaking-change signal. The new
repo's `^0.1` requirement won't accept it without an explicit bump.

`google-cast-protocol` does **not** need publishing (not pulled in
on Android; see parent §1.3).

`app-updater` does **not** need publishing (not pulled in on
Android target).

### 2.6 Option C — Vendor as Git submodule (alternative)

Also **not** the default. Pick this only if STEP-1 §2.7 explicitly
chose C.

```bash
cd /tmp/new-repo
mkdir -p vendor
git submodule add https://github.com/kodyka/fcast vendor/fcast
cd vendor/fcast
git checkout <SHA>
cd ../..
git add .gitmodules vendor/fcast
```

Then in `Cargo.toml`:

```toml
fcast-protocol = { path = "vendor/fcast/sdk/common/fcast-protocol" }
fcast-sender-sdk = { path = "vendor/fcast/sdk/sender/fcast-sender-sdk", default-features = false, features = ["fcast"] }
mcore = { path = "vendor/fcast/sdk/mirroring_core" }
```

CI must run `git submodule update --init --recursive` after clone.
See STEP-6.

---

## 3. Verification

### 3.1 `cargo metadata` resolves the three deps to a Git source

```bash
cd /tmp/new-repo
cargo metadata --format-version=1 \
    | jq '.packages[] | select(.name == "fcast-protocol" or .name == "fcast-sender-sdk" or .name == "mcore") | {name, source}'
```

Expected output (Option A):

```json
{"name":"fcast-protocol","source":"git+https://github.com/kodyka/fcast?rev=<SHA>#<SHA-prefix>"}
{"name":"fcast-sender-sdk","source":"git+https://github.com/kodyka/fcast?rev=<SHA>#<SHA-prefix>"}
{"name":"mcore","source":"git+https://github.com/kodyka/fcast?rev=<SHA>#<SHA-prefix>"}
```

All three sources should have the **same** `<SHA-prefix>` (the
suffix Cargo appends). Different prefixes mean Cargo cloned the
Git repo multiple times — usually a sign that one of the three
`git = ...` URLs is misspelled (e.g. trailing `.git`, different
case).

### 3.2 `Cargo.lock` references the Git source

```bash
grep -A2 'name = "fcast-protocol"' Cargo.lock
```

Expected:

```toml
[[package]]
name = "fcast-protocol"
source = "git+https://github.com/kodyka/fcast?rev=<SHA>#<SHA-prefix>"
```

If `source` is missing or starts with `registry+`, you're in Option
B (registry) — confirm that's what you picked.

### 3.3 No `cargo check` yet

STEP-3 alone is insufficient — workspace-deps from STEP-4 still need
inlining and the Slint helper still needs vendoring in STEP-5. Do
**not** run `cargo check` yet. If you must (e.g. for sanity), expect
errors about `workspace = true` not being found — that's STEP-4's
job to fix.

---

## 4. Pitfalls specific to this step

### P1 — Different `git` URLs for the three deps

If you accidentally write
`git = "https://github.com/kodyka/fcast"` for one and
`git = "https://github.com/kodyka/fcast.git"` for another (note the
trailing `.git`), Cargo treats them as two different Git sources
and clones the monorepo twice. Use the **same** URL for all three.

### P2 — `branch = "..."` instead of `rev = "..."`

`branch = "master"` is not a pin — it's a request for "whatever is
currently at the tip of master". Builds become non-reproducible.
Always pin to `rev = "<40-char-SHA>"`.

### P3 — Expecting shallow Cargo Git fetches

Cargo does **not** support shallow Git deps. The first `cargo fetch`
clones the **full** `kodyka/fcast` history (~50 MB at the time of
writing). This is a one-time cost per Cargo home; it's not free but
it's acceptable.

If the monorepo's history grows to GB-scale, consider publishing
(Option B) — Cargo's registry source does support partial fetches.

### P4 — Forgetting to update `Cargo.lock`

After editing `Cargo.toml`, `cargo fetch` updates `Cargo.lock`. If
you skip `cargo fetch` and commit only the `Cargo.toml` change,
the lockfile still references the path source. The next `cargo
check` will appear to work locally (because Cargo updates the lock
silently), but CI will see a stale lockfile and may reject it (some
projects run `cargo check --locked` in CI).

Always commit `Cargo.toml` **and** the updated `Cargo.lock`
together.

### P5 — Committing GitHub tokens via `insteadOf`

The `git config insteadOf` URL rewrite (§2.4 option a) embeds the
token into rewritten URLs. If you `git config --local` it in a
repo, the token may end up in `.git/config`. Use
`git config --global` and **never** commit `~/.gitconfig`.

In CI, never echo `$GITHUB_TOKEN` to logs. Mask it via the CI
provider's secret-handling.

### P6 — The `mcore` inline-key form

The pre-STEP-3 `Cargo.toml` has `mcore.path = "..."` (inline-table-
key form) while the other two use full inline-table form. STEP-3
rewrites all three to inline-table form for consistency. The
semantics are identical, but mixing forms makes `git diff` reviews
harder.

### P7 — Path subspec assumes Cargo ≥ 1.74

The `{ git = "...", rev = "...", path = "..." }` subspec syntax
needs Cargo 1.74 (released Nov 2023). The nightly toolchain pinned
by the project is newer than this. If a CI worker is pinned to a
very old toolchain (e.g. 1.65), the subspec form falls back to
"path is ignored, dep resolves to the workspace root", which is
almost certainly not what you want. Confirm `cargo --version` in
CI is ≥ 1.74.

---

## 5. Next step

[Step 4 — standalone `Cargo.toml` (inline workspace deps)](./MVP-PHASE-10-STEP-4-standalone-cargo-toml.md).

The path deps now point at `kodyka/fcast` via Git. STEP-4 makes the
new repo's `Cargo.toml` actually compile by inlining every
`workspace = true` reference with an explicit version + features.

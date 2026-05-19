# MVP-PHASE-10 — Step 8: remove `senders/android/` from the monorepo

> Part 8 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 7 — first build + verification](./MVP-PHASE-10-STEP-7-first-build-verification.md).
>
> **Irreversible-ish step.** Only run this after STEP-7 §3.1-3.7
> are all green. STEP-7 §3.8 (size comparison) and §3.6 (e2e cast)
> are also strongly recommended.

---

## 0. Goal

Delete `senders/android/` from `kodyka/fcast`, remove its
workspace-member entry from the root `Cargo.toml`, regenerate
`Cargo.lock`, and update any monorepo-side CI references that
mention `senders/android`.

After STEP-8:

- `senders/android/` is gone from the monorepo's working tree.
- The root `Cargo.toml` no longer lists `"senders/android"` (twice
  — there's currently a duplicate entry; both go).
- `Cargo.lock` no longer has entries scoped to the android-sender
  crate.
- `cargo check --workspace` in the monorepo is clean.
- The monorepo's `.gitlab-ci.yml` `include:` list no longer
  references `senders/android/.gitlab-ci.yml`.
- The monorepo's `.github/workflows/android-release-apk.yml` is
  either deleted **or** redirected at the new repo via
  `repository_dispatch` (decided in §1.3).

This is **one PR against `kodyka/fcast`**. Do not split it; the
state between "Cargo.toml updated" and "directory deleted" is a
broken workspace.

---

## 1. Pre-flight

### 1.1 STEP-7 must be green

| Check | Required? |
|---|---|
| STEP-7 §3.1 (cargo check) | Yes |
| STEP-7 §3.2 (cargo build release) | Yes |
| STEP-7 §3.3 (Gradle debug APK) | Yes |
| STEP-7 §3.4 (install + launch) | Yes |
| STEP-7 §3.5 (PHASE-9 quick-actions) | Yes |
| STEP-7 §3.6 (e2e cast) | **Strongly recommended.** If untested, document the risk in the STEP-8 PR description. |
| STEP-7 §3.7 (CI green on new repo) | Yes |
| STEP-7 §3.8 (APK size comparison) | Recommended. |

### 1.2 Live state in the monorepo's root `Cargo.toml`

The current `[workspace]` block has `"senders/android"` listed **twice**:

```toml
[workspace]
members = [
    ...
    "senders/desktop",
    "senders/android",
    "tools/fast",
    "sdk/file-server",
    "senders/android",       # ← duplicate (pre-existing bug)
    ...
]
```

STEP-8 removes **both** entries. This is a quality-of-life fix on
top of the extraction; the duplicate has been benign because
Cargo deduplicates workspace members, but removing it now keeps
the post-PHASE-10 root tidy.

### 1.3 Decide: delete the monorepo's GHA workflow or repurpose?

The monorepo's `.github/workflows/android-release-apk.yml` runs on
**every** push to the monorepo. After STEP-8, the workflow has no
android sender to build — the source tree is gone. Three options:

| Option | Pros | Cons |
|---|---|---|
| **A. Delete the workflow** | Simplest; matches "this code lives in the new repo now". | Loses the "monorepo CI also builds android" safety net during SDK changes. |
| **B. Replace with a `repository_dispatch` trigger** that fires the new repo's android-release-apk workflow when an SDK PR merges. | Keeps SDK changes signalling android-side breakage. | Requires a Personal Access Token / GitHub App; more moving parts. |
| **C. Replace with a one-shot job** that clones the new repo, bumps the SDK pin to the current monorepo SHA, and runs the new repo's CI. | Highest-signal — actually proves the SDK change doesn't break the consumer. | Slow; complex CI YAML; cross-repo state machine. |

**Default: A (delete).** STEP-9 documents the manual pin-bump
procedure that catches breakage post-merge. Options B and C are
worth revisiting in a future PHASE if SDK ↔ android-side
breakages become common.

### 1.4 Tools

- A clean checkout of `kodyka/fcast` `master`.
- `cargo` (any recent version).
- Push permissions to the monorepo.

---

## 2. The change

This is one Git commit on a feature branch. Don't split.

### 2.1 Branch off `master`

```bash
cd /path/to/kodyka-fcast
git checkout master && git pull
git checkout -b devin/phase-10-step-8-remove-android-from-monorepo
```

### 2.2 Delete the directory

```bash
git rm -r senders/android/
git status --short | head    # → "D" lines for the deleted files
```

### 2.3 Edit root `Cargo.toml`

Remove **both** `"senders/android",` lines:

```diff
 [workspace]
 members = [
     ...
     "senders/desktop",
-    "senders/android",
     "tools/fast",
     "sdk/file-server",
-    "senders/android",
     "receivers/experimental/rcore",
     ...
 ]
```

After this edit, `members` should not contain any `senders/android`
reference. Verify:

```bash
grep -n 'senders/android' Cargo.toml
# → 0 matches.
```

### 2.4 Regenerate `Cargo.lock`

```bash
cargo check --workspace 2>&1 | tail
```

`cargo check` rewrites `Cargo.lock` automatically. Confirm the
lockfile no longer contains `android-sender`:

```bash
grep -E '^name = "android-sender"' Cargo.lock
# → 0 matches.
```

If `cargo check` still references android-sender (e.g. a `tools/`
crate dev-deps it for some reason), the workspace still has a
dependent. Resolve that first.

### 2.5 Delete the monorepo's Gitlab CI include

In `.gitlab-ci.yml`:

```diff
 include:
   - local: 'docs/.gitlab-ci.yml'
   ...
   - local: 'sdk/sender/.gitlab-ci.yml'
-  - local: 'senders/android/.gitlab-ci.yml'
   - local: 'senders/desktop/.gitlab-ci.yml'
   - local: 'website/.gitlab-ci.yml'
```

The included file was already deleted with `git rm -r` in §2.2;
the `include:` reference becomes a dangling pointer until this
edit.

### 2.6 Delete (or rewire) the monorepo's GHA workflow

Per §1.3 decision (default: delete):

```bash
git rm .github/workflows/android-release-apk.yml
```

If you chose option B or C, replace its contents instead of
deleting. The workflow filename can stay the same.

### 2.7 Search for any other lingering references

```bash
# Any path inside the monorepo that mentions senders/android.
grep -rn 'senders/android' . \
    --include='*.toml' --include='*.yml' --include='*.yaml' \
    --include='*.md' --include='*.sh' --include='*.rs' \
    | grep -v '^./draft/'   # exclude the doc tree (those are
                              # historical references — they stay).
```

For each match outside the doc tree:
- If it's a `path = "senders/android/..."` in some Cargo.toml,
  remove the entry (it's now broken).
- If it's a script (`cd senders/android && ...`), remove the
  script step.
- If it's a README link, update or remove.

**Don't** touch the doc tree (`draft/slint-ui/phases/`). The
PHASE-1..10 docs are historical and reference `senders/android/`
paths intentionally — those are the records of what was. Leave
them.

### 2.8 Update top-level README if it links to senders/android

```bash
grep -n 'senders/android' README.md 2>/dev/null
```

If the monorepo's README has a section pointing at
`senders/android/` (e.g. "Android sender lives at
`senders/android/`"), rewrite to point at the new repo:

```markdown
- **Android sender**: lives in a separate repository,
  [kodyka/fcast-android-sender](https://github.com/kodyka/fcast-android-sender).
```

### 2.9 Commit

```bash
git add -A
git commit -m "extract: remove senders/android (now lives at kodyka/fcast-android-sender)

The Android sender (Slint UI + migration runtime) now lives in its
own repository at https://github.com/kodyka/fcast-android-sender,
extracted at commit <STEP-1-§1.1-SHA> per MVP-PHASE-10.

Changes:
- Delete senders/android/ tree.
- Remove duplicate \"senders/android\" workspace member from root
  Cargo.toml.
- Drop senders/android/.gitlab-ci.yml include.
- Delete .github/workflows/android-release-apk.yml (now lives in
  the new repo).
- Update README link to point at the new repo.

The four SDK crates the android sender consumed
(fcast-protocol, fcast-sender-sdk, mcore, google-cast-protocol)
remain in this repo. The new repo pins them via Git deps with the
'path' subspec.

See draft/slint-ui/phases/MVP-PHASE-10-android-sender-repo-extraction.md
for the full extraction guide."
```

### 2.10 Push and open PR

```bash
git push -u origin devin/phase-10-step-8-remove-android-from-monorepo
gh pr create --base master --title "extract: remove senders/android (now lives at kodyka/fcast-android-sender)" \
    --body-file /tmp/step-8-pr-body.md
```

The PR body should mirror the commit message **plus** the STEP-7
verification record (which §3.1-3.7 passed, which §3.8 found, any
known issues).

---

## 3. Verification

### 3.1 `cargo check --workspace` is clean

```bash
cd /path/to/kodyka-fcast      # on the PHASE-10-STEP-8 branch
cargo check --workspace 2>&1 | tail
echo $?    # → 0
```

### 3.2 `cargo build --release --workspace` succeeds

```bash
cargo build --release --workspace 2>&1 | tail
```

Watch for any crate that fails because it depended on
`android-sender` directly (none should — `android-sender` is a
cdylib, no other crate consumes it). If you do see a failure,
diagnose and fix in the same PR.

### 3.3 No `senders/android` references in non-doc tree

```bash
grep -rn 'senders/android' . \
    --include='*.toml' --include='*.yml' --include='*.yaml' \
    --include='*.md' --include='*.sh' --include='*.rs' \
    | grep -v '^./draft/'
```

Expected: 0 matches (or only matches inside files you intentionally
left, e.g. a CHANGELOG entry).

### 3.4 Monorepo CI passes

After pushing the branch:

```bash
gh pr checks <PR-number>
```

Both `ui-validate` and `build-android-arm64-debug` jobs should
**not run** (the workflow file was deleted in §2.6). Other CI
jobs (receiver-side, sdk, desktop sender) should still pass.

### 3.5 Cross-check: the new repo still works

After STEP-8 lands on the monorepo's `master`, the new repo
**must still build**. The new repo's Git-dep pin points at a SHA
**before** STEP-8 (`STEP-1-§1.1-SHA`); Cargo clones that historical
state, which still has the SDK crates. STEP-8 doesn't move or
rename the SDK crates — only `senders/android/` is touched. So
the new repo's CI is unaffected by STEP-8.

Verify by re-running the new repo's CI after STEP-8 merges:

```bash
cd /tmp/new-repo
gh workflow run android-release-apk.yml --ref main
# Watch the run; expected: green.
```

### 3.6 PR sanity checklist

Before requesting a merge:

- [ ] STEP-7 §3.1-3.7 all green (PR body lists them).
- [ ] §3.6 (e2e cast) either tested or risk-acknowledged in PR
  body.
- [ ] Diff stat is "as expected" — only `senders/android/` and a
  few config files. No accidental sweeping changes elsewhere.
- [ ] `cargo check --workspace` reproduces clean on a fresh
  checkout of the branch.

---

## 4. Pitfalls specific to this step

### P1 — Splitting the delete across multiple PRs

Don't. The intermediate state ("Cargo.toml updated, directory
still present" or vice versa) leaves the workspace inconsistent.
One commit; one PR.

### P2 — Running STEP-8 before STEP-7 §3.6 passes

If the e2e cast hasn't been exercised, you don't know if PHASE-1..8
behaviour transferred to the new repo. STEP-8 is irreversible-ish;
do not race past STEP-7.

### P3 — Leaving the GHA workflow in place "just in case"

The monorepo's `android-release-apk.yml` workflow tries to build a
directory that no longer exists. After STEP-8, every push to the
monorepo fails this workflow → red CI noise. Either delete it
(default) or rewire it (§1.3 options B/C). Don't leave it dangling.

### P4 — Not regenerating `Cargo.lock`

If you only edit `Cargo.toml` and don't run `cargo check`, the
lockfile still has `android-sender` entries. Some PR checks
verify `Cargo.lock` is in sync (`cargo metadata --locked`); they
will fail.

### P5 — Forgetting to update `Cargo.lock` for transitive deps

Removing `android-sender` from the workspace also removes its
transitive dependencies (the ones nothing else needs). `cargo
check` should prune them automatically; if it doesn't, run
`cargo update --workspace` to refresh.

### P6 — Deleting the doc tree references

The `draft/slint-ui/phases/MVP-PHASE-*.md` files reference
`senders/android/` paths extensively. Those are **historical**
records of what was — leave them. Future maintainers will read
PHASE-1..10 as the story of how this happened.

### P7 — Accidentally deleting `sdk/sender/` too

The SDK crates (`sdk/sender/fcast-sender-sdk`,
`sdk/common/fcast-protocol`, etc.) **stay in the monorepo**. Only
`senders/android/` is deleted. Double-check the `git rm` target:

```bash
git rm -r senders/android/        # right
git rm -r sdk/sender/             # wrong — would break everything
```

### P8 — Not bumping the SDK version after the extraction lands

The new repo pins the SDK to `<STEP-1-§1.1-SHA>`. After STEP-8
lands, the SDK keeps evolving in the monorepo. STEP-9 documents
the bump cadence. Don't expect the new repo to magically pick up
SDK changes — they're a separate, intentional bump.

---

## 5. Next step

[Step 9 — long-term cross-repo sync workflow](./MVP-PHASE-10-STEP-9-cross-repo-sync.md).

The extraction is technically complete. STEP-9 documents the
day-to-day workflow: how do SDK changes in the monorepo reach
`fcast-android-sender`? How does someone with a feature PR open
the right pair of PRs?

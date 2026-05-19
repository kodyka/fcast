# MVP-PHASE-10 — Step 9: long-term cross-repo sync workflow

> Part 9 of 9. Parent doc: [`MVP-PHASE-10-android-sender-repo-extraction.md`](./MVP-PHASE-10-android-sender-repo-extraction.md).
> Previous: [Step 8 — remove from monorepo](./MVP-PHASE-10-STEP-8-remove-from-monorepo.md).
>
> **Doc-only step.** Output is a `docs/cross-repo-sync.md` file
> committed to the new repo. No source code changes.

---

## 0. Goal

Document the day-to-day workflow for the post-PHASE-10 world:

- How an SDK PR in `kodyka/fcast` reaches `fcast-android-sender`.
- How a feature PR in `fcast-android-sender` that needs an SDK
  change opens a pair of coordinated PRs.
- How frequently the SDK pin is bumped (cadence policy).
- How to re-vendor the Slint helpers (STEP-5) after an upstream
  change.
- How to handle a release-blocking SDK regression.

The output is a `docs/cross-repo-sync.md` file in the new repo
**and** a section added to the new repo's `CONTRIBUTING.md` (if
one exists). This is the user-facing documentation that
external contributors and the team will reference.

---

## 1. Pre-flight

### 1.1 Strategy chosen in STEP-1 §2.7 affects this step

| Strategy | Pin update | Sync friction |
|---|---|---|
| **A. Git dep with subpath** (default) | Edit `Cargo.toml` `rev = "..."`; run `cargo update -p fcast-protocol -p fcast-sender-sdk -p mcore`. | Low to medium — one PR per bump. |
| **B. Publish to crates.io** | Edit version requirement; run `cargo update`. | Very low (if SDK semver is stable). |
| **C. Submodule** | `cd vendor/fcast && git fetch && git checkout <SHA>`; commit submodule pointer. | High — submodule UX trips contributors. |

The §2 procedure below is for **A (default)**. STEP-9 docs for B/C
diverge in only the bump command; the rest of the policy applies.

### 1.2 Audience

- **External contributors**: docs/cross-repo-sync.md should be
  understandable without internal context.
- **Maintainers**: need a "how to release / how to bump" runbook.

Write the doc for the external contributor; the maintainer-only
nuances go in the new repo's `MAINTAINERS.md`.

---

## 2. The change

Create `docs/cross-repo-sync.md` in the new repo with the
following structure.

### 2.1 The doc skeleton

````markdown
# Cross-repo workflow: `fcast-android-sender` ↔ `kodyka/fcast`

The Android sender depends on three Rust crates that live in
the FCast monorepo (`kodyka/fcast`):

- `fcast-protocol` (`sdk/common/fcast-protocol/`)
- `fcast-sender-sdk` (`sdk/sender/fcast-sender-sdk/`)
- `mcore` (`sdk/mirroring_core/`)

They are pulled in as Git dependencies with the `path` subspec; see
the `[dependencies]` block in `Cargo.toml`. This document describes
how the two repos stay in sync.

## When does the SDK pin get bumped?

- **Routine:** weekly, on a fixed day (suggest: Mondays). One PR
  per week, bumping all three SDK pins to the current
  `kodyka/fcast` `master` HEAD.
- **On-demand:** any SDK change that the Android sender needs
  immediately (security fix, feature dependency).
- **Coordinated:** part of a "PR pair" (see §3).

The cadence is intentional: too-frequent bumps churn the
lockfile; too-infrequent bumps accumulate drift and make individual
bumps risky.

## How to bump the SDK pin

1. Decide the target SHA. Usually `kodyka/fcast master` HEAD:
   ```bash
   git ls-remote https://github.com/kodyka/fcast master
   ```
2. In a `fcast-android-sender` checkout on a feature branch, edit
   `Cargo.toml`:
   ```diff
   - fcast-protocol = { git = "...", rev = "<OLD_SHA>", path = "..." }
   + fcast-protocol = { git = "...", rev = "<NEW_SHA>", path = "..." }
     ... (same for fcast-sender-sdk and mcore)
   ```
   All three `rev = "..."` must match — Cargo treats them as a
   single Git source.
3. Update the lockfile:
   ```bash
   cargo update -p fcast-protocol -p fcast-sender-sdk -p mcore
   ```
4. Sanity-build:
   ```bash
   cargo +nightly check --target aarch64-linux-android
   ```
5. Re-vendor the Slint helpers if upstream touched them (see §4
   "Re-vendoring").
6. Open a PR titled `chore(sdk): bump fcast SDK pin to <SHA-prefix>`
   with the upstream commit range linked in the body:

   ```
   Range: kodyka/fcast@<OLD_SHA>...<NEW_SHA>
   Changes in range: (paste `git log --oneline <OLD>..<NEW>` here)
   ```

7. Watch CI. If green and no behaviour changes are needed, merge.

## How to ship a feature that spans both repos (PR pair)

If your feature requires both an SDK change (in `kodyka/fcast`) and
a consumer-side change (in `fcast-android-sender`), open two PRs:

### Step 1: SDK-side PR (in `kodyka/fcast`)

- Branch: `feat/<feature-name>` off `master`.
- Make the SDK change.
- Land it via the monorepo's normal review process.
- Record the merge SHA.

### Step 2: Consumer-side PR (in `fcast-android-sender`)

- Branch: `feat/<feature-name>` off `main`.
- Bump the SDK pin to the merge SHA from Step 1 (see "How to bump"
  above).
- Add the consumer-side change (UI, handler wiring, etc.).
- The PR body must reference the SDK-side PR with a permalink.

### Step 3: Verify

- `cargo +nightly check` clean.
- CI green.
- Smoke-test on a real device (or document why not).

### Anti-pattern: bumping before the SDK PR lands

Don't open the consumer-side PR pointing at a not-yet-merged SDK
commit. If that SDK PR gets rebased, your `rev = "..."` becomes
unreachable, and CI breaks with a confusing "object not found"
error.

## Re-vendoring the Slint helpers

The new repo vendors two Slint files from the monorepo (see
`ui/components/mcore/README.md`):

- `ui/components/mcore/common.slint` (from
  `sdk/mirroring_core/ui/common.slint`)
- `ui/components/std/` (from `senders/ui-components/`)

If the upstream SDK changes these files (rare but possible),
re-vendor as part of the same SDK-pin-bump PR:

```bash
# In the consumer repo's working dir:
SRC=/path/to/kodyka-fcast
cp "$SRC/sdk/mirroring_core/ui/common.slint" \
    ui/components/mcore/common.slint
cp -a "$SRC/senders/ui-components/." \
    ui/components/std/

# Re-apply the import-path rewrite at the two edge files
# (settings_page.slint:21 and common.slint:1) — see
# `ui/components/mcore/README.md` for the exact rewrites.

git diff ui/components/      # review the changes
```

If the upstream change broke the import-path rewrite (e.g. renamed
`std-widgets.slint` to `widgets.slint`), patch the rewrite and
note it in the PR body.

## Release-blocking SDK regression

If a Mondaily bump introduces a regression that's hard to fix:

1. Revert the bump PR in the consumer repo (back to the previous
   SHA).
2. Open an SDK-side PR in `kodyka/fcast` with the fix.
3. Once that merges, retry the consumer-side bump pointing at the
   new SHA.

Do **not** branch the consumer repo from "old SDK + new UI" and
try to forward-port the SDK fix into a fork — the divergence cost
is much higher than the revert-and-retry cost.

## When to publish (Option B) instead

If the SDK starts having external consumers besides the Android
sender (e.g. a third-party app), consider publishing to crates.io.
The bump procedure then becomes:

```bash
# In the consumer repo:
cargo update -p fcast-protocol -p fcast-sender-sdk -p mcore
# Cargo picks up the latest semver-compatible version from crates.io.
```

Publishing requires SDK-side discipline (semver, CHANGELOG, version
bumps per PR). The decision to publish is its own PHASE (call it
PHASE-11 if it happens).
````

### 2.2 Add a link from the README

In the new repo's top-level README (created in STEP-2 §2.4), under
"SDK dependencies", add:

```markdown
To bump the SDK pin, see
[`docs/cross-repo-sync.md`](docs/cross-repo-sync.md).
```

### 2.3 Add a CODEOWNERS rule (optional but recommended)

Create `.github/CODEOWNERS` in the new repo:

```
# SDK pin changes require review by the SDK maintainers.
/Cargo.toml          @kodyka/fcast-sdk-maintainers
/Cargo.lock          @kodyka/fcast-sdk-maintainers
/ui/components/      @kodyka/fcast-sdk-maintainers
```

(Adjust team names to match the actual GitHub teams that exist for
the org.)

### 2.4 Add a CONTRIBUTING.md cross-link

If the new repo has (or gets) a `CONTRIBUTING.md`, add a section:

```markdown
## SDK changes

If your contribution requires changes to the FCast SDK (the
`fcast-protocol`, `fcast-sender-sdk`, or `mcore` crates), see
[`docs/cross-repo-sync.md`](docs/cross-repo-sync.md) for the
PR-pair workflow.
```

---

## 3. Verification

### 3.1 The doc renders correctly

```bash
# In a browser, view docs/cross-repo-sync.md on the new repo's
# GitHub.
```

Or use a local markdown previewer. Make sure all links resolve.

### 3.2 The PR-pair workflow has been exercised at least once

After STEP-9 lands, the next opportunity to exercise the workflow
is the first SDK pin bump. Use that PR as a dry-run of the
procedure. If anything is confusing or wrong in the doc, fix it
in a follow-up doc PR.

### 3.3 CODEOWNERS is enforced

Open a no-op PR that touches `Cargo.toml` (e.g. adds a comment).
GitHub should auto-request review from
`@kodyka/fcast-sdk-maintainers`. If it doesn't, the team
doesn't exist or the rule is mis-spelled.

---

## 4. Pitfalls specific to this step

### P1 — Documenting a workflow that no one follows

A doc that says "bump weekly" is worthless if no one is on the
hook for the weekly bump. Either name an owner (in the doc) or
automate the bump-PR creation via a scheduled job. A weekly cron
that opens "chore: bump SDK pin" PRs is a 20-line workflow file;
worth it.

### P2 — Letting drift accumulate

If the SDK is bumped quarterly instead of weekly, each bump pulls
in ~50 commits. The risk of a single bump introducing a regression
scales superlinearly with the commit range. Weekly bumps stay
small.

### P3 — Not re-vendoring Slint helpers when needed

The Slint helpers are vendored — they don't auto-update with the
SDK pin. If upstream renames a widget and the consumer doesn't
re-vendor, the next build fails with a confusing error
("ComboBox not found in std-widgets.slint" — but it's in the
vendored copy, not the upstream one). The "Re-vendoring" section
of the doc must be top-of-mind.

### P4 — Treating CODEOWNERS as enforcement

CODEOWNERS auto-requests review. It does **not** block merging
without an approval — that's a separate "Require review from
Code Owners" branch protection setting. If you want SDK pin
bumps gated, enable the branch protection too.

### P5 — Forgetting to document the rare cases

The "release-blocking SDK regression" workflow (§2.1 of the doc)
is rare but high-impact. If it isn't documented, the next person
to hit it improvises a fix that may make things worse (e.g. forks
the SDK, branches the consumer, etc.). Keep it in the doc even if
it's been hit zero times.

### P6 — Publishing without semver discipline

The "When to publish (Option B)" section warns about this, but
the warning isn't enough — if the team decides to publish, the
SDK-side maintainers need an explicit semver policy and a release
PR template **before** the first publish. Don't skip that work
"because we'll figure it out as we go".

---

## 5. Wrap-up — PHASE-10 done

The doc lands. STEP-1 through STEP-9 are complete:

- ✓ STEP-1: inventory and strategy
- ✓ STEP-2: new repo bootstrapped, source copied
- ✓ STEP-3: path deps resolved
- ✓ STEP-4: workspace deps inlined; `[profile.release]` copied
- ✓ STEP-5: Slint helpers vendored
- ✓ STEP-6: CI / Gradle / Dockerfile / env documented
- ✓ STEP-7: first build verified end-to-end
- ✓ STEP-8: monorepo cleaned up
- ✓ STEP-9: cross-repo workflow documented

The Android sender is now a standalone repository with a small,
explicit, public contract (the SDK crates as Git deps).

**Next** PHASES (out of scope for PHASE-10):

- **PHASE-11**: publish the SDK to crates.io (if/when external
  consumers materialise).
- **PHASE-12**: similarly extract the desktop sender (if/when the
  monorepo / desktop release cadence diverges).
- Implementation work for PHASE-1..9 continues unaffected in
  `kodyka/fcast`; the new repo picks them up via the weekly pin
  bump cadence documented in STEP-9.

Return to the [parent doc](./MVP-PHASE-10-android-sender-repo-extraction.md)
§3 "Verification (phase-level)" to confirm the phase-level
acceptance criteria.

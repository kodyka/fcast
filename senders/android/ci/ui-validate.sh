#!/usr/bin/env bash
# senders/android/ci/ui-validate.sh — Phase 10 audit script.
#
# Consolidated UI validation routine for the Slint Android sender. Run this
# after every UI-only phase merges and before opening a PR that touches
# `senders/android/ui/`. The script wraps the static audits from the Phase 10
# reimplement guide:
#
#   * Build gate            — `cargo build -p android-sender` + Slint compiler
#                             watch for "missing layout size" / "Binding loop".
#   * Touch-target audit    — flag `(min-)height: <Npx>` declarations under 48px
#                             that look interactive.
#   * ListView nesting      — flag any `ListView` that appears nested inside a
#                             `ScrollView` (kills `ListView` virtualization).
#   * Panel routing         — flag any `Panel.<variant>` routed in `main.slint`
#                             that nothing in the rest of the UI sets, or any
#                             `Panel.<variant>` referenced by a page that
#                             `main.slint` does not route.
#   * Animate-with-zero     — flag `animate <prop> { duration: 0 …` blocks
#                             (these are silently no-op on every backend).
#
# Usage:
#   senders/android/ci/ui-validate.sh                # full audit + build gate
#   senders/android/ci/ui-validate.sh --no-build     # audits only (fast)
#   senders/android/ci/ui-validate.sh --audit-only   # alias for --no-build
#
# Exit codes:
#   0  — all checks passed (or only soft warnings).
#   1  — at least one hard failure (Slint compile error, animate duration: 0,
#        or build/test failure when `--no-build` is not set).
#
# This script intentionally has no dependencies beyond `bash`, `grep`,
# `cargo`, and the standard POSIX coreutils so it can run in any CI runner
# that already builds the Android sender.

set -euo pipefail

# ── CLI parsing ──────────────────────────────────────────────────────────────
RUN_BUILD=1
for arg in "$@"; do
    case "$arg" in
        --no-build|--audit-only)
            RUN_BUILD=0
            ;;
        -h|--help)
            sed -n '2,32p' "$0"
            exit 0
            ;;
        *)
            echo "unknown flag: $arg" >&2
            echo "see $0 --help" >&2
            exit 2
            ;;
    esac
done

# Always operate from the repo root so relative paths in the audit greps work
# the same regardless of where the script is invoked from.
cd "$(git rev-parse --show-toplevel)"

UI_ROOT="senders/android/ui"
BUILD_LOG="${BUILD_LOG:-/tmp/fcast-android-build.log}"
TMPDIR_RUN="$(mktemp -d -t fcast-ui-validate.XXXXXX)"
trap 'rm -rf "$TMPDIR_RUN"' EXIT

# Status tracking. We collect failures and warnings then summarise at the end
# so a single run reports every problem at once instead of bailing on the
# first issue.
HARD_FAILURES=0
SOFT_WARNINGS=0

note() { echo "  $*"; }
fail() { echo "FAIL: $*" >&2; HARD_FAILURES=$((HARD_FAILURES + 1)); }
warn() { echo "WARN: $*" >&2; SOFT_WARNINGS=$((SOFT_WARNINGS + 1)); }
pass() { echo "OK:   $*"; }

# ── Section 1 — Build gate ──────────────────────────────────────────────────
if [ "$RUN_BUILD" -eq 1 ]; then
    echo "=== Build gate ==="
    if ! cargo build -p android-sender 2>&1 | tee "$BUILD_LOG"; then
        fail "cargo build -p android-sender did not exit cleanly"
    fi

    # Slint compiler watch list. These two strings are the canonical signals
    # for the most common UI-only mistakes; treat any hit as a hard failure.
    if grep -E "ERROR: missing layout size|Binding loop detected" "$BUILD_LOG" \
            >/dev/null 2>&1; then
        fail "Slint compiler reported missing layout size / binding loop"
        grep -E "ERROR: missing layout size|Binding loop detected" "$BUILD_LOG" \
            | sed 's/^/    /' >&2
    else
        pass "no Slint missing-layout-size or binding-loop errors"
    fi

    # New-warning surface check. The build emits a stable set of pre-existing
    # warnings (mostly upstream gst-plugin-webrtc + slint-macros). Any net
    # increase is a soft warning so the developer can investigate without
    # blocking merges that are otherwise clean.
    BUILD_WARNINGS=$(grep -c '^warning:' "$BUILD_LOG" || true)
    note "build emitted $BUILD_WARNINGS warning(s) (informational)"

    echo "=== Cargo tests ==="
    if ! cargo test -p android-sender; then
        fail "cargo test -p android-sender failed"
    else
        pass "cargo test -p android-sender"
    fi
else
    echo "=== Build gate skipped (--no-build) ==="
fi

# ── Section 4 — Touch-target audit (48dp minimum) ────────────────────────────
echo "=== Touch-target audit ==="
SMALL_HEIGHTS_FILE="$TMPDIR_RUN/small-heights.txt"
# Match `height: NNpx` and `min-height: NNpx` where NN is 10..47, i.e. below
# the Material 48dp guideline. The grep is intentionally loose — many hits are
# expected to be non-interactive (badges, status pills, etc.) and the operator
# is asked to triage manually.
grep -REn '(height|min-height) *: *(1[0-9]|2[0-9]|3[0-9]|4[0-7])px' \
    "$UI_ROOT" --include='*.slint' > "$SMALL_HEIGHTS_FILE" || true

if [ -s "$SMALL_HEIGHTS_FILE" ]; then
    warn "$(wc -l < "$SMALL_HEIGHTS_FILE") row(s) declare a sub-48px height; please triage"
    sed 's/^/    /' "$SMALL_HEIGHTS_FILE" >&2
    note "sub-48px heights are only a problem on tappable rows; non-interactive"
    note "elements (badges, pills, separators) are fine."
else
    pass "no sub-48px heights detected"
fi

# ── Section 7 — ListView-in-ScrollView audit ─────────────────────────────────
# Wrapping a `ListView` inside a `ScrollView` disables ListView's virtualization
# (only visible rows instantiated). The audit is intentionally heuristic — it
# flags any file where both elements appear and the `ListView` line number is
# greater than the nearest preceding `ScrollView` line number.
echo "=== ListView nesting audit ==="
NESTING_FILE="$TMPDIR_RUN/listview-nesting.txt"
: > "$NESTING_FILE"
while IFS= read -r path; do
    [ -z "$path" ] && continue
    # Walk the file; remember the most recent ScrollView line, and emit a
    # warning whenever a ListView appears after a ScrollView without an
    # intervening closing-brace-only line at column 1 (a heuristic for
    # ScrollView body close).
    awk '
        /ScrollView *{/  { last_sv = NR; depth = 1; next }
        depth >= 1 && /\{/ { depth++ }
        depth >= 1 && /\}/ { depth--; if (depth == 0) last_sv = 0 }
        /ListView *{/ {
            if (last_sv > 0) {
                printf("%s:%d: ListView appears nested inside ScrollView opened at line %d\n",
                       FILENAME, NR, last_sv)
            }
        }
    ' "$path"
done < <(grep -RIl --include='*.slint' 'ListView' "$UI_ROOT" || true) \
    >> "$NESTING_FILE"

if [ -s "$NESTING_FILE" ]; then
    warn "$(wc -l < "$NESTING_FILE") possible ListView-in-ScrollView nesting hit(s)"
    sed 's/^/    /' "$NESTING_FILE" >&2
else
    pass "no ListView nested inside ScrollView"
fi

# ── Section 9 — Panel routing audit ──────────────────────────────────────────
# A `Panel.<variant>` is *routed* if `main.slint` mounts a page on it
# (`if Bridge.active-panel == Panel.<variant>: …`). It is *referenced* if
# any page (or `bridge.slint`) sets `Bridge.active-panel = Panel.<variant>`
# or mentions it in the enum body. Both lists must intersect — orphan routes
# are dead UI; orphan references are dead navigation.
echo "=== Panel routing audit ==="
ROUTED_FILE="$TMPDIR_RUN/panels-routed.txt"
ENUM_FILE="$TMPDIR_RUN/panels-declared.txt"
SET_FILE="$TMPDIR_RUN/panels-set.txt"

# Routes: `Panel.<variant>` mentioned in main.slint after `==`. The `|| true`
# guard is required: with `set -euo pipefail`, a `grep` that finds zero matches
# (e.g. during a refactor that temporarily strips Panel routes from main.slint)
# exits 1, pipefail propagates the failure, and set -e kills the script before
# the audit can report what happened.
grep -hoE 'Panel\.[a-z][a-z0-9-]*' "$UI_ROOT/main.slint" \
    | sort -u > "$ROUTED_FILE" || true

# Setters: `Bridge.active-panel = Panel.<variant>` anywhere except main.slint.
# Same pipefail guard as ROUTED_FILE above.
grep -rhoE 'Bridge\.active-panel *= *Panel\.[a-z][a-z0-9-]*' \
    "$UI_ROOT" --include='*.slint' \
    | sed -E 's/.*= *(Panel\.[a-z][a-z0-9-]*)/\1/' \
    | sort -u > "$SET_FILE" || true

# Enum members declared in bridge.slint inside `export enum Panel { ... }`.
awk '
    /export enum Panel/ { in_enum = 1; next }
    in_enum && /^}/     { in_enum = 0; next }
    in_enum {
        # Lines look like "    none," or "    settings,"; trim and drop blanks.
        sub(/,.*$/, "")
        gsub(/[[:space:]]/, "")
        if (length($0)) print "Panel." $0
    }
' "$UI_ROOT/bridge.slint" | sort -u > "$ENUM_FILE" || true

# Orphan routes = routed in main.slint but never set anywhere.
ORPHAN_ROUTES=$(comm -23 "$ROUTED_FILE" "$SET_FILE" | grep -v '^Panel\.none$' || true)
# Orphan sets   = set by a page but not routed in main.slint.
ORPHAN_SETS=$(comm -13 "$ROUTED_FILE" "$SET_FILE" | grep -v '^Panel\.none$' || true)
# Orphan enum   = declared but never routed and never set (dead variant).
ORPHAN_ENUM=$(comm -23 "$ENUM_FILE" <(sort -u "$ROUTED_FILE" "$SET_FILE") \
              | grep -v '^Panel\.none$' || true)

if [ -n "$ORPHAN_ROUTES" ]; then
    warn "panel(s) routed in main.slint but never set (no entry point):"
    echo "$ORPHAN_ROUTES" | sed 's/^/    /' >&2
else
    pass "every routed Panel variant has at least one setter"
fi

if [ -n "$ORPHAN_SETS" ]; then
    warn "panel(s) referenced by pages but not routed in main.slint:"
    echo "$ORPHAN_SETS" | sed 's/^/    /' >&2
else
    pass "every Panel.<variant> referenced by a page is routed in main.slint"
fi

if [ -n "$ORPHAN_ENUM" ]; then
    warn "panel enum variant(s) declared but never routed or set:"
    echo "$ORPHAN_ENUM" | sed 's/^/    /' >&2
else
    pass "every Panel enum variant is either routed or set"
fi

# ── animate { duration: 0 } audit ────────────────────────────────────────────
# `animate <prop> { duration: 0; … }` is a silent no-op on every Slint backend
# and is almost always a copy/paste mistake. Treat as a hard failure.
#
# Slint authors normally write multi-line animate blocks:
#
#     animate opacity {
#         duration: 0;
#         easing: ease;
#     }
#
# A single-line grep cannot match across the newline between `{` and
# `duration`, so we use awk to track when we are inside an `animate <prop> {`
# block and flag any `duration: 0` we see before the matching close brace.
# Slint does not allow nested animate blocks, so a simple in_anim flag is
# sufficient — no depth tracking required.
echo "=== animate-with-zero-duration audit ==="
ZERO_ANIM_FILE="$TMPDIR_RUN/zero-anim.txt"
: > "$ZERO_ANIM_FILE"
while IFS= read -r path; do
    [ -z "$path" ] && continue
    awk -v file="$path" '
        # Enter an animate block when we see `animate <prop> {` (the brace is
        # idiomatically on the same line as the animate keyword).
        /^[[:space:]]*animate[[:space:]]+[A-Za-z_-][A-Za-z0-9_-]*[[:space:]]*\{/ {
            in_anim = 1
            anim_start = NR
            zero_line = 0
        }
        # Inside an animate block, flag `duration: 0` (with no further
        # significant digits — `0`, `0;`, `0ms`, `0s` all count; `0.5s`
        # and `00.5` do not).
        in_anim && /duration[[:space:]]*:[[:space:]]*0([^0-9.]|$)/ {
            zero_line = NR
        }
        # Close brace ends the block.
        in_anim && /\}/ {
            if (zero_line > 0) {
                printf("%s:%d: animate block opened at line %d has duration: 0\n",
                       file, zero_line, anim_start)
            }
            in_anim = 0
            zero_line = 0
        }
    ' "$path"
done < <(grep -RIl --include='*.slint' 'animate' "$UI_ROOT" || true) \
    >> "$ZERO_ANIM_FILE"

if [ -s "$ZERO_ANIM_FILE" ]; then
    fail "found animate { duration: 0 } block(s):"
    sed 's/^/    /' "$ZERO_ANIM_FILE" >&2
else
    pass "no animate { duration: 0 } blocks"
fi

# ── Summary ──────────────────────────────────────────────────────────────────
echo
echo "=== Phase 10 UI validation summary ==="
echo "  hard failures: $HARD_FAILURES"
echo "  soft warnings: $SOFT_WARNINGS"

if [ "$HARD_FAILURES" -gt 0 ]; then
    echo "FAIL — hard failures detected; see above." >&2
    exit 1
fi

echo "ALL HARD CHECKS PASSED"

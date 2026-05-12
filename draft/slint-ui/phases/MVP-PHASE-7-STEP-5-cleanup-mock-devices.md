# MVP-PHASE-7 — Step 5 (optional): remove `mock-devices` / `mock-empty` from `ConnectView`

> Part 5 of 5. Parent doc: [`MVP-PHASE-7-receiver-item-promotion.md`](./MVP-PHASE-7-receiver-item-promotion.md).
> Previous: [Step 4 — click handler passes id](./MVP-PHASE-7-STEP-4-click-handler-passes-id.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

Remove two now-unused `in-out property`s from `ConnectView` in
`senders/android/ui/pages/connect_page.slint`:

- `mock-devices: [string]` (line ~22) — leftover from an earlier
  design where the page received its own mock list rather than
  reading from `Bridge.devices`.
- `mock-empty: bool` (line ~25) — toggled the empty-state view
  using a hardcoded boolean rather than checking `Bridge.devices.length`.

After this step, `ConnectView`'s `in-out property`s are pruned to
just the context-menu state (`show-context-menu`,
`context-receiver-id`, `context-receiver-name`, `context-menu-y`)
+ the forget-confirm state (`show-forget-confirm`).

This step is **fully optional**. The build works without it. Take
this step if you value tight, deletion-friendly Slint files; skip
it if you'd rather defer the cleanup to a follow-up PR.

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| `ConnectView` declaration | `senders/android/ui/pages/connect_page.slint:1-60` |
| `mock-devices` `in-out property` | `connect_page.slint:22` |
| `mock-empty` `in-out property` | `connect_page.slint:25` |
| Any external readers of `mock-devices` | **none expected** — confirm via grep (§3) |

### 1.2 Why these are safe to remove

`mock-devices` was the page's own list, but post-MVP-PHASE-1, the
receiver iterator binds to `Bridge.devices` directly. The
`mock-devices` property became a write-only orphan — Slint doesn't
warn about unused `in-out property`s, so it lingered.

`mock-empty` similarly tracked an emptiness state, but the actual
empty-state view (`@MVP-PHASE-1 §6.5`) reads
`Bridge.devices.length == 0` directly.

### 1.3 Why "optional"

The cleanup adds value (fewer dead properties, tighter file), but
it's not strictly required for PHASE-7's functional goal. If your
team's PR convention prefers "one thing per PR", split this into
a follow-up. Otherwise, fold it into the PHASE-7 commit.

---

## 2. The change

**File:** `senders/android/ui/pages/connect_page.slint`

**Before** (around lines 18-26):

```slint
export component ConnectView inherits Rectangle {
    // UI-only state. The receiver list comes from Bridge.devices.

    // Mock state from an earlier design — no longer wired.
    in-out property <[string]> mock-devices: [];
    in-out property <bool>     mock-empty: false;

    // Context menu state.
    in-out property <bool>   show-context-menu: false;
    in-out property <string> context-receiver-id;
    in-out property <string> context-receiver-name;
    in-out property <length> context-menu-y: 0px;

    // Forget confirmation state.
    in-out property <bool> show-forget-confirm: false;

    /* …existing layout… */
}
```

**After:**

```slint
export component ConnectView inherits Rectangle {
    // UI-only state. The receiver list comes from Bridge.devices.

    // Context menu state.
    in-out property <bool>   show-context-menu: false;
    in-out property <string> context-receiver-id;
    in-out property <string> context-receiver-name;
    in-out property <length> context-menu-y: 0px;

    // Forget confirmation state.
    in-out property <bool> show-forget-confirm: false;

    /* …existing layout… */
}
```

### 2.1 Verify nothing reads `mock-devices` first

Before deleting, grep:

```bash
grep -rn 'mock-devices\|mock-empty' senders/android/ui/
```

Expect **only** the two declarations in `connect_page.slint`. Any
external readers (e.g. tests, previews) must be removed first.

### 2.2 Removing in-out properties is a breaking API change

`in-out property`s on `export component`s are part of the
component's Slint API. External Slint files that `import` and
instantiate `ConnectView` could in principle set
`<ConnectView mock-devices: [...] />` — Slint would compile that
and the property would silently no-op (because nothing reads it
internally).

After deletion, any such external setter fails to compile. Run the
grep in §2.1 to confirm there are no callers.

---

## 3. Verification

### 3.1 Slint compile

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean**. If a hidden caller exists:

```
error: cannot set property 'mock-devices' on ConnectView — no such property
   --> senders/android/ui/.../foo.slint:42:5
    |
42  |     mock-devices: [];
    |     ^^^^^^^^^^^^^
```

Then either:

- (a) Restore the `mock-devices` property and split this cleanup
  into a separate PR with more research.
- (b) Update the caller to not set `mock-devices`.

### 3.2 Grep

```bash
grep -rn 'mock-devices\|mock-empty' senders/android/
# → 0 matches (file changes complete, no callers)
```

### 3.3 Visual smoke

This step has **no runtime effect**. Re-run the PHASE-7 smoke test
from [Step 4 §3.2](./MVP-PHASE-7-STEP-4-click-handler-passes-id.md#32-runtime-smoke)
to confirm nothing regressed.

---

## 4. Pitfalls specific to this step

### P1 — Removing `show-context-menu` by mistake

The state properties `show-context-menu` and `show-forget-confirm`
are **actively used**. Removing them would break the context menu
and the forget confirmation dialog. Only delete `mock-devices` and
`mock-empty`.

### P2 — Leaving behind a setter for the removed property

If the page body has `Button { mock-empty = !mock-empty; }`-style
references, those need to go too. The grep in §2.1 covers all
references inside the file.

### P3 — Skipping this step but adding TODOs

Don't:

```slint
// TODO: remove in MVP-PHASE-7 STEP-5
in-out property <[string]> mock-devices: [];
```

The TODO stays forever. Either delete now or take it in a
follow-up PR — but file a tracking issue.

### P4 — Folding additional cleanup into this step

Tempted to also rename `connect-receiver` → `connect-by-id`?
Don't. Scope this step strictly to removing the two `mock-*`
properties. Anything else is a separate STEP file or follow-up PR.

### P5 — "It compiled, so it's fine"

The `cargo +nightly check` covers Rust + Slint compilation but
not runtime visual regressions. Always pair Step 5 with a manual
visual smoke (§3.3).

---

## 5. PHASE-7 complete

After Step 5 (or Step 4 if you skip Step 5), PHASE-7 is fully
landed. Verify:

- `Bridge.devices: [ReceiverItem]` (Step 1).
- `update_receivers_in_ui` constructs `ReceiverItem` from
  `self.devices` (Step 2).
- Connect-page row shows name + address; long-press captures
  id + name; click passes id (Steps 3 + 4).
- (Optional) `mock-devices` / `mock-empty` deleted (Step 5).

Return to the parent [`MVP-PHASE-7`](./MVP-PHASE-7-receiver-item-promotion.md)
doc's §3 Verification for the full PHASE-7 smoke recipe, and §6
"Why this matters" for context on the next phases.

The MVP itself ends with PHASE-7. PHASE-8 (SRT destination family)
is an optional protocol-expansion phase that does not block any
MVP delivery target.

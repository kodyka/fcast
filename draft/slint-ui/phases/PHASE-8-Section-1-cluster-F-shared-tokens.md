# Phase 8 — Section 1: Cluster F — shared Theme + Bridge tokens

> Section 1 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) first.

**Cluster F is the smallest and unblocks all the others. Do it first.**

| Item | What we add | Why first |
|---|---|---|
| F1 | `Bridge.banner-message`, `Bridge.banner-visible`, `Bridge.banner-severity` | The destructive flows in Section 5 (Cluster D) all need a Rust-driven banner. Promoting them now lets every later cluster reuse the same primitive instead of inventing per-page banner state. |
| F2 | `BannerSeverity` enum (`info`, `success`, `warning`, `error`) | Same rationale — promoted once, consumed many times. |
| F3 | (already done on master) `Theme.success`, `Theme.warning`, `Theme.error`, `Theme.success-fg`, `Theme.warning-fg`, `Theme.error-fg` | These severity tokens already exist in master since Phase 27 (component utilities). Re-confirm they are present and skip the migration. |

**Net new code:** ~30 lines in `bridge.slint`, ~10 lines in `lib.rs` (a no-op handler for now), and one consumer migration in `components/info_banner.slint`.

**Risk:** very low. `Bridge.banner-*` is added but no consumer points at it yet — the existing pages keep using their page-local banners until you migrate them in Cluster D.

---

## 1.1 Confirm Theme severity tokens already exist

```sh
grep -nE 'success|warning|error' senders/android/ui/theme.slint
```

**Expected match (master, Phase 27 already shipped):**

```slint
out property <color> success: #2e7d32;
out property <color> warning: #ed6c02;
out property <color> error:   #c62828;
out property <color> success-fg: #c8e6c9;
out property <color> warning-fg: #ffe0b2;
out property <color> error-fg:   #ffcdd2;
```

If these are missing, you are **not** on the post-Phase-27 master. Stop and rebase first; do not re-add them yourself — Phase 27 owns the tokens and the design rationale.

---

## 1.2 Add `Bridge.banner-*` properties + `BannerSeverity` enum

### File: `senders/android/ui/bridge.slint`

**Add the enum** (sits next to the other enums, before the structs):

```diff
 export enum LifecycleMode {
     normal,
     lock-screen,
     stealth,
     snapshot-countdown,
 }

+// New in Phase 8 / Cluster F. Drives both Bridge.banner-* and any page-
+// local banner that wants to consume the same semantics.
+export enum BannerSeverity {
+    info,
+    success,
+    warning,
+    error,
+}
+
 export enum RecordingState {
     idle,
     recording,
     paused,
     finalizing,
 }
```

**Add the properties** (to the `Bridge` global, in the data-properties block):

```diff
 export global Bridge {
     // ── Data properties (Rust → Slint) ──────────────────────────────────
     in property <[string]> devices: [];
     in-out property <AppState> app-state: AppState.Disconnected;
     in-out property <Panel>    active-panel: Panel.none;
+    // ── Banner (Phase 8 / Cluster F1) ───────────────────────────────────
+    // Rust pushes a banner with severity; auto-hide is handled by a
+    // Bridge-side timer (consumer is `components/info_banner.slint`).
+    in-out property <string>          banner-message:  "";
+    in-out property <bool>            banner-visible:  false;
+    in-out property <BannerSeverity>  banner-severity: BannerSeverity.info;
     // …
 }
```

**Why each piece:**

- `banner-message: string` — the rendered text. Empty string = no banner; gives consumers a one-liner `if Bridge.banner-visible: …` test.
- `banner-visible: bool` — explicit visibility flag instead of `banner-message != ""`, because some banner copies are deliberately empty during animation in/out.
- `banner-severity: BannerSeverity` — drives colour/icon mapping in the consumer. The consumer never owns its own colour table — it picks from `Theme.success/warning/error`.
- `in-out` (not `in`) — Slint can dismiss the banner locally on click without round-tripping through Rust. Slint docs guidance: prefer `in-out` for flags Slint may toggle; reserve `in` for true read-only data. See `guide/language/coding/properties.mdx`.

**Slint doc citations for this step:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — enum declaration syntax.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — `in` / `in-out` / `out` direction semantics.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — global declaration block.

---

## 1.3 Migrate `components/info_banner.slint` to Bridge

The component itself stays a stateless dumb-pipe — it renders whatever it's given. The migration is to make it the **default** consumer of `Bridge.banner-*`, and let any page that wants page-local banner state still pass props directly.

### Before (master, Phase 27 shipped):

```slint
// info_banner.slint — stateless banner.
import { Theme } from "../theme.slint";

export enum BannerSeverity { info, success, warning, error }

export component InfoBanner inherits Rectangle {
    in property <string>         message;
    in property <bool>           visible: true;
    in property <BannerSeverity> severity: BannerSeverity.info;
    callback dismissed();
    // … render …
}
```

### After (post-F1, importing the canonical enum from bridge.slint):

```diff
 // info_banner.slint — stateless banner.
 import { Theme } from "../theme.slint";
+import { Bridge, BannerSeverity } from "../bridge.slint";

-export enum BannerSeverity { info, success, warning, error }
-
 export component InfoBanner inherits Rectangle {
-    in property <string>         message;
-    in property <bool>           visible: true;
-    in property <BannerSeverity> severity: BannerSeverity.info;
+    // Defaults follow Bridge — pages may still override these props for
+    // page-local banners (see Backup/Reset which has its own pending-action
+    // banner).
+    in property <string>         message:  Bridge.banner-message;
+    in property <bool>           visible:  Bridge.banner-visible;
+    in property <BannerSeverity> severity: Bridge.banner-severity;
     callback dismissed();
     // … render …
 }
```

**Why:**

- The default-binding form `in property <string> message: Bridge.banner-message;` makes Bridge-driven banners "free" — drop the component, get the content. Consumers pass nothing.
- Pages that need their own banner content (e.g. backup/reset's "Confirm reset?" prompt) still write `InfoBanner { message: root.local-message; … }` to override the defaults. **Both modes work simultaneously.** Slint default-bindings are reactive — a default binding is recomputed only as long as no one writes the prop directly, exactly the semantics we want.
- The local `BannerSeverity` enum gets removed because every consumer should now reach the same identity through `bridge.slint`. Removing avoids duplicate-symbol confusion when both files are imported in the same scope.

**Slint doc citations for this step:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — default-binding semantics ("a binding becomes a one-shot initial value once written to imperatively").
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — `import { Bridge, ... }` syntax.

---

## 1.4 Add a Rust handle for the banner (declarative-only is fine for now)

In `senders/android/src/lib.rs`, add a tiny no-op publisher near the other `set_*` calls. Nothing pushes a banner *yet* — Cluster D will hook destructive flows up to it. But putting the function there now means Cluster D can land without churning bridge.slint a second time.

```rust
// lib.rs — somewhere near the other Bridge setters (≈ line 1000 in master).
//
// Helper for any callback that needs to flash a banner. Centralised so we
// only own one upgrade-on-event-loop pattern.
fn set_banner(
    ui_handle: slint::Weak<MainWindow>,
    msg: String,
    severity: BannerSeverity,
) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        let bridge = ui.global::<Bridge>();
        bridge.set_banner_message(msg.into());
        bridge.set_banner_severity(severity);
        bridge.set_banner_visible(true);
    });
}

fn clear_banner(ui_handle: slint::Weak<MainWindow>) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        ui.global::<Bridge>().set_banner_visible(false);
    });
}
```

Add the `BannerSeverity` import where the other Slint generated types are imported:

```rust
slint::include_modules!();   // already there

// (no-op — BannerSeverity is generated alongside Panel and AppState)
```

**Auto-hide note:** Slint's generated `BannerSeverity` is a Rust enum, so `bridge.set_banner_severity(BannerSeverity::Warning)` is type-checked. If you want auto-hide-after-3s (matching how `network_page.slint` does it for the Wi-Fi-Aware banner), add an explicit `tokio::time::sleep` chained to the same `upgrade_in_event_loop`:

```rust
fn flash_banner(
    ui_handle: slint::Weak<MainWindow>,
    msg: String,
    severity: BannerSeverity,
    duration: std::time::Duration,
) {
    set_banner(ui_handle.clone(), msg, severity);
    tokio::spawn(async move {
        tokio::time::sleep(duration).await;
        clear_banner(ui_handle);
    });
}
```

**Why a separate `flash_banner` and not a Slint Timer:**

- A Slint Timer with `running: Bridge.banner-visible` works, but it ties auto-hide policy to the **consumer**, not the **producer**. Cluster D may want different durations per action ("clear cast history" → 2s, "settings imported" → 5s).
- Having the Rust producer own the duration also means tests can `flash_banner(..., Duration::from_millis(0))` and the next assertion can read `banner-visible == false` immediately.

**Slint doc citations for this step:**

- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx` — Slint-side timer fallback (used by the network_page banner today).

---

## 1.5 Verification

```sh
# Build green
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings

# Bridge property additions visible
grep -nE 'banner-(message|visible|severity)' senders/android/ui/bridge.slint
# Expect 3 lines.

# Mock count unchanged (Cluster F adds, doesn't remove)
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l
# Expect: same as Section 0.2 baseline. If lower, you accidentally
# replaced a mock-* somewhere — revert that change.

# Smoke test (only if slint-viewer is on $PATH)
slint-viewer senders/android/ui/main.slint
# Should render normally — no banner is ever shown because no producer
# has called Bridge.set_banner_visible(true) yet.
```

---

## 1.6 Commit message

```
feat(slint-ui): Phase 8 / Cluster F — shared banner tokens

- Add BannerSeverity enum to bridge.slint
- Add Bridge.banner-message, Bridge.banner-visible, Bridge.banner-severity
- Make components/info_banner.slint default-bind those props
- Add lib.rs helpers `set_banner`, `clear_banner`, `flash_banner`
- No consumer pushes yet; Cluster D will wire backup/reset and history.

Mock-count delta: 0 (Cluster F adds bridge state without removing any
page-local mock-* — that's expected; the consumer migrations land in
Clusters A-D).
```

---

## 1.7 Exit criteria for Section 1

- [x] `bridge.slint` declares `BannerSeverity`, `banner-message`, `banner-visible`, `banner-severity`
- [x] `components/info_banner.slint` no longer declares its own `BannerSeverity` enum
- [x] `lib.rs` has `set_banner`, `clear_banner`, `flash_banner` helpers (or one of them, depending on whether you choose to ship `flash_banner` now or wait)
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green
- [x] `slint-viewer` smoke test renders without binding-loop warnings
- [x] `mock-*` inventory count is unchanged from Section 0.2

You can now move to **Section 2 — Cluster A: read-only view models** at [`PHASE-8-Section-2-cluster-A-readonly-view-models.md`](./PHASE-8-Section-2-cluster-A-readonly-view-models.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`

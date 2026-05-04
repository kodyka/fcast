# Phase 5 — Status Overlay reimplementation guide (UI-only)

**Audience:** developer applying the **updated** [`draft/slint-ui/phases/PHASE-5-status-overlay.md`][spec] to the current `senders/android` tree.
**Goal:** rewind the existing functional Phase 5 wiring to a **UI-only placeholder** that matches the v2 phase spec — inline mock data on the page, no `Bridge.status-items` property, no Rust setter.
**Scope:** Slint UI + minimal Rust (remove 5 `set_status_items` call sites). No new features.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-5-status-overlay.md

---

## Why this guide exists

Audit of the current `migrate` branch (post-merge of PR #8) shows the existing code already implements a **functional** Phase 5: Rust populates `Bridge.status-items` from `lib.rs`, and `casting_page.slint` binds the overlay to that property. The updated phase spec rewinds that to UI-only:

- `Bridge` should **not** declare `status-items` (only the struct + enum types).
- `CastingView` should declare an inline `in-out property <[StatusItem]> mock-status-items: […]` and bind the overlay to that.
- Rust should **not** call `set_status_items`. Functional wiring is parked in Phase 8 (deferred).

Reverting to UI-only is a deliberate scope rollback, not a regression — it lets the UI evolve independently and keeps Phase 5 alignable with the rest of the UI-only roadmap (Phases 6, 7, 12–48).

### Audit before you start

Run these from the repo root to confirm the drift:

```sh
# Files that should NOT mention Bridge.status-items after this guide is applied
grep -rn 'Bridge\.status-items\|set_status_items\|status-items:' \
  senders/android/ui/ senders/android/src/

# Expected output BEFORE you start (5 Rust + 1 Slint + 1 Bridge property + 1 component comment):
# senders/android/src/lib.rs:642   ui.global::<Bridge>().set_status_items(...)
# senders/android/src/lib.rs:740   ui.global::<Bridge>().set_status_items(...)
# senders/android/src/lib.rs:764   ui.global::<Bridge>().set_status_items(...)
# senders/android/src/lib.rs:867   ui.global::<Bridge>().set_status_items(status_model.into())
# senders/android/ui/bridge.slint:43   in property <[StatusItem]> status-items: []
# senders/android/ui/components/status_overlay.slint:6 // Requires ... status-items property
# senders/android/ui/pages/casting_page.slint:60   items: Bridge.status-items;

# Expected output AFTER (zero matches):
# (empty)
```

Once everything in this guide is applied, the only mention of `status_items` left should be **inside `lib.rs:build_status_items`** if you choose to keep the helper for Phase 8 reactivation, or zero matches if you delete it too.

---

## Prerequisites

```sh
# Branch off migrate (per session policy)
git fetch origin
git checkout migrate
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-5-ui-only
```

Build commands you'll run as you go:

```sh
# Quick Slint compile-only sanity (much faster than full Android build)
cargo check -p android-sender

# Full debug build
cargo build -p android-sender
```

---

## Step 1 — Drop `status-items` from `Bridge`

**File:** `senders/android/ui/bridge.slint`

The struct and enum stay. The `in property <[StatusItem]> status-items: []` line goes.

### Diff

```diff
 export enum StatusSeverity { info, warning, error }

 export struct StatusItem {
     label:    string,
     value:    string,
     severity: StatusSeverity,
 }

 export global Bridge {
     // ── Data properties (Rust → Slint) ──────────────────────────────────
     in property <[string]> devices: [
         // "Device 1", "Device 2",
     ];
     in-out property <AppState> app-state: AppState.Disconnected;
     in-out property <bool>     show-debug: false;
     in-out property <string>   test-status: "";
     in property <[QuickAction]> quick-actions: [];
-    in property <[StatusItem]> status-items: [];

     // ── Callbacks (Slint → Rust) ─────────────────────────────────────────
     callback connect-receiver(string);
```

### Why

Per spec 5-A: *"Do not add `in property <[StatusItem]> status-items` to `Bridge`. The mock data lives at the page level (5-D)."* Keeping the struct + enum exports is fine; consuming code in `status_overlay.slint` and `casting_page.slint` still imports them.

### Build check

```sh
cargo check -p android-sender
```

This **will fail** with `error: no method named 'set_status_items'` on the 5 Rust call sites — that's expected. Step 2 fixes it.

---

## Step 2 — Remove the Rust setter call sites

**File:** `senders/android/src/lib.rs`

The compiler error from Step 1 will point you at the 5 lines below. Three are "clear the model on state-X" no-ops; one is the populate-on-cast call; one is `build_status_items` itself.

### Sub-step 2-A: Remove the four `set_status_items` call sites

Delete each `ui.global::<Bridge>().set_status_items(...);` line. They are at:
- `lib.rs:642` (clear on disconnect)
- `lib.rs:740` (clear on connect retry)
- `lib.rs:764` (clear on stop-casting)
- `lib.rs:866-867` (populate on cast start — also delete the `let status_items = build_status_items(...);` and `let status_model = ...;` lines feeding into it)

Leave a one-line `// Phase 8 (deferred): wire Bridge.status-items here.` breadcrumb at each removal site so future Phase 8 work is easy to slot back in.

Example for `lib.rs:863-867`:

```diff
-                let status_items = build_status_items(&receiver_name, encoder_name, &network_info);
-                let _ = ui_weak.upgrade_in_event_loop(move |ui| {
-                    let status_model = std::rc::Rc::new(slint::VecModel::from(status_items));
-                    ui.global::<Bridge>().set_status_items(status_model.into());
-                });
+                // Phase 8 (deferred): wire Bridge.status-items here from
+                // build_status_items(&receiver_name, encoder_name, &network_info).
```

Apply the analogous removal at the other three sites. Each clear-call collapses to:

```diff
-                    ui.global::<Bridge>().set_status_items(std::rc::Rc::new(slint::VecModel::default()).into());
+                    // Phase 8 (deferred): clear Bridge.status-items here.
```

### Sub-step 2-B: Decide what to do with `build_status_items`

The `build_status_items` helper at `lib.rs:503-521` is the only remaining caller-less function. Two options:

**Option A — Keep the helper (recommended for fast Phase 8 reactivation):**

```rust
// Phase 8 (deferred): producer of Bridge.status-items. Currently unused —
// CastingView renders mock-status-items inline. Keep this helper so the
// Rust side of Phase 8 is a one-line wire-up.
#[allow(dead_code)]
fn build_status_items(receiver_name: &str, encoder: &str, network: &str) -> Vec<crate::StatusItem> {
    /* …unchanged body… */
}
```

**Option B — Delete the helper:**

Remove `lib.rs:503-521` entirely. Phase 8 will rebuild the helper anyway.

Pick whichever fits your project's "tolerate `#[allow(dead_code)]` vs. delete unused code" stance. Both are spec-compliant.

### Build check

```sh
cargo check -p android-sender
```

Should now compile clean. If you took Option A, expect zero warnings about the unused helper (the `#[allow(dead_code)]` silences them). If you took Option B, also delete the line `use crate::StatusItem;` if it becomes unused.

---

## Step 3 — Add inline mock on `CastingView`

**File:** `senders/android/ui/pages/casting_page.slint`

Replace the `Bridge.status-items` binding with an inline `mock-status-items` property declared on `CastingView`. The `StatusItem` / `StatusSeverity` types are still imported from `bridge.slint`.

### Diff

```diff
 import { VerticalBox } from "std-widgets.slint";
 import { Bridge, AppState } from "../bridge.slint";
+import { StatusItem, StatusSeverity } from "../bridge.slint";
 import { LoadingView, DestructiveButton } from "../components/buttons.slint";
 import { StatusOverlay } from "../components/status_overlay.slint";
```

```diff
 // ── CastingView ───────────────────────────────────────────────────────────────

 export component CastingView inherits Rectangle {
+    // UI-only placeholder (Phase 5). Replace with `Bridge.status-items` when
+    // Phase 8 (Rust wiring) lands — see PHASE-8-rust-bridge.md.
+    in-out property <[StatusItem]> mock-status-items: [
+        { label: "Receiver", value: "Living Room TV",       severity: StatusSeverity.info },
+        { label: "Encoder",  value: "amcvidenc (H.264)",    severity: StatusSeverity.info },
+        { label: "Network",  value: "192.168.1.42",         severity: StatusSeverity.info },
+    ];
+
     VerticalBox {
         alignment: center;
         Text {
             horizontal-alignment: center;
             text: "Casting";
         }

         CastButton {}
     }

     StatusOverlay {
         x: 0;
         y: 0;
         width: parent.width * 0.5;
-        items: Bridge.status-items;
+        items: root.mock-status-items;
     }
 }
```

### Notes

- The mock list has 3 entries to mirror what `build_status_items` produced before — that way visual QA against the old Rust-driven build is a direct A/B.
- Keep the `Bridge, AppState` import — `CastingView` still references `Bridge.app-state` indirectly via `CastButton`. Add `StatusItem, StatusSeverity` as a separate import line for readability.
- `width: parent.width * 0.5` is unchanged from the existing layout — the overlay anchors to the top-left half of the casting screen, leaving the right half free for future widgets.

### Build check

```sh
cargo build -p android-sender
```

Visual smoke test: launch the APK, transition to `AppState.Casting`. The three info pills should render in the top-left corner stacked vertically.

---

## Step 4 — Add the warning / error alt mock for visual QA

**File:** `senders/android/ui/pages/casting_page.slint`

Per spec 5-E, add a second mock list demonstrating all three severities so you can manually flip the binding to verify color paths.

### Diff

```diff
     // UI-only placeholder (Phase 5). Replace with `Bridge.status-items` when
     // Phase 8 (Rust wiring) lands — see PHASE-8-rust-bridge.md.
     in-out property <[StatusItem]> mock-status-items: [
         { label: "Receiver", value: "Living Room TV",       severity: StatusSeverity.info },
         { label: "Encoder",  value: "amcvidenc (H.264)",    severity: StatusSeverity.info },
         { label: "Network",  value: "192.168.1.42",         severity: StatusSeverity.info },
     ];
+
+    // Severity-coverage stub. Flip `items: root.mock-status-items-error;` once,
+    // screenshot to verify warning/error coloring, then revert before commit.
+    in-out property <[StatusItem]> mock-status-items-error: [
+        { label: "Receiver", value: "Living Room TV", severity: StatusSeverity.info },
+        { label: "Network",  value: "Reconnecting…",  severity: StatusSeverity.warning },
+        { label: "Encoder",  value: "Failed",         severity: StatusSeverity.error },
+    ];
```

### Why a second property and not a `bool` toggle?

A `bool` plus a ternary `items: condition ? a : b;` works too, but two named lists are easier to spot in `git blame` and survive code-review better. The trade-off: the second list is dead-code at runtime — fine for a UI-only placeholder.

### Build check

```sh
cargo build -p android-sender
```

Then optionally swap the `StatusOverlay.items:` binding to `root.mock-status-items-error` for one screenshot, verify info → muted dark, warning → orange-brown, error → dark red, **revert** the swap, and commit.

---

## Step 5 — Update the `status_overlay.slint` comment header

**File:** `senders/android/ui/components/status_overlay.slint`

The header comment claims this component requires `Bridge.status-items` — no longer true.

### Diff

```diff
 // status_overlay.slint — Transparent status-pill overlay for the casting screen.
-// Stub created in Phase 1 to establish the import chain.
-// Components are implemented in Phase 5.
+// Implemented in Phase 5 as a UI-only placeholder. Real data is fed from
+// CastingView's inline `mock-status-items` property until Phase 8 wires Rust.
 //
 // Will export: StatusOverlay
-// Requires bridge.slint to export: StatusItem struct, status-items property
+// Requires bridge.slint to export: StatusItem struct, StatusSeverity enum
 // Reference:  draft/moblin-ui/Moblin/View/Stream/StreamOverlayView.swift (design ref only)
```

The `import { StatusItem, StatusSeverity } from "../bridge.slint";` line at the top of the file is already correct — no change needed.

---

## Step 6 — Final audit

Run the audit greps again. Both should now return **zero matches**:

```sh
# Slint side: no Bridge.status-items reference anywhere
grep -rn 'Bridge\.status-items' senders/android/ui/

# Rust side: no setter calls (the helper at lib.rs:503 is allowed if you
# kept Option A above; it's defined but never called)
grep -rn 'set_status_items' senders/android/src/

# Bridge property is gone
grep -n 'status-items' senders/android/ui/bridge.slint
```

If all three commands return empty, you're spec-compliant.

### Final build + visual

```sh
cargo build -p android-sender                  # release build optional
./gradlew :app:assembleDebug                   # Android APK
```

Install on device, navigate to the casting screen, confirm the three pills render. Edit `mock-status-items` at design time to verify reactivity (Slint should re-render without a rebuild if you're using `slint-viewer`; otherwise rebuild and reinstall).

---

## Exit criteria checklist (mirrors `PHASE-5-status-overlay.md`)

- [ ] `status_overlay.slint` exports `StatusOverlay` (no change — already true).
- [ ] `bridge.slint` defines `StatusItem` + `StatusSeverity` (types only — no `status-items` property).
- [ ] `CastingView` declares `mock-status-items: […]` inline.
- [ ] `CastingView` renders three info pills from the inline stub.
- [ ] Severity coloring works against `mock-status-items-error` (info / warning / error).
- [ ] No `Bridge.status-items` reference exists anywhere in `senders/android/ui/` or `senders/android/src/`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

When you're ready to wire real data, the migration from this UI-only state is **a single binding change** in `casting_page.slint`:

```diff
     StatusOverlay {
         x: 0;
         y: 0;
         width: parent.width * 0.5;
-        items: root.mock-status-items;
+        items: Bridge.status-items;
     }
```

Plus, in Phase 8, restore:
- `bridge.slint`: `in property <[StatusItem]> status-items: [];`
- `lib.rs`: `set_status_items` calls at the 4 lifecycle sites (using `build_status_items` if you kept Option A).

The `mock-status-items*` properties on `CastingView` can be **deleted** at that point — they're vestigial once `Bridge.status-items` is live. Or leave them as `#[allow]`-style design-time reference (Slint won't warn about unused properties).

---

## Slint-doc references used

These are the exact upstream docs that justify each pattern in the guide:

- **Inline `in-out property` with array literal as stub model** — [Slint Properties guide][props]. Reactivity works the same whether the source is a literal or a `Bridge` setter, so swapping later is mechanical.
- **`StatusSeverity` enum + `StatusItem` struct** — [Structs and enums][structs]. Closed value sets are type-safe and survive renames.
- **`for item in root.items: StatusPill { item: item; }`** — [Repetition][repeat]. The `for` loop reactively rebuilds when the bound list changes.
- **`visible: root.items.length > 0`** — [Visibility & opacity][visible]. `visible: false` skips both rendering and event handling — preferred over `opacity: 0` for hiding overlays.
- **`@allow(dead_code)` on `build_status_items`** — Rust-side, not Slint. Keep the helper alive so Phase 8 reactivation is a one-line wire-up.

[props]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[structs]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx
[repeat]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/repetition.mdx
[visible]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/rectangle.mdx

---

## What's NOT in this guide

These belong to other phases and stay parked:

- Real Rust population of receiver / encoder / network values → **Phase 8** (deferred).
- Severity escalation on connection-loss events → Phase 8 + new event source.
- Real-time bitrate / framerate pill → depends on GStreamer pipeline introspection (deferred to a future capture-telemetry phase).
- `@tr("…")` wrapping of "Receiver" / "Encoder" / "Network" / "Reconnecting…" / "Failed" → **Phase 9** (localization sweep).
- Animations on pill insert / remove → not in spec; add as a future polish phase.

If you want to do any of the above, pause and write a separate phase doc first.

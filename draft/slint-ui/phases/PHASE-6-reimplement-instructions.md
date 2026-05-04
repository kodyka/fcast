# Phase 6 — Receiver List & Discovery UX reimplementation guide (UI-only)

**Audience:** developer applying the **updated** [`draft/slint-ui/phases/PHASE-6-receiver-list.md`][spec] to the current `senders/android` tree.
**Goal:** upgrade the receiver list from a flat one-line `[string]` `ListView` to a typed two-line `ReceiverItem` card layout with animated empty state and a manual-IP row — **UI-only**, no Rust discovery changes.
**Scope:** Slint UI only. **No Rust changes.** The legacy `Bridge.devices: [string]` property and `Bridge.connect-receiver(string)` callback are intentionally left in place so the existing Rust mDNS code keeps compiling untouched.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-6-receiver-list.md

---

## Why this guide exists

Audit of the current `migrate` branch (post-merge of PR #8) shows the connect screen is still on its **Phase 1** shape:

- `bridge.slint` has no `ReceiverItem` struct yet.
- `connect_page.slint` reads `Bridge.devices: [string]` directly, renders a single elided line per row, and binds the row tap to the live `Bridge.connect-receiver(device)` callback.
- The empty state is a static `"Searching for receivers..."` `Text`.
- There is no manual-IP fallback row.

The updated PHASE-6 spec keeps that legacy data path untouched (Rust still pushes strings into `Bridge.devices`) and asks us to **build the new visual surface alongside it** using inline mock data:

- Add a typed `ReceiverItem { name, address }` struct to `bridge.slint` (struct only — **no** `[ReceiverItem]` `Bridge` property).
- Declare `mock-devices: [ReceiverItem]` and `mock-empty: bool` on `ConnectView` itself.
- Render a two-line card per entry (`name` over `address`, both elided).
- Replace the static empty-state text with a `Spinner` + label card.
- Add a `LineEdit` + `PrimaryButton` manual-IP row at the bottom.

This is a **strictly additive** UI rebuild on top of the existing page: the new `for device in root.mock-devices: ...` loop replaces the existing `for device in Bridge.devices: ...` loop, but the legacy `Bridge.devices` property and `connect-receiver` callback in `bridge.slint` stay exactly where they are. Rust does not need to be touched.

### Audit before you start

Run these from the repo root to confirm the current state:

```sh
# Should be ZERO matches before you start (everything new lives in this guide):
grep -rn 'ReceiverItem\|mock-devices\|mock-empty' senders/android/ui/

# Should show legacy bindings on connect_page.slint that this guide will REPLACE:
grep -n 'Bridge\.devices\|Bridge\.connect-receiver' senders/android/ui/pages/connect_page.slint
# Expected:
#   26:        if Bridge.devices.length == 0: Rectangle {
#   40:        if Bridge.devices.length > 0: ListView {
#   41:            for device in Bridge.devices: Rectangle {
#   45:                    clicked => Bridge.connect-receiver(device);

# Should still exist in bridge.slint after this guide is applied (legacy, untouched):
grep -n 'devices:\s*\[\|connect-receiver' senders/android/ui/bridge.slint
# Expected:
#   36:    in property <[string]> devices: [
#   46:    callback connect-receiver(string);
```

After this guide is applied:

```sh
# `Bridge.devices` and `Bridge.connect-receiver` should NO LONGER appear in connect_page.slint
grep -n 'Bridge\.devices\|Bridge\.connect-receiver' senders/android/ui/pages/connect_page.slint
# Expected: (empty)

# But the legacy declarations in bridge.slint MUST remain (Rust still binds them):
grep -n 'in property <\[string\]> devices\|callback connect-receiver' senders/android/ui/bridge.slint
# Expected:
#   36:    in property <[string]> devices: [
#   46:    callback connect-receiver(string);
```

---

## Prerequisites

```sh
# Branch off migrate (per session policy)
git fetch origin
git checkout migrate
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-6-ui-only
```

Build commands you'll run as you go:

```sh
# Slint compile-only sanity check (fastest)
cargo check -p android-sender

# Full debug build
cargo build -p android-sender

# APK (after final visual)
./gradlew :app:assembleDebug
```

---

## Step 1 — Add `ReceiverItem` struct to `bridge.slint`

**File:** `senders/android/ui/bridge.slint`
**Spec:** PHASE-6 task 6-A.

The struct goes alongside the existing `QuickAction` and `StatusItem` exports. The legacy `in property <[string]> devices: []` and `callback connect-receiver(string)` **stay untouched** — Rust still owns them.

### Diff

```diff
 export struct StatusItem {
     label:    string,
     value:    string,
     severity: StatusSeverity,
 }

+export struct ReceiverItem {
+    name:    string,
+    address: string,
+}
+
 export global Bridge {
     // ── Data properties (Rust → Slint) ──────────────────────────────────
     in property <[string]> devices: [
         // "Device 1", "Device 2",
     ];
```

### Why

Per spec 6-A: *"Type definition only — no `Bridge` property yet."* The struct is exported so `connect_page.slint` can `import { ReceiverItem }` from the bridge module. We deliberately do **not** add `in property <[ReceiverItem]> receivers: []` to `Bridge` — that's Phase 8's job. Inline mock data on `ConnectView` (Step 2) supplies the rows for now.

Slint structs are anonymous structural types that can be defined in any `.slint` file and exported alongside globals — see the structs guide ([structs-and-enums.mdx][structs]).

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. The new struct is unused at this point but defining an unused struct is not an error in Slint 1.15.1.

---

## Step 2 — Refactor `connect_page.slint` imports & declare inline stub state

**File:** `senders/android/ui/pages/connect_page.slint`
**Spec:** PHASE-6 task 6-B + 6-D + 6-E (import side effects).

We need to:

1. Drop the `ListView` import (the new layout uses a plain `for` loop inside `VerticalBox`, no virtualised scrolling at this size).
2. Add `Spinner` and `LineEdit` from `std-widgets.slint`.
3. Add the `ReceiverItem` import from `../bridge.slint`.
4. Declare `mock-devices` and `mock-empty` `in-out` properties at the top of `ConnectView`.

> **Note on `ListView` vs. `for` in a `VerticalBox`:** Slint's `ListView` virtualises children — only visible rows are instantiated ([listview.mdx][listview]). For a typical receiver list of 0–8 entries on a phone this is overkill, and `ListView` enforces an internal scroll viewport that complicates layout under our `VerticalBox { alignment: start }` chassis. Phase 6's spec dropped `ListView` from the loop; we follow that. If you find yourself routinely scanning >20 receivers (e.g. during a discovery torture test), wrap the `for` in `ListView { ... }` again — `for` syntax is identical inside both ([repetition.mdx][repeat]).

### Diff (top of file)

```diff
-import { VerticalBox, ListView } from "std-widgets.slint";
-import { Bridge } from "../bridge.slint";
+import { VerticalBox, Spinner, LineEdit } from "std-widgets.slint";
+import { Bridge, ReceiverItem } from "../bridge.slint";
 import { DebugPage } from "debug_page.slint";
 import { Theme } from "../theme.slint";
 import { PrimaryButton } from "../components/buttons.slint";

 export component ConnectView inherits Rectangle {
+    // ── UI-only stub state ───────────────────────────────────────────────
+    // Live discovery still pushes strings into `Bridge.devices` from Rust,
+    // but this page renders the new typed model from inline mock data.
+    // Phase 8 will swap `root.mock-devices` for `Bridge.receivers` and
+    // `root.mock-empty` for a derived `Bridge.scanning` property.
+    in-out property <[ReceiverItem]> mock-devices: [
+        { name: "Living Room TV",     address: "192.168.1.50" },
+        { name: "Office Display",     address: "192.168.1.51" },
+        { name: "Kitchen Chromecast", address: "192.168.1.52:46899" },
+    ];
+    in-out property <bool> mock-empty: false;
+
     VerticalBox {
         alignment: start;
         Text {
```

### Why

- **Inline `in-out property <[ReceiverItem]>`** — Slint `in-out` properties accept array literals as their default value, and the `for ... in root.mock-devices` loop reactively rebuilds when the property changes ([properties.mdx][props], [repetition.mdx][repeat]). Swapping the source from a literal to `Bridge.receivers` later is a one-line edit (Step 7 below).
- **`in-out property <bool> mock-empty`** — declared as `in-out` rather than `in` because we want a developer to be able to flip it from another component during visual QA without recompiling. In the live build it stays `false`.
- **`Spinner`** — `import { Spinner } from "std-widgets.slint"` is the standard widget for indeterminate progress. The `indeterminate: true` mode renders a continuously-rotating ring and requires no driver code ([spinner.mdx][spinner]).
- **`LineEdit`** — single-line text input, supports `placeholder-text` and `text` (`in-out`) plus `accepted` / `edited` callbacks ([lineedit.mdx][lineedit]).

### Build check

```sh
cargo check -p android-sender
```

Expected: still clean. We added imports and properties but haven't yet referenced them. Removing the `ListView` import will trigger a build error in the *next* step's interim state — that's fine, the same step reroutes the loop.

---

## Step 3 — Replace the static empty-state with a `Spinner` card

**File:** `senders/android/ui/pages/connect_page.slint`
**Spec:** PHASE-6 task 6-D.

The new empty-state surfaces while the spec's `mock-empty` is true **or** when `mock-devices` is empty. The Phase 6 spec leans on the boolean override so a developer can preview the empty-state visually without emptying the array.

### Diff

```diff
         // ── Empty state: searching ────────────────────────────────────────
-        if Bridge.devices.length == 0: Rectangle {
+        if root.mock-empty || root.mock-devices.length == 0: Rectangle {
             height: 90px;
             border-radius: Theme.radius-card;
             background: Theme.surface-card;

-            Text {
-                horizontal-alignment: center;
-                vertical-alignment: center;
-                color: Theme.text-secondary;
-                text: "Searching for receivers...";
+            HorizontalLayout {
+                alignment: center;
+                spacing: Theme.spacing-default;
+
+                Spinner {
+                    indeterminate: true;
+                    width:  24px;
+                    height: 24px;
+                }
+                Text {
+                    text: "Searching for receivers…";
+                    color: Theme.text-secondary;
+                    vertical-alignment: center;
+                }
             }
         }
```

### Why

- **`if root.mock-empty || root.mock-devices.length == 0`** — `||` on a boolean and a length comparison is short-circuit: the spinner card renders both during normal "no receivers yet" startup *and* whenever a developer flips `mock-empty: true` in the editor for visual QA. Spec 6-D explicitly recommends this pattern.
- **`HorizontalLayout { alignment: center; spacing: Theme.spacing-default; }`** — `HorizontalLayout` is the simplest two-child centred row; `alignment: center` centres horizontally inside the parent rectangle, and `spacing` adds the gap between children. See the [layouts guide][layouts].
- **`Spinner { indeterminate: true }`** — `indeterminate` is the right mode when the operation has no measurable progress (mDNS browse). The widget is fully self-driving ([spinner.mdx][spinner]).
- **`vertical-alignment: center` on the `Text`** — without this the label baselines below the spinner.
- **The trailing ellipsis is the U+2026 character (`…`)**, not three dots. Phase 5's spec uses the same convention.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. Visual: nothing changes yet because `mock-devices` has 3 entries and `mock-empty` is false, so the empty-state branch doesn't render. To confirm the new card paints, **temporarily** flip `mock-empty: true` at the top of the file, rebuild, and screenshot. **Revert the flip before committing.**

---

## Step 4 — Replace the populated `ListView` with two-line cards

**File:** `senders/android/ui/pages/connect_page.slint`
**Spec:** PHASE-6 task 6-C.

This is the visual heart of the phase. We swap the single-line `ListView` for a `for` loop that renders a card per `ReceiverItem`, with `name` (primary, white, `font-size-body`) over `address` (secondary, `text-secondary`, `font-size-label`), both elided. The wrapper `Rectangle` provides a touch target of `Theme.row-height + 18px` (= 66 px) and the inner `Rectangle` provides the visible card with its press-state colouring.

### Diff

```diff
         // ── Populated state: device list ──────────────────────────────────
-        if Bridge.devices.length > 0: ListView {
-            for device in Bridge.devices: Rectangle {
-                height: Theme.row-height;
-
-                device_ta := TouchArea {
-                    clicked => Bridge.connect-receiver(device);
-                }
-
-                Rectangle {
-                    width: parent.width - 10px;
-                    height: parent.height - 10px;
-                    background: device_ta.pressed ? Theme.accent-pressed : Theme.accent-muted;
-                    border-radius: Theme.radius-card;
-                    Text {
-                        vertical-alignment: center;
-                        horizontal-alignment: left;
-                        text: device;
-                    }
-                }
-            }
-        }
+        if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
+            spacing: Theme.spacing-default;
+
+            for device in root.mock-devices: Rectangle {
+                height: Theme.row-height + 18px;
+
+                ta := TouchArea {
+                    // UI-only — no real connect handler in this phase.
+                    clicked => { /* placeholder: would call connect-receiver(device.address) */ }
+                }
+
+                Rectangle {
+                    width: parent.width - 10px;
+                    height: parent.height - 8px;
+                    background: ta.pressed ? Theme.accent-pressed : Theme.surface-card;
+                    border-radius: Theme.radius-card;
+
+                    VerticalLayout {
+                        padding-left:  Theme.padding-screen;
+                        padding-right: Theme.padding-screen;
+                        alignment: center;
+                        spacing: 2px;
+
+                        Text {
+                            text: device.name;
+                            color: Theme.text-primary;
+                            font-size: Theme.font-size-body;
+                            overflow: elide;
+                        }
+                        Text {
+                            text: device.address;
+                            color: Theme.text-secondary;
+                            font-size: Theme.font-size-label;
+                            overflow: elide;
+                        }
+                    }
+                }
+            }
+        }
```

### Why each piece

- **`if !root.mock-empty && root.mock-devices.length > 0`** — mirrors the empty-state condition. Both conditional blocks are mutually exclusive: when `mock-empty` is true, the spinner shows and the list is hidden; otherwise we choose between spinner (length 0) and list (length > 0). This is the canonical "either/or" pattern with two `if`s rather than `else` (Slint conditional elements use bare `if`, see the [language reference for conditional elements][conditional]).
- **Outer `Rectangle { height: row-height + 18px }`** — outer touch surface. `Theme.row-height` is 48 px; `+ 18px` gives 66 px to fit two stacked text lines plus padding. Adding length values is a basic Slint expression ([expressions.mdx][expressions]).
- **`ta := TouchArea { clicked => { /* placeholder */ } }`** — the named `TouchArea` exposes the `pressed` boolean used by the inner card's `background` binding for press feedback ([toucharea.mdx][toucharea]). The empty handler block is a deliberate no-op — we are *not* supposed to call `Bridge.connect-receiver(device.address)` in this phase. Leave the explanatory comment so the next reader (or you in Phase 8) knows what goes there.
- **Inner `Rectangle { width: parent.width - 10px; height: parent.height - 8px; }`** — the visible card, inset 5 px on each side and 4 px top/bottom. This produces the gap between cards without requiring per-child margins.
- **`background: ta.pressed ? Theme.accent-pressed : Theme.surface-card`** — switched **away from** `accent-muted` (Phase 1's pale lightsteelblue) **to** `surface-card` (`#222633`, the dark grey used by the rest of the cards on this page). The press state still goes to `accent-pressed` (`#1e40af`, deep navy) for visible feedback. This aligns the receiver list with the visual language of the empty-state card and the bottom debug panel — the previous lightsteelblue rectangles looked like a styling vestige from before Phase 2's theme sweep.
- **Inner `VerticalLayout { padding-left/right: padding-screen; alignment: center; spacing: 2px }`** — the two-line text stack. `alignment: center` centres the pair vertically; `spacing: 2px` keeps the lines tight. `padding-screen` is 12 px from the theme — same as the rest of the page.
- **Both `Text { overflow: elide }`** — long device names ("…") and IPv6 addresses (`fe80::abcd:1234:5678:dead`) are common; `overflow: elide` truncates with an ellipsis instead of clipping or wrapping ([text.mdx][text]). For elide to take effect the parent must constrain width — `VerticalLayout` does this implicitly because it's inside the inner `Rectangle` which has a fixed `width: parent.width - 10px`.
- **Outer `VerticalLayout { spacing: Theme.spacing-default }`** — replaces `ListView`'s implicit row spacing. 8 px between cards.

### Build check

```sh
cargo check -p android-sender
```

Expected: clean. Run `cargo build -p android-sender` and screenshot the connect screen. You should see three dark-grey cards stacked, each showing a bold-ish name on top and a thin address below, both elided. Tap-and-hold any card → it should darken to deep navy until release.

---

## Step 5 — Add the manual-IP fallback row

**File:** `senders/android/ui/pages/connect_page.slint`
**Spec:** PHASE-6 task 6-E.

This row sits below the device list (or below the empty-state spinner) and gives the user a way in when discovery fails or the receiver isn't on mDNS. Like everything else in this phase it's UI-only — the `clicked` handler is a no-op.

### Diff

```diff
         // ── Populated state: device list ──────────────────────────────────
         if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
             ...
         }

+        // ── Manual IP entry (UI-only placeholder) ─────────────────────────
+        HorizontalLayout {
+            spacing: Theme.spacing-default;
+
+            ip-input := LineEdit {
+                placeholder-text: "Receiver IP address";
+                horizontal-stretch: 1;
+            }
+            PrimaryButton {
+                label: "Connect";
+                enabled: ip-input.text != "";
+                // UI-only — no real connect handler in this phase.
+                clicked => { /* placeholder: would call connect-receiver(ip-input.text) */ }
+            }
+        }
+
         // ── Debug panel (conditionally visible) ───────────────────────────
         // DebugPage inherits VerticalBox, so it slots into this layout
         // exactly as the original inline VerticalBox did.
         if Bridge.show-debug: DebugPage { }
```

### Why each piece

- **`HorizontalLayout { spacing: Theme.spacing-default }`** — single-row two-child layout. We avoid `HorizontalBox` because that adds extra padding around the row; we want this row to align flush with the cards above ([layouts.mdx][layouts]).
- **`ip-input := LineEdit { ... horizontal-stretch: 1 }`** — naming the `LineEdit` exposes its `text` property to the sibling button. `horizontal-stretch: 1` tells the row to allocate all unused width to the input — without it, `LineEdit` would size to its content (zero width when empty). Stretch values are documented in the [layouts guide][layouts].
- **`placeholder-text: "Receiver IP address"`** — single-line greyed prompt that disappears on focus, surfaces again when empty ([lineedit.mdx][lineedit]).
- **`PrimaryButton { label: "Connect"; enabled: ip-input.text != "" }`** — the button is reactive to the input being non-empty. `PrimaryButton` already exists from Phase 3 and accepts `label`, `enabled`, and `clicked`. The component lives at `senders/android/ui/components/buttons.slint`.
- **`clicked => { /* placeholder */ }`** — same UI-only pattern as Step 4. Spec 6-E says *"Click is a no-op."*

### Build check

```sh
cargo check -p android-sender
```

Expected: clean.

```sh
cargo build -p android-sender && ./gradlew :app:assembleDebug
```

Visual: a `LineEdit` and a `PrimaryButton` side-by-side under the receiver cards. Type a few characters — the **Connect** button should brighten from disabled-grey to its enabled steelblue. Tapping it does nothing, which is intended.

---

## Step 6 — Final audit

Run the audit greps from the top of this guide again:

```sh
# 1. New names exist on the page (and only on the page).
grep -rn 'ReceiverItem\|mock-devices\|mock-empty' senders/android/ui/
# Expected:
#   senders/android/ui/bridge.slint:35:export struct ReceiverItem {
#   senders/android/ui/pages/connect_page.slint:18:    in-out property <[ReceiverItem]> mock-devices: [
#   senders/android/ui/pages/connect_page.slint:24:    in-out property <bool> mock-empty: false;
#   senders/android/ui/pages/connect_page.slint:34:        if root.mock-empty || root.mock-devices.length == 0: Rectangle {
#   senders/android/ui/pages/connect_page.slint:54:        if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
#   senders/android/ui/pages/connect_page.slint:56:            for device in root.mock-devices: Rectangle {
# (line numbers will drift slightly depending on diff)

# 2. The legacy bindings are gone from the page.
grep -n 'Bridge\.devices\|Bridge\.connect-receiver' senders/android/ui/pages/connect_page.slint
# Expected: (empty)

# 3. The legacy declarations are still in bridge.slint (Rust binds them).
grep -n 'in property <\[string\]> devices\|callback connect-receiver' senders/android/ui/bridge.slint
# Expected:
#   36:    in property <[string]> devices: [
#   46:    callback connect-receiver(string);

# 4. No accidental `Bridge.receivers` binding (that's Phase 8's name).
grep -rn 'Bridge\.receivers' senders/android/
# Expected: (empty)

# 5. Imports are tidy — `ListView` should NOT appear in connect_page.slint anymore.
grep -n 'ListView' senders/android/ui/pages/connect_page.slint
# Expected: (empty)
```

If commands 2, 4, and 5 return empty and 1 + 3 show only the rows above, you are spec-compliant.

### Final visual

```sh
cargo build -p android-sender
./gradlew :app:assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

On the connect screen you should see (top to bottom):

1. The Phase-2 header `"Connect to your receiver"` (bold, centred).
2. Three dark-grey two-line cards. Top text white + 16 px (`name`); bottom text grey + 12 px (`address`). Tapping any card briefly turns it deep navy then back.
3. A `LineEdit` + grey-disabled `Connect` button. Typing enables the button.
4. The bottom `CastControlBar` from Phase 4 with the five quick-action cards.

Briefly flip `mock-empty: true` at the top of `connect_page.slint`, rebuild, screenshot — you should now see the spinner card with `"Searching for receivers…"` and the cards / list should disappear. **Revert the flip before committing.**

---

## Exit criteria checklist (mirrors `PHASE-6-receiver-list.md`)

- [ ] `bridge.slint` defines `export struct ReceiverItem { name, address }` (struct only — no `Bridge` property added).
- [ ] `connect_page.slint` shows two-line cards (name + address) for each entry in `mock-devices`.
- [ ] Empty state shows an animated `Spinner` plus `"Searching for receivers…"` text when `mock-empty` is true or `mock-devices.length == 0`.
- [ ] Manual-IP row renders with a `LineEdit` + `PrimaryButton`. The button is disabled when the input is empty. Click is a no-op.
- [ ] Existing `Bridge.devices: [string]` is untouched — Rust still builds with no changes to `senders/android/src/`.
- [ ] No reference to `Bridge.devices` or `Bridge.connect-receiver` exists in `senders/android/ui/pages/connect_page.slint`.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

When the Rust mDNS layer is updated to push typed `ReceiverItem`s, the migration from this UI-only state is **a small, mechanical edit** localised to `connect_page.slint` and `bridge.slint`:

### `bridge.slint` — promote the struct to a list property

```diff
 export struct ReceiverItem {
     name:    string,
     address: string,
 }

 export global Bridge {
     in property <[string]> devices: [];   // ← legacy, can be removed in Phase 8
+    in property <[ReceiverItem]> receivers: [];
+    in-out property <bool> scanning: false;
     in-out property <AppState> app-state: AppState.Disconnected;
     ...
-    callback connect-receiver(string);
+    callback connect-receiver(string /* address */);
     ...
 }
```

(The `connect-receiver(string)` signature stays — it already takes the address. Rust just needs to source addresses from the new `ReceiverItem.address` field.)

### `connect_page.slint` — swap the four bindings

```diff
-    in-out property <[ReceiverItem]> mock-devices: [
-        { name: "Living Room TV",     address: "192.168.1.50" },
-        { name: "Office Display",     address: "192.168.1.51" },
-        { name: "Kitchen Chromecast", address: "192.168.1.52:46899" },
-    ];
-    in-out property <bool> mock-empty: false;

-    if root.mock-empty || root.mock-devices.length == 0: Rectangle { ... }
+    if Bridge.scanning || Bridge.receivers.length == 0: Rectangle { ... }

-    if !root.mock-empty && root.mock-devices.length > 0: VerticalLayout {
-        for device in root.mock-devices: Rectangle {
+    if !Bridge.scanning && Bridge.receivers.length > 0: VerticalLayout {
+        for device in Bridge.receivers: Rectangle {
             ...
             ta := TouchArea {
-                clicked => { /* placeholder ... */ }
+                clicked => { Bridge.connect-receiver(device.address); }
             }
             ...
         }
     }

     HorizontalLayout {
         ip-input := LineEdit { ... }
         PrimaryButton {
             label: "Connect";
             enabled: ip-input.text != "";
-            clicked => { /* placeholder ... */ }
+            clicked => { Bridge.connect-receiver(ip-input.text); }
         }
     }
```

The `mock-devices` and `mock-empty` properties can either be deleted (cleanest) or left alongside the live bindings as design-time references — Slint doesn't warn on unused properties.

### Rust side (Phase 8 territory, **not** part of this guide)

- Map `DeviceInfo` (mDNS resolution) → `ReceiverItem { name, address }` (likely as a `From` impl).
- Replace `ui.global::<Bridge>().set_devices(...)` calls with `set_receivers(...)`.
- Add a `set_scanning(true/false)` setter around the mDNS browse window.
- Drop the legacy `Bridge.devices: [string]` once nothing imports it.

---

## Slint-doc references used

These are the exact upstream docs that justify each pattern in the guide:

- **Inline `in-out property <[ReceiverItem]>` with array literal as stub model** — [Properties][props]. Reactivity works the same whether the source is a literal or a `Bridge` setter, so swapping later is mechanical.
- **`ReceiverItem { name, address }` struct** — [Structs and enums][structs]. Anonymous structural types; can be exported and imported across `.slint` files.
- **`for device in root.mock-devices: Rectangle { ... }`** — [Repetition][repeat]. The `for` loop reactively rebuilds when the bound list changes; the syntax is identical inside `VerticalLayout`, `HorizontalLayout`, and `ListView`.
- **`if root.mock-empty || root.mock-devices.length == 0` & `if !root.mock-empty && root.mock-devices.length > 0`** — [Conditional elements][conditional]. Bare `if` (no `else`); for either/or branches use two `if`s with mutually-exclusive conditions.
- **`Spinner { indeterminate: true }`** — [Spinner][spinner]. Indeterminate mode renders a self-driving rotating ring; no animation/timer wiring required.
- **`LineEdit { placeholder-text: ...; horizontal-stretch: 1 }`** & **named alias `ip-input := LineEdit { ... }`** — [LineEdit][lineedit] for properties/callbacks; [Layouts][layouts] for `horizontal-stretch`. Naming an element with `:=` lets siblings read its `text` reactively.
- **`ta := TouchArea { clicked => { ... } }` & `ta.pressed`** — [TouchArea][toucharea]. The `pressed` boolean is reactive and can be bound from a sibling `Rectangle.background` for press feedback.
- **`overflow: elide` on `Text`** — [Text][text]. Truncates with an ellipsis when the parent constrains width; required for unknown-length names and IPv6 addresses.
- **`HorizontalLayout` & `VerticalLayout` with `spacing` / `alignment` / `padding-*`** — [Layouts][layouts]. The bare `*Layout` variants have no implicit margin (unlike `*Box`), so they line up flush with sibling cards.
- **Length arithmetic (`Theme.row-height + 18px`)** — [Expressions][expressions]. Length values support `+`, `-`, `*` against numeric literals.
- **No `ListView` for short lists** — [ListView][listview] documents that virtualisation only instantiates visible rows. For a 0–8-row receiver list this is unnecessary overhead and complicates layout under a `VerticalBox { alignment: start }` parent. Wrap the loop in `ListView { ... }` again only if the list grows past one screen.

[props]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[structs]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx
[repeat]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/repetition.mdx
[conditional]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/conditional-elements.mdx
[spinner]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/spinner.mdx
[lineedit]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx
[listview]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx
[toucharea]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/gestures/touch-area.mdx
[text]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/text.mdx
[layouts]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[expressions]: https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/language/coding/expressions.mdx

---

## What's NOT in this guide

These belong to other phases and stay parked:

- **Mapping `DeviceInfo` → `ReceiverItem` in Rust, replacing `Bridge.devices: [string]` with `[ReceiverItem]`, and wiring `connect-receiver(address)` to a real TCP connect** → **Phase 8** (Rust bridge hardening).
- **Per-receiver "last seen" / `kind` / signal-strength badges, "Forget" / saved-receiver history** → future polish phase; needs persistent storage and a Rust source of truth.
- **`accessible-role: list-item` / `accessible-label: "{device.name} at {device.address}"` on each card** → not in spec; deferred to a future accessibility phase.
- **`@tr("Connect to your receiver")` / `@tr("Searching for receivers…")` / `@tr("Receiver IP address")` / `@tr("Connect")` wrapping** → **Phase 9** (localization sweep).
- **Pull-to-refresh / swipe-to-forget gestures** → out of scope; requires gesture support beyond `TouchArea` and a Rust rescan callback.
- **Manual IPv6 / hostname validation on the `LineEdit`** → defer to Phase 8 when the click actually does something.
- **Animations on card insert / remove** → not in spec; consider after Phase 8 when receivers actually arrive asynchronously.

If you want to do any of the above, pause and write a separate phase doc first.

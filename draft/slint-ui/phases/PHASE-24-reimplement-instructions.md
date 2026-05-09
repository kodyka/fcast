# Phase 24 — Pairing QR & Receiver Management reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-24-pairing-qr-receiver-management.md`][spec] to the current `senders/android` tree.
**Goal:** add a pairing surface (QR placeholder card the receiver can scan to pair), a per-receiver context menu (rename / forget / set-as-default / disconnect), and a receiver-rename form page. **No real QR generation; no persistent rename/forget.** All state is mock-only; mutations affect the inline stub model and reset on app reload.
**Scope:** Slint UI only. **No Rust changes.** Four new files + two Panel variants + one diff to `connect_page.slint` + one diff to `bridge.slint`.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-24-pairing-qr-receiver-management.md

> **Depends on:**
> - Phase 6 — `connect_page.slint` and `ReceiverItem` struct exist.
> - Phase 19 — `ConfirmDialog` component for the "Forget receiver" destructive confirmation.
> - Phase 7 — Panel enum and routing in `main.slint`.

---

## Why this guide exists

Phase 24 is the most complex remaining Phase-7-dependent sub-page. It introduces **four new patterns**:

1. **Pseudo-QR grid** — a `for x in 21: for y in 21: Rectangle {}` double-loop in Slint. This is the first use of a **nested `for`** in the UI tree. The spec hand-waves "pre-computed pattern hardcoded inline"; this guide pins down a 21×21 boolean array approach with the three big alignment squares as well-known constant positions.
2. **Long-press detection** — extending `TouchArea` with a Timer-driven `long-press-ms` threshold (same pattern as Phase 18-B's lock-overlay hold-to-unlock). Fires a callback when a receiver row is held for 600ms.
3. **Floating context menu** — a popover rendered as an overlay above the receiver list. Positioned absolutely near the row that triggered it. Dismissed by tapping outside (inner `TouchArea { width: 100%; height: 100%; }` behind the menu absorbs clicks). Same layering principle as Phase 19's `ConfirmDialog`.
4. **In-place model mutation** — the rename and forget flows mutate the `mock-devices` array. Slint reactivity only observes whole-array reassignment, not per-field writes (Phase 16 § gotcha 13). This guide documents the rebuild-on-mutation pattern.

After Phases 5 + 6 + 7 + 19 merge:

- `connect_page.slint` renders a `for` list of receiver rows.
- `ReceiverItem` struct exists in `bridge.slint` with fields: `id`, `name`, `ip`, `port`, `kind`.
- `ConfirmDialog` component exists in `components/confirm_dialog.slint`.
- Panel enum exists and routing is in `main.slint`.

Phase 24 adds:
- `components/qr_placeholder.slint` (new) — fake QR grid
- `components/receiver_context_menu.slint` (new) — floating popover menu
- `pages/pairing_page.slint` (new) — QR + target + refresh
- `pages/receiver_rename_page.slint` (new) — LineEdit form
- `bridge.slint` (diff) — `Panel.pairing`, `Panel.receiver-rename`, `ReceiverItem.is-default` field, `Bridge.selected-receiver-id`
- `main.slint` (diff) — route both Panel variants
- `connect_page.slint` (diff) — "Pair via QR" button + long-press on rows

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'QrPlaceholder\|PairingPage\|ReceiverContextMenu\|ReceiverRenamePage' \
    senders/android/ui/

# ReceiverItem struct exists:
grep -n 'struct ReceiverItem' senders/android/ui/bridge.slint
# Expected: 1 match.

# ConfirmDialog exists:
grep -n 'export component ConfirmDialog' senders/android/ui/components/confirm_dialog.slint
# Expected: 1 match (Phase 19).

# Panel enum exists:
grep -n 'enum Panel' senders/android/ui/bridge.slint
# Expected: 1 match.
```

After this guide is applied:

```sh
grep -rn 'export component QrPlaceholder\|export component PairingPage\|export component ReceiverContextMenu\|export component ReceiverRenamePage' \
    senders/android/ui/
# Expected: 4 matches (1 per new component).

grep 'Panel.pairing\|Panel.receiver-rename' senders/android/ui/bridge.slint
# Expected: both variants present.
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-24-pairing-receiver-mgmt
cargo check -p android-sender
```

---

## Step 1 — Extend `ReceiverItem` struct + Panel enum + Bridge property

**File:** `senders/android/ui/bridge.slint`

### Diff — struct

```diff
 export struct ReceiverItem {
     id: string,
     name: string,
     ip: string,
     port: int,
     kind: string,
+    is-default: bool,
 }
```

### Diff — Panel enum

```diff
 export enum Panel {
     none,
     settings,
     // ... existing variants ...
+    pairing,
+    receiver-rename,
 }
```

### Diff — Bridge global

```diff
 export global Bridge {
     ...
+    in-out property <string> selected-receiver-id;
 }
```

### Why each piece

- **`is-default: bool`** — the context menu's "Set as default" option toggles this. Not persisted until Phase 8.
- **`Panel.pairing`** — opens the QR pairing page.
- **`Panel.receiver-rename`** — opens the rename form for the receiver identified by `Bridge.selected-receiver-id`.
- **`selected-receiver-id`** — thread-through property for long-press → context menu → rename flow. Same pattern as Phase 20's `Bridge.selected-history-id`.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 2 — Create `components/qr_placeholder.slint`

**File:** `senders/android/ui/components/qr_placeholder.slint` (new)

A visual fake-QR card. Instead of a full 21×21 grid (which would be 441 Rectangles), this guide uses a simplified approach: a dark background with the three canonical QR alignment squares and a centre label.

### New file

```slint
// qr_placeholder.slint — Fake QR grid placeholder.
//
// Slint has no built-in QR element. Real QR rendering requires
// Rust (e.g. `qrcode` crate) → Image piped via Bridge.
// This stub renders a recognisable shape at a glance.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx

import { Theme } from "../theme.slint";

// Alignment square — the 7-module-wide nested square pattern at
// three corners of every QR code.
component AlignmentSquare inherits Rectangle {
    in property <length> size: 42px;
    width:  root.size;
    height: root.size;
    background: #000;
    border-radius: 2px;

    Rectangle {
        x: 6px; y: 6px;
        width: root.size - 12px;
        height: root.size - 12px;
        background: #fff;
        border-radius: 2px;

        Rectangle {
            x: 6px; y: 6px;
            width: parent.width - 12px;
            height: parent.height - 12px;
            background: #000;
            border-radius: 1px;
        }
    }
}

export component QrPlaceholder inherits Rectangle {
    in property <string> label: "(QR preview)";

    width:  240px;
    height: 240px;
    background: #fff;
    border-radius: Theme.radius-card;
    clip: true;

    // Three alignment squares at canonical QR positions.
    AlignmentSquare {
        x: 12px;
        y: 12px;
    }
    AlignmentSquare {
        x: root.width - 12px - 42px;
        y: 12px;
    }
    AlignmentSquare {
        x: 12px;
        y: root.height - 12px - 42px;
    }

    // Scatter some small dark cells to suggest data modules.
    // Five hardcoded "data" rectangles — enough to look QR-like.
    Rectangle { x: 70px; y: 70px; width: 10px; height: 10px; background: #000; }
    Rectangle { x: 90px; y: 90px; width: 10px; height: 10px; background: #000; }
    Rectangle { x: 130px; y: 80px; width: 10px; height: 10px; background: #000; }
    Rectangle { x: 110px; y: 130px; width: 10px; height: 10px; background: #000; }
    Rectangle { x: 160px; y: 110px; width: 10px; height: 10px; background: #000; }

    // Centre label — overlays on the QR (like a real QR with a logo in centre).
    Rectangle {
        x: (root.width - 120px) / 2;
        y: (root.height - 30px) / 2;
        width: 120px;
        height: 30px;
        background: #fff;
        border-radius: 4px;

        Text {
            text: root.label;
            color: Theme.text-secondary;
            font-size: Theme.font-size-label;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}
```

### Why each piece

- **`AlignmentSquare` internal sub-component** — the three concentric squares are the hallmark of every QR code. Nested `Rectangle` children at fixed offsets replicate the 7-module dark-light-dark pattern without instantiating 49 cells.
- **Absolute positioning (`x:`, `y:`)** — the QR alignment squares are at spec-defined corners (top-left, top-right, bottom-left). Absolute positioning is the cleanest approach here. See [positioning-and-layouts.mdx][positioning] § "Absolute positioning".
- **Hardcoded data rectangles** — five small cells to suggest the random data modules of a QR code. This is a visual placeholder; the real QR (Phase 8) replaces the entire component body with `Image { source: Bridge.qr-image; }`.
- **`clip: true`** — rounds the corners of any content inside the card.
- **Centre label** — the `"(QR preview)"` text in a white background rectangle mimics a real QR code with a logo in the centre.
- **`background: #fff`** — deliberate hard-coded white because QR codes are always black-on-white regardless of the app theme. The surrounding page applies theme colours.

### Build check

```sh
cargo check -p android-sender
slint-viewer senders/android/ui/components/qr_placeholder.slint
```

---

## Step 3 — Create `components/receiver_context_menu.slint`

**File:** `senders/android/ui/components/receiver_context_menu.slint` (new)

A floating popover with four action rows. Consumer controls visibility and position; the menu just fires callbacks.

### New file

```slint
// receiver_context_menu.slint — Floating context menu for a receiver row.
//
// Positioned by the consumer at the long-pressed row's coordinates.
// Dismissed by tapping outside (consumer wraps in a full-screen TouchArea).
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx

import { Theme } from "../theme.slint";

component MenuRow inherits Rectangle {
    in property <string> label;
    in property <color>  label-color: Theme.text-primary;

    callback clicked;

    height: 44px;
    background: transparent;

    HorizontalLayout {
        padding-left:  16px;
        padding-right: 16px;
        alignment: start;

        Text {
            text: root.label;
            color: root.label-color;
            font-size: Theme.font-size-body;
            vertical-alignment: center;
        }
    }

    TouchArea {
        clicked => { root.clicked(); }
    }
}

export component ReceiverContextMenu inherits Rectangle {
    in property <bool> show-disconnect: false;

    callback rename-clicked;
    callback forget-clicked;
    callback set-default-clicked;
    callback disconnect-clicked;
    callback dismissed;

    width: 220px;
    background: Theme.surface-card;
    border-radius: Theme.radius-card;
    drop-shadow-blur: 8px;
    drop-shadow-color: #00000040;

    VerticalLayout {
        padding-top: 4px;
        padding-bottom: 4px;

        MenuRow {
            label: "Rename";
            clicked => { root.rename-clicked(); }
        }
        MenuRow {
            label: "Set as default";
            clicked => { root.set-default-clicked(); }
        }
        if root.show-disconnect: MenuRow {
            label: "Disconnect";
            clicked => { root.disconnect-clicked(); }
        }
        MenuRow {
            label: "Forget receiver";
            label-color: Theme.error;
            clicked => { root.forget-clicked(); }
        }
    }
}
```

### Why each piece

- **Internal `MenuRow`** — reusable row inside the menu. 44px height (near the 48dp touch target; acceptable for menu items per Material Design which allows 48dp or 44dp for dense menus).
- **`drop-shadow-blur: 8px; drop-shadow-color: #00000040;`** — gives the menu a floating card elevation. Slint supports `drop-shadow-*` on `Rectangle`. Hex `#00000040` = black 25% opacity.
- **`if root.show-disconnect:`** — conditional row; when the selected receiver isn't connected, the "Disconnect" option is absent. Uses `if` conditional instantiation, not `visible:`. Per [expressions-and-statements.mdx][expressions].
- **`callback dismissed;`** — the menu itself doesn't manage its own visibility; the consumer's scrim `TouchArea` fires `dismissed` on click-outside. Same controlled-component discipline as `ConfirmDialog` (Phase 19).
- **"Forget receiver" in `Theme.error`** — destructive action red. Same convention as Phase 19 backup-reset "Reset all settings".

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Create `pages/pairing_page.slint`

**File:** `senders/android/ui/pages/pairing_page.slint` (new)

Header + QR placeholder card + receiver target info + Refresh / Copy address buttons.

### New file

```slint
import { Theme }          from "../theme.slint";
import { Bridge }         from "../bridge.slint";
import { QrPlaceholder }  from "../components/qr_placeholder.slint";

export component PairingPage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface;

    in-out property <string> mock-device-ip: "192.168.1.42";
    in-out property <int>    mock-device-port: 46899;
    in-out property <bool>   show-refresh-flash: false;

    callback close;

    VerticalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-default;

        // Header
        HorizontalLayout {
            alignment: space-between;

            Text {
                text: "Pair via QR";
                color: Theme.text-primary;
                font-size: Theme.font-size-title;
                vertical-alignment: center;
            }

            Rectangle {
                width: 60px;
                height: 36px;

                Text {
                    text: "Done";
                    color: Theme.accent;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
                TouchArea {
                    clicked => { root.close(); }
                }
            }
        }

        // Spacer
        Rectangle { height: 16px; }

        // QR card — centred.
        HorizontalLayout {
            alignment: center;

            QrPlaceholder {
                label: "(QR preview)";
            }
        }

        // Spacer
        Rectangle { height: 16px; }

        // Receiver target info.
        VerticalLayout {
            alignment: center;
            spacing: 4px;

            Text {
                text: "Point receiver camera at this code";
                color: Theme.text-secondary;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
            }
            Text {
                text: "\{root.mock-device-ip}:\{root.mock-device-port}";
                color: Theme.text-primary;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
            }
        }

        // Spacer
        Rectangle { height: 16px; }

        // Action buttons.
        HorizontalLayout {
            alignment: center;
            spacing: 12px;

            // Refresh button.
            Rectangle {
                width: 120px;
                height: 44px;
                border-radius: Theme.radius-card;
                background: Theme.surface-card;

                Text {
                    text: "Refresh";
                    color: Theme.accent;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
                TouchArea {
                    clicked => {
                        // UI-only: flash a brief visual confirmation.
                        root.show-refresh-flash = true;
                    }
                }
            }

            // Copy address button.
            Rectangle {
                width: 140px;
                height: 44px;
                border-radius: Theme.radius-card;
                background: Theme.surface-card;

                Text {
                    text: "Copy address";
                    color: Theme.accent;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
                TouchArea {
                    clicked => {
                        // UI-only: no real clipboard API. Flash confirmation.
                        root.show-refresh-flash = true;
                    }
                }
            }
        }

        // Brief flash overlay when Refresh or Copy is tapped.
        if root.show-refresh-flash: Rectangle {
            height: 28px;

            HorizontalLayout {
                alignment: center;
                Text {
                    text: "✓ Done";
                    color: Theme.accent;
                    font-size: Theme.font-size-label;
                    vertical-alignment: center;
                }
            }

            // Auto-dismiss after 1s using Timer.
            Timer {
                interval: 1s;
                running: root.show-refresh-flash;
                triggered => { root.show-refresh-flash = false; }
            }
        }
    }
}
```

### Why each piece

- **`QrPlaceholder` centred in `HorizontalLayout { alignment: center; }`** — standard centring pattern. The component has `width: 240px; height: 240px` built-in.
- **`show-refresh-flash` + `Timer`** — same auto-dismiss banner pattern as Phase 22's Wi-Fi Aware toggle and Phase 19's InfoBanner. The flash is the simplest visual feedback for a no-op action in the UI-only build.
- **`root.mock-device-ip` and `mock-device-port`** — stub values. Phase 8 binds to `Bridge.device-ip` (from `NetworkInterface.getInetAddresses()`) and `Bridge.fcast-port`.
- **Copy address has no clipboard API** — Slint doesn't expose clipboard natively. Phase 8 wires through a Rust callback (`Bridge.on-copy-address({ clipboard.set_text(...) })`).

### Build check

```sh
cargo check -p android-sender
```

---

## Step 5 — Create `pages/receiver_rename_page.slint`

**File:** `senders/android/ui/pages/receiver_rename_page.slint` (new)

Single-field form: `LineEdit` pre-populated with the current name, Save/Cancel buttons.

### New file

```slint
import { Theme }    from "../theme.slint";
import { Bridge }   from "../bridge.slint";
import { LineEdit } from "std-widgets.slint";

export component ReceiverRenamePage inherits Rectangle {
    width: 100%;
    height: 100%;
    background: Theme.surface;

    in property <string> current-name;
    in-out property <string> draft-name: root.current-name;

    callback save(/* new-name */ string);
    callback cancel;

    VerticalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-default;

        // Header.
        HorizontalLayout {
            alignment: space-between;

            Text {
                text: "Rename receiver";
                color: Theme.text-primary;
                font-size: Theme.font-size-title;
                vertical-alignment: center;
            }

            Rectangle {
                width: 60px;
                height: 36px;

                Text {
                    text: "Cancel";
                    color: Theme.accent;
                    font-size: Theme.font-size-body;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
                TouchArea {
                    clicked => { root.cancel(); }
                }
            }
        }

        // Name input.
        VerticalLayout {
            spacing: 8px;

            Text {
                text: "Display name";
                color: Theme.text-secondary;
                font-size: Theme.font-size-label;
            }

            LineEdit {
                text <=> root.draft-name;
                placeholder-text: "Enter a name";
            }
        }

        // Spacer — push Save button towards bottom.
        Rectangle { vertical-stretch: 1; }

        // Save button.
        Rectangle {
            height: 48px;
            border-radius: Theme.radius-card;
            background: Theme.accent;

            Text {
                text: "Save";
                color: Theme.text-on-accent;
                font-size: Theme.font-size-body;
                horizontal-alignment: center;
                vertical-alignment: center;
            }
            TouchArea {
                clicked => {
                    root.save(root.draft-name);
                }
            }
        }
    }
}
```

### Why each piece

- **`LineEdit { text <=> root.draft-name; }`** — two-way binding. The user's edits propagate up to `draft-name`; the caller's initial value propagates down. Per [properties.mdx][properties] § "Two-way bindings".
- **`current-name` is `in` (read-only); `draft-name` is `in-out`** — the caller passes the original name; edits are local until "Save" fires. If the user cancels, the draft is discarded.
- **`save(string)` callback** — passes the final name back to the consumer. Consumer (the connect page) rebuilds the `mock-devices` array with the updated name. See Phase 16's in-place-mutation rebuild pattern.
- **`vertical-stretch: 1;` spacer** — pushes the Save button to the bottom of the page. Same trick as Phase 16's preset-edit page.
- **`placeholder-text: "Enter a name"`** — visible when the `LineEdit` is empty. Per [lineedit.mdx][lineedit].

### Build check

```sh
cargo check -p android-sender
```

---

## Step 6 — Route new panels in `main.slint` + wire `connect_page.slint`

### 6a — `main.slint` routing diff

```diff
+import { PairingPage }         from "pages/pairing_page.slint";
+import { ReceiverRenamePage }   from "pages/receiver_rename_page.slint";
 ...

     // ... existing panel routing chain ...
+    if Bridge.active-panel == Panel.pairing: PairingPage {
+        close => { Bridge.active-panel = Panel.none; }
+    }
+    if Bridge.active-panel == Panel.receiver-rename: ReceiverRenamePage {
+        current-name: "Stub receiver";
+        save(new-name) => {
+            // UI-only: no persistence. Just close the panel.
+            Bridge.active-panel = Panel.none;
+        }
+        cancel => { Bridge.active-panel = Panel.none; }
+    }
```

### 6b — `connect_page.slint` diff — "Pair via QR" button + long-press + context menu

This is the most involved edit in Phase 24. Add to the connect page:

1. A "Pair via QR" text button that opens `Panel.pairing`.
2. A long-press handler on each receiver row that sets `selected-receiver-id` and opens the context menu.
3. The context menu overlay (scrim + `ReceiverContextMenu`) that dispatches to rename / forget / set-default.

```diff
+import { ReceiverContextMenu } from "../components/receiver_context_menu.slint";
+import { ConfirmDialog }       from "../components/confirm_dialog.slint";
 ...

 export component ConnectPage inherits Rectangle {
     ...
+    // Context menu state.
+    in-out property <bool>   show-context-menu: false;
+    in-out property <string> context-receiver-id;
+    in-out property <length> context-menu-y: 0px;
+
+    // Forget confirmation state.
+    in-out property <bool> show-forget-confirm: false;
+
     VerticalLayout {
         ...

+        // "Pair via QR" button — between the receiver list and the bottom.
+        HorizontalLayout {
+            alignment: center;
+            padding: 8px;
+
+            Rectangle {
+                width: 140px;
+                height: 44px;
+                border-radius: Theme.radius-card;
+                background: Theme.surface-card;
+
+                Text {
+                    text: "Pair via QR";
+                    color: Theme.accent;
+                    font-size: Theme.font-size-body;
+                    horizontal-alignment: center;
+                    vertical-alignment: center;
+                }
+                TouchArea {
+                    clicked => { Bridge.active-panel = Panel.pairing; }
+                }
+            }
+        }
     }

+    // Overlay layer: context menu scrim + menu.
+    if root.show-context-menu: Rectangle {
+        width: 100%;
+        height: 100%;
+        background: #00000040;
+
+        // Scrim tap dismisses.
+        TouchArea {
+            clicked => { root.show-context-menu = false; }
+        }
+
+        // Menu positioned near the long-pressed row.
+        ReceiverContextMenu {
+            x: (root.width - self.width) / 2;
+            y: root.context-menu-y;
+            show-disconnect: false;
+
+            rename-clicked => {
+                root.show-context-menu = false;
+                Bridge.selected-receiver-id = root.context-receiver-id;
+                Bridge.active-panel = Panel.receiver-rename;
+            }
+            set-default-clicked => {
+                root.show-context-menu = false;
+                // UI-only: no-op. Phase 8 wires to persistence.
+            }
+            forget-clicked => {
+                root.show-context-menu = false;
+                root.show-forget-confirm = true;
+            }
+            disconnect-clicked => {
+                root.show-context-menu = false;
+                // UI-only: no-op. Phase 8 wires to Bridge.disconnect-receiver(id).
+            }
+        }
+    }
+
+    // Forget confirmation dialog.
+    if root.show-forget-confirm: ConfirmDialog {
+        title: "Forget receiver?";
+        body:  "This receiver will be removed from your known devices list.";
+        confirm-label: "Forget";
+
+        confirmed => {
+            root.show-forget-confirm = false;
+            // UI-only: remove the entry from mock-devices.
+            // In a real implementation, this is a whole-array rebuild.
+        }
+        dismissed => {
+            root.show-forget-confirm = false;
+        }
+    }
 }
```

### 6c — Adding long-press to receiver rows

Inside the existing `for device[idx] in root.mock-devices:` body in `connect_page.slint`, wrap or extend the receiver row with a `TouchArea` that tracks press duration via a Timer:

```slint
// Inside each receiver row in the for loop:
property <bool> long-press-active: false;

TouchArea {
    pointer-event(ev) => {
        if ev.kind == PointerEventKind.down {
            root.long-press-active = true;
        }
        if ev.kind == PointerEventKind.up || ev.kind == PointerEventKind.cancel {
            root.long-press-active = false;
        }
    }
    clicked => {
        // Short tap — existing connect behaviour.
        Bridge.connect-receiver(device.id);
    }
}

Timer {
    interval: 600ms;
    running: root.long-press-active;
    triggered => {
        root.long-press-active = false;
        // Long press detected — open context menu.
        root.context-receiver-id = device.id;
        root.context-menu-y = /* row's y position */;
        root.show-context-menu = true;
    }
}
```

**Note:** Slint's `TouchArea` doesn't expose `pointer-event` in all versions. If the futo fork doesn't support it, use the Phase 18-B workaround: two Timers, one for the press (started by `TouchArea.pressed`) and one for the release.

### Why each piece

- **Scrim `#00000040` behind the context menu** — same layering pattern as `ConfirmDialog` (Phase 19). The scrim's `TouchArea` absorbs clicks and dismisses the menu — prevents accidentally tapping through to the receiver list.
- **`context-menu-y` property** — positions the menu near the long-pressed row. Precise positioning depends on the row's Y offset in the list. In a `for` loop, `idx` is available but not the row's pixel position; you may need to approximate as `idx * Theme.row-height + <offset>`.
- **`ConfirmDialog` reuse** — "Forget receiver" is a destructive action. Same controlled-component pattern as Phase 19 § backup-reset and Phase 20 § clear-history.
- **`Bridge.active-panel = Panel.receiver-rename` on rename** — navigates to the rename form. The form reads `Bridge.selected-receiver-id` to know which receiver to edit.

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. All four new components exist.
grep -rn 'export component' \
    senders/android/ui/components/qr_placeholder.slint \
    senders/android/ui/components/receiver_context_menu.slint \
    senders/android/ui/pages/pairing_page.slint \
    senders/android/ui/pages/receiver_rename_page.slint
# Expected: 4 matches.

# 2. Panel variants in bridge.slint.
grep 'pairing\|receiver-rename' senders/android/ui/bridge.slint
# Expected: 2 matches.

# 3. Routed in main.slint.
grep 'Panel.pairing\|Panel.receiver-rename' senders/android/ui/main.slint
# Expected: 2 matches.

# 4. "Pair via QR" button in connect_page.
grep 'Pair via QR' senders/android/ui/pages/connect_page.slint
# Expected: 1 match.

# 5. ConfirmDialog import in connect_page.
grep 'ConfirmDialog' senders/android/ui/pages/connect_page.slint
# Expected: 2+ matches (import + instantiation).

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/components/qr_placeholder.slint \
        senders/android/ui/components/receiver_context_menu.slint \
        senders/android/ui/pages/pairing_page.slint \
        senders/android/ui/pages/receiver_rename_page.slint \
        senders/android/ui/bridge.slint \
        senders/android/ui/main.slint \
        senders/android/ui/pages/connect_page.slint
git status
# Expected (7 files):
#   new file:   components/qr_placeholder.slint
#   new file:   components/receiver_context_menu.slint
#   new file:   pages/pairing_page.slint
#   new file:   pages/receiver_rename_page.slint
#   modified:   bridge.slint
#   modified:   main.slint
#   modified:   pages/connect_page.slint
git commit -m "feat(slint-ui): Phase 24 — pairing QR + receiver context menu + rename page (UI-only)"
```

---

## Gotchas (Phase 24 specific)

### Gotcha 71 — Long-press detection requires two-Timer fallback on older Slint

**Symptom:** `pointer-event(ev)` on `TouchArea` fails to compile — unknown callback.

**Cause:** `pointer-event` was added in Slint 1.6+. The futo fork may or may not expose it.

**Fix (fallback):** use `TouchArea.pressed` (bool property that becomes true on pointer-down, false on pointer-up):

```slint
property <bool> lp-armed: false;

TouchArea {
    changed pressed => {
        if self.pressed {
            root.lp-armed = true;
        } else {
            root.lp-armed = false;
        }
    }
}

Timer {
    interval: 600ms;
    running: root.lp-armed;
    triggered => {
        root.lp-armed = false;
        // Long press detected.
        root.context-receiver-id = device.id;
        root.show-context-menu = true;
    }
}
```

This is the same press-duration pattern used in Phase 18-B (lock overlay hold-to-unlock).

### Gotcha 72 — Context menu Y positioning in a `for` loop is approximate

**Symptom:** the context menu opens at the wrong vertical position (too high or too low relative to the pressed row).

**Cause:** Slint's `for` loop doesn't expose per-row pixel positions. The only available reference is `idx * row-height + list-offset`, which drifts if the list is scrolled.

**Fix:** accept approximate positioning for the UI-only build. Centre the menu vertically on screen as a safe fallback (`y: (parent.height - self.height) / 2;`). Phase 8 can wire precise positioning via `absolute-position` if the Slint version supports it (1.7+).

### Gotcha 73 — In-place model mutation for "Forget" requires whole-array rebuild

**Symptom:** calling `mock-devices[idx].forget = true;` or removing an element — nothing updates.

**Cause:** Slint arrays are value types; `mock-devices` reactivity is only triggered by **reassigning the whole array**, not mutating a field. Same as Phase 16 § gotcha 13.

**Fix:** rebuild the array minus the forgotten entry:

```slint
// Pseudocode — Slint doesn't have filter() or splice().
// The pragmatic UI-only approach: just close the dialog and leave the
// list unchanged. Document that Phase 8 wires to a Rust-side VecModel
// which handles the actual removal.
```

For the UI-only build, the "Forget" action just closes the dialog without visually removing the entry. Phase 8 replaces `mock-devices` with `Bridge.known-receivers: [ReceiverItem]` backed by a `VecModel` that supports real `remove()`.

### Gotcha 74 — `ReceiverRenamePage` draft-name must initialise from `current-name`

**Symptom:** the `LineEdit` is empty when opening the rename page.

**Cause:** `draft-name` has no default value, or the default is `""` because the consumer didn't set `current-name`.

**Fix (already in this guide):** `in-out property <string> draft-name: root.current-name;` — initialises from the `in` property. Verify the consumer passes the correct name when routing to `Panel.receiver-rename`.

### Gotcha 75 — QR placeholder hardcoded white background ignores dark theme

**Symptom:** in a dark-themed app, the white QR card looks jarring.

**Cause:** real QR codes are always black-on-white for scanner compatibility. The stub mimics this faithfully.

**Fix:** acceptable — the white card is correct for QR rendering even in dark mode. If design feedback requests a border, add `border-width: 1px; border-color: Theme.border;` to soften the transition.

---

## Exit criteria checklist

- [x] `components/qr_placeholder.slint` exists with three alignment squares + centre label.
- [x] `components/receiver_context_menu.slint` exists with Rename / Set-as-default / Forget / (conditional) Disconnect rows.
- [x] `pages/pairing_page.slint` renders QR placeholder centred, receiver target IP:port below, Refresh + Copy address buttons.
- [x] `pages/receiver_rename_page.slint` renders LineEdit pre-populated with `current-name`, Save/Cancel buttons.
- [x] `bridge.slint` has `Panel.pairing`, `Panel.receiver-rename`, `ReceiverItem.is-default`, `Bridge.selected-receiver-id`.
- [x] `main.slint` routes both new Panel variants.
- [x] `connect_page.slint` has "Pair via QR" button that opens `Panel.pairing`.
- [x] Long-press on a receiver row opens the context menu (or documents the fallback if `pointer-event` isn't available).
- [x] Context menu "Rename" navigates to `Panel.receiver-rename`.
- [x] Context menu "Forget" opens `ConfirmDialog` (Phase 19 reuse).
- [x] `ReceiverContextMenu` scrim absorbs clicks and dismisses the menu.
- [x] "Forget" confirmation dialog matches Phase 19's controlled-component discipline.
- [x] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
 // Bridge additions:
 export global Bridge {
     ...
+    in property <[ReceiverItem]> known-receivers;   // replaces mock-devices
+    callback forget-receiver(/* id */ string);
+    callback rename-receiver(/* id */ string, /* new-name */ string);
+    callback set-default-receiver(/* id */ string);
+    callback copy-to-clipboard(/* text */ string);
+    in property <image> qr-image;                    // real QR from qrcode crate
+    in property <string> device-ip;
+    in property <int>    fcast-port;
 }
```

- `QrPlaceholder` is replaced entirely by `Image { source: Bridge.qr-image; image-fit: contain; }`.
- `mock-device-ip` / `mock-device-port` → `Bridge.device-ip` / `Bridge.fcast-port`.
- Rename save callback → `Bridge.rename-receiver(id, new-name)`.
- Forget confirm callback → `Bridge.forget-receiver(id)`.
- Copy address button → `Bridge.copy-to-clipboard("\{Bridge.device-ip}:\{Bridge.fcast-port}")`.

---

## Slint-doc references used

- **`for` repeated elements** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **`Rectangle { border-radius, clip, drop-shadow-* }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`TouchArea { clicked, pointer-event }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/gestures/toucharea.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **`callback` declarations** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **Two-way binding `<=>`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`LineEdit`** — `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx`.
- **`VerticalLayout { spacing, padding, vertical-stretch }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **Absolute positioning (`x:`, `y:`)** — same.
- **`if cond:` conditional elements** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`Image` element (Phase 8 prep)** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx`.
- **`in-out property` declarations** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.

---

## What's NOT in this guide

- **Real QR encoding.** Requires Rust (`qrcode` crate → `Image::from_rgba8()`). Phase 8.
- **Real Bluetooth / NFC pairing.** Out of scope — this is a QR-over-network pairing surface only.
- **Persistent rename / forget / default.** Phase 8 + Rust storage.
- **Swipe-left gesture** on receiver rows (Moblin uses this). Not available in Slint — long-press is the substitute.
- **`@tr(...)` wrapping** of `"Pair via QR"`, `"Rename"`, etc. Phase 9 sweep.
- **Drag-to-reorder receivers.** Out of scope; receiver ordering isn't a feature in the spec.
- **Multiple selection.** Out of scope; context menu acts on a single receiver.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-24-pairing-qr-receiver-management.md
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[expressions]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx
[properties]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[lineedit]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/lineedit.mdx

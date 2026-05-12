# MVP-PHASE-7 — Step 3: read `device.id` / `device.name` / `device.address` in the connect page

> Part 3 of 5. Parent doc: [`MVP-PHASE-7-receiver-item-promotion.md`](./MVP-PHASE-7-receiver-item-promotion.md).
> Previous: [Step 2 — `update_receivers_in_ui` rewrite](./MVP-PHASE-7-STEP-2-update-receivers-in-ui.md).
>
> **Doc-only.** Snippets are illustrative — no source tree files are
> modified by reading this guide.

---

## 0. Goal of this step

After Steps 1 + 2, `device` inside the connect-page iterator is a
`ReceiverItem` struct, not a raw `string`. Update the two places
where the iterator reads from `device`:

1. The long-press timer that captures the receiver id / name for
   the context menu (`device.id` + `device.name` instead of just
   `device`).
2. The visible row text (`device.name` as the primary line +
   `device.address` as the new secondary line).

After this step, the connect page renders a two-line list with
proper id-based context menu.

This is a **medium-sized Slint-only step** (~20 lines of Slint).

---

## 1. Pre-flight

### 1.1 Live state

| Component | Location |
|---|---|
| The connect-page receiver iterator | `senders/android/ui/pages/connect_page.slint:80-127` (post-PHASE-1) |
| Long-press timer | `connect_page.slint:90-100` |
| Row body (the `VerticalLayout` with the primary text) | `connect_page.slint:113-126` |
| `context-receiver-id` + `context-receiver-name` `in-out property`s | `connect_page.slint:18-19` |
| `device` (iterator variable) typing | becomes `ReceiverItem` after Step 1 |

### 1.2 Why two separate captures (`id` + `name`)

The context menu shows the receiver name (for human readability),
but the user's actions (Connect, Set Default, Forget) need to
operate on a stable id. With `[string]`, those were the same value.
With `[ReceiverItem]`, they diverge:

- `id` is the mDNS service name (stable identifier).
- `name` is the displayed text (may eventually be customisable by
  the user; for now, identical to `id`).

Capturing both separately means future PHASE-24 work (custom
display names, persistent storage) can change them independently
without touching the context-menu code.

### 1.3 Why a two-line row (name + address)

PHASE-1 collapsed the row to a single line because `device` was a
raw `string` — the only available datum. With the full
`ReceiverItem`, restoring the secondary address line makes the UI
significantly clearer:

- Helps the user pick between receivers on the same LAN with
  similar names (e.g. "FCast Receiver" appearing twice).
- Matches ChromeCast / Roku conventions.

---

## 2. The change

### 2.1 Long-press timer

**File:** `senders/android/ui/pages/connect_page.slint`

**Before** (post-PHASE-1, lines 90-100):

```slint
Timer {
    interval: 600ms;
    running: parent.lp-armed;
    triggered => {
        parent.lp-armed = false;
        // MVP: Bridge.devices is [string]. We don't have a stable id
        // yet (see MVP-PHASE-7). Use the receiver name for both fields.
        root.context-receiver-id = device;
        root.context-receiver-name = device;
        root.context-menu-y = (parent.height * idx) + 100px;
        root.show-context-menu = true;
    }
}
```

**After:**

```slint
Timer {
    interval: 600ms;
    running: parent.lp-armed;
    triggered => {
        parent.lp-armed = false;
        // Long press detected — capture the receiver's stable id
        // (for actions) and its display name (for the menu header).
        root.context-receiver-id = device.id;
        root.context-receiver-name = device.name;
        root.context-menu-y = (parent.height * idx) + 100px;
        root.show-context-menu = true;
    }
}
```

The MVP-comment is removed because the workaround no longer
applies.

### 2.2 Row body (the visible list item)

**Before** (post-PHASE-1, lines 113-126):

```slint
VerticalLayout {
    padding-left: Theme.padding-screen;
    padding-right: Theme.padding-screen;
    alignment: center;

    Text {
        text: device;
        color: Theme.text-primary;
        font-size: Theme.font-size-body;
        overflow: elide;
    }
    // Secondary address row removed: Bridge.devices is [string].
    // Restored in MVP-PHASE-7 once promoted to [ReceiverItem].
}
```

**After:**

```slint
VerticalLayout {
    padding-left: Theme.padding-screen;
    padding-right: Theme.padding-screen;
    alignment: center;
    spacing: 2px;

    Text {
        text: device.name;
        color: Theme.text-primary;
        font-size: Theme.font-size-body;
        overflow: elide;
    }
    Text {
        text: device.address;
        color: Theme.text-secondary;
        font-size: Theme.font-size-label;
        overflow: elide;
    }
}
```

### 2.3 Don't forget `spacing: 2px`

Without it, the two `Text { }` blocks render touching each other
— visually heavy. `2px` matches the existing convention in
`recent-receivers-page.slint` (if present) for two-line rows.

### 2.4 Why `Theme.text-secondary` for the address

Matches existing conventions for de-emphasised secondary text.
`Theme.text-secondary` is the muted variant; `Theme.text-primary`
is the bold variant. Search the Slint theme file
(`senders/android/ui/theme.slint`) for the exact tokens if you've
diverged.

### 2.5 `device.address` formatting in IPv6 case

Step 2's `update_receivers_in_ui` wraps IPv6 in `[...]`. So
`device.address` becomes e.g. `[fe80::1]:46899` for an IPv6
receiver. The `Text` element renders it verbatim — `overflow:
elide;` handles the case where the text is too wide for the row.

---

## 3. Verification

### 3.1 Slint compile

```bash
cargo +nightly check -p fcast-sender-android --target aarch64-linux-android
```

Expect **clean** once Steps 1 + 2 + 3 are all in. Most likely
failures:

- `error: type 'string' is incompatible with type 'ReceiverItem'`
  — the Step 1 type change didn't land. Re-check `bridge.slint`.
- `error: field 'id' is not declared on struct 'ReceiverItem'` —
  the struct in `bridge.slint:110-118` doesn't match what Step 1
  expects. Verify the struct declaration is unchanged.

### 3.2 Visual smoke

Launch the app on a device with ≥ 2 discoverable receivers:

- Each row should show two lines (name + address).
- Long-pressing a row should pop up the context menu titled with
  the **name**, but its actions (Connect / Set Default / Forget)
  operate on the **id**.

### 3.3 Grep

```bash
grep -nE 'device\.(id|name|address)' senders/android/ui/pages/connect_page.slint
# → at least 4 matches:
#   - device.id (in Timer.triggered)
#   - device.name (in Timer.triggered + row primary text)
#   - device.address (in row secondary text)
```

---

## 4. Pitfalls specific to this step

### P1 — Forgetting `device.id` ≠ `device` after PHASE-7

Tempted to copy the PHASE-1 single-line capture (`device`,
`device`). With `[ReceiverItem]`, `device` is a struct — using it
where a `string` is expected fails to compile. Always go through
`device.<field>`.

### P2 — Hardcoding the secondary line format in Slint

Tempted (it's where the user sees it):

```slint
Text { text: device.ip + ":" + device.port; }
```

Don't. The format is already done in Rust (Step 2 §2.3), and
including the `[...]` IPv6 brackets in Slint string concatenation
is awkward. Pre-formatting in Rust keeps the Slint side trivial.

### P3 — Don't change `Theme.text-primary` to bold

The primary line is already bold via `font-size: Theme.font-size-body;`.
Adding `font-weight: 600;` would be inconsistent with the rest of
the connect page. Match the existing token set.

### P4 — `overflow: elide` on a too-narrow row

If the `padding-screen` consumes too much width, the address will
elide aggressively (e.g. `192.168.1.4...`). Slint's elide is
left-anchored, so the port disappears first. For PHASE-7 scope,
this is fine — the user can tap to connect, no need to read the
full address. Future polish: a small `MaxWidth` constraint or
two-line wrap.

### P5 — `context-receiver-name` shown in the menu header

If the context menu currently displays `context-receiver-id`, the
menu header will show the raw mDNS service name (e.g.
`"FCast Receiver._fcast._tcp.local."`) — ugly. Make sure the menu
header reads `context-receiver-name`:

```bash
grep -n 'context-receiver-' senders/android/ui/pages/connect_page.slint
```

Verify the header uses `-name` and the actions use `-id`.

### P6 — `device.id` for `Bridge.connect-receiver(...)`

That's [Step 4](./MVP-PHASE-7-STEP-4-click-handler-passes-id.md)'s
job — don't pre-empt it here. The MVP-comment in the existing
click handler is still accurate post-Step-3.

---

## 5. Next step

Once this lands, [Step 4](./MVP-PHASE-7-STEP-4-click-handler-passes-id.md)
updates the click handler from
`Bridge.connect-receiver(device)` to
`Bridge.connect-receiver(device.id)`.

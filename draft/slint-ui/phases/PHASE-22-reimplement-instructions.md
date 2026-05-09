# Phase 22 — Network Interface & Wi-Fi Aware reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-22-network-interface-wifi-aware.md`][spec] to the current `senders/android` tree.
**Goal:** add a `NetworkPage` settings sub-page that lists stub network interfaces (Wi-Fi / cellular / loopback) with per-row enable toggle + tap-to-expand details, plus a Wi-Fi Aware (NAN) opt-in toggle with a placeholder banner. Wired into the `Panel` overlay layer; linked from `FullSettingsPage` under a **new** `ADVANCED` section.
**Scope:** Slint UI only. **No Rust changes.** No real `NetworkInterface.getNetworkInterfaces()` enumeration; no permission flow.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-22-network-interface-wifi-aware.md

> **Read [`PHASE-14-reimplement-instructions.md`][p14] first** for chrome and row primitives. The new things in Phase 22: an inline expander pattern (per-row local boolean), a transient banner that auto-hides, and the introduction of a brand-new `ADVANCED` section in `FullSettingsPage`.

[p14]: ./PHASE-14-reimplement-instructions.md

---

## Why this guide exists

Phase 22 is the first phase to add a **new top-level section** to `FullSettingsPage` (`ADVANCED`). Phases 14/15/16 all extended an existing section (`AUDIO & VIDEO`); Phase 21 modified `ABOUT`. Phase 22 inserts `ADVANCED` between `CODEC & DEBUG` and `ABOUT & SUPPORT`. Subsequent phases (23 = recording, 26 = debug log) will extend the same `ADVANCED` section.

The spec's main subtleties:

1. **Per-row expander state.** Each row stores its own `expanded: bool` so multiple rows can be expanded independently (or, if you want exclusive expansion, hold a `mock-expanded-id: string` on the page root). The spec calls out the local-property approach. Implementing this in Slint requires the row to be a sub-component (cannot store local state inside an inline `for` block — the loop body is stateless across iterations).
2. **Transient banner.** The Wi-Fi Aware toggle flips, then a banner appears for ~3s and auto-hides. This needs a `Timer` element gated on a "banner visible" property, which the timer itself clears.
3. **No `Bridge.network-interfaces` property.** The spec says struct is placeholder-only.

After Phases 14 + 15 + 16 + 21 merge:

- `Panel { none, settings, debug, codec-test, audio, camera, bitrate-presets, bitrate-preset-edit, about, version-history, attributions, help }`. Phase 22 adds **one** variant: `network`.
- `FullSettingsPage` has sections: `RECEIVER`, `VIDEO QUALITY`, `AUDIO & VIDEO`, `CODEC & DEBUG`, `ABOUT & SUPPORT`. Phase 22 inserts `ADVANCED` between `CODEC & DEBUG` and `ABOUT & SUPPORT`.

This is **strictly additive** Slint work spread across **three existing files** plus **one new file**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'NetworkPage\|NetworkInterface\|Panel\.network\|mock-interfaces\|mock-wifi-aware-enabled' \
    senders/android/ui/

# No ADVANCED section yet:
grep -n 'ADVANCED' senders/android/ui/pages/settings_page.slint
# Expected: (empty)
```

After this guide is applied:

```sh
grep -n 'export struct NetworkInterface' senders/android/ui/bridge.slint   # Expected: 1
grep -n 'network,' senders/android/ui/bridge.slint                         # Expected: 1 (in Panel enum)
grep -n 'Panel\.network' senders/android/ui/main.slint                     # Expected: 1
grep -n 'export component NetworkPage' senders/android/ui/pages/network_page.slint  # Expected: 1
grep -n 'ADVANCED' senders/android/ui/pages/settings_page.slint            # Expected: 1
grep -n 'Panel\.network' senders/android/ui/pages/settings_page.slint      # Expected: 1
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-22-network-page
cargo check -p android-sender
```

---

## Step 1 — Add `NetworkInterface` struct + `Panel.network` in `bridge.slint`

```diff
 export struct BitratePreset {
     id:           string,
     name:         string,
     bitrate-kbps: int,
     active:       bool,
 }
+
+export struct NetworkInterface {
+    name:        string,
+    kind:        string,    // "wifi" / "ethernet" / "cellular" / "loopback"
+    address-v4:  string,
+    address-v6:  string,
+    enabled:     bool,
+}
```

```diff
 export enum Panel {
     ...
     attributions,
     help,
+    network,
 }
```

Why `kind: string` not an enum? Slint enums are closed; user-defined kinds (e.g. "tunnel0") would be rejected if the Rust side ever publishes anything outside the Phase 8 closed set. The Moblin model uses an open enum (raw string), and a string here keeps Phase 8 free to change its mind.

---

## Step 2 — Route `Panel.network` in `main.slint`

```diff
 import { HelpPage }                     from "pages/help_page.slint";
+import { NetworkPage }                  from "pages/network_page.slint";
```

```diff
     if Bridge.active-panel == Panel.help:            HelpPage { }
+    if Bridge.active-panel == Panel.network:         NetworkPage { }
 }
```

---

## Step 3 — Create `pages/network_page.slint`

**File:** `senders/android/ui/pages/network_page.slint` (new)

The interface row is a sub-component because it needs local `expanded` state — Slint `for` loop bodies are not sub-components, so any state must live in a real component declaration.

### New file

```slint
// network_page.slint — Network interfaces list + Wi-Fi Aware opt-in (UI-only).
//
// Reachable from FullSettingsPage's "Network" row (sets
// `Bridge.active-panel = Panel.network`). All toggles flip inline state;
// no real interface enumeration, no Wi-Fi Aware permission flow. Phase 8
// will swap mock-interfaces for a Bridge-published list and the Wi-Fi
// Aware toggle for a real WifiAwareManager opt-in.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/scrollview.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx

import { ScrollView } from "std-widgets.slint";
import { Bridge, Panel, NetworkInterface } from "../bridge.slint";
import { Theme } from "../theme.slint";
import { TextButton } from "../components/buttons.slint";
import {
    SettingsSection,
    SettingsToggleRow,
} from "../components/settings_rows.slint";

// Internal sub-component — needs its own `expanded: bool` so the page can
// have multiple rows expanded simultaneously. Cannot live as inline
// markup inside the for-loop body because the loop body has no
// per-iteration component instance to attach state to.
component NetworkInterfaceRow inherits Rectangle {
    in property <NetworkInterface> data;
    callback toggle-enabled(bool);

    property <bool> expanded: false;

    border-radius: Theme.radius-card;
    background: Theme.surface-card;

    // Element height grows when expanded — using a `min-height` instead of
    // `height` allows the inner content to size naturally.
    min-height: 64px;

    VerticalLayout {
        // ── Always-visible row (collapsed view) ─────────────────────────
        Rectangle {
            height: 64px;
            ta := TouchArea {
                clicked => { root.expanded = !root.expanded; }
            }
            HorizontalLayout {
                padding-left:  Theme.padding-screen;
                padding-right: Theme.padding-screen;
                spacing: Theme.spacing-default;

                // Kind icon — single-glyph fallback while real assets land later.
                Rectangle {
                    width: 32px;
                    height: 32px;
                    border-radius: 8px;
                    background: Theme.accent-active.darker(20%);
                    Text {
                        text:
                            root.data.kind == "wifi"     ? "W" :
                            root.data.kind == "ethernet" ? "E" :
                            root.data.kind == "cellular" ? "M" :
                                                           "•";
                        color: Theme.text-primary;
                        horizontal-alignment: center;
                        vertical-alignment: center;
                        font-size: Theme.font-size-body;
                    }
                }

                VerticalLayout {
                    alignment: center;
                    horizontal-stretch: 1;
                    Text {
                        text: root.data.name;
                        color: Theme.text-primary;
                        font-size: Theme.font-size-body;
                    }
                    Text {
                        text: root.data.address-v4 == ""
                            ? "(no IPv4 address)"
                            : root.data.address-v4;
                        color: Theme.text-secondary;
                        font-size: Theme.font-size-label;
                        // Defend against very long IPv6 strings.
                        overflow: elide;
                    }
                }

                // Enable toggle — uses the same SettingsToggleRow row
                // pattern but inline (no title, just the toggle).
                Rectangle {
                    width: 56px;
                    Text {
                        text: root.data.enabled ? "On" : "Off";
                        color: root.data.enabled
                            ? Theme.accent-active
                            : Theme.text-secondary;
                        horizontal-alignment: end;
                        vertical-alignment: center;
                        font-size: Theme.font-size-label;
                    }
                    TouchArea {
                        clicked => { root.toggle-enabled(!root.data.enabled); }
                    }
                }
            }
        }

        // ── Expanded section ────────────────────────────────────────────
        if root.expanded: Rectangle {
            VerticalLayout {
                padding-left:  Theme.padding-screen;
                padding-right: Theme.padding-screen;
                padding-bottom: Theme.padding-screen;
                spacing: 6px;
                Text {
                    text: "IPv6: " + (root.data.address-v6 == "" ? "(none)" : root.data.address-v6);
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                    overflow: elide;
                }
                Text {
                    text: "Kind: " + root.data.kind;
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                }
                // The "Use for cast traffic" toggle is purely visual in
                // UI-only build — it's not even connected to a stub
                // property because the spec is explicit that per-interface
                // routing is deferred. Render the row but do not wire it.
            }
        }
    }
}

export component NetworkPage inherits Rectangle {
    // ── UI-only stub state ──────────────────────────────────────────────
    in-out property <[NetworkInterface]> mock-interfaces: [
        { name: "wlan0",  kind: "wifi",     address-v4: "192.168.1.42",  address-v6: "fe80::1234", enabled: true  },
        { name: "rmnet0", kind: "cellular", address-v4: "10.20.30.40",   address-v6: "",           enabled: false },
        { name: "lo",     kind: "loopback", address-v4: "127.0.0.1",     address-v6: "::1",        enabled: true  },
    ];
    in-out property <bool> mock-wifi-aware-enabled: false;
    property <bool>        banner-visible:          false;

    width: 100%;
    height: 100%;
    background: Theme.surface-primary;

    // Auto-hide banner timer. Re-arms whenever banner-visible flips to true.
    Timer {
        interval: 3s;
        running: root.banner-visible;
        triggered => { root.banner-visible = false; }
    }

    // Helper: rebuild mock-interfaces with one row's enabled flipped.
    // Same in-place-mutation caveat as Phase 16's preset selector.
    function set-enabled(name: string, value: bool) {
        root.mock-interfaces = [
            { name: root.mock-interfaces[0].name, kind: root.mock-interfaces[0].kind,
              address-v4: root.mock-interfaces[0].address-v4,
              address-v6: root.mock-interfaces[0].address-v6,
              enabled: root.mock-interfaces[0].name == name ? value : root.mock-interfaces[0].enabled },
            { name: root.mock-interfaces[1].name, kind: root.mock-interfaces[1].kind,
              address-v4: root.mock-interfaces[1].address-v4,
              address-v6: root.mock-interfaces[1].address-v6,
              enabled: root.mock-interfaces[1].name == name ? value : root.mock-interfaces[1].enabled },
            { name: root.mock-interfaces[2].name, kind: root.mock-interfaces[2].kind,
              address-v4: root.mock-interfaces[2].address-v4,
              address-v6: root.mock-interfaces[2].address-v6,
              enabled: root.mock-interfaces[2].name == name ? value : root.mock-interfaces[2].enabled },
        ];
    }

    VerticalLayout {
        // ── Header ──────────────────────────────────────────────────────
        Rectangle {
            height: 56px;
            background: Theme.surface-card;
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Network";
                    color: Theme.text-primary;
                    font-size: Theme.font-size-heading;
                    vertical-alignment: center;
                    horizontal-stretch: 1;
                }
                TextButton {
                    label: "Done";
                    clicked => { Bridge.active-panel = Panel.none; }
                }
            }
        }

        // ── Banner (animated visibility) ────────────────────────────────
        if root.banner-visible: Rectangle {
            height: 40px;
            background: Theme.accent-active.darker(20%);
            HorizontalLayout {
                padding: Theme.padding-screen;
                Text {
                    text: "Wi-Fi Aware enabled (placeholder — no permission requested).";
                    color: Theme.text-primary;
                    vertical-alignment: center;
                    font-size: Theme.font-size-label;
                }
            }
        }

        // ── Body ────────────────────────────────────────────────────────
        ScrollView {
            VerticalLayout {
                spacing: Theme.spacing-default;
                padding: Theme.padding-screen;

                Text {
                    text: "INTERFACES";
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                }
                for iface in root.mock-interfaces: NetworkInterfaceRow {
                    data: iface;
                    toggle-enabled(value) => { root.set-enabled(iface.name, value); }
                }

                SettingsSection {
                    title: "WI-FI AWARE";
                    SettingsToggleRow {
                        title: "Enable Wi-Fi Aware discovery";
                        checked: root.mock-wifi-aware-enabled;
                        toggled(checked) => {
                            root.mock-wifi-aware-enabled = checked;
                            // Show transient banner only on enable transitions.
                            if (checked) {
                                root.banner-visible = true;
                            }
                        }
                    }
                }

                Text {
                    text: "Wi-Fi Aware (NAN) requires location permission. This build "
                        + "is UI-only — toggling the switch does not request the "
                        + "permission or open a Wi-Fi Aware session.";
                    color: Theme.text-secondary;
                    font-size: Theme.font-size-label;
                    wrap: word-wrap;
                }
            }
        }
    }
}
```

### Why each piece

- **`NetworkInterfaceRow` as a real `component`** — the local `expanded: bool` state cannot live in `for iface in mock-interfaces: Rectangle { property <bool> expanded; ... }`. Slint's `for` loop body is rebuilt on each model change; properties declared inside the body do not persist per iteration. A real sub-component instance per iteration **does** persist its state for as long as the model entry exists. See [repetition-and-data-models.mdx][repeat] (sub-component instantiation in `for`).
- **`property <bool> expanded: false` (no `in-out`/`in`)** — internal property, not exposed to consumers. Default property direction is `private`. Per [properties.mdx][props].
- **`Timer { interval: 3s; running: root.banner-visible; triggered => { ... } }`** — Slint's [`Timer`][timer] only runs while `running == true`; setting `banner-visible = true` arms it; the timer body sets it back to `false` after `interval`, which both stops the timer and hides the banner. Re-flipping `banner-visible` to `true` re-arms.
- **`if root.banner-visible: Rectangle { ... }`** in the layout — banner appears in the layout flow. Conditional elements inside layouts are part of the same flow when present and absent when not, so `VerticalLayout` re-flows automatically.
- **`set-enabled(name, value)` rebuilds the array** — same Phase-16 in-place-mutation caveat. For 3 entries this is acceptable; a longer list would warrant a Bridge-side helper.
- **Inline toggle area** uses a `TouchArea + Text` rather than `SettingsToggleRow` because the row already has its own clickable surface (the whole expander); embedding a full `SettingsToggleRow` would create overlapping touch targets. The inline `Off`/`On` label + tappable rectangle is the cleanest pattern.

### Build check

```sh
cargo check -p android-sender
```

---

## Step 4 — Add `ADVANCED` section in `FullSettingsPage`

**File:** `senders/android/ui/pages/settings_page.slint`

Insert a new `ADVANCED` section between `CODEC & DEBUG` and `ABOUT & SUPPORT`.

### Diff

```diff
                 // ── Section: CODEC & DEBUG ────────────────────────────────
                 SettingsSection {
                     title: "CODEC & DEBUG";
                     SettingsValueRow {
                         title: "H.264 encoder test";
                         value: "Open";
                         clicked => { Bridge.active-panel = Panel.codec-test; }
                     }
                     SettingsToggleRow {
                         title: "Show debug panel";
                         checked: root.debug-panel;
                         toggled(checked) => { root.debug-panel = checked; }
                     }
                 }

+                // ── Section: ADVANCED ─────────────────────────────────────
+                SettingsSection {
+                    title: "ADVANCED";
+                    SettingsValueRow {
+                        title: "Network";
+                        value: "Open";
+                        clicked => { Bridge.active-panel = Panel.network; }
+                    }
+                }
+
                 // ── Section: ABOUT & SUPPORT ──────────────────────────────
                 SettingsSection {
                     title: "ABOUT & SUPPORT";
```

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. NetworkInterface struct + Panel.network present.
grep -n 'export struct NetworkInterface\|^\s*network,\b' senders/android/ui/bridge.slint
# Expected: 2 matches.

# 2. Page exists.
grep -n 'export component NetworkPage\|component NetworkInterfaceRow' \
    senders/android/ui/pages/network_page.slint
# Expected: 2 matches (sub-component + page).

# 3. Page uses a Timer for the auto-hide banner.
grep -n 'Timer {' senders/android/ui/pages/network_page.slint
# Expected: 1 match.

# 4. ADVANCED section in FullSettingsPage.
grep -n 'ADVANCED\|Panel\.network' senders/android/ui/pages/settings_page.slint
# Expected: 2 matches (title + Panel.network handler).

# 5. main.slint routes Panel.network.
grep -n 'Panel\.network' senders/android/ui/main.slint
# Expected: 1 match.

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/
git status
# Expected (4 files):
#   modified:   senders/android/ui/bridge.slint
#   modified:   senders/android/ui/main.slint
#   modified:   senders/android/ui/pages/settings_page.slint
#   new file:   senders/android/ui/pages/network_page.slint
git commit -m "feat(slint-ui): Phase 22 — network interfaces & Wi-Fi Aware opt-in (UI-only)"
```

---

## Gotchas (Phase 22 specific)

### Gotcha 13 — Per-iteration state requires a sub-component

**Symptom:** writing `for iface in mock-interfaces: Rectangle { property <bool> expanded; ... }` either fails to compile or re-renders all rows with `expanded == false` whenever any other state changes.

**Cause:** Slint `for` loop bodies are not their own components. Local `property` declarations in the body have no per-iteration identity — the framework cannot track "which iteration owns which value" because the body is recomputed from scratch when the model changes.

**Fix:** declare `NetworkInterfaceRow` as a real component (with `component RowName inherits ...` syntax) and instantiate it inside the loop. Slint creates one instance per model entry, each with its own `expanded` state, and reuses instances when the model permutes (much like keyed children in React).

### Gotcha 14 — `Timer { running: <expr> }` is reactive

**Symptom:** banner appears for 3s then disappears, but enabling Wi-Fi Aware again immediately doesn't re-show the banner — it only flashes briefly or doesn't appear at all.

**Cause:** the `Timer.triggered` body sets `banner-visible = false`, which clears `running`, which destroys the timer's pending tick. If you set `banner-visible = true` *during* a tick callback, the timer might not see the rising edge cleanly.

**Fix (already in the snippet):** only set `banner-visible = true` from the toggle's `toggled(checked)` handler, which runs synchronously outside the timer tick. The timer's `triggered` body should *only* clear the property, never re-arm — re-arming is the toggle's job.

### Gotcha 15 — Long IPv6 addresses overflow the row

**Symptom:** rows with `address-v6: "2001:db8:85a3:8d3:1319:8a2e:370:7344"` blow out the row width and clip the toggle on the right.

**Fix:** add `overflow: elide;` to the IPv6 `Text` element (already in the snippet). Per [text.mdx][text], `overflow: elide` clips at the right with `…`. Without it, `Text` defaults to no clipping.

---

## Exit criteria checklist

- [ ] `bridge.slint` exports `NetworkInterface` struct and `Panel.network` variant.
- [ ] `main.slint` routes `Panel.network`.
- [ ] `NetworkPage` lists 3 stub interfaces with kind glyph + name + IPv4 address.
- [ ] Tapping a row flips its `expanded` flag — the IPv6 / kind details slide into view, others stay collapsed.
- [ ] Per-row enable toggle flips the `enabled` flag (whole-array reassignment).
- [ ] Wi-Fi Aware toggle flips state; on enable, a banner appears for 3s and then auto-hides.
- [ ] `FullSettingsPage` has a new `ADVANCED` section with a `Network` row.
- [ ] `cargo build -p android-sender` passes.

---

## When Phase 8 reactivates

```diff
+    in property <[NetworkInterface]> network-interfaces;
+    in-out property <bool>           wifi-aware-enabled;
+    callback set-interface-enabled(string, bool);
+    callback request-wifi-aware-permission();
```

- `network-interfaces` ← Rust polls `NetworkInterface.getNetworkInterfaces()` periodically; pushes via `slint::Weak`.
- `set-interface-enabled(name, bool)` → JNI to bind/unbind the interface for cast traffic. Real implementation may require root; alternatively, FCast tracks "preferred interfaces" in app config and the Rust networking layer respects that.
- `request-wifi-aware-permission()` → JNI to launch `ActivityCompat.requestPermissions(ACCESS_FINE_LOCATION, ...)`. The toggle's `toggled(checked)` should call this when `checked == true`; a callback on permission grant flips `wifi-aware-enabled` from Rust.

---

## Slint-doc references used

- **`export struct NetworkInterface`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`.
- **Sub-component declaration `component NetworkInterfaceRow inherits Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **Per-iteration state via sub-component instances in `for ...`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`.
- **Conditional element inside a layout `if root.banner-visible: Rectangle { ... }`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx`.
- **`Timer { interval, running, triggered }`** — `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`.
- **`Text.overflow: elide`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`.
- **Conditional ternary `cond ? "x" : "y"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`if (checked) { ... }` imperative-style block in a callback body** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`.
- **`SettingsSection`, `SettingsToggleRow`** — FCast components in `senders/android/ui/components/settings_rows.slint`.

---

## What's NOT in this guide

- **Real `NetworkInterface.getNetworkInterfaces()` enumeration.** Phase 8.
- **Real interface enable/disable.** Requires root or VPN profile; Phase 8 + new Android permission UX.
- **Real Wi-Fi Aware (NAN) permission flow + session lifecycle.** Phase 8.
- **Per-interface routing for cast traffic.** Phase 8 + Rust networking layer.
- **"Use for cast traffic" toggle wiring.** Spec defers this; the row is rendered visually only.
- **`@tr(...)` wrapping** — Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-22-network-interface-wifi-aware.md
[p14]: ./PHASE-14-reimplement-instructions.md
[file]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/file.mdx
[props]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx
[repeat]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx
[timer]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx
[text]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx

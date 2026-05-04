# Phase 41 — Streaming Navigation Overlay Placeholder

> **UI-only.** Floating navigation overlay that lets the streamer flip between
> scenes / quick-actions / chat / replay using a side-popping menu.
> **No real navigation, no real mode-switching.**

**Status:** `[ ] Not started`
**Depends on:** Phases 1, 2, 3, 4, 33 (scenes), 39 (HUD)
**Functional integration:** Permanently deferred.

**Moblin source analogues** (1 file):
- `View/Stream/Overlay/StreamOverlayNavigationView.swift`

**Files to add:**
- `senders/android/ui/components/streaming_nav_overlay.slint`

---

## Goal

Render Moblin's streaming navigation overlay — a popover-style menu with
quick links to scenes, chat, replay, settings, etc.

---

## Moblin source pattern

```swift
// StreamOverlayNavigationView.swift (excerpt)
ZStack {
    Color.black.opacity(0.5)
    VStack(spacing: 16) {
        Button("Scenes")  { /* show scene picker */ }
        Button("Chat")    { /* open chat */ }
        Button("Replay")  { /* open replay viewer */ }
        Button("Settings"){ /* open settings */ }
    }
}
.onTapGesture { dismiss() }
```

A modal scrim with a centered button stack.

---

## Tasks

### 41-A — `StreamingNavOverlay`

```slint
export struct NavLink {
    id: string,
    label: string,
    icon: string,           // unicode glyph fallback
    panel: Panel,
}

export component StreamingNavOverlay inherits Rectangle {
    in-out property <[NavLink]> mock-links: [
        { id: "scenes",   label: "Scenes",         icon: "🎬", panel: Panel.scenes },
        { id: "chat",     label: "Chat overlay",    icon: "💬", panel: Panel.chat-settings },
        { id: "replay",   label: "Replay buffer",  icon: "⏪", panel: Panel.replay-settings },
        { id: "right-hud",label: "Right HUD",       icon: "🎚", panel: Panel.none /* toggles HUD */ },
        { id: "settings", label: "Settings",        icon: "⚙",  panel: Panel.settings },
    ];

    background: #000000aa;
    visible: Bridge.show-streaming-nav;       // bool flipped by a quick-action

    ta := TouchArea {
        clicked => { Bridge.show-streaming-nav = false; }
    }

    VerticalLayout {
        alignment: center;
        spacing: Theme.spacing-md;
        for link in root.mock-links: Rectangle {
            width: 240px; height: 56px;
            background: Theme.surface-card;
            border-radius: Theme.radius-card;
            HorizontalLayout {
                padding: Theme.spacing-md; spacing: 12px;
                Text { text: link.icon; font-size: 24px;
                       vertical-alignment: center; }
                Text { text: link.label; color: Theme.text-primary;
                       font-size: Theme.font-size-body;
                       vertical-alignment: center; }
            }
            ta := TouchArea {
                clicked => {
                    Bridge.active-panel = link.panel;
                    Bridge.show-streaming-nav = false;
                }
            }
        }
    }
}
```

### 41-B — Wire toggle into bar

- [ ] Add a quick-action `id: "nav-overlay"` to the Phase 4 bar that
  flips `Bridge.show-streaming-nav` (a new `in-out property <bool>`
  on `Bridge` — added here rather than Phase 1 because it is
  navigation-specific).

### 41-C — Embed in `MainWindow`

- [ ] Render `StreamingNavOverlay` as the topmost child of `MainWindow`
  so the scrim covers everything.

---

## Exit criteria

1. Tapping the "nav-overlay" quick action shows a black scrim with 5
   centred buttons.
2. Tapping a button routes via `Bridge.active-panel` and dismisses the
   overlay.
3. Tapping the scrim dismisses without routing.
4. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real "swipe in from the side" gesture (the overlay opens on click only).
- Animated scrim fade-in / fade-out.
- Per-quick-action shortcuts.

---

## Slint best practices applied here

- **Topmost child = topmost z-order** in Slint — declaration order
  determines paint order, so `MainWindow` keeps `StreamingNavOverlay`
  at the bottom of its children list.
- **`background: #000000aa;`** for a translucent scrim — RGBA hex
  literal works directly as a `color`.
- **Two `TouchArea`s** (one on the scrim, one per button) — Slint
  routes click events to the deepest match, so button taps are not
  swallowed by the scrim.

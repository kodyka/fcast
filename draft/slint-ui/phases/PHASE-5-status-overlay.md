# Phase 5 — Status Overlay

> Add a transparent heads-up overlay to the casting screen showing live connection
> and encoder status from Rust. Inspired by Moblin `View/Stream/StreamOverlayView`.
> Reference: `draft/moblin-ui/Moblin/View/Stream/`

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 4 (control bar height defined)
**Unlocks:** Phase 8 (Rust bridge feeds status items)
**Blocked by:** `TODO.codecs.md` P0-1 (encoder name only available once `amcvidenc` selection works)
**Related files:**
- `senders/android/ui/components/status_overlay.slint` — stub from Phase 1-H
- `senders/android/ui/bridge.slint` — `StatusItem` struct added here
- `senders/android/ui/pages/casting_page.slint` — overlay layered here
- `senders/android/src/lib.rs` — Rust populates `status-items` from cast state

**Moblin source analogues (reference only):**

| Moblin file | Slint equivalent |
|---|---|
| `StreamOverlayView.swift` | `StatusOverlay` |
| `Overlay/LeftOverlayView.swift` | left-side status items |
| `Overlay/RightOverlayView.swift` | right-side status items (defer) |
| `CameraLevelView.swift` | not applicable — no camera level in FCast sender |
| `DrawOnStreamView.swift` | not applicable — defer |

---

## Tasks

### 5-A — Add `StatusItem` struct to `bridge.slint`

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```
  export struct StatusItem {
      label:    string,
      value:    string,
      severity: string,   // "info" | "warning" | "error"
  }
  ```

- [ ] Add to `Bridge`:

  ```
      in property <[StatusItem]> status-items: [];
  ```

- [ ] **Build check.**

---

### 5-B — Implement `StatusPill` sub-component

- [ ] Open `senders/android/ui/components/status_overlay.slint`.
- [ ] Add imports: `import { Theme } from "../theme.slint"; import { StatusItem } from "../bridge.slint";`.
- [ ] Implement internal (non-exported) `component StatusPill`:

  ```
  component StatusPill inherits Rectangle {
      in property <StatusItem> item;

      height: 28px;
      border-radius: Theme.radius-pill;
      background: root.item.severity == "error"   ? Theme.error
                : root.item.severity == "warning" ? Theme.warning
                :                                   Theme.surface-overlay;

      HorizontalLayout {
          padding-left:  Theme.padding-card;
          padding-right: Theme.padding-card;
          spacing: 6px;
          Text {
              text: root.item.label;
              color: Theme.text-secondary;
              font-size: Theme.font-size-label;
              vertical-alignment: center;
          }
          Text {
              text: root.item.value;
              color: Theme.text-primary;
              font-size: Theme.font-size-label;
              vertical-alignment: center;
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 5-C — Implement `StatusOverlay` component

- [ ] Implement `export component StatusOverlay`:

  ```
  export component StatusOverlay inherits Rectangle {
      in property <[StatusItem]> items;

      background: transparent;
      // Do not show overlay at all when there are no items
      visible: root.items.length > 0;

      VerticalLayout {
          padding: Theme.padding-card;
          spacing: 4px;
          alignment: start;

          for item in root.items: StatusPill { item: item; }
      }
  }
  ```

- [ ] **Build check.**

---

### 5-D — Layer `StatusOverlay` into `casting_page.slint`

The casting page needs a `ZStack`-style layering: the status overlay floats above the
existing content without affecting layout flow.

- [ ] Open `senders/android/ui/pages/casting_page.slint`.
- [ ] Import `StatusOverlay` from `../components/status_overlay.slint`.
- [ ] Import `Bridge` from `../bridge.slint`.
- [ ] Wrap `CastingView` content in a parent `Rectangle` that uses absolute positioning:

  ```
  export component CastingView inherits Rectangle {
      // existing content layout:
      VerticalBox { ... }

      // overlay floats top-left:
      StatusOverlay {
          x: 0;
          y: 0;
          width: parent.width * 0.5;   // half-width so it doesn't block controls
          items: Bridge.status-items;
      }
  }
  ```

- [ ] Verify overlay does not block the Stop Casting button.
- [ ] **Build check.**

---

### 5-E — Define initial status item set in Rust

- [ ] In `lib.rs`, create a helper function `build_status_items(state: &CastState) -> Vec<StatusItem>`:

  ```rust
  fn build_status_items(receiver_name: &str, encoder: &str) -> Vec<StatusItem> {
      vec![
          StatusItem {
              label: "Receiver".into(),
              value: receiver_name.into(),
              severity: "info".into(),
          },
          StatusItem {
              label: "Encoder".into(),
              value: encoder.into(),
              severity: "info".into(),
          },
      ]
  }
  ```

- [ ] Call this when cast state changes and push to `Bridge.status-items`.
- [ ] Use `slint::VecModel` to push updates: `bridge.set_status_items(model.into())`.
- [ ] Clear `status-items` (set to empty vec) when app leaves `Casting` state.
- [ ] **Build check.**

---

### 5-F — Add severity escalation for error states

- [ ] When `AppState` transitions to an error condition (connection lost, encoder failed),
  update the affected `StatusItem` with `severity: "error"`.
- [ ] When GStreamer codec blocker P0-1 is resolved, read the actual selected encoder name
  from the destination node and populate the "Encoder" status item.
- [ ] Add a "Network" status item sourced from the Rust device info (IP address / hostname).
- [ ] **Build check.**

---

### 5-G — Move debug log scroll area to `debug_page.slint`

The existing `ScrollView` + `status-text` in `debug_page.slint` is the text log for test
output. Confirm it is distinct from the casting overlay status pills.

- [ ] Verify `debug_page.slint` contains a `ScrollView` that shows `Bridge.test-status`.
- [ ] Confirm `StatusOverlay` in `casting_page.slint` does NOT read `test-status` (it reads
  `Bridge.status-items` only).
- [ ] They are parallel, not the same data source.

---

## Exit criteria

Phase 5 is complete when:

1. `status_overlay.slint` exports `StatusOverlay`.
2. `bridge.slint` defines `StatusItem` struct and `status-items` property.
3. `StatusOverlay` is visible in `CastingView` when `status-items` is non-empty.
4. Overlay is invisible (not just transparent) when `status-items` is empty.
5. Severity coloring works: info → `Theme.surface-overlay`, warning → `Theme.warning`,
   error → `Theme.error`.
6. Rust populates at least "Receiver" and "Encoder" items on cast start.
7. `cargo build -p android-sender` passes.

---

## Notes

- Do **not** port Moblin chat, navigation, replay, or right-side overlays — those have no
  FCast sender equivalent.
- Bitrate overlay item should be added once the Rust side exposes real-time bitrate from
  the GStreamer pipeline. Leave as a placeholder with empty `value` until then.
- The overlay width cap (`parent.width * 0.5`) is a starting point. Adjust during Phase 10
  device testing if items truncate awkwardly.

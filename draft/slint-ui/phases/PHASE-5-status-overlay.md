# Phase 5 — Status Overlay (UI-only placeholder)

> Add a transparent heads-up overlay to the casting screen showing connection
> and encoder status pills. **UI placeholder only — no Rust wiring.** Real data
> is fed by static stub model declared inline in the `.slint` file.

**Status:** `[ ] UI placeholder — no functionality`
**Depends on:** Phase 1 (modules), Phase 2 (theme), Phase 4 (control bar height defined)
**Functional integration:** Deferred — Rust never touches `status-items` in this phase.
**Related Moblin sources (reference only):**
- `Stream/StreamOverlayView.swift` → `StatusOverlay`
- `Stream/Overlay/LeftOverlayView.swift` → left-side status pills
- *Not ported:* `Overlay/Right*`, `CameraLevelView`, `DrawOnStreamView` (see `futures/NOT-APPLICABLE.md`)

**Related files:**
- `senders/android/ui/components/status_overlay.slint` — stub from Phase 1-H
- `senders/android/ui/pages/casting_page.slint` — overlay layered here
- `senders/android/ui/bridge.slint` — `StatusItem` struct + enum added here (UI-only — no Rust setter)

---

## Goal

Build the visual surface of the status overlay and prove the pill rendering, severity
coloring, and layout work end-to-end with **inline mock data**. Skip every Rust handler.

---

## Tasks

### 5-A — Add `StatusSeverity` enum and `StatusItem` struct to `bridge.slint`

These are *type definitions only* — they don't bind to any Rust setter. The struct is
consumed by a stub property declared further down (5-D).

- [ ] Open `senders/android/ui/bridge.slint`.
- [ ] Add before the `Bridge` global:

  ```slint
  export enum StatusSeverity { info, warning, error }

  export struct StatusItem {
      label:    string,
      value:    string,
      severity: StatusSeverity,
  }
  ```

- [ ] **Do not** add `in property <[StatusItem]> status-items` to `Bridge`. The mock
  data lives at the page level (5-D) so this phase stays UI-only.
- [ ] **Build check.**

---

### 5-B — Implement `StatusPill` sub-component

- [ ] Open `senders/android/ui/components/status_overlay.slint`.
- [ ] Add imports: `import { Theme } from "../theme.slint"; import { StatusItem, StatusSeverity } from "../bridge.slint";`.
- [ ] Implement internal (non-exported) `component StatusPill`:

  ```slint
  component StatusPill inherits Rectangle {
      in property <StatusItem> item;

      height: 28px;
      border-radius: Theme.radius-pill;
      background: root.item.severity == StatusSeverity.error   ? Theme.error
                : root.item.severity == StatusSeverity.warning ? Theme.warning
                :                                                Theme.surface-overlay;

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

  ```slint
  export component StatusOverlay inherits Rectangle {
      in property <[StatusItem]> items;

      background: transparent;
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

### 5-D — Layer `StatusOverlay` into `casting_page.slint` with inline stub data

- [ ] Open `senders/android/ui/pages/casting_page.slint`.
- [ ] Import `StatusOverlay` from `../components/status_overlay.slint`.
- [ ] Import `StatusItem, StatusSeverity` from `../bridge.slint`.
- [ ] Declare an inline mock model on the existing `CastingView` component:

  ```slint
  export component CastingView inherits Rectangle {
      // UI-only placeholder. Replace with `Bridge.status-items` when Rust wiring lands.
      in-out property <[StatusItem]> mock-status-items: [
          { label: "Receiver", value: "Living Room TV",       severity: StatusSeverity.info },
          { label: "Encoder",  value: "amcvidenc (H.264)",     severity: StatusSeverity.info },
          { label: "Network",  value: "192.168.1.42",          severity: StatusSeverity.info },
      ];

      // existing VerticalBox with "Casting" + Stop Casting button stays unchanged

      StatusOverlay {
          x: 0;
          y: 0;
          width: parent.width * 0.5;
          items: root.mock-status-items;
      }
  }
  ```

- [ ] Verify overlay does not block the Stop Casting button.
- [ ] **Build check.**

---

### 5-E — Add a "warning" + "error" stub variant for visual QA

- [ ] In the same `casting_page.slint`, add a second mock list demonstrating severities:

  ```slint
  // Toggle between healthy/error stubs by editing this property at design time.
  in-out property <[StatusItem]> mock-status-items-error: [
      { label: "Receiver", value: "Living Room TV",  severity: StatusSeverity.info },
      { label: "Network",  value: "Reconnecting…",   severity: StatusSeverity.warning },
      { label: "Encoder",  value: "Failed",          severity: StatusSeverity.error },
  ];
  ```

- [ ] (Optional) Hard-flip `items: root.mock-status-items-error;` once and screenshot to
  verify warning/error coloring.
- [ ] Revert to the healthy mock before committing.

---

## Exit criteria

1. `status_overlay.slint` exports `StatusOverlay`.
2. `bridge.slint` defines `StatusItem` + `StatusSeverity` (types only — no setter).
3. `CastingView` renders three info pills from the inline stub on every build.
4. Severity coloring works against the alt mock (info → `Theme.surface-overlay`,
   warning → `Theme.warning`, error → `Theme.error`).
5. No `Bridge.status-items` reference exists anywhere in this phase.
6. `cargo build -p android-sender` passes.

---

## What's NOT in this phase (deferred)

- Rust populating real receiver / encoder / network values.
- Severity escalation on connection-loss events.
- Real-time bitrate / framerate pill (depends on GStreamer pipeline introspection).
- `@tr(...)` wrapping (Phase 9).
- Wiring `mock-status-items` to a real `Bridge.status-items` setter.

These all live in `futures/` and will be promoted into a new phase when Rust ships
the underlying telemetry.

---

## Slint best practices applied here

- **Use enums for closed value sets.** `StatusSeverity` is type-safe and survives
  renames better than magic strings.
- **`visible: false` skips both rendering and event handling.** Preferred over
  `opacity: 0` for hiding overlays.
- **Inline `in-out` properties make great stub models.** Slint reactively updates
  the `for item in ...` loop when the list changes, so swapping mocks for live
  data later is a one-line change at the call site — no component rewrite needed.

# Phase 3 — Reusable Button and Row Components

> Build the shared component library that every page will use.
> Inspired by Moblin `View/Utils` (36 files); implemented as pure Slint.
> Reference: `draft/moblin-ui/Moblin/View/Utils/`

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (module layout exists), Phase 2 (Theme tokens available)
**Unlocks:** Phase 4 (control bar uses buttons), Phase 7 (settings page uses rows)
**Related files:**
- `senders/android/ui/components/buttons.slint` — stub from Phase 1-H
- `senders/android/ui/components/settings_rows.slint` — stub from Phase 1-H
- `senders/android/ui/theme.slint` — provides all color/size tokens

**Moblin source analogues (reference only — do not copy Swift):**

| Moblin file | Slint equivalent |
|---|---|
| `ButtonView.swift` | `PrimaryButton` |
| `BorderlessButtonView.swift` | `TextButton` |
| `CloseToolbarView.swift` | `PanelHeader` |
| `TextItemView.swift` | `SettingsTextRow` |
| `TextValueView.swift` | `SettingsValueRow` |
| `SliderView.swift` | `SettingsSliderRow` |
| `InlinePickerView.swift` | `SegmentedChoiceRow` (Phase 7) |

---

## Tasks

### 3-A — `buttons.slint` — `PrimaryButton`

- [ ] Open `senders/android/ui/components/buttons.slint`.
- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Implement `export component PrimaryButton`:

  ```
  export component PrimaryButton inherits Rectangle {
      in property <string>  label;
      in property <bool>    enabled: true;
      callback clicked();

      height: Theme.row-height;
      border-radius: Theme.radius-card;
      background: ta.pressed ? Theme.accent-pressed : Theme.accent;
      opacity: root.enabled ? 1.0 : 0.45;

      ta := TouchArea {
          enabled: root.enabled;
          clicked => { root.clicked(); }
      }
      Text {
          text: root.label;
          color: Theme.text-primary;
          horizontal-alignment: center;
          vertical-alignment: center;
          font-size: Theme.font-size-body;
          font-weight: FontWeight.semi-bold;
      }
  }
  ```

  > Note: `font-weight` uses Slint's built-in `FontWeight` namespace, not a `Theme` token.
  > See Phase 2-B — the property is typed `int` and `FontWeight.bold` / `FontWeight.semi-bold` etc.
  > are the canonical constants.

- [ ] **Build check.**

---

### 3-B — `buttons.slint` — `TextButton`

- [ ] Implement `export component TextButton` (borderless):

  ```
  export component TextButton inherits Rectangle {
      in property <string> label;
      in property <bool>   enabled: true;
      callback clicked();

      height: Theme.row-height;
      background: transparent;
      opacity: root.enabled ? 1.0 : 0.45;

      ta := TouchArea {
          enabled: root.enabled;
          clicked => { root.clicked(); }
      }
      Text {
          text: root.label;
          color: Theme.accent;
          horizontal-alignment: center;
          vertical-alignment: center;
          font-size: Theme.font-size-body;
      }
  }
  ```

- [ ] **Build check.**

---

### 3-C — `buttons.slint` — `DestructiveButton`

- [ ] Implement `export component DestructiveButton` (red stop/cancel):

  ```
  export component DestructiveButton inherits Rectangle {
      in property <string> label;
      in property <bool>   enabled: true;
      callback clicked();

      height: Theme.row-height;
      border-radius: Theme.radius-card;
      background: ta.pressed ? Theme.error.darker(20%) : Theme.error;
      opacity: root.enabled ? 1.0 : 0.45;

      ta := TouchArea {
          enabled: root.enabled;
          clicked => { root.clicked(); }
      }
      Text {
          text: root.label;
          color: Theme.text-primary;
          horizontal-alignment: center;
          vertical-alignment: center;
      }
  }
  ```

- [ ] **Build check.**

---

### 3-D — `buttons.slint` — `LoadingView`

> Shared spinner + label used by `ConnectingView` and `WaitingForMediaView`.

- [ ] Add `import { Spinner } from "std-widgets.slint";`.
- [ ] Implement `export component LoadingView`:

  ```
  export component LoadingView inherits VerticalLayout {
      in property <string> label: "Loading";
      alignment: center;
      spacing: Theme.spacing-default;

      Spinner {
          indeterminate: true;
          width: 48px;
          height: 48px;
      }
      Text {
          text: root.label;
          color: Theme.text-primary;
          font-size: Theme.font-size-body;
          horizontal-alignment: center;
      }
  }
  ```

- [ ] Update `connecting_page.slint`: replace inline `Spinner + Text` with `LoadingView { label: "Connecting"; }`.
- [ ] Update `casting_page.slint` (`WaitingForMediaView`): replace inline `Spinner + Text` with `LoadingView { label: "Waiting for media"; }`.
- [ ] **Build check.**

---

### 3-E — `settings_rows.slint` — `SettingsTextRow`

- [ ] Open `senders/android/ui/components/settings_rows.slint`.
- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Implement `export component SettingsTextRow` (label-only, non-interactive):

  ```
  export component SettingsTextRow inherits Rectangle {
      in property <string> title;
      in property <string> subtitle: "";

      height: root.subtitle == "" ? Theme.row-height : Theme.row-height + 18px;
      background: transparent;

      VerticalLayout {
          padding-left:  Theme.padding-screen;
          padding-right: Theme.padding-screen;
          alignment: center;
          Text {
              text: root.title;
              color: Theme.text-primary;
              font-size: Theme.font-size-body;
          }
          if root.subtitle != "": Text {
              text: root.subtitle;
              color: Theme.text-secondary;
              font-size: Theme.font-size-label;
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 3-F — `settings_rows.slint` — `SettingsValueRow`

- [ ] Implement `export component SettingsValueRow` (label + trailing value + chevron):

  ```
  export component SettingsValueRow inherits Rectangle {
      in property <string> title;
      in property <string> value: "";
      in property <bool>   enabled: true;
      in property <bool>   show-chevron: true;
      callback clicked();

      height: Theme.row-height;
      opacity: root.enabled ? 1.0 : 0.45;
      background: ta.pressed ? Theme.surface-card : transparent;

      ta := TouchArea {
          enabled: root.enabled;
          clicked => { root.clicked(); }
      }
      HorizontalLayout {
          padding-left:  Theme.padding-screen;
          padding-right: Theme.padding-screen;
          spacing: Theme.spacing-default;
          Text {
              text: root.title;
              color: Theme.text-primary;
              vertical-alignment: center;
              horizontal-stretch: 1;
          }
          Text {
              text: root.value;
              color: Theme.text-secondary;
              vertical-alignment: center;
          }
          if root.show-chevron: Text {
              text: "›";
              color: Theme.text-secondary;
              vertical-alignment: center;
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 3-G — `settings_rows.slint` — `SettingsToggleRow`

- [ ] Add `import { CheckBox } from "std-widgets.slint";`.
- [ ] Implement `export component SettingsToggleRow`:

  ```
  export component SettingsToggleRow inherits Rectangle {
      in property <string> title;
      in-out property <bool> checked: false;
      in property <bool> enabled: true;
      callback toggled(bool);

      height: Theme.row-height;
      opacity: root.enabled ? 1.0 : 0.45;

      HorizontalLayout {
          padding-left:  Theme.padding-screen;
          padding-right: Theme.padding-screen;
          Text {
              text: root.title;
              color: Theme.text-primary;
              vertical-alignment: center;
              horizontal-stretch: 1;
          }
          CheckBox {
              checked <=> root.checked;
              enabled: root.enabled;
              // CheckBox.toggled has signature `()` — read state from `self.checked`.
              // Reference: reference/std-widgets/basic-widgets/checkbox.mdx
              toggled() => { root.toggled(self.checked); }
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 3-H — `settings_rows.slint` — `SettingsSliderRow`

- [ ] Add `import { Slider } from "std-widgets.slint";`.
- [ ] Implement `export component SettingsSliderRow`:

  ```
  export component SettingsSliderRow inherits Rectangle {
      in property <string> title;
      in property <string> unit: "";
      in property <float>  minimum: 0;
      in property <float>  maximum: 100;
      in-out property <float> value: 50;
      callback changed(float);

      height: Theme.row-height * 1.5;

      VerticalLayout {
          padding-left:  Theme.padding-screen;
          padding-right: Theme.padding-screen;
          padding-top:   4px;
          HorizontalLayout {
              Text {
                  text: root.title;
                  color: Theme.text-primary;
                  vertical-alignment: center;
                  horizontal-stretch: 1;
              }
              Text {
                  text: Math.round(root.value) + root.unit;
                  color: Theme.text-secondary;
                  vertical-alignment: center;
              }
          }
          Slider {
              minimum: root.minimum;
              maximum: root.maximum;
              value <=> root.value;
              // Slider.changed has signature `changed(float)` — the new value is
              // passed as a parameter. Bind it explicitly to `value` rather than
              // re-reading from `self.value`.
              // Reference: reference/std-widgets/basic-widgets/slider.mdx
              changed(value) => { root.changed(value); }
          }
      }
  }
  ```

- [ ] **Build check.**

---

### 3-I — `settings_rows.slint` — `SettingsSection`

- [ ] Implement `export component SettingsSection` (titled card container):

  ```
  export component SettingsSection inherits VerticalLayout {
      in property <string> title: "";
      spacing: 1px;

      if root.title != "": Text {
          text: root.title;
          color: Theme.text-secondary;
          font-size: Theme.font-size-label;
          padding-left: Theme.padding-screen;
          padding-bottom: 4px;
      }

      @children
  }
  ```

- [ ] **Build check.**

---

### 3-J — Replace raw `Button` usages with `PrimaryButton`

- [ ] In `connect_page.slint`:
  - [ ] Replace `Button { text: "Scan QR"; clicked => Bridge.scan-qr(); }` with
    `PrimaryButton { label: "Scan QR"; clicked => Bridge.scan-qr(); }`.
- [ ] In `settings_page.slint` (`SettingsPageView`):
  - [ ] Replace `Button { text: "Start"; ... }` with `PrimaryButton`.
  - [ ] Replace `Button { text: "Disconnect"; ... }` with `DestructiveButton`.
- [ ] In `casting_page.slint`:
  - [ ] Replace `Button { text: "Cancel"; ... }` with `DestructiveButton`.
  - [ ] Replace `Button { text: "Stop"; ... }` with `DestructiveButton`.
- [ ] In `debug_page.slint`:
  - [ ] Replace all plain `Button` instances with `PrimaryButton` or `TextButton` as appropriate.
- [ ] **Final build check.**

---

## Exit criteria

Phase 3 is complete when:

1. `buttons.slint` exports: `PrimaryButton`, `TextButton`, `DestructiveButton`, `LoadingView`.
2. `settings_rows.slint` exports: `SettingsTextRow`, `SettingsValueRow`, `SettingsToggleRow`,
   `SettingsSliderRow`, `SettingsSection`.
3. No raw `Button` from `std-widgets.slint` remains in page files (all replaced with typed buttons).
4. `ConnectingView` and `WaitingForMediaView` use `LoadingView`.
5. `cargo build -p android-sender` passes cleanly.
6. All five app states still render and function correctly on device or `slint-viewer`.

---

## Slint best practices applied here

- **Std-widget callback signatures match the docs verbatim.** `CheckBox.toggled` is `()`,
  `Slider.changed` is `changed(float)`. When forwarding to a wrapper component callback,
  always declare the parameter list explicitly (`toggled() => { ... }`, `changed(value) => { ... }`)
  rather than relying on the no-param shorthand. References:
  [`reference/std-widgets/basic-widgets/checkbox.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx),
  [`reference/std-widgets/basic-widgets/slider.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx).
- **`@children` is the canonical way to make a container component.** `SettingsSection`
  uses `@children` to accept arbitrary row components, which is how std-widgets like
  `VerticalBox` work internally.
- **Color manipulation uses brush methods**, e.g. `Theme.error.darker(20%)`,
  `Theme.accent.brighter(0.2)`. Reference:
  [`reference/colors-and-brushes.mdx`](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/colors-and-brushes.mdx).
- **Button accessibility tip (future improvement):** the std-widget `Button` exposes
  accessibility properties (`accessible-role`, `accessible-label`) that our custom
  Rectangle-based buttons currently miss. If accessibility becomes a requirement, swap
  the `inherits Rectangle` chassis for `inherits TouchArea` (no Rectangle wrapping needed)
  or add explicit `accessible-*` properties.

# Phase 2 — Theme and Design Tokens

> Replace every hardcoded hex color, font size, and magic number with named constants
> exported from `ui/theme.slint`. No new components; existing components only get cleaner.

**Status:** `[ ] Not started`
**Depends on:** Phase 1 (module split complete — all page files exist)
**Unlocks:** Phase 3 (components can import theme tokens from day one)
**Related files:**
- `senders/android/ui/theme.slint` — stub created in Phase 1-A, filled here
- `senders/android/ui/pages/*.slint` — all page files updated to use tokens
- `draft/slint-ui/ui/migration-skeleton.slint` — reference: shows `#0b1020`, `#222633`, etc.

---

## Color inventory

All hardcoded colors found in `main.slint` before the split:

| Raw value | Location | Proposed token name |
|---|---|---|
| `#222633` | device list card background, debug section | `Theme.surface-card` |
| `#B8BECD` | empty-state hint text | `Theme.text-secondary` |
| `steelblue` | device row pressed / scan row background (active) | `Theme.accent` |
| `lightsteelblue` | device row idle / scan row background (idle) | `Theme.accent-muted` |
| `white` (implicit `Text` default) | all primary text labels | `Theme.text-primary` |
| `#0b1020` | proposed window background (from skeleton) | `Theme.surface-primary` |
| `#7f1d1d` | error severity in status overlay (Phase 5) | `Theme.error` |
| `#78350f` | warning severity in status overlay (Phase 5) | `Theme.warning` |
| `#1f2937cc` | info severity pill background (Phase 5) | `Theme.info` |
| `#2563eb` | quick action active badge (Phase 4) | `Theme.accent-active` |

---

## Tasks

### 2-A — Define color palette in `theme.slint`

- [ ] Open `senders/android/ui/theme.slint` (created as empty stub in Phase 1-A).
- [ ] Add `export global Theme` with `out property` color constants:

  ```
  export global Theme {
      // Surfaces
      out property <color> surface-primary:  #0b1020;
      out property <color> surface-card:     #222633;
      out property <color> surface-overlay:  #1f2937cc;

      // Text
      out property <color> text-primary:     #ffffff;
      out property <color> text-secondary:   #b8becd;
      out property <color> text-disabled:    #6b7280;

      // Accent / interactive
      out property <color> accent:           #4682b4;   // steelblue
      out property <color> accent-muted:     #b0c4de;   // lightsteelblue
      out property <color> accent-active:    #2563eb;
      out property <color> accent-pressed:   #1e40af;

      // Severity
      out property <color> error:            #7f1d1d;
      out property <color> warning:          #78350f;
      out property <color> success:          #14532d;
  }
  ```

- [ ] **Build check** after adding the global.

---

### 2-B — Define font size tokens

- [ ] Add font-size properties to `Theme`:

  ```
      // Typography
      out property <length> font-size-heading:  20px;
      out property <length> font-size-body:     16px;
      out property <length> font-size-label:    12px;
      out property <length> font-weight-bold:   700;
      out property <length> font-weight-normal: 400;
  ```

  > Note: `font-weight` is a `float` in Slint, not `length`. Adjust type as needed.

- [ ] **Build check.**

---

### 2-C — Define spacing and radius tokens

- [ ] Add spacing/radius properties to `Theme`:

  ```
      // Spacing
      out property <length> padding-screen:   12px;
      out property <length> padding-card:      8px;
      out property <length> spacing-default:   8px;

      // Shape
      out property <length> radius-card:       8px;
      out property <length> radius-pill:        6px;
      out property <length> row-height:        44px;
      out property <length> control-bar-height: 72px;
  ```

- [ ] **Build check.**

---

### 2-D — Apply tokens in `connect_page.slint`

- [ ] Add `import { Theme } from "../theme.slint";` at top.
- [ ] Replace `background: #222633` → `background: Theme.surface-card`.
- [ ] Replace `color: #B8BECD` on hint text → `color: Theme.text-secondary`.
- [ ] Replace `background: device_ta.pressed ? steelblue : lightsteelblue`
  → `background: device_ta.pressed ? Theme.accent-pressed : Theme.accent-muted`.
- [ ] Replace `background: scan_ta.pressed ? steelblue : lightsteelblue` same pattern.
- [ ] Replace `border-radius: 8px` → `border-radius: Theme.radius-card`.
- [ ] Replace `height: 45px` on rows → `height: Theme.row-height`.
- [ ] **Build check.**

---

### 2-E — Apply tokens in `connecting_page.slint`

- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Replace any hardcoded font sizes with `Theme.font-size-body`.
- [ ] **Build check.**

---

### 2-F — Apply tokens in `settings_page.slint` (SettingsPageView)

- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Replace any hardcoded font sizes and colors.
- [ ] **Build check.**

---

### 2-G — Apply tokens in `casting_page.slint`

- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Replace any hardcoded values.
- [ ] **Build check.**

---

### 2-H — Apply tokens in `debug_page.slint`

- [ ] Add `import { Theme } from "../theme.slint";`.
- [ ] Replace `height: 100px` on `ScrollView` with a semantic token or leave as explicit constant
  with a comment — this is an intrinsic height, not a token.
- [ ] **Build check.**

---

### 2-I — Set `MainWindow` background

- [ ] In `main.slint`, set `background: Theme.surface-primary` on `MainWindow`.
- [ ] Import `Theme` in `main.slint`.
- [ ] **Build check.**

---

### 2-J — Final audit

- [ ] Run `grep -r "#[0-9a-fA-F]" senders/android/ui/` and confirm zero hits (all colors
  replaced with tokens).
- [ ] Run `grep -r "steelblue\|lightsteelblue" senders/android/ui/` and confirm zero hits.
- [ ] Confirm `theme.slint` is the only file containing raw color literals.

---

## Exit criteria

Phase 2 is complete when:

1. `theme.slint` exports all tokens listed in section 2-A through 2-C.
2. No raw hex or named color literals remain in page files (only in `theme.slint`).
3. `cargo build -p android-sender` passes cleanly.
4. Visual appearance is unchanged vs. Phase 1 (same colors, same sizes).

---

## Notes

- `steelblue` and `lightsteelblue` are CSS named colors. Their hex equivalents are
  `#4682b4` and `#b0c4de` respectively. Use those in `Theme` for consistency.
- The accent colors will likely be updated in Phase 3–4 to match an FCast brand color.
  The token names are stable; only the values change.
- `Theme.row-height: 44px` matches iOS HIG minimum touch target. Android's is 48dp —
  consider bumping to `48px` during Phase 10 testing if touch targets feel small on device.

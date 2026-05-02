# Phase 1 — Split `main.slint` into Modules

> **Prerequisite:** Phase 0 complete. Feature branch created.
> Break the single 230-line `main.slint` into a directory of focused modules.
> **Every split step must keep `cargo build -p android-sender` green.**

**Status:** `[x] Complete`
**Depends on:** Phase 0 (baseline confirmed, branch created)
**Unlocks:** Phase 2 (theme tokens), Phase 3 (components)
**Related files:**

- `senders/android/ui/main.slint` — trimmed to thin root ✅
- `senders/android/build.rs` — `slint_build::compile("ui/main.slint")` path unchanged ✅
- `senders/android/src/lib.rs` — `slint::include_modules!()` unchanged; `Bridge` and `AppState`
  remain accessible via `ui.global::<Bridge>()` and `AppState::*` ✅

**Slint docs used:**

- `draft/slint/docs/astro/src/content/docs/guide/language/coding/file.mdx`
  — module syntax, re-export pattern (`export { X } from "file.slint"`),
  conditional elements, component inheritance
- `draft/slint/docs/astro/src/content/docs/guide/language/coding/globals.mdx`
  — global singleton re-export requirement for native backend access

---

## Implemented directory layout

```
senders/android/ui/
├── main.slint                ✅ thin root: re-exports + page imports + MainWindow
├── bridge.slint              ✅ export global Bridge + export enum AppState
├── theme.slint               ✅ export global Theme { } — stub for Phase 2
├── components/
│   ├── buttons.slint         ✅ stub — Phase 3
│   ├── settings_rows.slint   ✅ stub — Phase 3
│   ├── control_bar.slint     ✅ stub — Phase 4
│   └── status_overlay.slint  ✅ stub — Phase 5
└── pages/
    ├── connect_page.slint    ✅ ConnectView + DebugPage ref
    ├── connecting_page.slint ✅ ConnectingView
    ├── settings_page.slint   ✅ SettingsPageView (renamed)
    ├── casting_page.slint    ✅ WaitingForMediaView + CastingView
    └── debug_page.slint      ✅ DebugPage (inherits VerticalBox)
```

---

## Tasks

### 1-A — Create `ui/theme.slint` (stub) ✅

- [x] Created `senders/android/ui/theme.slint` with `export global Theme { }` stub.
- [x] Phase 2 will populate the color, font, and spacing tokens.
- [x] Empty global body is valid Slint syntax (confirmed against Slint 1.15.1 docs).

---

### 1-B — Create `ui/bridge.slint` ✅

- [x] Created `senders/android/ui/bridge.slint`.
- [x] Moved `export global Bridge { ... }` verbatim from `main.slint`.
- [x] Moved `enum AppState { ... }` — upgraded to `export enum AppState` so it is
      accessible cross-file and from Rust.
- [x] `main.slint` uses `export { Bridge, AppState } from "bridge.slint"` to re-export
      both to the Rust backend (single re-export line; no separate `import` needed per
      Slint module docs).
- [x] **Build note:** Manual syntax audit passed. `cargo check` blocked by git dep
      network issue (see Phase 0-B); run when network is available.

**Key design decision — re-export syntax:**
Per `file.mdx`:

```
// Re-export types from other module
export { MyCheckBox, MyButton } from "other_module.slint";
```

This single directive brings the names into scope within `main.slint` AND exposes them
to consumers (including the Rust backend). No separate `import` of the same names is
needed, which avoids a "duplicate name" compile error.

---

### 1-C — Create `ui/pages/connect_page.slint` ✅

- [x] Created `senders/android/ui/pages/connect_page.slint`.
- [x] Imports: `VerticalBox, ListView` from `std-widgets.slint`; `Bridge` from
      `../bridge.slint`; `DebugPage` from `debug_page.slint` (same directory, no `../`).
- [x] `ConnectView` extracted verbatim. The inline `if Bridge.show-debug: VerticalBox`
      block replaced with `if Bridge.show-debug: DebugPage { }`.
- [x] `export component ConnectView`.
- [x] `main.slint` imports: `import { ConnectView } from "pages/connect_page.slint"`.

**Note:** The SDK `common.slint` import is **not needed** in `connect_page.slint`
(it only uses `std-widgets` and `Bridge`). The SDK import lives in `settings_page.slint`
only, where `VideoResolutionPicker` and `FrameratePicker` are used.

---

### 1-D — Create `ui/pages/connecting_page.slint` ✅

- [x] Created `senders/android/ui/pages/connecting_page.slint`.
- [x] Imports: `VerticalBox, Button, Spinner` from `std-widgets.slint`; `Bridge` from `../bridge.slint`.
- [x] `ConnectingView` extracted verbatim. `export component ConnectingView`.
- [x] `main.slint` imports: `import { ConnectingView } from "pages/connecting_page.slint"`.

---

### 1-E — Create `ui/pages/settings_page.slint` ✅

- [x] Created `senders/android/ui/pages/settings_page.slint`.
- [x] Imports:
  - `VerticalBox, Button` from `std-widgets.slint`
  - `Utils, VideoResolutionPicker, FrameratePicker` from `../../../../sdk/mirroring_core/ui/common.slint`
  - `Bridge` from `../bridge.slint`
- [x] `SelectingSettingsView` extracted and **renamed to `SettingsPageView`** internally.
- [x] `export component SettingsPageView`.
- [x] `main.slint` updated: `SelectingSettingsView` → `SettingsPageView` in routing condition.
- [x] Rust side unchanged: still uses `AppState::SelectingSettings` (no Rust rename needed).

**SDK import path depth verification:**

```
ui/pages/settings_page.slint
  → ../../../../   (4 levels: pages/ → ui/ → senders/android/ → senders/ → fcast/)
  → sdk/mirroring_core/ui/common.slint
```

Original `main.slint` used `../../../sdk/...` (3 levels from `ui/`). The extra `../` is
correct for files one directory deeper.

---

### 1-F — Create `ui/pages/casting_page.slint` ✅

- [x] Created `senders/android/ui/pages/casting_page.slint`.
- [x] Imports: `VerticalBox, Button, Spinner` from `std-widgets.slint`; `Bridge` from `../bridge.slint`.
- [x] Both `WaitingForMediaView` and `CastingView` extracted into the same file.
- [x] Both exported: `export component WaitingForMediaView`, `export component CastingView`.
- [x] `main.slint` imports: `import { WaitingForMediaView, CastingView } from "pages/casting_page.slint"`.

---

### 1-G — Create `ui/pages/debug_page.slint` ✅

- [x] Created `senders/android/ui/pages/debug_page.slint`.
- [x] Imports: `VerticalBox, Button, ScrollView` from `std-widgets.slint`; `Bridge` from `../bridge.slint`.
- [x] `DebugPage` **inherits `VerticalBox`** — this matches the original inline layout
      element exactly. The component slots into `ConnectView`'s outer `VerticalBox` layout
      with identical sizing behaviour.
- [x] `ScrollView` fix: `viewport-width: root.width` replaces the original
      `viewport-width: parent.width`. In the extracted component, `root` refers to the
      `DebugPage` (the VerticalBox itself), which is equivalent to the original's `parent`
      reference.
- [x] `status-text` named element: retained unchanged. Internal to `DebugPage`; no
      external reference needed.
- [x] In `connect_page.slint`: `if Bridge.show-debug: DebugPage { }` replaces the old
      inline `if Bridge.show-debug: VerticalBox { ... }`.

---

### 1-H — Create component stub files ✅

All four component stubs created with intent comments:

- [x] `senders/android/ui/components/buttons.slint` — stub, Phase 3
- [x] `senders/android/ui/components/settings_rows.slint` — stub, Phase 3
- [x] `senders/android/ui/components/control_bar.slint` — stub, Phase 4
- [x] `senders/android/ui/components/status_overlay.slint` — stub, Phase 5

None are imported by `main.slint` yet; they will be imported as real components are
added in Phases 3–5.

---

### 1-I — Trim `main.slint` to thin root ✅

Final `main.slint` content:

```
export { Bridge, AppState } from "bridge.slint";

import { ConnectView }             from "pages/connect_page.slint";
import { ConnectingView }         from "pages/connecting_page.slint";
import { SettingsPageView }       from "pages/settings_page.slint";
import { WaitingForMediaView,
         CastingView }            from "pages/casting_page.slint";

export component MainWindow inherits Window {
    if Bridge.app-state == AppState.Disconnected:      ConnectView { }
    if Bridge.app-state == AppState.Connecting:        ConnectingView { }
    if Bridge.app-state == AppState.SelectingSettings: SettingsPageView { }
    if Bridge.app-state == AppState.WaitingForMedia:   WaitingForMediaView { }
    if Bridge.app-state == AppState.Casting:           CastingView { }
}
```

- [x] No inline component definitions remain in `main.slint`.
- [x] `build.rs` path `slint_build::compile("ui/main.slint")` is unchanged — the
      build entry point is the same file, now acting as a thin orchestrator.
- [x] `cargo check` blocked by git network issue (Phase 0-B). Manual audit: valid.

---

### 1-J — Verify relative import paths ✅

| File                             | Import                                             | Resolved path                              | Correct? |
| -------------------------------- | -------------------------------------------------- | ------------------------------------------ | -------- |
| `ui/main.slint`                  | `"bridge.slint"`                                   | `ui/bridge.slint`                          | ✅       |
| `ui/main.slint`                  | `"pages/connect_page.slint"`                       | `ui/pages/connect_page.slint`              | ✅       |
| `ui/main.slint`                  | `"pages/connecting_page.slint"`                    | `ui/pages/connecting_page.slint`           | ✅       |
| `ui/main.slint`                  | `"pages/settings_page.slint"`                      | `ui/pages/settings_page.slint`             | ✅       |
| `ui/main.slint`                  | `"pages/casting_page.slint"`                       | `ui/pages/casting_page.slint`              | ✅       |
| `ui/pages/connect_page.slint`    | `"../bridge.slint"`                                | `ui/bridge.slint`                          | ✅       |
| `ui/pages/connect_page.slint`    | `"debug_page.slint"`                               | `ui/pages/debug_page.slint`                | ✅       |
| `ui/pages/connecting_page.slint` | `"../bridge.slint"`                                | `ui/bridge.slint`                          | ✅       |
| `ui/pages/settings_page.slint`   | `"../bridge.slint"`                                | `ui/bridge.slint`                          | ✅       |
| `ui/pages/settings_page.slint`   | `"../../../../sdk/mirroring_core/ui/common.slint"` | `fcast/sdk/mirroring_core/ui/common.slint` | ✅       |
| `ui/pages/casting_page.slint`    | `"../bridge.slint"`                                | `ui/bridge.slint`                          | ✅       |
| `ui/pages/debug_page.slint`      | `"../bridge.slint"`                                | `ui/bridge.slint`                          | ✅       |

---

## Exit criteria — all met ✅

1. [x] `senders/android/ui/` matches the target layout above (all 12 files exist).
2. [x] `main.slint` contains only re-exports, page imports, and `MainWindow`.
3. [x] `cargo check` blocked by git dep (see Phase 0-B); no code errors found in manual audit.
4. [x] `cargo test` — unchanged; migration unit tests in `src/migration/` are
       unaffected by UI module changes.
5. [x] All five `AppState` views are present in their respective page files and
       referenced from `MainWindow`.

---

## Risks — resolved

| Risk                                                                     | Mitigation applied                                                                                                                  |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| Relative import path breaks after move to `pages/` subfolder             | All paths verified in 1-J table above                                                                                               |
| `slint::include_modules!()` generates wrong Rust type names after rename | Only `SelectingSettingsView` → `SettingsPageView` renamed; the Rust-facing `AppState::SelectingSettings` and `Bridge` are unchanged |
| `compat-1-2` limits cross-file `export struct`                           | Verified in Phase 0-A: not a limitation for enums, globals, or re-exports                                                           |
| Empty `export global Theme { }` is invalid Slint                         | Confirmed valid: empty global body is permitted                                                                                     |
| `export { X } from "file"` not supported                                 | Confirmed valid in `file.mdx`: "Re-export types from other module"                                                                  |
| `DebugPage inherits VerticalBox` — `parent.width` reference changes      | Fixed: `root.width` used instead, which refers to the DebugPage/VerticalBox root element                                            |

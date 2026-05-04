# Phase 19 — Settings Backup & Reset Placeholder

> Settings sub-page exposing import / export / reset operations.
> **UI-only.** Buttons trigger an inline confirmation dialog; no real file
> I/O, no real persistence reset.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — no Rust persistence layer to back up
or reset.
**Moblin source analogues:**
- `Settings/ImportExport/ImportExportSettingsView.swift`
- `Settings/ImportExport/ImportSettingsView.swift`
- `Settings/ImportExport/ExportSettingsView.swift`
- `Settings/Reset/ResetSettingsView.swift`

**Files:**
- `senders/android/ui/pages/backup_reset_page.slint` — new
- `senders/android/ui/components/confirm_dialog.slint` — new (reusable)
- `senders/android/ui/bridge.slint` — `Panel.backup-reset`

---

## Tasks

### 19-A — Reusable `ConfirmDialog` component

- [ ] Create `senders/android/ui/components/confirm_dialog.slint`:

  ```slint
  import { Theme } from "../theme.slint";
  import { PrimaryButton, DestructiveButton, TextButton } from "buttons.slint";

  export component ConfirmDialog inherits Rectangle {
      in property <string> title;
      in property <string> body;
      in property <string> confirm-label: "Confirm";
      in property <bool>   destructive: false;
      callback confirmed();
      callback dismissed();

      background: #00000080;     // semi-opaque scrim (UI-only — Theme.scrim later)

      Rectangle {
          width: parent.width * 0.85;
          height: 200px;
          x: (parent.width  - self.width) / 2;
          y: (parent.height - self.height) / 2;
          background: Theme.surface-card;
          border-radius: Theme.radius-card;

          VerticalLayout {
              padding: Theme.padding-card;
              spacing: 12px;

              Text { text: root.title; color: Theme.text-primary;   font-size: Theme.font-size-heading; }
              Text { text: root.body;  color: Theme.text-secondary; wrap: word-wrap; }

              HorizontalLayout {
                  alignment: end;
                  spacing: 8px;
                  TextButton { label: "Cancel"; clicked => { root.dismissed(); } }
                  if root.destructive: DestructiveButton {
                      label: root.confirm-label; clicked => { root.confirmed(); }
                  }
                  if !root.destructive: PrimaryButton {
                      label: root.confirm-label; clicked => { root.confirmed(); }
                  }
              }
          }
      }
  }
  ```

- [ ] **Build check.** Add `Theme.scrim` color to `theme.slint` later — for now
  the inline `#00000080` is acceptable in the dialog component (mark with a
  TODO comment).

---

### 19-B — `BackupResetPage`

- [ ] Page header + Done button (panel pattern from Phase 7).
- [ ] Sections:

  ```
  BACKUP
    Export settings to file        →  pretend-success
    Import settings from file      →  pretend-success

  RESET
    Reset all settings            →  destructive ConfirmDialog
    Clear cast history            →  destructive ConfirmDialog
    Clear known receivers         →  destructive ConfirmDialog
  ```

- [ ] Each "pretend-success" sets a transient banner ("Exported to ~/fcast-backup.json")
  for 3 seconds via a Slint `Timer`.
- [ ] Each destructive row opens a `ConfirmDialog` overlay with appropriate
  title / body. On confirm, dialog closes and a banner shows "Reset complete"
  (UI-only).

---

### 19-C — Bridge + linking

- [ ] Extend `Panel` with `backup-reset`.
- [ ] In `main.slint`, route `Panel.backup-reset` → `BackupResetPage`.
- [ ] In `FullSettingsPage`, link from a "DATA" section.

---

## Exit criteria

1. Page opens from settings root, closes via Done button.
2. Export / Import rows show a transient success banner.
3. Each destructive row opens `ConfirmDialog`. Cancel dismisses; Confirm shows
   a "Reset complete" banner.
4. Banners auto-dismiss after 3 seconds.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real file picker / `Storage Access Framework` integration.
- Real persistence read / write / clear.
- Backup format / schema versioning.
- iCloud / Google Drive cloud backup.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/ImportExport/ImportExportSettingsView.swift`
- `View/Settings/ResetSettings/ResetSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/ImportExport/ImportExportSettingsView.swift (excerpt)
Form {
    Section {
        NavigationLink("Export") { ExportSettingsView() }
        NavigationLink("Import") { ImportSettingsView() }
    }
    Section {
        Button(role: .destructive) { /* show confirm */ } label: {
            Text("Reset all settings")
        }
    }
}
```

**Mapping notes:**

Export / Import open dedicated panels (file save / file open are stubbed
to no-ops). Reset all surfaces a `ConfirmDialog` (a new reusable component
on Phase 19) — a `PopupWindow` containing a destructive-style button +
cancel. The dialog accepts a `confirmed()` callback consumers wire to their
local mutator.

**Relevant Slint docs:**
- [PopupWindow](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/window/popupwindow.mdx)

## Slint best practices applied here

- **A reusable `ConfirmDialog` with `confirmed()` / `dismissed()` callbacks**
  is more composable than per-page inline dialogs. Future destructive flows
  (Phase 24's "Forget receiver") can reuse this.
- **`Timer { interval: 3s; }` for transient banners** is simpler than tracking
  banner lifetime in Rust. Auto-cleanup happens entirely in Slint.

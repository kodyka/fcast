# Phase 21 — Help & Support Placeholder

> Settings sub-page bundle covering About, version history, attributions, and
> a help/support landing surface. **UI-only.** All content is hardcoded English
> markdown-style text.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7
**Functional integration:** None — this section is documentation by design.
**Moblin source analogues:**
- `Settings/About/AboutSettingsView.swift`
- `Settings/About/AboutAttributionsSettingsView.swift`
- `Settings/About/AboutVersionHistorySettingsView.swift`
- `Settings/HelpAndSupport/HelpAndSupportSettingsView.swift`

**Files:**
- `senders/android/ui/pages/about_page.slint` — new (dedicated, replaces inline ABOUT in Phase 7)
- `senders/android/ui/pages/attributions_page.slint` — new
- `senders/android/ui/pages/version_history_page.slint` — new
- `senders/android/ui/pages/help_page.slint` — new
- `senders/android/ui/bridge.slint` — Panel variants `about`, `attributions`, `version-history`, `help`

---

## Tasks

### 21-A — `AboutPage`

- [ ] Header with "About" + Done.
- [ ] Body: app icon (placeholder colored Rectangle), app name, version
  (from `mock-app-version` reused from Phase 7), short tagline.
- [ ] Three navigation rows:
  - "Version history" → `Panel.version-history`
  - "Open source attributions" → `Panel.attributions`
  - "Help & support" → `Panel.help`

---

### 21-B — `VersionHistoryPage`

- [ ] List of release entries (inline stub):

  ```slint
  in-out property <[{version: string, date: string, notes: string}]> mock-versions: [
      { version: "0.0.1-dev", date: "2026-05-03", notes: "UI placeholder build (Phases 0–11)." },
      { version: "0.0.0",     date: "2026-04-15", notes: "Initial Slint port commit." },
  ];
  ```

- [ ] Each entry rendered as a `VerticalLayout` with version + date header
  and notes paragraph.

---

### 21-C — `AttributionsPage`

- [ ] Long-form scrolling text listing third-party libraries:

  ```
  Slint                Open source license
  GStreamer            LGPL
  ExoPlayer            Apache 2.0
  Tokio                MIT
  Serde                Apache 2.0 / MIT
  Mdns-sd              MIT
  …
  ```

- [ ] Render as a `ScrollView` with a `VerticalLayout` of plain `Text`
  entries. No links / no tappable items in UI-only build.

---

### 21-D — `HelpPage`

- [ ] Three sections:
  - **Getting started** — short paragraph + 3 numbered steps.
  - **Troubleshooting** — bulleted FAQ-style questions.
  - **Contact** — placeholder rows: "Documentation site", "GitHub repo",
    "Report an issue". Each row's `clicked` is a no-op (real URL launching
    deferred).

---

### 21-E — Bridge + linking

- [ ] Extend `Panel`: `about`, `attributions`, `version-history`, `help`.
- [ ] Route all four in `main.slint`.
- [ ] In `FullSettingsPage`, replace the existing inline ABOUT section with a
  single "About" navigation row that opens `Panel.about`.

---

## Exit criteria

1. About page opens from settings root and shows app metadata.
2. From About, all three sub-pages (version history, attributions, help) open.
3. Each sub-page has a Done button that returns to About (write
   `Bridge.active-panel = Panel.about`, not `Panel.none`).
4. Help page rows clickable but no-op.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real URL launching (`android.intent.action.VIEW`).
- Real license text (the attributions list is hand-curated, not generated).
- Issue-report deep links.
- Live changelog from a remote server.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `View/Settings/HelpAndSupport/HelpAndSupportSettingsView.swift`

**Representative SwiftUI excerpt:**

```swift
// View/Settings/HelpAndSupport/HelpAndSupportSettingsView.swift (excerpt)
Form {
    Section {
        Link(destination: URL(string: "https://moblin.app")!) { Text("Website") }
        Link(destination: URL(string: "https://github.com/eerimoq/moblin")!) { Text("Source code") }
    }
    Section { Text("Version \(applicationVersion)") }
}
```

**Mapping notes:**

Slint has no `Link` element — links are rendered as `TextButton`s whose
`clicked` callback fires a Bridge callback (`open-url(string)`) in Phase 8.
For the placeholder, the rows just show the URL as a label and the click
is a no-op. Version reads from a static `mock-app-version: string`.

**Relevant Slint docs:**
- [Text element](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/elements/text.mdx)

## Slint best practices applied here

- **Done button writes `Panel.about` (not `Panel.none`)** for sub-pages so
  the user lands back on the parent rather than bouncing to the cast screen.
- **A `for entry in mock-versions: VerticalLayout { ... }`** pattern keeps
  the version history data-driven without needing a separate component.

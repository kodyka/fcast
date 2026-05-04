# Phase 20 — Cast History Placeholder

> Settings sub-page listing recent cast sessions (receiver, start time,
> duration, status). **UI-only.** History entries come from inline mock model.

**Status:** `[ ] Not started`
**Depends on:** Phases 2, 3, 7
**Functional integration:** Deferred — no Rust event log to read from.
**Moblin source analogues:**
- `Settings/StreamingHistory/StreamingHistorySettingsView.swift`
- `Settings/StreamingHistory/StreamingHistoryStreamSettingsView.swift`

**Files:**
- `senders/android/ui/pages/cast_history_page.slint` — new
- `senders/android/ui/pages/cast_history_detail_page.slint` — new (per-session detail)
- `senders/android/ui/bridge.slint` — `CastHistoryEntry` struct + `Panel.cast-history*`

---

## Tasks

### 20-A — `CastHistoryEntry` struct

- [ ] In `bridge.slint`:

  ```slint
  export struct CastHistoryEntry {
      id:         string,
      receiver:   string,
      started-at: string,    // formatted display string
      duration-s: int,
      status:     string,    // "Completed" / "Cancelled" / "Failed"
  }
  ```

---

### 20-B — `CastHistoryPage` list

- [ ] Inline mock model (5 entries, varied status):

  ```slint
  in-out property <[CastHistoryEntry]> mock-history: [
      { id: "h1", receiver: "Living Room TV",     started-at: "Today 19:42", duration-s: 5400, status: "Completed" },
      { id: "h2", receiver: "Office Display",     started-at: "Today 11:15", duration-s: 600,  status: "Completed" },
      { id: "h3", receiver: "Kitchen Chromecast", started-at: "Yesterday 22:08", duration-s: 30, status: "Cancelled" },
      { id: "h4", receiver: "Living Room TV",     started-at: "Yesterday 20:33", duration-s: 7200, status: "Completed" },
      { id: "h5", receiver: "Office Display",     started-at: "Mon 09:50",   duration-s: 0,    status: "Failed" },
  ];
  ```

- [ ] Each row shows: receiver name + status pill (color-coded by status),
  started-at + duration. Tap opens
  `Bridge.active-panel = Panel.cast-history-detail` and writes
  `Bridge.selected-history-id = entry.id`.

- [ ] Empty state: "No casts yet."

- [ ] Trailing toolbar button: "Clear all" → triggers `ConfirmDialog` from
  Phase 19's reusable component.

---

### 20-C — `CastHistoryDetailPage`

- [ ] Header with the receiver name + status pill.
- [ ] Body: list of fields (started-at, duration formatted as `HH:MM:SS`,
  bitrate average, peak bitrate, dropped frames) — all from inline stub data.
- [ ] Footer: "Cast again to <receiver>" `PrimaryButton` (no-op in UI-only build).

---

### 20-D — Bridge + linking

- [ ] Extend `Panel`: `cast-history`, `cast-history-detail`.
- [ ] Add `in-out property <string> selected-history-id: "";` to `Bridge`.
- [ ] Route both panels in `main.slint`.
- [ ] Link from `FullSettingsPage` "DATA" section.

---

## Exit criteria

1. List page renders 5 stub entries with status pills.
2. Tapping a row opens detail page with matching id.
3. Empty state appears when `mock-history` is emptied.
4. "Clear all" opens `ConfirmDialog`; confirm clears the inline list.
5. `cargo build -p android-sender` passes.

---

## What's NOT in this phase

- Real cast event log from Rust.
- Persistence (clearing the list resets on reload).
- Statistics aggregation ("most-cast receiver this week").
- Export history as CSV / JSON.

---

## Moblin source mapping & Slint primitives

**Source files referenced:**
- `(no direct Moblin analogue — FCast-specific feature)`

**Representative SwiftUI excerpt:**

_(no direct Moblin source — FCast cast history doesn't exist in Moblin.
The closest visual analogue is Moblin's "Recordings" browser:
`View/Settings/Recordings/RecordingsSettingsView.swift`)_
**Mapping notes:**

Modeled visually on Moblin's RecordingsSettingsView — a `List` of past
sessions with timestamp + duration + thumbnail. In FCast the "session" is
a past cast (not a recording); the row metadata is `device-name +
started-at + duration`. Tap a row to re-cast (placeholder: routes back to
ConnectView with the device pre-selected).

**Relevant Slint docs:**
- [ListView virtualization](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx)

## Slint best practices applied here

- **A `selected-id: string` property + a route** is simpler than push/pop
  navigation stacks. The detail page reads from `Bridge.selected-history-id`
  (or in the UI-only build, from the page's own `in-out property` that the
  list page sets before flipping `active-panel`).
- **Reusable `ConfirmDialog` from Phase 19** keeps the destructive UX consistent.

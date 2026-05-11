# Bridge invariants (post-Phase 8)

This file is the canonical reference for `Bridge.active-panel`,
`Bridge.lifecycle`, and `Bridge.app-state` writers. Any PR that adds a
new writer to one of these properties must update this file too.

## Bridge.active-panel
- Slint writers: panel-opening clicks (settings row, control-bar button, page-internal navigation). Direct writes (`Bridge.active-panel = Panel.x;`).
- Rust writer: `open_panel(p)` in `lib.rs`.
- Reader chain: `main.slint` (`if Bridge.active-panel == Panel.x: …`).

## Bridge.lifecycle
- Slint writers: LockOverlay / StealthOverlay / SnapshotCountdown exit paths.
- Rust writers: `set_lifecycle` called by `on_engage_lock`, `on_engage_stealth`, `on_start_snapshot_countdown`, and `on_exit_lifecycle` handlers in `lib.rs`.

## Bridge.app-state
- Slint writers: ONLY via `Bridge.change-state(to)`.
- Rust writers: `invoke_change_state` in `lib.rs`.

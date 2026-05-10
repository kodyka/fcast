# Phase 8 — Section 2: Cluster A — read-only view models

> Section 2 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) and [`PHASE-8-Section-1-cluster-F-shared-tokens.md`](./PHASE-8-Section-1-cluster-F-shared-tokens.md) first.

**Cluster A is "Rust pushes, Slint observes."** None of the items in this cluster require Slint→Rust callbacks. Each is a one-way data flow: a Rust producer (battery manager / version constant / interface enumerator / recording state machine / log subscriber) writes a `Bridge.<name>` property and Slint pages re-render.

| Item | Slint property added | Rust producer | Effort |
|---|---|---|---|
| A1 | `Bridge.status-items: [StatusItem]` | `BatteryManager` + `PowerManager` + `ConnectivityManager` | Medium |
| A2 | `Bridge.app-version: string` | `env!("CARGO_PKG_VERSION")` constant | Tiny |
| A3 | `Bridge.network-interfaces: [NetworkInterface]` | `getifaddrs()` via `NetworkInterface.getNetworkInterfaces()` JNI bridge | Medium (JNI deferred to Phase 11) |
| A4 | `Bridge.recording-state: RecordingState` + `Bridge.recording-elapsed-s: int` | tokio interval pushing once per second | Medium |
| A5 | `Bridge.log-entries: [LogEntry]` | `tracing-subscriber` custom layer + ring buffer | Medium |

**Net new code:** ~70 lines bridge.slint, ~50 lines per consumer page (mostly deletions), ~250 lines lib.rs.

**Risk:** medium. `upgrade_in_event_loop` discipline matters; see Section 8 / Risk R2 if you've never used it.

---

## 2.1 — A1 — Status overlay items

### What's there today

`senders/android/ui/components/status_badges.slint` (Phase 13) currently has 4 hardcoded `mock-*` props, each rendered as one Badge:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/components/status_badges.slint" lines="45-79" />

This is fine for offline rendering but ignores the three real Android signals.

### What we want

A single `Bridge.status-items: [StatusItem]` model. Rust pushes 3 entries (network / thermal / battery); Slint renders whatever's in the list. Slint reads severity from the struct; no string-equality colour lookup.

### Step 1: extend `bridge.slint`

```diff
 export enum StatusSeverity { info, warning, error }

 export struct StatusItem {
     label:    string,
     value:    string,
     severity: StatusSeverity,
+    // Optional UI hint — keeps the existing emoji-glyph fallback that
+    // status_badges.slint uses today. Real icons land in Phase 27.
+    icon-glyph: string,
 }

 export global Bridge {
     // …
+    // ── Status overlay (Phase 8 / Cluster A1) ───────────────────────────
+    in property <[StatusItem]> status-items: [];
     // …
 }
```

**Why a struct list instead of three separate props:**

- The list grows over time (signal strength, location, audio routing). A `[StatusItem]` model means every future signal is a one-line `set_status_items` push, not a Slint markup change.
- Severity is per-item, not per-signal. Battery <20% maps to error; thermal "Critical" maps to error; both want the same colour. A struct field collapses this to a single mapping table on the Slint side.
- See `guide/language/coding/structs-and-enums.mdx` for the inheritance pattern (struct fields are immutable per row, not per element — this matches Slint's reactive system).

### Step 2: rewrite `components/status_badges.slint`

```diff
 import { Theme } from "../theme.slint";
 import { IconAndText } from "icon_and_text.slint";
+import { Bridge, StatusItem, StatusSeverity } from "../bridge.slint";

 component Badge inherits Rectangle {
     in property <string> icon-glyph;
     in property <string> value;
     in property <color>  fg: Theme.text-secondary;
     // … (unchanged) …
 }

 export component StatusBadgesRow inherits Rectangle {
-    in-out property <int>    mock-battery-pct: 87;
-    in-out property <bool>   mock-charging:    false;
-    in-out property <string> mock-thermal:     "Nominal";
-    in-out property <string> mock-network:     "Wi-Fi";
-
     height: 28px;
     background: transparent;

     HorizontalLayout {
         alignment: end;
         spacing: 6px;
         padding-right: Theme.padding-screen;

-        Badge {
-            icon-glyph: "📶";
-            value: root.mock-network;
-        }
-        Badge {
-            icon-glyph: root.mock-thermal == "Critical" ? "🔥" : "🌡";
-            value: root.mock-thermal;
-            fg: root.mock-thermal == "Critical" ? Theme.error-fg
-              : root.mock-thermal == "Serious"  ? Theme.warning-fg
-              :                                    Theme.text-secondary;
-        }
-        Badge {
-            icon-glyph: root.mock-charging ? "⚡" : "🔋";
-            value: @tr("{}%", root.mock-battery-pct);
-            fg: root.mock-battery-pct < 20 ? Theme.error-fg : Theme.text-secondary;
-        }
+        for item in Bridge.status-items: Badge {
+            icon-glyph: item.icon-glyph;
+            value: item.value;
+            fg: item.severity == StatusSeverity.error   ? Theme.error-fg
+              : item.severity == StatusSeverity.warning ? Theme.warning-fg
+              :                                            Theme.text-secondary;
+        }
     }
 }
```

**Why each piece:**

- `for item in Bridge.status-items` — Slint's list rendering primitive. The loop body is the only Badge declaration we need; the model owns count + ordering. See `guide/language/coding/repetition-and-data-models.mdx`.
- The colour lookup is still in Slint, not Rust — design tokens live in `theme.slint`, not `lib.rs`. `severity` is the cross-language vocabulary; colours are per-platform.
- The `[StatusItem]` initial value is an **empty array**, *not* a placeholder set of 3 entries. Rust pushes the real list within milliseconds of startup (`set_status_items` is one of the first things called). A placeholder would flicker — see Section 8 / R4.

### Step 3: Rust producer

```rust
// senders/android/src/lib.rs — append after the existing helpers (~line 1090).

use slint::{Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex;

// Cached snapshot. Re-pushed in full whenever any signal changes —
// VecModel doesn't support diff-based updates, and the list is 3 elements
// long, so a full rebuild is fine.
#[derive(Clone, Default)]
struct StatusSnapshot {
    network_label: String,            // "Wi-Fi" / "4G" / "Off"
    thermal_label: String,            // "Nominal" / "Serious" / "Critical"
    battery_pct:   i32,               // 0..100
    charging:      bool,
}

fn push_status(ui_handle: slint::Weak<MainWindow>, snap: StatusSnapshot) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        let bridge = ui.global::<Bridge>();
        let items = vec![
            StatusItem {
                label:      "network".into(),
                value:      snap.network_label.into(),
                severity:   StatusSeverity::Info,
                icon_glyph: "📶".into(),
            },
            StatusItem {
                label:      "thermal".into(),
                value:      snap.thermal_label.clone().into(),
                severity:   match snap.thermal_label.as_str() {
                    "Critical" => StatusSeverity::Error,
                    "Serious"  => StatusSeverity::Warning,
                    _          => StatusSeverity::Info,
                },
                icon_glyph: if snap.thermal_label == "Critical" { "🔥".into() } else { "🌡".into() },
            },
            StatusItem {
                label:      "battery".into(),
                value:      format!("{}%", snap.battery_pct).into(),
                severity:   if snap.battery_pct < 20 { StatusSeverity::Error } else { StatusSeverity::Info },
                icon_glyph: if snap.charging { "⚡".into() } else { "🔋".into() },
            },
        ];
        let model: ModelRc<StatusItem> = Rc::new(VecModel::from(items)).into();
        bridge.set_status_items(model);
    });
}
```

And in `init_ui` (or wherever you build `MainWindow`), kick off the producer with whatever signal source you have. For Phase 8 the canonical approach is a **periodic poll** — every 5 seconds, read `BatteryManager`, `PowerManager.thermalStatus`, and `ConnectivityManager.activeNetworkInfo`. Real event-driven listeners are deferred to Phase 11 (peripherals & lifecycle).

```rust
let ui_handle = ui.as_weak();
let snap = Arc::new(Mutex::new(StatusSnapshot::default()));

tokio::spawn(async move {
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(5));
    loop {
        tick.tick().await;
        // TODO Phase 11 — replace with real JNI calls. For now, push a
        // believable placeholder so the row isn't empty.
        let snap_now = StatusSnapshot {
            network_label: "Wi-Fi".into(),
            thermal_label: "Nominal".into(),
            battery_pct:   87,
            charging:      false,
        };
        push_status(ui_handle.clone(), snap_now);
    }
});
```

**Why a 5-second poll:**

- Battery / thermal / network all change on the seconds-to-minutes timescale. A 5s tick keeps the badge fresh without holding event-loop pressure.
- A polled snapshot is simpler than a pub/sub channel for the volume (3 items × 1 update/5s = 1.6 KB/min). Phase 11 can replace this with real broadcast receivers without changing `bridge.slint`.

**Slint doc citations for A1:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx` — `for x in model` syntax + `VecModel`.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — `StatusItem` struct + `StatusSeverity` enum.
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx` — Rust tab shows the canonical `Rc<VecModel<…>>::into()` push idiom.

---

## 2.2 — A2 — App version

### What's there today

The settings_page (Phase 21 about/version row) likely has either a hard-coded `"0.0.1-dev"` literal or a page-local `mock-app-version` property. UI-REVIEW-2026-05-10.md / B10 catalogues this. The fix is one line on each side.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Build / version (Phase 8 / Cluster A2) ──────────────────────────
+    in property <string> app-version: "";
     // …
 }
```

### Step 2: consumer migration

Find the row that renders the version. Most likely in `pages/about_page.slint`:

```diff
 import { Theme } from "../theme.slint";
 import { Bridge, Panel } from "../bridge.slint";
 // …

 SettingsValueRow {
     title: @tr("App version");
-    value: "0.0.1-dev";
+    value: Bridge.app-version;
     show-chevron: false;
 }
```

### Step 3: Rust producer

`env!("CARGO_PKG_VERSION")` is a compile-time string from Cargo.toml. The push is one line, performed once during `init_ui`:

```rust
// senders/android/src/lib.rs — in init_ui (~line 1100), after the bridge.slint
// generated globals are accessible.
ui.global::<Bridge>()
    .set_app_version(env!("CARGO_PKG_VERSION").into());
```

**Why not a constant in Slint:**

- Slint files are compiled at `slint_build::compile()` time, but the Cargo version isn't known to slintc — slintc reads `.slint`, not `Cargo.toml`. So the literal would have to be regenerated by build.rs. Pushing once from Rust at startup is far simpler and gives you free debug-build suffix support (`env!("CARGO_PKG_VERSION_PRE")`).

**Slint doc citations for A2:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx` — read-only `in property <string>`.

---

## 2.3 — A3 — Network interfaces

### What's there today

`pages/network_page.slint` initialises 3 hardcoded interfaces inline:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/network_page.slint" lines="139-143" />

### What we want

`Bridge.network-interfaces: [NetworkInterface]` driven by Rust enumeration. The Slint page becomes a dumb list renderer.

### Step 1: extend `bridge.slint`

```diff
 export struct NetworkInterface {
     name:        string,
     kind:        string,
     address-v4:  string,
     address-v6:  string,
     enabled:     bool,
 }

 export global Bridge {
     // …
+    // ── Network interfaces (Phase 8 / Cluster A3) ───────────────────────
+    in property <[NetworkInterface]> network-interfaces: [];
+    // Per-interface enable toggle goes through this callback. The Rust
+    // side is responsible for re-emitting the updated list after the
+    // toggle takes effect (or rejecting it).
+    callback set-interface-enabled(string, bool);
     // …
 }
```

The callback is **Slint→Rust** with no return value. Slint doesn't optimistically flip the row state — it asks Rust, and Rust either succeeds (pushes the list back with the new enabled flag) or fails (re-emits the list unchanged, optionally raises a `flash_banner`).

### Step 2: consumer migration

```diff
 export component NetworkPage inherits Rectangle {
-    in-out property <[NetworkInterface]> mock-interfaces: [
-        { name: "wlan0",  kind: "wifi",     address-v4: "192.168.1.42",  address-v6: "fe80::1234", enabled: true  },
-        { name: "rmnet0", kind: "cellular", address-v4: "10.20.30.40",   address-v6: "",           enabled: false },
-        { name: "lo",     kind: "loopback", address-v4: "127.0.0.1",     address-v6: "::1",        enabled: true  },
-    ];
     in-out property <bool> mock-wifi-aware-enabled: false;   // moves to Cluster B5
     property <bool>        banner-visible:          false;
     // …

-    function set-enabled(name: string, value: bool) {
-        root.mock-interfaces = [
-            // … 21 lines of in-place rebuild …
-        ];
-    }
-
     // …
-    for iface in root.mock-interfaces: NetworkInterfaceRow {
+    for iface in Bridge.network-interfaces: NetworkInterfaceRow {
         data: iface;
-        toggle-enabled(value) => { root.set-enabled(iface.name, value); }
+        toggle-enabled(value) => { Bridge.set-interface-enabled(iface.name, value); }
     }
 }
```

**Why this is much smaller:**

- The 12-line `set-enabled` rebuild function disappears entirely. Mutation goes Rust-side via `set-interface-enabled`. Rust does the rebuild and re-emits — Slint just re-renders.
- See Section 8 / R3 for why we don't use `<=>` two-way binding here. Two-way bindings need a settable model; `Bridge.network-interfaces: [NetworkInterface]` is `in property`, not `in-out`, so any imperative write would silently no-op.

### Step 3: Rust producer + handler

```rust
// senders/android/src/lib.rs

// Real implementation deferred to Phase 11 — for Phase 8 we just push
// the same 3 entries so the page still renders.
fn enumerate_interfaces() -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            name: "wlan0".into(),
            kind: "wifi".into(),
            address_v4: "192.168.1.42".into(),
            address_v6: "fe80::1234".into(),
            enabled: true,
        },
        NetworkInterface {
            name: "rmnet0".into(),
            kind: "cellular".into(),
            address_v4: "10.20.30.40".into(),
            address_v6: "".into(),
            enabled: false,
        },
        NetworkInterface {
            name: "lo".into(),
            kind: "loopback".into(),
            address_v4: "127.0.0.1".into(),
            address_v6: "::1".into(),
            enabled: true,
        },
    ]
}

fn push_interfaces(ui_handle: slint::Weak<MainWindow>, list: Vec<NetworkInterface>) {
    let _ = ui_handle.upgrade_in_event_loop(move |ui| {
        let model: ModelRc<NetworkInterface> = Rc::new(VecModel::from(list)).into();
        ui.global::<Bridge>().set_network_interfaces(model);
    });
}

// In init_ui:
push_interfaces(ui.as_weak(), enumerate_interfaces());

let interfaces = Arc::new(Mutex::new(enumerate_interfaces()));
let interfaces_for_callback = interfaces.clone();
let ui_for_callback = ui.as_weak();
ui.global::<Bridge>()
    .on_set_interface_enabled(move |name, value| {
        let interfaces = interfaces_for_callback.clone();
        let ui_handle = ui_for_callback.clone();
        tokio::spawn(async move {
            let mut list = interfaces.lock().await;
            if let Some(iface) = list.iter_mut().find(|i| i.name == name.as_str()) {
                iface.enabled = value;
            }
            push_interfaces(ui_handle, list.clone());
            // Phase 11 will also call into NetworkInterface.setUp/setDown.
        });
    });
```

**Why the `Arc<Mutex<Vec<…>>>`:**

- The callback closure is `'static` (Slint requires it) and may run on the UI thread or a tokio task — needs interior mutability either way.
- `tokio::sync::Mutex` is used because the lock is held across an `.await` (the `push_interfaces` call wraps `upgrade_in_event_loop` which is async-friendly via the `_for_callback` weak handle).
- Cloning the `Vec<NetworkInterface>` per push is fine — the model is 3-10 entries deep. If you outgrow that (>1000 interfaces, unlikely on Android), switch to a long-lived `VecModel` and call `set_row_data` per change.

**Slint doc citations for A3:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

---

## 2.4 — A4 — Recording elapsed counter

### What's there today

`pages/recording_page.slint` (Phase 23) drives the elapsed counter from a Slint `Timer`:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/recording_page.slint" lines="44-49" />

This is fine for UI-only stub state, but in Phase 8 the timer source-of-truth is Rust (the actual `MediaRecorder` knows when it started, how long it has been buffering, etc.). Slint's job is to render `Bridge.recording-elapsed-s` as a HH:MM:SS string.

### Step 1: extend `bridge.slint`

The `RecordingState` enum already exists. Add the two properties:

```diff
 export global Bridge {
     // …
+    // ── Recording (Phase 8 / Cluster A4) ────────────────────────────────
+    in property <RecordingState> recording-state:    RecordingState.idle;
+    in property <int>            recording-elapsed-s: 0;
     // …
 }
```

(The Slint→Rust callbacks `start-recording` / `pause-recording` / `resume-recording` / `stop-recording` are part of **Cluster B3** in the next section, since they involve writes from Slint, not just reads. A4 is read-only.)

### Step 2: consumer migration

```diff
 export component RecordingPage inherits Rectangle {
-    in-out property <RecordingState> mock-state:        RecordingState.idle;
-    in-out property <int>             mock-elapsed-s:    0;
+    // mock-state and mock-elapsed-s now read from Bridge.
+    // Page-local props remain for things Slint owns: format, folder, audio toggle.
     in-out property <int>             mock-format-idx:   0;
     in-out property <int>             mock-folder-idx:   0;
     in-out property <bool>            mock-record-audio: true;
     in-out property <int>             mock-disk-free-mb: 12480;

-    property <string> elapsed-display: format-elapsed(root.mock-elapsed-s);
+    property <string> elapsed-display: format-elapsed(Bridge.recording-elapsed-s);
     property <string> disk-free-display: …;

-    Timer {
-        interval: 1s;
-        running: root.mock-state == RecordingState.recording;
-        triggered => { root.mock-elapsed-s += 1; }
-    }

     // … rest unchanged, except mock-state →  Bridge.recording-state ……

     Rectangle {
         background:
-            root.mock-state == RecordingState.idle      ? #cc0000
-            : root.mock-state == RecordingState.recording ? #cc0000
-            : root.mock-state == RecordingState.paused    ? Theme.accent-active
-            : Theme.surface-primary;
+            Bridge.recording-state == RecordingState.idle      ? #cc0000
+            : Bridge.recording-state == RecordingState.recording ? #cc0000
+            : Bridge.recording-state == RecordingState.paused    ? Theme.accent-active
+            : Theme.surface-primary;
         // … similar swaps for inner glyph + Stop button enabled …
     }
 }
```

(The `on-record-clicked` and `on-stop-clicked` *handlers* themselves move to Cluster B3 — see Section 3.)

### Step 3: Rust producer

```rust
// senders/android/src/lib.rs

#[derive(Clone, Copy, Debug, PartialEq)]
struct RecordingTickerState {
    started_at: Option<std::time::Instant>,
    paused_for: std::time::Duration,        // accumulated paused time
    pause_started: Option<std::time::Instant>,
    state: RecordingState,
}

impl Default for RecordingTickerState {
    fn default() -> Self {
        Self {
            started_at: None,
            paused_for: std::time::Duration::ZERO,
            pause_started: None,
            state: RecordingState::Idle,
        }
    }
}

fn elapsed_seconds(s: &RecordingTickerState) -> i32 {
    let Some(started) = s.started_at else { return 0; };
    let mut elapsed = started.elapsed();
    elapsed = elapsed.saturating_sub(s.paused_for);
    if let Some(pause_start) = s.pause_started {
        elapsed = elapsed.saturating_sub(pause_start.elapsed());
    }
    elapsed.as_secs() as i32
}

fn spawn_recording_ticker(
    ui_handle: slint::Weak<MainWindow>,
    state: Arc<Mutex<RecordingTickerState>>,
) {
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(std::time::Duration::from_millis(500));
        loop {
            tick.tick().await;
            let snap = *state.lock().await;
            let secs = elapsed_seconds(&snap);
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                let bridge = ui.global::<Bridge>();
                bridge.set_recording_state(snap.state);
                bridge.set_recording_elapsed_s(secs);
            });
            if snap.state == RecordingState::Idle && snap.started_at.is_none() {
                // Stay subscribed — start can come back any time.
            }
        }
    });
}
```

**Why 500 ms tick instead of 1 s:**

- A 1 s tick can drift up to 999 ms on slow event loops. The HH:MM:SS string only changes every full second, but pushing twice per second guarantees no visible "stutter" between the wall clock and the on-screen display.
- The cost is one extra `upgrade_in_event_loop` per second per page; negligible.

**Why we keep `format-elapsed` in Slint:**

- The HH:MM:SS formatter (`pad2 → "HH:MM:SS"`) is purely presentational. Slint owns presentation; Rust owns timekeeping. This keeps the producer unaware of locale or future "1d 03:14:22" formats.

**Slint doc citations for A4:**

- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx` — Slint timer is removed in favour of Rust ticker.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx` — `Bridge.recording-state == RecordingState.idle ? … : …` ternary chain.

---

## 2.5 — A5 — Debug log entries

### What's there today

`pages/debug_log_page.slint` (Phase 26) initialises 5 hardcoded entries:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/debug_log_page.slint" lines="46-58" />

`mock-min-level-idx` is the filter dropdown (kept as Slint-side state — Cluster C5 wires Clear-all but the filter is a UI selection, not a Rust signal).

### What we want

A bounded ring buffer in Rust populated by a `tracing-subscriber` layer. Slint reads `Bridge.log-entries` as a virtualised list.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Debug log (Phase 8 / Cluster A5) ────────────────────────────────
+    in property <[LogEntry]> log-entries: [];
     // …
 }
```

(The `clear-log-entries()` callback is part of Cluster C5 in Section 4.)

### Step 2: consumer migration

```diff
 export component DebugLogPage inherits Rectangle {
-    in-out property <[LogEntry]> mock-log: [
-        { level: LogLevel.info,    timestamp: "12:34:56.012",
-          target: "fcast::discovery", message: "mDNS scan started" },
-        // … 4 more …
-    ];
     in-out property <int> mock-min-level-idx: 1;

     // … filter chips unchanged …

-    ListView {
-        for entry in root.mock-log: Rectangle {
+    ListView {
+        for entry in Bridge.log-entries: Rectangle {
             // … existing filter + render block unchanged …
         }
     }

     Rectangle {   // bottom toolbar
         // …
         TextButton {
             label: @tr("Clear");
-            clicked => { root.mock-log = []; }
+            clicked => { Bridge.clear-log-entries(); }
         }
         // … (Copy all stays Slint-side for now — Phase 11) …
     }
 }
```

The `level-as-int`, `level-color`, `level-name` pure functions stay in Slint — they're presentation. The model is the only thing that moves.

### Step 3: Rust producer (tracing subscriber + ring buffer)

```rust
// senders/android/src/lib.rs (or a new senders/android/src/log_ring.rs)

use std::sync::Mutex;
use tracing::{Subscriber, span, Event};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

const LOG_RING_CAP: usize = 1024;

#[derive(Clone)]
pub struct LogRing {
    entries: Arc<Mutex<std::collections::VecDeque<LogEntry>>>,
    ui_handle: slint::Weak<MainWindow>,
}

impl LogRing {
    pub fn new(ui_handle: slint::Weak<MainWindow>) -> Self {
        Self {
            entries: Arc::new(Mutex::new(
                std::collections::VecDeque::with_capacity(LOG_RING_CAP),
            )),
            ui_handle,
        }
    }

    pub fn clear(&self) {
        self.entries.lock().unwrap().clear();
        self.push_to_ui();
    }

    fn push_to_ui(&self) {
        let snap: Vec<LogEntry> = self.entries.lock().unwrap().iter().cloned().collect();
        let _ = self.ui_handle.upgrade_in_event_loop(move |ui| {
            let model: ModelRc<LogEntry> = Rc::new(VecModel::from(snap)).into();
            ui.global::<Bridge>().set_log_entries(model);
        });
    }
}

impl<S: Subscriber> Layer<S> for LogRing {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = LogEventVisitor::default();
        event.record(&mut visitor);
        let metadata = event.metadata();
        let entry = LogEntry {
            level:     match *metadata.level() {
                tracing::Level::TRACE => LogLevel::Trace,
                tracing::Level::DEBUG => LogLevel::Debug,
                tracing::Level::INFO  => LogLevel::Info,
                tracing::Level::WARN  => LogLevel::Warning,
                tracing::Level::ERROR => LogLevel::Error,
            },
            timestamp: chrono::Local::now().format("%H:%M:%S%.3f").to_string().into(),
            target:    metadata.target().into(),
            message:   visitor.message.into(),
        };
        let mut q = self.entries.lock().unwrap();
        if q.len() == LOG_RING_CAP {
            q.pop_front();
        }
        q.push_back(entry);
        drop(q);
        self.push_to_ui();
    }
}

#[derive(Default)]
struct LogEventVisitor { message: String }
impl tracing::field::Visit for LogEventVisitor {
    fn record_str(&mut self, f: &tracing::field::Field, v: &str) {
        if f.name() == "message" { self.message = v.to_owned(); }
    }
    fn record_debug(&mut self, f: &tracing::field::Field, v: &dyn std::fmt::Debug) {
        if f.name() == "message" { self.message = format!("{:?}", v); }
    }
}
```

Wire the subscriber once in `init_ui`:

```rust
let log_ring = LogRing::new(ui.as_weak());
let log_ring_for_clear = log_ring.clone();

tracing_subscriber::registry()
    .with(log_ring.clone())
    .init();

ui.global::<Bridge>().on_clear_log_entries(move || {
    log_ring_for_clear.clear();
});
```

**Why the ring buffer cap:**

- 1024 entries × ~150 B/entry = ~150 KB, comfortably below typical Android Slint memory budget.
- A bounded ring removes any need to truncate Slint-side. The page becomes purely declarative.

**Reentrancy warning — `std::sync::Mutex` inside `on_event` is not reentrant.** The
sketch above locks `self.entries` at the top of `on_event`, drops the guard, and
then calls `push_to_ui` which re-locks. That's safe as written, but it is *one
edit away* from a self-deadlock: if any code that runs **inside** the lock ever
emits a `tracing` event (allocator hooks, instrumented `VecDeque` impls in some
toolchains, panic-on-overflow paths, a future `tracing::trace!` added inside
`LogEventVisitor`), the subscriber re-enters `on_event` on the same thread and
deadlocks on the same `Mutex`. `std::sync::Mutex` is not reentrant.

Pick one of the following before shipping:

1. **Filter the subscriber so it ignores its own target.** Cheapest:
   ```rust
   if metadata.target().starts_with("fcast::log_ring") { return; }
   ```
   plus an explicit `tracing_subscriber::filter::Targets` rule in `init_ui`.
2. **Use `try_lock` and silently drop on contention.** Acceptable for a debug log
   ring — losing a re-entrant event is preferable to a deadlock:
   ```rust
   let Ok(mut q) = self.entries.try_lock() else { return; };
   ```
3. **Use a reentrant mutex** (`parking_lot::ReentrantMutex`). Last resort — adds a
   dependency and masks the underlying problem rather than fixing it.

The same caveat applies to `clear()` at the top of `LogRing` — it locks, calls
`push_to_ui` (which re-locks), and would deadlock under reentrant tracing. The
guard drops at end-of-statement today, but a refactor that holds the guard
longer would break this. See [`PHASE-8-Section-8-pitfalls.md`](./PHASE-8-Section-8-pitfalls.md) §8.13.

**Why `push_to_ui` rebuilds the whole `VecModel`:**

- See Section 8 / R3 — `VecModel<T>` doesn't have a public diff API. For a 1024-entry list, full rebuild is O(N) but Slint's `ListView` virtualises rendering at 60 fps, so even a 5000-entry rebuild is <2 ms in practice. The cost is dominated by the `ModelRc<...>` allocation, not the iteration.

**Slint doc citations for A5:**

- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx` — `ListView` virtualisation guarantee.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx` — `font-family: "monospace"` semantics.

---

## 2.6 Cluster A verification

```sh
# 1. mock-* count drops by 7 (4 status + 2 recording + 5 log = 11; minus ones still
#    held over by Slint-only filter state, etc. — see preflight 0.2 chart for the
#    precise expected delta).
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l

# 2. Bridge declarations exist.
grep -nE '(status-items|app-version|network-interfaces|recording-state|recording-elapsed-s|log-entries)' \
    senders/android/ui/bridge.slint

# 3. Producer functions exist.
grep -nE '(push_status|set_app_version|push_interfaces|spawn_recording_ticker|LogRing::new)' \
    senders/android/src/lib.rs

# 4. Build green
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

If `slint-viewer` is on PATH:

```sh
slint-viewer senders/android/ui/main.slint
# Status badges should render empty (Bridge pushes from real Rust only at
# runtime; slint-viewer doesn't run lib.rs). Verify NO binding-loop or
# missing-layout-size warnings appear in the slint-viewer console.
```

---

## 2.7 Commit messages

One commit per item is overkill but valuable for review. Suggested split:

```
feat(android): Phase 8 / A1 — status overlay items via Bridge.status-items

feat(android): Phase 8 / A2 — Bridge.app-version pushed at startup

feat(android): Phase 8 / A3 — Bridge.network-interfaces + set-interface-enabled

feat(android): Phase 8 / A4 — Bridge.recording-state + recording-elapsed-s

feat(android): Phase 8 / A5 — Bridge.log-entries via tracing-subscriber ring
```

If you prefer one commit per cluster, group the five into "Phase 8 / Cluster A — read-only view models." Either way, the **mock-count delta** mentioned in the commit body should match Section 0.2's expectation.

---

## 2.8 Exit criteria for Section 2

- [ ] All 5 items listed in this cluster have their Bridge declaration **and** their consumer migration **and** their Rust producer
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green
- [ ] No `mock-*` properties relating to status / version / interfaces / recording-state / log remain
- [ ] No `Slint Timer` driving recording elapsed counter (replaced by Rust ticker)
- [ ] Tracing subscriber `LogRing` initialised exactly once
- [ ] Smoke-test confirmed: `slint-viewer` shows no binding loops; on-device the status badges populate within ~5s of launch

You can now move to **Section 3 — Cluster B: single-page state with one or two callbacks** at [`PHASE-8-Section-3-cluster-B-single-page-state.md`](./PHASE-8-Section-3-cluster-B-single-page-state.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/elements/text.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/views/listview.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx`

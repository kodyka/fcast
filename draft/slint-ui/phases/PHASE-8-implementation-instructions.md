# Phase 8 — Rust Bridge Reactivation: Step-by-step Implementation Guide

**Audience:** developer ready to *execute* Phase 8 — wire the deferred `mock-*` properties on every shipped UI page to real Rust producers/consumers in `senders/android/src/lib.rs`.
**Goal:** by the end of this guide, every `pages/*.slint` file is free of `mock-*` initialisers, every promoted property is a `Bridge.*` property, every promoted mutation goes through a Slint→Rust callback, and `cargo build && cargo clippy --all-targets -- -D warnings` is clean.
**Out of scope:** any new UI work; `@tr()` audits (Phase 9); UI-only validation tooling (Phase 10); chat / streaming / scenes phases (28+).
**Constraint of this *document*:** guide-only, with full Slint and Rust snippets. The actual migration happens in a separate PR.

> **Companion documents.**
>
> - [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md) — original Phase 8 spec (the "what").
> - [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) — strategy / risk register / per-cluster index (the "why").
> - **This file** — step-by-step execution walkthrough with concrete snippets (the "how").
>
> Read the migration plan first if you haven't. This guide assumes its terminology (Cluster A / B / C / D / E / F) and its per-phase index table.

---

## Section 0 — Pre-flight checklist

Before you write a single line, confirm the following state on `master`. Each command should match the expected output exactly (line counts may shift if newer phases land — what matters is that the **shape** matches).

### 0.1 Confirm shipped UI phases

Phase 8 is only sensible after the UI phases land. Run:

```sh
ls senders/android/ui/pages/
```

Expected (alphabetical, from `master` as of 2026-05-09):

```
audio_page.slint                bitrate_presets_page.slint    casting_page.slint        connect_page.slint        debug_page.slint         macros_page.slint           pairing_page.slint         recording_page.slint
backup_reset_page.slint         camera_page.slint              codec_test_page.slint     connecting_page.slint     debug_video_page.slint   network_page.slint          quick_actions_page.slint    settings_page.slint
bitrate_preset_edit_page.slint  cast_history_detail_page.slint
                                cast_history_page.slint        debug_log_page.slint      macro_edit_page.slint     receiver_rename_page.slint
```

If a file is missing, the corresponding cluster step in this guide has nothing to migrate — *skip* that step rather than try to wire a property to a non-existent producer.

### 0.2 Inventory of `mock-*` properties (the work surface)

```sh
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/
```

Expected: 31 lines as of 2026-05-09 (the number drops as you migrate). Save the output to a scratch file — you will re-run this after each cluster to confirm the count drops by exactly the items the cluster claims.

### 0.3 Inventory of already-wired bindings (do not touch)

```sh
grep -nE 'global::<Bridge>|on_(connect|start|stop|invoke|change)|set_(devices|app_state|show_debug|test_status|quick_actions|test_status)' senders/android/src/lib.rs
```

Expected matches (around line 992 of `lib.rs`):

| Already-wired binding | Don't touch |
|---|---|
| `Bridge.devices: [string]` ← `set_devices` (mDNS) | yes |
| `Bridge.app-state: AppState` ← `invoke_change_state(...)` | yes |
| `Bridge.show-debug: bool` ← `set_show_debug` (debug-build gate) | yes |
| `Bridge.test-status: string` ← `set_test_status` (codec test runner) | yes |
| `Bridge.quick-actions: [QuickAction]` ← `set_quick_actions` (startup) | **read-only, but consumer is wrong — see step 6** |
| `Bridge.connect-receiver(string)` → `on_connect_receiver` | yes |
| `Bridge.start-casting(...)` → `on_start_casting` | yes |
| `Bridge.stop-casting()` → `on_stop_casting` | yes |
| `Bridge.invoke-action(string)` → `on_invoke_action` | yes |
| `Bridge.change-state(AppState)` (Slint-side public function) | yes |

### 0.4 Build green on `master` first

```sh
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

If `master` does not build, your migrations cannot be A/B-tested against a green baseline. **Fix the build first** — do not start this phase from a broken tree.

### 0.5 Branch

```sh
git checkout master && git pull
git checkout -b devin/$(date +%s)-phase-8-bridge-reactivation
```

(One PR per cluster is recommended — see [`PHASE-8-bridge-migration-plan.md`](./PHASE-8-bridge-migration-plan.md) Section 1. If you prefer one big PR, branch once and stack commits per cluster.)

---

## Section 1 — Step 1 / Cluster F: Add shared Theme + Bridge tokens

This cluster is the smallest and unblocks all the others. Do it first.

### 1.1 Theme severity tokens

Currently `theme.slint` has `success`, `warning`, `error` already wired in master:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/theme.slint" lines="34-39" />

So **F3 is already done** as of master. You can confirm:

```sh
grep -nE 'success|warning|error' senders/android/ui/theme.slint
```

If you see `success: #2e7d32`, `warning: #ed6c02`, `error: #c62828` — skip 1.1 entirely.

### 1.2 `InfoBanner` Bridge globals (F1)

Add to `bridge.slint` so any page can show a banner via Rust:

```diff
 export enum StatusSeverity { info, warning, error }
+
+export enum BannerSeverity { info, success, warning, error }

 export global Bridge {
     /* … existing properties … */
+
+    // Phase 27 InfoBanner — promoted to Bridge so Rust can surface
+    // banners from any side effect.
+    in property <string>          banner-message;
+    in-out property <bool>        banner-visible: false;
+    in property <BannerSeverity>  banner-severity: BannerSeverity.info;
 }
```

**Slint-doc:** `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` (enum declaration), `…coding/properties.mdx` (`in` vs `in-out`).

The page-level consumers (Phase 19 backup-reset, Phase 22 wifi-aware) keep working — their `banner-visible: bool` page property stays as a fallback. Migrate consumers one-at-a-time in step 5.

No Rust changes yet — the producer comes online when destructive flows are wired (step 5).

### 1.3 Verification

```sh
slint-viewer senders/android/ui/main.slint
# Window opens, no compile errors, no visible banner anywhere.
```

```sh
cargo build -p android-sender
# Builds; no Rust changes yet.
```

Commit: `chore(slint): add Bridge.banner-* + BannerSeverity for Cluster F`.

---

## Section 2 — Step 2 / Cluster A: Read-only view models

These are the lowest-risk migrations. Each one is a pure producer (Rust → UI) — no callback round-trip, no destructive consequences. Five items.

### 2.1 A1 — Status overlay items

**Phase 13 source:** `components/status_badges.slint`. Currently:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/components/status_badges.slint" lines="46-49" />

The component declares four `mock-*` strings. We're going to promote these to a single `Bridge.status-items: [StatusItem]` collection (the existing `StatusItem` struct in `bridge.slint:94-98` already has the right shape).

#### 2.1.1 Promote in `bridge.slint`

```diff
 export global Bridge {
+    // Phase 13 status badges — populated by encoder pipeline metrics.
+    in property <[StatusItem]> status-items: [];
     /* … */
 }
```

**Doc:** `…coding/repetition-and-data-models.mdx`.

#### 2.1.2 Migrate the consumer (`components/status_badges.slint`)

```diff
 export component StatusBadgesRow inherits HorizontalLayout {
-    // ── UI-only stub state ───────────────────────────────────────────
-    in-out property <int>    mock-battery-pct: 87;
-    in-out property <bool>   mock-charging:    false;
-    in-out property <string> mock-thermal:     "Nominal";
-    in-out property <string> mock-network:     "Wi-Fi";
-
     spacing: Theme.spacing-default;
-    Badge { /* network */
-        icon-glyph: "📶";
-        value: root.mock-network;
-    }
-    Badge { /* thermal */ … }
-    Badge { /* battery */ … }
+    for item in Bridge.status-items: Badge {
+        icon-glyph: item.label;     // glyph lives in StatusItem.label per Phase 13
+        value: item.value;
+        fg: item.severity == StatusSeverity.error   ? Theme.error-fg
+          : item.severity == StatusSeverity.warning ? Theme.warning-fg
+          : Theme.text-secondary;
+    }
 }
```

The visual change: a Phase-13-shaped row that adapts to whatever Rust provides. The old hardcoded battery / thermal / network ordering is gone — Rust orders the items.

#### 2.1.3 Wire the producer in `lib.rs`

Find a place that already runs at startup (a good anchor is right after `set_quick_actions`, line 1002 of master):

```rust
// senders/android/src/lib.rs — near line 1003, after the quick_actions producer
ui.global::<Bridge>().set_status_items(
    std::rc::Rc::new(slint::VecModel::from(vec![
        // Initial empty/default. Replaced when encoder pipeline starts.
        StatusItem {
            label: "📶".into(),
            value: "Wi-Fi".into(),
            severity: StatusSeverity::Info,
        },
        StatusItem {
            label: "🌡".into(),
            value: "Nominal".into(),
            severity: StatusSeverity::Info,
        },
        StatusItem {
            label: "🔋".into(),
            value: "—".into(),
            severity: StatusSeverity::Info,
        },
    ]))
    .into(),
);
```

When the real metrics-publishing path lands (battery polling, thermal-state listener, network-change broadcast receiver), each pushes a fresh `set_status_items(...)`. For Phase 8 the static initial set is enough to make the UI compile and render; **no behaviour regression** vs. the stub.

**Doc:** `…tutorial/creating_the_tiles.mdx` (canonical `Rc<VecModel<…>>` push pattern), `…coding/repetition-and-data-models.mdx`.

#### 2.1.4 Verification

```sh
grep -n 'mock-' senders/android/ui/components/status_badges.slint
# Expected: 0 matches.

grep -n 'set_status_items' senders/android/src/lib.rs
# Expected: 1 match.

cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

Commit: `feat(bridge): promote Phase 13 status-items to Bridge (Cluster A1)`.

### 2.2 A2 — App version

**File:** `pages/settings_page.slint`, line 78.

#### 2.2.1 Promote in `bridge.slint`

```diff
 export global Bridge {
+    in property <string> app-version: "0.0.1-dev";   // overwritten by Rust at startup
 }
```

#### 2.2.2 Migrate `settings_page.slint`

```diff
 export component FullSettingsPage inherits Rectangle {
-    in-out property <string> mock-app-version: "0.0.1-dev";
     /* … */
                     SettingsValueRow {
                         title: @tr("App version");
-                        value: root.mock-app-version;
+                        value: Bridge.app-version;
                         show-chevron: false;
                     }
 }
```

#### 2.2.3 Wire in `lib.rs`

```rust
// At startup, after `let ui = MainWindow::new().unwrap();`
ui.global::<Bridge>().set_app_version(env!("CARGO_PKG_VERSION").into());
```

**Doc:** `env!` is a `std` macro (Rust); `set_app_version` is generated by `slint_build` from the `in property <string> app-version` declaration — see `…guide/language/coding/properties.mdx` for the `in` semantics.

#### 2.2.4 Verification

```sh
grep -n 'mock-app-version' senders/android/ui/
# Expected: 0 matches.

grep -n 'set_app_version' senders/android/src/lib.rs
# Expected: 1 match.
```

Commit: `feat(bridge): wire app-version from CARGO_PKG_VERSION (Cluster A2)`.

### 2.3 A3 — Network interfaces

**File:** `pages/network_page.slint`, line 139.

#### 2.3.1 Promote

```diff
 export global Bridge {
+    in property <[NetworkInterface]> network-interfaces: [];
 }
```

#### 2.3.2 Consumer

```diff
 export component NetworkPage inherits Rectangle {
-    in-out property <[NetworkInterface]> mock-interfaces: [
-        /* … hardcoded entries … */
-    ];
     /* …
        Replace `for iface in root.mock-interfaces:` with
                 `for iface in Bridge.network-interfaces:`
        … */
 }
```

The wifi-aware toggle stays Slint-side until B5.

#### 2.3.3 Producer (`lib.rs`)

```rust
// helpers above setup
fn enumerate_network_interfaces() -> Vec<NetworkInterface> {
    // On Android, JNI-call NetworkInterface.getNetworkInterfaces() and pluck
    // (name, ipv4, ipv6, kind, connected). For Phase 8's bring-up, return
    // a single-row vec so the page renders.
    vec![NetworkInterface {
        name: "wlan0".into(),
        kind: "wifi".into(),
        address_v4: "—".into(),
        address_v6: "—".into(),
        enabled: true,
    }]
}

// At startup
ui.global::<Bridge>().set_network_interfaces(
    std::rc::Rc::new(slint::VecModel::from(enumerate_network_interfaces())).into(),
);
```

The real JNI implementation is *not* part of Phase 8 — that lives in a follow-up "real interface enumeration" task. Phase 8 just gets the wiring right so a producer can drop into one place.

#### 2.3.4 Verification — same shape as 2.1.4. Commit: `feat(bridge): promote network interfaces (Cluster A3)`.

### 2.4 A4 — Recording elapsed counter

**File:** `pages/recording_page.slint`, lines 26-27.

This one is unusual — the page currently runs a Slint-side `Timer` to drive `mock-elapsed-s`. We replace the Timer with a Rust-driven push.

#### 2.4.1 Promote

```diff
 export global Bridge {
+    in property <RecordingState> recording-state: RecordingState.idle;
+    in property <int>            recording-elapsed-s: 0;
 }
```

`RecordingState` already exists in `bridge.slint:18-23`.

#### 2.4.2 Consumer

```diff
 export component RecordingPage inherits Rectangle {
-    in-out property <RecordingState> mock-state:        RecordingState.idle;
-    in-out property <int>             mock-elapsed-s:    0;
     /* … */
-    // ── 1-second tick driving the elapsed counter ────────────────────────
-    Timer {
-        interval: 1s;
-        running: root.mock-state == RecordingState.recording;
-        triggered => { root.mock-elapsed-s += 1; }
-    }
     /* … */
+    // Page-derived view of the canonical Bridge state.
+    property <RecordingState> state: Bridge.recording-state;
+    property <int>            elapsed-s: Bridge.recording-elapsed-s;
+
+    /* Replace `root.mock-state`   → `root.state`
+               `root.mock-elapsed-s` → `root.elapsed-s` everywhere */
 }
```

The Slint Timer goes away. Rust now owns the counter.

#### 2.4.3 Producer (Rust `lib.rs`)

The Rust side has to start an interval task when state transitions to `recording`, stop it when transitioning to `idle`/`finalizing`. Since we don't have a real recorder pipeline yet, Phase 8's producer is a thin placeholder that mirrors the old Slint Timer:

```rust
// Inside the runtime closure. Spawn once, drive on state transitions.
let ui_weak_for_recording = ui.as_weak();
runtime.spawn(async move {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        let _ = ui_weak_for_recording.upgrade_in_event_loop(|ui| {
            let bridge = ui.global::<Bridge>();
            if bridge.get_recording_state() == RecordingState::Recording {
                let n = bridge.get_recording_elapsed_s();
                bridge.set_recording_elapsed_s(n + 1);
            }
        });
    }
});
```

**Risk register R2 (`upgrade_in_event_loop` panics if UI gone)** — the `let _ = ...` discard is correct here; we don't care about the result if the window is closed.

#### 2.4.4 Callbacks for the buttons (B3 — covered in step 3)

The big-button click currently calls `root.on-record-clicked()` which mutates `mock-state`. In step 3 we replace it with `Bridge.start-recording()` etc. For **A4 alone**, leave the buttons as-is — they keep mutating a now-unused page property; we'll swap them in B3.

(*If you prefer to combine A4 + B3 into one commit, that's fine — they're contiguous.*)

#### 2.4.5 Verification — same shape. Commit: `feat(bridge): promote recording state + elapsed counter (Cluster A4)`.

### 2.5 A5 — Debug log entries

**File:** `pages/debug_log_page.slint`, lines 46-58.

#### 2.5.1 Promote

```diff
 export global Bridge {
+    in property <[LogEntry]> log-entries: [];
 }
```

#### 2.5.2 Consumer

```diff
 export component DebugLogPage inherits Rectangle {
-    in-out property <[LogEntry]> mock-log: [
-        /* … hardcoded log entries … */
-    ];
     in-out property <int> mock-min-level-idx: 1;        // STAYS — pure UI filter
     /* … replace `root.mock-log` → `Bridge.log-entries` … */
 }
```

#### 2.5.3 Producer (`lib.rs`) — bounded ring buffer

```rust
use std::sync::Arc;
use std::sync::Mutex;

struct LogRing {
    entries: Mutex<std::collections::VecDeque<LogEntry>>,
    cap: usize,
}

impl LogRing {
    fn new(cap: usize) -> Self {
        Self { entries: Mutex::new(VecDeque::with_capacity(cap)), cap }
    }
    fn push(&self, e: LogEntry) {
        let mut g = self.entries.lock().unwrap();
        if g.len() == self.cap { g.pop_front(); }
        g.push_back(e);
    }
    fn snapshot(&self) -> Vec<LogEntry> {
        self.entries.lock().unwrap().iter().cloned().collect()
    }
}

let log_ring = Arc::new(LogRing::new(500));

// Wire to a tracing-subscriber layer that calls log_ring.push(...) on each event.
// On every push, also schedule a UI update. Throttle if traffic is high (R3).
let ui_weak_for_log = ui.as_weak();
let log_ring_for_subscriber = log_ring.clone();
// (subscriber init goes here; on each event:)
//   log_ring_for_subscriber.push(...);
//   let snapshot = log_ring_for_subscriber.snapshot();
//   let _ = ui_weak_for_log.upgrade_in_event_loop(move |ui| {
//       ui.global::<Bridge>().set_log_entries(
//           std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
//       );
//   });
```

The `clear-log-entries` callback is added in C5 (step 4).

#### 2.5.4 Verification, commit. Cluster A complete.

---

## Section 3 — Step 3 / Cluster B: Single-page state with one or two callbacks

These are the "live data" promotions — settings panels whose values are read by Rust at cast-start time.

### 3.1 B1 — Audio settings

**File:** `pages/audio_page.slint`, line 13 onwards.

#### 3.1.1 Promote (one entry per `mock-*`)

```diff
 export global Bridge {
+    in-out property <int>   audio-source-idx: 0;
+    in-out property <bool>  audio-muted: false;
+    in-out property <float> audio-input-gain: 0.7;
+    in-out property <int>   audio-bitrate-idx: 1;
 }
```

Use `in-out` here because the Slint UI directly mutates these (cycler `clicked => { idx = mod(idx+1, N); }`). Rust reads them when starting a cast.

#### 3.1.2 Consumer

```diff
 export component AudioPage inherits Rectangle {
-    in-out property <int>   mock-source-idx:   0;
-    in-out property <bool>  mock-muted:        false;
-    in-out property <float> mock-input-gain:   0.7;
-    in-out property <int>   mock-bitrate-idx:  1;
     /* …
        Replace every `root.mock-x` → `Bridge.audio-x`.
        The cycler clicked handlers stay in Slint (no callback round-trip).
        … */
 }
```

#### 3.1.3 Rust-side reader (no producer)

The audio settings are *consumed* by Rust at cast-start. Update `on_start_casting`:

```rust
ui.global::<Bridge>().on_start_casting({
    let event_tx = event_tx.clone();
    let ui_weak = ui.as_weak();
    move |scale_width: i32, scale_height: i32, max_framerate: i32| {
        let (audio_source_idx, audio_muted, audio_input_gain, audio_bitrate_idx) =
            ui_weak.upgrade()
                .map(|ui| {
                    let b = ui.global::<Bridge>();
                    (b.get_audio_source_idx(), b.get_audio_muted(),
                     b.get_audio_input_gain(), b.get_audio_bitrate_idx())
                })
                .unwrap_or_default();
        event_tx
            .send(Event::StartCast {
                scale_width: scale_width as u32,
                scale_height: scale_height as u32,
                max_framerate: max_framerate as u32,
                audio_source_idx, audio_muted,
                audio_input_gain, audio_bitrate_idx,
            })
            .unwrap();
    }
});
```

Then update `Event::StartCast` in `events.rs` (or wherever your event enum lives) to carry the new fields, and the `Application::run_event_loop` consumer to forward them to the encoder pipeline.

**Optional** — if you also want Rust to react to live audio-setting changes (e.g. mid-cast), declare callbacks:

```diff
 callback start-casting(scale-width: int, scale-height: int, max-framerate: int);
+callback audio-source-changed(int);
+callback audio-muted-changed(bool);
```

and switch the cycler from `clicked => { Bridge.audio-source-idx = ...; }` to `clicked => { let n = ...; Bridge.audio-source-idx = n; Bridge.audio-source-changed(n); }`. Most pipelines re-read at start, so the optional callbacks aren't necessary for parity.

#### 3.1.4 Verification + commit. Pattern is identical for 3.2 and 3.3.

### 3.2 B2 — Camera settings

**File:** `pages/camera_page.slint`. Same pattern as B1 with more properties (source / resolution / framerate / mirror / stabilization / tap-to-focus / zoom). Rust reads all at cast-start.

### 3.3 B3 — Recording controls (callbacks)

**File:** `pages/recording_page.slint` (continuing from A4).

#### 3.3.1 Add callbacks in `bridge.slint`

```diff
 export global Bridge {
+    callback start-recording();
+    callback pause-recording();
+    callback resume-recording();
+    callback stop-recording();
 }
```

#### 3.3.2 Consumer

```diff
 // recording_page.slint — replace the on-record-clicked() helper body.
-    function on-record-clicked() {
-        if (root.mock-state == RecordingState.idle) {
-            root.mock-state = RecordingState.recording;
-            root.mock-elapsed-s = 0;
-        } else if (root.mock-state == RecordingState.recording) {
-            root.mock-state = RecordingState.paused;
-        } else if (root.mock-state == RecordingState.paused) {
-            root.mock-state = RecordingState.recording;
-        }
-    }
-
-    function on-stop-clicked() {
-        if (root.mock-state == RecordingState.recording
-         || root.mock-state == RecordingState.paused) {
-            root.mock-state = RecordingState.finalizing;
-            root.mock-state = RecordingState.idle;
-            root.mock-elapsed-s = 0;
-        }
-    }
+    function on-record-clicked() {
+        if (root.state == RecordingState.idle)        { Bridge.start-recording(); }
+        else if (root.state == RecordingState.recording) { Bridge.pause-recording(); }
+        else if (root.state == RecordingState.paused)    { Bridge.resume-recording(); }
+    }
+    function on-stop-clicked() { Bridge.stop-recording(); }
```

The button glyph helpers can stay as-is (`root.state == ...?` ternaries already updated in A4).

#### 3.3.3 Producer (`lib.rs`)

```rust
ui.global::<Bridge>().on_start_recording({
    let ui_weak = ui.as_weak();
    move || {
        // Real impl: start a MediaRecorder via JNI, transition state on success.
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            let b = ui.global::<Bridge>();
            b.set_recording_elapsed_s(0);
            b.set_recording_state(RecordingState::Recording);
        });
    }
});
ui.global::<Bridge>().on_pause_recording({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_recording_state(RecordingState::Paused);
        });
    }
});
// resume + stop analogously.
```

The interval task from A4 only ticks while `state == Recording`, so the elapsed counter naturally pauses.

#### 3.3.4 Verification, commit. Cluster B is complete.

### 3.4 B4 — Lifecycle modes

The settings PRIVACY rows currently flip `Bridge.lifecycle = LifecycleMode.x` directly (lines 119, 124, 131 of `settings_page.slint`). Promote to callbacks so Rust can also drive a real lock / stealth / countdown:

```diff
 export global Bridge {
+    callback engage-lock();
+    callback engage-stealth();
+    callback start-snapshot-countdown(int);
 }
```

```diff
 // settings_page.slint
                     SettingsValueRow {
                         title: @tr("Lock UI");
                         value: @tr("Enter");
-                        clicked => { Bridge.lifecycle = LifecycleMode.lock-screen; }
+                        clicked => { Bridge.engage-lock(); }
                     }
                     SettingsValueRow {
                         title: @tr("Stealth mode");
                         value: @tr("Enter");
-                        clicked => { Bridge.lifecycle = LifecycleMode.stealth; }
+                        clicked => { Bridge.engage-stealth(); }
                     }
                     SettingsValueRow {
                         title: @tr("Cast with countdown");
-                        clicked => {
-                            Bridge.mock-snapshot-secs = 5;
-                            Bridge.lifecycle = LifecycleMode.snapshot-countdown;
-                        }
+                        clicked => { Bridge.start-snapshot-countdown(5); }
                     }
```

Rust handlers (`lib.rs`):

```rust
ui.global::<Bridge>().on_engage_lock({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_lifecycle(LifecycleMode::LockScreen);
        });
        // Real impl: call Android KeyguardManager via JNI.
    }
});
ui.global::<Bridge>().on_engage_stealth({
    let ui_weak = ui.as_weak();
    move || {
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_lifecycle(LifecycleMode::Stealth);
        });
        // Real impl: setSystemUiVisibility(FLAG_FULLSCREEN | FLAG_HIDE_NAVIGATION).
    }
});
ui.global::<Bridge>().on_start_snapshot_countdown({
    let ui_weak = ui.as_weak();
    move |secs| {
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            let b = ui.global::<Bridge>();
            b.set_mock_snapshot_secs(secs);
            b.set_lifecycle(LifecycleMode::SnapshotCountdown);
        });
    }
});
```

`Bridge.mock-snapshot-secs` also gets renamed to `Bridge.snapshot-secs` (drop the `mock-` prefix) in this step — pure rename, run `sed -i 's/mock-snapshot-secs/snapshot-secs/g' senders/android/ui/**/*.slint senders/android/src/lib.rs`.

### 3.5 B5 — Wi-Fi Aware toggle (Phase 22)

**File:** `pages/network_page.slint`, line 144.

```diff
 export global Bridge {
+    in-out property <bool> wifi-aware: false;
+    callback set-wifi-aware(bool);
 }
```

```diff
 // network_page.slint
-    in-out property <bool> mock-wifi-aware-enabled: false;
     /* …
        toggled(v) => {
-           root.mock-wifi-aware-enabled = v;
+           Bridge.set-wifi-aware(v);
            root.banner-visible = true;
        }
        … */
```

Rust:

```rust
ui.global::<Bridge>().on_set_wifi_aware({
    let ui_weak = ui.as_weak();
    move |v| {
        // Real impl: WifiAwareManager.attach() / detach() via JNI.
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            let b = ui.global::<Bridge>();
            b.set_wifi_aware(v);
            b.set_banner_message(if v { "Wi-Fi Aware enabled" } else { "Wi-Fi Aware disabled" }.into());
            b.set_banner_visible(true);
        });
    }
});
```

Cluster B done. Verify, commit.

---

## Section 4 — Step 4 / Cluster C: List pages with mutations

These pages have a list-shaped `mock-*` (e.g. presets, bar actions, macros) and **multiple** mutators (add/remove/edit/swap/select). Each mutator becomes a callback; Rust holds the canonical list and pushes back via `set_*`.

### 4.1 C1 — Bitrate presets

**Files:** `pages/bitrate_presets_page.slint`, `pages/bitrate_preset_edit_page.slint`.

#### 4.1.1 Promote

```diff
 export global Bridge {
+    in property <[BitratePreset]> presets: [];
+    in-out property <string>      selected-preset-id: "";
+    callback save-preset(string, string, int);   // (id, name, kbps)  id="" means "new"
+    callback delete-preset(string);
+    callback set-active-preset(string);
 }
```

#### 4.1.2 Consumers

```diff
 // bitrate_presets_page.slint
 export component BitratePresetsPage inherits Rectangle {
-    in-out property <[BitratePreset]> mock-presets: [
-        /* … 4 hardcoded entries … */
-    ];
-
-    function select(id: string) {
-        root.mock-presets = [
-            /* … hardcoded 4-row rebuild … */
-        ];
-    }
     /* …
        for preset[i] in root.mock-presets:
        → for preset[i] in Bridge.presets:
        TouchArea clicked => { root.select(preset.id); }
        → TouchArea clicked => { Bridge.set-active-preset(preset.id); }
        … */
 }
```

The hardcoded N-row rebuild helper goes away — Rust holds the canonical list and rewrites it on each `set-active-preset` call.

```diff
 // bitrate_preset_edit_page.slint
 export component BitratePresetEditPage inherits Rectangle {
-    in-out property <string> mock-name:         "New preset";
     /* …
        On Save:
-       Bridge.active-panel = Panel.bitrate-presets;
+       Bridge.save-preset(Bridge.selected-preset-id, root.draft-name, root.draft-kbps);
+       Bridge.active-panel = Panel.bitrate-presets;
        … */
 }
```

#### 4.1.3 Producer + handlers (`lib.rs`)

```rust
let presets: Arc<Mutex<Vec<BitratePreset>>> = Arc::new(Mutex::new(vec![
    BitratePreset { id: "low".into(),  name: "Low".into(),     bitrate_kbps: 1500,  active: false },
    BitratePreset { id: "med".into(),  name: "Medium".into(),  bitrate_kbps: 4000,  active: true  },
    BitratePreset { id: "high".into(), name: "High".into(),    bitrate_kbps: 8000,  active: false },
    BitratePreset { id: "max".into(),  name: "Maximum".into(), bitrate_kbps: 15000, active: false },
]));

let push_presets = {
    let presets = presets.clone();
    let ui_weak = ui.as_weak();
    move || {
        let snapshot = presets.lock().unwrap().clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_presets(
                std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
            );
        });
    }
};
push_presets();   // initial render

ui.global::<Bridge>().on_save_preset({
    let presets = presets.clone();
    let push = push_presets.clone();
    move |id, name, kbps| {
        let mut g = presets.lock().unwrap();
        if id.is_empty() {
            g.push(BitratePreset {
                id: format!("custom-{}", g.len()).into(),
                name: name.into(),
                bitrate_kbps: kbps,
                active: false,
            });
        } else if let Some(p) = g.iter_mut().find(|p| p.id == id) {
            p.name = name.into();
            p.bitrate_kbps = kbps;
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_delete_preset({
    let presets = presets.clone();
    let push = push_presets.clone();
    move |id| {
        presets.lock().unwrap().retain(|p| p.id != id);
        push();
    }
});

ui.global::<Bridge>().on_set_active_preset({
    let presets = presets.clone();
    let push = push_presets.clone();
    move |id| {
        for p in presets.lock().unwrap().iter_mut() {
            p.active = p.id == id;
        }
        push();
    }
});
```

**Risk register R3** (`VecModel::from` allocates on every push) — for a 4-row presets list this is fine. Reuse the **same** `Rc<VecModel<…>>` and call `.set_vec()` on it instead if your list grows >100 entries.

#### 4.1.4 Verification, commit.

### 4.2 C2 — Quick-action customisation (and the live `CastControlBar` unification)

This is the **architectural** fix for B12 in [`UI-REVIEW-2026-05-10.md`](./UI-REVIEW-2026-05-10.md). It's the most important migration in Cluster C because it affects the always-visible bar.

#### 4.2.1 Promote (re-use existing `Bridge.quick-actions`)

`Bridge.quick-actions` already exists. We just need to repurpose `mock-bar-actions` (the customisation page) to mutate the same canonical list:

```diff
 export global Bridge {
     in property <[QuickAction]> quick-actions: [];   // unchanged

+    callback move-bar-action(int, int);              // (from-idx, to-idx)
+    callback set-bar-action-enabled(int, bool);      // (idx, enabled)
+    callback save-bar-actions();                     // commit pending edits
 }
```

#### 4.2.2 Consumers

```diff
 // components/control_bar.slint  ← FIX FOR B12
 export component CastControlBar inherits Rectangle {
-    in-out property <[QuickAction]> mock-quick-actions: [
-        /* … 8 hardcoded entries that ignore the Rust model … */
-    ];

     HorizontalLayout {
         /* … */
-        for action in root.mock-quick-actions: QuickActionButton {
+        for action in Bridge.quick-actions: QuickActionButton {
             /* … */
         }
     }
 }
```

```diff
 // pages/quick_actions_page.slint
 export component QuickActionsPage inherits Rectangle {
-    in-out property <[QuickAction]> mock-bar-actions: [
-        /* … 5 hardcoded entries … */
-    ];
-
-    function swap(i: int, j: int) {
-        /* … hardcoded 5-row rebuild … */
-    }
-    function set-enabled(i: int, v: bool) {
-        /* … hardcoded 5-row rebuild … */
-    }
     /* …
        for action[i] in root.mock-bar-actions:
        → for action[i] in Bridge.quick-actions:
        clicked → Bridge.move-bar-action(...) / Bridge.set-bar-action-enabled(...)
        … */
 }
```

#### 4.2.3 Producer + handlers (`lib.rs`)

```rust
// Replace the existing actions vec at line 988-1000 with the SUPERSET that
// the bar shows in the screenshots:
let mut actions = vec![
    QuickAction { id: "settings".into(),        title: "Settings".into(),     enabled: true, active: false, is_macro: false },
    QuickAction { id: "debug".into(),           title: "Debug".into(),        enabled: true, active: false, is_macro: false },
    QuickAction { id: "codec-test".into(),      title: "Codec test".into(),   enabled: true, active: false, is_macro: false },
    QuickAction { id: "scan-qr".into(),         title: "Scan QR".into(),      enabled: true, active: false, is_macro: false },
    QuickAction { id: "record".into(),          title: "Record".into(),       enabled: true, active: false, is_macro: false },
    QuickAction { id: "pair".into(),            title: "Pair".into(),         enabled: true, active: false, is_macro: false },
    QuickAction { id: "bitrate".into(),         title: "Bitrate".into(),      enabled: true, active: false, is_macro: false },
];
let show_debug = cfg!(debug_assertions);
ui.global::<Bridge>().set_show_debug(show_debug);
if show_debug {
    actions.extend([
        QuickAction { id: "migrated-server".into(), title: "Migrated srv".into(), enabled: true, active: false, is_macro: false },
        QuickAction { id: "test-getinfo".into(),    title: "GetInfo".into(),      enabled: true, active: false, is_macro: false },
        QuickAction { id: "test-crossfade".into(),  title: "Crossfade".into(),    enabled: true, active: false, is_macro: false },
        QuickAction { id: "test-smoke".into(),      title: "Smoke Graph".into(),  enabled: true, active: false, is_macro: false },
    ]);
}

let bar_actions: Arc<Mutex<Vec<QuickAction>>> = Arc::new(Mutex::new(actions));
let push_bar = {
    let bar_actions = bar_actions.clone();
    let ui_weak = ui.as_weak();
    move || {
        let snapshot = bar_actions.lock().unwrap().clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_quick_actions(
                std::rc::Rc::new(slint::VecModel::from(snapshot)).into(),
            );
        });
    }
};
push_bar();

ui.global::<Bridge>().on_move_bar_action({
    let bar_actions = bar_actions.clone();
    let push = push_bar.clone();
    move |from, to| {
        let mut g = bar_actions.lock().unwrap();
        if let (Some(from_u), Some(to_u)) = (usize::try_from(from).ok(), usize::try_from(to).ok()) {
            if from_u < g.len() && to_u < g.len() && from_u != to_u {
                let item = g.remove(from_u);
                g.insert(to_u, item);
            }
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_set_bar_action_enabled({
    let bar_actions = bar_actions.clone();
    let push = push_bar.clone();
    move |idx, enabled| {
        let mut g = bar_actions.lock().unwrap();
        if let Some(i) = usize::try_from(idx).ok() {
            if let Some(a) = g.get_mut(i) { a.enabled = enabled; }
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_save_bar_actions({
    let push = push_bar.clone();
    move || {
        // Persist to disk (DataStore / SharedPreferences via JNI), then re-push.
        push();
    }
});
```

After this commit lands, B12 from the UI review is **fixed** — the bar reads from the same `Bridge.quick-actions` that the customisation page mutates.

#### 4.2.4 Verification

```sh
grep -n 'mock-quick-actions\|mock-bar-actions' senders/android/ui/
# Expected: 0 matches.

slint-viewer senders/android/ui/components/control_bar.slint
# Bar renders empty (no Bridge in viewer) — that's expected; verify in the running app instead.

cargo build -p android-sender
adb install target/.../android-sender.apk    # or your usual install path
adb shell am start org.fcast.android.sender/.MainActivity
# Verify the bar shows 7 actions in release / 11 actions in debug.
```

Commit: `feat(bridge): unify CastControlBar + customisation onto Bridge.quick-actions (Cluster C2)`.

### 4.3 C4 — Macros

**Files:** `pages/macros_page.slint`, `pages/macro_edit_page.slint`. Largest callback surface in Phase 8.

```diff
 export global Bridge {
+    in property <[Macro]> macros: [];
+    callback save-macro(string, string, [MacroStep], bool);  // (id, name, steps, enabled); id="" → new
+    callback delete-macro(string);
+    callback move-step(string, int, int);                    // (macro-id, from-idx, to-idx)
+    callback add-step(string, string);                       // (macro-id, action-id)
+    callback remove-step(string, int);                       // (macro-id, step-idx)
+    callback run-macro(string);                              // (macro-id)
 }
```

Rename `Bridge.mock-macro-edit-id` → `Bridge.macro-edit-id` in the same step.

The Slint pages (`macros_page.slint`, `macro_edit_page.slint`) get the same treatment as 4.1: replace `mock-macros` consumers with `Bridge.macros`, replace direct mutations with callbacks.

The control-bar `▶` glyph for `id.starts-with("macro:")` (added in Phase 17) **stays Slint-side** — it's purely visual, no Rust round-trip.

Rust-side: same `Arc<Mutex<Vec<Macro>>>` + push-snapshot pattern as C1/C2, plus a `run_macro(id)` execution engine. Phase 8's bring-up impl can be a stub that logs the run; the real engine is a follow-up task.

### 4.4 C5 — Debug log filter / clear

**File:** `pages/debug_log_page.slint`.

`mock-min-level-idx` **stays Slint-side** (pure UI filter).

Add the clear callback:

```diff
 export global Bridge {
+    callback clear-log-entries();
 }
```

```diff
 // debug_log_page.slint — replace direct clear with callback.
-                clicked => { /* clear locally */ }
+                clicked => { Bridge.clear-log-entries(); }
```

Rust handler (continuing from A5):

```rust
ui.global::<Bridge>().on_clear_log_entries({
    let log_ring = log_ring.clone();
    let ui_weak = ui.as_weak();
    move || {
        log_ring.entries.lock().unwrap().clear();
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_log_entries(
                std::rc::Rc::new(slint::VecModel::from(Vec::<LogEntry>::new())).into(),
            );
        });
    }
});
```

Cluster C done. 4 commits, one per item.

---

## Section 5 — Step 5 / Cluster D: Destructive flows

These are the high-stakes migrations — anything that deletes user data needs the round-trip through Rust so a real implementation can show OS-level confirmation, do the destructive work, and surface a banner on completion.

### 5.1 D1 — Backup / reset

**File:** `pages/backup_reset_page.slint`.

Add callbacks:

```diff
 export global Bridge {
+    callback export-settings();
+    callback import-settings();
+    callback reset-settings();
+    callback clear-cast-history();
+    callback clear-known-receivers();
 }
```

The page's `pending-action: string` and the `on-confirm()` dispatcher **stay Slint-side**. Their *effects* (the actual export / import / reset) become Rust callbacks. Replace each `pending-action` branch's body with a `Bridge.<callback>()` invocation.

Rust handlers (`lib.rs`):

```rust
ui.global::<Bridge>().on_export_settings({
    let ui_weak = ui.as_weak();
    move || {
        // Real impl: launch ACTION_CREATE_DOCUMENT via JNI.
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            let b = ui.global::<Bridge>();
            b.set_banner_message("Settings export started…".into());
            b.set_banner_severity(BannerSeverity::Info);
            b.set_banner_visible(true);
        });
    }
});
ui.global::<Bridge>().on_import_settings({ /* analogous */ });
ui.global::<Bridge>().on_reset_settings({
    let ui_weak = ui.as_weak();
    move || {
        // Real impl: clear all DataStore keys / SharedPreferences. Then push reset state.
        let _ = ui_weak.upgrade_in_event_loop(|ui| {
            let b = ui.global::<Bridge>();
            b.set_banner_message("Settings reset to defaults".into());
            b.set_banner_severity(BannerSeverity::Success);
            b.set_banner_visible(true);
        });
    }
});
```

The banner is now driven from Bridge (Cluster F1 of Section 1).

### 5.2 D2 — Cast history

**Files:** `pages/cast_history_page.slint`, `pages/cast_history_detail_page.slint`.

```diff
 export global Bridge {
+    in property <[CastHistoryEntry]> history: [];
+    in property <CastHistoryEntry>   selected-history-entry;
+    callback clear-history();
+    callback delete-history-entry(string);
+    callback recast(string);                          // (entry-id)
 }
```

`Bridge.selected-history-id` already exists. The detail page's old `find-entry(id)` Slint helper (a hardcoded N-row search) is replaced by Rust pushing `selected-history-entry` whenever `selected-history-id` changes:

```rust
// In lib.rs, on a property-changed handler. Slint exposes a
// `set_selected_history_id` callback for two-way binding; you can also poll.
ui.global::<Bridge>().on_set_selected_history_id({   // hypothetical callback name
    let ui_weak = ui.as_weak();
    let history = history.clone();
    move |id| {
        if let Some(entry) = history.lock().unwrap().iter().find(|e| e.id == id).cloned() {
            let _ = ui_weak.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_selected_history_entry(entry);
            });
        }
    }
});
```

A simpler shape: declare `selected-history-id` as `in-out` and have the detail page read **both** from `Bridge` (id) and a Slint-side `find-entry` that searches `Bridge.history`. That keeps the detail page's existing logic; Rust just pushes the source of truth (`Bridge.history`) and the search runs in Slint. Both shapes work — pick the one that minimises diff.

### 5.3 D3 — ADVANCED reset

Currently no destructive ADVANCED row exists. Skip — there's nothing to migrate.

Cluster D complete.

---

## Section 6 — Step 6 / Cluster E: Document overlay invariants

**No code changes.** Cluster E's job is to write down the discipline so subsequent phases don't break it. Add the following to `senders/android/ui/main.slint` (top-of-file comment) or to a new `phases/PANEL-INVARIANTS.md`:

```text
INVARIANT — Bridge.active-panel
  - Mutated from Slint by panel-opening clicks (settings row, control-bar
    button, page-internal navigation).
  - Mutated from Rust ONLY through a single dispatcher: open_panel(p: Panel).
    Do not write Bridge.active-panel from multiple Rust call sites; if you
    need to surface a panel from a Rust path (e.g. "connection lost" auto-
    routes to a status panel), call open_panel.
  - The chain of `if Bridge.active-panel == Panel.x:` blocks in main.slint is
    read-only from the implementation's perspective. New panels add a single
    `if` branch; the old branches stay in the same order.

INVARIANT — Bridge.lifecycle (LifecycleMode)
  - Mutated from BOTH Slint (settings PRIVACY rows: lock / stealth /
    countdown) AND Rust (real lock-engagement / inactivity / countdown
    completion). Dual writers are fine HERE because the rows mutate via
    callbacks (engage-lock / engage-stealth / start-snapshot-countdown)
    that go through Rust. Rust is the only writer that calls set_lifecycle.

INVARIANT — Bridge.app-state (AppState)
  - Mutated from Slint via the public function Bridge.change-state(to).
  - Rust calls Bridge.change-state(...) to drive lifecycle transitions.
    Never mutate Bridge.app-state directly; always go through change-state
    so future hooks (e.g. logging, analytics) attach in one place.
```

Commit: `docs(slint): document Panel / LifecycleMode / AppState invariants (Cluster E)`.

---

## Section 7 — Per-cluster verification

After **each** cluster, run all of these. Don't batch — catch regressions per cluster.

### 7.1 No remaining `mock-*` on the migrated surface

```sh
# Per-cluster — after migrating files <list>:
grep -n 'mock-' senders/android/ui/<paths-touched-this-cluster>
# Expected: 0 matches (or only an explicit "intentionally page-local"
# comment, e.g. mock-min-level-idx in debug_log_page.slint).
```

### 7.2 The promoted Bridge property is declared

```sh
grep -n '<promoted_property>' senders/android/ui/bridge.slint
# Expected: at least 1 match.
```

### 7.3 The promoted callback has a Rust handler

```sh
grep -n 'on_<promoted_callback>\|set_<promoted_property>' senders/android/src/lib.rs
# Expected: 1+ matches per item in the cluster.
```

### 7.4 No Slint-side direct mutations of properties that should be Rust-driven

```sh
grep -n 'Bridge\.<promoted_property> *=' senders/android/ui/
# Expected: 0 matches (all writes flow through callbacks).
```

(Exception: `Bridge.lifecycle = ...` direct mutations stay in `LockOverlay` / `StealthOverlay` for state-transition exits — that's by design per Cluster E.)

### 7.5 Build + lint

```sh
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

### 7.6 Smoke test in `slint-viewer`

```sh
slint-viewer senders/android/ui/main.slint
# Each page renders. Without Rust, properties show their declared default
# (empty list, empty string, false, 0). That's expected — verify there are
# no compile errors and no obvious layout regressions.
```

### 7.7 On-device sanity (only after all clusters)

```sh
cargo build -p android-sender --release
# Install + open the app. Walk through:
#   1. Connect page — receivers populate (mDNS path).
#   2. Settings → Audio / Camera — values persist across panel open/close.
#   3. Settings → Bitrate — preset list, select active, edit a name, save.
#   4. Settings → Recording — start, pause, resume, stop. Elapsed counter ticks.
#   5. Settings → Backup & reset → Reset → confirm — banner appears.
#   6. Quick action bar — full action set visible per build mode.
#   7. Pair via QR — page renders (real QR comes later).
```

---

## Section 8 — Common pitfalls

These are the recurring traps you'll hit during execution. Each links to a specific risk in the migration plan's **R**egister.

### 8.1 Forgetting to remove the `mock-*` initialiser (R4)

If you add `Bridge.x` and the page binding `<=> Bridge.x` but **leave** `in-out property <T> mock-x: <stub>;` in the page, Slint silently shadows the binding with the stub on first frame — visible flicker. **Always** remove the stub initialiser in the same commit as the wiring.

```sh
# After every cluster migration:
grep -n 'mock-' senders/android/ui/pages/<migrated-page>.slint
# Should be 0.
```

### 8.2 Reactivity loop via `<=>` two-way binding (R1)

If both Slint and Rust write to the same `in-out` property, you can get a oscillation. Phase 8's discipline:

- Use `in property <T>` for Rust-pushed values (Slint reads, Rust writes via `set_<x>`). Examples: `presets`, `history`, `log-entries`, `network-interfaces`.
- Use `in-out property <T>` only when Slint *also* writes (cycler increments, slider drags, text fields). Examples: `audio-source-idx`, `audio-input-gain`, `selected-receiver-id`.
- Never use `<=>` to bind a `Bridge` property to a page-local property of the same name — collapses to a single property at compile time, and Rust's `set_<x>` writes also propagate up to the page, defeating the abstraction.

### 8.3 `upgrade_in_event_loop` panics if UI is gone (R2)

Always handle the `Result`:

```rust
let _ = ui_weak.upgrade_in_event_loop(|ui| { /* … */ });
```

Discarding with `let _ = ...` is the correct fire-and-forget shape. **Don't** unwrap — when the user backgrounds the app and Rust still has work to do, the upgrade fails.

### 8.4 `VecModel::from(...)` allocates a fresh model on every push (R3)

For low-volume lists (presets, network interfaces, history) this is fine. For high-volume lists (log entries at high tracing volume) reuse the `Rc<VecModel<T>>` and call `model.set_vec(snapshot)` to mutate in place.

### 8.5 `Bridge.active-panel` race conditions (R5)

If both Slint and Rust write to `Bridge.active-panel` in the same tick, the visible panel may not match what either side expects. Cluster E's invariant — "Slint writes via clicks; Rust writes via a single `open_panel(p)` dispatcher" — keeps this safe.

### 8.6 Slint enum naming (`-` vs `_` vs PascalCase)

Slint enums use kebab-case (`Panel.cast-history`, `RecordingState.recording`); Rust generated bindings use PascalCase (`Panel::CastHistory`, `RecordingState::Recording`). The compiler tells you which one to use — don't fight it.

### 8.7 Stub initialiser for a Rust-pushed property

```diff
+in property <[StatusItem]> status-items: [];
```

The `: []` initialiser is the value Slint sees **before** Rust's first `set_status_items` call. If your page's `for x in Bridge.status-items:` paints something jarring on the empty list (e.g. a "0 items" banner), provide a non-empty stub that matches what Rust would push for a default state.

---

## Section 9 — Stop conditions

Phase 8 is "done" — the placeholder gate in [`PHASE-8-rust-bridge.md`](./PHASE-8-rust-bridge.md) can be removed — when **all** of:

1. ✅ Every UI phase that shipped to `master` has its corresponding cluster items wired (per the index table in the migration plan).
2. ✅ Every `mock-*` property on a `pages/*.slint` or `components/*.slint` file is gone, OR is documented in the migration plan as "intentionally page-local" (e.g. `mock-min-level-idx`, `pending-action`).
3. ✅ Every `Bridge.*` property declared in `bridge.slint` has either a producer (Rust → UI `set_<x>`) or a consumer (UI → Rust `on_<callback>`) — no orphan declarations.
4. ✅ `cargo build -p android-sender` and `cargo clippy -p android-sender --all-targets -- -D warnings` are clean.
5. ✅ Every cluster verification passed (Section 7) — record per-commit which clusters' verifications were rerun.
6. ✅ The on-device sanity walkthrough (7.7) succeeds.

When done, also update:

- `STATUS.md` — flip Phase 8 from `[~] Migration plan only` to `[x] Complete` with the date and commit range.
- `PHASE-8-rust-bridge.md` — replace the "explicitly **deferred**" header with "Reactivated YYYY-MM-DD; see `PHASE-8-implementation-instructions.md` for the execution log".

---

## Section 10 — What's NOT in this guide

- **`@tr()` localisation sweep over the new strings.** That's Phase 9. Do it as a follow-up — every `set_*` call site that pushes user-visible text needs `@tr` wrapping in the Slint consumer, but the Rust side stays English-only (Slint's translator extractor walks `.slint` only).
- **Real implementations** (JNI calls, MediaRecorder integration, real DataStore persistence). The Rust handlers in this guide are *bring-up* — they put the wiring in place and push reasonable defaults. The actual side-effects ship in follow-up tasks per the project's broadcast / persistence architecture decisions.
- **Phase 27 `IconAndText` raster icon assets.** Add them when the asset pipeline lands; the Slint-side properties (`Theme.icon-*`) just resolve to `image` literals via `@image-url(...)`.
- **Test coverage for the wired bindings.** Phase 10. The on-device walkthrough in 7.7 is a smoke test, not a regression suite.
- **Removing the placeholder audit gate** in `PHASE-8-rust-bridge.md`. Do that as the final commit of Phase 8 — confirms the gate held until completion.
- **Architectural decisions** (where cast history persists, ring-buffer size for log entries, JNI threading model). These are project-level decisions the implementer makes; this guide stops at "the wiring exists and the placeholder defaults render".

---

## Slint-doc references used

Every pattern in this guide is grounded in an upstream doc path under `draft/slint-ui/docs/astro/src/content/docs/`. Verified to exist on disk on `master` (2026-05-09):

| Reference | Path |
|---|---|
| `global Bridge { … }` declaration | `guide/language/coding/globals.mdx` |
| `in` / `in-out` / `out` property semantics | `guide/language/coding/properties.mdx` |
| `<=>` two-way binding | `guide/language/coding/properties.mdx` |
| `callback name(args);` declaration | `guide/language/coding/functions-and-callbacks.mdx` |
| `[T]` model property + `for x in <model>:` | `guide/language/coding/repetition-and-data-models.mdx` |
| Slint enum ↔ Rust enum mapping | `guide/language/coding/structs-and-enums.mdx` |
| Conditional elements (`if x: Element { … }`) | `guide/language/coding/repetition-and-data-models.mdx` |
| Imports / re-exports | `guide/language/coding/file.mdx` |
| Expressions / statements (helpers, ternaries) | `guide/language/coding/expressions-and-statements.mdx` |
| Translation form (`@tr`, `=> "default"`) | `guide/development/translations.mdx` |
| `Image` element (for the eventual QR / icon migration) | `reference/elements/image.mdx` |
| `Rectangle` (`background`, `border-*`) | `reference/elements/rectangle.mdx` |
| `Text` (`font-family`, `wrap`) | `reference/elements/text.mdx` |
| `Timer` element (kept for short-lived UI flashes; promoted away for elapsed counters) | `reference/timer.mdx` |
| `ScrollView` (used by every panel) | `reference/std-widgets/views/scrollview.mdx` |
| `LineEdit` (settings input fields) | `reference/std-widgets/views/lineedit.mdx` |
| `VecModel<T>` and `ModelRc<T>` (canonical Rust-side patterns) | `tutorial/creating_the_tiles.mdx` (Rust tab) + the Slint Rust API rustdocs (out-of-tree) |

If a path doesn't resolve on your tree, run `find draft/slint-ui/docs -name <basename>.mdx` and update this list.

---

## Per-phase quick-reference (reading order)

The guide above sequences Phase-8 work by **risk cluster**. If you'd rather sequence by UI phase (e.g. if you're deferring some UI phases for a later wave), here's the same work indexed by phase number — pull the cluster steps that mention each phase and stop at the boundary you need:

| UI phase | Bridge promotions (Cluster step) | Callback promotions (Cluster step) |
|---|---|---|
| 13 | A1 (`status-items`) | — |
| 14 | B1 (`audio-*-idx` / `audio-muted` / `audio-input-gain`) | optional B1 callbacks |
| 15 | B2 (`camera-*`) | optional B2 callbacks |
| 16 | C1 (`presets`, `selected-preset-id`) | C1 (`save-preset`, `delete-preset`, `set-active-preset`) |
| 17 | C2 (`quick-actions` re-use; **fixes B12**) | C2 (`move-bar-action`, `set-bar-action-enabled`, `save-bar-actions`) |
| 18 | (B4) `lifecycle`, `snapshot-secs` | B4 (`engage-lock`, `engage-stealth`, `start-snapshot-countdown`) |
| 19 | F1 (`banner-*`) | D1 (`export-settings`, `import-settings`, `reset-settings`, `clear-cast-history`, `clear-known-receivers`) |
| 20 | D2 (`history`, `selected-history-entry`) | D2 (`clear-history`, `delete-history-entry`, `recast`) |
| 21 | A2 (`app-version`) | — |
| 22 | A3 (`network-interfaces`), B5 (`wifi-aware`) | B5 (`set-wifi-aware`) |
| 23 | A4 (`recording-state`, `recording-elapsed-s`) | B3 (`start-recording`, `pause-recording`, `resume-recording`, `stop-recording`) |
| 25 | C4 (`macros`, `macro-edit-id`) | C4 (`save-macro`, `delete-macro`, `move-step`, `add-step`, `remove-step`, `run-macro`) |
| 26 | A5 (`log-entries`) | C5 (`clear-log-entries`) |
| 27 | F1 (`banner-*`); F3 already done | — |
| Cluster E (overlay invariants) | — | — (docs only) |

This index is the inverse of the migration plan's per-phase table, kept in sync as a cross-reference.

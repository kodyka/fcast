# Phase 8 — Section 3: Cluster B — single-page state with one or two callbacks

> Section 3 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md), [`PHASE-8-Section-1-cluster-F-shared-tokens.md`](./PHASE-8-Section-1-cluster-F-shared-tokens.md), and [`PHASE-8-Section-2-cluster-A-readonly-view-models.md`](./PHASE-8-Section-2-cluster-A-readonly-view-models.md) first.

**Cluster B is "Slint owns the value, Rust observes via callback."** The user toggles a switch, picks a value, drags a slider — Slint mutates locally and emits a callback so Rust can react (start MediaRecorder, change bitrate, etc.). On startup Rust *may* push initial values, but post-startup the canonical store is Slint.

| Item | Slint property/properties | Slint→Rust callbacks | Effort |
|---|---|---|---|
| B1 | Audio: `audio-source-idx`, `audio-muted`, `audio-input-gain`, `audio-bitrate-idx` | optional `audio-settings-changed()` | Small |
| B2 | Camera: `camera-idx`, `resolution-idx`, `framerate-idx`, `mirror-front`, `stabilization`, `tap-to-focus`, `zoom-level` | optional `camera-settings-changed()` | Small |
| B3 | Recording controls (state machine writes) | `start-recording`, `pause-recording`, `resume-recording`, `stop-recording` | Medium |
| B4 | Lifecycle modes + snapshot countdown | `engage-lock`, `engage-stealth`, `start-snapshot-countdown` (+ rename `mock-snapshot-secs` → `snapshot-secs`) | Small |
| B5 | Wi-Fi Aware toggle | `set-wifi-aware(bool)` | Tiny |

**Net new code:** ~50 lines bridge.slint, ~120 lines per consumer page (mostly `mock-` → `Bridge.` renames), ~250 lines lib.rs.

---

## 3.1 — B1 — Audio settings

### What's there today

`pages/audio_page.slint` (Phase 14) holds 4 page-local props:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/audio_page.slint" lines="9-19" />

Mutations stay inside the page (`root.mock-source-idx = Math.mod(...)`). For Phase 8 the values themselves move to Bridge so Rust can read them at cast-start, but the *write path* stays Slint — Slint just calls `Bridge.set_audio_source_idx(...)` semantically (no, that's a Rust-side setter; in Slint we just **assign to an `in-out` property on Bridge**).

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Audio settings (Phase 8 / Cluster B1) ───────────────────────────
+    // Slint mutates these directly; Rust reads at cast-start. If you need
+    // Rust to react live (e.g. swap input source mid-cast), uncomment the
+    // audio-settings-changed callback below and wire it in lib.rs.
+    in-out property <int>   audio-source-idx:  0;     // Mic / System / Both
+    in-out property <bool>  audio-muted:       false;
+    in-out property <float> audio-input-gain:  0.7;   // 0..1
+    in-out property <int>   audio-bitrate-idx: 1;     // 64 / 128 / 192 / 256 kbps
+    // callback audio-settings-changed();
     // …
 }
```

**Why `in-out`:** Slint will write these (toggle, cycle, slider drag); Rust will read them (and *may* write them back if a settings-import flow restores defaults). `in` would make Slint→Slint writes silently no-op. See `guide/language/coding/properties.mdx`.

### Step 2: consumer migration

```diff
 export component AudioPage inherits Rectangle {
-    // UI-only stub state.
-    in-out property <bool>  mock-muted:         false;
-    in-out property <int>   mock-source-idx:    0;
-    in-out property <float> mock-input-gain:    0.7;
-    in-out property <int>   mock-bitrate-idx:   1;
-
     width: 100%;
     height: 100%;
     background: Theme.surface-primary;

     VerticalLayout {
         // … header unchanged …

         ScrollView {
             VerticalLayout {
                 SettingsSection {
                     title: @tr("INPUT");
                     SettingsValueRow {
                         title: @tr("Source");
-                        value: [@tr("Microphone"), @tr("System audio"), @tr("Both")][root.mock-source-idx];
-                        clicked => { root.mock-source-idx = Math.mod(root.mock-source-idx + 1, 3); }
+                        value: [@tr("Microphone"), @tr("System audio"), @tr("Both")][Bridge.audio-source-idx];
+                        clicked => { Bridge.audio-source-idx = Math.mod(Bridge.audio-source-idx + 1, 3); }
                     }
                     SettingsToggleRow {
                         title: @tr("Mute");
-                        checked: root.mock-muted;
-                        toggled(checked) => { root.mock-muted = checked; }
+                        checked: Bridge.audio-muted;
+                        toggled(checked) => { Bridge.audio-muted = checked; }
                     }
                     SettingsSliderRow {
                         title: @tr("Input gain");
-                        value: root.mock-input-gain * 100;
+                        value: Bridge.audio-input-gain * 100;
                         minimum: 0; maximum: 100; unit: "%";
-                        changed(v) => { root.mock-input-gain = v / 100; }
+                        changed(v) => { Bridge.audio-input-gain = v / 100; }
                     }
                 }

                 SettingsSection {
                     title: @tr("ENCODING");
                     SettingsValueRow {
                         title: @tr("Bitrate");
-                        value: [@tr("64 kbps"), @tr("128 kbps"), @tr("192 kbps"), @tr("256 kbps")][root.mock-bitrate-idx];
-                        clicked => { root.mock-bitrate-idx = Math.mod(root.mock-bitrate-idx + 1, 4); }
+                        value: [@tr("64 kbps"), @tr("128 kbps"), @tr("192 kbps"), @tr("256 kbps")][Bridge.audio-bitrate-idx];
+                        clicked => { Bridge.audio-bitrate-idx = Math.mod(Bridge.audio-bitrate-idx + 1, 4); }
                     }
                     // … codec row unchanged …
                 }
             }
         }
     }
 }
```

**Sanity grep:**

```sh
grep -nE 'mock-(muted|source-idx|input-gain|bitrate-idx)' senders/android/ui/pages/audio_page.slint
# Should be 0 lines.
```

### Step 3: Rust reader

Reading is "look at what Slint stored, when needed":

```rust
// senders/android/src/lib.rs — wherever start_casting decides what audio source to capture.

fn read_audio_settings(ui: &MainWindow) -> AudioSettings {
    let bridge = ui.global::<Bridge>();
    AudioSettings {
        source: match bridge.get_audio_source_idx() {
            0 => AudioSource::Microphone,
            1 => AudioSource::SystemAudio,
            _ => AudioSource::Both,
        },
        muted:      bridge.get_audio_muted(),
        input_gain: bridge.get_audio_input_gain(),
        bitrate_kbps: match bridge.get_audio_bitrate_idx() {
            0 => 64, 1 => 128, 2 => 192, _ => 256,
        },
    }
}
```

This is called from the existing `on_start_casting` handler at `lib.rs:1017`. No bridge-level callback is required — Rust pulls the values when it needs them.

If you **do** want a callback (e.g. live mute toggle during cast):

```diff
 // bridge.slint
+    callback audio-settings-changed();
```

```diff
 // audio_page.slint
     SettingsToggleRow {
         title: @tr("Mute");
         checked: Bridge.audio-muted;
-        toggled(checked) => { Bridge.audio-muted = checked; }
+        toggled(checked) => {
+            Bridge.audio-muted = checked;
+            Bridge.audio-settings-changed();
+        }
     }
```

```rust
// lib.rs
ui.global::<Bridge>().on_audio_settings_changed({
    let ui_handle = ui.as_weak();
    move || {
        let Some(ui) = ui_handle.upgrade() else { return; };
        let settings = read_audio_settings(&ui);
        // Apply to a running cast pipeline if one exists. Otherwise no-op.
        // Phase 11 will plumb this into mediarecorder.
    }
});
```

**Slint doc citations for B1:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx`

---

## 3.2 — B2 — Camera settings

### What's there today

`pages/camera_page.slint` (Phase 15):

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/camera_page.slint" lines="55-61" />

Same shape as B1, more properties (7 instead of 4).

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Camera settings (Phase 8 / Cluster B2) ──────────────────────────
+    in-out property <int>   camera-idx:           1;    // 0 Front / 1 Back / 2 External
+    in-out property <int>   resolution-idx:       2;    // 0 480p .. 3 4K
+    in-out property <int>   framerate-idx:        1;    // 0 24 / 1 30 / 2 60 fps
+    in-out property <bool>  camera-mirror-front:  true;
+    in-out property <bool>  camera-stabilization: true;
+    in-out property <bool>  camera-tap-to-focus:  true;
+    in-out property <float> camera-zoom-level:    1.0;  // 0.5 .. 5.0
     // …
 }
```

### Step 2: consumer migration

The diff is mechanical — every `root.mock-camera-idx` becomes `Bridge.camera-idx`, every `root.mock-resolution-idx` becomes `Bridge.resolution-idx`, etc. The internal `PresetChip` component stays untouched. Sanity grep:

```sh
grep -nE 'mock-(camera-idx|resolution-idx|framerate-idx|mirror-front|stabilization|tap-to-focus|zoom-level)' \
    senders/android/ui/pages/camera_page.slint
# Should be 0 lines after migration.
```

### Step 3: Rust reader

Same shape as B1 — `read_camera_settings(ui: &MainWindow)` reads `Bridge.*` at cast-start. No callback unless a live setting needs to apply mid-cast (most don't — camera switch is at-cast-start only on Android).

**Slint doc citations for B2:**

- (same as B1)
- `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx` — `Math.mod` cycler.

---

## 3.3 — B3 — Recording controls (state machine writes)

### What's there today

`pages/recording_page.slint` (Phase 23) drives the state machine entirely Slint-side via two helper functions:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/recording_page.slint" lines="64-86" />

For Phase 8, `Bridge.recording-state` is **published from Rust** (Cluster A4 already did the read side). The write side becomes 4 callbacks: `start-recording`, `pause-recording`, `resume-recording`, `stop-recording`. Slint emits; Rust authoritatively decides the next state and pushes it back via `set_recording_state`.

### Step 1: extend `bridge.slint`

A4 already added `recording-state` and `recording-elapsed-s`. Now add the callbacks:

```diff
 export global Bridge {
     // …
     in property <RecordingState> recording-state:    RecordingState.idle;
     in property <int>            recording-elapsed-s: 0;
+    callback start-recording();
+    callback pause-recording();
+    callback resume-recording();
+    callback stop-recording();
     // …
 }
```

### Step 2: consumer migration

```diff
 export component RecordingPage inherits Rectangle {
-    in-out property <RecordingState> mock-state:        RecordingState.idle;
-    in-out property <int>             mock-elapsed-s:    0;
+    // recording-state and recording-elapsed-s now read from Bridge (A4).

     // …

     // ── 1-second tick driving the elapsed counter ────────────────────────
-    Timer {
-        interval: 1s;
-        running: root.mock-state == RecordingState.recording;
-        triggered => { root.mock-elapsed-s += 1; }
-    }
+    // Removed: Rust drives Bridge.recording-elapsed-s via a tokio interval.

     // …

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
+        if (Bridge.recording-state == RecordingState.idle) {
+            Bridge.start-recording();
+        } else if (Bridge.recording-state == RecordingState.recording) {
+            Bridge.pause-recording();
+        } else if (Bridge.recording-state == RecordingState.paused) {
+            Bridge.resume-recording();
+        }
+    }
+
+    function on-stop-clicked() {
+        if (Bridge.recording-state == RecordingState.recording
+         || Bridge.recording-state == RecordingState.paused) {
+            Bridge.stop-recording();
+        }
+    }
 }
```

**Why we don't synchronously flip state:**

- The recording state machine is owned by the Rust producer (because real recording involves `MediaRecorder` lifecycle: start → pause → resume → stop → finalize). Slint asking Rust "please start" and waiting for Rust to push back `RecordingState::Recording` is the only way to guarantee the UI reflects what the recorder actually did.
- If `start-recording` fails (no permission, disk full), Rust pushes a banner via `flash_banner` (Cluster F) and **leaves recording-state at `idle`**. Slint will not show the recording button as red because `Bridge.recording-state` never changed.
- See Section 8 / R5 for the related "panel race" pitfall.

### Step 3: Rust handlers

```rust
// senders/android/src/lib.rs

let recorder_state = Arc::new(Mutex::new(RecordingTickerState::default()));
let ui_handle = ui.as_weak();

ui.global::<Bridge>().on_start_recording({
    let recorder_state = recorder_state.clone();
    let ui_handle = ui_handle.clone();
    move || {
        let recorder_state = recorder_state.clone();
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            // Phase 11 will: jni::start_media_recorder().await
            let mut s = recorder_state.lock().await;
            s.started_at = Some(std::time::Instant::now());
            s.paused_for = std::time::Duration::ZERO;
            s.pause_started = None;
            s.state = RecordingState::Recording;
            // Push state immediately. The ticker (A4) will push elapsed.
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_recording_state(RecordingState::Recording);
                ui.global::<Bridge>().set_recording_elapsed_s(0);
            });
        });
    }
});

ui.global::<Bridge>().on_pause_recording({
    let recorder_state = recorder_state.clone();
    let ui_handle = ui_handle.clone();
    move || {
        let recorder_state = recorder_state.clone();
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            let mut s = recorder_state.lock().await;
            if s.state != RecordingState::Recording { return; }
            s.pause_started = Some(std::time::Instant::now());
            s.state = RecordingState::Paused;
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_recording_state(RecordingState::Paused);
            });
        });
    }
});

ui.global::<Bridge>().on_resume_recording({
    let recorder_state = recorder_state.clone();
    let ui_handle = ui_handle.clone();
    move || {
        let recorder_state = recorder_state.clone();
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            let mut s = recorder_state.lock().await;
            if s.state != RecordingState::Paused { return; }
            if let Some(started) = s.pause_started.take() {
                s.paused_for += started.elapsed();
            }
            s.state = RecordingState::Recording;
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_recording_state(RecordingState::Recording);
            });
        });
    }
});

ui.global::<Bridge>().on_stop_recording({
    let recorder_state = recorder_state.clone();
    let ui_handle = ui_handle.clone();
    move || {
        let recorder_state = recorder_state.clone();
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            // First push: finalizing (so UI shows the spinner placeholder).
            {
                let mut s = recorder_state.lock().await;
                s.state = RecordingState::Finalizing;
            }
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                ui.global::<Bridge>().set_recording_state(RecordingState::Finalizing);
            });

            // Phase 11: jni::stop_media_recorder_and_finalize().await ...

            // Then idle:
            let mut s = recorder_state.lock().await;
            s.started_at = None;
            s.paused_for = std::time::Duration::ZERO;
            s.pause_started = None;
            s.state = RecordingState::Idle;
            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                let bridge = ui.global::<Bridge>();
                bridge.set_recording_state(RecordingState::Idle);
                bridge.set_recording_elapsed_s(0);
            });
        });
    }
});

// Pass `recorder_state` into spawn_recording_ticker (A4) — it needs to read
// the same Mutex<RecordingTickerState> the handlers above mutate.
spawn_recording_ticker(ui.as_weak(), recorder_state.clone());
```

**Cross-reference:** `recorder_state` declared here is the same Arc<Mutex> consumed by `spawn_recording_ticker` from Cluster A4. Make sure A4's stub `Default::default()` gets replaced by this real handle when you do B3 — otherwise the ticker reads from a different lock than the handlers mutate, and elapsed seconds won't update.

**Slint doc citations for B3:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx` — multi-branch if-else inside a Slint `function`.

---

## 3.4 — B4 — Lifecycle modes + snapshot countdown

### What's there today

`bridge.slint` already exports `LifecycleMode` (`normal`, `lock-screen`, `stealth`, `snapshot-countdown`) and `Bridge.lifecycle: LifecycleMode`. There's also `Bridge.mock-snapshot-secs: int` which is the misnamed countdown duration. `pages/settings_page.slint` writes to `Bridge.lifecycle` directly when the user taps "Lock screen" / "Stealth" / "Cast with countdown".

For Phase 8 we want Rust to **also** be able to engage these modes (e.g. an Android Auto request, an external command). So we promote the Slint→Rust transitions to callbacks:

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
     in-out property <LifecycleMode> lifecycle: LifecycleMode.normal;
-    in-out property <int> mock-snapshot-secs: 5;
+    in-out property <int>           snapshot-secs: 5;     // renamed in Phase 8
+
+    // Phase 8 / B4 — callbacks instead of Slint-side direct writes to
+    // Bridge.lifecycle. (Both paths still allowed during transition; see
+    // Section 8 / R5.)
+    callback engage-lock();
+    callback engage-stealth();
+    callback start-snapshot-countdown(int);   // duration in seconds
+    callback exit-lifecycle();                // back to LifecycleMode.normal
     // …
 }
```

**Renaming `mock-snapshot-secs` to `snapshot-secs`:** it's been a `Bridge` property all along; the `mock-` prefix was misleading. Search-and-replace once across the tree, no semantic change:

```sh
grep -rn 'mock-snapshot-secs' senders/android/ui/
# Replace each occurrence with snapshot-secs.
```

### Step 2: consumer migration

```diff
 // settings_page.slint (or wherever the lifecycle rows live)
 SettingsValueRow {
     title: @tr("Lock screen");
-    clicked => { Bridge.lifecycle = LifecycleMode.lock-screen; }
+    clicked => { Bridge.engage-lock(); }
 }
 SettingsValueRow {
     title: @tr("Stealth mode");
-    clicked => { Bridge.lifecycle = LifecycleMode.stealth; }
+    clicked => { Bridge.engage-stealth(); }
 }
 SettingsValueRow {
     title: @tr("Cast with countdown");
-    value: @tr("{}s", Bridge.mock-snapshot-secs);
-    clicked => { Bridge.lifecycle = LifecycleMode.snapshot-countdown; }
+    value: @tr("{}s", Bridge.snapshot-secs);
+    clicked => { Bridge.start-snapshot-countdown(Bridge.snapshot-secs); }
 }
```

The lock / stealth / countdown overlays in `main.slint` continue to render `if Bridge.lifecycle == LifecycleMode.lock-screen: LockOverlay { … }`. Slint reads `Bridge.lifecycle` exactly as before; Rust is now the one who writes it.

### Step 3: Rust handlers

```rust
ui.global::<Bridge>().on_engage_lock({
    let ui_handle = ui.as_weak();
    move || {
        let _ = ui_handle.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_lifecycle(LifecycleMode::LockScreen);
        });
        // Phase 11: also tell Android to disable system gestures.
    }
});

ui.global::<Bridge>().on_engage_stealth({
    let ui_handle = ui.as_weak();
    move || {
        let _ = ui_handle.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_lifecycle(LifecycleMode::Stealth);
        });
    }
});

ui.global::<Bridge>().on_start_snapshot_countdown({
    let ui_handle = ui.as_weak();
    move |seconds: i32| {
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            // Show the countdown overlay.
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>()
                    .set_lifecycle(LifecycleMode::SnapshotCountdown);
            });
            tokio::time::sleep(std::time::Duration::from_secs(seconds.max(0) as u64)).await;
            // After countdown, exit overlay and start cast.
            let _ = ui_handle.upgrade_in_event_loop(|ui| {
                ui.global::<Bridge>().set_lifecycle(LifecycleMode::Normal);
                // Phase 11: trigger start_casting here.
            });
        });
    }
});

ui.global::<Bridge>().on_exit_lifecycle({
    let ui_handle = ui.as_weak();
    move || {
        let _ = ui_handle.upgrade_in_event_loop(|ui| {
            ui.global::<Bridge>().set_lifecycle(LifecycleMode::Normal);
        });
    }
});
```

**Why both writers (Slint and Rust) is OK:**

- Each path goes through a single chokepoint. Slint writes via callback (which is a function call into Rust); Rust writes via `set_lifecycle`. Neither sees stale state because both update `Bridge.lifecycle` synchronously on the UI thread.
- This is **safe specifically because** the mutation is atomic (one property, one type). For multi-property state machines (like recording) we use the read-only `in property` pattern instead — see Section 8 / R5.

**Slint doc citations for B4:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — `LifecycleMode` enum.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

---

## 3.5 — B5 — Wi-Fi Aware toggle

### What's there today

`pages/network_page.slint` has `mock-wifi-aware-enabled: bool` and a 3-second auto-hide banner that fires when the toggle flips on:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/network_page.slint" lines="143-156" />

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Wi-Fi Aware (Phase 8 / Cluster B5) ──────────────────────────────
+    in-out property <bool> wifi-aware-enabled: false;
+    callback set-wifi-aware(bool);
     // …
 }
```

### Step 2: consumer migration

```diff
 // network_page.slint
 export component NetworkPage inherits Rectangle {
     // … network-interfaces moves to A3 …
-    in-out property <bool> mock-wifi-aware-enabled: false;
-    property <bool>        banner-visible:          false;

-    // Auto-hide banner timer. Re-arms whenever banner-visible flips to true.
-    Timer {
-        interval: 3s;
-        running: root.banner-visible;
-        triggered => { root.banner-visible = false; }
-    }

     // … inside the Wi-Fi Aware section …
     SettingsToggleRow {
         title: @tr("Wi-Fi Aware");
-        checked: root.mock-wifi-aware-enabled;
-        toggled(checked) => {
-            root.mock-wifi-aware-enabled = checked;
-            root.banner-visible = checked;
-        }
+        checked: Bridge.wifi-aware-enabled;
+        toggled(checked) => { Bridge.set-wifi-aware(checked); }
     }
 }
```

The auto-hide banner is now driven from `Bridge.banner-*` (Cluster F1), so the page-local `banner-visible` and `Timer` go away too.

### Step 3: Rust handler

```rust
ui.global::<Bridge>().on_set_wifi_aware({
    let ui_handle = ui.as_weak();
    move |enabled| {
        let ui_handle = ui_handle.clone();
        tokio::spawn(async move {
            // Phase 11: actually enable WifiAwareManager.
            let success = true;  // placeholder.

            let _ = ui_handle.upgrade_in_event_loop(move |ui| {
                let bridge = ui.global::<Bridge>();
                bridge.set_wifi_aware_enabled(enabled && success);
            });

            // Flash banner via the helper from Cluster F.
            flash_banner(
                ui_handle,
                if enabled {
                    "Wi-Fi Aware enabled (placeholder — no permission requested).".into()
                } else {
                    "Wi-Fi Aware disabled.".into()
                },
                BannerSeverity::Info,
                std::time::Duration::from_secs(3),
            );
        });
    }
});
```

**Why we don't optimistically flip:**

- If Wi-Fi Aware permission is denied (or the device doesn't support it), Rust gets `success = false` and calls `set_wifi_aware_enabled(false)`. The toggle springs back to "Off". The banner copy explains why.
- Optimistic UI ("flip immediately, revert on failure") works for fast operations. WifiAwareManager is async with a permission prompt — could take 5+ seconds. Pessimistic UI is correct here.

**Slint doc citations for B5:**

- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx` — Slint timer is removed in favour of Rust-driven auto-hide via Cluster F's `flash_banner`.

---

## 3.6 Cluster B verification

```sh
# 1. mock-* count drops by ~17 (4 audio + 7 camera + 4 recording + 1 wifi + 1 snapshot)
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l

# 2. Bridge declarations
grep -nE '(audio-(source-idx|muted|input-gain|bitrate-idx)|camera-(idx|mirror-front|stabilization|tap-to-focus|zoom-level)|resolution-idx|framerate-idx|wifi-aware-enabled|snapshot-secs|engage-lock|engage-stealth|start-snapshot-countdown|set-wifi-aware|start-recording|pause-recording|resume-recording|stop-recording)' \
    senders/android/ui/bridge.slint

# 3. Old mock-snapshot-secs is gone everywhere
grep -rn 'mock-snapshot-secs' senders/android/ui/
# Should be empty.

# 4. Build green
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

---

## 3.7 Commit messages

```
feat(android): Phase 8 / B1 — audio settings via Bridge.audio-*
feat(android): Phase 8 / B2 — camera settings via Bridge.camera-*/resolution-*/framerate-*
feat(android): Phase 8 / B3 — recording state-machine callbacks
feat(android): Phase 8 / B4 — lifecycle callbacks + rename mock-snapshot-secs → snapshot-secs
feat(android): Phase 8 / B5 — Wi-Fi Aware toggle through Bridge
```

---

## 3.8 Exit criteria for Section 3

- [x] All 5 items wired (B1-B5)
- [x] No `Slint Timer` driving banner auto-hide on `network_page.slint`
- [x] `Bridge.snapshot-secs` (no longer `Bridge.mock-snapshot-secs`)
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green
- [x] mock-* inventory dropped by ~17 lines

You can now move to **Section 4 — Cluster C: list mutations** at [`PHASE-8-Section-4-cluster-C-list-mutations.md`](./PHASE-8-Section-4-cluster-C-list-mutations.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/slider.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/std-widgets/basic-widgets/checkbox.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/global-functions/math.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/reference/timer.mdx`

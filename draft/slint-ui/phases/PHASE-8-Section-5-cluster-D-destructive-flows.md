# Phase 8 — Section 5: Cluster D — destructive flows

> Section 5 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-4-cluster-C-list-mutations.md`](./PHASE-8-Section-4-cluster-C-list-mutations.md) first.

**Cluster D is the high-stakes group.** Anything that deletes user data needs the Slint→Rust round-trip so the real implementation can show OS-level confirmation, perform the destructive work, and surface a banner on completion via Cluster F's `flash_banner`.

| Item | Slint property/properties | Mutators (callbacks) | Effort |
|---|---|---|---|
| D1 | (no list — page-local `pending-action`) | `export-settings`, `import-settings`, `reset-settings`, `clear-cast-history`, `clear-known-receivers` | Medium |
| D2 | `Bridge.history: [CastHistoryEntry]`, `Bridge.selected-history-entry: CastHistoryEntry`, `Bridge.selected-history-id: string` | `clear-history`, `delete-history-entry`, `recast` | Medium |

D3 (ADVANCED reset) is **skipped** — there is no destructive ADVANCED row to migrate as of master 2026-05-10.

**Net new code:** ~50 lines bridge.slint, ~80 lines per consumer page (mostly removing local mock data), ~250 lines lib.rs.

**Prerequisite:** Cluster F's `Bridge.banner-*` and `flash_banner` helper must already exist (see [`PHASE-8-Section-1-cluster-F-shared-tokens.md`](./PHASE-8-Section-1-cluster-F-shared-tokens.md)).

---

## 5.1 — D1 — Backup / reset

### What's there today

`pages/backup_reset_page.slint` (Phase 19) holds a `pending-action: string` and an inline `ConfirmDialog`. When the user taps a row (Export / Import / Reset / Clear history / Clear receivers), the page sets `pending-action` and shows the dialog. On confirm, it does *nothing* — the body is a placeholder comment, because Phase 19 was UI-only.

### What we want

Keep the dialog flow exactly as Phase 19 ships it. The only change is the **body** of the on-confirm dispatcher: each branch calls a Rust callback instead of sitting empty.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Backup / reset (Phase 8 / Cluster D1) ───────────────────────────
+    callback export-settings();
+    callback import-settings();
+    callback reset-settings();
+    callback clear-cast-history();
+    callback clear-known-receivers();
     // …
 }
```

**Why these 5 callbacks instead of one omnibus `do-destructive-action(string)`:**

- Type-safe call sites — Slint can't typo a callback name. A string-keyed dispatcher would silently no-op on typos.
- Each handler in lib.rs has different platform plumbing: Export = `ACTION_CREATE_DOCUMENT`; Import = `ACTION_OPEN_DOCUMENT`; Reset = clear DataStore; Clear history = wipe `history` Mutex; Clear receivers = wipe `known-receivers` (Phase 24).
- Distinct callbacks make the success-banner severity per-action obvious (Export = info, Reset = success, etc.).

### Step 2: consumer migration

Inside `backup_reset_page.slint`, locate the `on-confirm()` function (or similar dispatcher tied to the ConfirmDialog's `confirmed` callback):

```diff
 function on-confirm() {
     if (root.pending-action == "export") {
-        // UI-only: would trigger Bridge.export-settings() in Phase 8.
+        Bridge.export-settings();
     } else if (root.pending-action == "import") {
-        // UI-only: would trigger Bridge.import-settings() in Phase 8.
+        Bridge.import-settings();
     } else if (root.pending-action == "reset") {
-        // UI-only: would trigger Bridge.reset-settings() in Phase 8.
+        Bridge.reset-settings();
     } else if (root.pending-action == "clear-history") {
-        // UI-only: would trigger Bridge.clear-cast-history() in Phase 8.
+        Bridge.clear-cast-history();
     } else if (root.pending-action == "clear-receivers") {
-        // UI-only: would trigger Bridge.clear-known-receivers() in Phase 8.
+        Bridge.clear-known-receivers();
     }
     root.pending-action = "";
     root.confirm-visible = false;
 }
```

**Stays Slint-side:**

- `pending-action: string` — page-local UI state (which row was tapped).
- `confirm-visible: bool` — page-local visibility flag for `ConfirmDialog`.
- The pure functions `confirm-title(action)` / `confirm-body(action)` / `confirm-label(action)` — they are presentation, not platform.

**Why we keep the page-local pending-action:**

- The dialog is transactional. The user can dismiss it ("Cancel") and Rust never knows. Pushing every "I tapped reset" to Rust would generate noise.
- Round-tripping through Rust to display a dialog adds ~16 ms of latency for no benefit.

### Step 3: Rust handlers

```rust
// senders/android/src/lib.rs

// Each handler does the destructive work then flashes the banner via
// Cluster F's helper.

ui.global::<Bridge>().on_export_settings({
    let ui_weak = ui.as_weak();
    move || {
        let ui_weak = ui_weak.clone();
        tokio::spawn(async move {
            // Phase 11: launch ACTION_CREATE_DOCUMENT via JNI; await user
            // file pick; serialise settings; write to URI.
            //
            // For Phase 8 stub: pretend the action takes 500 ms then succeeds.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            flash_banner(
                ui_weak,
                "Settings exported (placeholder).".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(3),
            );
        });
    }
});

ui.global::<Bridge>().on_import_settings({
    let ui_weak = ui.as_weak();
    move || {
        let ui_weak = ui_weak.clone();
        tokio::spawn(async move {
            // Phase 11: launch ACTION_OPEN_DOCUMENT, parse JSON, write to DataStore.
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            flash_banner(
                ui_weak,
                "Settings imported (placeholder).".into(),
                BannerSeverity::Success,
                std::time::Duration::from_secs(3),
            );
        });
    }
});

ui.global::<Bridge>().on_reset_settings({
    let presets         = presets.clone();              // from C1
    let bar_actions     = bar_actions.clone();          // from C2
    let macros          = macros.clone();               // from C4
    let history         = history.clone();              // from D2 below
    let push_presets    = push_presets.clone();
    let push_bar        = push_bar.clone();
    let push_macros     = push_macros.clone();
    let push_history    = push_history.clone();
    let ui_weak         = ui.as_weak();
    move || {
        // Reset every Cluster-C/D model to factory defaults. Mutate the
        // Arc<Mutex<…>> guards we share with the C handlers; then push.
        *presets.lock().unwrap() = vec![
            BitratePreset { id: "low".into(),  name: "Low".into(),     bitrate_kbps: 1500,  active: false },
            BitratePreset { id: "med".into(),  name: "Medium".into(),  bitrate_kbps: 4000,  active: true  },
            BitratePreset { id: "high".into(), name: "High".into(),    bitrate_kbps: 8000,  active: false },
            BitratePreset { id: "max".into(),  name: "Maximum".into(), bitrate_kbps: 15000, active: false },
        ];
        *macros.lock().unwrap() = vec![];
        *history.lock().unwrap() = vec![];
        // bar_actions = the same default action list rebuilt the same way as init.
        // Avoid duplicating the literal — extract a `default_quick_actions()` helper.
        *bar_actions.lock().unwrap() = default_quick_actions();

        push_presets();
        push_bar();
        push_macros();
        push_history();

        // Phase 11: also clear DataStore / SharedPreferences via JNI.

        flash_banner(
            ui_weak,
            "Settings reset to defaults".into(),
            BannerSeverity::Success,
            std::time::Duration::from_secs(3),
        );
    }
});

ui.global::<Bridge>().on_clear_cast_history({
    let history      = history.clone();
    let push_history = push_history.clone();
    let ui_weak      = ui.as_weak();
    move || {
        history.lock().unwrap().clear();
        push_history();

        flash_banner(
            ui_weak,
            "Cast history cleared".into(),
            BannerSeverity::Success,
            std::time::Duration::from_secs(2),
        );
    }
});

ui.global::<Bridge>().on_clear_known_receivers({
    let ui_weak = ui.as_weak();
    move || {
        // Phase 11: clear known-receivers DataStore. For now, just announce.
        flash_banner(
            ui_weak,
            "Known receivers cleared".into(),
            BannerSeverity::Success,
            std::time::Duration::from_secs(2),
        );
    }
});

fn default_quick_actions() -> Vec<QuickAction> {
    let mut actions = vec![
        QuickAction { id: "settings".into(),   title: "Settings".into(),    enabled: true,  active: false, is_macro: false },
        QuickAction { id: "debug".into(),      title: "Debug".into(),       enabled: true,  active: false, is_macro: false },
        QuickAction { id: "codec-test".into(), title: "Codec test".into(),  enabled: true,  active: false, is_macro: false },
        QuickAction { id: "scan-qr".into(),    title: "Scan QR".into(),     enabled: true,  active: false, is_macro: false },
        QuickAction { id: "record".into(),     title: "Record".into(),      enabled: true,  active: false, is_macro: false },
        QuickAction { id: "pair".into(),       title: "Pair".into(),        enabled: true,  active: false, is_macro: false },
        QuickAction { id: "bitrate".into(),    title: "Bitrate".into(),     enabled: true,  active: false, is_macro: false },
    ];
    if cfg!(debug_assertions) {
        actions.extend([
            QuickAction { id: "migrated-server".into(), title: "Migrated srv".into(), enabled: true, active: false, is_macro: false },
            QuickAction { id: "test-getinfo".into(),    title: "GetInfo".into(),      enabled: true, active: false, is_macro: false },
            QuickAction { id: "test-crossfade".into(),  title: "Crossfade".into(),    enabled: true, active: false, is_macro: false },
            QuickAction { id: "test-smoke".into(),      title: "Smoke Graph".into(),  enabled: true, active: false, is_macro: false },
        ]);
    }
    actions
}
```

**Why we factor out `default_quick_actions()`:**

- The init code (Cluster C2) and the reset handler use the same literal. Without the helper, you'd have two copies that drift. With the helper, drift is impossible.
- Same applies to `default_presets()` if you want to factor that too.

**Slint doc citations for D1:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx` — the if/else chain in `on-confirm()`.

---

## 5.2 — D2 — Cast history

### What's there today

`pages/cast_history_page.slint` (Phase 20) initialises 5 hardcoded entries inline; `cast_history_detail_page.slint` does a Slint-side linear search via a `find-entry(id)` helper.

### What we want

`Bridge.history: [CastHistoryEntry]` driven from Rust + a `selected-history-entry` snapshot pushed when the user opens a row.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Cast history (Phase 8 / Cluster D2) ─────────────────────────────
+    in property <[CastHistoryEntry]> history: [];
+    // selected-history-id already exists (Phase 20). Add a derived snapshot
+    // so the detail page doesn't need to search the list every render.
+    in property <CastHistoryEntry> selected-history-entry;
+    // Slint-side change observer — Slint does NOT auto-generate
+    // on_<prop>_changed callbacks. We declare an explicit callback and
+    // wire it from a `changed` handler below. See Step 4 for the Rust
+    // binding and Section 8 / pitfall 8.14 for the rationale.
+    callback selected-history-id-changed(string);
+    changed selected-history-id => {
+        Bridge.selected-history-id-changed(Bridge.selected-history-id);
+    }
+    callback clear-history();
+    callback delete-history-entry(string);
+    callback recast(string);                          // (entry-id)
     // …
 }
```

`selected-history-id` keeps its existing `in-out` direction since Slint writes it when the user taps a row.

### Step 2: consumer migration — `cast_history_page.slint`

```diff
 export component CastHistoryPage inherits Rectangle {
-    in-out property <[CastHistoryEntry]> mock-history: [
-        // … 5 hardcoded entries …
-    ];

     // …

-    for entry in root.mock-history: Rectangle {
+    for entry in Bridge.history: Rectangle {
         // … row layout …
         TouchArea {
             clicked => {
                 Bridge.selected-history-id = entry.id;
                 Bridge.active-panel = Panel.cast-history-detail;
             }
         }
     }

     // ── Toolbar with Clear-all ──────────────────────────────────────────
     // ConfirmDialog imported from Phase 19 (D1 sibling). Stays Slint-side.
     ConfirmDialog {
         visible: root.confirm-visible;
         title: @tr("Clear cast history?");
         body:  @tr("This will remove all entries. Cannot be undone.");
         confirm-label: @tr("Clear");
-        confirmed => {
-            // UI-only: Phase 8 will hook Bridge.clear-history.
-            root.confirm-visible = false;
-        }
+        confirmed => {
+            Bridge.clear-history();
+            root.confirm-visible = false;
+        }
         dismissed => { root.confirm-visible = false; }
     }
 }
```

### Step 3: consumer migration — `cast_history_detail_page.slint`

```diff
 export component CastHistoryDetailPage inherits Rectangle {
-    pure function find-entry(id: string) -> CastHistoryEntry {
-        for e in root.mock-history { if e.id == id { return e; } }
-        return { id: "", receiver-name: "", started-at: "", duration: "",
-                 status: "", failure-reason: "", bytes-sent: 0 };
-    }
-
-    in-out property <[CastHistoryEntry]> mock-history: [
-        // … duplicated from cast_history_page.slint, the gotcha-41 trap …
-    ];
-
-    property <CastHistoryEntry> entry: find-entry(Bridge.selected-history-id);
+    // Rust pushes Bridge.selected-history-entry whenever Bridge.selected-history-id
+    // changes (see lib.rs handler below). No more duplicated mock-history.
+    property <CastHistoryEntry> entry: Bridge.selected-history-entry;

     // … render fields from `root.entry` …

     // Recast button
     PrimaryButton {
         label: @tr("Recast");
-        clicked => {
-            // UI-only: Phase 8 will hook Bridge.recast.
-        }
+        clicked => { Bridge.recast(root.entry.id); }
     }

     // Delete button
     TextButton {
         label: @tr("Delete entry");
-        clicked => {
-            // UI-only: would call Bridge.delete-history-entry in Phase 8.
-        }
+        clicked => {
+            Bridge.delete-history-entry(root.entry.id);
+            Bridge.active-panel = Panel.cast-history;   // route back
+        }
     }
 }
```

**Important:** the Phase-20 detail page had its own copy of `mock-history` because Slint doesn't share state across page-local props. Cluster A in this section eliminates that duplicate (gotcha 41 in Phase 20's reimplement guide). After D2 lands, deleting one entry on the list page no longer leaves a stale row in the detail view.

### Step 4: Rust producer + handlers

```rust
// senders/android/src/lib.rs

let history: Arc<Mutex<Vec<CastHistoryEntry>>> = Arc::new(Mutex::new(vec![
    // Phase 8 bring-up: 3 placeholder entries for visual continuity.
    CastHistoryEntry {
        id: "h1".into(), receiver_name: "Living Room TV".into(),
        started_at: "Today 12:34".into(), duration: "00:12:45".into(),
        status: "completed".into(), failure_reason: "".into(),
        bytes_sent: 320_000_000,
    },
    CastHistoryEntry {
        id: "h2".into(), receiver_name: "Bedroom TV".into(),
        started_at: "Yesterday 22:10".into(), duration: "00:01:08".into(),
        status: "interrupted".into(), failure_reason: "Wi-Fi dropped".into(),
        bytes_sent: 21_500_000,
    },
    CastHistoryEntry {
        id: "h3".into(), receiver_name: "Office Mac".into(),
        started_at: "Yesterday 09:00".into(), duration: "00:32:00".into(),
        status: "completed".into(), failure_reason: "".into(),
        bytes_sent: 880_000_000,
    },
]));

let push_history = {
    let history = history.clone();
    let ui_weak = ui.as_weak();
    move || {
        let snap = history.lock().unwrap().clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_history(
                std::rc::Rc::new(slint::VecModel::from(snap)).into(),
            );
        });
    }
};
push_history();

ui.global::<Bridge>().on_clear_history({
    let history      = history.clone();
    let push_history = push_history.clone();
    move || {
        history.lock().unwrap().clear();
        push_history();
    }
});

ui.global::<Bridge>().on_delete_history_entry({
    let history      = history.clone();
    let push_history = push_history.clone();
    move |id| {
        history.lock().unwrap().retain(|e| e.id != id);
        push_history();
    }
});

ui.global::<Bridge>().on_recast({
    let ui_weak = ui.as_weak();
    let history = history.clone();
    move |id| {
        let entry_opt = history.lock().unwrap().iter()
            .find(|e| e.id == id).cloned();
        let Some(entry) = entry_opt else { return; };
        // Phase 11: trigger reconnection + start_casting with the same receiver.
        flash_banner(
            ui_weak.clone(),
            format!("Recasting to {}", entry.receiver_name).into(),
            BannerSeverity::Info,
            std::time::Duration::from_secs(2),
        );
    }
});

// Push selected-history-entry whenever the id changes.
//
// IMPORTANT: Slint does NOT auto-generate `on_<property>_changed` callbacks
// for `in-out` global properties. We declared an explicit callback
// `selected-history-id-changed(string)` plus a `changed` handler in
// bridge.slint (see Step 1) that re-emits it whenever Slint writes the
// property. The Rust binding below is for that explicit callback, not for
// any synthetic "property changed" hook.

ui.global::<Bridge>().on_selected_history_id_changed({
    let history = history.clone();
    let ui_weak = ui.as_weak();
    move |id: slint::SharedString| {
        let id = id.to_string();
        let entry = history.lock().unwrap().iter()
            .find(|e| e.id == id).cloned();
        let Some(entry) = entry else { return; };
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_selected_history_entry(entry);
        });
    }
});
```

**A simpler alternative for the selected-history-entry plumbing:**

- Skip `Bridge.selected-history-entry` entirely. Have the detail page do its own lookup over `Bridge.history` using the same `find-entry(id)` Slint helper as before, but with `Bridge.history` (not `mock-history`). This trades a tiny bit of Slint complexity for one less Rust handler.
- Pick whichever shape minimises diff.

**Slint doc citations for D2:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`

---

## 5.3 — D3 — ADVANCED reset (skip)

There is no destructive ADVANCED row on master. Skip — nothing to migrate. If a future phase introduces "Forget all paired devices" or "Wipe debug logs from disk", treat it as a new D-cluster item and copy the D1 pattern.

---

## 5.4 Cluster D verification

```sh
# 1. mock-* count drops by ~6 (5 cast history + 1 from the duplicate detail-page copy)
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l

# 2. Bridge declarations
grep -nE '(history|selected-history-entry|export-settings|import-settings|reset-settings|clear-cast-history|clear-known-receivers|clear-history|delete-history-entry|recast)' \
    senders/android/ui/bridge.slint

# 3. Old mock-history is gone
grep -rn 'mock-history' senders/android/ui/
# Should be empty.

# 4. Build green
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

**On-device check (the most important check for Cluster D):**

```
1. Open Settings → Backup & reset → tap Reset → confirm.
   - Banner appears. Severity = success.
   - Open Bitrate presets → see only 4 default presets (any custom one was deleted).
2. Open Cast history → tap Clear all → confirm.
   - Banner appears.
   - List is empty.
3. Open Cast history → tap a row → tap Delete entry.
   - List loses that row. Detail page routes back.
   - No stale row visible from a previous "Open cast history" trip.
```

---

## 5.5 Commit messages

```
feat(android): Phase 8 / D1 — backup/reset callbacks (5 actions, banner via Cluster F)
feat(android): Phase 8 / D2 — cast history via Bridge.history + clear/delete/recast callbacks
```

---

## 5.6 Exit criteria for Section 5

- [x] All 2 items wired (D1, D2)
- [x] **Reset → confirm visually verified on-device** (banner + presets back to defaults)
- [x] No `mock-history` anywhere in `senders/android/ui/`
- [x] No duplicate `mock-history` in `cast_history_detail_page.slint` (gotcha 41 fixed)
- [x] `default_quick_actions()` (and any sibling `default_*()` helpers) used by both init and reset
- [x] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green
- [x] mock-* inventory dropped by ~6 lines

You can now move to **Section 6 — Cluster E: overlay invariants** at [`PHASE-8-Section-6-cluster-E-overlay-invariants.md`](./PHASE-8-Section-6-cluster-E-overlay-invariants.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`

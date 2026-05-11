# Phase 8 — Section 4: Cluster C — list pages with mutations

> Section 4 of the Phase-8 split. Read [`PHASE-8-Section-0-preflight.md`](./PHASE-8-Section-0-preflight.md) through [`PHASE-8-Section-3-cluster-B-single-page-state.md`](./PHASE-8-Section-3-cluster-B-single-page-state.md) first.

**Cluster C is the largest by line count.** Each item has a list-shaped property (`[BitratePreset]`, `[QuickAction]`, `[Macro]`, `[LogEntry]`) and **multiple** mutators (add / remove / select / reorder / save). Each mutator becomes a callback; Rust holds the canonical list in `Arc<Mutex<Vec<…>>>` and pushes the entire list back via `set_*` after every change.

| Item | Slint property | Mutators (callbacks) | Effort |
|---|---|---|---|
| C1 | `Bridge.presets: [BitratePreset]` | `save-preset`, `delete-preset`, `set-active-preset` | Medium |
| C2 | `Bridge.quick-actions: [QuickAction]` (already declared, re-purposed) | `move-bar-action`, `set-bar-action-enabled`, `save-bar-actions` | **High — fixes B12** |
| C4 | `Bridge.macros: [Macro]` | `save-macro`, `delete-macro`, `move-step`, `add-step`, `remove-step`, `run-macro` | High |
| C5 | `Bridge.log-entries` (already declared in A5) | `clear-log-entries` | Tiny |

C3 (cast history list mutations) is in Cluster D, not C, because deletion is destructive. See Section 5.

**Net new code:** ~120 lines bridge.slint, ~250 lines per consumer page (mostly deletions of N-row rebuild helpers), ~400 lines lib.rs.

**Risk:** medium. The N-row rebuild pattern is replaced by Rust-side `Vec::insert/remove/retain`. Any UI-side cache (e.g. "which preset is selected") needs careful handling — see Section 8 / R3.

---

## 4.1 — C1 — Bitrate presets

### What's there today

`pages/bitrate_presets_page.slint` (Phase 16) declares 4 hardcoded presets and a 12-line "rebuild the whole list with one row's flag flipped" helper:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/pages/bitrate_presets_page.slint" lines="22-53" />

`pages/bitrate_preset_edit_page.slint` mutates a draft locally (`draft-name`, `draft-kbps`) and writes back on Save by calling its parent.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Bitrate presets (Phase 8 / Cluster C1) ──────────────────────────
+    in property <[BitratePreset]> presets: [];
+    in-out property <string>      selected-preset-id: "";   // for the edit-page route-back
+    callback save-preset(string, string, int);              // (id, name, kbps); id="" → new
+    callback delete-preset(string);                         // (id)
+    callback set-active-preset(string);                     // (id)
     // …
 }
```

**Why three callbacks instead of one omnibus `mutate-presets`:**

- Each mutation has a distinct intent. `save-preset(id, name, kbps)` carries the new field values as parameters; `delete-preset(id)` is just one string; `set-active-preset(id)` is also just one string but the semantics are different (toggle active flag, not delete).
- Distinct callbacks make the lib.rs handlers straightforward and let Rust validate per-intent (e.g. reject `delete-preset("med")` if Medium is the active preset).

### Step 2: consumer migration — `bitrate_presets_page.slint`

```diff
 export component BitratePresetsPage inherits Rectangle {
-    in-out property <[BitratePreset]> mock-presets: [
-        { id: "low",  name: "Low",     bitrate-kbps: 1500,  active: false },
-        { id: "med",  name: "Medium",  bitrate-kbps: 4000,  active: true  },
-        { id: "high", name: "High",    bitrate-kbps: 8000,  active: false },
-        { id: "max",  name: "Maximum", bitrate-kbps: 15000, active: false },
-    ];
-
     width: 100%;
     height: 100%;
     background: Theme.surface-primary;

-    function select(id: string) {
-        root.mock-presets = [
-            { id: root.mock-presets[0].id, name: root.mock-presets[0].name,
-              bitrate-kbps: root.mock-presets[0].bitrate-kbps,
-              active: root.mock-presets[0].id == id },
-            { id: root.mock-presets[1].id, name: root.mock-presets[1].name,
-              bitrate-kbps: root.mock-presets[1].bitrate-kbps,
-              active: root.mock-presets[1].id == id },
-            { id: root.mock-presets[2].id, name: root.mock-presets[2].name,
-              bitrate-kbps: root.mock-presets[2].bitrate-kbps,
-              active: root.mock-presets[2].id == id },
-            { id: root.mock-presets[3].id, name: root.mock-presets[3].name,
-              bitrate-kbps: root.mock-presets[3].bitrate-kbps,
-              active: root.mock-presets[3].id == id },
-        ];
-    }

     VerticalLayout {
         // … header unchanged …
         ScrollView {
             VerticalLayout {
-                for preset[i] in root.mock-presets: Rectangle {
+                for preset[i] in Bridge.presets: Rectangle {
                     // … rect props unchanged …
                     ta := TouchArea {
-                        clicked => { root.select(preset.id); }
+                        clicked => { Bridge.set-active-preset(preset.id); }
                     }
                     // … row layout unchanged …
                 }

                 // ── Add preset button ───────────────────────────────────
                 // (B7 from UI-REVIEW recommends adding a separator before
                 // this; that fix is independent of Phase 8.)
                 PrimaryButton {
                     label: @tr("Add preset");
                     clicked => {
+                        Bridge.selected-preset-id = "";
                         Bridge.active-panel = Panel.bitrate-preset-edit;
                     }
                 }
             }
         }
     }
 }
```

### Step 3: consumer migration — `bitrate_preset_edit_page.slint`

The edit page reads `selected-preset-id` to decide whether it's editing an existing preset or creating a new one. On Save it calls `Bridge.save-preset(...)` and routes back.

```diff
 export component BitratePresetEditPage inherits Rectangle {
-    in-out property <string> mock-name:         "New preset";
-    in-out property <int>    mock-bitrate-kbps: 4000;
+    // Source-of-truth for "are we editing an existing preset?" comes from
+    // Bridge.selected-preset-id. The page maintains a draft locally so the
+    // user can cancel without persisting partial edits.
+    property <string> draft-name:
+        Bridge.selected-preset-id == ""
+            ? "New preset"
+            : preset-by-id(Bridge.presets, Bridge.selected-preset-id).name;
+    property <int>    draft-kbps:
+        Bridge.selected-preset-id == ""
+            ? 4000
+            : preset-by-id(Bridge.presets, Bridge.selected-preset-id).bitrate-kbps;

     // … form fields read draft-name / draft-kbps …

     // ── Toolbar ─────────────────────────────────────────────────────────
     HorizontalLayout {
         TextButton {
             label: @tr("Cancel");
             clicked => { Bridge.active-panel = Panel.bitrate-presets; }
         }
         PrimaryButton {
             label: @tr("Save");
             clicked => {
+                Bridge.save-preset(
+                    Bridge.selected-preset-id,
+                    root.draft-name,
+                    root.draft-kbps,
+                );
                 Bridge.active-panel = Panel.bitrate-presets;
             }
         }
     }
 }

+// Helper since Slint has no built-in dictionary lookup. Linear scan over a
+// 4-row list is fine.
+pure function preset-by-id(list: [BitratePreset], id: string) -> BitratePreset {
+    for p[i] in list { if p.id == id { return p; } }
+    return { id: "", name: "", bitrate-kbps: 0, active: false };
+}
```

**Note:** Slint doesn't have a `find` helper on lists. The `for p[i] in list { if … return … }` pattern is the canonical workaround — see `guide/language/coding/repetition-and-data-models.mdx`.

### Step 4: Rust producer + handlers

```rust
// senders/android/src/lib.rs

let presets: Arc<Mutex<Vec<BitratePreset>>> = Arc::new(Mutex::new(vec![
    BitratePreset { id: "low".into(),  name: "Low".into(),     bitrate_kbps: 1500,  active: false },
    BitratePreset { id: "med".into(),  name: "Medium".into(),  bitrate_kbps: 4000,  active: true  },
    BitratePreset { id: "high".into(), name: "High".into(),    bitrate_kbps: 8000,  active: false },
    BitratePreset { id: "max".into(),  name: "Maximum".into(), bitrate_kbps: 15000, active: false },
]));

// Monotonic counter for new preset ids. Do NOT use `g.len()` for id
// generation — after a delete-then-add cycle, len can return a value
// that was previously used (delete one of 5 → len=4 → next add reuses
// `custom-4` if it ever existed). An AtomicUsize is the simplest
// always-unique source; alternatively use uuid::Uuid::new_v4().
use std::sync::atomic::{AtomicUsize, Ordering};
let next_preset_id = Arc::new(AtomicUsize::new(0));

// `push_presets` is a closure so it captures `presets` and `ui_weak` once
// and can be cloned into each callback handler. The body just rebuilds
// the VecModel and calls set_presets.
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
    let presets       = presets.clone();
    let next_id       = next_preset_id.clone();
    let push          = push_presets.clone();
    move |id, name, kbps| {
        let mut g = presets.lock().unwrap();
        if id.is_empty() {
            // New preset. Use the monotonic counter — never `g.len()`,
            // which can collide with previously-deleted ids.
            let new_id = format!("custom-{}", next_id.fetch_add(1, Ordering::Relaxed));
            g.push(BitratePreset {
                id:           new_id.into(),
                name:         name.into(),
                bitrate_kbps: kbps,
                active:       false,
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
    let push    = push_presets.clone();
    move |id| {
        let mut g = presets.lock().unwrap();
        g.retain(|p| p.id != id);
        // Edge-case: if the deleted preset was active, promote first to active.
        let any_active = g.iter().any(|p| p.active);
        if !any_active {
            if let Some(first) = g.first_mut() { first.active = true; }
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_set_active_preset({
    let presets = presets.clone();
    let push    = push_presets.clone();
    move |id| {
        let mut g = presets.lock().unwrap();
        for p in g.iter_mut() {
            p.active = p.id == id;
        }
        drop(g);
        push();
    }
});
```

**Why `Mutex<Vec<…>>` not `parking_lot` / `tokio::Mutex`:**

- The std `Mutex` is sync-only, but every handler closure runs on the UI thread (Slint dispatches callbacks on the slint event loop). No `await` is held under the lock; the whole critical section is `g.lock().unwrap() → mutate → drop(g) → push()`.
- `push()` does NOT hold the lock — the snapshot is copied out before the upgrade-in-event-loop. So the lock is held for microseconds.

**Why `push()` is a separate closure not a method:**

- Cloning a closure is cheap; cloning state would be expensive. By having `push_presets` capture `presets.clone()` and `ui_weak.clone()` once, every handler can `let push = push_presets.clone();` without re-capturing.
- A trait method on `BridgeHandle` would also work, but the closure form is shorter and matches what's already used by other Bridge handlers in `lib.rs`.

**Slint doc citations for C1:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx` — Rust tab `Rc<VecModel<…>>` push pattern.

---

## 4.2 — C2 — Quick-action customisation (and **the live `CastControlBar` unification — fixes B12**)

This is the **architectural** fix for B12 in [`UI-REVIEW-2026-05-10.md`](./UI-REVIEW-2026-05-10.md). It's the most important migration in Cluster C because the bar is always-visible.

### Why this is special

`Bridge.quick-actions` already exists and is already pushed by Rust at startup (`set_quick_actions` call at lib.rs:1002). **But** the consumer is wrong — `components/control_bar.slint` reads `root.mock-quick-actions` (a hardcoded 8-row inline list), ignoring what Rust provides:

<ref_snippet file="/home/ubuntu/repos/fcast/senders/android/ui/components/control_bar.slint" lines="45-82" />

Meanwhile `pages/quick_actions_page.slint` (Phase 17) lets the user reorder/disable bar entries — but mutates its own `root.mock-bar-actions`, **not** `Bridge.quick-actions`. So we have two lists that pretend to be one.

**The fix:** unify on `Bridge.quick-actions`. The customisation page mutates via callbacks; Rust pushes to `Bridge.quick-actions`; the bar reads from `Bridge.quick-actions`.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
     in property <[QuickAction]> quick-actions: [];   // already there since Phase 5

+    // ── Bar customisation (Phase 8 / Cluster C2 — fixes B12) ────────────
+    callback move-bar-action(int, int);             // (from-idx, to-idx)
+    callback set-bar-action-enabled(int, bool);     // (idx, enabled)
+    callback save-bar-actions();                    // commit pending edits to disk
     // …
 }
```

### Step 2: consumer migration — `components/control_bar.slint`

```diff
 export component CastControlBar inherits Rectangle {
-    // UI-only stub model. Phase 8 swaps for `Bridge.quick-actions` driven
-    // by Rust. The spec calls these out as the canonical Phase-7 set:
-    //   settings, debug, codec-test, scan-qr, migrated-server.
-    in-out property <[QuickAction]> mock-quick-actions: [
-        { id: "settings",        title: "Settings",       enabled: true, active: false },
-        { id: "debug",           title: "Debug",          enabled: true, active: false },
-        { id: "codec-test",      title: "Codec test",     enabled: true, active: false },
-        { id: "scan-qr",         title: "Scan QR",        enabled: true, active: false },
-        { id: "record",          title: "Record",         enabled: true, active: false },
-        { id: "pair",            title: "Pair",           enabled: true, active: false },
-        { id: "migrated-server", title: "Migrated srv",   enabled: true, active: false },
-        { id: "bitrate",         title: "Bitrate",        enabled: true, active: false },
-    ];
-
     height: Theme.control-bar-height;
     background: Theme.surface-bar;

     HorizontalLayout {
         padding: Theme.padding-card;
         spacing: Theme.spacing-default;
         alignment: start;

-        for action in root.mock-quick-actions: QuickActionButton {
+        for action in Bridge.quick-actions: QuickActionButton {
             action: action;
             invoked(id) => {
                 // Panel-opening ids stay in Slint — no Rust round-trip.
                 if (id == "settings")    { Bridge.active-panel = Panel.settings;    return; }
                 if (id == "debug")       { Bridge.active-panel = Panel.debug;       return; }
                 if (id == "codec-test")  { Bridge.active-panel = Panel.codec-test;  return; }
                 if (id == "record")      { Bridge.active-panel = Panel.recording;   return; }
                 if (id == "pair")        { Bridge.active-panel = Panel.pairing;     return; }
                 if (id == "bitrate")     { Bridge.active-panel = Panel.bitrate-presets; return; }
                 // Non-panel ids still go through the Rust handler.
                 Bridge.invoke-action(id);
             }
         }
     }
 }
```

**Why we don't move panel-opening to Rust too:**

- Opening a panel is a pure Slint state change (`Bridge.active-panel = Panel.foo`). It doesn't need device access.
- Round-tripping through Rust adds latency (one event-loop hop) for no benefit.
- Slint owns the panel router. Rust is the data layer.

### Step 3: consumer migration — `pages/quick_actions_page.slint`

```diff
 export component QuickActionsPage inherits Rectangle {
-    in-out property <[QuickAction]> mock-bar-actions: [
-        { id: "settings",   title: "Settings",   enabled: true, active: false, is-macro: false },
-        { id: "debug",      title: "Debug",      enabled: true, active: false, is-macro: false },
-        { id: "codec-test", title: "Codec test", enabled: true, active: false, is-macro: false },
-        { id: "scan-qr",    title: "Scan QR",    enabled: true, active: false, is-macro: false },
-        { id: "record",     title: "Record",     enabled: true, active: false, is-macro: false },
-    ];
-
-    function swap(i: int, j: int) {
-        // … 5-row hardcoded rebuild …
-    }
-    function set-enabled(i: int, v: bool) {
-        // … 5-row hardcoded rebuild …
-    }

     // …

-    for action[i] in root.mock-bar-actions: Rectangle {
+    for action[i] in Bridge.quick-actions: Rectangle {
         // … row layout …
         // ▲ button:
         clicked => {
-            if (i > 0) { root.swap(i, i - 1); }
+            if (i > 0) { Bridge.move-bar-action(i, i - 1); }
         }
         // ▼ button:
         clicked => {
-            if (i < root.mock-bar-actions.length - 1) { root.swap(i, i + 1); }
+            if (i < Bridge.quick-actions.length - 1) { Bridge.move-bar-action(i, i + 1); }
         }
         // Enable toggle:
         toggled(checked) => {
-            root.set-enabled(i, checked);
+            Bridge.set-bar-action-enabled(i, checked);
         }
     }
 }
```

### Step 4: Rust producer + handlers

```rust
// senders/android/src/lib.rs — replace the existing actions vec at lines 988-1000.

let mut actions = vec![
    QuickAction { id: "settings".into(),   title: "Settings".into(),    enabled: true,  active: false, is_macro: false },
    QuickAction { id: "debug".into(),      title: "Debug".into(),       enabled: true,  active: false, is_macro: false },
    QuickAction { id: "codec-test".into(), title: "Codec test".into(),  enabled: true,  active: false, is_macro: false },
    QuickAction { id: "scan-qr".into(),    title: "Scan QR".into(),     enabled: true,  active: false, is_macro: false },
    QuickAction { id: "record".into(),     title: "Record".into(),      enabled: true,  active: false, is_macro: false },
    QuickAction { id: "pair".into(),       title: "Pair".into(),        enabled: true,  active: false, is_macro: false },
    QuickAction { id: "bitrate".into(),    title: "Bitrate".into(),     enabled: true,  active: false, is_macro: false },
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
    let push        = push_bar.clone();
    move |from, to| {
        let mut g = bar_actions.lock().unwrap();
        if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
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
    let push        = push_bar.clone();
    move |idx, enabled| {
        let mut g = bar_actions.lock().unwrap();
        if let Ok(i) = usize::try_from(idx) {
            if let Some(a) = g.get_mut(i) { a.enabled = enabled; }
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_save_bar_actions({
    let bar_actions = bar_actions.clone();
    let push        = push_bar.clone();
    move || {
        // Phase 11: persist to DataStore via JNI here.
        // For now, just re-push the in-memory state.
        push();
    }
});
```

**After this commit lands, B12 from the UI review is fixed** — the bar reads from the same `Bridge.quick-actions` that the customisation page mutates.

### Step 5: Verification (C2-specific)

```sh
# Both consumers point at the same global property.
grep -n 'mock-quick-actions\|mock-bar-actions' senders/android/ui/
# Expected: 0 matches.

grep -n 'Bridge.quick-actions' senders/android/ui/
# Expected: ≥2 matches (control_bar.slint AND quick_actions_page.slint).

cargo build -p android-sender
adb install <build path>
adb shell am start org.fcast.android.sender/.MainActivity
# Verify the bar shows 7 actions in release / 11 actions in debug. The bar's
# entries match what's listed on the customisation page.
```

**Slint doc citations for C2:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx` — `Bridge.quick-actions` is a global property; both consumers see the same identity.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

---

## 4.3 — C4 — Macros

**Files:** `pages/macros_page.slint`, `pages/macro_edit_page.slint`. Largest callback surface in Phase 8.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    // ── Macros (Phase 8 / Cluster C4) ───────────────────────────────────
+    in property <[Macro]> macros: [];
+    callback save-macro(string, string, [MacroStep], bool);   // (id, name, steps, enabled); id="" → new
+    callback delete-macro(string);                            // (id)
+    callback move-step(string, int, int);                     // (macro-id, from-idx, to-idx)
+    callback add-step(string, string);                        // (macro-id, action-id)
+    callback remove-step(string, int);                        // (macro-id, step-idx)
+    callback run-macro(string);                               // (macro-id)
     // …
 }
```

Rename `Bridge.mock-macro-edit-id` → `Bridge.macro-edit-id` in the same diff. The `mock-` prefix was a hold-over; the property has always been Bridge-shaped.

### Step 2: consumer migration

The diff is mechanical. In `macros_page.slint`:

```diff
 export component MacrosPage inherits Rectangle {
-    in-out property <[Macro]> mock-macros: [ /* …hardcoded entries… */ ];
-
-    function swap-macro(i: int, j: int) { /* …N-row rebuild… */ }
-    function delete-macro(id: string) { /* …filter rebuild… */ }
-
     // …
-    for m in root.mock-macros: MacroRow {
+    for m in Bridge.macros: MacroRow {
         data: m;
         tap-edit => {
-            Bridge.mock-macro-edit-id = m.id;
+            Bridge.macro-edit-id = m.id;
             Bridge.active-panel = Panel.macro-edit;
         }
-        tap-delete => { root.delete-macro(m.id); }
+        tap-delete => { Bridge.delete-macro(m.id); }
-        tap-run    => { Bridge.invoke-action("macro:" + m.id); }
+        tap-run    => { Bridge.run-macro(m.id); }
     }

     PrimaryButton {
         label: @tr("New macro");
         clicked => {
-            Bridge.mock-macro-edit-id = "";
+            Bridge.macro-edit-id = "";
             Bridge.active-panel = Panel.macro-edit;
         }
     }
 }
```

In `macro_edit_page.slint`, the page maintains a `draft-steps: [MacroStep]` locally (the user is editing — we don't push every keystroke to Rust). On Save:

```diff
     PrimaryButton {
         label: @tr("Save");
         clicked => {
-            // … N-row hardcoded rebuild of root.draft-steps to a new mock-macros … */
+            Bridge.save-macro(
+                Bridge.macro-edit-id,    // "" if creating new
+                root.draft-name,
+                root.draft-steps,
+                root.draft-enabled,
+            );
             Bridge.active-panel = Panel.macros;
         }
     }
```

Within the page, the user can still reorder / add / remove steps inline. Two strategies:

1. **Reorder mutates `draft-steps` only** (current behavior, if Phase 25 was implemented that way). Save commits the whole list.
2. **Reorder calls `Bridge.move-step(macro-edit-id, i, j)` immediately** and Rust pushes back. This works if you want consistency between the edit page and any other view of the same macro.

For Phase 8, **option 1** is recommended — keeps the edit page transactional ("Cancel" reverts; "Save" commits). Option 2 needs additional plumbing for "Cancel" semantics.

### Step 3: Rust producer + handlers

The pattern is identical to C1 / C2 but with more callbacks. Skeleton:

```rust
let macros: Arc<Mutex<Vec<Macro>>> = Arc::new(Mutex::new(vec![
    // Phase 8 bring-up: empty list. User creates macros via the UI.
]));

// Same caveat as C1: do NOT use `g.len()` for new-macro ids. After a
// delete-then-add cycle len() can collide with a previously-issued id.
let next_macro_id = Arc::new(AtomicUsize::new(0));

let push_macros = {
    let macros = macros.clone();
    let ui_weak = ui.as_weak();
    move || {
        let snap = macros.lock().unwrap().clone();
        let _ = ui_weak.upgrade_in_event_loop(move |ui| {
            ui.global::<Bridge>().set_macros(
                std::rc::Rc::new(slint::VecModel::from(snap)).into(),
            );
        });
    }
};
push_macros();

ui.global::<Bridge>().on_save_macro({
    let macros  = macros.clone();
    let next_id = next_macro_id.clone();
    let push    = push_macros.clone();
    move |id, name, steps, enabled| {
        let steps_vec: Vec<MacroStep> =
            steps.iter().collect::<Vec<_>>().into_iter().collect();
        let mut g = macros.lock().unwrap();
        if id.is_empty() {
            let new_id = format!("macro-{}", next_id.fetch_add(1, Ordering::Relaxed));
            g.push(Macro {
                id:      new_id.into(),
                name:    name.into(),
                steps:   std::rc::Rc::new(slint::VecModel::from(steps_vec)).into(),
                enabled,
            });
        } else if let Some(m) = g.iter_mut().find(|m| m.id == id) {
            m.name    = name.into();
            m.enabled = enabled;
            m.steps   = std::rc::Rc::new(slint::VecModel::from(steps_vec)).into();
        }
        drop(g);
        push();
    }
});

ui.global::<Bridge>().on_delete_macro({
    let macros = macros.clone();
    let push   = push_macros.clone();
    move |id| {
        macros.lock().unwrap().retain(|m| m.id != id);
        push();
    }
});

ui.global::<Bridge>().on_run_macro({
    let macros = macros.clone();
    let ui_weak = ui.as_weak();
    move |id| {
        let snap = macros.lock().unwrap().iter()
            .find(|m| m.id == id).cloned();
        let Some(m) = snap else {
            // Banner via Cluster F.
            flash_banner(ui_weak.clone(),
                format!("Macro {} not found", id), BannerSeverity::Error,
                std::time::Duration::from_secs(3));
            return;
        };
        // Phase 11: real macro engine (iterate m.steps, dispatch each via on_invoke_action).
        flash_banner(ui_weak.clone(),
            format!("Ran macro: {}", m.name), BannerSeverity::Success,
            std::time::Duration::from_secs(2));
    }
});

// move-step / add-step / remove-step are skipped if you took option 1.
// If you took option 2, the handlers operate on macros[id].steps directly:

ui.global::<Bridge>().on_move_step({
    let macros = macros.clone();
    let push   = push_macros.clone();
    move |macro_id, from, to| {
        let mut g = macros.lock().unwrap();
        if let Some(m) = g.iter_mut().find(|m| m.id == macro_id) {
            // m.steps is a ModelRc<MacroStep>. Need to pull it into a Vec, mutate, push back.
            let mut steps_vec: Vec<MacroStep> = m.steps.iter().collect();
            if let (Ok(from_u), Ok(to_u)) = (usize::try_from(from), usize::try_from(to)) {
                if from_u < steps_vec.len() && to_u < steps_vec.len() && from_u != to_u {
                    let s = steps_vec.remove(from_u);
                    steps_vec.insert(to_u, s);
                }
            }
            m.steps = std::rc::Rc::new(slint::VecModel::from(steps_vec)).into();
        }
        drop(g);
        push();
    }
});
```

**Why `Vec<MacroStep>` round-trip for `move-step`:**

- `ModelRc<T>` is read-only without a concrete model implementation. The simplest correct approach is "pull all rows into a Vec, mutate, push a new VecModel back." For nested struct-of-list this is unavoidable — see `tutorial/creating_the_tiles.mdx`.
- Performance: 10-30 steps per macro × the reorder cost ~~ negligible.

**Slint doc citations for C4:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx` — `Macro` struct with nested `steps: [MacroStep]`.
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

---

## 4.4 — C5 — Debug log clear

**File:** `pages/debug_log_page.slint`. Cluster A5 already wired the read side. C5 adds the clear callback.

`mock-min-level-idx` **stays Slint-side** — it's a UI filter selection, not Rust signal data.

### Step 1: extend `bridge.slint`

```diff
 export global Bridge {
     // …
+    callback clear-log-entries();
     // …
 }
```

### Step 2: consumer migration

```diff
 // debug_log_page.slint — bottom toolbar
 TextButton {
     label: @tr("Clear");
-    // No-op in UI-only build — Phase 8 will clear the
-    // Rust-side ring buffer.
-    clicked => { root.mock-log = []; }
+    clicked => { Bridge.clear-log-entries(); }
 }
```

(`Copy all` stays Slint-side for now — Phase 11 wires it through `Bridge.copy-log()` once clipboard JNI lands.)

### Step 3: Rust handler — already shown in Cluster A5

```rust
ui.global::<Bridge>().on_clear_log_entries({
    let log_ring = log_ring.clone();
    let ui_weak  = ui.as_weak();
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

(If you didn't ship Cluster A5 yet, this handler can stand alone — it just doesn't push to the UI in any meaningful way until A5 lands.)

**Slint doc citations for C5:**

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`

---

## 4.5 Cluster C verification

```sh
# 1. mock-* count drops by ~10 (1 presets + 1 macros + 5 quick-actions list + 2 bar + 1 debug-log entries + 1 macro-edit-id rename = ~9)
grep -rnE 'in-out property <[^>]+> mock-|in property <[^>]+> mock-' senders/android/ui/ | wc -l

# 2. Bridge declarations
grep -nE '(presets|macros|quick-actions|log-entries|move-bar-action|set-bar-action-enabled|save-bar-actions|save-preset|delete-preset|set-active-preset|save-macro|delete-macro|move-step|add-step|remove-step|run-macro|clear-log-entries|macro-edit-id)' \
    senders/android/ui/bridge.slint

# 3. Old mock-quick-actions / mock-bar-actions / mock-presets / mock-macros / mock-macro-edit-id are gone everywhere
grep -rn 'mock-quick-actions\|mock-bar-actions\|mock-presets\|mock-macros\|mock-macro-edit-id' senders/android/ui/
# Should be empty.

# 4. Build green
cargo build -p android-sender
cargo clippy -p android-sender --all-targets -- -D warnings
```

---

## 4.6 Commit messages

```
feat(android): Phase 8 / C1 — bitrate presets via Bridge.presets + 3 callbacks
feat(android): Phase 8 / C2 — unify CastControlBar + customisation onto Bridge.quick-actions (fixes B12)
feat(android): Phase 8 / C4 — macros via Bridge.macros + 6 callbacks
feat(android): Phase 8 / C5 — debug log clear via Bridge.clear-log-entries
```

---

## 4.7 Exit criteria for Section 4

- [x] All 4 items wired (C1, C2, C4, C5)
  - [x] C5 Slint-side wired (Rust handler pending A5)
- [x] **B12 visually verified on-device:** the bar shows the same actions the user customised on the Quick-actions page
- [x] No `mock-quick-actions`, `mock-bar-actions`, `mock-presets`, `mock-macros`, `mock-macro-edit-id` anywhere in `senders/android/ui/`
- [x] `Bridge.macro-edit-id` (no `mock-` prefix)
- [ ] `cargo build` and `cargo clippy --all-targets -- -D warnings` are green
- [ ] mock-* inventory dropped by ~10 lines

You can now move to **Section 5 — Cluster D: destructive flows** at [`PHASE-8-Section-5-cluster-D-destructive-flows.md`](./PHASE-8-Section-5-cluster-D-destructive-flows.md).

---

## Slint-doc references used

- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/structs-and-enums.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/globals.mdx`
- `draft/slint-ui/docs/astro/src/content/docs/tutorial/creating_the_tiles.mdx`

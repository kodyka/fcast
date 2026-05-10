# Phase 9 — Localisation `@tr()` Sweep reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-9-localization.md`][spec] to the current `senders/android` tree.
**Goal:** wrap every user-visible string literal in `senders/android/ui/**/*.slint` in `@tr("...")` so future translations can land without touching the UI source. Generate `messages.pot` template, add `.po`/`.mo` to `.gitignore`, ship English-only fallback (Slint's runtime falls back to the `msgid` literal when no `.mo` is loaded).
**Scope:** Slint UI only. **No Rust changes.** No real translations — `messages.pot` is the extraction template, no `.po` files are committed.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-9-localization.md

> **Run this phase only after the UI surface has stabilised.** Each in-flight UI phase ships English literals; `@tr()` wrapping is **not** the responsibility of those phases. Phase 9 is a single sweep across already-merged pages, repeatable each time a new UI phase merges. The audit greps in Section 3 are the regression guard — run them in CI to catch missed wrappings.

---

## Why this guide exists

Phase 9 is **ongoing** like Phase 27 — it isn't done in one shot. Every time a UI phase merges, Phase 9 sweeps the new strings. This guide:

1. **Establishes the canonical `@tr()` form** (plain, with context, with plurals).
2. **Lists every page/component touched by Phases 5–7 and 12–27** as a sweep checklist.
3. **Documents three patterns that are NOT translatable** (action ids, enum names, debug log strings) so the developer doesn't waste effort wrapping them.
4. **Pins down the `slint-tr-extractor` invocation** that survives CI (find/xargs form, not bash globstar).
5. **Documents context-disambiguation** for short/duplicated strings ("Cancel", "Done", "Save") and the plural-form syntax — which is what Phase 9's spec gets wrong (`@tr("a" | "b" % n)` is **not** for context).

After this guide is applied:
- All user-visible strings are wrapped.
- `senders/android/ui/i18n/messages.pot` exists.
- `.gitignore` excludes `i18n/*.po` and `i18n/*.mo`.
- The audit grep produces zero unexpected hits.

---

## Section 0 — Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-9-localization

# Verify pinned Slint version supports @tr (added in 1.3; current futo fork is 1.15+).
grep -rn '^slint = ' senders/android/Cargo.toml
# Or the Cargo.lock entry:
grep -A1 'name = "slint"' senders/android/Cargo.lock | head -2

# Install the extractor (versioned independently of slint itself):
cargo install slint-tr-extractor

cargo check -p android-sender
```

**If the build fails complaining about `@tr`,** stop here. Phase 9 is deferred until the Slint version is bumped. Document the deferral in the repo README.

---

## Section 1 — The `@tr()` form (cheatsheet)

### Plain wrapping

```slint
Text { text: @tr("Casting"); }                // simple
TextButton { label: @tr("Done"); }
SettingsValueRow { title: @tr("Network"); }
```

The literal `"Casting"` becomes the `msgid` and the English fallback in one go.

### With context (disambiguating short strings that recur with different meanings)

Slint context syntax is `@tr("<context-key>" => "<source string>")`:

```slint
TextButton { label: @tr("cancel-cast-button"   => "Cancel"); }
TextButton { label: @tr("dismiss-dialog-button" => "Cancel"); }
TextButton { label: @tr("start-cast-button"     => "Start"); }
TextButton { label: @tr("close-panel-button"    => "Done"); }
TextButton { label: @tr("save-preset-button"    => "Save"); }
```

Each context produces a distinct `.po` entry; translators can render the same English string differently for each context. See [translations.mdx][translations] § "Context".

### With format arguments (interpolation)

```slint
Text { text: @tr("{n} found", root.found-count); }
Text { text: @tr("{} of {}", root.current, root.total); }
```

The `{}` placeholder is positional; `{n}` is named. Slint extracts the placeholder type from the bound expression. Interpolation arguments are passed as separate parameters after the format string.

### With plurals

The `|` operator is for **plurals only** — the syntax is:

```slint
@tr("{n} receiver" | "{n} receivers" % count)
```

Three parts: singular form, plural form, count expression. Slint emits `nplurals=2` `.po` entries; the runtime selects based on the count.

**Common error in spec drafts:** `@tr("a" | "b" % n)` for context disambiguation. **This is wrong** — the `|` is for plurals, not context. Use `@tr("ctx" => "string")` for context.

---

## Section 2 — Sweep checklist

Each row is a file. For each, audit the file with the regex below, then wrap. The regex catches multi-word capitalised strings — most user-visible labels follow that pattern. Single-word strings (`"Done"`, `"Save"`) need eyeballing.

### Phase 5–7 baseline (already merged)

- [x] `senders/android/ui/pages/connect_page.slint`
- [x] `senders/android/ui/pages/connecting_page.slint`
- [x] `senders/android/ui/pages/casting_page.slint`
- [x] `senders/android/ui/pages/settings_page.slint` (both `SettingsPageView` and `FullSettingsPage`)
- [x] `senders/android/ui/pages/debug_page.slint`
- [x] `senders/android/ui/pages/codec_test_page.slint`
- [x] `senders/android/ui/components/buttons.slint`
- [x] `senders/android/ui/components/settings_rows.slint`
- [x] `senders/android/ui/components/cast_control_bar.slint`
- [x] `senders/android/ui/components/status_overlay.slint`

### Phase 12 (capture preview)

- [x] `senders/android/ui/components/capture_preview.slint`
  - `"Screen capture"`, `"● LIVE"`, `"○ Idle"` — the bullet glyphs **stay literal** (they're symbols, not localisable text). Wrap only `"Screen capture"`.

### Phase 14 (audio settings)

- [x] `senders/android/ui/pages/audio_page.slint`
  - `"Audio capture"`, `"Source"`, `"Mute"`, `"Input gain"`, `"Bitrate"`, `"Done"`, plus the cycler value strings `"Microphone"`, `"System audio"`, `"Microphone + system"`.

### Phase 15 (camera settings)

- [x] `senders/android/ui/pages/camera_page.slint`
  - `"Camera"`, `"Source"`, `"Resolution"`, `"Frame rate"`, `"Mirror preview"`, `"Stabilization"`, `"Tap to focus"`, `"Zoom"`, plus cycler values.

### Phase 16 (bitrate presets)

- [x] `senders/android/ui/pages/bitrate_presets_page.slint`
- [x] `senders/android/ui/pages/bitrate_preset_edit_page.slint`
  - `"Bitrate presets"`, `"Active"`, `"Edit preset"`, `"Name"`, `"Bitrate"`, `"Cancel"`, `"Save"`. The unit suffix `"kbps"` and `"Mbps"` are also wrapped — units are localisable in some locales.

### Phase 17 (quick-action customisation)

- [x] `senders/android/ui/pages/quick_actions_page.slint`
  - `"Quick actions"`, `"Enable"`, `"More than 6 enabled — extras hide on small screens."`. The action labels themselves (`"Settings"`, `"Codec test"`, …) are wrapped at the QuickAction struct site (Phase 4).

### Phase 18 (lifecycle overlays)

- [x] `senders/android/ui/components/lock_overlay.slint`
- [x] `senders/android/ui/components/stealth_overlay.slint`
- [x] `senders/android/ui/components/snapshot_countdown.slint`
- [x] `senders/android/ui/pages/settings_page.slint` (the new PRIVACY section title, row labels)
  - `"UI Locked"`, `"Press and hold to unlock"`, `"Tap to wake"`, `"Snapshot in"`, `"PRIVACY"`, `"Engage lock screen"`, `"Stealth mode"`, `"Snapshot countdown"`.

### Phase 19 (backup/reset + ConfirmDialog)

- [x] `senders/android/ui/components/confirm_dialog.slint`
- [x] `senders/android/ui/pages/backup_reset_page.slint`
  - `"Backup & reset"`, `"Export settings"`, `"Import settings"`, `"Reset all settings"`, plus the dialog titles/bodies/labels.

### Phase 20 (cast history)

- [x] `senders/android/ui/pages/cast_history_page.slint`
- [x] `senders/android/ui/pages/cast_history_detail_page.slint`
  - `"Cast history"`, `"Clear all"`, `"No casts yet."`, `"Started"`, `"Duration"`, `"Status"`, `"Avg bitrate"`, `"Peak bitrate"`, `"Dropped frames"`, plus the receiver names from stub data (those are **data**, not labels — leave unwrapped). The status pill text `"Completed"`, `"Cancelled"`, `"Failed"` is wrapped — these are user-visible.

### Phase 21 (help & support)

- [x] `senders/android/ui/pages/about_page.slint`
- [x] `senders/android/ui/pages/version_history_page.slint`
- [x] `senders/android/ui/pages/attributions_page.slint`
- [x] `senders/android/ui/pages/help_page.slint`
  - `"About"`, `"Version history"`, `"Attributions"`, `"Help"`, `"Report a bug"`, plus help-link labels.

### Phase 22 (network & Wi-Fi Aware)

- [x] `senders/android/ui/pages/network_page.slint`
  - `"Network interfaces"`, `"Wi-Fi Aware"`, `"Wi-Fi Aware deferred to Phase 8"`, plus interface kind labels.

### Phase 23 (recording controls)

- [x] `senders/android/ui/pages/recording_page.slint`
  - `"Local recording"`, `"Start"`, `"Pause"`, `"Resume"`, `"Stop"`, `"Idle"`, `"Recording"`, `"Paused"`, `"Finalizing"`, plus elapsed-counter labels.

### Phase 25 (macros)

- [x] `senders/android/ui/pages/macros_page.slint`
- [x] `senders/android/ui/pages/macro_edit_page.slint`
  - `"Macros"`, `"Add macro"`, `"No macros yet."`, `"Name"`, `"Enabled"`, `"Steps"`, `"Add step"`, `"Pick an action"`, plus action-picker labels.

### Phase 26 (debug log + video pipeline)

- [x] `senders/android/ui/pages/debug_log_page.slint`
- [x] `senders/android/ui/pages/debug_video_page.slint`
  - `"Debug log"`, `"All"`, `"Trace"`, `"Debug"`, `"Info"`, `"Warn"`, `"Error"`, `"Clear"`. Log entries themselves (`entry.message`) are **data**, not localisable.

### Phase 27 (shared utils)

- [x] `senders/android/ui/components/info_banner.slint` — no literal strings; the message is a property.
- [x] `senders/android/ui/components/icon_and_text.slint` — no literal strings.

---

## Section 3 — Audit greps (use these in CI)

### A. Find unwrapped user-visible strings

```sh
# Find any double-quoted multi-word capitalised string (the most common label
# shape) that is NOT inside @tr(). Heuristic — false positives include action
# ids and ASCII-only data.
grep -REn '"[A-Z][a-z]+ [a-z]+' senders/android/ui/ \
    --include='*.slint' \
  | grep -v '@tr(' \
  | grep -v -- '// ' \
  | grep -v 'placeholder-text:' \
  > /tmp/unwrapped-multi-word.txt

if [ -s /tmp/unwrapped-multi-word.txt ]; then
    echo "FAIL: $(wc -l < /tmp/unwrapped-multi-word.txt) unwrapped multi-word strings"
    head -20 /tmp/unwrapped-multi-word.txt
    exit 1
else
    echo "OK: no unwrapped multi-word strings"
fi
```

### B. Find single-word labels that are likely UI text

```sh
# Words that nearly always indicate user-visible labels.
grep -REn '"(Done|Save|Cancel|Edit|Delete|Add|Remove|Open|Close|Start|Stop|Pause|Resume)"' \
    senders/android/ui/ --include='*.slint' \
  | grep -v '@tr(' \
  | grep -v -- '//'
# Expected: 0 matches after the sweep.
```

### C. Find strings inside `text:`, `label:`, `title:` bindings

```sh
# Catches the most common label binding sites.
grep -REn '(text|label|title|placeholder-text):[[:space:]]*"' \
    senders/android/ui/ --include='*.slint' \
  | grep -v '@tr(' \
  | grep -v -- '// '
# Expected: 0 matches after the sweep.
# False positives: action-id strings or symbol glyphs (▶ ●) — review and
# decide per-row whether to wrap.
```

### D. Don't accidentally wrap action ids

```sh
# Action ids are passed to invoke-action callbacks; they're protocol strings.
grep -RnE 'invoke-action\([[:space:]]*@tr\(' senders/android/ui/ \
  && echo "FAIL: action ids should never be wrapped in @tr" \
  || echo "OK"
```

---

## Section 4 — What NOT to wrap

### N1. Action ids (`Bridge.invoke-action("scan-qr")`)

These are protocol identifiers passed to Rust. Translating them would break dispatch. Same for `Panel.<variant>` enum names, `id` fields in stub structs (`mock-presets[i].id`), and `kind` fields in `NetworkInterface`.

### N2. Debug / internal log strings

```slint
debug("camera source changed to \{idx}");   // do NOT wrap
```

These never reach the user. The Slint extractor doesn't pick them up anyway, but resist the urge.

### N3. Symbol glyphs

```slint
Text { text: "›"; ... }    // disclosure indicator — do NOT wrap
Text { text: "●"; ... }    // LIVE bullet
Text { text: "▶"; ... }    // macro indicator
Text { text: "▲"; ... }    // reorder up arrow
Text { text: "▼"; ... }
Text { text: "✕"; ... }
Text { text: "—"; ... }    // em-dash for "no data"
```

Symbols are universal; wrapping them creates pointless `.po` entries.

### N4. Receiver / device / preset names from stub data

```slint
mock-history: [
    { id: "h1", receiver: "Living Room TV", ... },   // do NOT wrap
];
```

These are **data**, not UI labels. Real production data comes from Rust. Stub strings stay literal because they're throwaway.

### N5. Numeric / formatted strings without language content

```slint
Text { text: "\{root.bitrate-kbps} kbps"; }   // wrap as @tr("{n} kbps", root.bitrate-kbps);
Text { text: "1920×1080"; }                    // do NOT wrap — pure numeric
Text { text: "v\{Bridge.app-version}"; }       // do NOT wrap — version string
```

Hairline call: `"kbps"` is conceptually a unit (could be localised). For consistency wrap units (`@tr("kbps")`) but leave purely numeric/version strings alone.

### N6. The `placeholder-text` of an empty form field

`LineEdit { placeholder-text: "Macro name"; }` — wrap. But `LineEdit { placeholder-text: ""; }` — leave (empty string is not a translation candidate).

---

## Section 5 — Generate `messages.pot`

```sh
mkdir -p senders/android/ui/i18n

find senders/android/ui -name '*.slint' \
  | xargs slint-tr-extractor -o senders/android/ui/i18n/messages.pot
```

Why `find ... | xargs` and not bash `**/*.slint` glob:
- Bash globstar (`**`) requires `shopt -s globstar` on the host. CI shells often don't enable it.
- `find ... | xargs` is portable across `sh`, `bash`, `zsh`, and CI runners.

After generation, inspect:

```sh
head -40 senders/android/ui/i18n/messages.pot
```

The file should start with a `msgid ""` header and contain `msgid "..."` / `msgstr ""` pairs for every wrapped string. Verify counts match expectations.

---

## Section 6 — `.gitignore` discipline

```diff
+# Phase 9 — localisation: track only the .pot template, not compiled .po/.mo
+senders/android/ui/i18n/*.po
+senders/android/ui/i18n/*.mo
```

`messages.pot` is committed (the extraction template); per-language `.po` files are added on demand by translators or — if the team opts to vendor translations — checked in deliberately to a sub-directory like `i18n/translations/`.

---

## Section 7 — Worked examples

### Example 1 — Plain wrap on `cast_history_page.slint`

Before:

```slint
Text { text: "Cast history"; ... }
TextButton { label: "Clear all"; ... }
TextButton { label: "Done"; ... }
Text { text: "No casts yet."; ... }
```

After:

```slint
Text { text: @tr("Cast history"); ... }
TextButton { label: @tr("clear-history-button" => "Clear all"); ... }
TextButton { label: @tr("close-panel-button" => "Done"); ... }
Text { text: @tr("No casts yet."); ... }
```

Notes:
- `"Clear all"` gets a context — if other pages use `"Clear all"` for different actions, the contexts disambiguate.
- `"Done"` always uses the same context across panels (close-panel intent).
- `"Cast history"` and `"No casts yet."` are unique enough to not need context.

### Example 2 — Plural on a count

Before:

```slint
Text { text: "\{root.found-count} found"; }
```

After:

```slint
Text {
    text: @tr("{n} receiver found" | "{n} receivers found" % root.found-count);
}
```

### Example 3 — Cycler with localised value strings

Before (Phase 14 audio source cycler):

```slint
SettingsValueRow {
    title: "Source";
    value: ["Microphone", "System audio", "Microphone + system"][root.source-idx];
    clicked => { root.source-idx = Math.mod(root.source-idx + 1, 3); }
}
```

After:

```slint
SettingsValueRow {
    title: @tr("Source");
    value: [
        @tr("Microphone"),
        @tr("System audio"),
        @tr("Microphone + system")
    ][root.source-idx];
    clicked => { root.source-idx = Math.mod(root.source-idx + 1, 3); }
}
```

**Gotcha:** the array literal `[@tr("..."), @tr("..."), ...]` is re-evaluated per access. For a 3-item array per cycler tick that's fine; for hundreds of items, hoist to a property:

```slint
property <[string]> source-labels: [
    @tr("Microphone"),
    @tr("System audio"),
    @tr("Microphone + system"),
];
```

---

## Section 8 — Gotchas (Phase 9 specific)

### Gotcha 55 — `|` is for plurals, not context

**Symptom:** the spec example `@tr("a" | "b" % n)` looks like a context-disambiguator and gets misused.

**Cause:** `|` separates singular and plural forms. The third `% n` slot is the count expression. There is no "context" form using `|`.

**Fix:** use `@tr("context-key" => "string")` for context. Use `@tr("singular" | "plural" % n)` for plurals. They're orthogonal forms; you can combine them: `@tr("ctx" => "{n} item" | "{n} items" % n)`.

### Gotcha 56 — `slint-tr-extractor` is a separate cargo binary

**Symptom:** running `slint --extract-tr` fails — no such subcommand.

**Cause:** `slint-tr-extractor` ships as its own crate, versioned independently of `slint` itself. Install via `cargo install slint-tr-extractor`.

**Fix:** the `cargo install` step is in Section 0. Don't conflate with `slint` or `slint-build`.

### Gotcha 57 — Bash `**` globstar is non-portable

**Symptom:** `slint-tr-extractor -o messages.pot senders/android/ui/**/*.slint` works locally but fails in CI with "no such file" or only finds top-level matches.

**Cause:** globstar (`**`) requires `shopt -s globstar` in bash. POSIX `sh` doesn't support it at all. CI runners often use POSIX `sh` or non-globstar bash.

**Fix:** use `find ... | xargs slint-tr-extractor -o messages.pot`. Always works.

### Gotcha 58 — `messages.pot` must be regenerated after every UI phase merges

**Symptom:** translators see stale `msgid` entries; new strings not present in `.po` files.

**Cause:** `.pot` is generated, not edited by hand. Each new UI string adds entries; each removed string leaves stale entries.

**Fix:** add a CI job that runs the extractor and `git diff --exit-code senders/android/ui/i18n/messages.pot`. Fail the PR if `messages.pot` drifts. Or run the extractor locally before each UI phase PR and include the `.pot` diff in the PR.

### Gotcha 59 — Slint runtime falls back silently to English on missing `.mo`

**Symptom:** translation marked as "shipping" but users see English. No error.

**Cause:** Slint's `@tr()` runtime returns the `msgid` (English literal) when no compiled `.mo` is loaded for the current locale. This is intentional — better than crashing — but it means a missing translation is invisible.

**Fix:** wire a runtime check that logs locale + loaded `.mo` paths at startup (Rust side). Alternatively, ship English-only and document that translations require explicit opt-in (Phase 9 spec § 9-G).

### Gotcha 60 — Don't wrap stub data

**Symptom:** `mock-presets[0].name` is wrapped in `@tr()`, the `.pot` fills with random data strings, translators are confused.

**Cause:** the developer mistook stub names for UI labels. Stub data is throwaway; real data comes from Rust.

**Fix:** review every `@tr()` site. If the wrapped string is inside a `mock-*` initialiser and represents domain data (receiver names, preset names, log entries), unwrap. Section 4 N4 covers the rule.

---

## Section 9 — Exit criteria checklist

- [x] `slint-tr-extractor` installed (`cargo install slint-tr-extractor`).
- [x] All files in Section 2 sweep checklist have `@tr()`-wrapped user-visible strings.
- [x] Audit grep A (multi-word) produces 0 unexpected hits.
- [x] Audit grep B (single-word labels) produces 0 hits outside expected exceptions.
- [x] Audit grep C (label/title/text bindings) produces 0 unexpected hits.
- [x] Audit grep D (action ids never wrapped) reports OK.
- [x] Stub data fields (receiver names, preset names, log entries) NOT wrapped.
- [x] Symbol glyphs (▶ ● ✕ ▲ ▼ › —) NOT wrapped.
- [x] `senders/android/ui/i18n/messages.pot` exists and contains entries for every wrapped string.
- [x] `.gitignore` excludes `senders/android/ui/i18n/*.po` and `*.mo`.
- [x] Context keys are used for short ambiguous strings (`"Cancel"`, `"Done"`, `"Save"`, `"Edit"`).
- [x] Plural forms use `|` operator, not context-disambiguator pattern.
- [x] `cargo build -p android-sender` passes — runtime falls back to English.
- [x] `cargo test -p android-sender` passes — no test reads literal labels.

---

## Section 10 — Re-running on each new UI phase

When a UI phase merges (e.g. a new Phase 28+):

1. Run the audit grep A on the new files.
2. Wrap each unwrapped string per Section 1.
3. Decide which strings need context (Section 1 form 2).
4. Decide which strings have plural forms (Section 1 form 3).
5. Skip the patterns from Section 4.
6. Regenerate `messages.pot` per Section 5.
7. Verify `.gitignore` discipline holds (Section 6).

The sweep takes ~30 minutes per UI phase. Run it as the **last** task before the phase's PR is merged, after all other reviews.

---

## Slint-doc references used

- **`@tr("...")`** — `draft/slint-ui/docs/astro/src/content/docs/guide/development/translations.mdx`.
- **Context syntax `@tr("ctx" => "string")`** — same § "Context".
- **Plural syntax `@tr("a" | "b" % n)`** — same § "Plurals".
- **Format arguments `@tr("{n}", expr)`** — same § "Format strings".
- **Slint version requirement (`@tr` since 1.3)** — `draft/slint-ui/docs/astro/src/content/docs/guide/development/translations.mdx` § intro.
- **`slint-tr-extractor` tool invocation** — same § "Extracting strings".
- **Why CI shouldn't use bash globstar** — operational rule, not a Slint doc reference.

---

## What's NOT in this guide

- **Real translations.** No `.po` files committed; English-only fallback is the runtime story.
- **Wiring `tr` into Phases 14–26 themselves.** Each phase's reimplement guide ships English literals deliberately; Phase 9 is the sweep that wraps them after the fact.
- **Locale detection / loading at runtime.** Rust integration territory — pulls in `gettextrs` or similar; out of scope while Rust bridge is deferred.
- **RTL layout** considerations. Slint supports RTL via `LayoutAlignment`; revisit when a real RTL translation lands.
- **Pluralisation rules for languages with > 2 plural forms** (Russian, Arabic, Polish). Slint's `@tr` plural syntax supports `nplurals` > 2; the `.po` per-language header configures which forms exist. Out of scope while shipping English-only.
- **Test plan for ensuring no string regresses to unwrapped state.** Phase 10 covers the audit grep as a CI step.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-9-localization.md
[translations]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/development/translations.mdx

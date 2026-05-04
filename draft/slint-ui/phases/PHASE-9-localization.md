# Phase 9 — Localization Preparation (UI-only)

> Wrap user-visible strings in `@tr("...")` so they can be translated later.
> **UI-only phase** — no `.po` files, no real translations, no Rust changes.
> The placeholder build ships English-only forever until the team explicitly
> opts in to translation work.

**Status:** `[ ] Not started`
**Depends on:** Phase 7 (settings strings exist before wrapping them) + any of Phases 12–27 that have shipped.
**Functional integration:** None. `@tr()` falls back to the literal English string at runtime when no `.po` is loaded.
**Related files:**
- All `senders/android/ui/**/*.slint`
- `senders/android/ui/i18n/messages.pot` — generated template (committed)

> **Note:** Do **not** copy `draft/moblin-ui/Common/Localizable.xcstrings` — it's
> iOS/macOS XLIFF format and is not usable by Slint's `@tr` system.

---

## Tasks

### 9-A — Confirm `@tr` support in pinned Slint version

- [ ] Check `Cargo.lock` — the futo fork is on 1.15.x, well past `@tr`'s
  introduction in 1.3. No action needed unless the build fails.
- [ ] If a build fails complaining about `@tr`, mark this phase **deferred**
  and skip to Phase 10.

---

### 9-B — Wrap strings in pages built by Phases 5–7

For each page touched by Phases 5–7, replace literal `"text"` strings with
`@tr("text")`. The literal becomes the `msgid` and the default English
fallback in one go.

Pages to wrap (skip any that haven't been built yet):

- [ ] `connect_page.slint`
- [ ] `connecting_page.slint`
- [ ] `casting_page.slint`
- [ ] `settings_page.slint` (both `SettingsPageView` and `FullSettingsPage`)
- [ ] `debug_page.slint`
- [ ] `codec_test_page.slint` (Phase 7-I)
- [ ] All component files: `buttons.slint`, `settings_rows.slint`,
  `cast_control_bar.slint`, `status_overlay.slint`

**What to wrap:** anything a user reads. **What NOT to wrap:** action ids
(`"scan-qr"`, `"settings"`), enum names (`Panel.settings`), property names,
debug/internal log strings.

---

### 9-C — Wrap strings in pages built by Phases 12–27 (ongoing)

Each new UI phase (12–27) ships English literals. **Do not** add `@tr()`
inside those phases — that's this phase's job. When a UI phase merges, sweep
its new `.slint` files here.

Acceptance criterion: no literal user-visible string survives outside `@tr()`.
The audit grep below catches regressions.

```sh
grep -REn '"[A-Z][a-z]+ [a-z]+' senders/android/ui/ \
  --include='*.slint' \
  | grep -v '@tr(' \
  | grep -v -- '// ' \
  | grep -v 'placeholder-text' \
  || echo "OK: every multi-word capitalised string is wrapped"
```

(The grep is heuristic. False positives — e.g. action ids — are fine; they
should look obviously non-user-facing.)

---

### 9-D — Wrap strings with context where ambiguous

For short/ambiguous strings that recur with different meanings, add a context
prefix using Slint's context syntax `@tr("ctx" => "string")`. The `|` operator
is for **plurals** (`@tr("one" | "many" % n)`) and is **not** valid for context.

```slint
@tr("cancel-cast-button"   => "Cancel")
@tr("dismiss-dialog-button" => "Cancel")
@tr("start-cast-button"     => "Start")
@tr("close-panel-button"    => "Done")
```

Reference: [translations § Context](https://github.com/slint-ui/slint/blob/master/docs/astro/src/content/docs/guide/development/translations.mdx#context).

---

### 9-E — Plural forms (where applicable)

For pluralised strings (e.g. receiver count in Phase 7's settings root), use
the plural operator:

```slint
@tr("{n} receiver" | "{n} receivers" % count)
```

- [ ] Wrap `"3 found"` in `FullSettingsPage` (Phase 7-E) once a real count
  property exists.

---

### 9-F — Generate `.pot` template

- [ ] Install the extractor (versioned independently from `slint`):

  ```sh
  cargo install slint-tr-extractor
  ```

- [ ] Create directory `senders/android/ui/i18n/` if missing.
- [ ] Run the extractor — use `find ... | xargs` (the bash `**` glob requires
  `globstar` and is non-portable across CI):

  ```sh
  find senders/android/ui -name '*.slint' \
    | xargs slint-tr-extractor -o senders/android/ui/i18n/messages.pot
  ```

- [ ] Commit `messages.pot` as the translation template.
- [ ] Add `senders/android/ui/i18n/*.po` and `senders/android/ui/i18n/*.mo` to
  `.gitignore` until actual translations are provided.

---

### 9-G — Decide English-only shipping

- [ ] Decide whether to ship English-only (just `messages.pot`, no `.po` files)
  or invite community translations. Document the decision in the repo README.
- [ ] No further work required for English-only — `@tr()` falls back to the
  literal English `msgid` when no compiled `.mo` is loaded at runtime.

---

## Exit criteria

1. All user-visible strings in `senders/android/ui/**/*.slint` are wrapped in `@tr()`.
2. The audit grep in 9-C produces no unexpected hits.
3. `senders/android/ui/i18n/messages.pot` is generated and committed.
4. `.gitignore` excludes `i18n/*.po` and `i18n/*.mo`.
5. `cargo build -p android-sender` passes — runtime falls back to English.

---

## What's NOT in this phase

- Real `.po` translations (deferred indefinitely; community contribution).
- Locale switcher in settings (no demand).
- RTL layout flipping (no demand).

---

## Slint best practices applied here

- **`@tr("ctx" => "msg")` for context, `@tr("one" | "many" % n)` for plurals.**
  Mixing these up is the #1 `@tr` mistake — the `|` operator is plurals-only.
- **No `.po` files in version control until translations exist.** Slint falls
  back to the literal `msgid` (the English string in the source) when nothing
  is loaded, so an English-only build needs no `i18n/` runtime assets.
- **The extractor crate is versioned independently.** Pin it explicitly if a
  fork divergence ever changes parsing semantics.

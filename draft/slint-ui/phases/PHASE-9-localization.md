# Phase 9 — Localization Preparation

> Wrap all user-visible strings in `@tr("...")` macros so they can be translated later.
> FCast Android sender ships English-only today — this phase lays the groundwork without
> requiring actual translation work.

**Status:** `[ ] Not started`
**Depends on:** Phase 7 (all page strings exist before wrapping them)
**Related files:**
- All `senders/android/ui/**/*.slint` files — strings wrapped here
- `senders/android/build.rs` — may need `slint_build` i18n feature flag
- `Cargo.lock` — confirm `slint-tr-extractor` version available

> **Note:** Do **not** copy `draft/moblin-ui/Common/Localizable.xcstrings` — it is
> iOS/macOS XLIFF format and is not usable by Slint's `@tr` system.

---

## Tasks

### 9-A — Confirm `@tr` support in pinned Slint version

- [ ] Check `Cargo.lock` / Slint release notes for the pinned version.
- [ ] Confirm `@tr("...")` is supported (available since Slint 1.3).
- [ ] Confirm `slint-tr-extractor` binary is available:
  `cargo install slint-tr-extractor --version <locked version>`.
- [ ] If `@tr` is not yet supported, mark this phase **deferred** and skip to Phase 10.

---

### 9-B — Wrap strings in `connect_page.slint`

- [ ] `"Connect to your receiver"` → `@tr("Connect to your receiver")`
- [ ] `"Searching for receivers…"` → `@tr("Searching for receivers…")`
- [ ] `"Scan QR"` → `@tr("Scan QR")`
- [ ] `"Migrated server tests"` heading → `@tr("Migrated server tests")`
- [ ] `"Start Migrated Server"` → `@tr("Start Migrated Server")`
- [ ] `"Test Legacy GetInfo"` → `@tr("Test Legacy GetInfo")`
- [ ] `"Test Legacy Crossfade"` → `@tr("Test Legacy Crossfade")`
- [ ] `"Test Graph Smoke"` → `@tr("Test Graph Smoke")`
- [ ] **Build check.**

---

### 9-C — Wrap strings in `connecting_page.slint`

- [ ] `"Connecting"` → `@tr("Connecting")`
- [ ] `"Cancel"` → `@tr("Cancel")`
- [ ] **Build check.**

---

### 9-D — Wrap strings in `settings_page.slint`

- [ ] `"Max resolution"` → `@tr("Max resolution")`
- [ ] `"Max framerate"` → `@tr("Max framerate")`
- [ ] `"Start"` → `@tr("Start")`
- [ ] `"Disconnect"` → `@tr("Disconnect")`
- [ ] `"Settings"` (heading) → `@tr("Settings")`
- [ ] `"Done"` → `@tr("Done")`
- [ ] `"RECEIVER"` section title → `@tr("RECEIVER")`
- [ ] `"VIDEO QUALITY"` section title → `@tr("VIDEO QUALITY")`
- [ ] `"CODEC & DEBUG"` section title → `@tr("CODEC & DEBUG")`
- [ ] `"ABOUT"` section title → `@tr("ABOUT")`
- [ ] `"Discovered receivers"` → `@tr("Discovered receivers")`
- [ ] `"mDNS discovery"` → `@tr("mDNS discovery")`
- [ ] `"H.264 encoder test"` → `@tr("H.264 encoder test")`
- [ ] `"Show debug panel"` → `@tr("Show debug panel")`
- [ ] `"App version"` → `@tr("App version")`
- [ ] `"FCast protocol"` → `@tr("FCast protocol")`
- [ ] **Build check.**

---

### 9-E — Wrap strings in `casting_page.slint`

- [ ] `"Casting"` → `@tr("Casting")`
- [ ] `"Stop Casting"` → `@tr("Stop Casting")`
- [ ] `"Waiting for media"` → `@tr("Waiting for media")`
- [ ] **Build check.**

---

### 9-F — Wrap strings in `debug_page.slint`

- [ ] `"Codec Test"` → `@tr("Codec Test")`
- [ ] Any other user-visible debug labels (keep internal identifiers like action IDs as plain strings).
- [ ] **Build check.**

---

### 9-G — Wrap strings with context where needed

For strings that are short or ambiguous, add a context hint:

```
@tr("Cancel" | "cancel-cast-button")
@tr("Start"  | "start-cast-button")
@tr("Done"   | "close-panel-button")
```

- [ ] Identify any string that appears in multiple places with different meanings.
- [ ] Add context suffixes to distinguish them for translators.

---

### 9-H — Generate `.pot` template

- [ ] After wrapping all strings, run:
  ```
  slint-tr-extractor senders/android/ui/**/*.slint -o senders/android/ui/i18n/messages.pot
  ```
- [ ] Create directory `senders/android/ui/i18n/` if it does not exist.
- [ ] Commit `messages.pot` as the translation template.
- [ ] Add `i18n/*.po` and `i18n/*.mo` to `.gitignore` until actual translations are provided.

---

### 9-I — Decide English-only shipping

- [ ] Decide whether to ship with only English (`messages.pot` template, no `.po` files).
  - Recommendation: **yes, English-only for initial release.** The `@tr` wrapping still
    provides zero-cost extraction for future translators.
- [ ] Document the decision in `senders/android/README.md` or a `ui/i18n/README.md` file.

---

## Strings to leave as-is (not localized)

| String type | Reason |
|---|---|
| Action IDs (`"scan-qr"`, `"settings"`, etc.) | Internal identifiers, never shown to user |
| Severity values (`"info"`, `"warning"`, `"error"`) | Programmatic tags, not displayed directly |
| Version strings from `env!("CARGO_PKG_VERSION")` | Set in Rust, not in `.slint` |
| Debug log output from `Bridge.test-status` | Raw technical output, not translated |
| `Panel` enum values | Internal routing, not displayed |

---

## Exit criteria

Phase 9 is complete when:

1. Every user-visible string literal in `senders/android/ui/**/*.slint` uses `@tr("...")`.
2. `messages.pot` exists in `senders/android/ui/i18n/`.
3. `cargo build -p android-sender` passes.
4. English text is visually unchanged (same strings, just wrapped).

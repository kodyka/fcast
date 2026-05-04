# Slint upstream documentation — verbatim mirror

This directory is a **verbatim copy** of the [`docs/`](https://github.com/slint-ui/slint/tree/master/docs) directory from the [`slint-ui/slint`](https://github.com/slint-ui/slint) upstream repository. It is checked in as a **read-only reference** for the FCast Slint UI port — used by phase docs, reimplement guides, and the Moblin → Slint migration notes that live one level up under `draft/slint-ui/`.

## Why is this here?

Phase docs (`draft/slint-ui/phases/PHASE-*.md`) and the reimplement guides (`PHASE-*-reimplement-instructions.md`) cite Slint upstream documentation by relative path on the `slint-ui/slint:master` branch — for example:

```text
draft/slint/docs/astro/src/content/docs/guide/language/coding/properties.mdx
```

Cloning `slint-ui/slint` into the dev environment is slow (≈1 GB on first clone, mostly examples and test fixtures unrelated to docs), so this directory ships those `.md` / `.mdx` files alongside the FCast source so a porter can `grep`, `read`, and link to them without a separate clone.

## Source

- **Upstream repo:** https://github.com/slint-ui/slint
- **Subtree imported:** [`docs/`](https://github.com/slint-ui/slint/tree/master/docs)
- **Commit pinned at import:** `d79203f422c285a7da7812a37308897d851197d4` (`d79203f`, 2026-05-02 — "Improve data URI image embedding from #9414 (#11602)")
- **Slint package version at that commit:** 1.17.0 (per `astro/package.json`)

The FCast Slint sender is currently pinned to Slint 1.15.1 (via the futo fork in `Cargo.toml`). The 1.17 docs are forward-compatible reference for the language, std-widgets, and primitives we use; any 1.16/1.17-only feature noted in the docs will be flagged inline in the consuming phase doc.

## What's mirrored

Three top-level layers, mirroring upstream verbatim:

1. **`*.md` at this directory** — `building.md`, `development.md`, `embedded-tutorials.md`, `install_qt.md`, `release-artifacts.md`, `release-notes.md`, `nightly-release-notes.md`, `testing.md`, `torizon.md`, `readme.md`. These document how to **build Slint itself** (not how to use it). They're kept for completeness but are rarely cited from FCast phase docs.
2. **`astro/`** — the Astro project that builds [docs.slint.dev](https://docs.slint.dev). The bulk of citations point inside `astro/src/content/docs/`:
   - `guide/language/coding/` — Slint language guide (properties, structs/enums, repetition, conditional elements, expressions, layouts, callbacks, globals, animations, focus, gestures).
   - `reference/elements/` — primitive elements (`rectangle.mdx`, `text.mdx`, `image.mdx`, `path.mdx`).
   - `reference/std-widgets/basic-widgets/` — `button.mdx`, `checkbox.mdx`, `slider.mdx`, `spinner.mdx`, `switch.mdx`, etc.
   - `reference/std-widgets/views/` — `listview.mdx`, `lineedit.mdx`, `scrollview.mdx`, `tabwidget.mdx`, `textedit.mdx`, etc.
   - `reference/gestures/` — `touch-area.mdx`, `pointer-event.mdx`.
3. **`common/`, `development/`, `internal/`, `nodejs/`, `safety/`, `site/`, `skills/`** — shared scripts, Astro components, dev notes, Node.js bindings docs, safety / certification notes, marketing site copy. Mirrored verbatim for completeness.

## What's NOT mirrored

- The rest of the upstream repo — `api/`, `examples/`, `tests/`, `tools/`, the Rust crates, the C++ generators, `internal/`, `slint-cpp/`, etc. None of these are docs.
- Any build artefacts (`docs/astro/dist/`, `docs/astro/.astro/`, `node_modules/`).
- The astro `_config_*.ts` / `package-lock.json` are present (committed upstream) but are **not used** here — we never run the docs site locally; we just `grep` and `read` the `.md` / `.mdx` source files.

## License

Upstream Slint is tri-licensed (GPL-3.0-only, Royalty-free, or paid commercial — see [LICENSES](https://github.com/slint-ui/slint/tree/master/LICENSES) on the upstream repo). The `.md` / `.mdx` content under `docs/` carries the same tri-license unless a per-file SPDX header overrides it. This mirror is intended for **internal reference only** during the FCast Slint port — not for redistribution, not for hosting a docs site, not for derivative documentation.

If you need to re-sync to a newer Slint release:

```sh
cd /home/ubuntu/repos/slint-upstream
git fetch && git checkout <new-tag-or-commit>
rsync -a --delete --exclude '_MIRROR.md' \
    /home/ubuntu/repos/slint-upstream/docs/ \
    /home/ubuntu/repos/fcast/draft/slint-ui/docs/
# Then bump the commit hash + Slint version above.
```

## Pre-existing FCast notes

These three files were already committed under `draft/slint-ui/docs/` before this import and are **not** part of the upstream mirror — they are FCast-authored migration aids:

- `current-fcast-slint-notes.md` — FCast-specific notes on the current Slint code under `senders/android/ui/`.
- `slint-docs-used.md` — running index of upstream docs cited from FCast phase docs.
- `swiftui-to-slint-guide.md` — Moblin SwiftUI → Slint pattern map.

Keep the `swiftui-to-slint-guide.md` and `slint-docs-used.md` updated when phase docs cite a new upstream page so the index stays useful.

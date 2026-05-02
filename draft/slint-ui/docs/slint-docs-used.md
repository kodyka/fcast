# Slint docs used

From cloned repo `draft/slint`:

- `README.md` — Slint overview and `.slint` markup concept.
- `AGENTS.md` — build/test guidance and architecture overview.
- `docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`
  - explicit vs automatic layout, logical pixels, `VerticalLayout`, `HorizontalLayout`, `GridLayout`.
- `docs/astro/src/content/docs/guide/language/coding/properties.mdx`
  - property bindings, `in`/`out`/`in-out`, change callback warnings.
- `docs/astro/src/content/docs/guide/language/coding/globals.mdx`
  - global singletons for cross-component app state and backend access.
- `docs/astro/src/content/docs/guide/language/coding/functions-and-callbacks.mdx`
  - callbacks for backend-owned logic.
- `docs/astro/src/content/docs/guide/language/coding/repetition-and-data-models.mdx`
  - `for item in model`, arrays/models.
- `docs/astro/src/content/docs/guide/language/coding/states.mdx`
  - state-driven visual changes and transitions.
- `docs/astro/src/content/docs/guide/development/best-practices.mdx`
  - separate UI/code/assets and accessibility basics.
- `docs/astro/src/content/docs/guide/development/custom-controls.mdx`
  - reusable controls and Rust callback wiring patterns.
- `docs/astro/src/content/docs/guide/development/translations.mdx`
  - `@tr(...)` translation workflow.
- `docs/astro/src/content/docs/guide/platforms/mobile/android.mdx`
  - Android uses Rust, `android_main`, and `slint::android::init`/activity setup.

Example files inspected:

- `examples/todo-mvc/ui/index.slint` — simple page navigation pattern.
- `examples/gallery/gallery.slint` — sidebar/page selection and `@tr` usage.
- `examples/gstreamer-player/scene.slint` — video/status overlay and `states` pattern.
- `examples/native-gestures/native-gestures.slint` — custom components, globals, touch/gesture handling.

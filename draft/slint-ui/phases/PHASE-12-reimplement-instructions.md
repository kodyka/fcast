# Phase 12 — Capture Preview reimplementation guide (UI-only)

**Audience:** developer applying [`draft/slint-ui/phases/PHASE-12-capture-preview.md`][spec] to the current `senders/android` tree.
**Goal:** add a reusable `CapturePreview` component (a "what's being cast" placeholder card) and embed it on the existing `CastingPage` above the status overlay. **No real frame data** — the preview shows a label + LIVE/Idle indicator until Phase 8 wires a `Bridge.preview-image: image` source.
**Scope:** Slint UI only. **No Rust changes.** Single new component file + a 2-line edit to one existing page.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-12-capture-preview.md

> **Read [`PHASE-5-reimplement-instructions.md`][p5] first** (or the live `casting_page.slint`). Phase 12 doesn't introduce a Panel — it slots a new component into the existing casting screen. The status overlay (Phase 5) must remain on top; this guide enforces the z-order via render-order in the existing `VerticalLayout` rather than absolute positioning.

[p5]: ./PHASE-5-reimplement-instructions.md

---

## Why this guide exists

Phase 12 is the smallest of the UI-only phases — one new component (~50 lines), one embed site. Two things make it worth a dedicated guide:

1. **It's the first phase that pre-declares a `Bridge` migration shape.** The component takes `mock-source-label: string` + `mock-active: bool` today, but the Phase 8 migration replaces these with a single `image-source: image` property. The shape change is a real API break — calling that out keeps consumers from over-specialising.
2. **Z-order against `StatusOverlay` is non-obvious in Slint.** Slint paints children in declaration order; later children paint on top. The casting page's existing `StatusOverlay` declaration must remain the **last** child of the casting page's outer container so it covers the new preview. This guide pins down the embed site precisely.

After Phases 5 + 6 + 7 merge:

- `senders/android/ui/pages/casting_page.slint` exists with structure: outer `Rectangle { VerticalLayout { ... title, body, StatusOverlay } }`.
- `senders/android/ui/components/` holds existing components but no `capture_preview.slint`.
- No Bridge property for capture preview.

Phase 12 adds **one new file** plus **one diff to `casting_page.slint`**.

### Audit before you start

```sh
# Should be ZERO matches before you start:
grep -rn 'CapturePreview\|capture_preview\|capture-preview' senders/android/ui/

# casting_page.slint exists and has StatusOverlay:
grep -n 'StatusOverlay\|export component CastingPage' senders/android/ui/pages/casting_page.slint
# Expected: 2+ matches (one for StatusOverlay, one for the page export).

# theme.slint has the tokens we'll need:
grep -nE 'surface-card|accent-pressed|surface-overlay|error|text-secondary|text-primary|font-size-(label|body)|radius-card|padding-screen' \
    senders/android/ui/theme.slint | head -10
# Expected: at least 6 matches.
```

After this guide is applied:

```sh
grep -rn 'export component CapturePreview' senders/android/ui/components/
# Expected: 1 match.

grep -n 'CapturePreview' senders/android/ui/pages/casting_page.slint
# Expected: 2 matches (1 import + 1 instantiation).
```

---

## Prerequisites

```sh
git fetch origin
git checkout master
git pull --ff-only
git checkout -b devin/$(date +%s)-phase-12-capture-preview
cargo check -p android-sender
```

---

## Step 1 — Create `components/capture_preview.slint`

**File:** `senders/android/ui/components/capture_preview.slint` (new)

A self-contained card with `mock-source-label` + `mock-active` properties. Two stacked elements: a low-opacity background tint and a centred label group (LIVE/Idle badge + source name). Designed so the Phase 8 migration is a one-property swap.

### New file

```slint
// capture_preview.slint — "What's being cast" placeholder card.
//
// UI-only stub. Two stub properties:
//   mock-source-label: string — what's being captured ("Screen capture")
//   mock-active:       bool   — is the capture currently live?
//
// Phase 8 migration replaces both with:
//   image-source: image       — actual frame data from MediaProjection / GStreamer
//   capture-active: bool      — derived from cast state
// and replaces the centre VerticalLayout with `Image { source: root.image-source; }`.
//
// Slint docs ref:
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx
//   draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx
//   draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx

import { Theme } from "../theme.slint";

export component CapturePreview inherits Rectangle {
    in property <string> mock-source-label: "Screen capture";
    in property <bool>   mock-active: true;

    background: Theme.surface-card;
    border-radius: Theme.radius-card;
    clip: true;

    // Background tint stand-in for the eventual frame image. Slint
    // paints children in declaration order, so this Rectangle paints
    // first (under the label).
    Rectangle {
        background: root.mock-active
                      ? Theme.accent-pressed
                      : Theme.surface-overlay;
        opacity: 0.15;
    }

    // Centred label group.
    VerticalLayout {
        alignment: center;
        spacing: 6px;

        Text {
            text: root.mock-active ? "● LIVE" : "○ Idle";
            color: root.mock-active ? Theme.error : Theme.text-secondary;
            font-size: Theme.font-size-label;
            horizontal-alignment: center;
        }

        Text {
            text: root.mock-source-label;
            color: Theme.text-primary;
            font-size: Theme.font-size-body;
            horizontal-alignment: center;
        }
    }
}
```

### Why each piece

- **`inherits Rectangle`** — `CapturePreview` **is** a card, not a thing inside one. Direct inheritance lets consumers set `width`, `height`, position, etc. as direct properties without a wrapper element. See [positioning-and-layouts.mdx][positioning].
- **`clip: true`** — required so the future `Image` element (Phase 8) doesn't bleed past the rounded corners of the card. Same trick as `InfoBanner` (Phase 27 §gotcha 48). See [rectangle.mdx][rectangle].
- **Two stacked children, no explicit z-index** — Slint draws children in declaration order; the background `Rectangle` is declared first, the label `VerticalLayout` second, so labels paint on top. Slint has no `z-index` property; ordering is the mechanism. See [positioning-and-layouts.mdx][positioning] § "Z-order: declaration order".
- **`opacity: 0.15`** on the tint Rectangle — so future bright frame data behind doesn't get fully obscured. The tint just hints at activity state.
- **`● LIVE` vs `○ Idle`** — Unicode bullet/circle glyphs (U+25CF, U+25CB). Same convention as the cast-history disclosure indicator (Phase 20).
- **`Theme.error`** for the LIVE bullet — using the existing red severity token (Phase 27 documents the eventual `Theme.success/warning/error` tokens; `Theme.error` already exists in the post-Phase-2 theme).
- **No `Image` element today** — Slint's `Image` requires an asset path or programmatic source per [image.mdx][image]. The placeholder is text-only until Phase 8 lands a real source.

### Build check

```sh
cargo check -p android-sender
slint-viewer senders/android/ui/components/capture_preview.slint  # optional standalone preview
```

---

## Step 2 — Embed in `CastingPage`

**File:** `senders/android/ui/pages/casting_page.slint`

The casting page's existing structure (post-Phase-5):

```slint
export component CastingPage inherits Rectangle {
    ...
    VerticalLayout {
        padding: Theme.padding-screen;
        spacing: Theme.spacing-default;

        Text {
            text: "Casting";   // or "@tr(...)" if Phase 9 has run
            ...
        }

        // ... existing casting body ...

        StatusOverlay { ... }     // MUST remain last so it paints on top
    }
}
```

The embed sits **between the title and the StatusOverlay**, and the StatusOverlay declaration order is preserved.

### Diff

```diff
+import { CapturePreview }   from "../components/capture_preview.slint";
 import { StatusOverlay }    from "../components/status_overlay.slint";
 ...
```

```diff
     VerticalLayout {
         padding: Theme.padding-screen;
         spacing: Theme.spacing-default;

         Text {
             text: "Casting";
             ...
         }

+        CapturePreview {
+            height: 200px;
+            mock-source-label: "Screen capture (1920×1080)";
+            mock-active: true;
+        }
+
         // ... existing casting body ...

         StatusOverlay { ... }
     }
```

### Why each piece

- **`height: 200px;`** — fixed height card. Don't bind to `parent.height * 0.3` or similar; keep the size predictable so the rest of the casting page's flow doesn't shift when the preview's rendering changes.
- **No `width:` set** — inherits the parent `VerticalLayout`'s full width minus padding. `VerticalLayout` stretches children horizontally by default (per [positioning-and-layouts.mdx][positioning]).
- **`mock-source-label: "Screen capture (1920×1080)"`** — multi-line resolution suffix. Once Phase 15 ships and `Bridge.camera-resolution` is wired (Phase 8 cluster B2), the label can compute live: `mock-source-label: "Screen capture (\{Bridge.capture-width}×\{Bridge.capture-height})"`.
- **`mock-active: true`** — for the UI-only stub, hardcode `true`. Phase 8 binds `mock-active <=> Bridge.capture-active` derived from `Bridge.app-state == AppState.Casting`.
- **StatusOverlay stays last** — Slint paints children in declaration order; if you rearrange so StatusOverlay precedes CapturePreview, the overlay disappears behind the preview. Verify with `slint-viewer` or on-device.

### Build check

```sh
cargo build -p android-sender
```

---

## Sanity grep before commit

```sh
# 1. Component exists.
grep -n 'export component CapturePreview' senders/android/ui/components/capture_preview.slint
# Expected: 1 match.

# 2. Imported and instantiated in casting_page.
grep -n 'CapturePreview' senders/android/ui/pages/casting_page.slint
# Expected: 2 matches (import + instantiation).

# 3. StatusOverlay still declared after CapturePreview (z-order invariant).
awk '/CapturePreview *{/{cp=NR} /StatusOverlay *{/{so=NR} END{
    if (cp && so && so > cp) print "OK: StatusOverlay paints on top of CapturePreview";
    else print "FAIL: z-order broken or one element missing";
}' senders/android/ui/pages/casting_page.slint

# 4. clip: true on the component (rounded corner discipline).
grep -n 'clip: true' senders/android/ui/components/capture_preview.slint
# Expected: 1 match.

cargo build -p android-sender
```

Commit:

```sh
git add senders/android/ui/components/capture_preview.slint \
        senders/android/ui/pages/casting_page.slint
git status
# Expected (2 files):
#   modified:   senders/android/ui/pages/casting_page.slint
#   new file:   senders/android/ui/components/capture_preview.slint
git commit -m "feat(slint-ui): Phase 12 — capture preview placeholder card on casting page"
```

---

## Gotchas (Phase 12 specific)

### Gotcha 51 — Slint has no `z-index`; ordering is the only mechanism

**Symptom:** the StatusOverlay disappears or flickers behind the CapturePreview.

**Cause:** rearranging `VerticalLayout`'s children so the overlay comes before the preview. Slint paints in declaration order — earlier children paint underneath later children. There is no `z-index` property on Slint elements.

**Fix:** keep `StatusOverlay { ... }` as the **last** child of the casting page's outer `VerticalLayout`. Add a comment in the file (`// MUST remain last for overlay z-order`) to deter future reorderings.

### Gotcha 52 — `clip: true` is required; without it future Image bleeds past corners

**Symptom:** when Phase 8 swaps in a real `Image { source: ... }`, the image's corners are square instead of matching `border-radius`.

**Cause:** Slint `Rectangle` doesn't clip children by default. The `Image` element renders to the rectangle's bounding box but ignores the rounded corners.

**Fix (already in this guide):** `clip: true;` on the outer `Rectangle`. The `Image`'s pixels then clip to the rounded shape. Same trick used in Phase 27's `InfoBanner` for animated height collapse.

### Gotcha 53 — `inherits Rectangle` means consumers set `height` directly

**Symptom:** the embed site does `Rectangle { ... CapturePreview { ... } }` and the preview takes zero height.

**Cause:** wrapping `CapturePreview` in another `Rectangle` adds a layer that has no explicit height. Slint's `Rectangle` defaults to zero size when not in a layout slot.

**Fix:** instantiate `CapturePreview` directly under a layout container (`VerticalLayout` / `HorizontalLayout`). Set `height` directly on the component (`CapturePreview { height: 200px; ... }`) — `inherits Rectangle` makes `height` a built-in property.

### Gotcha 54 — `mock-active: true` hard-coding masks real activity

**Symptom:** during testing, the preview always says LIVE even when not casting.

**Cause:** the stub hard-codes `mock-active: true` for design preview purposes. In a real flow, `mock-active` should derive from `Bridge.app-state`.

**Fix (UI-only build):** acceptable as-is — the design comp is "what does this look like during a cast", and the casting page only renders during a cast anyway. **Phase 8 fix:** replace with `mock-active: Bridge.app-state == AppState.Casting;` for tighter coupling.

---

## Exit criteria checklist

- [ ] `components/capture_preview.slint` exists with `export component CapturePreview`.
- [ ] Component takes `mock-source-label: string` and `mock-active: bool` properties.
- [ ] Background `Rectangle` opacity-tint is declared **before** the centre `VerticalLayout` (label paints on top of tint).
- [ ] `clip: true` is set on the outer Rectangle.
- [ ] `casting_page.slint` imports and instantiates `CapturePreview`.
- [ ] `StatusOverlay` declaration appears **after** `CapturePreview` in the casting page (paints on top).
- [ ] LIVE state shows `● LIVE` in `Theme.error` color; Idle state shows `○ Idle` in `Theme.text-secondary`.
- [ ] `mock-source-label` renders centered below the LIVE/Idle badge.
- [ ] `cargo build -p android-sender` passes.
- [ ] `slint-viewer senders/android/ui/components/capture_preview.slint` opens the component standalone (optional).

---

## When Phase 8 reactivates

```diff
 export component CapturePreview inherits Rectangle {
-    in property <string> mock-source-label: "Screen capture";
-    in property <bool>   mock-active: true;
+    in property <image>  image-source;
+    in property <bool>   capture-active;
+    in property <string> source-label: "";

     background: Theme.surface-card;
     border-radius: Theme.radius-card;
     clip: true;

-    Rectangle {
-        background: root.mock-active ? Theme.accent-pressed : Theme.surface-overlay;
-        opacity: 0.15;
-    }
+    Image {
+        source: root.image-source;
+        image-fit: cover;
+        width: 100%;
+        height: 100%;
+    }

-    VerticalLayout { ... LIVE/Idle + label ... }
+    if root.source-label != "": Rectangle {
+        // small bottom-left corner overlay with source label,
+        // matching the camera app convention
+        ...
+    }
 }
```

- `Bridge` gets `image-source: image` produced by Rust (MediaProjection → GStreamer `appsink` → Slint `image::Image`).
- `capture-active` derives from `app-state == AppState.Casting`.
- The LIVE/Idle badge becomes a small corner overlay rather than the centre label (which is no longer needed once real frames render).
- `image-fit: cover` is the canonical mode for "fill the card without distorting"; Slint supports `cover`, `contain`, `fill`, `preserve` per [image.mdx][image].

---

## Slint-doc references used

- **`inherits Rectangle`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`clip: true`** — same.
- **Painting / declaration order is z-order** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **`Image` element + `image-fit` modes (Phase 8 prep)** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx`.
- **`opacity` on `Rectangle`** — `draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx`.
- **`color` type, named colors** — `draft/slint-ui/docs/astro/src/content/docs/reference/colors-and-brushes.mdx`.
- **String interpolation `"\{n}"`** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/expressions-and-statements.mdx`.
- **`in` property declaration** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/properties.mdx`.
- **`VerticalLayout { alignment }` (centring)** — `draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx`.
- **`Theme` global tokens** — FCast file at `senders/android/ui/theme.slint` (added by Phase 2).
- **`StatusOverlay`** — FCast component in `senders/android/ui/components/status_overlay.slint` (added by Phase 5).

---

## What's NOT in this guide

- **Real MediaProjection / GStreamer frame source.** Phase 8.
- **Audio waveform / level meter.** Out of scope; would be a separate component.
- **Picture-in-picture mode.** Out of scope.
- **Tap-to-pause-preview gesture.** Out of scope; UI-only build doesn't have a "pause capture" callback.
- **Resolution / framerate overlays in the corner.** Phase 8 + Phase 15 (camera) add this once the resolution properties are Bridge-driven.
- **`@tr(...)` wrapping** of the placeholder strings. Phase 9 sweep.

[spec]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/phases/PHASE-12-capture-preview.md
[p5]: ./PHASE-5-reimplement-instructions.md
[positioning]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/guide/language/coding/positioning-and-layouts.mdx
[rectangle]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/rectangle.mdx
[image]: https://github.com/varxe-alt/fcast/blob/migrate/draft/slint-ui/docs/astro/src/content/docs/reference/elements/image.mdx

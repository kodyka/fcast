# FCast Slint UI migration draft

This folder is a planning workspace for migrating ideas from Moblin's SwiftUI UI (`draft/moblin-ui`) into FCast Android sender's Slint UI.

## Contents

- `TODO.md` — actionable migration checklist for porting Moblin UI concepts to Slint.
- `docs/swiftui-to-slint-guide.md` — concept mapping, architecture notes, Slint patterns, and risks.
- `analysis/summary.md` — generated inventory summary of the copied Moblin SwiftUI files.
- `analysis/moblin-swiftui-inventory.csv` — per-file SwiftUI pattern counts.
- `source-inventory/moblin-view-files.md` — copied list of all 279 Moblin `View/**/*.swift` files.
- `source-inventory/moblin-ui-info.md` — copied Moblin UI architecture notes from the previous draft.
- `ui/migration-skeleton.slint` — draft-only Slint skeleton showing proposed globals, components, control bar, status overlay, and settings page shape.

## Important conclusion

Moblin UI cannot be copied directly into FCast Android sender. Moblin is SwiftUI/iOS, while FCast Android sender is Rust + Slint. The correct path is to reimplement the layouts and interaction concepts in `.slint`, with Rust providing state and callbacks through Slint globals/models.

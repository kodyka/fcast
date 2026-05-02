# Draft UI research files

This directory contains research artifacts for exploring Moblin SwiftUI UI concepts and planning a Slint migration for the FCast Android sender.

## Included in this repository

- `moblin-ui/` — UI-only subset copied from `https://github.com/eerimoq/moblin`, containing `Moblin/View`, localization files, and analysis notes.
- `slint-ui/` — migration plan, generated inventory, Slint docs notes, and a draft `.slint` skeleton for reimplementing Moblin UI concepts in FCast Android sender.

## Not included

The full upstream clones used for research are intentionally not committed:

- `draft/moblin/` — full Moblin clone.
- `draft/slint/` — full Slint clone.

Those repositories are large nested Git repositories and should remain local research inputs unless vendoring is explicitly required.

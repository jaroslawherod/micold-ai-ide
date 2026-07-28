# Reference typeface provenance

**File**: `Roboto-Regular.ttf`

| | |
|---|---|
| Family / subfamily | `Roboto` / `Regular` (name IDs 1 and 2) |
| Source | <https://github.com/googlefonts/roboto-2> — `src/hinted/Roboto-Regular.ttf` |
| Retrieved | 2026-07-28 |
| Size | 515,100 bytes |
| SHA-256 | `56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d` |
| Licence | Apache License 2.0 — matches this workspace's own licence |
| Copyright | Copyright 2015 Google Inc. All Rights Reserved. |

## Why this file exists

It is the **measuring basis** for the layout snapshot (feature 019, research R2). It is *not*
registered with the application and does not affect what any user sees.

The layout snapshot builds its own headless renderer, and therefore chooses that renderer's default
font. That matters because `iced_graphics`'s global font system calls
`fontdb::Database::load_system_fonts()` unconditionally — 391 faces were counted on the development
machine — and `iced::Font::DEFAULT` is `Family::SansSerif`, which resolves through a per-platform
table. A fixture measured that way would pass only on the machine that generated it, which is
precisely what FR-006 forbids.

Pinning this face as the renderer's default makes text metrics identical on Linux, macOS and
Windows. `crates/micold-client/src/ui/material/text.rs` sets no font on body text, so it falls back
to that default and every recorded width follows from this file.

## Two things to know before touching it

**Replacing this file is a fixture-wide change.** Every recorded geometry derives from its metrics.
A different version of Roboto is a different set of advance widths, so swapping it means
regenerating `layout_snapshot.txt` in the same commit and reviewing the whole diff.

**A host font of the same name can silently displace it.** `load_system_fonts()` still runs, so a
machine with its own `Roboto` installed may win the family-name lookup and shift every measurement
at once — presenting as a mass layout regression rather than a font problem. The guard assertion in
`crates/micold-client/tests/layout_apparatus.rs` pins a known measurement of a known string so that
failure names itself. That risk does not go away when feature 018 ships Roboto as the application
font; Roboto is a common system font name, so the guard is permanent.

## Relationship to feature 018

Feature 018's FR-008/FR-008a require shipping Roboto as the *application's* typeface, so that users
see the same text everywhere. Its T015 registers it. **That task must reuse this file rather than
committing a second copy** — one asset, two consumers.

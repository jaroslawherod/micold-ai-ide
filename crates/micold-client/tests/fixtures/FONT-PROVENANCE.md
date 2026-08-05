# Reference typeface provenance

**The layout snapshot measures against the faces the application ships**, in `assets/fonts/`:

| File | Role in the snapshot |
|---|---|
| `Roboto-Regular.ttf` | body text and every type role below weight 500 |
| `Roboto-Medium.ttf` | the type roles that resolve to weight >= 500 (`material/text.rs`) |
| `MaterialSymbolsOutlined.ttf` | icons, which are glyphs and are shaped and measured like any other text |

See `assets/fonts/PROVENANCE.md` for their source, versions and licences. This file records why the
snapshot uses them and what breaks if that changes.

## This file used to describe a second copy, and that was the defect

Feature 019 originally committed its own `tests/fixtures/Roboto-Regular.ttf` as a measuring basis,
before feature 018 shipped Roboto to the application. T002 said explicitly that the asset **must not
be committed twice** and that 018's T015 should reuse it. Both features shipped, and it was.

The two files were **different builds of Roboto with different bytes** — a tenth of a pixel apart
over the guard's reference string. So the gate was measuring text against a face the application
does not draw with: reproducible, stable, and wrong. The duplicate is deleted and the snapshot reads
`assets/fonts/` directly.

## Why pinning is necessary at all

The layout snapshot builds its own headless renderer and therefore chooses that renderer's default
font. `iced_graphics`'s global font system calls `fontdb::Database::load_system_fonts()`
unconditionally — 391 faces were counted on the development machine — and `iced::Font::DEFAULT` is
`Family::SansSerif`, which resolves through a per-platform table. A fixture measured that way would
pass only on the machine that generated it, which is precisely what FR-006 forbids.

## Three things to know before touching these files

**Replacing one is a fixture-wide change.** Every recorded geometry derives from their metrics. A
different build of Roboto is a different set of advance widths, so swapping one means regenerating
`layout_snapshot.txt` in the same commit and reviewing the whole diff.

**A host font of the same name can silently displace one.** `load_system_fonts()` still runs, so a
machine with its own `Roboto` installed may win the family-name lookup and shift every measurement
at once — presenting as a mass layout regression rather than a font problem. The guards in
`crates/micold-client/tests/layout_apparatus.rs` pin known measurements so that failure names
itself. Roboto is a common system font name, so those guards are permanent.

**Every face the application registers must be registered here too.** The icon font was missed for
the whole of feature 019: icons resolved through the host's fallback, at whatever width the machine
offered. Local runs were green, and so were the macOS and Windows CI runners, because the fixture
happened to match what they resolved. Only the Ubuntu runner disagreed — icon nodes 8.4px where
6.3px was recorded, shifting every adjacent label 2.1px.

That is the failure mode to watch for: not a face that is absent, but a face that is *substituted*,
which measures something plausible and wrong. `the_icon_face_parses_and_is_the_face_that_measures`
pins each glyph to the committed face's own advance rather than to a constant, so a substitution
fails rather than being recorded.

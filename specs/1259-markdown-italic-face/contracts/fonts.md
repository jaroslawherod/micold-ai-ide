# Contract: the bundled Roboto faces, upright and italic

Module `crates/micold-material/src/roboto.rs` (new; in the `micold-material` crate since #1262
split the generic building blocks out of the client), re-exported from `micold_material` (so from
`ui::material`, which re-exports the crate whole) and from `ui` beside `FIRA_MONO_*_BYTES` and
`MATERIAL_SYMBOLS_BYTES`. The three upright byte constants and `ROBOTO` move here from
`micold-material/src/text.rs`; their public paths (`ui::material::*`, `ui::*`) stay.

`fontdb` is `iced::advanced::graphics::text::cosmic_text::fontdb`, `FontSystem` is
`iced::advanced::graphics::text::FontSystem` (no new dependency).

## C-1 Constants

```rust
pub const ROBOTO: Font = Font::with_name("Roboto");            // moved, unchanged
pub const ROBOTO_FAMILY: &str = "Roboto";
pub const ROBOTO_REGULAR_BYTES: &[u8];          // assets/fonts/Roboto-Regular.ttf (moved)
pub const ROBOTO_MEDIUM_BYTES: &[u8];           // assets/fonts/Roboto-Medium.ttf (moved)
pub const ROBOTO_SEMIBOLD_BYTES: &[u8];         // assets/fonts/Roboto-SemiBold.ttf (moved)
pub const ROBOTO_ITALIC_BYTES: &[u8];           // assets/fonts/Roboto-Italic.ttf (new)
pub const ROBOTO_MEDIUM_ITALIC_BYTES: &[u8];    // assets/fonts/Roboto-MediumItalic.ttf (new)
pub const ROBOTO_SEMIBOLD_ITALIC_BYTES: &[u8];  // assets/fonts/Roboto-SemiBoldItalic.ttf (new)
pub const ROBOTO_FACES: [&[u8]; 6];             // the six above, upright then italic
pub const ROBOTO_WEIGHTS: [u16; 3];             // [400, 500, 600], the shipped weights
```

Each italic constant parses as TrueType, is static (no `fvar`), reports typographic family
`ROBOTO_FAMILY` (name ID 16, else ID 1), the weight of its upright twin (400, 500, 600) and
`is_italic()`, and its bytes differ from every upright face. Each upright face reports
`!is_italic()`. Asserted by `tests/roboto_font.rs`.

## C-2 `pin_roboto_italics`

```rust
pub fn pin_roboto_italics(db: &mut fontdb::Database) -> usize
```

- Removes every face whose `families` contains `ROBOTO_FAMILY`, whose `source` is not
  `fontdb::Source::Binary`, and that either has a `style` other than `fontdb::Style::Normal` or a
  `weight` other than 400, 500 and 600 (the shipped weights, `ROBOTO_WEIGHTS`). Returns how many it
  removed. Research R2 and ledger D8 say why the off-weight upright faces go too.
- Leaves every bundled face, every host upright `Roboto` face at 400, 500 or 600, and every face of
  every other family.
- Idempotent: a second call returns 0.

Postconditions, given the six bundled faces are loaded as `Source::Binary`, even when the database
held a file-loaded Roboto Italic, a file-loaded upright Roboto Regular (400) and a file-loaded
Roboto Bold (700, upright) before the call:

| Query (`Family::Name("Roboto")`) | Resolves to |
|---|---|
| weight 400, italic | `ROBOTO_ITALIC_BYTES` |
| weight 500, italic | `ROBOTO_MEDIUM_ITALIC_BYTES` |
| weight 600, italic | `ROBOTO_SEMIBOLD_ITALIC_BYTES` |
| weight 700 (`Weight::Bold`), italic | `ROBOTO_SEMIBOLD_ITALIC_BYTES` |
| weight 700 (`Weight::Bold`), normal | `ROBOTO_SEMIBOLD_BYTES`, never the host Bold 700 (FR-003, SC-002) |
| weight 400, 500 or 600, normal | the face it resolved to before the call (FR-004) |

The weight-700 rows are SC-002 itself: upright bold and bold-italic resolve to faces of the same
`weight()` (600) on every host, with or without a host Roboto Bold. Each row is a `roboto.rs` unit
test on a database seeded as above. No 700 face ships, so the test makes its host Bold by writing
`ROBOTO_SEMIBOLD_BYTES` to a temp file with the OS/2 `usWeightClass` field patched to 700 (glyphs
unchanged; `fontdb` reads the weight from that field), loaded with `load_font_file` before the
bundled faces, as `mono.rs`'s host-copy test does.

## C-3 `install_roboto`

```rust
pub fn install_roboto(fonts: &mut FontSystem)
```

Loads each of `ROBOTO_FACES` with `FontSystem::load_font`, then `alias_bold`: each bundled 600 face
(upright and italic) is pushed again as a `FaceInfo` at weight 700 sharing the same bytes, once
(D10). Then calls `pin_roboto_italics(fonts.raw().db_mut())`.

Why the alias: cosmic-text 0.15 (`FontFallbackIter::default_font_match_key`) takes a face of the
requested family only when its weight equals the requested weight; otherwise the run goes to the
fallback lists, where any host face of exactly that weight and style wins. `Weight::Bold` asks for
700, which no Roboto face ships, so before the alias bold drew in DejaVu Sans Bold and bold italic
in DejaVu Sans Mono Bold Oblique (visual pass T018), on `origin/main` too for upright bold. A
fontdb `Query` (nearest weight) hid this; the resolution tests shape through cosmic-text instead
(`test_support::resolves_to_bundled`). Glue: no branch, no error path (`load_font` logs and
skips bytes it cannot parse; `tests/roboto_font.rs` fails on such a file, so it cannot ship).

Milestone note: M1 ships `install_roboto` without the pin call; the call joins in M2 (T020).

**Callers** (each exactly once, before any text is shaped), replacing the Roboto registration each
does today:

| Caller | Where | Replaces |
|---|---|---|
| the application | `shell/startup.rs`, beside `install_monospace`, before `iced::application(...)` | the three `.font(ROBOTO_*_BYTES)` calls |
| the showcase | `showcase/main.rs`, likewise | the three `.font(ROBOTO_*_BYTES)` calls |
| in-crate component tests | `micold-material/src/test_support.rs::renderer`, inside its `Once` | the three `load_font(ROBOTO_*)` calls |
| layout snapshot and apparatus tests | `tests/support/layout.rs::renderer`, inside its `Once` | the three Roboto `load_font` calls (its `REFERENCE_FONT_*` constants stay for the apparatus guard) |

## C-4 `painted` (existing, `micold-material/src/link_text.rs`)

A span with `style.italic` gets `Slant::Italic` unless `style.code`; a code span is
`Font::MONOSPACE`, upright, at bold weight when bold. Nothing else in `painted` changes.

Before `painted`, `LinkText` passes its spans through `upright_where_italic_lacks` (D11): an
emphasis span (italic, not code) is split per grapheme where Roboto's italic has no glyph
(`roboto::italic_has_glyph`, the 400 italic's cmap; the three italics cover the same characters),
and those pieces lose `italic`. cosmic-text falls back only to faces of the requested style and
hosts ship CJK upright only, so italic 漢字 drew boxes (T018 B6); upright, it falls back as upright
text does (spec Edge: coverage). Pieces keep the span's link, so a split link stays one focus stop.
Spans that need no split are borrowed unchanged.

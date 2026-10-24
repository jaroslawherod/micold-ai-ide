# Contract: the bundled Roboto faces, upright and italic

Module `crates/micold-client/src/ui/material/roboto.rs` (new), re-exported from `ui::material` and
`ui` beside `FIRA_MONO_*_BYTES` and `MATERIAL_SYMBOLS_BYTES`. The three upright byte constants and
`ROBOTO` move here from `material/text.rs`; their public paths (`ui::material::*`, `ui::*`) stay.

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
```

Each italic constant parses as TrueType, is static (no `fvar`), reports typographic family
`ROBOTO_FAMILY` (name ID 16, else ID 1), the weight of its upright twin (400, 500, 600) and
`is_italic()`, and its bytes differ from every upright face. Each upright face reports
`!is_italic()`. Asserted by `tests/roboto_font.rs`.

## C-2 `pin_roboto_italics`

```rust
pub fn pin_roboto_italics(db: &mut fontdb::Database) -> usize
```

- Removes every face whose `families` contains `ROBOTO_FAMILY`, whose `style` is not
  `fontdb::Style::Normal`, and whose `source` is not `fontdb::Source::Binary`. Returns how many it
  removed.
- Leaves every upright `Roboto` face (host or bundled) and every face of every other family.
- Idempotent: a second call returns 0.

Postconditions, given the six bundled faces are loaded as `Source::Binary`, even when the database
held a file-loaded Roboto Italic (and a file-loaded upright Roboto) before the call:

| Query (`Family::Name("Roboto")`) | Resolves to |
|---|---|
| weight 400, italic | `ROBOTO_ITALIC_BYTES` |
| weight 500, italic | `ROBOTO_MEDIUM_ITALIC_BYTES` |
| weight 600, italic | `ROBOTO_SEMIBOLD_ITALIC_BYTES` |
| weight 700 (`Weight::Bold`), italic | `ROBOTO_SEMIBOLD_ITALIC_BYTES` |
| any weight, normal | the face it resolved to before the call (FR-004) |

## C-3 `install_roboto`

```rust
pub fn install_roboto(fonts: &mut FontSystem)
```

Loads each of `ROBOTO_FACES` with `FontSystem::load_font`, then calls
`pin_roboto_italics(fonts.raw().db_mut())`. Glue: no branch, no error path (`load_font` logs and
skips bytes it cannot parse; `tests/roboto_font.rs` fails on such a file, so it cannot ship).

**Callers** (each exactly once, before any text is shaped), replacing the Roboto registration each
does today:

| Caller | Where | Replaces |
|---|---|---|
| the application | `shell/startup.rs`, beside `install_monospace`, before `iced::application(...)` | the three `.font(ROBOTO_*_BYTES)` calls |
| the showcase | `showcase/main.rs`, likewise | the three `.font(ROBOTO_*_BYTES)` calls |
| in-crate component tests | `material/test_support.rs::renderer`, inside its `Once` | the three `load_font(ROBOTO_*)` calls |
| layout snapshot and apparatus tests | `tests/support/layout.rs::renderer`, inside its `Once` | the three Roboto `load_font` calls (its `REFERENCE_FONT_*` constants stay for the apparatus guard) |

## C-4 `painted` (existing, `material/link_text.rs`)

A span with `style.italic` gets `Slant::Italic` unless `style.code`; a code span is
`Font::MONOSPACE`, upright, at bold weight when bold. Nothing else in `painted` changes.

# Contract: the bundled monospace face

Module `crates/micold-client/src/ui/material/mono.rs` (new), re-exported from `ui::material` and
`ui` beside `ROBOTO_*_BYTES` and `MATERIAL_SYMBOLS_BYTES`.

## C-1 Constants

```rust
pub const MONO_FAMILY: &str = "Fira Mono";
pub const FIRA_MONO_REGULAR_BYTES: &[u8]; // include_bytes!(assets/fonts/FiraMono-Regular.ttf)
pub const FIRA_MONO_BOLD_BYTES: &[u8];    // include_bytes!(assets/fonts/FiraMono-Bold.ttf)
```

Both byte constants parse as TrueType, report family `MONO_FAMILY` (name ID 1), weight 400 and 700,
are fixed-pitch, and advance 600 units per 1000-unit em for every printable ASCII character.

## C-2 `pin_monospace`

```rust
pub fn pin_monospace(db: &mut fontdb::Database) -> usize
```

`fontdb` is `iced::advanced::graphics::text::cosmic_text::fontdb` (no new dependency).

- Removes every face whose `families` contains `MONO_FAMILY` and whose `source` is not
  `Source::Binary`. Returns how many it removed.
- Sets the monospace family: afterwards `db.family_name(&Family::Monospace) == MONO_FAMILY`.
- Leaves every face of every other family in place.
- Idempotent: a second call returns 0 and changes nothing.

Postconditions, given the bundled faces are loaded as `Source::Binary`:

| Query (`Family::Monospace`) | Resolves to |
|---|---|
| weight 400, style normal | the Regular bundled face |
| weight 700 | the Bold bundled face |
| style italic | the bundled face of the nearest weight (upright) |

…even when the database held a face of the same family loaded from a file before the call.

## C-3 `install_monospace`

```rust
pub fn install_monospace(fonts: &mut iced::advanced::graphics::text::FontSystem)
```

Loads `FIRA_MONO_REGULAR_BYTES` and `FIRA_MONO_BOLD_BYTES` with `FontSystem::load_font`, then calls
`pin_monospace(fonts.raw().db_mut())`. Glue: no branch of its own.

**Callers** (each exactly once, before any text is shaped):

| Caller | Where |
|---|---|
| the application | `shell/startup.rs`, before `iced::application(...).run()` |
| the showcase | `showcase/main.rs`, likewise |
| in-crate component tests | `material/test_support.rs::renderer`, inside its `Once` |
| layout snapshot and apparatus tests | `tests/support/layout.rs::renderer`, inside its `Once`, replacing its own `load_font` + `set_monospace_family` of the fixture |

## C-4 Unchanged call sites

`Font::MONOSPACE` stays the way code-like text asks for the face. No call site changes for FR-002;
`terminal.rs::cell_font` is unchanged under FR-007 (research R7 has the alternative).

## C-5 Repository and bundles

- Exactly one `FiraMono-*.ttf` of each weight exists in the repository, under `assets/fonts/`.
- `assets/fonts/LICENSE-FiraMono-OFL.txt` is the verbatim upstream `LICENSE` (the committed copy
  uses LF line endings where upstream uses CRLF; text otherwise identical).
- `assets/fonts/PROVENANCE.md` has a Fira Mono section: upstream, commit, version, SHA-256 of each
  file, licence file.
- Every release bundle contains the font licence set (data-model.md) at the research R5 destination.

# Data model: Markdown italic draws in a true italic face

No runtime state, no persisted data. The only entity is the spec's **Interface face**, a
compile-time asset.

## Interface face

| Field | Values | Source of truth |
|---|---|---|
| file | `assets/fonts/Roboto-{Regular,Medium,SemiBold,Italic,MediumItalic,SemiBoldItalic}.ttf` | `assets/fonts/` |
| family | `Roboto` (typographic family, name ID 16, else ID 1) | the font's `name` table |
| weight | 400, 500, 600 | OS/2 `usWeightClass` |
| style | upright (`Normal`) or italic | OS/2 `fsSelection` italic bit |
| bytes constant | `ROBOTO_*_BYTES` | `micold-material/src/roboto.rs` (contract C-1) |
| provenance | upstream, upstream sha256, cut command, own sha256, weight, licence file | `assets/fonts/PROVENANCE.md` |

**Validation rules** (asserted by `tests/roboto_font.rs`):

- The set is closed: exactly one face per (weight, style) for weights {400, 500, 600} × styles
  {upright, italic}; six faces, listed once in `ROBOTO_FACES`.
- Each face parses, is static, belongs to family `Roboto`, and reports its weight and style.
- No two faces have equal bytes.

**Relationship**: an italic face is the twin of the upright face of the same weight (same upstream
project, same `wdth=100` cut). A markdown run's requested weight (`TypeRole` weight, or 700 for
bold) and slant select a face by the font matcher; bold resolves to the 600 twin.

**State transitions**: none. Faces are registered once per process (contract C-3) and never change.

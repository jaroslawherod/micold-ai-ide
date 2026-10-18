# Data model: Overview with larger cards, five per row

Nothing is stored. One derived value.

## Card grid layout (derived, per layout pass)

| Field | Type | Rule |
|---|---|---|
| `width` | `f32`, logical px | the width iced gives the grid (`responsive`) |
| `min` | `f32` | `MIN_WIDTH = 280.0` unless `CardGrid::min_width` sets it |
| `gap` | `f32` | `spacing::MD = 16.0` unless `CardGrid::spacing` sets it |
| `max` | `usize` | `MAX_COLUMNS = 5` unless `CardGrid::max_columns` sets it; `0` is treated as `1` |
| `columns` | `usize` | `clamp(floor((width + gap) / (min + gap)), 1, max)` |
| card width | `f32` | `(width − (columns − 1)·gap) / columns`, the same for every card, partial rows included |
| card height | `f32` | `overview_card::HEIGHT`, unchanged (feature 813) |

Validation: `1 ≤ columns ≤ 5` for every `width ≥ 0`; `columns = 5` iff `width ≥ 1464` (defaults);
card width ≥ `min` whenever `width ≥ min`.

No state transitions: the layout is recomputed from the width on every pass; a card changing state
changes only the order of the cards it is given.

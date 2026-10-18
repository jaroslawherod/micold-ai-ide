# UI contract: `CardGrid` five-column cap

Supersedes the `CardGrid` row of U3 and the column rule of U4 in
`specs/813-worktree-overview/contracts/ui.md`. Everything else there stands.

## Component API (Principle VIII)

| Component | Builder | Showcase poses |
|---|---|---|
| `CardGrid` | `new(cards, roles).min_width(280.0).spacing(..).max_columns(5)` → `Element`; defaults `MIN_WIDTH = 280.0`, `spacing::MD`, `MAX_COLUMNS = 5` | 1, 3, 30 cards; narrow (unchanged names) |

`pub fn columns(width: f32, min: f32, gap: f32, max: usize) -> usize` is the pure column rule.

## Geometry gates (in-crate tests)

- `columns = clamp(floor((w + gap) / (min + gap)), 1, max)`, `max = 0` treated as 1.
- Defaults: 1463 px → 4, 1464 px → 5, 2400 px → 5, 3840 px → 5, 279 px → 1.
- Every card has the same width `(w − (n − 1)·gap) / n`; a partial row's cards keep it and start at
  the left edge.
- At 2400 px a card is ≥ 1.5 × the width feature 813's rule gives (8 columns).
- No card overlaps another and none extends past the grid's width, from 279 px to 3840 px.
- 30 cards scroll vertically; the grid never scrolls sideways.

## Call sites (unchanged)

- `ui/overview.rs`: `CardGrid::new(cards, r)` → five per row by default.
- `showcase/sections/surfaces.rs::card_grid`: same defaults, so the gallery shows the Overview's
  layout (FR-011).

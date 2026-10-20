# Contract: Overview card layout, palette and grid (client)

## K1. Anatomy (`OverviewCard`, `ui/material/overview_card.rs`)

Top to bottom, full card width:

1. **Header row**: fill `Palette.header`, top corners radius 12, padding `spacing::SM` vertical,
   `spacing::MD` horizontal. Start: state glyph + state word (`TypeRole::Label`, `on_header`),
   tooltip = reason. End: type glyph + type word (`Label`, `on_header`), tooltip = type word.
2. **Body**: fill `Palette.body`, padding `spacing::MD`, gap `spacing::XS`, lines in this order, each
   one `Ellipsized` line:
   - title (`TypeRole::CardTitle`, `on_body`), tooltip = display name;
   - branch line when `CardText.branch` (`Body`, `on_body_variant`), tooltip = full branch;
   - sessions line (`Body`, `on_body_variant`);
   - activity line when `CardText.activity` (`Body`, `on_body_variant`), tooltip = its text;
   - progress row when `CardText.progress` (`Body`, `on_body_variant`), then the `ProgressBar` when
     it has a bar;
   - pull request row when `CardText.prs` (`Body`, `on_body_variant`), tooltip = unknown reason.
3. **Outline layer** (top of a `stack`): radius 12, `primary` border `TARGET_WIDTH` when targeted,
   else none; the *selected* state layer in `on_body` over header and body fills when targeted.

`Ripple` (colour `on_body`), `TakesTheKeyboard` (focus ring `secondary`, layer `on_body`, radius 12)
and `ContextArea` wrap the whole stack (FR-016). Height: shrink to content, accepting a larger
minimum from its limits; no fixed height.

## K2. Even rows in `CardGrid`'s `Grid` (`ui/material/card_grid.rs`)

Amended in M3: feature 970 lays every card out in one `Grid` widget that places cards on a ribbon
and cuts a card across two rows during a sidebar zoom, so no per-row widget can sit in it and no
`cdk::EvenRow` is built (T023, T028 superseded). `Grid::layout` measures then stretches (R12):

- Each card is measured at its card width with limits `(w, MIN_HEIGHT) .. (w, ∞)`.
- A row's height `H_r = max(MIN_HEIGHT, tallest measured card with a piece in row r)`; rows stack a
  gap apart, and the grid's height is `Σ H_r` plus the gaps.
- A card is laid out at exactly `w × max(H_r of its rows)`: at rest, its row's height. During a zoom a
  card cut across two rows is clipped to each row's height. `ribbon()` keeps its uniform-height
  signature (feature 970's tests).
- A missing slot in the last row stays empty; draw, events, operations and overlays are the
  `Grid`'s as before.

## K3. `CardGrid`

The column rule of features 813/1024 is unchanged; rows are even per K2, with
`overview_card::MIN_HEIGHT` as the least row height.

## K4. Geometry invariants (client tests)

- At card widths 280, 380, 480 a card with every optional row: no two text leaves overlap; every
  leaf inside the card; the header spans the card's width; the state cluster ends left of where the
  type cluster starts.
- A grid row whose cards have different rows: every card's height equals the row's tallest; a lone
  sparse card is `MIN_HEIGHT` tall.
- All five states: header fill = `Palette.header` of the state, body fill identical across states
  (pixel probe), both schemes.

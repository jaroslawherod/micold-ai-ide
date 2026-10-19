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

## K2. `cdk::EvenRow` (`ui/cdk/even_row.rs`)

`EvenRow::new(children: Vec<Element>, columns: usize).spacing(f32).min_height(f32)` → `Element`.

- Width: fills its limits. Slot width `(W − spacing·(columns − 1)) / columns`, `columns` ≥ 1;
  children beyond `columns` are a caller error (`CardGrid` chunks by `columns`).
- Height: `H = max(min_height, max natural height of the children at slot width)`; every child laid
  out at exactly `slot × H`. Fewer children than `columns` leave the trailing slots empty.
- Draw, events, operations and overlays forwarded to the children; draws nothing itself.

## K3. `CardGrid`

Rows are `EvenRow`s of `columns(width, MIN_WIDTH, spacing::MD, MAX_COLUMNS)` slots with
`min_height(overview_card::MIN_HEIGHT)`; the column rule of features 813/1024 is unchanged.

## K4. Geometry invariants (client tests)

- At card widths 280, 380, 480 a card with every optional row: no two text leaves overlap; every
  leaf inside the card; the header spans the card's width; the state cluster ends left of where the
  type cluster starts.
- A grid row whose cards have different rows: every card's height equals the row's tallest; a lone
  sparse card is `MIN_HEIGHT` tall.
- All five states: header fill = `Palette.header` of the state, body fill identical across states
  (pixel probe), both schemes.

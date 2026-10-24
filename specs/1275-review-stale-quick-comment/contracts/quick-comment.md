# Contract: drag to comment and suggestion chips (client, M4 and M5)

`features::changes::update` behaviour, tested in `crates/micold-client/tests/features_changes.rs`;
the gutter's published messages in `ui/material/diff_view.rs`'s in-crate tests. Core rules:
[core-api.md](./core-api.md) D1–D8, K1–K5.

## Drag (FR-015–FR-021)

- **Q1** `GutterPressed { extend: false }` where picking is allowed starts `view.drag` with the
  pick and composer before the press, and picks the pressed line as today. Where picking is not
  allowed (binary, non-text, not-shown large file, read-only states, a browsed file), nothing
  starts (FR-021).
- **Q2** `GutterPressed { extend: true }` extends the pick as today and starts no drag (FR-018).
- **Q3** `GutterDragOver` updates `view.pick` to `Drag::span` after every row (FR-016); rows with
  no number on the drag's side leave it (FR-017).
- **Q4** `GutterDragReleased` after a `Click` release: `view.pick` is the one line, no composer
  opens, **Add comment** is offered (FR-018, US2 AS4).
- **Q5** `GutterDragReleased` after a `Range` release with no composer open: `view.pick` is the
  range, a `New` composer opens with empty text under the released line, and the shell focuses its
  text area (FR-015, AS1).
- **Q6** Same with a `New` composer open: the composer moves to the range and keeps its text
  (FR-019, AS5).
- **Q7** Same with an `Edit` or `Answer` composer open: the pick changes, the composer and its text
  stay (FR-019, Edge *Edit box open*).
- **Q8** `GutterDragCancelled` (**Esc**): `view.pick` and `view.composer` equal what they were
  before the press; `view.drag` is `None` (FR-020, AS6).
- **Q9** A release outside the gutter or the window ends the drag at the last line passed (the
  shell's window-wide listener sends `GutterDragReleased`); the diff does not scroll (Edge *Drag
  leaves the diff*).

## Chips (FR-023–FR-027)

- **Q10** `chips_shown` is false with the setting off, whatever composer is open (FR-023, AS1,
  SC-004); with it on, true for `New`, `Edit` and `Answer` composers (AS2).
- **Q11** `ChipPressed(i)` sets `composer.text = chips::insert(text, &CHIPS[i])`; Save becomes
  available by the existing blank-text rule; the shell syncs the editor, moves the cursor to the end
  and focuses the text area (FR-024, FR-025, AS3, AS4). With no composer or the setting off, it does
  nothing.
- **Q12** `SuggestionChipsInForce(b)` sets `state.suggestion_chips`; an open composer follows from
  its next draw, and boxes opened later follow it (FR-026, AS5).

## Glue checked by the §B pass

The chips follow the text area in keyboard order and show the focus ring (FR-027); the drag works
with a mouse and with a touchpad press-and-drag (Edge *Cross-platform*).

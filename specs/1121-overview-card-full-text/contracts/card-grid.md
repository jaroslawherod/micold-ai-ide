# Contract: the card and the grid (`micold-client`, `ui/material`)

## U1 — `OverviewCard` builders (added)

| Builder | Effect |
|---|---|
| `.expanded(bool)` | the expanded form (research R3); default collapsed |
| `.on_keyboard_focus(f: Fn(bool) -> M)` | passed to `TakesTheKeyboard::on_shown_change` (added): published when `focused && visible` changes, i.e. keyboard focus only. A pointer press takes the keyboard without showing it, so it reports nothing; a press on a keyboard-focused card reports `false`. Client test: press on a card, move away → nothing expanded |
| `.on_rest(m)`, `.on_leave(m)` | read by `CardGrid` (U3), not by the card |
| `.on_page(f: Fn(bool) -> M)` | claims Page Down / Page Up while focused; set only when `page_target` is `Some` |
| `.scroll_id(id)`, `.on_body_scroll(f)` | the expanded body's scrollable and its edge report |

Collapsed: title `LineClamped` 3 lines at `CardTitle`, activity `LineClamped` 2 lines at `Body`,
branch `Ellipsized`; no tooltip on title, branch or activity; header tooltips kept. Expanded: as
R3; every line wraps with `WordOrGlyph`, none ellipsized. Both forms: the focus ring, ripple and
target mark cover the card as drawn (FR-011).

## U2 — `CardGrid::expanded(index, twin: OverviewCard)`

- Row heights and every card's slot are equal to the same grid with nothing expanded (FR-009).
- The expanded card's node: slot x and width; height `min(natural, last view height)`.

## U3 — Draw and events

- Drawn after every other card, in its own layer, at `place(...)` (K3) from the last viewport.
- Events and `mouse_interaction` go to the expanded card first; while the cursor is inside its drawn
  rectangle every other card sees `cursor.levitate()` (FR-010).
- One `ShowTimer` for the topmost card under the cursor; `on_rest` after `EXPAND_REST`, `on_leave`
  when the cursor leaves the card it was over; a redraw is requested at `wake_at`. When the topmost
  card under the cursor changes (even with no frame outside every card, e.g. from the expanded
  card's drawn area straight onto the neighbour beneath), the grid publishes the old card's
  `on_leave` and calls `ShowTimer::reset()`. Test: rest A, move straight onto B → B rests. None of
  this while a zoom runs.
- The collapsed twin is built with no `.id`, focus wiring or messages and kept in its own tree
  slot after the cards; `operate`, `overlay`, `update`, `mouse_interaction` and `draw` skip it,
  only `layout` measures it.

## U4 — Wiring (`ui/overview.rs`, `features/overview.rs`, `app.rs`, `main.rs`)

- Each card: `.expanded(expansion.expanded() == Some(location))`,
  `.on_keyboard_focus(|f| Msg::Expansion(FocusGained/FocusLost(location)))`,
  `.on_rest(Msg::Expansion(PointerRested(location)))`, `.on_leave(...PointerLeft(location))`.
- `CardMenuOpened` → `MenuOpened`; `CardMenuDismissed`/`EntryChosen` → `MenuClosed`;
  `settle_card_menu`: `!open` → `Expansion::default()` (covers `Closed`, `Toggled`, `CardActivated`;
  app test: activate a card, reopen → none expanded, FR-013), else `Listed`.
- `FullScreenToggled` sends nothing: the expansion stays as in the normal Overview (FR-011).
- `on_escape`: expanded → `Msg::Expansion(Escape)`, else `Msg::Closed` (D1, R7).
- `WindowFocusChanged(false)` → `Msg::Expansion(WindowBlurred)`.
- `Msg::CardPaged { down }` → `scrollable::scroll_by(card_scroll_id, ±one view height)`.

# Contract: the card and the grid (`micold-client`, `ui/material`)

## U1 — `OverviewCard` builders (added)

| Builder | Effect |
|---|---|
| `.expanded(bool)` | the expanded form (research R3); default collapsed |
| `.focused(bool)`, `.on_focus_change(f)` | passed to `TakesTheKeyboard`: the app tracks the focused card |
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
  when the cursor leaves the card it was over; a redraw is requested at `wake_at`. None of this
  while a zoom runs.

## U4 — Wiring (`ui/overview.rs`, `features/overview.rs`, `app.rs`, `main.rs`)

- Each card: `.expanded(expansion.expanded() == Some(location))`, `.focused(expansion.focused ==
  location)`, `.on_focus_change(|f| Msg::Expansion(FocusGained/FocusLost(location)))`,
  `.on_rest(Msg::Expansion(PointerRested(location)))`, `.on_leave(...PointerLeft(location))`.
- `CardMenuOpened` → `MenuOpened`; `CardMenuDismissed`/`EntryChosen` → `MenuClosed`;
  `Closed`/`Toggled` (closing) → `Expansion::default()`; `settle_card_menu` → `Listed`.
- `on_escape`: expanded → `Msg::Expansion(Escape)`, else `Msg::Closed` (D1, R7).
- `WindowFocusChanged(false)` → `Msg::Expansion(WindowBlurred)`.
- `Msg::CardPaged { down }` → `scrollable::scroll_by(card_scroll_id, ±one view height)`.

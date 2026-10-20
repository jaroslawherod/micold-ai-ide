# Contract: core expansion rules and expanded text (`micold-core`)

## K1 — `Expansion::apply(&mut self, Event)`, `expanded() -> Option<&SessionLocation>`, `cause()`

Transitions as in [data-model.md](../data-model.md#events-and-transitions). Each spec scenario and
Edge Case is a test:

- Rest on A → A expanded (Pointer). Leave A → none.
- Rest A then rest B → B only (Story 2 sc.4, FR-006).
- Focus A → A (Focus); focus B → B, A collapsed (Story 3 sc.1).
- Focus A, rest B, leave B → A again; focus A, Escape, rest B, leave B → none (FR-005, Edge Case).
- Focus A, rest B, window blurred → none; nothing re-expands (Edge Case *Window regains focus*).
- Escape with A focus-expanded → none, `focused` still A; focus B and back to A → A (Story 3 sc.5).
- Rest A, Escape → none; further events with the pointer still on A → none; leave A and rest A → A.
- Focus A, rest A (cause becomes Pointer), Escape, rest B, leave B → none: A was escaped after it
  last got focus, whatever the cause it was expanded by (FR-005, Edge Case *Hover ends while another
  card has focus*).
- Menu open: rest B / focus B → no change; menu closed → rests and focus act again (FR-012).
- `Listed` without A while A expanded → none; focus hand-back `FocusGained(first)` → first (Edge Case).
- `Default` (fresh) → none (FR-013).

## K2 — `EXPAND_REST: Duration`

400 ms; a test asserts `<= 500 ms` (SC-006).

## K3 — `place(slot: (top, height), natural: f32, view: (top, height)) -> Placement`

- `height = min(natural, view.height)`, `scrolls = natural > view.height`.
- `top = slot.top` when `slot.top + height <= view.bottom`; otherwise
  `max(view.top, view.bottom - height)`, never greater than `slot.top`.
- The result covers the slot: `top <= slot.top` and `top + height >= slot.top + slot.height`
  whenever `natural >= slot.height` (expanded is never shorter than collapsed) and the slot ends
  inside the view; a slot ending below the view cannot be both covered and seen whole, and FR-008's
  shift wins.

## K4 — `page_target(&Expansion, down: bool) -> Option<PageTarget>`

`Some(Card)` only when a card is expanded with cause Focus and `body_edges` allows the direction;
else `None` (the key is not claimed, behaves as today).

## K5 — `ExpandedText::of(&CardModel, home: Option<&Path>) -> ExpandedText`

- `title`: the card title in full.
- `display_name`: `Some` only when it differs from the title.
- `branch`: the full branch, `Some` only when there is one and the title is not the branch.
- `dir`: `worktree_dir(&model.dir, home)`.
- `activity`: the whole activity text (`activity_text`, sanitised, not clipped to a line).

## K6 — `worktree_dir(path: &Path, home: Option<&Path>) -> String` (D2)

- `home/code/app/.worktrees/x` → `~/code/app/.worktrees/x` (platform separator after `~`).
- `home` itself → `~`. Outside `home`, or `home` `None` → the path in full.
- A path that only shares a string prefix (`/home/al` vs `/home/alice`) is not shortened.

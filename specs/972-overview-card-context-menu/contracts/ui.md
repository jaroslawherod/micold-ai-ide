# UI contract: Overview Card Context Menu

**Feature**: 972 | **Plan**: [../plan.md](../plan.md) | **Data model**: [../data-model.md](../data-model.md)

## C1 — Entries (FR-002, SC-002)

`ui::overview::card_menu_items(state, &location)` returns *Open* followed by
`ui::location_menu_items(state, &location)`, every message wrapped in
`Msg::CardMenuEntryChosen`. Labels and icons after *Open* equal the row menu's, in order, for:
Default; a worktree (plain, included, with a pull request, with one session, with no daemon);
an orphan / invalid folder. The worktree row menu is built by the same `location_menu_items`.

## C2 — Shared component APIs (Principle VIII)

| Component | New builder | Behaviour |
|---|---|---|
| `OverviewCard` | `.on_secondary_press(Fn((u16,u16)) -> M)` | secondary press over the card publishes with the window-pixel press point (`ContextArea`) |
| `OverviewCard` | `.on_menu_key(M)` | while focused, Menu key and Shift+F10 publish `M` |
| `OverviewCard` | `.targeted(bool)` | draws the target mark (R7); drops the card's Enter/Space/menu-key claims |
| `CardGrid` | `.on_scroll(M)` | the grid's scrollable publishes `M` on scroll |
| `MenuOverlay` | `.highlighted(Option<usize>)` | draws that item with the focus state layer and ring |
| `TakesTheKeyboard` | `.chord(Named, Modifiers, M)` | a key claimed only with exactly these modifiers |

## C3 — Keys

| State | Key | Result |
|---|---|---|
| Card focused, no menu | Menu, Shift+F10 | `CardMenuRequested(location)` → menu opens inside the card, first entry highlighted |
| Card focused, no menu | F10 alone, Shift+other | unchanged (not claimed) |
| No card focused | Menu, Shift+F10 | nothing |
| Card menu open | Up / Down | highlight moves, wrapping; card focus does not move |
| Card menu open | Enter / Space | highlighted entry runs; nothing if none highlighted |
| Card menu open | Escape | menu closes; Overview stays; keyboard origin → focus to target card |

## C4 — Geometry and look (gates and visual pass)

- The menu panel stays inside the window for anchors at every window edge and at the minimum
  window size (`clamp_menu_anchor`, `tests/gates/context_menu_anchor.rs`).
- A keyboard-opened anchor lies inside the target card's bounds.
- The target mark is visibly different from rest, hover and focus in light and dark (a pixel test
  in `overview_card.rs` like `the_focused_card_draws_the_focus_ring`; contrast of `primary` on the
  card fills in `composition_contrast.rs`).

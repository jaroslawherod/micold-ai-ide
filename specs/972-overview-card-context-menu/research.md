# Research: Overview Card Context Menu

**Feature**: 972 | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

Each decision records what was chosen, why, and what was rejected. Paths are relative to
`crates/micold-client/src/` unless they say otherwise.

## R1 — One item builder for the row menu and the card menu (FR-002, FR-004, SC-002)

- **Decision**: Extract the body of the `worktree_menu` closure in `ui/mod.rs` (today lines
  ~549–637: the Default branch, `worktree_menu_items`, `additional_session_item`,
  `single_session_items`, *Bind to daemon...*) into
  `pub fn location_menu_items(state: &State, location: &SessionLocation) -> Vec<material::MenuItem<Message>>`.
  The worktree row menu calls it with `SessionLocation::Worktree(dir)` (or `Default` for `""`); the
  card menu is `[Open] ++ location_menu_items(state, &target)` with each item's message wrapped
  (R4).
- **Rationale**: The spec's assumption "the card menu reuses the row menus so the two cannot drift"
  becomes structural: one function, two callers. SC-002 then holds by construction and a test
  compares labels from both callers for every location type.
- **Alternatives rejected**: a second list built for cards (drifts, fails SC-002 the first time a
  row entry is added); building the card menu by opening the real row menu (`worktree.menu_open`)
  with a flag (the row menu's state, anchor rules and displacement are the sidebar's; the target
  mark and keyboard origin would leak into the worktree feature).

## R2 — Card menu state lives in `features::overview` and registers as a floating surface (FR-012, FR-014)

- **Decision**: `features::overview::State` gains `card_menu: Option<CardMenu>` (data-model). A new
  surface `OverviewCardMenu` (`floating_surface!`, `layer: ContextMenu`,
  `cancelled_by: Message::Overview(Msg::CardMenuDismissed)`) is registered in
  `overlay/registry.rs`. It displaces `ProjectContextMenu`, `WorktreeContextMenu`,
  `GroupContextMenu`, `SessionContextMenu` and `SessionStartMenu`, and each of those four row menus
  plus the start menu displaces it. Opening returns `surface_opened(.., OverviewCardMenu::ID)`
  like `worktree::menu_toggled`.
- **Rationale**: Escape (`registry::escape`), scroll-beneath (`registry::scroll_beneath`),
  `clear_for_dialog` and displacement already work for every registered surface; registering is
  how the card menu gets FR-007, FR-012 (scroll) and FR-014 without new dismissal code. The
  terminal's two menus are not listed: the terminal is not on screen while the Overview is.
- **Alternatives rejected**: an unregistered menu drawn by the Overview view (Escape and scroll
  would need their own paths, the defect feature 021 removed); a field on `worktree::State` (the
  Overview is not the worktree feature's).

## R3 — Pointer opening and dismissal (FR-001, FR-008)

- **Decision**: `OverviewCard` gains `.on_secondary_press(impl Fn((u16, u16)) -> M)`, implemented
  with the existing `cdk::context_area::ContextArea`, which reports the press point in window
  pixels. The view passes `Msg::CardMenuOpened { location, anchor, origin: Pointer }`. The anchor
  is clamped at render time with `features::project::clamp_menu_anchor` and
  `material::menu_panel_size(items.len())`, as every row menu does.
  - A primary click outside: the surface's invisible backdrop already publishes the dismissal and
    swallows the press, so nothing beneath activates.
  - A secondary click outside: the backdrop's `mouse_area` does not catch it today, so a
    right-click on card B already reaches B's `ContextArea` and opens B's menu (which replaces A's
    state). For a secondary click on empty Overview space, the Overview's container is wrapped in a
    `ContextArea` whose secondary press publishes `CardMenuDismissed`; a press over a card is
    captured by the card first. A secondary click on a sidebar row opens that row's menu, which
    displaces the card menu (R2).
- **Rationale**: reuses the one press-point primitive (BUG-008's lesson) and the backdrop as is; no
  change to the shared backdrop's behaviour for other menus (spec Out of Scope: row menus
  unchanged).
- **Alternatives rejected**: a backdrop that catches secondary presses for every menu (changes row
  menus, and would swallow the right-click on card B, breaking US2.4); `mouse_area::on_right_press`
  (drops the press point).

## R4 — Running an entry: wrap, then dispatch the row menu's own message (FR-003–FR-005, FR-011)

- **Decision**: Every card-menu item's message `m` becomes
  `Message::Overview(Msg::CardMenuEntryChosen(Box::new(m)))`. *Open*'s inner message is
  `Message::Overview(Msg::CardActivated(card.activation))`, the card's own activation. The reducer
  closes the menu and, when it was opened from the keyboard, arms `focus_return` with the target;
  `main.rs::overview_message` then sends the inner message through `update_inner`, the same path a
  row-menu pick takes (shell effects such as opening the pull request page included).
  Whether the Overview closes is decided by the entry's existing handler, not by the card menu:
  `Outcome::ChangesRequested` and `Outcome::MainAreaTaken` (session selected or started) and
  `CardActivated` already close it; *Rename*, *Delete*, *Copy name*, *Stop showing*,
  *Open pull request* and *Bind to daemon...* do not, and their dialogs draw over whatever main
  area is shown.
- **Rationale**: FR-004 ("same effect as the row menu, including any confirmation") holds because
  it is the same message; FR-005's navigating/in-place split is the existing handlers' and cannot
  drift from the row menu's.
- **Alternatives rejected**: a navigating/in-place table in the card menu (a second statement of
  what each handler already does); closing the card menu in each entry's handler (touches every
  row-menu handler and the row menus are out of scope).

## R5 — Keyboard opening: the card claims Menu and Shift+F10 (FR-009, Edge: keys elsewhere)

- **Decision**: `TakesTheKeyboard` (`ui/material/keyboard_focus.rs`) gains
  `.chord(key: Named, modifiers: Modifiers, message: M)` beside `.key(..)` (data, inspectable, as
  `.key` is); the card claims `Named::ContextMenu` (no modifiers) and `Named::F10` with Shift,
  sending `Msg::CardMenuRequested(location)`. `main.rs` answers it with the existing
  `ui::focused_bounds_task()` (the 813 `OfferStart` chain) and opens the menu with
  `origin: Keyboard` at the card's top-left inset by `spacing::MD` on both axes — inside the card
  (US3.1), clamped like R3. Focus traversal is unchanged, so the keys are claimed only while a card
  holds the keyboard: outside the Overview nothing changes, and the terminal is not shown while the
  Overview is.
- **Rationale**: the keys reach the focused card through the widget that already answers its Enter
  and Space; no global listener has to know which card is focused.
- **Alternatives rejected**: a global key listener plus a "which card is focused" query (no such
  query exists; per-card ids alone do not say which one holds focus); a closure receiving the
  card's bounds inside `TakesTheKeyboard` (breaks its data-not-closure key set, see its doc).

## R6 — Keyboard navigation inside the menu (FR-010, US3.2, US3.4)

- **Decision**: The menu primitive gains a highlighted entry: `MenuOverlay::highlighted(Option<usize>)`
  draws that item with §5's focus state layer and ring (the same `Indicator` colours
  `TakesTheKeyboard` uses), so any menu can adopt it later. The card menu state holds
  `highlighted: Option<usize>` (`Some(0)` when opened from the keyboard, `None` from the pointer).
  While a card menu is open, `ui::subscription` swaps the Overview arrow listener for a card-menu
  key listener (named `fn`, the `TypeId` rule): Up/Down → `Msg::CardMenuMoved { forward }`
  (wrapping, pure in the reducer over `items_len`), Enter/Space → `Msg::CardMenuActivated`, which
  `main.rs` resolves with `ui::overview::card_menu_entry(state)` (the highlighted item's wrapped
  message) and dispatches as R4. While the menu is open the cards are built without their
  Enter/Space/menu-key claims so the focused card does not also activate (`OverviewCard::targeted`,
  R7, drops them).
- **Rationale**: iced 0.14 gives buttons no focus; a state-held highlight is how the picker and
  type-ahead already do list navigation, and keeps the reducer testable.
- **Alternatives rejected**: making each menu item a `TakesTheKeyboard` and moving widget focus
  (focus traversal would leave the panel and wrap through the whole window); a card-menu-only
  panel (Principle VIII: the highlight belongs in the shared menu).

## R7 — The target mark (FR-006, US2.1)

- **Decision**: `OverviewCard::targeted(bool)`. A targeted card draws a 2dp outline in the
  `primary` role around its edge **and** the `state::SELECTED` (12 %) layer of its content colour
  over its fill — Material's *selected* state. It is distinct from the focus ring (`secondary`,
  inset, `FOCUS` layer) and from hover (`HOVER` 8 %, no outline) by shape as well as colour: an
  outline with a filled layer, versus a ring alone or a layer alone. The view sets it from
  `card_menu.location == model.location`, so it follows the location when cards reorder.
- **Rationale**: reuses existing state tokens (`micold_core::tokens::state`) and roles; one more
  builder flag on the shared card.
- **Alternatives rejected**: elevation (cards sit on `surface_container_low` with no shadow
  vocabulary in the grid; a lifted card reads as dragged); a badge glyph (needs room in a header
  that already ellipsizes).

## R8 — Closing when the target goes or the Overview closes (FR-012, Edge cases)

- **Decision**: At the end of the outermost `app::State::update` (beside
  `agent_confirm::release`), `ui::overview::settle_card_menu(state)` clears `card_menu` (and any
  `focus_return`) when the Overview is closed, or when no card in `ui::overview::cards(state)` has
  the target location. Scrolling: `CardGrid` gains `.on_scroll(M)`; the Overview passes
  `Msg::GridScrolled`, which the root answers with `State::dismiss_on_scroll_beneath()` — the
  shared rule (feature 017 FR-009).
- **Rationale**: one place covers every way a location disappears (another window, a delete, a
  project switch) and every way the Overview closes (`Closed`, `Toggled`, `MainAreaTaken`,
  `ChangesRequested`), without each of those arms remembering the menu.
- **Alternatives rejected**: clearing in each closing arm (five arms, the next one forgets);
  hiding a stale menu only in the view (state would still route Enter to a gone location).

## R9 — Focus return (FR-011, US3.3)

- **Decision**: Cards get stable ids `ui::overview::card_id(&SessionLocation)` (the
  `start_control_id` pattern); `FIRST_CARD_ID` is replaced by the first card's `card_id`.
  `overview::State::focus_return: Option<SessionLocation>` is armed when a keyboard-opened menu
  closes (Escape, outside click) or runs an entry. `main.rs` drains it after each message, like
  `tab_reveal_scroll`: only while the Overview is open and no dialog is open
  (`registry::open_dialog(..).is_none()`), it focuses the target's `card_id` if that card is still
  listed, else the first card's (decided by the tested `ui::overview::focus_return_target`).
- **Rationale**: one drain covers Escape, an in-place entry, and a dialog that closes later;
  waiting on "no dialog" is what makes the rename dialog's cancel return to the card.
- **Alternatives rejected**: refocusing in each handler (dialog close happens in other features).

## R10 — The *Open* entry's glyph

- **Decision**: a new `Icon::Open` on Material Symbols `login` (enter), pinned in
  `tests/icons.rs` / `tests/icons_font.rs` and listed in `docs/user-guide/icons.md`.
- **Alternatives rejected**: `OpenInBrowser` (says a browser opens); no icon (the row-menu entries
  all have one, so *Open*'s label would sit out of line).

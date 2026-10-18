---

description: "Task list for feature 972: Overview Card Context Menu"
---

# Tasks: Overview Card Context Menu

**Input**: Design documents from `specs/972-overview-card-context-menu/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/ui.md](./contracts/ui.md),
[quickstart.md](./quickstart.md)

**Tests**: Constitution Principle I (Test-First, NON-NEGOTIABLE): every test task is written first
and seen failing for the right reason before the implementation task after it. Glue is exercised
by quickstart §B only: the `ui/overview.rs` view wiring, the card-menu surface push in
`ui/mod.rs::view_with`, the `ui/mod.rs` key-listener swap, and the `main.rs` chains (keyboard open
via focused bounds, entry dispatch through `update_inner`, the focus-return drain). Every decision
those chains take is a tested function (`card_menu_items`, `card_menu_entry`, `settle_card_menu`,
`focus_return_target`, `keyboard_anchor`).

**Documentation**: Principle VII: each story's user-guide task is in that story's phase and
milestone (CI's user-guide gate needs it in the same `feat` PR).

**Cross-platform**: Principle VI: the secondary press is iced's (macOS Ctrl+click included);
Shift+F10 and `Named::ContextMenu` are platform-neutral; no OS branch; CI runs Linux, macOS and
Windows.

**Paths**: `crates/micold-client/` prefix omitted for `src/` and `tests/`; `docs/` is the
repository's.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US3)

---

## Phase 1: Setup

No setup: the client crate, its test layout and the overlay registry exist (feature 813, 017).

---

## Phase 2: Foundational (blocks every story)

**Purpose**: the *Open* glyph and the one item builder the row menu and the card menu share (R1,
R10).

- [X] T001 [P] Test first: in tests/icons.rs pin `Icon::Open` to Material Symbols `login`'s
  codepoint and add it to the distinctness list; in tests/icons_font.rs assert the glyph is in
  `assets/fonts/MaterialSymbolsOutlined.ttf`; both fail (variant missing)
- [X] T002 Add `Icon::Open` (`login`) in src/icons.rs and its row (*Open*: the card menu's first
  entry) in docs/user-guide/icons.md; T001 passes
- [X] T003 [P] Extract the body of the `worktree_menu` closure in src/ui/mod.rs (today ~548–637:
  Default branch, `worktree_menu_items`, `additional_session_item`, `single_session_items`,
  *Bind to daemon...*) into `pub fn location_menu_items(state: &State, location: &SessionLocation)
  -> Vec<material::MenuItem<Message>>`; the closure calls it with `SessionLocation::Default` for
  the project root and `SessionLocation::Worktree(dir)` otherwise; `worktree_menu_items` stays as
  its inner part. Behaviour-preserving refactor: tests/worktree_menu_pull_request.rs and every
  row-menu test stay green unchanged

**Checkpoint**: the row menu is built by one public function; `Icon::Open` exists.

---

## Phase 3: User Story 1 — Act on a location from its card with the pointer (P1) 🎯 MVP

**Goal**: a right-click on a card opens a menu at the pointer: *Open*, then exactly the location's
row-menu entries; choosing one does what the row menu (or activation, for *Open*) does; the menu
is a registered `ContextMenu` surface, so Escape, a primary click outside, displacement and a grid
scroll close it, and it closes when its target goes or the Overview closes.

**Independent Test**: spec US1's: open the Overview, right-click a worktree card, choose *Rename*:
the rename dialog opens over the Overview, and after renaming the card shows the new name.

### Tests for US1 (write first)

- [X] T004 [US1] New tests/overview_card_menu.rs, reducer part (data-model `Msg` table):
  `CardMenuOpened { location, anchor, origin: Pointer }` sets `card_menu` with that `location`,
  `anchor`, `origin` and `highlighted == None`, and the outcome is
  `SurfaceOpened(OverviewCardMenu::ID)`; with `origin: Keyboard` it sets `highlighted == Some(0)`;
  opening on B while A's menu is open replaces it with B's (US2.4); `CardMenuDismissed` and
  `CardMenuEntryChosen(inner)` set `card_menu = None`, and set `focus_return = Some(target)` for a
  `Keyboard` origin and leave it `None` for `Pointer` (FR-011, state half; M3 reaches it); `GridScrolled` through `app::State::update` closes it (FR-012); an in-place
  entry dispatched as the inner message (`RenameStarted`, `DeleteRequested`) leaves
  `overview.open == true` and opens its dialog; *Review changes* (`ChangesRequested`) and a session
  select (`MainAreaTaken`) close the Overview (FR-005); `CardActivated(activation)` as *Open*'s
  inner message closes it as a primary click does (FR-003). Fails (types missing)
- [X] T005 [US1] tests/overview_card_menu.rs, composition part: `ui::overview::card_menu_items(state,
  &location)` is *Open* (`Icon::Open`) followed by items whose labels and icons equal
  `ui::location_menu_items(state, &location)`, in order, for Default, a plain worktree, an included
  worktree, a worktree with a pull request, one with a session, one with no daemon, and an orphan /
  invalid folder (FR-002, SC-002); every item's message is
  `Message::Overview(Msg::CardMenuEntryChosen(_))` and *Open*'s inner message is
  `Message::Overview(Msg::CardActivated(card.activation))`; `settle_card_menu` clears `card_menu`
  and `focus_return` when the Overview is closed (`Closed`, `MainAreaTaken`, `ChangesRequested`,
  project switch) or the target is no longer in `ui::overview::cards(state)`, and keeps it when the
  target's state changed or the cards reordered (FR-012, Edge cases). Fails (functions missing).
  Same file as T004: after it, not in parallel
- [X] T006 [P] [US1] tests/popover_displacement.rs: the ordered pairs `OverviewCardMenu` ↔
  `ProjectContextMenu`, `WorktreeContextMenu`, `GroupContextMenu`, `SessionContextMenu`,
  `SessionStartMenu` both ways (R2, FR-014); tests/overlay_registry.rs: Escape reaches the card
  menu as topmost and `scroll_beneath` dismisses it; tests/overlay_registration.rs: the surface is
  registered with layer `ContextMenu` and `cancelled_by` `Message::Overview(Msg::CardMenuDismissed)`.
  Fail (surface missing)
- [X] T007 [P] [US1] Tests in src/ui/material/overview_card.rs: with
  `.on_secondary_press(f)` a secondary press over the card publishes `f(point)` with the
  window-pixel press point; a primary press still publishes `on_press` only (FR-013); without the
  builder a secondary press publishes nothing. Tests in src/ui/material/card_grid.rs: with
  `.on_scroll(m)` a wheel scroll over the grid publishes `m`. Fail (builders missing)
- [X] T008 [P] [US1] tests/gates/context_menu_anchor.rs (run by the `layout_snapshot` target): the card menu's panel
  (`menu_panel_size(card_menu_items(..).len())`) clamped with `clamp_menu_anchor` stays inside the
  window for anchors at the right and bottom edges and at the minimum window size (FR-001, Edge:
  window edge, small window); fails until `card_menu_items` exists

### Implementation for US1

- [X] T009 [US1] src/features/overview.rs: `MenuOrigin { Pointer, Keyboard }`, `CardMenu
  { location: SessionLocation, anchor: (u16, u16), origin: MenuOrigin, highlighted: Option<usize> }`,
  `State::{card_menu: Option<CardMenu>, focus_return: Option<SessionLocation>}`, `Msg::{CardMenuOpened
  { location, anchor, origin }, CardMenuDismissed, CardMenuEntryChosen(Box<Message>), GridScrolled}`
  with the data-model's effects (`highlighted = Some(0)` iff `Keyboard`; dismissal and entry arm
  `focus_return` iff `Keyboard`; nothing opens a `Keyboard` menu until M3), and `floating_surface!(OverviewCardMenu, id:
  SurfaceId::new("overview_card_menu"), layer: ContextMenu, cancelled_by:
  Message::Overview(Msg::CardMenuDismissed))` with `Registered` (open iff `card_menu.is_some()`),
  as `ProjectContextMenu` in src/features/project.rs; register it with R2's displacement lists in
  src/overlay/registry.rs; src/app.rs routes `GridScrolled` to `dismiss_on_scroll_beneath()`;
  T004 (reducer parts) and T006 pass
- [X] T010 [US1] src/ui/overview.rs: `card_menu_items(state, &SessionLocation) ->
  Vec<material::MenuItem<Message>>` (*Open* + `location_menu_items`, each message wrapped in
  `CardMenuEntryChosen`) and `settle_card_menu(&mut app::State)` (R8); src/app.rs calls
  `settle_card_menu` at the outermost-update epilogue beside `agent_confirm::release`; T005 and
  T008 pass
- [X] T011 [US1] src/ui/material/overview_card.rs `OverviewCard::on_secondary_press(impl Fn((u16,
  u16)) -> M)` wrapping the card in `cdk::context_area::ContextArea`; src/ui/material/card_grid.rs
  `CardGrid::on_scroll(M)` on its scrollable; T007 passes
- [ ] T012 [US1] Wiring (glue, quickstart §B B1, B3–B9, B14–B16, B19): src/ui/overview.rs view passes
  `.on_secondary_press(|anchor| CardMenuOpened { location, anchor, origin: Pointer })` per card and
  `.on_scroll(GridScrolled)` to the grid; src/ui/mod.rs `view_with` pushes the card-menu surface
  (`MenuOverlay::new(card_menu_items(..), CardMenuDismissed, roles).anchor(clamped)`, anchor
  clamped with `clamp_menu_anchor` and `menu_panel_size`) beside the row menus; src/main.rs
  `overview_message` answers `CardMenuEntryChosen(inner)` by running the reducer, then
  `update_inner(*inner)`, the path a row-menu pick takes, so a failing or stale entry
  behaves as it does from the row menu (Edge: entry fails, stale entry, by construction) and its
  dialog draws over the Overview as over any main area (research R4); B19 checks the bind dialog;
  tests/context_menu_anchor_call_sites.rs stays green
- [ ] T013 [US1] docs/user-guide/overview.md: new section *Acting on a card* after *Activating a
  card*: right-click (Ctrl+click / two-finger click on macOS) opens the menu at the pointer; *Open*
  then the row menu's entries; which entries keep the Overview open and which close it; Escape, a
  click outside or scrolling closes it without running anything

**Checkpoint**: the MVP: a pointer user acts on any location from its card.

---

## Phase 4: User Story 2 — See which card the menu belongs to, and dismiss it (P1)

**Goal**: the target card shows a selected-style mark (2dp `primary` outline + `state::SELECTED`
layer) that follows the location, distinct from hover and focus by shape; a right-click on empty
Overview space closes the menu.

**Independent Test**: spec US2's: right-click a card, move the pointer over the menu and off the
card: the card keeps its mark; press Escape: the menu closes, the mark goes, the Overview stays.

### Tests for US2 (write first)

- [ ] T014 [P] [US2] Pixel tests in src/ui/material/overview_card.rs beside
  `the_focused_card_draws_the_focus_ring`: a `.targeted(true)` card draws a 2dp `primary` outline
  at its edge and the `state::SELECTED` layer over its fill; an untargeted card does not; the
  targeted render differs from the focused one (ring `secondary`, inset) and the hovered one
  (layer only, no outline) (FR-006); a targeted card does not publish `on_press` on Enter or Space
  (R6, FR-013). Fail (builder missing)
- [ ] T015 [P] [US2] src/ui/material/composition_contrast.rs: `primary` against each card
  palette fill (`overview_card::palette` for every `CardState`) at ≥ 3:1 (non-text) in light and
  dark (FR-006)
- [ ] T016 [P] [US2] tests/showcase_completeness.rs: the showcase lists a targeted `OverviewCard`.
  Fails
- [ ] T016b [US2] tests/overview_card_menu.rs: `ui::overview::is_targeted(state, &location)` (the
  view's predicate) is true only for the open menu's target, and still true for it after the cards
  reorder (Edge: cards reorder). Fails. Shares the file with T004/T005/T023: sequential

### Implementation for US2

- [ ] T017 [US2] `OverviewCard::targeted(bool)` in src/ui/material/overview_card.rs: outline and
  layer per R7; drops the card's Enter/Space claims while targeted; T014 and T015 pass
- [ ] T018 [US2] src/ui/overview.rs `is_targeted` and the view's `.targeted(is_targeted(..))`;
  the Overview container wrapped in a `ContextArea` whose secondary press sends
  `CardMenuDismissed` (empty-space right-click, R3); the targeted card in the `OverviewCard`
  showcase section under src/showcase/sections/; T016 and T016b pass (glue: quickstart §B B2, B6, B17)
- [ ] T019 [US2] docs/user-guide/overview.md *Acting on a card*: the target mark, and that a click
  outside the menu closes it without activating what it lands on

**Checkpoint**: both P1 stories work.

---

## Phase 5: User Story 3 — Open the card menu from the keyboard (P2)

**Goal**: on a focused card the Menu key or Shift+F10 opens the menu inside the card with its first
entry highlighted; Up/Down move the highlight (wrapping), Enter/Space run it, and focus returns to
the target card (or the first card when it is gone) after a keyboard-opened menu closes.

**Independent Test**: spec US3's: open the Overview, Tab to a card, press Shift+F10, Down, Enter:
the second entry runs for that card.

### Tests for US3 (write first)

- [ ] T020 [P] [US3] Tests in src/ui/material/keyboard_focus.rs: `.chord(Named::F10,
  Modifiers::SHIFT, m)` publishes `m` on Shift+F10 only, not on F10 alone or Ctrl+Shift+F10;
  `.key(Named::ContextMenu, m)` publishes on the Menu key (R5, FR-009). Fail (`chord` missing)
- [ ] T021 [P] [US3] Tests in src/ui/material/overview_card.rs: with `.on_menu_key(m)` a focused
  card publishes `m` on the Menu key and on Shift+F10, not on F10 alone, and captures the event so
  it does not propagate to another handler (Edge: not passed to a terminal); unfocused, nothing; a
  targeted card drops the claim (FR-009, FR-013). Fail
- [ ] T022 [P] [US3] Tests in src/ui/material/menu.rs (the menu-anatomy tests): with
  `.highlighted(Some(i))` item `i` draws the focus state layer and ring and the others do not; the
  panel size equals `menu_panel_size` with and without a highlight; tests/showcase_completeness.rs
  lists a highlighted `MenuOverlay` (FR-010). Fail
- [ ] T023 [US3] tests/overview_card_menu.rs, keyboard part (the `Keyboard` origin's
  `highlighted` and `focus_return` effects are T004's, M1): `CardMenuRequested(loc)` changes no
  state; `CardMenuMoved
  { forward, items }` steps by one and wraps last→first and first→last, and from `None` starts at
  the first (forward) or last (back) entry; `card_menu_entry(state)` returns the highlighted item's wrapped message, `None` with
  no highlight and `None` for a stale index past the current list; `focus_return_target(state,
  &loc)` is `loc` while its card is listed, else the first card's location; `keyboard_anchor(bounds)`
  lies inside `bounds` (top-left inset by `spacing::MD`); `card_id(&a) != card_id(&b)` for
  distinct locations and is stable across calls. Fails (missing)

### Implementation for US3

- [ ] T024 [US3] `TakesTheKeyboard::chord(Named, Modifiers, M)` in
  src/ui/material/keyboard_focus.rs (data in the key set, like `.key`); T020 passes
- [ ] T025 [US3] `OverviewCard::on_menu_key(M)` (claims `Named::ContextMenu` and Shift+F10;
  dropped while targeted) in src/ui/material/overview_card.rs; `MenuOverlay::highlighted(Option<usize>)`
  in src/ui/material/menu.rs with the `Indicator` colours `TakesTheKeyboard` uses; the highlighted
  menu in the `MenuOverlay` showcase in src/showcase/sections/floating.rs; T021 and T022 pass
- [ ] T026 [US3] src/features/overview.rs `Msg::{CardMenuRequested(SessionLocation),
  CardMenuMoved { forward: bool, items: usize }, CardMenuActivated}`; src/ui/overview.rs
  `card_id(&SessionLocation)` (replaces `FIRST_CARD_ID`, `start_control_id` pattern),
  `card_menu_entry`, `keyboard_anchor`, `focus_return_target`; T023 passes
- [ ] T027 [US3] Wiring (glue, quickstart §B B10–B13, B18): src/ui/overview.rs view sets `.id(card_id(..))`
  and `.on_menu_key(CardMenuRequested(location))` per card; src/ui/mod.rs `view_with` passes
  `.highlighted(card_menu.highlighted)`, and `subscription` swaps the Overview arrow listener for a
  named-`fn` card-menu key listener while a card menu is open (Up/Down → `CardMenuMoved`,
  Enter/Space → `CardMenuActivated`); src/main.rs `overview_message`: `CardMenuRequested` →
  `focused_bounds_task()` → `CardMenuOpened { origin: Keyboard, anchor: keyboard_anchor(bounds) }`,
  `CardMenuActivated` → `card_menu_entry` → `CardMenuEntryChosen`; a `card_focus_return` drain
  beside `tab_reveal_scroll` that, while the Overview is open and `registry::open_dialog(..)` is
  `None`, takes `focus_return` and focuses `card_id(focus_return_target(..))`; Overview opening
  focuses the first card's `card_id` (was `FIRST_CARD_ID`)
- [ ] T028 [US3] docs/user-guide/overview.md *Acting on a card* and *Keyboard*: Menu key /
  Shift+F10 on a focused card, Up/Down/Enter/Space in the menu, Escape returns focus to the card

**Checkpoint**: every card-menu action is reachable without a pointer (SC-004).

---

## Phase 6: Polish & Cross-Cutting (close unit; no code)

- [ ] T029 Run quickstart.md §B (B1–B19) with the `visual-pass` skill and record the evidence in
  specs/972-overview-card-context-menu/visual-pass/ (steps a milestone already recorded are cited,
  not re-run)
- [ ] T030 Record quickstart.md §C: the CI jobs that cover macOS and Windows, and the human pass
  (B1 with Ctrl+click, B6, B10) still to run on real desktops

---

## Dependencies & Execution Order

- Foundational (T001–T003) → US1 (M1). US2 (M2) and US3 (M3) each need US1's state, surface and
  wiring; US3's `targeted` claim-drop test (T021) needs US2's `targeted` (T017), so M3 follows M2.
- Within each story: tests first and failing, `features/` before `ui/`, shared components before
  the view that uses them, glue last, docs in the same milestone.
- T004, T005, T016b and T023 share tests/overview_card_menu.rs: sequential.

## Parallel Example: US1

```text
T004 → T005 (tests/overview_card_menu.rs, sequential)
T006 popover_displacement.rs / overlay_registry.rs / overlay_registration.rs · T007 overview_card.rs + card_grid.rs · T008 gates/context_menu_anchor.rs
```

## Implementation Strategy

MVP first: Foundational + US1 put a working right-click menu on `main` (dismissal by Escape,
outside click and scroll come from the registry). M2 adds the target mark and empty-space
dismissal; M3 the keyboard path and focus return. Each milestone leaves the Overview complete for
the stories before it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Right-click a card to act on its location 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: right-clicking an Overview card opens a menu at the pointer with *Open* and then
  exactly that location's row-menu entries; choosing one does what the row menu (or a primary click,
  for *Open*) does; Escape, a click outside, a grid scroll, the target disappearing or the Overview
  closing close it without running anything
- **Satisfies**: US1 acceptance scenarios 1–7; US2 acceptance scenarios 2, 3 (primary click), 4;
  FR-001–FR-005, FR-007, FR-008 (primary click, card B), FR-012, FR-013 (primary click), FR-014; SC-001, SC-002, SC-003,
  SC-005; edge cases window edge, small window, target disappears, target changes, grid scrolls,
  Overview closes, orphan, several sessions, stale entry and entry fails (by construction, R4),
  dialog cancelled; SC-003 by construction (items built per render from held state, as the row
  menus)
- **Verify**: `cargo test -p micold-client --test overview_card_menu --test popover_displacement
  --test overlay_registry --test overlay_registration --test icons --test icons_font --test layout_snapshot`;
  `cargo test -p micold-client --lib -- overview_card card_grid`; `scripts/gate.sh`; quickstart §B
  B1 (entries), B3–B9, B14–B16, B19
- **Depends on**: —
- **Tier**: full

### M2 — Target mark on the card and empty-space dismissal

- **Tasks**: T014–T019 (with T016b)
- **Deliverable**: while a card menu is open its card shows a `primary` outline with the selected
  layer, unlike hover and focus, following the location when cards reorder; a right-click on empty
  Overview space closes the menu
- **Satisfies**: US2 acceptance scenarios 1–3; FR-006, FR-008 (secondary click on empty space),
  FR-013 (Enter/Space while targeted); edge case cards reorder
- **Verify**: `cargo test -p micold-client --lib -- overview_card composition_contrast`;
  `cargo test -p micold-client --test overview_card_menu --test showcase_completeness`;
  quickstart §B B2, B6, B17
- **Depends on**: M1
- **Tier**: light

### M3 — Open the card menu from the keyboard

- **Tasks**: T020–T028
- **Deliverable**: on a focused card the Menu key or Shift+F10 opens the menu inside the card with
  *Open* highlighted; Up/Down move (wrapping), Enter/Space run the entry, and after Escape or an
  in-place entry focus is back on the card (or the first card when it is gone)
- **Satisfies**: US3 acceptance scenarios 1–5; FR-009, FR-010, FR-011, FR-013; SC-004; edge case
  Shift+F10 and the Menu key elsewhere
- **Verify**: `cargo test -p micold-client --lib -- keyboard_focus overview_card menu`;
  `cargo test -p micold-client --test overview_card_menu --test showcase_completeness`; quickstart
  §B B10–B13, B18
- **Depends on**: M2
- **Tier**: full

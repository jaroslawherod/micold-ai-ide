# Contract: the sidebar (client)

Render-free parts (`features/sidebar.rs`, `features/overview.rs`, `features/navigation.rs`) are
tested in `crates/micold-client/tests/*.rs` or their in-crate `#[cfg(test)]` modules. View glue
(`ui/sidebar.rs`) composes those answers and the shared components; it is checked by geometry and
pixel tests in `ui/material/*`, the layout snapshot and quickstart §B.

## S1. Layout, top to bottom

With a project open: the Overview row; the chip bar; the start-up offer banner when shown (feature
582); the scrolling list of group headers and rows; the no-worktrees / no-match hint. With no project
open: no Overview row, chip bar, header or row (Edge Cases). The "Worktrees" title, the filter
button and the filter accordion are gone (FR-014, spec Assumptions).

## S2. Overview row

- First row, above the chip bar, not in the scroll region (US2.1, FR-007).
- Pressed (pointer, Enter, Space) while a project is active and Settings is closed: sends
  `overview::Msg::Opened`; the Overview opens; pressed again while open: nothing changes (US2.3).
  Otherwise it has no press action (US2.5).
- Selected look (`Selection::Primary`) exactly while `overview.open`; then no location or session
  row is selected (US2.4, FR-009).
- Badge: present exactly when `features::overview::badge(&ui::overview::cards(state))` is `Some`,
  showing its `label` in the tone's container pair; the tooltip equals the top bar button's (SC-002,
  FR-008).

## S3. Chip bar

- **All** first, then one `ToggleChip` per `available_tag_filters()` entry in that order (US4.1).
- **All** active iff `filters.is_empty()`; pressing it clears every filter (US4.2, US4.3).
- Type chips toggle as today (OR, current-session exception, no-match message unchanged; US4.2,
  US4.4, US4.8).
- Add, hide and more on the first line, right-aligned; chips wrap onto further lines left of them;
  no chip or action is cut off or overlaps at 180, 300 and 600 px sidebar width (US4.7, FR-016).
- Add sends `FormMsg::Opened`; hide sends `SidebarMsg::Toggled` (US4.5).
- More toggles `SidebarMoreMenu`: a `MenuOverlay` hung from the trigger's corner, items in order
  *Refresh the worktree list*, *Attach existing…*, *Run in parallel…*, each disabled exactly when
  today's header button is (`can_refresh_worktrees()`, a project is active, a project is active),
  each with today's tooltip; choosing one sends today's message and closes the menu; Escape, a click
  outside and a displacing surface close it (feature 017) (US4.6, FR-015).

## S4. Groups

- `sidebar_list()` is: for each group of `sidebar_groups()` (contract G3 order, empty groups
  absent), a header row then, when open, the group's rows (US1.1, US1.4, FR-001, FR-002).
- Header text is contract G4's; the header shows the twisty open or closed; pointer press, Enter or
  Space sends `GroupToggled(group)` (US5.2, FR-017).
- `GroupToggled` stores `!group_open(...)` for the active project's group; the choice survives rows
  moving, a worktree re-read and a project switch and back (US5.3, FR-005).
- A location's group equals the group of its Overview card's state, from the same
  `location_card_state` call (US1.7, SC-007).

## S5. Rows

- Every location row: first line per data-model *Row text* (`TypeRole::SidebarName`, the error
  colour for a missing or invalid worktree); second line `sidebar_second_line` followed by the
  ticket status chip and the label chips (current session, daemon, outside this app, can be removed,
  missing/invalid status); no type or issue tag chip (US3, FR-011, FR-013).
- Both lines ellipsize; the tooltip begins with the full title and holds a `Branch` line with the full
  branch, then today's lines (US3.5, FR-012).
- State icon, unread count, pull request indicator, hover actions, run annotation, expander and
  session sub-rows as today (US3.7, Edge Cases).
- Selected row (`Selection::Primary`): `primary_container` fill, `on_primary_container` text on
  both lines (US3.6, FR-010).
- Ripple and right-click cover the whole two-line row; the context menu, its items, placement and
  dismissal are unchanged (US5.6, FR-018).
- Rows are keyed (R13): the keyboard focus stays on a row that changes group (US5.5).
- A row that moved since the pointer entered it shows no hover actions until hovered again (R14).

## S6. Keyboard and navigation

- Tab order: Overview row, chips, add, hide, more, then headers and rows top to bottom (US5.1,
  FR-017, SC-005).
- Navigation chords (feature 803) step through locations in `sidebar_groups()` order, closed groups
  included; landing on a location in a closed group opens the group, selects the row and scrolls it
  into view (US5.4, FR-006).
- Every reveal (feature 024 current session, the Overview's reveal, the quick switcher, a
  notification) whose target is in a closed group opens the group before the scroll is computed
  (FR-006, R5).

## S7. Scroll metrics

`row_heights(&[ListRow])`: header = one-line dense height; location row = two-line dense height;
session row = one-line dense height. `row_index`, `location_row` and `current_session_row` count
headers as rows and skip the rows of closed groups. The reveal lands the target fully in view
(feature 024 contract §6, unchanged).

## S8. Shared components touched (Principle VIII)

| Component | Change |
|---|---|
| `TreeItem` | `.supporting_text(text, tint)`, `.leading_chip(element)` (replaces `chip_after_tag`), `.heading()`, `.key(u64)` |
| `TreeView` | `.selection(Selection)` (default `Secondary`); lays keyed rows out in `cdk::KeyedColumn` |
| `cdk::KeyedColumn` | moved from `ui/transcript/keyed.rs`, unchanged API |
| `TypeRole` | `SidebarGroup` (12 px bold) |
| `Icon` | `MoreHorizontal` (`more_horiz`, U+E5D3), pinned in `tests/icons.rs` |
| `MenuOverlay`, `MenuTrigger`, `ToggleChip`, `Tag`, `IconButton`, `cdk::ContextArea` | reused as they are |

All builders stay chainable and end in `.into()`.

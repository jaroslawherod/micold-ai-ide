# Research: Sidebar grouped by state, with Overview as its first row (#1125)

Each decision names what was chosen, why, and what was rejected. Paths are workspace-relative.
"Core" is `crates/micold-core`; "client" is `crates/micold-client`.

## R1. Where the grouping and row-text rules live

**Decision**: A new render-free core module `micold_core::sidebar_groups`
(`crates/micold-core/src/sidebar_groups.rs`) holds the group vocabulary and every rule FR-020 names:
which group a card state belongs to, the group order, the stable partition that keeps today's order
inside a group, the counts, which groups show, the header text, a run group's state, and the rule
that says whether a group is open. The second line's text goes into the existing
`micold_core::overview_text` beside the feature 1120 rules it composes (`sidebar_second_line`).
Contract: [contracts/sidebar-groups.md](contracts/sidebar-groups.md).

**Rationale**: FR-020 and Principle I: the rules are unit-tested in `micold-core` with no GUI
(`mise run test-core`). The client's render-free `features/sidebar.rs` only feeds them the window's
state and turns their answer into rows, as it already does for `matches_filters` and
`effective_open`.

**Alternatives rejected**:
- Put the grouping in `features/sidebar.rs`: render-free, but FR-020 asks for the core, and the
  rule is generic over what a row is, so it needs nothing of the client.
- Reuse `micold_core::overview::order` (sort by `CardState`): its order is worst first with Done
  before Idle and Needs attention separate; the sidebar has four groups with Done last (spec
  Assumptions, D2, D3). One sort key cannot serve both.

## R2. Where a location's card state comes from

**Decision**: Extract the card-state half of `ui/overview.rs::card` (building `CardSignals` from
the location's un-archived sessions, pending agent requests and pull request signal, then
`micold_core::overview::card_state`) into a render-free
`features::overview::location_card_state(&State, &SessionLocation, Option<&str>) ->
(CardState, CardReason)`, and move `pr_signal` with it. `ui/overview.rs::card` and the sidebar
grouping both call it.

**Rationale**: US1.7 and FR-001: the sidebar and the Overview must never disagree. One function
for both makes disagreement unrepresentable rather than tested for. It also moves decision logic
out of the `ui/` glue, where the constitution's Principle I exception does not cover it.

**Alternatives rejected**:
- Call `ui::overview::cards(state)` from the sidebar and look each location up: builds the whole
  card model (progress, pull request lists, activity) to read one field, and leaves the rule in
  the render layer, where navigation and scroll metrics (render-free) cannot reach it.
- Use feature 905's `LocationState` (the row's state icon): it has no Done and no pull request
  signals, so a merged worktree and a failing check would group differently from their card
  (spec Assumptions).

## R3. How the grouped list is represented

**Decision**: `sidebar_entries()` keeps its meaning (every listed entry, today's order, flat), so
`cards()`, the badge and the filter rules do not change. A new `State::sidebar_groups()` returns
`Vec<SidebarGroup>` (data-model): each group's kind, count, open flag and entries, in group order,
empty groups dropped. The drawn list is a new closed enum `ListRow { Header(GroupHeaderRow),
Entry(SidebarEntry) }` built by `State::sidebar_list()`: a header per shown group, then that group's
entries only when it is open. `row_heights`, `row_index`, `location_row` and `current_session_row`
take `&[ListRow]` and count a header as one row.

**Rationale**: the scroll metrics (feature 024) and the keyboard row (feature 803) are positions in
what is drawn; a header is drawn, so it must be a row in that walk, and a collapsed group's rows are
not drawn, so they must be absent from it. A separate enum keeps `SidebarEntry` meaning "a location
or run group" (Principle V): code that walks locations (`cards`, navigation) cannot meet a header.

**Alternatives rejected**:
- A `SidebarEntry::Header` variant: every location walk (`cards`, `location_nodes`,
  `unread_count`) would need an arm that ignores it, and a header could reach the Overview.
- One `TreeView` per group inside its own `AccordionSection`: gives an open/close animation, but
  the scroll metrics would have to model section paddings and headers of another component, and the
  rows would no longer be one list for Tab order.

## R4. Collapsed groups: what is remembered and the open rule

**Decision**: `sidebar::State` gains `group_choices: BTreeMap<PathBuf, BTreeMap<StateGroup, bool>>`:
the user's own open/closed choice per project and group, in memory only. Whether a group is open is
`sidebar_groups::group_open(choice, group, holds_current)`: the choice when there is one, else
`group.starts_open() || holds_current` (`starts_open` is false only for Done). A header press
(`SidebarMsg::GroupToggled(StateGroup)`) stores the negation of the group's *effective* open state
for the active project. `project_entered()` does not touch it; `worktrees_replaced` does not prune
it (it is keyed by group, not by row).

**Rationale**: FR-005 and US5.3: Done starts collapsed, every group collapses, a choice holds for the
run per project and survives rows moving, project switches and re-reads. The current-session rule
mirrors feature 024's `effective_open` for a row (Edge Cases: "the Done group expands to show it"),
and toggling against the effective state is the lesson feature 024 recorded for rows
(`toggle_location`): a toggle against the stored set alone would leave a header that visibly does
nothing.

**Alternatives rejected**:
- Seed the map when a project is first shown: at that moment the worktree list and pull request
  states may not have arrived, so which group holds the current session is not yet known.
- Store a set of collapsed groups: cannot tell "the user expanded Done" from "never touched", so the
  current-session rule could not yield to a user's explicit collapse.
- Persist to settings: out of scope (spec Out of scope).

## R5. Reveals and chords into a collapsed group

**Decision**: `State::open_group_of_reveal_target()` (render-free, `features/sidebar.rs`) finds the
reveal's target location (the chord's `keyboard_row`, else the current session's location, else
`sidebar.revealed`) and, when its group is closed, records the choice `true` for it. `main.rs`
calls it at the top of `reveal_scroll` and `revealed_scroll`, before the "is the row listed" check,
so the arm drains once the row is drawn. Navigation's `location_nodes` walks `sidebar_groups()` in
group order including closed groups (FR-006: chords step through the grouped order and expand the
target's group); collapsed run groups stay out of the order as today.

**Rationale**: every reveal path (feature 024, the Overview, the quick switcher, a notification, a
chord) already ends in one of the two drains, so one call there covers all of them. Recording a
choice (not a transient flag) keeps the group open after the reveal, as a user's expand would.

**Alternatives rejected**:
- Expand in `reveal_scroll_armed()`: at arm time the target session is often not selected yet.
- Make reveals skip collapsed groups: contradicts FR-006.

## R6. The row's two lines and its chips

**Decision**: `TreeItem` gains `.supporting_text(text, tint)`: a second line under the label, at
the label's indent, drawn as `Ellipsized` at `TypeRole::SidebarTag`-sized caption text and
`on_surface_variant`. A row with supporting text is Material's two-line item
(`density::LIST_ROW_TWO_LINE_BASE`). The row's remaining chips (ticket status, *current session*,
daemon, *outside this app*, *can be removed*, missing/invalid status) sit on that same second line
after the text; the text takes the remaining width and ends in an ellipsis. Type and issue tags are
no longer passed as chips (FR-013); `node.tags` still drives the filters. The ticket status chip,
which today follows the issue tag (`chip_after_tag`), becomes the first chip after the text
(`.leading_chip(element)`, replacing `chip_after_tag`, whose only caller is the sidebar). Every
location row, Default included, is two-line, so `row_heights` makes location rows two-line always.

**Rationale**: US3 and the mockup: two lines per row. FR-013 keeps the label chips, and the daemon
chip is on every worktree row (`daemon_row_label`), so a separate chip line would make every row
three lines tall and double the list's height against the mockup. Today's chip line already carries
the type and issue tags those chips sat beside; dropping those two frees the space.

**Alternatives rejected**:
- A third line for chips: three lines on every worktree row (the daemon chip is always there).
- Move the label chips into the tooltip: FR-013 says they stay on the row.
- Wrap chips onto further lines when they do not fit: row height would depend on text width, which
  the render-free scroll metrics (`row_heights`) cannot know (feature 024 R6).

## R7. The selected look

**Decision**: `TreeView` gains `.selection(Selection::Primary)`; the default stays
`Selection::Secondary` (today's `secondary_container` pill), so `ui/changes.rs` and the showcase's
other trees do not change. The sidebar's list and its Overview row pass `Selection::Primary`:
`primary_container` fill, `on_primary_container` for the label, the supporting line, the state
icon slot's tint is unchanged (it carries the state). While the Overview is open, no location or
session row is selected (FR-009); the Overview row is.

**Rationale**: FR-010; Principle VIII (extend the shared tree, do not fork a sidebar row).
`on_primary_container` on `primary_container` is a Material pair and is checked at 4.5:1 in both
themes by the contrast test (R12).

**Alternatives rejected**: change `TreeView`'s selection colour for every caller: alters the
Changes view, which this feature does not touch.

## R8. Group headers

**Decision**: A group header is a `TreeItem` row built with a new `.heading()` builder: label in a
new `TypeRole::SidebarGroup` (12 px, bold, `on_surface_variant`, sentence case: "Needs action · 2"),
no state icon, the twisty showing open or closed, the whole row pressable (`on_press` = the toggle),
one-line dense height. It gets the same keyboard focus wrapper as every row, so Enter and Space
toggle it (feature 803 / `TakesTheKeyboard`).

**Rationale**: reuses the tree's row machinery (twisty, ripple, focus ring, hover, height floor)
rather than a new widget beside it (Principle VIII), and keeps headers in the one flat list the
scroll metrics walk (R3). Sentence case because the spec writes the header as "Needs action · 2";
the mockup's uppercase is styling, and uppercase in a 12 px label is harder to read.

**Alternatives rejected**:
- A new `GroupHeader` widget: would duplicate the row's press, focus and ripple handling.
- `AccordionSection` per group: R3.
- Pin Done at the panel's foot as the mockup draws it: a second scroll-free region whose height
  changes when Done opens, and the reveal metrics would need two coordinate systems. Done is the
  list's last group instead (spec FR-001 asks for order, not for pinning).

## R9. The Overview row

**Decision**: The Overview row is a one-item `TreeView` above the chip bar, outside the scrolling
list: `TreeItem::new(0, "Overview", on_surface).with_icon(Icon::Overview)`,
`.selected(state.overview.open)`, `.on_press(...)` (attached only while a project is active and
Settings is closed, so the row is otherwise inert as the top bar's button is) sending a new
`overview::Msg::Opened { settings_open }`, and a
trailing badge: the shared `Tag` (`.solid`) with `badge.label` ("2 need action") in
`error_container`/`on_error_container` or `warning_container`/`on_warning_container`. The badge comes
from the same `features::overview::badge(&ui::overview::cards(state))` call the top bar makes. The
row tooltip is the top bar's (`overview_tooltip`, moved from `ui/toolbar.rs` to
`features::overview` so both read one function). `Opened` opens like `Toggled` and does nothing when
already open; it is ignored while an activation is pending, as `Toggled` is.

**Rationale**: US2, FR-007, FR-008. One badge function, one card list, one tooltip: the row and the
button cannot disagree (SC-002). Outside the scroll region, so the row is always first (US2.1) and
the scroll metrics do not shift.

**Alternatives rejected**:
- Send `Toggled`: pressing the row while open would close the Overview (spec Assumptions: the row
  opens and never closes).
- Draw the badge as the top bar's `UnreadMark` (icon and number): the spec asks for the same words
  ("2 need action") on the row, as the mockup shows.

## R10. The badge under a tag filter

**Decision**: The row's badge counts the cards the Overview shows, which are the locations the
sidebar lists (feature 813 FR-001: one card per listed location, so a tag filter narrows them), i.e.
exactly what the top bar's badge counts. Spec FR-008 and US2.6 say "regardless of filters" and "as
the top bar's badge does"; the second premise is false in the code (`ui::overview::cards` walks
`sidebar_entries()`, which applies the filters), so the two clauses cannot both hold. The plan keeps
the one SC-002 measures (equal to the top bar badge) and amends FR-008 and US2.6 to say so (ledger
D7).

**Rationale**: SC-002 and FR-008's first clause make equality the requirement; counting unfiltered
locations would need either a badge that differs from the button whenever a filter is on, or a
change to feature 813's Overview (its cards and badge), which the spec puts out of scope ("No new
card states, new state rules").

**Alternatives rejected**: an unfiltered badge on both the row and the button: changes 813's
button while the Overview it opens still shows the filtered cards, so "2 need action" would open
onto one.

## R11. The chip bar and the more menu

**Decision**: The chip bar is a `row![chips, actions]` under the Overview row. `chips` is a
wrapping `Row` (`Row::wrap()`) of `ToggleChip`s: **All** (active when no filter is) then today's
`filter_chip`s in `available_tag_filters()` order; **All** sends `SidebarMsg::FiltersCleared`.
`actions` holds, on the first line: add (`IconButton`, `Icon::AddWorktree`, today's tooltip and
message), hide (`Icon::HideSidebar`), more (`MenuTrigger` with a new `Icon::MoreHorizontal`,
`more_horiz`, U+E5D3, wrapped in `cdk::ContextArea::on_primary_bounds` to report its corner).
The menu is the shared `MenuOverlay` hung from that corner
(`SidebarMsg::MoreMenuAnchored`, clamped by `features::project::clamp_menu_anchor`), carrying three
`MenuItem`s (refresh, attach existing, run in parallel) whose `message` is `None` when today's
enabled rule says no and whose `trailing_tooltip` is today's tooltip. It is registered as a floating
surface `SidebarMoreMenu` (Popover layer, cancelled by `SidebarMsg::MoreMenuToggled`), so feature
017's dismissal (Escape, click outside, displacement) applies. Choosing an item sends the old
header button's message and closes the menu.

The filter accordion, its `FilterTrigger`, `sidebar.filter_open`, `SidebarMsg::FilterMenuToggled`
and the `SidebarFilterPanel` surface are removed (FR-014). Tests that used the filter panel as their
sample popover (`tests/overlay_registry.rs`, `tests/overlay_dispatch_ordering.rs`,
`tests/navigation.rs`) move to `SidebarMoreMenu`, which takes its place in the registry's
displacement lists. `FilterTrigger` stays in the library only if another caller uses it; the
showcase's accordion sample uses it, so it stays and its doc stops naming the sidebar.

**Rationale**: US4, FR-014–FR-016; D4 and D6 (shared `Menu` primitive, feature 017 rules). Wrapping
chips left of the actions keeps the actions on the first line (US4.7). The switcher's
anchor-reporting pattern (BUG-1023) already solves "hang a menu from a trigger that is not in the
app bar".

**Alternatives rejected**:
- `ContextMenu` at the press point: a pointer press has a point, a keyboard press does not.
- Keep the old header row for the three actions: spec D4 moved them into the more menu.
- Reuse `Icon::Menu` (⋮, `more_vert`): the spec names ⋯; the vertical glyph is the app bar's
  overflow menu, and two different menus with one glyph side by side would read as one.

## R12. Contrast

**Decision**: `ui/material/composition_contrast.rs` gains, for both schemes: header text
(`on_surface_variant` on the sidebar tone `surface_container_low`), first line (`on_surface` on the
sidebar tone), second line (`on_surface_variant` on the sidebar tone), selected first and second
line (`on_primary_container` on `primary_container`), the Overview row label on the sidebar tone and
on `primary_container`, and the two badge pairs (`on_error_container`/`error_container`,
`on_warning_container`/`warning_container`). Each must be ≥ 4.5:1.

**Rationale**: FR-019, SC-004; the existing test file is where every composition's pairs are
checked.

## R13. Keyboard focus stays on a moved row

**Decision**: Promote `ui/transcript/keyed.rs::KeyedColumn` to `ui/cdk/keyed_column.rs` (the
transcript imports it from there) and have `TreeView` lay its rows out in it when every item has a
key (`TreeItem::key(u64)`). The sidebar keys a location row by a hash of its `RowRef`, a session row
by its `SessionId`, a header by its `StateGroup`, a run group by its `GroupId`. Widget state (the
`TakesTheKeyboard` focus, the ripple, the hover reveal) then follows the row wherever it moves.

**Rationale**: US5.5 and FR-017: with an index-matched column, a row that moves to another group
leaves its focus in its old slot, on whatever row takes its place. `KeyedColumn` is the repo's
existing answer to exactly this (transcript R12); Principle VIII says promote it rather than copy it.

**Alternatives rejected**: iced's `keyed_column`: matches by index when the length is unchanged
(the transcript module's doc records why), which is the common case when a row only changes group.

## R14. Hover and menus on a row that moves

**Decision**: `WorktreeMsg::Hovered(dir)` also records the row's drawn index
(`worktree.hovered: Option<(String, usize)>`); the view treats the row as hovered only while its
current index in `sidebar_list()` equals the recorded one (`sidebar::hover_holds`). An open row
context menu keeps its anchor and its location; it is not closed by a move.

**Rationale**: Edge Cases: "a row moved out from under the pointer loses its hover"; iced sends no
exit event when content moves under a still pointer. The spec keeps the menu open on its location.

**Alternatives rejected**: clear `hovered` on every state update: the hover actions would flicker
off whenever any signal arrives.

## R15. Layout snapshot, showcase, docs

**Decision**: The layout snapshot's covered states (`tests/support/covered_states.rs`) that render
the sidebar change shape (Overview row, chip bar, headers, two-line rows); the fixture
(`tests/fixtures/layout_snapshot.txt`) is regenerated once the sidebar is done. A new covered state
`main-shell-sidebar-grouped` holds one location per state, a collapsed Done and the Overview open,
so headers, the selected Overview row and a collapsed group are all recorded. The
`CLIP_REVEALED` exemptions for the sidebar's filter accordion and the `revealing_states()` exception
in `docs/development/layout-snapshot.md` go with the accordion. The showcase
(`showcase/sections/atoms.rs`, TreeView entry) gains the Overview row with and without a badge, a
group header open and closed, and a two-line row unselected and selected (`Selection::Primary`); the
catalogue (`showcase/catalogue.rs`) lists `KeyedColumn` under cdk. The user guide's
`worktrees-and-sessions.md` (*Browsing worktrees*, *Reading a worktree: name & tags*, *Filtering
worktrees by tag*, *Refreshing the list*, *Resizing and hiding the sidebar*) and `overview.md` (its
opening section) describe the Overview row, the chip bar and more menu, the groups and the two-line
rows.

**Rationale**: FR-021, FR-022, Principle VII.

## R16. Shared layout with #1124, #1126 and #1127

**Decision**: This feature changes the sidebar column only: `ui/sidebar.rs`, `features/sidebar.rs`,
and the overlay stack in `ui/mod.rs` (one new `MenuOverlay` surface for the more menu, beside the
switcher's). It does not change `ui/mod.rs`'s main-window composition (sidebar | main area), the app
bar (`ui/toolbar.rs`, except moving `overview_tooltip` out of it), the terminal area or a status
bar. Shared files the parallel threads also touch: `ui/mod.rs` (overlay list), the layout snapshot
fixture and `covered_states.rs`, `composition_contrast.rs`, the showcase catalogue, and
`docs/user-guide/overview.md`. The fixture is never hand-merged: after a rebase onto a sibling's
merge, regenerate it and review the diff.

**Rationale**: the issue splits the redesign into four issues; keeping this one to the sidebar
keeps the merges mechanical.

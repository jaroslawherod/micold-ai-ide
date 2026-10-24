# Feature Specification: Sidebar grouped by state, with Overview as its first row

**Feature Branch**: `claude/project-thread-22fhnj`

**Created**: 2026-10-24

**Status**: Draft

**Input**: User description: "Implement GitHub issue #1125: Layout: sidebar grouped by state with
Overview as its first row. Blocked by: #1120 (title, short branch and counts text come from its core
logic). Sidebar: Overview row first, carrying the need-action/attention badge; tag chips with
add/hide actions beside them; worktrees grouped under Needs action / Working / Idle with counts,
Done collapsed. Rows are two lines: issue title (else branch, else name) and `type/number · session
state`. Selected row uses the primary container. Acceptance: grouping and ordering are pure core
logic with unit tests; groups keyboard-navigable; ripple, context menu and tag filtering unchanged;
contrast >= 4.5:1; docs, layout snapshot and visual pass updated. Mockup:
https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e (Layout Redesign, board \"Proposal: session
view\")." Labels: enhancement, flow:feature. Builds on features 813 (`specs/813-worktree-overview`),
905 (`specs/905-worktree-state-icon`), 1120 (`specs/1120-overview-card-readability`), 803
(`specs/803-keyboard-navigation`), 024 (`specs/024-reveal-current-session`), 008/009 (tag filters)
and 483 (`specs/483-parallel-agent-runs`, run groups). Part of the layout redesign: #1124 (status
bar), #1126 (worktree header and session tabs) and #1127 (title bar) change the rest of the window
and are out of scope here.

## Terms

- **Location**, **card state**, **Overview**, **attention badge**: as defined by feature 813
  (`specs/813-worktree-overview/spec.md#Terms`). The five card states are Needs action, Needs
  attention, Working, Done and Idle; a location's card state is the one its Overview card shows.
- **Title**, **short branch**, **sessions line**: as defined by feature 1120
  (`specs/1120-overview-card-readability/spec.md#Terms`).
- **Location row**: the sidebar row of one location, the *Default* entry (project root) or a
  worktree.
- **State group**: a labelled section of the sidebar's list holding the location rows of one card
  state (or of the states the group covers), headed by a **group header** that reads the group's
  name and how many rows it holds (for example "Needs action · 2").
- **Overview row**: the sidebar's first row, which opens the Overview and carries its attention
  badge.
- **Chip bar**: the line under the Overview row holding the tag filter chips and the sidebar's
  actions.
- **Run group**: the row of a parallel run (feature 483) with its runs nested below it.

## Clarifications

### Session 2026-10-24

- Q: Does a Needs attention location join the Needs action group or get its own group? → A: It joins
  the Needs action group; there is no separate Needs attention group, so the groups stay the issue's
  four. _(agent-resolved: issue #1125 body lists four groups; mockup board Main.dc.html#sidebar puts
  fix/1117, "PR #1118 open · checks failing" — Needs attention per
  crates/micold-core/src/overview.rs card_state — under "Needs action · 2")_

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See what needs me, grouped, without opening the Overview (Priority: P1)

A developer with a dozen worktrees wants to know which ones need them. Today the sidebar lists
locations in a fixed order with a small state icon each, so finding the blocked ones means scanning
every row. With this change the sidebar groups its location rows under headers by state, worst
first, each with a count: Needs action, then Working, then Idle, with Done at the bottom and
collapsed. The rows that need the user are always at the top.

**Why this priority**: It is the point of the issue: the sidebar answers "what needs me" at a
glance, the way the Overview does, while the terminal stays in view.

**Independent Test**: Open a project with at least one location in each card state, and check the
group headers, their order, their counts, which rows sit under which header, and that Done is
collapsed.

**Acceptance Scenarios**:

1. **Given** locations in several card states, **When** the sidebar shows them, **Then** each
   location row sits under the header of its card state's group, the groups appear in the order
   Needs action, Working, Idle, Done, and each header reads its name and the number of location
   rows in it (for example "Working · 2").
2. **Given** a location whose card state is Needs attention, **When** the sidebar shows it, **Then**
   it is listed in the Needs action group, counted in its header; within the group, rows keep
   today's order (scenario 5), whichever of the two states they are in.
3. **Given** a project just opened, **When** the sidebar first shows it, **Then** the Done group is
   collapsed (its header shows with its count and no rows below it), unless it holds the current
   session's row, which expands it (see Edge Cases).
4. **Given** a group with no location row (none in that state, or all hidden by a filter), **When**
   the sidebar is shown, **Then** neither that group's header nor any empty space for it is shown.
5. **Given** two locations in the same state, **When** they are listed, **Then** they keep the
   relative order the sidebar lists them in today (*Default* before worktrees, worktrees in their
   present order).
6. **Given** a location whose state changes (a session starts working, a permission prompt
   arrives, a pull request merges), **When** the sidebar next updates, **Then** its row moves to
   the new state's group within the time the Overview's card takes to change state (feature 813
   FR-008), and both groups' counts change with it.
7. **Given** a location whose Overview card shows a state, **When** the sidebar shows the same
   location, **Then** it is in that state's group: the sidebar and the Overview never disagree
   about a location's state.

---

### User Story 2 - Overview is the sidebar's first row (Priority: P1)

The developer wants the Overview where the locations are. With this change the sidebar's first row
is **Overview**: pressing it opens the Overview, and it carries the same attention badge as the top
bar's Overview button ("2 need action"), so the count of locations that need the user is beside the
list of them.

**Why this priority**: The redesign moves the Overview out of the title bar (#1127, which depends on
this feature); the sidebar has to offer it first.

**Independent Test**: With a project open and a location in Needs action, check the Overview row is
first, its badge matches the top bar button's badge, pressing it opens the Overview, and the row
looks selected while the Overview is open.

**Acceptance Scenarios**:

1. **Given** a project is open, **When** the sidebar is shown, **Then** its first row is Overview,
   with the Overview icon and the word "Overview", above the chip bar and every group.
2. **Given** some locations need action (or, with none needing action, some need attention),
   **When** the sidebar is shown, **Then** the Overview row carries a badge with the same state,
   count and words as the top bar's Overview badge (feature 813 FR-012, for example "2 need
   action"); with neither, it carries no badge.
3. **Given** the Overview is closed, **When** the user presses the Overview row (or activates it by
   keyboard), **Then** the Overview opens, as the top bar button and Ctrl+Shift+O (Cmd+Shift+O on
   macOS) open it; **given** it is open, **then** pressing the row leaves it open.
4. **Given** the Overview is open, **When** the sidebar is shown, **Then** the Overview row wears
   the selected look and no location row does; **When** the user then selects a location row,
   **Then** the Overview closes and that row wears the selected look, as selecting a session does
   today.
5. **Given** Settings is open or no project is active, **When** the sidebar is shown, **Then** the
   Overview row cannot be activated, as the top bar's button cannot (feature 813).
6. **Given** a tag filter is active, **When** the badge is shown, **Then** it counts every location
   of the project, filtered or not, as the top bar's badge does.

---

### User Story 3 - Rows say what each worktree is for, on two lines (Priority: P1)

Today a worktree row shows its folder name (such as `feat-813_worktree-overview`) and a row of tag
chips. With this change each location row has two lines: first the title (the issue's title when the
app knows it, else the branch, else the name), and below it the short branch and the sessions line
(`fix/1117 · 1 session · 1 running`). The row the user is on is filled with the primary container
colour.

**Why this priority**: Grouping helps only if each row says which piece of work it is.

**Independent Test**: List a worktree with a known issue title, one with a branch but no known title,
the *Default* entry, one with no sessions and the row holding the current session, and check both
lines of each row and the selected fill.

**Acceptance Scenarios**:

1. **Given** a worktree whose linked issue's title the app knows, **When** its row is shown,
   **Then** the first line is the issue's title and the second line starts with the short branch
   (`fix/1117`), followed by " · " and the sessions line ("fix/1117 · 1 session · 1 running").
2. **Given** a worktree with a branch and no known issue title, **When** its row is shown, **Then**
   the first line is the branch and the second line is only the sessions line (the branch is not
   repeated, feature 1120 FR-007).
3. **Given** the *Default* entry, **When** its row is shown, **Then** the first line is "Default"
   and the second line reads "project root · " followed by the sessions line.
4. **Given** a location with no session, **When** its row is shown, **Then** the second line ends in
   "No sessions" and the row's start action is offered as today.
5. **Given** a first or second line that does not fit the sidebar's width, **When** the row is
   shown, **Then** it ends in an ellipsis, and the row's tooltip shows the full title, the full
   branch name and the facts the tooltip shows today (path, state, unread line, pull request).
6. **Given** the row that holds the session in view (or the row the Overview sent the user to),
   **When** the sidebar is shown, **Then** that row is filled with the primary container colour and
   its text uses the matching on-primary-container colour, with a contrast of at least 4.5:1 in
   both themes.
7. **Given** any location row, **When** it is shown, **Then** it still shows the state icon
   (feature 905), the unread count (feature 575), the pull request indicator (feature 040), the
   ticket status chip (feature 1101), the hover actions (start, delete) and the label chips it has
   today (*current session*, the daemon's name, *outside this app*, *can be removed*, a run's number
   and CLI); the type and issue tags are no longer drawn as separate chips, because the second line's
   short branch carries them.

---

### User Story 4 - Filter by tag and act from one chip bar (Priority: P2)

Today the tag filters hide behind a filter button in the sidebar header. With this change the chip
bar under the Overview row shows the filter chips at all times, an **All** chip first, with the
add-worktree and hide-sidebar actions at its end.

**Why this priority**: Filtering is unchanged in behaviour; only where its chips sit changes.

**Independent Test**: With worktrees of several types, press chips in turn and check which rows
show, that All clears the filters, and that the add and hide actions work.

**Acceptance Scenarios**:

1. **Given** worktrees with tags, **When** the sidebar is shown, **Then** the chip bar shows an
   **All** chip followed by one chip per tag filter on offer today (each type present, *issue*,
   *untyped*, in today's order), without opening any panel.
2. **Given** no filter is active, **When** the chip bar is shown, **Then** **All** looks selected;
   **When** the user presses a type chip, **Then** only that type's rows show, **All** no longer
   looks selected, and pressing more chips combines them with OR, as today (feature 008).
3. **Given** filters are active, **When** the user presses **All**, **Then** every filter clears in
   one press, as **Clear filters** does today.
4. **Given** a filter that matches nothing, **When** the sidebar is shown, **Then** the message and
   its one-press clear (feature 008 FR-027) are shown, and the Overview row and the chip bar stay.
5. **Given** the chip bar, **When** the user presses its add action, **Then** the new-worktree form
   opens; **When** they press its hide action, **Then** the sidebar collapses to its strip, as the
   header's buttons do today.
6. **Given** the sidebar's other header actions today (refresh the worktree list, attach existing
   sessions, run in parallel), **When** the chip bar is shown, **Then** each is still reachable
   from the sidebar [NEEDS CLARIFICATION: the issue names only add and hide beside the chips, and
   the mockup draws only those two; do refresh, attach existing and run in parallel move into an
   overflow (more) menu at the chip bar's end, stay as icons beside add and hide, or leave the
   sidebar for #1127's title-bar overflow menu?].
7. **Given** more chips than fit one line, **When** the chip bar is shown, **Then** the chips wrap
   onto further lines and the actions stay on the first line; no chip is cut off or hidden.
8. **Given** the worktree holding the current session is excluded by a filter, **When** the sidebar
   is shown, **Then** it stays listed in its state's group with its *current session* chip (feature
   024 FR-012a), unchanged.

---

### User Story 5 - Move through groups by keyboard and pointer (Priority: P2)

The developer moves through the sidebar with the keyboard (feature 803) and opens or closes groups
to keep the list short.

**Why this priority**: The acceptance requires groups to be keyboard-navigable; collapsing is what
keeps Done out of the way.

**Independent Test**: With the keyboard on the sidebar, step through the Overview row, the group
headers and the rows; collapse and expand a group by keyboard and by pointer; use the navigation
chords to reach a location in a collapsed group.

**Acceptance Scenarios**:

1. **Given** the keyboard on the sidebar, **When** the user moves through it, **Then** the focus
   ring visits the Overview row, the chip bar's controls, each group header and each visible row,
   top to bottom, in the order they are shown.
2. **Given** the keyboard on a collapsible group header, **When** the user presses Enter or Space,
   **Then** the group collapses or expands; **When** they press the header with the pointer,
   **Then** the same happens. Which groups are collapsible: [NEEDS CLARIFICATION: the mockup's
   open question "keep the sidebar groups collapsible?"; only Done (collapsed at start, the
   issue's "Done collapsed") or every group (Done collapsed at start, the others expanded)?].
3. **Given** a group the user collapsed or expanded, **When** rows move between groups, the user
   switches project and back, or the worktree list is re-read, **Then** that group stays as the
   user left it for the rest of the run.
4. **Given** a navigation chord (feature 803) or a reveal (feature 024, the Overview, the quick
   switcher, a notification) that targets a location in a collapsed group, **When** it runs,
   **Then** the group expands, the row is selected and scrolled into view; the chords step through
   locations in the sidebar's grouped order.
5. **Given** the keyboard on a row whose location changes state, **When** the row moves to another
   group, **Then** the keyboard stays on that row, not on whatever row takes its old place.
6. **Given** any location row, **When** the user presses or right-clicks it, **Then** the row's
   ripple and the context menu are unchanged: the same menu, items and
   dismissal rules as today, now covering both lines of the row.

---

### Edge Cases

- **No project open**: the sidebar shows no Overview row, chip bar or group, as it shows no rows
  today.
- **Project with no worktrees**: the Overview row, the chip bar (with **All** only) and the
  *Default* row in its state's group show, with the "No worktrees yet" hint below.
- **Every location in one state**: one group header; the others are not shown.
- **Run groups** (feature 483): a run group's row is listed in the group of the worst card state
  among its runs' worktrees (a run without a worktree counts as Idle, a failed run as Needs
  attention), its runs nested below it as today; the group's count counts it as one row.
- **Session sub-rows**: a location with several sessions keeps its expander and its session rows
  below it inside its group (feature 551), and they move with it.
- **Several sessions in one location** (Principle II): the location's group comes from its card
  state over all its un-archived sessions; a location is never listed in two groups.
- **State changes while a row's context menu is open or the row is hovered**: the menu stays on its
  location and the row moves; a row moved out from under the pointer loses its hover, as when the
  list scrolls (feature 017 FR-009 dismissal rules apply to a moved row as to a scrolled one).
- **Many state changes in a short time** (several agents finishing at once): every row is in its
  latest state's group after the update; no row is shown twice or missing in between.
- **Current session in a collapsed group** at start (for example a merged worktree): the Done group
  expands to show it, as a reveal does.
- **Very long issue title or branch**: ellipsized on its line; the full text is in the tooltip.
- **Branch with no type and number** (`main`, `claude/project-thread-22fhnj`): the second line shows
  the branch as it is, ellipsized, as on the card (feature 1120 FR-007).
- **Missing or invalid worktree**: its title is drawn in the error colour and it keeps its status
  tag; its group is its card state's (Idle unless a session or pull request says otherwise).
- **Sidebar resized to its narrowest**: both lines and the group headers ellipsize; the count stays
  visible in the header; nothing overlaps.
- **Sidebar hidden, Overview full screen**: the collapsed strip and full screen (feature 970) are
  unchanged; the Overview row comes back with the sidebar.
- **Both themes**: group header text, both row lines, the selected row's text and the Overview
  badge meet 4.5:1 against what they sit on in light and dark.
- **Platforms and display scale** (Principle VI): the grouping, the two-line rows and the keyboard
  order are the same on Linux, macOS and Windows, at display scale 1.0 and 2.0; macOS shows Cmd
  where the others show Ctrl in tooltips.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The sidebar MUST list its location rows in state groups, in the order Needs action,
  Working, Idle, Done, each group holding the locations whose card state (feature 813, the same
  state the location's Overview card shows) belongs to it. A location in Needs attention MUST be
  listed in the Needs action group; there is no separate Needs attention group.
- **FR-002**: Each group MUST have a header reading the group's name, " · " and the number of
  location rows in it after filtering (a run group counts as one); a group with no row MUST NOT be
  shown.
- **FR-003**: Within a group, rows MUST keep the order the sidebar uses today (the *Default* entry
  before worktrees, worktrees in their present order, run groups where they are placed today).
- **FR-004**: A location whose card state changes MUST move to its new group in the same update that
  changes its state anywhere else in the window; counts MUST follow.
- **FR-005**: The Done group MUST be collapsed when a project is first shown in a run, unless it
  holds the current session's row (then FR-006 expands it); groups MUST be
  collapsible as US5 scenario 2 resolves; a group's collapsed state MUST hold for the rest of the run
  per project.
- **FR-006**: A reveal or navigation chord targeting a location in a collapsed group MUST expand that
  group before selecting and scrolling to the row; navigation chords MUST step through locations in
  the grouped order (feature 803's location order becomes this order).
- **FR-007**: The sidebar's first row MUST be the Overview row (icon and "Overview"); activating it
  MUST open the Overview, and MUST do nothing more when the Overview is already open; it MUST be
  inert while Settings is open or no project is active.
- **FR-008**: The Overview row MUST carry the same attention badge (state, count, words) as the top
  bar's Overview button, computed from every location of the project regardless of filters.
- **FR-009**: While the Overview is open the Overview row MUST wear the selected look and no
  location row MUST; otherwise the row holding the viewed session (or the row the Overview sent
  the user to) MUST.
- **FR-010**: The selected row's fill MUST be the primary container colour with on-primary-container
  text and icons.
- **FR-011**: Each location row MUST have two lines: the first is the title by feature 1120 FR-006
  (the *Default* row's is "Default"); the second is the short branch (feature 1120 FR-007), " · " and
  the sessions line (feature 1120 FR-008), with the branch left out when the title already is the
  branch, and "project root" in its place for the *Default* row.
- **FR-012**: A line that does not fit MUST end in an ellipsis; the row's tooltip MUST show the full
  title and branch in addition to what it shows today (feature 029, 905, 575, 040).
- **FR-013**: Each location row MUST keep the state icon, unread count, pull request indicator,
  ticket status chip, hover actions, run annotation and label chips (*current session*, daemon,
  *outside this app*, *can be removed*, missing/invalid status) it shows today; the type tag and
  the issue tag MUST NOT be drawn as chips on the row.
- **FR-014**: Under the Overview row the chip bar MUST show an **All** chip and one chip per tag
  filter on offer (feature 008), always visible; **All** MUST look selected when no filter is active
  and MUST clear every filter when pressed; the filters' meaning (OR across chips, the
  current-session exception, the no-match message) MUST be unchanged. The filter button and its
  accordion are removed.
- **FR-015**: The chip bar MUST end with the add-worktree and hide-sidebar actions, which do what the
  header's buttons do today; the refresh, attach-existing and run-in-parallel actions MUST stay
  reachable from the sidebar as US4 scenario 6 resolves, with today's enabled rules and tooltips.
- **FR-016**: Chips that do not fit one line MUST wrap to further lines; no chip or action may be cut
  off or overlap another.
- **FR-017**: The Overview row, every chip, every group header and every row MUST be reachable by
  keyboard in display order with a visible focus ring; Enter or Space on a collapsible header MUST
  toggle it; the keyboard MUST stay on a row that moves to another group.
- **FR-018**: The ripple, the context menu (items, placement, dismissal) and right-click on a
  location row MUST be unchanged, covering the whole two-line row.
- **FR-019**: Group header text, both row lines, the selected row's text and the Overview row and
  its badge MUST have a contrast of at least 4.5:1 against their background in both themes.
- **FR-020**: Which group a location belongs to, the group order, the order within a group, the
  counts, which groups show, and the second line's text MUST be computed in the render-free core and
  unit-tested without the GUI (Principle I).
- **FR-021**: The sidebar MUST be built from the shared UI components with colours from the theme's
  roles (Principle VIII); the component showcase MUST show the Overview row with and without a
  badge, a group header, and a two-line row unselected and selected.
- **FR-022**: The user guide's sidebar sections (browsing worktrees, filtering by tag, the
  Overview's opening section) MUST describe the Overview row, the chip bar, the groups and the
  two-line rows; the recorded layout snapshot MUST be updated for the new sidebar; a visual pass
  MUST compare the sidebar with the mockup in both themes (Principle VII).
- **FR-023**: Issue titles and branch names MUST stay on this machine (feature 813 FR-020); nothing
  new is logged.

### Key Entities

- **Sidebar group**: a card state group (name, state(s) it holds, collapsed or not, its rows in
  order, its count); derived each render, only the collapsed flag is remembered.
- **Location row text**: the row's first line, second line and tooltip, derived from the same card
  model the Overview uses; not stored.
- **Overview row**: label, icon, badge (from feature 813's attention summary), selected or not,
  enabled or not.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: With one location in each of the five card states, every location is listed under
  exactly one group header, the headers appear in the order of FR-001, and each header's count
  equals the rows under it (or would be under it, for collapsed Done).
- **SC-002**: The Overview row is the first row in every sidebar with a project open, and its badge
  equals the top bar's badge in 100% of the tested attention cases (none, attention only, action
  only, both).
- **SC-003**: For the examples in Story 3, both lines of each row read exactly as stated.
- **SC-004**: In both themes, every text colour in the sidebar's new parts has a contrast of at least
  4.5:1 against its background.
- **SC-005**: Every interactive element of the sidebar (Overview row, chips, actions, group headers,
  rows) can be reached and activated with the keyboard alone.
- **SC-006**: With the same filters active, the same location rows are listed as before the change,
  and a row's ripple and context menu items are identical to before the change.
- **SC-007**: A location whose state changes appears in its new group in the same update as its
  Overview card changes state.

## Out of scope

- Removing the top bar's Overview button or redesigning the title bar (#1127).
- The status bar (#1124), the worktree header and session tabs above the terminal (#1126), and the
  Overview cards' look (#1298).
- New card states, new state rules, or a different order within a state.
- A setting to turn grouping off, or to choose which groups collapse at start.
- Remembering collapsed groups across restarts.
- Removing the session sub-rows under a worktree (the session tabs of #1126 may later replace them).

## Assumptions

- The groups use the Overview's card state (feature 813), not feature 905's location state, so the
  sidebar and the Overview agree; feature 905's state icon stays on the row.
- Done is listed last, after Idle, as the issue and the mockup order it, although the card state
  order puts Done before Idle: a merged worktree is the least likely to need the user.
- "Session state" on the second line is feature 1120's sessions line ("1 session · 1 running"), the
  "counts text" the issue says comes from #1120's core logic; the state's reason stays in the
  tooltip.
- The *Default* row keeps "Default" as its title, as the mockup shows, rather than taking its branch
  as its card does; "project root" stands where a worktree shows its branch.
- Pressing the Overview row opens and never closes the Overview; the top bar button and the shortcut
  keep toggling it.
- The mockup's Overview row fill (secondary container) marks it as the open destination; the
  selected look of FR-009 and FR-010 is used for it while the Overview is open.
- The collapsed state of groups is kept per project for the run, like a worktree's expansion today,
  and is not stored on disk.
- The filter chips keep their current labels (`fix`, `feat`, *issue*, *untyped*); the mockup's
  `bug` is a sample label.
- The sidebar's "Worktrees" header title is removed with the filter button; the Overview row and
  the group headers say what the list is.

---

description: "Tasks: Sidebar grouped by state, with Overview as its first row"
---

# Tasks: Sidebar grouped by state, with Overview as its first row

**Input**: Design documents from `/specs/1125-sidebar-state-groups/`

**Prerequisites**: plan.md (Design, Requirement map, Delivery order), spec.md (US1–US5), research.md
(R1–R16), data-model.md, contracts/sidebar-groups.md (G1–G6, T1), contracts/sidebar-ui.md (S1–S8),
quickstart.md

**Tests**: Constitution Principle I: every test task is written first and run, and must fail for the
stated reason (missing item, or today's ungrouped / one-line / filter-panel behaviour) before the
implementation task it covers. Core: `mise run test-core`. Client:
`scripts/build-lock.sh cargo test -p micold-client --test <name>` and
`scripts/build-lock.sh cargo test -p micold-client --lib -- <filter>`. `ui/sidebar.rs` is view glue
(Principle I exception): it composes answers the render-free tests pin, and is checked by the
layout snapshot, the containment gate and quickstart §B.

**Documentation**: Principle VII: each milestone's user-guide task ships in that milestone, so CI's
user-guide gate (`scripts/check-user-guide-updated.sh`) passes on every `feat` PR. The layout
snapshot fixture is regenerated in every milestone that changes the sidebar's drawn tree
(`UPDATE_LAYOUT_SNAPSHOT=1 scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot`),
never hand-edited or hand-merged; read its diff: only sidebar paths and covered states this feature
names may change.

**Cross-platform**: Principle VI: no platform code; the Overview row's tooltip chord comes from
`keymap::overview_chord_label()`.

**Organization**: by user story, in implementation order. US3 (P1) is implemented before US2 (P1)
because the Overview row's selected look (US2.4) is the `Selection::Primary` US3 introduces. Group
collapsing, reveals and grouped navigation (US5.2–US5.4) ship with US1, because a collapsed Done
without them would hide a revealed row (feature 024 regression); US5's phase keeps the rest (keyed
rows, hover on a moved row).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)

---

## Phase 1: Setup (Shared Infrastructure)

None: no new crate, dependency or fixture.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the core grouping rules and the one card-state function the sidebar and the Overview
share.

- [x] T001 [P] Create `crates/micold-core/src/sidebar_groups.rs` (`pub mod sidebar_groups;` in `crates/micold-core/src/lib.rs`) with failing `mod tests` for contract G1–G6 (contracts/sidebar-groups.md): G1 `StateGroup::of` over every `CardState::ALL` entry (`NeedsAction` and `NeedsAttention` → `NeedsAction`, `Working`, `Idle`, `Done` to their own); G2 `StateGroup::ALL == [NeedsAction, Working, Idle, Done]` and `Ord` agrees with it; G3 `group_by_state` returns only non-empty groups in G2 order, each group's items in input order, every input item in exactly one group, a Needs attention and a Needs action item keep their input order, empty input → empty output; G4 `header_text(group, count)` = `"{label} \u{b7} {count}"` ("Needs action · 2", "Working · 1", "Idle · 3", "Done · 4"); G5 `group_open(choice, group, holds_current)` for every row of the contract's table and `starts_open()` false only for `Done`; G6 `run_group_state`: worst over runs, `RunCard::Worktree(s)` as `s`, `NoWorktree { failed: true }` as `NeedsAttention`, `NoWorktree { failed: false }` as `Idle`, `Idle` for no runs
- [x] T002 Implement `crates/micold-core/src/sidebar_groups.rs` per data-model.md: `StateGroup` (derives `Copy`, `Hash`, `Ord` in display order; `ALL`, `of`, `label`, `starts_open`), `Group<T> { group, items }`, `group_by_state(items: impl IntoIterator<Item = (CardState, T)>) -> Vec<Group<T>>`, `header_text`, `group_open(choice: Option<bool>, group, holds_current: bool) -> bool`, `RunCard`, `run_group_state(runs: impl IntoIterator<Item = RunCard>) -> CardState`; pure (no I/O, clock or logging, FR-023); module doc names feature 1125 and FR-020; T001 passes (`mise run test-core`)
- [x] T003 [P] In `crates/micold-client/src/features/overview.rs` `mod tests`, add failing tests for `location_card_state(&State, &SessionLocation, Option<&str>) -> (CardState, CardReason)`: for a location with a permission prompt, one with failing checks, one with a running session, one with a merged pull request and one with no session it returns `NeedsAction`, `NeedsAttention`, `Working`, `Done`, `Idle`; and for each, the `state` of the card `ui::overview::cards(state)` builds for that location is the same value (US1.7, R2)
- [x] T004 Move the card-state half of `ui/overview.rs::card` (building `CardSignals` from the location's un-archived sessions, pending agent requests and pull request signal, then `micold_core::overview::card_state`) and `pr_signal` into `crates/micold-client/src/features/overview.rs` as `location_card_state` and `pr_signal` (R2); `crates/micold-client/src/ui/overview.rs` `card` calls it; no behaviour change (existing Overview tests unmodified); T003 passes

**Checkpoint**: the core answers which group, which order, which count and open or closed; the
client has one card-state function.

---

## Phase 3: User Story 1 - See what needs me, grouped (Priority: P1) 🎯 MVP

**Goal**: the sidebar's list is grouped under "Needs action · n", "Working · n", "Idle · n",
"Done · n" headers, Done collapsed at start, every header toggles, reveals and navigation chords
open a closed group (FR-001–FR-006, US5.2–US5.4). Rows keep today's look until Phase 4; the old
sidebar header (title, filter button, actions) stays until Phase 6.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test sidebar_state --test sidebar_tree --test features_navigation --test navigation_apply`; in the client, a project with locations in several states shows the headers in order, Done collapsed.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [x] T005 [P] [US1] In `crates/micold-client/tests/features_sidebar.rs`, add failing tests for `State::sidebar_groups()`: with *Default* and worktrees in each of the five card states, groups come in the order Needs action, Working, Idle, Done, the Needs attention worktree sits in Needs action in `sidebar_entries()` order with the Needs action one (US1.1, US1.2, SC-001); *Default* is before worktrees in its group (US1.5, FR-003); a group whose only worktree a tag filter hides is absent (US1.4, FR-002); a run group sits in the group of `run_group_state` over its runs and counts as one entry (Edge Cases *Run groups*); each entry's group is `StateGroup::of(location_card_state(..).0)` (US1.7); flipping one worktree's session from idle to running in the same `State` moves it from Idle to Working and both counts change (US1.6, FR-004, SC-007); the current session's worktree excluded by a filter stays listed in its state's group (US4.8, feature 024 FR-012a); no project open → no groups
- [x] T006 [P] [US1] In `crates/micold-client/tests/sidebar_state.rs`, add failing tests for `State::sidebar_list()` and `SidebarMsg::GroupToggled(StateGroup)`: a fresh project's list is header, entries for each open group and the Done header alone (US1.3); Done holding the current session's worktree starts open (G5, Edge Cases); `GroupToggled(Done)` opens a closed Done, and on a Done open only because it holds the current session it closes it (toggle against the effective state, R4); `GroupToggled(Working)` closes Working (US5.2); the choice survives a worktree moving into or out of the group, `worktrees_replaced` and a switch to another project and back, and is per project (US5.3, FR-005); a worktree whose row context menu is open keeps `worktree.menu_open` on its location when it changes group (Edge Cases)
- [x] T007 [P] [US1] In `crates/micold-client/tests/sidebar_tree.rs`, add failing tests for `row_heights`, `row_index`, `location_row` and `current_session_row` taking `&[ListRow]`: a header is one row of one-line dense height, location and session rows keep today's heights, the rows of a closed group are absent, a target in a closed group gives `None`; `revealed_scroll_offset` counts headers (contract S7)
- [x] T008 [P] [US1] In `crates/micold-client/tests/features_navigation.rs` and `crates/micold-client/tests/navigation_apply.rs`, add failing tests: `navigation::location_nodes` steps through locations in `sidebar_groups()` order, closed groups included, collapsed run groups left out as today (FR-006); a next-location chord landing on a location in a closed Done opens Done (a recorded `true` choice), selects the row, and the reveal offset puts it fully in view; `open_group_of_reveal_target()` opens the closed group of the current session's location and of `sidebar.revealed` (the Overview's reveal) and leaves an open group's choice unrecorded (US5.4, R5)
- [x] T009 [P] [US1] In `crates/micold-client/src/ui/material/tree_view.rs` `mod tests`, add failing tests for `TreeItem::heading()`: the row is one-line dense height, draws no icon slot, its twisty shows open or closed from `expanded`, a pointer press and Enter or Space while it has the keyboard (`keyboard_focus`) publish its `on_press` message, its label is drawn in `TypeRole::SidebarGroup` (US5.2, FR-017, R8); and in `crates/micold-client/src/ui/material/type_role_mapping.rs` tests that `TypeRole::SidebarGroup` is 12 px bold
- [x] T010 [P] [US1] In `crates/micold-client/src/ui/material/composition_contrast.rs`, add the group header pair (`on_surface_variant` on `surface_container_low`) for light and dark, asserted ≥ 4.5:1 (FR-019, R12)

### Implementation for User Story 1

- [x] T011 [US1] Add `TypeRole::SidebarGroup` (12 px, bold) in `crates/micold-client/src/ui/material/text.rs` and admit it in `crates/micold-client/src/ui/material/type_role_mapping.rs`; add `TreeItem::heading()` in `crates/micold-client/src/ui/material/tree_view.rs` (label in `SidebarGroup`, `on_surface_variant`, no state icon, twisty from `expanded`, whole row pressable, one-line dense height, the same keyboard-focus wrapper as every row) (R8); T009 passes
- [x] T012 [US1] In `crates/micold-client/src/features/sidebar.rs`: `SidebarGroup { group, open, entries }`, `GroupHeaderRow { group, count, open }`, `ListRow { Header(GroupHeaderRow), Entry(SidebarEntry) }`; `State::sidebar_groups()` (each `sidebar_entries()` entry's state from `features::overview::location_card_state`, a run group's from `run_group_state` over its runs, then `group_by_state`; `open` from `group_open(None, group, holds_current)` until T013, with `holds_current` from `current_session_location()`, a run group's runs included); `State::sidebar_list()` (each group's header, then its entries when open); module doc lists the new types; T005 passes
- [x] T013 [US1] In `crates/micold-client/src/features/sidebar.rs`: `group_choices: BTreeMap<PathBuf, BTreeMap<StateGroup, bool>>` on `sidebar::State` (not pruned, not persisted, untouched by `project_entered`), read by `sidebar_groups()` as `group_open`'s `choice`; `Msg::GroupToggled(StateGroup)` storing `!open` (the effective state) for the active project (R4); module doc lists the message and field; T006 passes
- [x] T014 [US1] In `crates/micold-client/src/features/sidebar.rs`: `row_heights`, `row_index`, `location_row` and `current_session_row` take `&[ListRow]` (a header one one-line row, closed groups' rows absent); update their callers (`revealed_scroll_offset`, the reveal offsets in `crates/micold-client/src/main.rs`, the keyboard row) and their test callers (`crates/micold-client/tests/features_sidebar.rs`, `sidebar_state.rs`, `overview.rs`) to pass `sidebar_list()`; T007 passes
- [x] T015 [US1] Add `State::open_group_of_reveal_target()` in `crates/micold-client/src/features/sidebar.rs` (target: the chord's `keyboard_row`, else the current session's location, else `sidebar.revealed`; records `true` for its group when closed); call it first in `reveal_scroll` and `revealed_scroll` in `crates/micold-client/src/main.rs`, before the "is the row listed" check; `crates/micold-client/src/features/navigation.rs` `location_nodes` walks `sidebar_groups()` in order, closed groups included (R5); T008 passes
- [x] T016 [US1] In `crates/micold-client/src/ui/sidebar.rs`, draw the list from `sidebar_list()`: a `ListRow::Header` as a `TreeItem::heading()` with `header_text(group, count)`, `expanded = open`, `on_press(SidebarMsg::GroupToggled(group))`; entries as today (FR-001, FR-002, S4); the header row, filter accordion and offer banner unchanged
- [x] T017 [P] [US1] In `crates/micold-client/src/showcase/sections/atoms.rs` (TreeView entry), add a group header open and closed (FR-021, R15); `scripts/build-lock.sh cargo test -p micold-client --test showcase_completeness` passes
- [x] T018 [US1] In `crates/micold-client/tests/support/covered_states.rs`, add covered state `main-shell-sidebar-grouped` (*Default* and one worktree per card state, Done collapsed by default) and re-point any sidebar anchor whose tree path the header rows shift; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; list the new state in `docs/development/layout-snapshot.md`; `--test layout_snapshot --test layout_text_overflow` and `tests/gates/containment.rs` pass
- [x] T019 [US1] In `docs/user-guide/worktrees-and-sessions.md` *Browsing worktrees*: the four state groups and their order, counts, Done collapsed at start, a header press (or Enter/Space) collapses or expands a group for the rest of the run per project, reveals and navigation chords open a collapsed group and step in grouped order, a Needs attention worktree is under Needs action (FR-022)

**Checkpoint**: grouped sidebar on `main`; chords and reveals reach rows in collapsed groups.

---

## Phase 4: User Story 3 - Rows say what each worktree is for, on two lines (Priority: P1)

**Goal**: every location row is two lines (title; short branch · sessions line), type and issue tag
chips are gone from rows, the selected row is filled with the primary container (FR-010–FR-013,
FR-018).

**Independent Test**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test sidebar_tree --lib -- tree_view composition_contrast`.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [x] T020 [P] [US3] In `crates/micold-core/src/overview_text.rs` `mod tests`, add failing tests for `sidebar_second_line(row: RowKind, title, branch, sessions)` covering every row of contract T1's table ("project root · no sessions", "project root · 2 sessions, 1 running", "fix/1117 · 1 session running", "1 session", "main · no sessions", "No sessions") (US3.1–US3.4, SC-003)
- [x] T021 [P] [US3] In `crates/micold-client/src/ui/material/tree_view.rs` `mod tests`, add failing tests: `.supporting_text(text, tint)` draws a second line under the label at the label's indent, ellipsized when narrower than the text, and makes the row `density::LIST_ROW_TWO_LINE_BASE` tall; `.leading_chip(element)` sits first after the supporting text on the second line; `TreeView::selection(Selection::Primary)` fills the selected row with `primary_container` and draws the label and supporting text in `on_primary_container`, while the default (`Selection::Secondary`) still draws today's `secondary_container` pill; the ripple bounds and the right-press area of a two-line row cover both lines (FR-010, FR-011, FR-012, FR-018, R6, R7)
- [x] T022 [P] [US3] In `crates/micold-client/tests/features_sidebar.rs`, add failing tests for the row's render-free text and chips: a worktree with a known issue title gives first line the title and second line "fix/1117 · 1 session running"; a branch-only worktree gives the branch and "1 session"; *Default* gives "Default" and "project root · …"; a location with no session ends in "No sessions" (US3.1–US3.4); the row's label chips (a render-free helper the view draws exactly) hold *current session*, the daemon name, *outside this app*, *can be removed* and missing/invalid status as today and no type or issue tag (US3.7, FR-013); `worktree_tooltip` begins with the full title, then a `Branch` line with the full branch, then today's lines unchanged (US3.5, FR-012)
- [x] T023 [P] [US3] In `crates/micold-client/tests/sidebar_tree.rs`, change `row_heights` expectations: every location row (*Default* included) is two-line dense height; headers and session rows stay one-line (R6, S7)
- [x] T024 [P] [US3] In `crates/micold-client/src/ui/material/composition_contrast.rs`, add for light and dark: first line (`on_surface` on `surface_container_low`), second line (`on_surface_variant` on `surface_container_low`), selected first and second line (`on_primary_container` on `primary_container`), each ≥ 4.5:1 (FR-019, SC-004)

### Implementation for User Story 3

- [x] T025 [US3] Add `RowKind { ProjectRoot, Worktree }` and `sidebar_second_line` in `crates/micold-core/src/overview_text.rs`, composed from `branch_line` and `meta_sessions` (no copy of either); T020 passes
- [x] T026 [US3] In `crates/micold-client/src/ui/material/tree_view.rs`: `TreeItem::supporting_text(text, tint)` (`Ellipsized`, `TypeRole::SidebarTag`-sized caption, two-line base height), `TreeItem::leading_chip(element)` replacing `chip_after_tag` (its only caller is the sidebar), `Selection { Primary, Secondary }` and `TreeView::selection(Selection)` defaulting to `Secondary`; builders stay chainable; T021 passes
- [x] T027 [US3] In `crates/micold-client/src/features/sidebar.rs`: the row text helpers (first line per data-model *Row text*, second line from `sidebar_second_line`), the label-chip helper without type and issue tags (`node.tags` still drives filters), `worktree_tooltip`'s title and `Branch` lines, `row_heights` two-line for every location row; T022, T023 pass
- [x] T028 [US3] In `crates/micold-client/src/ui/sidebar.rs` (`build_items`, `build_default_item`, `group_items`): every location row gets `.supporting_text(..)`, the ticket status chip as `.leading_chip(..)`, the label chips from T027's helper in today's order, the title in the error colour for a missing or invalid worktree; the list's `TreeView` uses `Selection::Primary`; state icon, unread count, pull request indicator, hover actions, run annotation, expander and session sub-rows unchanged (US3.6, US3.7, S5); T024 passes
- [x] T029 [P] [US3] In `crates/micold-client/src/showcase/sections/surfaces.rs` (the TreeView entry, per D15), add a two-line row unselected and selected with `Selection::Primary` (FR-021)
- [x] T030 [US3] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; `--test layout_snapshot --test layout_text_overflow` and `tests/gates/containment.rs` pass with both lines ellipsized at the narrowest covered width (FR-016, Edge Cases *Sidebar resized to its narrowest*)
- [x] T031 [US3] In `docs/user-guide/worktrees-and-sessions.md` *Reading a worktree: name & tags*: the two lines (title, else branch, else name; short branch · sessions line; "Default" / "project root"), type and issue tags now in the short branch rather than chips, the label chips that stay, the tooltip's full title and branch, the selected row's fill (FR-022)

**Checkpoint**: two-line rows with the primary-container selection on `main`.

---

## Phase 5: User Story 2 - Overview is the sidebar's first row (Priority: P1)

**Goal**: the sidebar's first row opens the Overview, carries the top bar's badge and tooltip, and is
the only selected row while the Overview is open (FR-007–FR-009).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview composition_contrast`; `--test features_sidebar`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [x] T032 [P] [US2] In `crates/micold-client/src/features/overview.rs` `mod tests`, add failing tests: `Msg::Opened { settings_open: false }` opens a closed Overview, leaves an open one open, and does nothing while an activation is pending; `Msg::Opened { settings_open: true }` does nothing (US2.3, US2.5, R9); `overview_tooltip(badge)` (moved here) returns today's top bar strings for no badge, attention and action (with both present the badge already carries action, `badge` takes the most urgent) (SC-002); with a tag filter active, `badge(&ui::overview::cards(state))` counts only the listed locations (US2.6, D7)
- [x] T033 [P] [US2] In `crates/micold-client/tests/features_sidebar.rs`, add failing tests for a render-free `row_selected(state, …)`: while `overview.open`, no location or session row is selected; once a location row is selected (the Overview closes, as selecting a session does today) only the row holding the viewed session is; with no session viewed and `sidebar.revealed` set by the Overview, the revealed location's row is (US2.4, FR-009)
- [x] T034 [P] [US2] In `crates/micold-client/src/ui/material/composition_contrast.rs`, add for light and dark: the Overview row label on `surface_container_low` and on `primary_container` (`on_primary_container`), and the badge pairs `on_error_container`/`error_container` and `on_warning_container`/`warning_container`, each ≥ 4.5:1 (FR-019)
- [x] T035 [P] [US2] In `crates/micold-client/src/ui/sidebar.rs` `mod tests` (mount `view` with `ui::material::test_support::Mounted`, as `ui/overview.rs` tests do), add failing tests: the sidebar's first focusable is the Overview row, Enter on it publishes `overview::Msg::Opened { settings_open: false }`; with Settings open it publishes nothing; with no project open the view has no Overview row (US2.1, US2.3, US2.5, Edge Cases *No project open*, FR-017)

### Implementation for User Story 2

- [x] T036 [US2] In `crates/micold-client/src/features/overview.rs`: `Msg::Opened { settings_open }` (opens like `Toggled`, no-op when open, while Settings is open or an activation is pending) and `overview_tooltip` moved from `crates/micold-client/src/ui/toolbar.rs`, which now calls it (nothing else in the toolbar changes, #1127 owns it); T032 passes
- [x] T037 [US2] In `crates/micold-client/src/features/sidebar.rs`, add `row_selected` and use it in `crates/micold-client/src/ui/sidebar.rs` for location and session rows; T033 passes
- [x] T038 [US2] In `crates/micold-client/src/ui/sidebar.rs`, add the Overview row first, above the header row and outside the scroll region: a one-item `TreeView` with `Selection::Primary`, `TreeItem::new(0, "Overview", on_surface).with_icon(Icon::Overview)`, `.selected(state.overview.open)`, `.on_press(overview::Msg::Opened { settings_open })` only while a project is active and Settings is closed, a trailing solid `Tag` with `badge.label` in the tone's container pair from `features::overview::badge(&ui::overview::cards(state))`, the tooltip from `overview_tooltip`; no Overview row with no project open (US2.1–US2.5, S1, S2); T034 and T035 pass
- [x] T039 [P] [US2] In `crates/micold-client/src/showcase/sections/surfaces.rs` (the `TreeView` entry, beside the sidebar's poses; D17), add the Overview row with and without a badge, and selected (FR-021)
- [x] T040 [US2] In `crates/micold-client/tests/support/covered_states.rs`, open the Overview in `main-shell-sidebar-grouped` (the selected Overview row is recorded, R15); regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; `--test layout_snapshot` and `tests/gates/containment.rs` pass
- [x] T041 [US2] In `docs/user-guide/overview.md` opening section: the sidebar's first row opens the Overview (it never closes it; the top bar button and Ctrl+Shift+O / Cmd+Shift+O toggle), its badge equals the top bar's, it looks selected while the Overview is open; in `docs/user-guide/icons.md` add the sidebar's Overview row to the *Dashboard (grid)* line's locations (FR-022)

**Checkpoint**: the Overview row is first in every sidebar with a project open, badge equal to the
top bar's.

---

## Phase 6: User Story 4 - Filter by tag and act from one chip bar (Priority: P2)

**Goal**: the chip bar under the Overview row shows **All** and the filter chips at all times, with
add, hide and ⋯ (refresh, attach existing, run in parallel); the header row, filter button and
accordion are gone (FR-014–FR-016).

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test overlay_registry --test overlay_dispatch_ordering --test navigation --test icons`; `--test layout_snapshot`.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [x] T042 [P] [US4] In `crates/micold-client/tests/features_sidebar.rs`, add failing tests: the chip bar's chips are **All** then `available_tag_filters()` in order (US4.1); **All** is active iff `filters.is_empty()` and `FiltersCleared` clears every active filter in one message (US4.2, US4.3); the more menu's items are, in order, refresh, attach existing, run in parallel, each disabled exactly when today's header button is (`can_refresh_worktrees()`, a project is active, a project is active) and with today's tooltip (`HEADER_ACTIONS` and the header buttons' texts); `MoreMenuToggled` opens and closes it, choosing an item sends today's message and closes it, `MoreMenuAnchored` records the anchor (US4.6, FR-015); existing filter tests (OR, no-match message, current-session exception) unmodified (US4.4, US4.8)
- [x] T043 [P] [US4] In the overlay test family, migrate every use of `SidebarFilterPanel`, `sidebar.filter_open` or `SidebarMsg::FilterMenuToggled` as the sample popover to `SidebarMoreMenu` / `more_menu_open` / `MoreMenuToggled`: `crates/micold-client/tests/overlay_registry.rs`, `overlay_dispatch_ordering.rs`, `overlay_registration.rs`, `overlay_dismissal_delta.rs`, `popover_displacement.rs`, `surface_registration_cost.rs`, `switcher_forget_menu.rs`, `root_state_is_shared.rs`, `inventory/mod.rs`; add failing cases: `SidebarMoreMenu` is a Popover-layer floating surface, closes on Escape, on an outside press and when a displacing surface opens (feature 017) (FR-015, S3)
- [x] T044 [P] [US4] In the sidebar, navigation and layout tests, make the same migration: `crates/micold-client/tests/navigation.rs`, `sidebar_state.rs`, `sidebar_tree.rs`, `terminal_focus.rs`, `layout_text_overflow.rs`, `gates/containment.rs`, `support/covered_states.rs`; afterwards `grep -rln 'SidebarFilterPanel\|filter_open\|FilterMenuToggled' crates/micold-client/tests` prints nothing
- [x] T045 [P] [US4] In `crates/micold-client/src/ui/sidebar.rs` `mod tests`, add a failing Tab-order test: with a project open, filters on offer and groups shown, `focus_next` from the top visits the Overview row, **All**, each filter chip, add, hide, ⋯, then each group header and each visible row, in display order, each control with a focus ring (US5.1, FR-017, SC-005)
- [x] T046 [P] [US4] In `crates/micold-client/tests/icons.rs`, pin `Icon::MoreHorizontal` to U+E5D3 (`more_horiz`)

### Implementation for User Story 4

- [x] T047 [US4] Add `Icon::MoreHorizontal` (U+E5D3) in `crates/micold-client/src/icons.rs` and its row in `assets/fonts/PROVENANCE.md`; T046 passes
- [x] T048 [US4] In `crates/micold-client/src/features/sidebar.rs` and `crates/micold-client/src/overlay/registry.rs`: `more_menu_open`, `more_menu_anchor: Option<(u16, u16)>`, `Msg::MoreMenuToggled`, `Msg::MoreMenuAnchored((u16, u16))`, the `SidebarMoreMenu` floating surface (Popover, cancelled by `MoreMenuToggled`) in the registry's surface list and displacement sets where `SidebarFilterPanel` was, the more menu's items with their enabled rules and tooltips; remove `filter_open`, `Msg::FilterMenuToggled`, `SidebarFilterPanel` and their uses in `crates/micold-client/src/app.rs`; update the module docs; T042, T043, T044 pass
- [x] T049 [US4] In `crates/micold-client/src/ui/sidebar.rs`, replace the header row (the "Worktrees" title, filter button and actions) and `filter_bar`'s accordion with the chip bar under the Overview row: a wrapping `Row` (`row().wrap()`) of `ToggleChip`s (**All** sending `FiltersCleared`, then `filter_chip`s) left of add (`Icon::AddWorktree`, `FormMsg::Opened`), hide (`Icon::HideSidebar`, `SidebarMsg::Toggled`) and more (`MenuTrigger` with `Icon::MoreHorizontal`, wrapped in `cdk::ContextArea::on_primary_bounds` sending `MoreMenuAnchored`) on the first line, each taking the keyboard (R11, S3); T045 passes; in `crates/micold-client/src/ui/mod.rs`, add the more menu's `MenuOverlay` surface beside the switcher's, anchored by `features::project::clamp_menu_anchor`; `FilterTrigger`'s doc in the material library stops naming the sidebar (the showcase's accordion sample keeps it)
- [x] T050 [US4] Remove the filter accordion's exemptions from `crates/micold-client/tests/gates/containment.rs` (`CLIP_REVEALED` sidebar entries), `revealing_states()` in `crates/micold-client/tests/support/covered_states.rs`, and the tests that pin the collapsed filter panel's overflow (`grep -rn 'filter_panel\|filter panel' crates/micold-client/tests`, e.g. `the_recorded_overflow_is_the_collapsed_filter_panel`), and from `docs/development/layout-snapshot.md` (the *One exception* paragraph, the `CLIP_REVEALED` entry, the caveat on the collapsed accordion); ensure covered states draw the chip bar at 180, 300 and 600 px sidebar width with six filter chips on offer (four types, *issue*, *untyped*), more than fit one line at 180 px; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`; `--test layout_snapshot --test layout_text_overflow` and the containment gate pass (US4.7, FR-016)
- [x] T051 [US4] Update `docs/development/architecture.md` where it names the sidebar's filter panel; in `docs/user-guide/worktrees-and-sessions.md` *Filtering worktrees by tag* (the chip bar, **All**), *Refreshing the list* and the attach / run-in-parallel mentions (the ⋯ menu), *Resizing and hiding the sidebar* (hide on the chip bar); in `docs/user-guide/icons.md`, the hide-sidebar location ("Sidebar chip bar") and a ⋯ line (FR-022)

**Checkpoint**: chip bar and ⋯ menu on `main`; no filter panel remains.

---

## Phase 7: User Story 5 - Move through groups by keyboard and pointer (Priority: P2)

**Goal**: the keyboard and the hover stay with a row that moves to another group (US5.5, Edge Cases
*State changes while … hovered*). Toggling, chords and reveals shipped in Phase 3.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client --lib -- keyed_column tree_view`; `--test sidebar_tree`.

### Tests for User Story 5 (MANDATORY — Constitution Principle I) ⚠️

- [x] T052 [P] [US5] In `crates/micold-client/src/ui/material/tree_view.rs` `mod tests`, add failing tests: a `TreeView` whose items all carry `.key(u64)` keeps a focused row's keyboard focus (and its ripple and hover state) on that row when a rebuild moves it to another position, with the list's length unchanged (R13, FR-017)
- [x] T053 [P] [US5] In `crates/micold-client/tests/sidebar_tree.rs`, add failing tests for `sidebar::hover_holds(state, dir, index)`: true while the row's current index in `sidebar_list()` equals the recorded one, false once a state change moves it (R14)

### Implementation for User Story 5

- [x] T054 [US5] Move `crates/micold-client/src/ui/transcript/keyed.rs` to `crates/micold-client/src/ui/cdk/keyed_column.rs` with its tests, API unchanged (`ui/cdk/mod.rs` exports it, `ui/transcript/mod.rs` imports it); list `KeyedColumn` under cdk in `crates/micold-client/src/showcase/catalogue.rs`; `--test showcase_completeness` passes
- [x] T055 [US5] Add `TreeItem::key(u64)` in `crates/micold-client/src/ui/material/tree_view.rs`; `TreeView` lays its rows out in `cdk::KeyedColumn` when every item has a key; T052 passes
- [x] T056 [US5] In `crates/micold-client/src/ui/sidebar.rs`, key a location row by a hash of its `RowRef`, a session row by its `SessionId`, a header by its `StateGroup`, a run group by its `GroupId` (R13)
- [x] T057 [US5] `worktree.hovered: Option<(String, usize)>` and `WorktreeMsg::Hovered(String, usize)` in `crates/micold-client/src/features/worktree.rs`; `sidebar::hover_holds` in `crates/micold-client/src/features/sidebar.rs`; `crates/micold-client/src/ui/sidebar.rs` sends the drawn index and shows hover actions only while `hover_holds`; the other `Hovered` sites (`crates/micold-client/src/main.rs`, tests) pass the index; an open row context menu keeps its anchor and location; T053 passes
- [x] T058 [US5] In `docs/user-guide/worktrees-and-sessions.md` *Browsing worktrees*: the keyboard stays on a row that moves to another group; Tab visits the Overview row, the chip bar, each header and each row in display order (FR-022)

**Checkpoint**: the whole feature is on `main`.

---

## Phase 8: Polish & Cross-Cutting Concerns

Code-free; done by the close unit in the close PR.

- [ ] T059 Run quickstart §A (including `! grep -nE 'tracing::|log::|println!|eprintln!|dbg!' crates/micold-core/src/sidebar_groups.rs` and the same pattern over the feature's added lines, `git diff origin/main...HEAD -- crates/ | grep '^+' | grep -E 'tracing::|log::|println!|eprintln!|dbg!'` printing nothing, FR-023) and `mise run gate`; record the result in `specs/1125-sidebar-state-groups/quickstart.md`
- [ ] T060 Run quickstart §B with the `visual-pass` skill against the mockup board's sidebar in both themes; save screenshots under `specs/1125-sidebar-state-groups/visual-pass/` and results in `specs/1125-sidebar-state-groups/visual-pass.md` (FR-022, SC-004, SC-005)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)**: none; blocks every story.
- **US1 (Phase 3)**: after Phase 2.
- **US3 (Phase 4)**: after Phase 3 (row heights over `ListRow`; the grouped list's rows).
- **US2 (Phase 5)**: after Phase 4 (`Selection::Primary`).
- **US4 (Phase 6)**: after Phase 5 (the chip bar sits under the Overview row).
- **US5 (Phase 7)**: after Phase 6 (keys the final row set; the `Hovered` sites are stable).
- **Polish (Phase 8)**: after all.

### Within Each User Story

Tests first and failing; core before client render-free; render-free before shared component;
shared component before `ui/sidebar.rs`; showcase, fixture and user guide last.

### Parallel Opportunities

- Phase 2: T001 beside T003.
- Phase 3: T005–T010 together (different files); T011 beside T012; T013 and T014 after T012; T015 after T014; T017 beside T016.
- Phase 4: T020–T024 together; T025 beside T026 beside T027; T029 beside T028.
- Phase 5: T032–T034 and T035 together; T039 beside T038.
- Phase 6: T042–T046, T044 and T045 together; T047 beside T048.
- Phase 7: T052 beside T053; T054 then T055.

## Parallel Example: User Story 1

```text
T005 features_sidebar   T006 sidebar_state   T007 sidebar_tree
T008 features_navigation + navigation_apply   T009 tree_view heading   T010 contrast
then T011 ‖ T012 → T013 → T014 → T015 → T016 ‖ T017 → T018 → T019
```

## Implementation Strategy

### MVP First

US1 (M1): the sidebar answers "what needs me" by group, with Done out of the way and every reveal
still reaching its row.

### Incremental Delivery

M2 makes rows say what each worktree is for; M3 puts the Overview at the top; M4 replaces the
filter panel with the chip bar; M5 keeps keyboard and hover on moved rows. Each merges green.

## Notes

- Between M1 and M3 the sidebar keeps its "Worktrees" header row with the filter button and actions;
  the Overview row arrives above it in M3, and M4 replaces it with the chip bar.
- Between M1 and M5, a row that moves to another group can leave keyboard focus and hover in its
  old slot (today's index-matched behaviour); M5 fixes it.
- `features/sidebar.rs` and `ui/sidebar.rs` are large: `grep -n` then `Read` with `offset`/`limit`.
- The parallel layout threads (#1124, #1126, #1127) touch `ui/mod.rs`, `covered_states.rs`, the
  fixture, `composition_contrast.rs` and `docs/user-guide/overview.md`: after a rebase, regenerate
  the fixture, never hand-merge it (R16).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Sidebar grouped by state 🎯 MVP

- **Tasks**: T001–T019
- **Deliverable**: on `main`, the sidebar lists its locations under "Needs action · n", "Working · n", "Idle · n" and "Done · n" headers in that order, Needs attention under Needs action, Done collapsed at start; pressing a header (or Enter/Space) collapses or expands it for the run; chords and reveals into a collapsed group open it.
- **Satisfies**: US1 acceptance scenarios 1–7; US5 sc.2–sc.4; US4 sc.8; FR-001–FR-006, FR-017 (header toggle), FR-019 (header), FR-020 (groups), FR-021 (header sample), FR-022 (browsing, snapshot); SC-001, SC-007
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test sidebar_state --test sidebar_tree --test features_navigation --test navigation_apply --test overview --test layout_snapshot --test layout_text_overflow --test showcase_completeness`; `scripts/build-lock.sh cargo test -p micold-client --lib -- tree_view type_role_mapping overview composition_contrast` (the containment gate runs inside `layout_snapshot`); quickstart §B2, §B6 (header and chord half)
- **Depends on**: —
- **Tier**: full
- **Size**: 19 tasks, kept whole: grouping without collapse would leave Done expanded against FR-005, and collapse without the reveal and chord changes would hide revealed rows (a feature 024 regression), so no split along an acceptance scenario leaves a working deliverable. The four Foundational tasks are core rules and one move.

### M2 — Two-line rows with the primary-container selection

- **Tasks**: T020–T031
- **Deliverable**: on `main`, every sidebar location row reads its title on the first line and "fix/1117 · 1 session running" (or "project root · …") on the second, without type or issue chips, and the selected row is filled with the primary container colour.
- **Satisfies**: US3 acceptance scenarios 1–7; US5 sc.6; FR-010–FR-013, FR-018, FR-019 (rows), FR-020 (second line), FR-021 (two-line samples), FR-022 (reading a worktree); SC-003, SC-004 (rows), SC-006 (ripple, menu)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test sidebar_tree --test layout_snapshot --test layout_text_overflow`; `scripts/build-lock.sh cargo test -p micold-client --lib -- tree_view composition_contrast`; quickstart §B3, §B4 (row half), §B8
- **Depends on**: M1
- **Tier**: full
- **Size**: 12 tasks, kept whole: the two lines and the dropped chips are one row change (dropping the chips without the second line would lose the type and issue from the row), and the selected fill must be checked against the two-line row's text.

### M3 — Overview as the sidebar's first row

- **Tasks**: T032–T041
- **Deliverable**: on `main`, the sidebar's first row is Overview with the top bar's badge ("2 need action") and tooltip; pressing it opens the Overview, and while it is open only the Overview row looks selected.
- **Satisfies**: US2 acceptance scenarios 1–6; FR-007–FR-009, FR-019 (Overview row, badge), FR-021 (Overview row samples), FR-022 (Overview opening section); SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- overview composition_contrast`; `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test layout_snapshot --test showcase_completeness`; quickstart §B1 (Overview row half), §B4
- **Depends on**: M2
- **Tier**: full

### M4 — Chip bar and the ⋯ menu replace the filter panel

- **Tasks**: T042–T051
- **Deliverable**: on `main`, under the Overview row the chip bar shows **All** and the tag chips at all times, wrapping when they do not fit, with add, hide and ⋯ (refresh, attach existing, run in parallel); the "Worktrees" header, filter button and accordion are gone.
- **Satisfies**: US4 acceptance scenarios 1–7; US5 sc.1 (Tab order); FR-014–FR-016, FR-017 (chip bar controls, Tab order), FR-022 (filtering, refreshing, hiding; snapshot exemptions); SC-005 (chip bar), SC-006 (filters)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test overlay_registry --test overlay_dispatch_ordering --test overlay_registration --test overlay_dismissal_delta --test popover_displacement --test surface_registration_cost --test switcher_forget_menu --test root_state_is_shared --test navigation --test sidebar_state --test sidebar_tree --test terminal_focus --test icons --test layout_snapshot --test layout_text_overflow`; `scripts/build-lock.sh cargo test -p micold-client --lib -- sidebar`; quickstart §B1 (chip bar half), §B5, §B8
- **Depends on**: M3
- **Tier**: full
- **Size**: 10 tasks; T043 and T044 migrate ~16 test files mechanically, so the diff may pass 800 lines. Kept whole: removing the filter panel and adding the chip bar are one observable change (either alone leaves filters unreachable or offered twice).

### M5 — Keyboard and hover stay on a moved row

- **Tasks**: T052–T058
- **Deliverable**: on `main`, a sidebar row that moves to another group keeps the keyboard focus ring and drops its hover actions until hovered again; `KeyedColumn` is a shared `cdk` component.
- **Satisfies**: US5 acceptance scenario 5; Edge Cases *State changes while a row's context menu is open or the row is hovered*; FR-017; SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- keyed_column tree_view transcript`; `scripts/build-lock.sh cargo test -p micold-client --test sidebar_tree --test showcase_completeness`; quickstart §B6, §B7
- **Depends on**: M4
- **Tier**: full

Phase 8 (T059–T060) changes no code: the close unit does it in the close PR.

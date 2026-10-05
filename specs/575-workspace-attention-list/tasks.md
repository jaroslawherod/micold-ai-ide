---

description: "Task list for #575: attention indicator on the sidebar's worktree rows"
---

# Tasks: Attention Indicator on the Sidebar's Worktree Rows

**Input**: Design documents from `/specs/575-workspace-attention-list/`

**Prerequisites**: plan.md (D1-D6), spec.md, research.md (R1-R9), data-model.md,
contracts/attention-indicator.md (A1-A10), quickstart.md

**Tests**: MANDATORY and first (Constitution I). Within each story, the test tasks come before the
implementation tasks they cover, and each test must be seen failing for the right reason first.
The view wiring in `crates/micold-client/src/ui/sidebar.rs` is glue: it only calls tested code.

**Documentation**: each user-facing change carries its user-guide task (Constitution VII).

**Cross-platform**: no platform code; CI runs the suite on Linux, macOS and Windows (Constitution VI).

**Organization**: grouped by user story. Paths: `crates/micold-core/src`, `crates/micold-client/src`,
client state tests in `crates/micold-client/tests/`, geometry and contrast gates in-crate.

## Phase 1: Setup

None: no new crate, module or dependency. `crates/micold-core/src/attention.rs` and every file
touched below already exist; `crates/micold-client/tests/sidebar_attention.rs` is created by T003.

## Phase 2: Foundational (blocks all stories)

- [x] T001 Write failing unit tests for `counts_as_unread` in the `mod tests` of `crates/micold-core/src/attention.rs`: an unread, not archived session with `in_view = None` counts; the same session with `in_view = Some(its id)` does not; a read session does not; an unread session with `archived = true` does not; an unread session counts while another session is in view (data-model "Counted session", FR-002).
- [x] T002 Implement `pub fn counts_as_unread(session: &Session, in_view: Option<SessionId>) -> bool` = `session.unread && !session.archived && in_view != Some(session.id)` in `crates/micold-core/src/attention.rs`, with a doc comment naming 039 FR-016, FR-019 and 039 US1 scenario 9 (R1, plan D1). Do not change `Workspace::unread_session_count` here (M2, T018).

## Phase 3: User Story 1 - See which worktrees need me without expanding them (P1) 🎯 MVP

**Goal**: every worktree row and the Default row with counted sessions shows `● n`, expanded or
collapsed; rows with none show nothing new.

**Independent Test**: `cargo test -p micold-client --test sidebar_attention`; the `tree_view.rs`
geometry tests and `composition_contrast.rs`; quickstart B1-B4 and B10.

- [x] T003 [P] [US1] Create `crates/micold-client/tests/sidebar_attention.rs` with failing state tests for `SidebarEntry::unread_count(in_view)` over `State::sidebar_entries()` (build the state as `crates/micold-client/tests/features_sidebar.rs` and `crates/micold-client/tests/switcher_unread.rs` do): `feature-x` with two unread sessions gives 2 and `feature-y` with only read sessions gives 0 (US1.1); one unread Default session gives the Default entry 1 (US1.2); the counts are equal with each row expanded and collapsed (US1.3, FR-003); one unread session plus one viewed awaiting-input session gives 1 (US1.4); the session `features::attention::in_view(&state)` returns is not counted (edge case "The session in view"); an archived unread session adds nothing to its worktree (US1.5, edge case "only unread sessions were closed"); a project with no worktrees has only the Default entry and it carries the count; a worktree hidden by a tag filter (008 FR-025) and an agent-owned worktree hidden by the "Show agent worktrees" setting off (014) have no entry, the feature 024 re-admitted row and a missing or invalid worktree (010 FR-011) carry their counts (contract A1, A2).
- [x] T004 [US1] In the same `crates/micold-client/tests/sidebar_attention.rs`, write failing tests for `features::sidebar::unread_tooltip_line(n)`: `None` at 0, `Some("1 unread session")` at 1, `Some("12 unread sessions")` at 12; and for `features::sidebar::with_unread_line(tooltip, n)`: the tooltip unchanged at 0, the line appended after a newline at `n ≥ 1`, after a multi-line `worktree_tooltip(..)` and after `DEFAULT_LOCATION_LABEL` (FR-005, contract A7, data-model "Tooltip line").
- [x] T005 [P] [US1] Write failing geometry tests in the `mod tests` of `crates/micold-client/src/ui/material/tree_view.rs`, beside `an_unread_row_has_the_height_of_a_read_one`, for a location row built with `.unread_count(n)`: the row has the height of a plain row, one-line and with tags; at `NARROW` width with `LONG_NAME` the label is ellipsized and the indicator keeps its full width (U8); the indicator lies before the trailing element and its bounds are equal with the row hovered and at rest and do not intersect the trailing cluster's (FR-004, contract A4, A5, R6); the label keeps its role (not the emphasised role) for `Count` (contract A6, R4); `.unread_count(0)` draws nothing; `.unread(true).unread_count(2)` and `.unread_count(2).unread(false)` follow "the last builder call wins" (data-model "Row unread state", contract A10); on a missing or invalid worktree row (error-tinted name) the count is drawn in the theme's text colour, not the error tint (contract A6, R4).
- [x] T006 [US1] Implement `impl SidebarEntry { pub fn unread_count(&self, in_view: Option<SessionId>) -> usize }` in `crates/micold-client/src/features/sidebar.rs`, counting `DefaultNode::sessions` / `WorktreeNode::sessions` with `micold_core::attention::counts_as_unread`, independent of `expanded` (R2, plan D2). No field is added to `DefaultNode` or `WorktreeNode`.
- [x] T007 [US1] Implement `pub fn unread_tooltip_line(n: usize) -> Option<String>` and `pub fn with_unread_line(tooltip: String, n: usize) -> String` in `crates/micold-client/src/features/sidebar.rs` (R5, plan D4). `worktree_tooltip`'s signature stays as it is.
- [x] T008 [US1] In `crates/micold-client/src/ui/material/tree_view.rs`, replace `pub unread: bool` with a private `enum RowUnread { Read, Unread, Count(NonZeroUsize) }` (default `Read`); keep `.unread(bool)` (`true` → `Unread`, `false` → `Read`); add the chainable builder `.unread_count(n: usize)` (`0` → `Read`, `n > 0` → `Count(n)`). In the render code, `Count(n)` pushes `UnreadMark::new(r).count(n.get()).role(label_role)` in the slot where `Unread` pushes the bare mark (after label and annotation, before the trailing element), never `.worded(true)`, in the theme's text colour also on an error-tinted row, and only `Unread` selects the emphasised label role (R3, R4, plan D3). Then `grep -rn '\.unread\b\|TreeItem {' crates/` and fix any direct read of the field or struct literal; `.unread(..)` calls in `crates/micold-client/src/ui/sidebar.rs`, the showcase and `crates/micold-client/src/ui/material/anatomy_size.rs` stay unchanged.
- [x] T009 [US1] Wire the indicator in `crates/micold-client/src/ui/sidebar.rs` (glue, plan D5): in `build_items`, before matching the entry, `let unread = entry.unread_count(crate::features::attention::in_view(state));` on the worktree row add `.unread_count(unread)` and wrap the tooltip as `with_unread_line(worktree_tooltip(..), unread)`; `build_default_item` takes the count as a parameter and does the same with `DEFAULT_LOCATION_LABEL`. Tags, the row-actions cluster and expansion are unchanged; session rows keep `.unread(row_unread(..))`.
- [x] T010 [P] [US1] Extend `the_unread_mark_is_legible_on_every_fill_a_host_draws_it_on` in `crates/micold-client/src/ui/material/composition_contrast.rs` with the hovered sidebar row fill (`style::state_layer(sidebar fill, on_surface, state::HOVER)`) for the mark at 3:1, and add a check that the count's colour (`on_surface`) meets 4.5:1 on the sidebar fill, the hovered fill and the selected pill (`secondary_container`), in the light and the dark scheme (FR-013, contract A9, R9). If a pair fails, fix the colour the mark or count takes in `tree_view.rs`, not the threshold.
- [x] T011 [US1] Add three posed location rows to `crates/micold-client/src/showcase/sections/atoms.rs::unread_mark`: a worktree row with `.unread_count(2)` collapsed, the same row expanded with its two `.unread(true)` session rows, and a worktree row with no indicator; update the `UnreadMark` entry's text in `crates/micold-client/src/showcase/catalogue.rs` to mention them (FR-012, contract §Showcase).
- [x] T012 [US1] Update `docs/user-guide/worktrees-and-sessions.md` § Unread sessions: the indicator `● n` on worktree and Default rows, what it counts (unread sessions of that row, not closed, not the one in view), that it shows on collapsed and expanded rows, that viewing a session lowers it, that the tooltip says "n unread sessions", and that a worktree the sidebar hides counts only on the project switcher (FR-014).
- [ ] T013 [US1] Run the `visual-pass` skill (through an `autopilot-worker`) for quickstart B1-B5, B7, B8 and B10 in the light and the dark scheme, and save the evidence in `specs/575-workspace-attention-list/visual/`.

## Phase 4: User Story 2 - The indicator follows the sessions on its own (P1)

**Goal**: the indicators change with the sessions' unread state and the session in view, in every
window and after a restart, with no new mechanism (R7).

**Independent Test**: `cargo test -p micold-client --test sidebar_attention`; quickstart B5, B7, B8.

- [x] T014 [US2] Add state tests to `crates/micold-client/tests/sidebar_attention.rs`, through the same update path `crates/micold-client/tests/features_attention.rs` and `crates/micold-client/tests/attention_view_report.rs` use: a catalog update that sets `unread` on a session of `feature-x` raises its count from 0 to 1 and one that clears it lowers it again (US2.1, US2.3, FR-007); selecting the session so it comes into view drops the count at once, before any catalog update (US2.2, 039 FR-019); a catalog update that removes or archives a session lowers the count (FR-007); several updates in one batch settle on the right count (edge case "A burst of changes"); expanding, collapsing and hovering a location row leave every session's `unread` and the window's view report unchanged (FR-009). See each new test fail by stubbing `SidebarEntry::unread_count` to return 0 before keeping it. Multi-window agreement (US2.4) and restart (US2.5) rest on 039's one service and persisted `unread` (FR-008, R7) and are checked by quickstart B7 and B8.

## Phase 5: User Story 1, scenario 5 - Closed sessions leave the switcher's counts (P1)

**Goal**: the switcher's per-project counts and button total skip closed sessions, so a project's
location rows add up to its switcher count (FR-010, 039 US1 scenario 9).

**Independent Test**: `mise run test-core`; `cargo test -p micold-client --test switcher_unread`;
`cargo test -p micold-client --test sidebar_attention`; quickstart B6, B9.

- [ ] T015 [P] [US1] Write failing unit tests in the `mod tests` of `crates/micold-core/src/workspace.rs`: `unread_session_count` leaves out an archived session whose `unread` is set, and `other_projects_unread` does too (contract A8).
- [ ] T016 [P] [US1] Write failing state tests in `crates/micold-client/tests/switcher_unread.rs`: a closed unread session leaves its project's `SwitcherEntry::unread_count` and the button total; the other projects' counts are unchanged (FR-010, US1.5).
- [ ] T017 [US1] Write a failing test in `crates/micold-client/tests/sidebar_attention.rs`: for a project with no hidden worktree and one closed unread session, the sum of `unread_count(in_view)` over `sidebar_entries()` equals the active project's `SwitcherEntry::unread_count` from `State::switcher_entries()`; and with a tag filter, or the agent-worktree setting, hiding a worktree that holds an unread session, the switcher still counts it while no entry does (FR-010, R8, spec Clarifications).
- [ ] T018 [US1] Make `Workspace::unread_session_count` in `crates/micold-core/src/workspace.rs` filter with `crate::attention::counts_as_unread(s, in_view)` instead of `s.unread && Some(s.id) != in_view`; `other_projects_unread` follows through it (R1, plan D1). Signatures unchanged.
- [ ] T019 [US1] Update `docs/user-guide/project-selection.md` where it describes the switcher's unread count: a closed session no longer counts (FR-010).

## Phase 6: Polish

- [ ] T020 Run quickstart §A, §B (B1-B11, through the `visual-pass` skill) and §C, and record the results in `specs/575-workspace-attention-list/quickstart.md`.
- [ ] T021 Check that `docs/user-guide/worktrees-and-sessions.md` and `docs/user-guide/project-selection.md` match the shipped behaviour of both stories and the closed-session rule.

## Dependencies & Execution Order

- Phase 2 (T001-T002) blocks everything: every count goes through `counts_as_unread`.
- US1 (T003-T013): tests T003-T005 first; then T006-T007 (state) and T008 (host); T009 needs T006-T008; T011 needs T008; T012 and T013 after T009.
- US2 (T014) needs T003 and T006; it adds no code (R7). Its tests guard behaviour the M1 code already gives, so their red evidence is the stub named in T014, not a missing feature.
- Phase 5 (T015-T019) needs T002 and T006; T018 after T015-T017.
- Polish (T020-T021) after everything.
- Parallel: T003 beside T005 (T004 after T003, same file); T010 beside T006-T009; T015 beside T016 (T017 shares `sidebar_attention.rs`).

## Implementation Strategy

MVP is M1 (Foundational, US1 scenarios 1-4, US2): the indicator on the location rows, live. M2
then makes the switcher skip closed sessions, so the rows add up to the switcher count. Polish
(T020-T021) changes no code and is left to the close unit: it runs quickstart B6, B9, B11 and §C,
which carry SC-002, SC-006 and FR-015.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Attention indicator on the sidebar's location rows 🎯 MVP

- **Tasks**: T001–T014
- **Deliverable**: each sidebar worktree row and the Default row holding unread sessions shows `● n` with the number of its unread, not closed, not-in-view sessions, collapsed or expanded, with a tooltip line "n unread sessions", and the count follows the sessions live; the showcase shows the posed location rows in both themes.
- **Satisfies**: US1 acceptance scenarios 1–4, and the sidebar half of scenario 5 (a closed session adds nothing to its row); US2 acceptance scenarios 1–5; FR-001–FR-009, FR-011–FR-014; SC-001 (checked once by B1), SC-003, SC-004, SC-005 (restart half, B8). Until M2 merges, the switcher still counts a closed unread session while the rows do not.
- **Verify**: `mise run test-core`; `cargo test -p micold-client` (the new `sidebar_attention` tests and 039's regression tests, FR-011); `cargo test -p micold-client --lib tree_view`; `cargo test -p micold-client --lib composition_contrast`; quickstart B1–B5, B7, B8, B10 (visual-pass, T013)
- **Depends on**: —
- **Tier**: full

### M2 — The switcher's counts skip closed sessions

- **Tasks**: T015–T019
- **Deliverable**: a closed session that was unread no longer counts on the project switcher's row or button, and a project's location-row indicators add up to its switcher count.
- **Satisfies**: US1 acceptance scenario 5 (switcher half); FR-010; SC-005 (sum half)
- **Verify**: `mise run test-core`; `cargo test -p micold-client --test switcher_unread --test sidebar_attention`. Quickstart B6 and B9 run in the close unit's T020.
- **Depends on**: M1
- **Tier**: light

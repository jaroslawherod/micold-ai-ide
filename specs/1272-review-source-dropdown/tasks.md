---
description: "Task list for #1272 review source dropdown and two-column layout"
---

# Tasks: Review UI — source dropdown and two-column layout

**Input**: `specs/1272-review-source-dropdown/` (spec.md, plan.md, research.md, data-model.md, contracts/source-control.md, quickstart.md)

**Tests**: mandatory (Constitution I). Within each phase the test tasks come first and must fail before the implementation tasks they cover. `MenuSelect` drawing and `ui/changes.rs` placement use the GUI-glue exception (geometry gates + quickstart Part B).

**Phases follow the plan's layers, not one phase per story** (ledger decisions 2 and 3): the view wiring rewrites the same header and body rows as #1270, so the model, reducer and component ship first and every story's view work lands in Phase 5, after #1270 has merged. Story labels name the story a task serves. See the note under *Milestones*.

**Documentation**: the user guide changes with the view, in Phase 5 (Constitution VII). M1 to M3 change nothing a user sees.

**Cross-platform**: no platform branch; the list floats through the one `cdk::picker::Picker` overlay on Linux, macOS and Windows (Constitution VI).

## Format: `[ID] [P?] [Story] Description`

Crate paths: `crates/micold-core/src/review/` (core), `crates/micold-client/src/` (client), `crates/micold-client/tests/` (client tests).

## Phase 1: Setup

- [x] T001 Refactor under the green suite (no new behaviour, no red step): create `crates/micold-core/src/review/source_menu.rs` (module doc: pure, no I/O, no clock, no iced; contract §1) and register `pub mod source_menu;` in `crates/micold-core/src/review/mod.rs`. Move the wording constants there (data-model §7): `WORKING_TREE`, `POSSIBLY_STALE`, `PRS_LOADING`, `PRS_UNAVAILABLE`, `PRS_EMPTY` verbatim from `crates/micold-client/src/ui/changes.rs:48-57` and `DEFAULT_ENTRY_NOTE` verbatim from `crates/micold-client/src/features/changes.rs:39`; add `PULL_REQUESTS` ("Pull requests"), `FILES` ("Files"), `KINDS_COMMITTED` ("committed"), `KINDS_UNCOMMITTED` ("uncommitted"), `KINDS_NONE` ("nothing shown") per the data-model §6 labels. The client's same-named constants become `pub use micold_core::review::source_menu::…` re-exports, so every existing caller and test keeps its path

## Phase 2: US1 + US2 (P1) — the control's model in core (M1)

**The P1 stories' core behaviour** (milestones.md rule 6): every decision of FR-010 for the source list (US1) and the Working-tree chips (US2), unreachable from the UI until M4. Tests first, in the module's `#[cfg(test)]`.

- [x] T002 [US1] Write failing unit tests for `rows(&MenuInput)` in `crates/micold-core/src/review/source_menu.rs` (data-model §4): one test per FR-005 state (SC-005) — `Absent` (setting off; Default entry) gives no `Heading` and no pull-request rows; `Loading` / `Unavailable` / `Empty` give `Heading` then `Note(PRS_LOADING | PRS_UNAVAILABLE | PRS_EMPTY)`; `List { stale: true }` gives `Heading`, `Stale`, then the rows; `List` rows in input order (newest first, as read). Invariants: exactly one `WorkingTree` row first and one `Files` row last; at most one `chosen` among Working tree and pull requests, exactly one when `shown` is the working tree or a listed number, none when it is an unlisted number; `Files.on == files_on` in every case; no `Heading` without a following `Note` or pull request. Chips: Committed is `available: committed_available`, `on: toggles.committed && available`; `note == Some(DEFAULT_ENTRY_NOTE)` exactly when Committed is unavailable
- [x] T003 [US1] Write failing unit tests for `stops(&[Row])` and `Stop::keeps_open` in `crates/micold-core/src/review/source_menu.rs` (data-model §5, R7): order Working tree, `Kind(Committed)`, `Kind(Uncommitted)`, each `PullRequest(n)` in row order, `Files`; `Kind(Committed)` disabled when unavailable (it stays in the list, so the highlight can land on it; taking it does nothing), every other stop enabled; heading, note and stale rows are no stops; the stops are the same for `stale: true` and `stale: false`; `keeps_open` true only for `Kind(_)`
- [x] T004 [US2] Write failing unit tests for `label`, `Label::full` and `tooltip` in `crates/micold-core/src/review/source_menu.rs`: every row of the data-model §6 table (working tree with both, only committed, only uncommitted, no listable kind; `Files · Working tree` with no kinds suffix; `#n · ` + title; `Files · #n · ` + title; `#n` / `Files · #n` with no title for an unlisted number); `full()` joins `fixed` and `title`; `tooltip(Some(base), &label)` is `full()` then the base line on a second line, `tooltip(None, &label)` is `full()` alone (FR-009)
- [x] T005 [US1] Implement `Shown<'a>`, `PrRows<'a>` (`Absent`, `Loading`, `Unavailable`, `Empty`, `List { prs: &'a [BranchPullRequest], stale: bool }`), `MenuInput<'a>` (`shown`, `files_on`, `toggles: review::base::Toggles`, `committed_available`, `prs`), `Chip { on, available }`, `Row<'a>` and `rows()` in `crates/micold-core/src/review/source_menu.rs` (T002 green). Every function total: no panic, no error (contract §1)
- [x] T006 [US1] Implement `Stop::{WorkingTree, Kind(Kind), PullRequest(u64), Files}`, `Kind::{Committed, Uncommitted}`, `stops()` and `Stop::keeps_open` in `crates/micold-core/src/review/source_menu.rs` (T003 green)
- [x] T007 [US2] Implement `Label<'a> { fixed: String, title: Option<&'a str> }`, `label()`, `Label::full()` and `tooltip()` in `crates/micold-core/src/review/source_menu.rs` (T004 green)
- [x] T008 [US1] Refactor under the green suite (no red step): replace the client's `PrSection` enum (`crates/micold-client/src/features/changes.rs:120`) with `pub use micold_core::review::source_menu::PrRows as PrSection;` so `section(view, enabled, now)`, its callers in `crates/micold-client/src/ui/changes.rs` and the existing tests in `crates/micold-client/tests/features_pr_changes.rs` keep their names and pass unmodified (SC-004)

**Checkpoint**: `mise run test-core` green; `mise run gate` green with no client test changed.

## Phase 3: Reducer — the control's state and picks (M2)

**Goal**: the render-free reducer owns openness and highlight, and every pick goes through the existing `SourceChosen` / `FilesPaneToggled` arms (FR-006, FR-007, contract §2). The view does not send the new messages yet.

- [x] T009 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: `SourceMenuToggled` opens with `highlight` on the chosen source's stop and closes again; `SourceMenuDismissed` closes, clears the highlight and leaves the source (FR-011); `SourceMenuMoved(Direction)` moves over `source_menu_stops` with `typeahead::move_highlight`, landing on a disabled `Kind(Committed)` stop like any other; `SourceMenuPicked` on the Working tree or a pull-request stop closes the list and yields the same state and `Effect` as `SourceChosen(None)` / `SourceChosen(Some(n))` (diff read, PR header, per-entry memory; FR-006); `SourceMenuPicked` with no highlight or on a disabled stop does nothing; `SourceChosen(_)` sent directly also closes an open list; `PullRequestsRead` keeps an open list open and re-seats the highlight with `typeahead::clamp_highlight`, and a shown pull request that disappeared falls back to the working tree with `label` changing to `Working tree`; previous/next (`PullRequestStepped`) changes `label`; two picks in a row: the last wins; `Msg::Opened` and a change of the view's entry reset `source_menu` to closed. Existing 1003 tests stay unmodified
- [x] T010 [P] [US2] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `SourceMenuPicked` on a `Kind(_)` stop flips that kind like `CommittedToggled` / `UncommittedToggled` and keeps the list open; `Kind(Committed)` on the Default entry is disabled and changes nothing; with a pull request shown, `CommittedToggled` / `UncommittedToggled` first return to the working tree (source `Source::Changes`, the entry's remembered pull request forgotten, diff pane reset), then flip the kind and request a read, and the list stays open (FR-004, R4)
- [x] T011 [US3] Write failing reducer tests in `crates/micold-client/tests/features_changes.rs`: `SourceMenuPicked` on the `Files` stop closes the list and does what `FilesPaneToggled` does (on, and off again); Files stays on across a Working tree or pull-request pick through the control, with the tree and browsed file read again at the new source and Back cleared (`reread_after_source_change`); a chip pressed with a pull request shown and Files on re-reads the tree at the working tree (Story 3 s4); `label` reads `Files · Working tree` / `Files · #n · ` (FR-001, FR-007). Existing 1150 tests stay unmodified
- [x] T012 [US1] Add `SourceMenu { open: bool, highlight: Option<usize> }` (default closed) as `OpenView::source_menu` in `crates/micold-client/src/features/changes.rs`, reset by `Msg::Opened` and whenever the view's entry changes (data-model §8)
- [x] T013 [US1] Add `source_menu_input(view: &OpenView, enabled: bool, now: u64) -> MenuInput<'_>` (for the view) and `source_menu_stops(view: &OpenView, enabled: bool) -> Vec<(Stop, bool)>` (for the reducer, `PrRows::List` forced to `stale: false`, no clock) over one private builder that takes the stale flag, in `crates/micold-client/src/features/changes.rs` (data-model §3)
- [x] T014 [US1] Add `Msg::SourceMenuToggled`, `SourceMenuDismissed`, `SourceMenuMoved(Direction)` and `SourceMenuPicked` (dispatching `SourceChosen(None)`, `CommittedToggled`, `UncommittedToggled`, `SourceChosen(Some(n))` or `FilesPaneToggled` per the highlighted stop, closing unless `keeps_open`); make `SourceChosen(_)` and `FilesPaneToggled` close the list; re-seat the highlight in `PullRequestsRead`, in `crates/micold-client/src/features/changes.rs` (data-model §9; T009, T011 green). No new `Effect` (contract §2)
- [x] T015 [US2] Extract `show_working_tree(state)` from the `Msg::SourceChosen(None)` arm (`crates/micold-client/src/features/changes.rs:1840-1852`); call it from `SourceChosen(None)` (which then closes the list) and, when a pull request is shown, from `CommittedToggled` / `UncommittedToggled` before the flip (which leave the list open) (R4; T010 green)

**Checkpoint**: `mise run gate` green; `features_changes.rs` and `features_pr_changes.rs` pre-existing tests unmodified.

## Phase 4: Shared component `MenuSelect` (M3)

**Goal**: the library gains a select whose options are caller-built rows that may hold chips (Principle VIII, R6, contract §3). Seen in the showcase only.

- [ ] T016 [P] [US1] Write a failing geometry gate `crates/micold-client/tests/gates/menu_select_fits.rs`, registered like its siblings: the open list floats below the trigger and the body below keeps its position; a long title ellipsizes while the fixed `#n · ` part stays whole; above `max_list_height` the list caps and scrolls inside itself; an `option_with_chips` row holds its chips and note inside the row; heading and note rows are not pickable; trigger press, `Enter` and `Space` emit `on_toggle`, an outside press, `Esc` and `Tab` emit `on_dismiss`, a chip press emits only the chip's message (FR-011). `tests/one_overlay_implementation.rs` stays green with no new exception
- [ ] T017 [US1] Implement `MenuSelect` and `MenuSelectRow` in `crates/micold-client/src/ui/material/menu_select.rs` per contract §3 (`new(fixed_label, rows, open, on_toggle, on_dismiss, roles)`, `.title`, `.tooltip`, `.keyboard`, `.on_move`, `.on_pick`, `.max_list_height`; `option(content, chosen, on_pick)`, `option_with_chips(content, chosen, on_pick, chips, note)`, `heading(text)`, `note(text)`), composed on `cdk::picker::Picker` (floating), `cdk::tooltip::Tooltip`, `Ellipsized`, `ToggleChip` and the `material::picker` list surface; trigger press / `Enter` / `Space` emits `on_toggle`, outside press / `Esc` / `Tab` emits `on_dismiss`, a chip press emits only the chip's message; `pub use` in `crates/micold-client/src/ui/material/mod.rs` (T016 green)
- [ ] T018 [P] [US1] Add `MenuSelect` poses, built from `source_menu::rows` and `label`, to `crates/micold-client/src/showcase/sections/review.rs`: closed, closed with a long title, open in every `PrRows` state, open on the Default entry (Committed unavailable with its note), in both themes

**Checkpoint**: `mise run gate` green; the showcase shows the component.

## Phase 5: View wiring — header, left column, two columns (M4)

**Does not start until #1270 has merged** (and #1269, if still open then, has merged or is confirmed not to touch the review header in `ui/changes.rs`): ledger decision 2, research R8.

- [ ] T019 [P] [US1] Write failing view tests in `crates/micold-client/tests/gates/session_area_fits.rs`: the left column holds no Pull requests section, no Committed / Uncommitted / Files toggle and no Default-entry note in every source state (working tree, pull request shown, Files on, Default entry, setting off) and starts with the file list (FR-003, SC-003); the header holds the source control and the control carries a tooltip with the base line (FR-009)
- [ ] T020 [US4] Write failing geometry tests in `crates/micold-client/tests/gates/session_area_fits.rs`: at 1280×800 the file column is 260 px and the diff at least 70% of the body's width (FR-008, SC-002); at 1600 and 900 px wide the file column stays 260 px; at 900 px the source control and Close stay visible and the subtitle truncates first (Edge "Narrow window")
- [ ] T021 [US1] Put `MenuSelect` in the review header in `crates/micold-client/src/ui/changes.rs`: `[title + subtitle (Fill, subtitle ellipsized)] [MenuSelect] … [Close]`, rows from `source_menu::rows(&source_menu_input(view, pr_enabled, now))` (pull requests as `PullRequestRow` content, the Working tree row's chips as `ToggleChip`s sending `CommittedToggled` / `UncommittedToggled`), label from `label`, tooltip from `tooltip(base_line, ..)`, keyboard from `source_menu_stops` and `view.source_menu`, messages `SourceMenu*`; the list never taller than the space below the header (R8)
- [ ] T022 [US1] Remove `pull_request_section`, the toggle row, the Default-entry note and `PRS_MAX_HEIGHT` from the left column in `crates/micold-client/src/ui/changes.rs` (T019 green)
- [ ] T023 [US4] Lay the body out as `row![file column Fixed(FILE_COLUMN_WIDTH = 260.0), diff Fill]` in `crates/micold-client/src/ui/changes.rs` (T020 green)
- [ ] T024 [US1] Re-record `session-area-changes-open` and add covered states for the open control (working tree, every `PrRows` state, pull request shown, Files on) in `crates/micold-client/tests/support/covered_states.rs` and `crates/micold-client/tests/fixtures/layout_snapshot.txt` (FR-012)
- [ ] T025 [US1] Update `docs/user-guide/reviewing-changes.md`: *Opening the Changes view* (the source control, its label and tooltip), *Pull requests* (the list in the control, every state's wording), *Committed and uncommitted* (chips in the Working tree option, chip while a PR is shown), *The Default entry* (the Committed note now in the option, no "Two toggles"), *Browsing the repository* (the Files option, `Files · ` label) (FR-012, Constitution VII)
- [ ] T026 [US1] Run the `visual-pass` skill through an `autopilot-worker` over quickstart Part B (1280×800, light and dark) and save the evidence in `specs/1272-review-source-dropdown/visual-pass/`, including step 4's two actions from working tree to a pull request (FR-012, SC-001)

**Checkpoint**: `mise run gate` green; quickstart Part B passes.

## Phase 6: Polish (no code; done by the close unit, not a milestone)

- [ ] T027 Record quickstart Part A and Part B results in `specs/1272-review-source-dropdown/quickstart.md` and tick the tasks above

## Dependencies and order

Phase 1 → Phase 2 → Phase 3 and Phase 4 (both need only Phase 2; Phase 4's showcase poses are built from `rows` and `label`) → Phase 5 (needs Phases 3 and 4, and #1270 merged) → Phase 6. Within a phase, test tasks marked [P] run in parallel and precede the implementation tasks that turn them green.

## Implementation Strategy

Model first: every decision is a core unit test before any reducer or widget exists (FR-010). The reducer then routes picks through the existing 1003 and 1150 arms so their suites prove FR-006 and FR-007 unchanged. The component ships in the showcase before any view uses it. The user-visible change ships once, with its docs, after #1270.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

**Cut by layer, deviating from milestones.md rules 1 and 2** (ledger decision 3). No story's UI can land before #1270 merges (decision 2, R8), so a per-story cut would put every story's milestone behind #1270. Rule 6 is applied instead: M1 ships the P1 stories' (US1, US2) core decisions with their tests and M2 their reducer behaviour, both unreachable from the UI until **M4**, which completes every story. The reducer is its own milestone because core + reducer is 15 tasks (rule 3: about 10).

### M1 — Source control model in core (US1, US2 core) 🎯 MVP

- **Tasks**: T001–T008
- **Deliverable**: `micold_core::review::source_menu` decides the control's rows, order, availability, chosen marks, keyboard stops, closed label and tooltip, covered by unit tests; the client's PR section state and wording come from it with no behaviour change
- **Satisfies**: US1 and US2 decisions (rows, order, chips, label; UI in M4); FR-005 (wording, every state), FR-009 (tooltip text), FR-010; SC-005; SC-004 (existing suites unmodified)
- **Verify**: `mise run test-core` (`review::source_menu` tests), then `mise run gate`
- **Depends on**: —
- **Tier**: full

### M2 — Reducer: control state and picks

- **Tasks**: T009–T015
- **Deliverable**: the reducer opens, moves, picks and dismisses the source control, routing every pick through `SourceChosen` / `FilesPaneToggled`, keeping the list open on chips and returning to the working tree on a chip while a pull request is shown; proven by reducer tests (not yet reachable from the view)
- **Satisfies**: FR-002, FR-004, FR-006, FR-007, FR-011 (reducer side); US1 s3–s6, US2 s2–s3, US3 s1, s4–s5 at the reducer; SC-004
- **Verify**: `mise run gate`; `cargo test -p micold-client --test features_changes --test features_pr_changes` output shows the new `source_menu` tests
- **Depends on**: M1
- **Tier**: full

### M3 — Shared `MenuSelect` component

- **Tasks**: T016–T018
- **Deliverable**: the showcase shows `MenuSelect` closed, with a long title, open in every pull-request list state and on the Default entry; its geometry gate passes
- **Satisfies**: Principle VIII; FR-001 truncation (component side); Edge "Many pull requests", "Long PR title"
- **Verify**: `mise run gate` (`menu_select_fits` gate); `mise run showcase`, review section
- **Depends on**: M1
- **Tier**: full

### M4 — Source control in the review header, two-column body

- **Tasks**: T019–T026
- **Deliverable**: the review header has the source control, the left column holds only the file list or the Files tree, and the body is a 260 px file column and the diff; documented in the user guide and checked in the visual pass
- **Satisfies**: US1–US4 acceptance scenarios; FR-001, FR-003, FR-008, FR-009, FR-012; SC-001, SC-002, SC-003
- **Verify**: `mise run gate` (`session_area_fits`, layout snapshot); quickstart Part B
- **Depends on**: M2, M3, **#1270 merged** (and #1269 merged or confirmed not to touch the review header) — do not start before
- **Tier**: full

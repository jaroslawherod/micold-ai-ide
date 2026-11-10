# Tasks: Structured view header in one row; status and actions move to the top bar

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/ui-contract.md](contracts/ui-contract.md)

**Tests**: Principle I applies: every test task precedes the implementation it covers and is written failing first. Pure draw glue in `ui/` is covered by geometry gates and quickstart Part B.

**Documentation**: Principle VII: each story's user-guide task sits in the milestone that ships its behaviour.

Paths are relative to the repo root. `client` = `crates/micold-client`, `material` = `crates/micold-material`.

## Phase 1: Setup and Foundational

- [ ] T001 [P] Test first: ContextRing builder tests (ratio to arc, empty state when ratio is None, warning role colour, text and tooltip set) in `crates/micold-material/src/context_ring.rs` (`#[cfg(test)]`)
- [ ] T002 [P] Test first: SegmentedControl builder tests (two segments, selected state, `on_select` message per segment, accessible name per segment) in `crates/micold-material/src/segmented.rs` (`#[cfg(test)]`)
- [ ] T003 [P] Test first: `crates/micold-client/tests/features_header_bar.rs` for `HeaderBarModel`: `None` on Overview, item order (FR-003, FR-004), `MeterText` text, tooltip and accessible label (tokens used/total, cost, turns; cost still present when text hides), empty state with no invented cost (FR-005, FR-011), `Status` Idle/Working mapped from per-session state (a failed or exited session maps to Idle, not Working), no cross-session leak with two sessions (Principle II), collapse rank constants (ring text 0, Changes label 1)
- [ ] T004 Implement `ContextRing` in `crates/micold-material/src/context_ring.rs` (chainable `.ratio().text().warning().tooltip()`, `Roles` tokens, `.into()`); export in `crates/micold-material/src/lib.rs`; add a showcase entry
- [ ] T005 Implement `SegmentedControl` in `crates/micold-material/src/segmented.rs`; export in `crates/micold-material/src/lib.rs`; add a showcase entry
- [ ] T006 Implement `HeaderBarModel`, `MeterText`, `Status` and rank constants in new `crates/micold-client/src/features/header_bar.rs` (registered in `crates/micold-client/src/features/mod.rs`); add the `Status` mapping beside `session_status` in `crates/micold-client/src/features/structured_insights.rs` (depends on T003)
- [ ] T007 [P] Test first: `HeaderMenu` reducer tests (`None | Plus | Worktree`, at most one open, closes on selection change and on dismiss) in `crates/micold-client/tests/features_header_bar.rs`
- [ ] T008 Implement the `HeaderMenu` state and reducer in `crates/micold-client/src/features/header_bar.rs`, wired into the client state (depends on T007)

## Phase 2: User Story 2 - Status, Changes and context meter in the top bar (P1) 🎯 MVP

- [ ] T009 [US2] Gate test first: `crates/micold-client/tests/gates/top_bar_items.rs`, registered with a `#[path]` entry in `crates/micold-client/tests/layout_snapshot.rs`, pins the top-bar order ring, status, Changes, then settings and app menu, none on Overview, no overlap at a wide width (pattern: `status_bar_fits.rs`)
- [ ] T010 [US2] Add the ring (with tooltip and accessible label), status badge and Changes button to `toolbar::view` actions in `crates/micold-client/src/ui/toolbar.rs`, ring text and Changes label inside `GiveWayRow` ranks per R2, hidden when `HeaderBarModel` is `None` (depends on T004, T006, T009, T028)
- [ ] T011 [US2] Route Changes activation to the existing changes view and verify the ring is focusable and named, in `crates/micold-client/src/ui/toolbar.rs` and `crates/micold-client/src/ui/mod.rs`
- [ ] T012 [US2] User guide: top-bar status, Changes and context meter in `docs/user-guide/structured-transcript.md`

## Phase 3: User Story 4 - Worktree `⋮` menu (P2)

- [ ] T013 [US4] Test first: worktree menu content tests with accessible names and Escape close (raw-events toggle reflects `settings.show_raw_events`, view segments Structured/Terminal for Claude Code only, none for other CLIs, switch sends the existing `Composer::Msg::SwitchRequested`, app `toolbar::overflow_items` unchanged) in `crates/micold-client/tests/features_header_bar.rs`
- [ ] T014 [US4] Add `WorktreeMenuModel` and build the worktree menu items in `crates/micold-client/src/features/header_bar.rs` (depends on T013)
- [ ] T015 [US4] Add the worktree `⋮` trigger to the top bar in `crates/micold-client/src/ui/toolbar.rs` and its `MenuOverlay` (raw-events toggle row, `SegmentedControl`) in `crates/micold-client/src/ui/mod.rs` beside `overflow_menu`; Escape and outside click close it
- [ ] T016 [US4] User guide: worktree `⋮` menu in `docs/user-guide/structured-transcript.md` and `docs/user-guide/worktrees-and-sessions.md`

## Phase 4: User Story 3 - Single `+` for a session or a terminal (P1)

- [ ] T017 [US3] Test first: `PlusMenuModel` tests in `crates/micold-client/tests/features_session_strip.rs` (accessible name, Escape closes, items "Another session" and "Terminal", session entry sends `AdditionalRequested(location)`, terminal entry sends `ShellInstanceOpenRequested` every time, one `+` only)
- [ ] T018 [US3] Implement `PlusMenuModel` and drop the `Member::Plus` split in `crates/micold-client/src/features/session_strip.rs` (depends on T017)
- [ ] T019 [US3] Replace the two add icons and the `Open in terminal` switch slot with one `+` in `crates/micold-client/src/ui/session_area.rs`; add its anchored `MenuOverlay` in `crates/micold-client/src/ui/mod.rs` (accessible name "Add session or terminal", Escape closes; depends on T008, T015, T018)
- [ ] T020 [US3] User guide: the `+` menu replaces `Open in terminal` in `docs/user-guide/worktrees-and-sessions.md` and `docs/user-guide/structured-transcript.md`

## Phase 5: User Story 1 - One-row header (P1)

- [ ] T021 [P] [US1] Test first: gate `crates/micold-client/tests/gates/header_row_fits.rs`, registered with a `#[path]` entry in `crates/micold-client/tests/layout_snapshot.rs` (one 40 px row above the first event, no title row, chip at 0 and 1000+, `+` and chip fit) and update `crates/micold-client/tests/session_area_stability.rs` for the removed `status_cluster`
- [ ] T022 [P] [US1] Test first: `crates/micold-client/tests/features_session_strip.rs` activity dot per session (working only, failed or exited sessions show no dot, no `running` text), chip accessible name "N hidden events, show", `HiddenChip` text `N hidden` absent at zero, activation emits `TranscriptMsg::ShowRawEvents(true)`
- [ ] T023 [US1] Implement the per-tab activity dot, `HiddenChip` and chip data in `crates/micold-client/src/features/session_strip.rs` (depends on T022)
- [ ] T024 [US1] Remove the title row `session_area::header`, the `running` text and `status_cluster`; draw the dot and the `N hidden` chip at the row end in `crates/micold-client/src/ui/session_area.rs` (depends on T021, T023)
- [ ] T034 [US1] Test first: the ring tooltip includes totals and running count (R6) in `crates/micold-client/tests/features_header_bar.rs`; the slim body row is present only when background tasks or a mode label exist (decision on R6 pending, see ledger)
- [ ] T025 [US1] Remove the session bar meter and raw-events toggle row from `view_full` in `crates/micold-client/src/ui/transcript/mod.rs` and `crates/micold-client/src/ui/transcript/session_bar.rs`; fold totals and running count into the ring tooltip (R6) in `crates/micold-client/src/features/header_bar.rs` (depends on T034)
- [ ] T026 [US1] Move the background-tasks accordion and mode label to a slim first row of the transcript body, shown only when there is something to show, and replace `hidden_notice` by the chip in `crates/micold-client/src/ui/transcript/entries.rs` and `crates/micold-client/src/ui/transcript/mod.rs` (R6); depends on T024
- [ ] T035 [US1] Update or retire the gates `crates/micold-client/tests/gates/session_bar_fits.rs` and `crates/micold-client/tests/gates/session_area_fits.rs` for the removed session bar and header, and their `#[path]` entries in `crates/micold-client/tests/layout_snapshot.rs`
- [ ] T027 [US1] User guide: one-row header, `N hidden` chip and activity dot in `docs/user-guide/structured-transcript.md`

## Phase 6: User Story 5 - Narrow windows (P2)

Delivered inside M1 and M4 (tests travel with the code): no separate milestone.

- [ ] T028 [US5] Gate test first (runs in M1, before T010): `crates/micold-client/tests/gates/top_bar_collapse.rs`, registered in `crates/micold-client/tests/layout_snapshot.rs`; ring text hides, then Changes label with icon kept, no overlap at wide (1280 px) and narrow (800 px) widths (pattern: `strip_controls_hold_their_size.rs`)
- [ ] T029 [US5] Gate test first (runs in M4, before T024): tabs shrink then scroll, `+` and chip stay reachable with many tabs, in `crates/micold-client/tests/gates/header_row_fits.rs`
- [ ] T030 Removed (folded into T010 and T024, which must make T028 and T029 green)
- [ ] T031 [US5] User guide: narrow-window behaviour in `docs/user-guide/structured-transcript.md`

## Phase 7: Polish

- [ ] T032 Quickstart Part B visual pass at wide and narrow widths, evidence saved in `specs/1793-structured-view-header-one-row/` (`visual-pass` skill; FR-014)
- [ ] T033 Tick tasks and record results in `specs/1793-structured-view-header-one-row/quickstart.md`

## Dependencies and Order

- Phase 1 blocks all stories. T001 to T003 and T007 run first (red); T004 to T006 and T008 turn them green.
- Order of delivery follows the milestones: US2 with the collapse gate, US4, US3, US1 with the tab-scroll gate. Old controls stay in the content area until US1 removes them, so no function is ever unreachable on `main`.
- `[P]` tasks touch disjoint files.

## Implementation Strategy

MVP is M1 (top bar). Each later milestone adds one surface and removes the matching old control only after its replacement shipped. Session-bar leftovers (R6): totals and running count in the ring tooltip, tasks accordion and mode label in a conditional slim row inside the transcript body (not a header row).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Top bar status, Changes and context meter 🎯 MVP

- **Tasks**: T001–T012, T028
- **Deliverable**: With a worktree or session selected, the top bar shows the context meter ring (`8% · $1.28`, tooltip), the Idle/Working status and the Changes button before settings and the app menu; none on Overview. Old header controls are untouched.
- **Satisfies**: US2 acceptance scenarios 1–4; US5 acceptance scenarios 1–2; FR-003, FR-004, FR-005, FR-011 (top-bar part), FR-013 (ring)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client features_header_bar` and `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot top_bar`
- **Depends on**: —
- **Tier**: full

### M2 — Worktree `⋮` menu

- **Tasks**: T013–T016
- **Deliverable**: A worktree `⋮` in the top bar holds the `Show raw events` toggle and the Structured/Terminal segmented control; the app `⋮` has no worktree entries.
- **Satisfies**: US4 acceptance scenarios 1–3; FR-009, FR-013 (worktree menu)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client features_header_bar`
- **Depends on**: M1
- **Tier**: full

### M3 — Single `+` menu

- **Tasks**: T017–T020
- **Deliverable**: One `+` after the tabs opens a menu with "Another session" and "Terminal"; the two add icons and the `Open in terminal` button are gone.
- **Satisfies**: US3 acceptance scenarios 1–4; FR-007, FR-008, FR-013 (`+` menu)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client features_session_strip`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — One-row header

- **Tasks**: T021–T027, T029, T031, T034, T035
- **Deliverable**: The structured view shows a single 40 px tab row with activity dots and a `N hidden` chip; the title row, `running` text, session bar and raw-events row are gone.
- **Satisfies**: US1 acceptance scenarios 1–4; US5 acceptance scenario 3; FR-001, FR-002, FR-006, FR-010, FR-011 (tabs), FR-012, FR-013 (chip)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test layout_snapshot header_row_fits`
- **Depends on**: M1, M2, M3
- **Tier**: full

T032 and T033 (quickstart Part B visual pass, FR-014, and bookkeeping) change no code and are left to the close unit.

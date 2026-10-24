---

description: "Task list for feature 1126: worktree context header and session tabs above the terminal"
---

# Tasks: Worktree context header and session tabs above the terminal

**Input**: Design documents from `specs/1126-session-context-header/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md) (R1–R14),
[data-model.md](data-model.md), [contracts/context-header.md](contracts/context-header.md) (H1–H10),
[contracts/session-area.md](contracts/session-area.md) (S1–S9, A1–A2), [quickstart.md](quickstart.md)

**Tests**: MANDATORY (Constitution I). Every test task is written first and seen failing before the
implementation task after it. A task that moves code with its tests (T003, T004) is green before and
after.

**Documentation**: each story's user-guide task is in its own phase (Constitution VII).

**Paths**: `core/` is `crates/micold-core/`, `client/` is `crates/micold-client/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup

**Purpose**: the new modules exist and compile empty, so later tasks only fill them.

- [x] T001 Declare the new modules: `pub mod context_header;` in `core/src/lib.rs` with an empty
  `core/src/context_header.rs` (module doc: text rules of the session area's header, render-free,
  no I/O); `pub mod session_strip;` in `client/src/features/mod.rs` with an empty
  `client/src/features/session_strip.rs` (module doc: render-free rules of the strip; it must not
  name `iced`, `tests/features_are_render_free.rs`); `pub mod session_area;` in
  `client/src/ui/mod.rs` with an empty `client/src/ui/session_area.rs`

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the session area's frame and the behaviour-preserving moves the strip builds on. No
visible change after this phase.

- [x] T002 Draw the session area through `client/src/ui/session_area.rs::view(state, id, ..)`:
  `column![terminal::pane(..)]` for now, and call it from the active-session branch of
  `client/src/ui/mod.rs::view_with` instead of `terminal::pane` (R12; touch only that branch, for the
  #1124/#1125 rebases). `tests/layout_snapshot.rs` stays green with the fixture unchanged
- [x] T003 Move `StripTab`, `marked_tab`, `process_stopped`, `session_status`, `restart_message`,
  `attached_process_restartable`, `session_title`, `overflowing`, `strip_overflow`, `scroll_into_view` and
  `marked_tab_index` with their unit tests from `client/src/ui/terminal.rs` into
  `client/src/features/session_strip.rs` (R7); `ui/terminal.rs` and `ui/panes.rs` import them from
  there, and `ui/terminal.rs` re-exports them (`pub use crate::features::session_strip::{StripTab,
  process_stopped, ..}`) so `ui/mod.rs::strip_tab_menu_items`/`strip_tab_menu_labels` and
  `tests/{app_state,popover_displacement,overlay_registration}.rs` keep the path
  `micold_client::ui::terminal::StripTab`; `client/src/features/session.rs`
  (`SessionState::shell_instance_menu`, `strip_tab_menu_requested`, `Msg::StripTabMenuRequested`)
  names `crate::features::session_strip::StripTab`. Moved tests pass unchanged;
  `tests/features_are_render_free.rs` passes
- [x] T004 [P] Promote `client/src/ui/transcript/keyed.rs::KeyedColumn` to
  `client/src/ui/cdk/keyed.rs` as `Keyed` with an axis (`Keyed::row(children)` /
  `Keyed::column(children)`, builder methods `spacing`, `padding`, `width`, ends in `.into()`;
  R10), export it from `client/src/ui/cdk/mod.rs`, switch `client/src/ui/transcript/` to it and
  delete `ui/transcript/keyed.rs`; its existing tests move with it and pass for both axes
- [x] T005 Add the header's follow entry: `HeaderCounts { entry: Option<(PathBuf, SessionLocation)> }`
  as `changes::State.header` and `Msg::HeaderFollowed { project, entry: Option<SessionLocation> }`
  in `client/src/features/changes.rs`, which sets or clears `entry` (data-model *HeaderCounts*:
  the follow rows without their read, which T048 adds with the `load` field); and the follow step
  at the end of `client/src/app.rs::State::update`: when `sidebar.current_session_location()` with
  the active project differs from `changes.header.entry`, apply `HeaderFollowed` (R6). Test first
  in `client/src/features/changes.rs` tests: a new entry is set; the same entry twice leaves the
  state unchanged; `None` clears it

**Checkpoint**: the app looks and behaves as before; the strip's rules live in `features`.

---

## Phase 3: User Story 1 — See which worktree I am in, above its terminal (P1) 🎯 MVP

**Goal**: a context header above the terminal names the shown location: title, state chip, branch
chip, pull request chip.

**Independent test**: open a worktree with an issue title, a branch and an open pull request; the
header shows each; switch worktrees and the header follows (quickstart B1 header part, B8).

### Tests for User Story 1 (write first, see them fail)

- [x] T006 [P] [US1] Core unit tests H1–H4 in `core/src/context_header.rs` (`#[cfg(test)] mod
  tests`): the four title rows and their tooltips (H1), each of the five card states' word
  `CardState::label` and `model.reason.label()` tooltip (H2), the three branch-chip rows (H3), the eight pull-request-chip rows and the stale tooltip note (H4); written against the
  two-argument `HeaderText::of` of T013, which T047 extends (these tests then pass `Unknown`)
- [x] T007 [P] [US1] Core unit test in `core/src/pr_list.rs` tests: `StatusPhrase::of_status` and
  `StatusPhrase::of` agree on every `(checks, review, conflict, state)` combination (one table test
  over both); `of_status` is `None` for merged and closed (R4)
- [x] T008 [P] [US1] Client unit test in `client/src/ui/overview.rs` tests: for the Default entry and
  for a worktree of a sidebar fixture, `location_card(state, &location)` equals the `CardModel` the
  Overview builds for the same place (title, state, reason, branch; SC-001), and `None` for a
  location that does not resolve
- [x] T009 [P] [US1] Client unit tests in `client/src/ui/session_area.rs` tests for
  `pr_chip_input(state, branch, now)`: `None` without a branch, `None` while `pr_status.enabled` is
  false, `None` with no status for the branch; otherwise `PrChipInput { number, state, review,
  conflict, stale: pull_request::is_stale(read_at, now) }` (data-model *PrChipInput*, FR-005)
- [x] T010 [P] [US1] Extend `client/tests/issues_are_requested_only_on_named_events.rs` and add a case
  to `client/tests/features_changes.rs` (through `State::update`): following a worktree whose issue
  title is not held asks for that one title once; following it again, or a session of the same
  location, asks nothing; the Default entry asks nothing (R3, H7 first two bullets); and a change of the location's
  state, branch or pull request reading applied through one `State::update` shows in the sidebar
  row and in `location_card`/`HeaderText::of` of the same state (US1 scenario 5, FR-014)
- [x] T011 [US1] Geometry gate `client/tests/gates/session_area_fits.rs` (compiled into the
  `layout_snapshot` binary, registered in `client/tests/layout_snapshot.rs`): at `MIN_WINDOW_SIZE`
  (640×480, sidebar 300 px) and 1280×800, light and dark, in the session covered states, the title
  and every chip lie inside the session area and overlap nothing (H10)

### Implementation for User Story 1

- [x] T012 [P] [US1] `StatusPhrase::of_status(&PullRequestStatus) -> Option<StatusPhrase>` in
  `core/src/pr_list.rs`, sharing one private candidate list with `of` (data-model *StatusPhrase*)
- [x] T013 [US1] `PrChipInput`, `Chip { text, tooltip }` and `HeaderText::of(&CardModel,
  Option<&PrChipInput>) -> HeaderText` with `title`, `title_tooltip`, `state`, `branch`,
  `pull_request` in `core/src/context_header.rs` (contract H1–H4; `branch` uses 1120's
  `short_branch`, falling back to the full branch; the `changes` field and the `CountsReading`
  argument come in T047); T006 and T007 pass
- [x] T014 [US1] `pub(crate) fn location_card(state, &SessionLocation) -> Option<CardModel>` in
  `client/src/ui/overview.rs`: resolves the `Place` (Default, or the worktree node with that
  `dir_name`) and calls the existing `card` (R2); T008 passes
- [x] T015 [US1] Split `client/src/shell/issues.rs::on_overview_opened` into
  `request_titles(app, dirs: &[String])` (seed, `titles_to_fetch`, `send_remotes_read`) called by
  `on_overview_opened` with every worktree's dir as today, and by the shell on a header follow
  (T005's `HeaderFollowed` to a new worktree) with that worktree's dir only (R3); T010 passes
- [x] T016 [P] [US1] `Tag::leading(Element)` (a glyph or `PullRequestIndicator` before the label) and
  `Tag::max_width(f32)` (ellipsis under a ceiling, the chip sized by its content below it; unlike
  `truncate_to`, which fixes the width) in `client/src/ui/material/tag.rs`, with builder-API tests
  as its siblings have (`tests/material_builder_api.rs` stays green); show both in the showcase's
  atoms section `client/src/showcase/sections/atoms.rs` (`tests/showcase_completeness.rs`)
- [x] T017 [US1] `session_area::header(state, &location)` in `client/src/ui/session_area.rs` (H9
  without the Changes button, which T052 adds): `column![title, chips]` at `Length::Fill`, padding
  `spacing::LG` horizontally; title `Ellipsized` at `TypeRole::Title` in a `Tooltip` with
  `title_tooltip`; chips row: state chip always (`overview_card::palette(state)` fill,
  `state_icon(state)` leading, the word; tooltip the reason), then branch and pull request chips when
  present (neutral `on_surface_variant` accent, pull request chip led by `PullRequestIndicator`, stale
  form under the sidebar's rule); each chip `.max_width(CHIP_MAX_WIDTH)` (new const) in a `Tooltip`
  with its full text; child lists fixed. `session_area::view` draws it above the pane area when
  `current_session_location()` resolves (A1, R12); T009, T011 pass
- [x] T018 [US1] Layout snapshot: add the anchor `session.header` to the session covered states in
  `client/tests/support/covered_states.rs`, regenerate `client/tests/fixtures/layout_snapshot.txt`
  (`UPDATE_LAYOUT_SNAPSHOT=1`), and check its diff changes only the session states (header added
  above the panes); `docs/development/layout-snapshot.md` *What is covered* names the header
- [x] T019 [P] [US1] User guide: `docs/user-guide/terminal-panes.md` gains a short section above
  *Split and pick a terminal* describing the header (title rule, state, branch and pull request
  chips, tooltips for cut text); the bar under the terminal is unchanged in this milestone
- [x] T020 [US1] Visual pass for the header (quickstart B1 header part, B8, B9 header part, B12),
  screenshots under `specs/1126-session-context-header/visual-pass/` and results in
  `specs/1126-session-context-header/visual-pass.md`

**Checkpoint**: the header names the shown location above its terminal; the bar is still below.

---

## Phase 4: User Story 2 — Switch, add and split from a tab strip above the terminal (P1)

**Goal**: the tab strip under the header holds the session tabs, the shown session's shell tabs and
"+", status and restart, **New session** and the split buttons; the bar under the terminal is gone.

**Independent test**: with two tabs, switch between them, open a new one and split both ways from
the strip; no bar under the terminal (quickstart B1–B5, B9, B10).

### Tests for User Story 2 (write first, see them fail)

- [x] T021 [P] [US2] Unit tests S1–S4 in `client/src/features/session_strip.rs` tests for
  `StripModel::of`: the S1 fixtures (main `M` with two shells and helpers `H1`, `H2`; `M` shown, `H1`
  shown; one session no shell → `[Session(M, shown, marked), Plus]`; an agent-started session after
  its origin, folded or not), exactly one member marked (S2), stopped per member and never across
  sessions, `restart` `Some` exactly when the shown marked tab is stopped and targeting that process
  (S3), `status` words unchanged (S4), `location` `None` when it no longer resolves; `None` without an
  active session
- [x] T022 [P] [US2] Unit tests S5 in `client/src/features/session_strip.rs` tests: each member's
  press and close message per the S5 table; another session's tab yields no
  `StripTabMenuRequested`; and in `client/src/ui/mod.rs` tests, for every member of the S1 fixtures,
  `strip_tab_menu_labels` contains *Restart* iff that member is `stopped` (FR-009)
- [x] T023 [P] [US2] Unit tests S6 in `client/src/features/session_strip.rs` tests:
  `member_offsets` from `SESSION_WIDTH`, `tab::WIDTH`, the "+" size and `spacing::SM` gaps; the three
  `strip_overflow` rows of the S6 table (viewport 0 means no edge beyond); `scroll_into_view` brings
  a marked member beyond either edge into view
- [x] T024 [P] [US2] Keyed slot test in `client/src/ui/cdk/keyed.rs` tests: in a `Keyed::row`, a
  member added before the pressed child keeps that child's tree state with its key (S8, R10)
- [x] T025 [P] [US2] `client/tests/session_area_stability.rs` replacing
  `client/tests/terminal_bar_stability.rs` (delete it): source-level gates that
  `session_area::strip` and `session_area::header` read no focus (`terminal_focused`), push no child
  under an `if` on session shape, and have no release-focus control (S8, FR-011); carry over every pin of the old file that still
  applies (restart wiring through `restart_message`, the `command_caption` label) and repoint the
  comments that cite it (`src/ui/terminal.rs`, `src/features/session.rs`,
  `tests/{terminal_tabs,terminal_empty_state,features_session}.rs`)
- [x] T026 [P] [US2] Retarget `client/tests/terminal_tabs.rs` and `client/tests/tab_reveal.rs` to
  the strip: pressing a session, shell, "+" or New session member sends the S5 message, and every
  strip control that changes the marked tab arms the reveal (`arm_tab_reveal`)
- [x] T027 [US2] Geometry gates: rename `client/tests/gates/bar_controls_hold_their_size.rs` →
  `strip_controls_hold_their_size.rs` (every member and fixed control at its requested width);
  retire `client/tests/gates/tabs_anchor_the_trailing_edge.rs` for
  `client/tests/gates/strip_controls_anchor_the_trailing_edge.rs` (New session and the split buttons
  are the strip's last children, in order, flush with its trailing padding); re-anchor
  `client/tests/gates/tab_children_fit.rs` on the strip; retarget
  `client/tests/gates/containment.rs`'s `the_recorded_tab_overflow_is_the_instance_strip` to
  `session-area-twelve-tabs` and the `EdgeFade` → `Scrollable` → keyed-row path (no padding
  container), its member count read from `covered_states.rs`, "+" included, doc comment naming the
  strip (R14); register the new files in `client/tests/layout_snapshot.rs`
- [x] T028 [US2] Extend `client/tests/gates/session_area_fits.rs`: with 1, 6 and 12 members at both
  sizes and both themes, no strip element overlaps another or leaves the window, New session and
  the split buttons whole (S9, SC-004), and nothing is laid out below the pane area inside the session area (A1, SC-005); with two panes, the header and the strip are each laid out
  once, above the pane area, and each pane keeps its own header (A2)
- [x] T029 [US2] Covered states in `client/tests/support/covered_states.rs`:
  `session-terminal-bottom-bar` → `session-area-one-tab`, `session-terminal-instance-tabs` →
  `session-area-six-tabs`, `session-terminal-instance-tabs-overflowing` →
  `session-area-twelve-tabs` (member counts 1, 6, 12, session and shell tabs together); anchors
  `session.strip`, `session.strip.tabs`, `session.strip.new_session`, `session.strip.split`; doc
  comments that named the bar name the strip

### Implementation for User Story 2

- [x] T030 [P] [US2] `SESSION_LABEL_WIDTH = LABEL_MAX_WIDTH` and `SESSION_WIDTH = SLOT_WIDTH + XS +
  SESSION_LABEL_WIDTH + XS + SLOT_WIDTH` in `client/src/ui/material/tab.rs`, with a test beside
  `WIDTH`'s that it is derived, not literal (R8)
- [x] T031 [P] [US2] `new_terminal_chord_label()` in `client/src/keymap.rs` as
  `overview_chord_label`, with a test for both platform words (R13)
- [x] T032 [US2] `StripModel::of`, `Member` (`Session { id, label, tooltip, shown, marked, stopped
  }`, `Shell { session, instance, label, marked, stopped }`, `Plus`), `MemberKey` and
  `member_offsets` in `client/src/features/session_strip.rs` (data-model *StripModel*): sessions in
  `navigation::location_node(..).unfolded_sessions()` order, the shown session's shells in creation
  order and `Plus` right after its tab; label `<command> · <session name>`
  (`session::command_caption`, `session_strip::session_title`), tooltip label + role (main or helper) +
  status; `strip_overflow`/`scroll_into_view` read `member_offsets`; T021–T023 pass
- [x] T033 [US2] `session_area::strip(state, &model)` in `client/src/ui/session_area.rs` (S7):
  `row![EdgeFade(Scrollable(Keyed::row(members)).id(TAB_STRIP_SCROLL_ID)) Fill, status_cluster,
  new_session, split_buttons]`, child list fixed; session tabs `Tab::new` at `SESSION_WIDTH` with the
  stopped mark leading and an empty reserved trailing slot; shell tabs as today; "+" with the
  `new_terminal_chord_label()` tooltip; status `Ellipsized` capped at `STATUS_MAX_WIDTH` (new
  const) with restart last; New session `IconButton(Icon::AddSession)` (tooltip *Start another
  session…*) sending `Session(AdditionalRequested(location))`; `panes::split_buttons` of the focused
  pane; indicator edge `Bottom`; the right-click menu per S5 through `strip_tab_menu_items`; marked
  edge cue per feature 027 FR-002e; T022, T024, T026 pass
- [x] T034 [US2] `session_area::view` draws header, strip, then `terminal::pane` (A1, A2); remove the
  bottom bar from `client/src/ui/terminal.rs::pane` together with `tab_strip_row`, `pinned_ai_tab`,
  `right_aligned_tabs`, `leading_slack` and `natural_strip_width` and their tests; `ui/panes.rs::header`
  unchanged; T025, T027–T029 pass
- [x] T035 [P] [US2] Show the strip's session tab (shown, other, stopped) in
  `client/src/showcase/sections/terminal.rs` (`tests/showcase_completeness.rs`)
- [x] T036 [US2] Regenerate `client/tests/fixtures/layout_snapshot.txt` and check its diff changes only
  the session-area states; `docs/development/layout-snapshot.md` *What is covered* lists
  `session-area-one-tab`, `-six-tabs`, `-twelve-tabs` and drops the bar's states
- [x] T037 [P] [US2] User guide: `docs/user-guide/terminal-panes.md` *Split and pick a terminal*
  describes the strip above the terminal (session tabs, the shown session's shell tabs and "+", New
  session with its confirmation, status and restart, splits at the right, scrolling with a faded
  edge) and no bar under the terminal; any other page that names the bar under the terminal
  (`grep -rn "bar under\|bottom bar" docs/user-guide`) is corrected
- [x] T038 [US2] Visual pass B1–B5, B9, B10, B12 (quickstart Part B), appended to
  `specs/1126-session-context-header/visual-pass.md`

**Checkpoint**: header and strip above the panes; nothing under them.

---

## Phase 5: User Story 4 — Keyboard and focus work as before (P1)

**Goal**: pane chords and focus act exactly as before the strip replaced the bar. Ships with US2,
whose change it guards.

**Independent test**: every chord of `terminal-panes.md#keyboard-shortcuts` with the terminal
focused; presses on header and strip controls leave focus as the bar did (quickstart B11).

- [x] T039 [P] [US4] Guard test (passes on arrival: it pins the bar's outcome and must stay green
  through T033–T034; written before T034) in `client/tests/terminal_focus.rs`: with the terminal focused, pressing a
  strip member, "+", a split button or restart leaves focus where the same bar control left it
  (US4 scenario 2); pressing New session leaves focus to its confirmation as the sidebar action does
- [x] T040 [P] [US4] Test, written before T033 and seen failing against a positional row, in `client/tests/terminal_focus.rs` (or `session_area_stability.rs`): a
  press in flight on a strip member is delivered to that member while a process stops, a chip
  appears or a shell instance opens before it (US4 scenario 3, FR-011)
- [x] T041 [US4] Keep `client/tests/keyboard.rs`, `client/tests/navigation.rs` and the lib's
  `ui::app_chord_tests` green with no assertion changed (FR-012, SC-002); any edit there is a
  behaviour change and goes back to the spec. CI runs these and the geometry gates on Linux, macOS
  and Windows, which is SC-002's and SC-004's per-platform check; the visual pass is Linux only and
  says so
- [x] T042 [US4] Visual pass B11 (every chord), appended to
  `specs/1126-session-context-header/visual-pass.md`

**Checkpoint**: US1, US2 and US4 complete.

---

## Phase 6: User Story 3 — Open the worktree's changes from the header (P2)

**Goal**: the header's Changes button shows the changed files and line counts and opens or closes the
Changes view.

**Independent test**: a worktree with three changed files reads *Changes · 3 files · +42 −7*;
pressing it opens the Changes view for it; pressing again returns (quickstart B6, B7).

### Tests for User Story 3 (write first, see them fail)

- [x] T043 [P] [US3] Core unit tests H5 and H6 in `core/src/context_header.rs` tests: the four label
  rows (`−` is U+2212), and `ChangeSummary::of` over a text (+10 −2), a binary (0, 0) and a renamed
  (+1 −1) file is `{ files: 3, added: 11, removed: 3 }` (saturating sums)
- [x] T044 [P] [US3] `HeaderCounts` transition tests in `client/src/features/changes.rs` tests, one
  per row of data-model *HeaderCounts*: follow to a new entry → `Loading { seq }` and `ReadCounts`;
  current-seq `Ok` → `Ready`, with a queued `again` reading once more; current-seq `Err` →
  `Failed`; old seq dropped; `Changed` from `Ready`/`Failed`/`Idle` with an entry → `Loading { last }`
  and `ReadCounts`; `Changed` while `Loading` → `again`; `WorktreesListed` without the entry's dir →
  `entry: None`, `Idle`; and `CountsReading` is `Unknown` for `Idle`, `Loading { .. }` whatever
  `last` holds, and `Failed` (US3 scenario 4)
- [x] T045 [P] [US3] H7 integration cases in `client/tests/features_changes.rs` through
  `State::update`: selecting a session of another location sends one `ReadCounts` for it; another
  session of the same location sends none; no active session or its worktree removed → entry
  `None`, no read; an answer for an earlier location is dropped; `Msg::Changed` with the view open on
  A and the header on B reads both lists
- [x] T046 [P] [US3] Unit test of `session_area::changes_press(state, &location)` in
  `client/src/ui/session_area.rs` tests: the three H8 rows (closed → `ReviewChangesRequested(shown)`;
  open on the shown location → `changes::Msg::Closed`, selected; open on another →
  `ReviewChangesRequested(shown)`, not selected)

### Implementation for User Story 3

- [x] T047 [US3] `ChangeSummary { files, added, removed }` with `ChangeSummary::of(&ChangeList)`,
  `CountsReading { Unknown, Known(ChangeSummary) }`, and `HeaderText.changes` (with the third
  argument `CountsReading` to `HeaderText::of`) in `core/src/context_header.rs` (H5, H6); T043 passes
- [x] T048 [US3] `Msg::HeaderCountsRead { seq, result: Result<ChangeSummary, String> }`,
  `Effect::ReadCounts { seq, entry }`, the field `HeaderCounts.load: Load<ChangeSummary>` and the
  `HeaderFollowed`, `Changed` and `WorktreesListed` transitions of data-model *HeaderCounts* in
  `client/src/features/changes.rs`; the effect goes out through `changes.pending` /
  `App::take_changes_effect`, batched with one already there (R6); T044 passes
- [x] T049 [US3] Run `ReadCounts` in `client/src/shell/changes.rs` with the view's `local` +
  `read_list` under `Toggles::default()`, reducing with `ChangeSummary::of` off the update thread;
  `local`'s refusal (service on another computer) answers `Err` (R5); T045 passes
- [x] T050 [US3] In `client/src/shell/subscriptions.rs`, also watch the header's entry while a session
  is shown and the Overview is closed, keyed so a view and a header on the same worktree share one
  watcher (R6); `client/tests/idle_subscriptions.rs` stays green (no watch without a shown session)
- [x] T051 [US3] `session_area::changes_press` in `client/src/ui/session_area.rs` (H8); T046 passes
- [x] T052 [US3] The Changes button in `session_area::header`: `material::Button` outlined with
  `Icon::Git` leading, label `HeaderText.changes`, `.selected(view open on this location)`, tooltip
  *Open the Changes view* / *Close the Changes view*, never narrower than it asks (H9); the header's
  row becomes `row![column![title, chips], changes_button]` with a fixed child list; and the
  Changes branch of `client/src/ui/mod.rs::view_with` draws `column![header, changes view]` when a
  session is shown, no strip, no header without one (R12, A1)
- [x] T053 [US3] Gate and snapshot (the gate part written before T052 and seen failing):
  `client/tests/gates/session_area_fits.rs` checks the Changes
  button is laid out at its requested width at both sizes and themes (H10); new covered state
  `session-area-changes-open` (header over the Changes view) and anchor `session.header.changes` in
  `client/tests/support/covered_states.rs`; regenerate the fixture and check only session states
  change; `docs/development/layout-snapshot.md` *What is covered* lists it
- [x] T054 [P] [US3] User guide: `docs/user-guide/reviewing-changes.md` *Opening the Changes view*:
  the header's Changes button, what its counts mean (no changes, no counts while reading), pressing
  it again to close; the right-click **Review changes** route still listed
- [x] T055 [US3] Visual pass B6, B7 (light and dark), appended to
  `specs/1126-session-context-header/visual-pass.md`

**Checkpoint**: all stories complete.

---

## Phase 7: Polish & cross-cutting

- [ ] T056 Run quickstart Part A end to end and record the results in
  `specs/1126-session-context-header/quickstart.md`
- [ ] T057 Run the whole quickstart Part B once more on the final tree, light and dark, and complete
  `specs/1126-session-context-header/visual-pass.md`

---

## Dependencies & execution order

### Phase dependencies

- Setup (T001) → Foundational (T002–T005) → US1 (T006–T020) → US2 (T021–T038) with US4
  (T039–T042) → US3 (T043–T055) → Polish (T056–T057).
- US3 depends on US1 (the header it adds a button to) and on T005 (the follow entry). It does not
  depend on US2, but runs after it so the session area's layout and snapshot settle once.
- US4 has no code of its own: it guards US2's bar removal and ships with it.

### Within each story

- Tests first, seen failing; core before client; rules (`features`, core) before views; views before
  gates, snapshot regeneration and the visual pass.
- T018, T036, T053 regenerate the fixture after every layout task of their story is done.

### Parallel opportunities

- Foundational: T004 beside T003 (different modules).
- US1: T006–T010 together; T012 and T016 beside T013–T015.
- US2: T021–T026 together; T030, T031, T035, T037 beside the model and view work.
- US3: T043–T046 together; T054 beside T047–T053.

## Parallel example: User Story 2

```text
T021 S1–S4 tests   T022 S5 tests   T023 S6 tests   T024 keyed slot test
T025 stability gates   T026 tabs/reveal retarget   T030 SESSION_WIDTH   T031 chord label
```

## Implementation strategy

### MVP first

Setup + Foundational + US1: the header names the location above its terminal. Stop, verify
(quickstart B1 header part, B8), merge.

### Incremental delivery

1. M1: header (US1). The bar under the terminal still works.
2. M2: strip replaces the bar (US2), with keyboard and focus guards (US4).
3. M3: Changes button with counts (US3).
4. Polish tasks go to the close unit (no code).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Context header above the terminal 🎯 MVP

- **Tasks**: T001–T020
- **Deliverable**: with a session shown, a header above its terminal shows the location's title,
  state chip, short branch chip and pull request chip, the same as its Overview card, and follows
  the shown location; the bar under the terminal is unchanged
- **Satisfies**: US1 acceptance scenarios 1–6; FR-002, FR-003, FR-004, FR-005, FR-013 (header),
  FR-014 (title, state, branch, pull request), FR-015 (header docs, snapshot, visual
  pass); SC-001
- **Verify**: `mise run test-core` (H1–H4, `of_status`); `scripts/build-lock.sh cargo test -p
  micold-client --lib -- overview::tests session_area changes::tests`; `scripts/build-lock.sh cargo
  test -p micold-client --test layout_snapshot --test features_changes --test
  issues_are_requested_only_on_named_events`; quickstart B1 (header part), B8, B12
- **Depends on**: —
- **Tier**: full

### M2 — Tab strip replaces the bar under the terminal

- **Tasks**: T021–T042
- **Deliverable**: the strip under the header switches sessions and shell tabs, opens a shell
  instance and a new session, splits, shows status and restart, and scrolls with the marked-edge
  cue; nothing is drawn under the terminal; every pane chord and focus rule acts as before
- **Satisfies**: US2 acceptance scenarios 1–8; US4 acceptance scenarios 1–3; FR-001, FR-008,
  FR-009, FR-010, FR-011, FR-012, FR-013 (strip), FR-015 (strip docs, snapshot, visual pass); SC-002, SC-004 (header and strip), SC-005, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib -- session_strip cdk::keyed
  app_chord_tests`; `scripts/build-lock.sh cargo test -p micold-client --test session_area_stability
  --test terminal_tabs --test tab_reveal --test terminal_focus --test keyboard --test navigation
  --test layout_snapshot`; quickstart B1–B5, B9–B12
- **Depends on**: M1
- **Tier**: full

### M3 — Changes button with counts in the header

- **Tasks**: T043–T055
- **Deliverable**: the header's Changes button reads the shown location's changed files and line
  counts (*no changes*, or no counts while reading), opens the Changes view under the header, shows
  it open and closes it on a second press; the counts follow edits
- **Satisfies**: US3 acceptance scenarios 1–5; FR-006, FR-007, FR-014 (counts), FR-015 (Changes docs, snapshot, visual pass); SC-003
- **Verify**: `mise run test-core` (H5, H6); `scripts/build-lock.sh cargo test -p micold-client --lib
  -- changes::tests session_area`; `scripts/build-lock.sh cargo test -p micold-client --test
  features_changes --test idle_subscriptions --test layout_snapshot`; quickstart B6, B7
- **Depends on**: M1 (header, T005); ordered after M2 so the session area's covered states and
  fixture settle once (no code of M2 is needed)
- **Tier**: full

Polish (T056–T057) changes no code and is left to the close unit.

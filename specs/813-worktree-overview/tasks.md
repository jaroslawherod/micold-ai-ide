---

description: "Task list for feature 813: Worktree Overview"
---

# Tasks: Worktree Overview

**Input**: Design documents from `specs/813-worktree-overview/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/ui.md](./contracts/ui.md),
[contracts/wire.md](./contracts/wire.md), [quickstart.md](./quickstart.md)

**Tests**: Constitution Principle I (Test-First, NON-NEGOTIABLE): every test task is written first
and seen failing for the right reason before the implementation task after it. Glue (`ui/` view
wiring, `shell/` subscription glue, showcase sections) is exercised by quickstart §B: here
`ui/overview.rs`, the toolbar button, the two `ui/mod.rs` key listeners (their key mapping is
tested), focusing the first card on open, and the title-read and watch subscriptions (their
parsers, debouncer and reducer are tested).

**Documentation**: Principle VII: each story's user-guide task is in that story's phase and
milestone (CI's user-guide gate needs it in the same `feat` PR).

**Cross-platform**: Principle VI: the chord is one predicate over the platform modifier; glyphs are
bundled; CI runs Linux, macOS and Windows.

**Paths**: `crates/` prefix omitted for `micold-core/`, `micold-client/`, `micold-daemon/`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story (US1–US5); US1 is cut into two milestones (M1, M2)

---

## Phase 1: Setup

- [X] T001 Register the empty modules with their module docs: `pub mod overview; pub mod
  task_progress; pub mod hook_activity;` in micold-core/src/lib.rs (files
  micold-core/src/overview.rs, micold-core/src/task_progress.rs, micold-core/src/hook_activity.rs
  holding only `//!` docs naming feature 813), and `pub mod overview;` in
  micold-client/src/features/mod.rs (file micold-client/src/features/overview.rs, docs only)

---

## Phase 2: Foundational (blocks every story)

**Purpose**: the theme roles and icons every card state and the button use (R11).

- [X] T002 [P] Test first: in micold-client/src/ui/material/composition_contrast.rs add the pairs
  `on_warning`/`warning`, `on_warning_container`/`warning_container`, `on_success`/`success`,
  `on_success_container`/`success_container` at ≥ 4.5:1 in light and dark, and in the existing
  roles snapshot test (micold-core/src/tokens/mod.rs tests) the eight new role names; both fail
  (roles missing)
- [X] T003 Add `warning`, `on_warning`, `warning_container`, `on_warning_container`, `success`,
  `on_success`, `success_container`, `on_success_container` to `Roles` in
  micold-core/src/tokens/mod.rs, as fixed palette tones per scheme in micold-core/src/tokens/palette.rs
  (`WARNING_*_LIGHT`/`_DARK`, `SUCCESS_*_LIGHT`/`_DARK`), the way `diff_added` uses
  `DIFF_ADDED_LIGHT`/`DIFF_ADDED_DARK`; export them wherever `diff_added` is exported
  (micold-core/src/tokens/css.rs if it lists roles); T002 passes
- [X] T004 [P] Test first: pin codepoints for `Icon::Overview`, `CardNeedsAction`,
  `CardNeedsAttention`, `CardDone`, `TypeFeature`, `TypeBugfix`, `TypeChore`, `TypeOther`,
  `TypeIncluded`, `TypeOrphan` in micold-client/tests/icons.rs (distinct from every other icon) and
  micold-client/tests/icons_font.rs (glyph present in `assets/fonts/MaterialSymbolsOutlined.ttf`)
- [X] T005 Add the ten icons to micold-client/src/icons.rs (enum, codepoint, the all-icons list),
  choosing Material Symbols glyphs present in the bundled font; T004 passes

**Checkpoint**: roles and icons exist; no behaviour changed.

---

## Phase 3: User Story 1 — See every worktree's state in one place (P1) 🎯 MVP

**Goal**: an Overview in the main pane, opened and closed from a top-bar button, with one card per
sidebar location showing name, branch, type, session counts and a card state by colour, icon and
word, ordered worst first, updating live.

**Independent Test**: spec US1's: root, three worktrees (one waiting for a permission, one working,
one without a session) and one included worktree → five cards, the permission card first, red,
*Needs action*; the idle card greyed.

### Part A — session and existing pull request signals (milestone M1)

#### Tests for US1 part A (write first, see them fail)

- [X] T006 [P] [US1] Core tests in micold-core/src/overview.rs `#[cfg(test)]`: `location_type`
  precedence "ProjectRoot (Default) → Included → OrphanInvalid (`Missing` / `Invalid`) → branch
  type (`feat` → Feature, `fix` → Bugfix, other conventional type → Chore, none → Other)" with one
  case per arm and per conventional type (`chore`, `docs`, `refactor`, `test`, `build`, `ci`,
  `perf`, `style`); `LocationType::label()` words *Feature*, *Bugfix*, *Chore*, *Other*,
  *Included*, *Orphan*, *Project root*
- [X] T007 [US1] Core tests in micold-core/src/overview.rs: `card_state` over every
  `LocationState` variant × `agent_request` {false, true} × `PrSignal` {`Unknown`, `Known` with no
  current, current open/draft with each of `checks_failing`, `changes_requested`, `conflict`,
  current merged with `removable` true/false} against data-model's table rows 1–10 (state and
  reason), generated exhaustively (SC-003); invariants as named tests: `Unknown` PR never yields
  rows 5–7 or 9; `LocationState::Unknown` never yields Working or Done by itself; rows 5–7 only
  for an open or draft current pull request; several signals → the first row (US1.5)
- [X] T008 [US1] Core tests in micold-core/src/overview.rs: `session_counts` skips archived
  sessions and counts running as 905 `session_state == Running` (US1.2: 3 sessions, 2 running);
  `order` is stable by state worst first with ties in input (sidebar) order (FR-007);
  `attention` returns the worst state with its card count only for NeedsAction or NeedsAttention,
  else `None` (FR-012)
- [X] T009 [P] [US1] Client test in micold-client/tests/features_agent_confirm.rs: a queued
  `Prompt` carries `session`, the caller of `ConfirmationRequested` (W4)
- [X] T010 [P] [US1] Client state tests in micold-client/tests/overview.rs (new):
  `update(Toggled { settings_open: false })` opens then closes; `Toggled { settings_open: true }`
  changes nothing (R2); `Closed` closes; a project switch keeps `open` and the cards follow the new
  project; `cards` from app state gives exactly one card per `sidebar_entries()` entry, in sidebar
  order before `order`, including included, orphan / invalid and Default (FR-001, edge: empty
  project → one card); counts per US1.2; an agent prompt queued for a session of a location →
  that card NeedsAction *agent request waiting* (R5); a pull request status with failing checks →
  NeedsAttention, merged + removable → Done, Default never PR-derived (R6 Default row); summary is
  the branch, else the display name, Default *Project root* (FR-015 without titles); after
  `shell::daemon_sync::on_disconnected`'s effects (catalog removed, pull request reading
  `Released`) the session-derived states are unchanged and PR-derived ones are not claimed (R14,
  decision D12); `App::terminal_focused()` is false and `main_area_taken` true while open; `cards` over 30 locations with 3 sessions each builds
  within 1 s (SC-007, a loose bound)
- [X] T011 [P] [US1] Geometry tests in micold-client/src/ui/material/overview_card.rs (new file,
  tests module first): card height equal for the five states with the same content; no text
  overlap at 280 px; a long display name ellipsizes with the full text in its tooltip (U4); each card shows its type icon with
  `LocationType::label()` and its state icon with `CardState::label()` (SC-005)
- [X] T012 [P] [US1] Geometry tests in micold-client/src/ui/material/card_grid.rs (new file, tests
  first): columns `= max(1, floor((w + gap) / (min + gap)))`; one column at 279 px; no card
  overlaps another; 30 cards scroll vertically (U4)
- [X] T013 [P] [US1] Contrast tests in micold-client/src/ui/material/composition_contrast.rs: per
  card state its text role ≥ 4.5:1 and icon role ≥ 3:1 on its card fill (`error_container`,
  `warning_container`, `primary_container`, `success_container`, Idle `surface_container_low` with
  `on_surface_variant` ≥ 4.5:1), light and dark (FR-005, FR-006, SC-005)
- [X] T014 [P] [US1] Test in micold-client/src/main_tests.rs: with the Overview open,
  `Message::Session(SessionMsg::Selected(id))` closes it and shows that session (FR-023)

#### Implementation for US1 part A

- [X] T015 [US1] Implement in micold-core/src/overview.rs: `LocationType` (+ `label()`),
  `CardSignals { location, agent_request, pr }`, `PrSignal::{Unknown, Known { current:
  Option<CurrentPr>, removable }}`, `CurrentPr { checks_failing, changes_requested, conflict,
  merged, open }`, `CardState` (derived `Ord`: `NeedsAction < NeedsAttention < Working < Done <
  Idle`, `label()`), `CardReason` (+ tooltip word), `card_state` (data-model table, first row
  wins), `location_type`, `session_counts`, `CardModel { location, display_name, branch, kind,
  sessions, state, reason, summary }`, `order`, `Attention { state, count }`, `attention`; T006–T008
  pass. Later milestones add `CardModel::prs`, `progress`, `activity`, `activation`
- [X] T016 [US1] Add `session: SessionId` to `Prompt` in micold-client/src/features/agent_confirm.rs
  and fill it from `DaemonMsg::ConfirmationRequested.caller` in
  micold-client/src/shell/daemon_sync.rs; update every `Prompt { .. }` literal; T009 passes
- [X] T017 [US1] Implement micold-client/src/features/overview.rs: `State { open }`, `Msg::{Toggled
  { settings_open }, Closed}`, `update`, and `cards(&crate::app::State) -> Vec<CardModel>`
  building `CardSignals` from `sidebar_entries()`, the project's sessions (`location_state`,
  `main_session` of micold-core), `agent_confirm`'s queue (`Prompt::session` → `Session::location`)
  and `pr_status` (`PrSignal::Known` from the branch's `PullRequestStatus` and the row's removable
  signal, `Unknown` when the reading is off, unavailable, released or not read, or for Default);
  add `overview: overview::State` to `crate::app::State`, `Message::Overview(overview::Msg)` in
  micold-client/src/app.rs, and the Overview to `terminal_focused` and
  `view_facts.main_area_taken`; T010 passes
- [X] T018 [US1] Implement `OverviewCard` in micold-client/src/ui/material/overview_card.rs:
  `new(&CardModel, roles) -> Element`, layout per contracts/ui.md U3 (type icon + display name
  ellipsized with tooltip, state icon + word with the reason as tooltip, branch, summary, footer
  *N sessions, M running*), state fill and icon from R11's mapping, Idle on the de-emphasised
  surface; no press handler yet (M4 adds it); register in ui/material/mod.rs; T011, T013 pass
- [X] T019 [US1] Implement `CardGrid` in micold-client/src/ui/material/card_grid.rs:
  `new(cards).min_width(280.0).spacing(..)`, columns from the available width through iced
  `responsive`, one column minimum, vertical scroll; register in ui/material/mod.rs; T012 passes
- [X] T020 [US1] Showcase: `OverviewCard` in each of the five states plus Default and orphan
  variants, and `CardGrid` with 1, 3 and 30 cards and narrow, in
  micold-client/src/showcase/catalogue.rs and micold-client/src/showcase/sections/surfaces.rs;
  extend micold-client/tests/showcase_completeness.rs for both entries (FR-021)
- [X] T021 [US1] Wire the view: micold-client/src/ui/overview.rs (new) `view` → `CardGrid` of
  `OverviewCard`s from `overview::cards` after `order`; micold-client/src/ui/mod.rs `view_with`
  order Settings → Overview → Changes → Compare → terminal → `shell::view`, `main_content_key`
  `u64::MAX - 3` while open (U1)
- [X] T022 [US1] Top-bar button in micold-client/src/ui/toolbar.rs after the project switcher:
  `Button::text("Overview", r).leading(Icon::Overview).tooltip("Overview")`, pressing sends
  `overview::Msg::Toggled { settings_open }`, disabled while Settings is open (R2); route
  `Message::Overview` in micold-client/src/main.rs and send `overview::Msg::Closed` from the
  `SessionMsg::Selected` arm; T014 passes
- [X] T023 [US1] User guide: new docs/user-guide/overview.md (opening and closing with the
  button, one card per location, the location types with their icons, session counts, the five
  card states with what puts a location in each from data-model's table, worst first, live
  updates, stale data under the connection banner), linked from docs/SUMMARY.md; the new icons'
  rows in docs/user-guide/icons.md (FR-022)

### Part B — pull request counts and the conflict signal (milestone M2)

#### Tests for US1 part B (write first)

- [x] T024 [P] [US1] Core tests in micold-core/src/pull_request.rs: `FRAGMENT` asks `mergeable`;
  `status_query(n)` has `oc{i}`/`mc{i}` `pullRequests(headRefName: $b{i}, states: OPEN|MERGED,
  first: 100) { nodes { isCrossRepository } }` per branch; `parse_status` → `StatusReading
  { statuses, counts }` with W5's table (cross-repo nodes not counted; 0/0 entry for every asked
  branch; 100 nodes → `merged_capped`; missing `oc` alias → whole reading
  `ReadingFailure::Passing`); `PullRequestStatus::conflict` `CONFLICTING` → true, `MERGEABLE` /
  `UNKNOWN` / missing → false
- [x] T025 [P] [US1] Client tests in micold-client/tests/features_pr_status.rs: `Msg::StatusesRead`
  stores `counts`; `Unavailable`, `Released` and disabling clear `counts` exactly when they clear
  `statuses`
- [x] T026 [P] [US1] Client tests in micold-client/tests/overview.rs: a branch with 1 open and 2
  merged reads `PrDisplay::Counts` 1 / 2 (US1.3); unknown with each reason — *pull request status
  is off*, *GitHub is not reachable*, *rate-limited until HH:MM*, *not read yet* (FR-004, edge
  case); `Hidden` for Default, detached HEAD and orphan; a current open pull request with
  `conflict` → NeedsAttention *merge conflict*

#### Implementation for US1 part B

- [x] T027 [US1] Implement in micold-core/src/pull_request.rs: `mergeable` in `FRAGMENT`,
  `PullRequestStatus::conflict`, the `oc`/`mc` aliases in `status_query`, `StatusReading`,
  `parse_status` and `read_outcome` returning it, `PrCounts { open: u8, merged: u8, open_capped:
  bool, merged_capped: bool }` in micold-core/src/overview.rs; no new interval, trigger or `gh`
  call (FR-019); T024 passes
- [x] T028 [US1] Carry counts in micold-client/src/features/pr_status.rs (`State::counts`,
  `Outcome::Ok`, `Msg::StatusesRead`) and micold-client/src/shell/pr_status.rs; T025 passes
- [x] T029 [US1] `CardModel::prs: PrDisplay::{Hidden, Unknown(reason), Counts(PrCounts)}` and
  `CurrentPr::conflict` from the reading in micold-client/src/features/overview.rs; the footer's
  *X open · Y merged* (*100+* when capped) or *—* with the reason as tooltip in
  micold-client/src/ui/material/overview_card.rs; showcase variant with unknown counts; T026
  passes
- [x] T030 [US1] docs/user-guide/overview.md: pull request counts, the unknown mark and its
  reasons, the merge-conflict reason

**Checkpoint**: US1 complete (scenarios 1–6).

---

## Phase 4: User Story 2 — Open and close the Overview from anywhere (P1)

**Goal**: the chord Ctrl+Shift+O (Cmd+Shift+O) toggles the Overview from anywhere and never reaches
a terminal; the button shows a pressed look and an attention badge.

**Independent Test**: spec US2's: focus a terminal, press the chord twice (Overview opens, then
closes, terminal gets nothing); a permission request puts a red badge on the button.

### Tests for US2 (write first)

- [X] T031 [P] [US2] Tests in micold-client/tests/keymap.rs: Ctrl+Shift+O (Linux / Windows
  modifiers) and Cmd+Shift+O (macOS) encode to `KeyOutput::ToggleOverview`; `O` without Shift,
  with Alt, and plain Ctrl+O are not the chord (Ctrl+O still `0x0f`); `route_key` routes
  `ToggleOverview` to the Overview toggle whether or not the terminal is focused;
  `overview_chord_label()` per platform (FR-011, SC-006)
- [X] T032 [P] [US2] Test in micold-client/src/ui/material/terminal_pane.rs tests, beside the
  `NewTerminalInstance` capture test: the chord is published as the toggle and the event is
  captured (0 bytes to the PTY)
- [X] T033 [P] [US2] Tests in micold-client/src/ui/material/button.rs and unread_mark.rs:
  `Button::selected(true)` draws the selected state layer and keeps its geometry;
  `Button::mark(UnreadMark)` places the configured mark trailing without changing the button's
  height; `UnreadMark::icon(Icon)` draws the icon and the count, tinted by the given role
- [X] T034 [P] [US2] Client tests in micold-client/tests/overview.rs: `overview::badge(&cards)` →
  `Some { tone: Error, count, label: "N need action" }` with any NeedsAction card, else `Warning`
  *N need attention*, else `None`, whether or not the Overview is open (US2.4, FR-012)

### Implementation for US2

- [X] T035 [US2] micold-client/src/keymap.rs: `pub fn is_overview_chord` =
  `is_primary_shift_chord(key, mods, 'o')`, `KeyOutput::ToggleOverview` in tier 1 beside
  `ReleaseFocus` and `NewTerminalInstance`, `overview_chord_label()`; micold-client/src/app.rs
  `KeyRouting` / `route_key`; micold-client/src/ui/material/terminal_pane.rs publishes and
  captures it; T031, T032 pass
- [X] T036 [US2] Unfocused chord: in micold-client/src/ui/mod.rs `subscription`, a
  `keyboard::listen().filter_map(overview_chord)` with a named `fn`, active while the terminal is
  not focused, sending `overview::Msg::Toggled { settings_open }` (R3)
- [X] T037 [US2] `Button::selected`, `Button::mark` in micold-client/src/ui/material/button.rs and
  `UnreadMark::icon` in micold-client/src/ui/material/unread_mark.rs; showcase entries (toggled
  button, tinted mark with icon) in micold-client/src/showcase/ and
  micold-client/tests/showcase_completeness.rs; T033 passes
- [X] T038 [US2] `overview::badge` in micold-client/src/features/overview.rs over `attention`; the
  toolbar button in micold-client/src/ui/toolbar.rs gets `.selected(open)`, tooltip *Overview
  (Ctrl+Shift+O)* / *(Cmd+Shift+O)* from `overview_chord_label()`, and `.mark(..)` tinted `error`
  or `warning` with the state's icon, the count and the label as tooltip line; T034 passes
- [X] T039 [US2] Docs: the shortcut and the badge in docs/user-guide/overview.md; the chord in
  docs/user-guide/terminal-panes.md §Keyboard shortcuts and in
  docs/user-guide/worktrees-and-sessions.md (US2.5, FR-022)

**Checkpoint**: US1 and US2 work.

---

## Phase 5: User Story 3 — Jump into a worktree from its card (P1)

**Goal**: activating a card (pointer, Enter or Space) closes the Overview and opens the location's
main session, offers to start one, or only reveals the row; Tab / arrows move between cards and
Escape closes.

**Independent Test**: spec US3's: click the red card; the Overview closes and that worktree's main
session is shown.

### Tests for US3 (write first)

- [X] T040 [P] [US3] Core tests in micold-core/src/overview.rs: `activation` → `OpenSession(id)`
  when `main_session` is `Some`; `OfferStart(location)` with no un-archived session and not
  orphan / invalid; `RevealOnly(location)` for orphan / invalid; an Idle card's activation is the
  same as any other (US3.5)
- [X] T041 [P] [US3] Client tests in micold-client/tests/overview.rs:
  `CardActivated(OpenSession)` closes and yields `SessionMsg::Selected(id)`; `OfferStart` closes,
  sets sidebar `revealed` and yields the start-menu follow-up, starting no session (FR-013);
  `RevealOnly` closes and only reveals; `revealed` clears on the next sidebar selection;
  `app::on_escape` closes the Overview after the overlay registry and the Settings draft (U5); the
  arrow mapping `overview::arrow_focus(key)` gives Right/Down forward, Left/Up back, other keys
  `None`
- [X] T042 [P] [US3] Test in micold-client/src/ui/focus.rs tests (beside `into_view`'s): the new
  read-only operation reports the focused control's bounds after its scrollable's translation
  (`None` with nothing focused) (decision D11)
- [X] T043 [P] [US3] Tests in micold-client/src/ui/material/overview_card.rs: with `on_press` the
  card is focusable (`TakesTheKeyboard`), draws the focus ring, and Enter or Space publishes the
  message; without it, it is not focusable

### Implementation for US3

- [X] T044 [US3] `Activation` / `activation` in micold-core/src/overview.rs and
  `CardModel::activation`; T040 passes
- [X] T045 [US3] micold-client/src/features/overview.rs `Msg::CardActivated(Activation)`,
  `arrow_focus`; micold-client/src/features/sidebar.rs `revealed: Option<SessionLocation>` set by
  reveal and cleared by the next selection; micold-client/src/app.rs `on_escape` closes the
  Overview; T041 passes
- [X] T046 [US3] Start menu anchor from a card (decision D11): in micold-client/src/ui/focus.rs a
  read-only operation returning the focused control's window bounds; in micold-client/src/main.rs
  the `OfferStart` follow-up reveals the row, focuses its start control by widget id, scrolls it
  into view (`scroll_focused_into_view`), reads its bounds and sends
  `SessionMsg::StartMenuAnchored` (bottom-left of the control) then `SessionMsg::StartMenuOpened
  { location, unavailable_default: None }`; give the row's start control a stable widget id in
  micold-client/src/ui/sidebar.rs; micold-client/tests/context_menu_anchor_call_sites.rs stays
  green; T042 passes
- [X] T047 [US3] `OverviewCard::on_press` (focusable, Enter / Space) in
  micold-client/src/ui/material/overview_card.rs; ui/overview.rs passes
  `overview::Msg::CardActivated(card.activation)`; T043 passes
- [X] T048 [US3] Keys and reveal wiring: the arrow-key listener (named `fn`) in
  micold-client/src/ui/mod.rs `subscription` while the Overview is open and the terminal unfocused,
  mapping to `Message::FocusMoved`; focus the first card on open; micold-client/src/ui/sidebar.rs
  draws `revealed` with `TreeItem`'s selected look and scrolls it into view (R12, R13)
- [X] T049 [US3] docs/user-guide/overview.md: activating a card (the three outcomes), keyboard
  traversal, Escape

**Checkpoint**: all P1 stories work.

---

## Phase 6: User Story 4 — Know what each worktree is working on and how far it got (P2)

**Goal**: the card's summary is the linked issue's title (else branch, else name) and its progress
the phase and task ratio from the worktree's spec-kit files and autopilot ledger, refreshed while
the Overview is open.

**Independent Test**: spec US4's: issue #42's title and *Implement · 6/10*; a plain worktree shows
its branch name and *No task progress*.

### Tests for US4 (write first)

- [ ] T050 [P] [US4] Core tests in micold-core/src/naming.rs: `ticket(dir_name)` for a numeric
  ticket, a Jira key (case kept), and none
- [ ] T051 [P] [US4] Core tests in micold-core/src/task_progress.rs: `checkbox_ratio` (lines
  `^\s*[-*] \[( |x|X)\] `, others ignored, none → `None`, never 0/0; "`total > 0` always");
  `ledger_phase` (spec/clarify/bug → Spec, plan → Plan, tasks → Tasks, milestones/bugfix/chore →
  Implement, close → PR, done → Done, numeric prefix stripped, unknown word → `None`);
  `feature_dir` (`.specify/feature.json` `feature_directory`, else the one `specs/<NNN>-*` matching
  the numeric ticket, several folders, else `None`); `artifact_phase` (ticked tasks.md → Implement,
  tasks.md → Tasks, plan.md → Plan, spec.md → Spec); `phase` precedence ledger → Done → PR →
  artifact; `read` with a missing, a > 1 MiB and a non-UTF-8 file (absent, no panic)
- [ ] T052 [P] [US4] Client tests in micold-client/tests/overview.rs: `ProgressRead { dir,
  progress }` is stored per worktree and a project switch clears `progress` and `titles`; card
  summary: known title → title, `Pending`/`Unavailable`/Jira → branch, Default *Project root*
  (US4.1); progress ratio and phase from `FileProgress` + card state + open pull request (US4.2,
  US4.3); no source → `Progress` with no phase and no ratio (US4.4, FR-017); `TitleRead` moves
  `Pending` → `Known` / `Unavailable`; titles to fetch on open are only numeric tickets with no
  entry, none when issue tooling is not GitHub or unavailable (R8, FR-019)
- [ ] T053 [P] [US4] Geometry tests for `ProgressBar` in micold-client/src/ui/material/progress.rs
  tests: fill width proportional to the ratio (0/10, 6/10, 10/10), same track as `StageProgress`;
  and in overview_card.rs: the progress row shows phase + ratio + bar only with a ratio, else the
  text *No task progress* and no bar

### Implementation for US4

- [ ] T054 [US4] `pub fn ticket(dir_name) -> Option<String>` in micold-core/src/naming.rs over
  `split_name`; T050 passes
- [ ] T055 [US4] micold-core/src/task_progress.rs: `Phase` (+ `label()`), `Ratio { done, total }`,
  `FileProgress`, `Progress { phase, ratio, source }`, `feature_dir`, `checkbox_ratio`,
  `ledger_phase`, `artifact_phase`, `phase`, and `read(root, ticket)` reading at most 1 MiB per
  file; T051 passes
- [ ] T056 [US4] Test first in micold-client/src/shell/overview_watch.rs: its watch targets for a
  root are `<root>/specs` and `<root>/.specify`, recursive (the `Debouncer` is already tested in
  micold-core/src/review/watch.rs). Then generalise `run` in micold-client/src/shell/changes_watch.rs over the read it
  performs (its key and message unchanged), and add micold-client/src/shell/overview_watch.rs: one
  subscription per valid worktree while the Overview is open, watching `<root>/specs` and
  `<root>/.specify` recursively with the 300 ms / 1 s `Debouncer`, reading with
  `task_progress::read` in `spawn_blocking` once on open and on change, emitting
  `overview::Msg::ProgressRead`; ask for it from micold-client/src/shell/subscriptions.rs (R9)
- [ ] T057 [US4] Titles: `TitleLookup` and `Msg::TitleRead` in
  micold-client/src/features/overview.rs, seeded from `IssueList::held`
  (features/worktree_form.rs); on open, micold-client/src/shell/issues.rs reads each needed title
  once through `GhCli::read_issue` in `spawn_blocking` when `issue_tooling` is GitHub and
  available; the read's failure log names the issue number only, never a title (FR-020, checked
  by a test in micold-client/src/shell/issues.rs over the log line it builds)
- [ ] T058 [US4] `CardModel::summary` from titles and `CardModel::progress` from `progress` +
  card state + open pull request in micold-client/src/features/overview.rs; T052 passes
- [ ] T059 [US4] `ProgressBar::new(ratio, roles).width(..)` in a new
  micold-client/src/ui/material/progress_bar.rs sharing the track and fill styling with
  `StageProgress` through one helper in progress.rs; the progress row in overview_card.rs;
  showcase `ProgressBar` (0/10, 6/10, 10/10) and `OverviewCard` with and without progress;
  showcase_completeness; T053 passes
- [ ] T060 [US4] docs/user-guide/overview.md: the summary line's sources, the phases and their
  precedence (ledger wins), the task ratio, the empty state, refresh within 5 s

**Checkpoint**: US4 works; cards stay useful without it.

---

## Phase 7: User Story 5 — See what the agent is doing now (P3)

**Goal**: where hooks report it, the card shows the agent's in-progress todo (else the latest
prompt) and its todo ratio as current activity; otherwise US4's text.

**Independent Test**: spec US5's: a 5-item todo list with 2 done → the in-progress item and *2/5*.

### Tests for US5 (write first)

- [ ] T061 [P] [US5] Core tests in micold-core/src/hook_activity.rs: wire.md W2's table
  (`UserPromptSubmit` first non-empty line; `TodoWrite` replaces todos; other tool or event
  unchanged and returns `false`; malformed items skipped, not an array → unchanged); bounds "≤ 50
  todos, text ≤ 200 chars, prompt = first non-empty line ≤ 200 chars"; `current()` precedence
  (in-progress `activeForm` → its `content` → prompt) and ratio completed / total only with todos
- [ ] T062 [P] [US5] Wire tests: micold-core/tests/protocol_roundtrip.rs `hook_activity`
  round-trips and an absent field decodes to `None`; micold-core/tests/schema_hash.rs re-pinned
  for protocol 49
- [ ] T063 [P] [US5] Daemon tests in micold-daemon/src/state.rs and
  micold-daemon/tests/hooks_receiver.rs: a `TodoWrite` body on `/hook/<uuid>` sets that live
  session's activity and broadcasts the catalog; another session is untouched; the overlay copies
  it to `SessionSummary::hook_activity`; micold-daemon/src/catalog.rs `session_summary` gives
  `None`; the MCP tools (micold-daemon/src/mcp/tools.rs) never return it; a log capture after the
  hooks holds neither the prompt nor a todo text (W2, W3, FR-020)
- [ ] T064 [P] [US5] Client tests in micold-client/tests/overview.rs: catalog adoption sets
  `Session::hook_activity`; the main session's activity becomes `CardModel::activity` beside the
  task-list ratio (US5.2); `None` and after reconnect → US4's summary and progress (US5.3); the
  field is not persisted

### Implementation for US5

- [ ] T065 [US5] micold-core/src/hook_activity.rs: `HookActivity { todos, prompt }`, `Todo { text,
  active, status }`, `TodoStatus`, `apply(&mut self, &serde_json::Value) -> bool`, `current() ->
  Option<CurrentActivity { line, ratio }>`; `Serialize`/`Deserialize`, no `Debug` used in logging;
  T061 passes
- [ ] T066 [US5] `SessionSummary::hook_activity: Option<HookActivity>` (`#[serde(default)]`) in
  micold-core/src/protocol/messages.rs, `PROTOCOL_VERSION` 48 → 49 with a doc line for feature 813
  in micold-core/src/protocol/version.rs, `None` in every `SessionSummary { .. }` literal; T062
  passes
- [ ] T067 [US5] Daemon: micold-daemon/src/hooks.rs `classify_hook` hands the parsed body on with
  `HookClass::Event`; micold-daemon/src/state.rs `LiveSession::hook_activity`,
  `note_hook_activity`, `overlay_live_summaries` copies it, broadcast when `note_activity` or
  `note_hook_activity` reports a change; micold-daemon/src/catalog.rs sets `None`; T063 passes
- [ ] T068 [US5] Client: `Session::hook_activity` runtime only (skipped like `activity` when
  persisting) in micold-core/src/session.rs, adopted in micold-client/src/catalog_sync.rs where
  `paused_on_user` is; `CardModel::activity` in features/overview.rs; the activity line in
  overview_card.rs; showcase card with activity; T064 passes
- [ ] T069 [US5] docs/user-guide/overview.md: hook-fed activity, how to get it (hooks configured),
  what stays on the machine

**Checkpoint**: all stories work.

---

## Phase 8: Polish & Cross-Cutting (close unit; no code)

- [ ] T070 Run quickstart.md §B with the `visual-pass` skill and record the evidence in
  specs/813-worktree-overview/ (B1–B16)
- [ ] T071 Record quickstart.md §C (macOS, Windows) results, or name the CI jobs that cover them

---

## Dependencies & Execution Order

- Setup (T001) → Foundational (T002–T005) → US1 part A (M1). After M1, M2, M3, M4 and M5 depend
  only on M1 and may merge in any order; M6 needs M5. They are cut in priority order, and the
  close unit's Polish comes last.
- US2 needs US1 (the button and `attention`); US3 needs US1 (cards) and is independent of US2;
  US4 and US5 need US1's `CardModel` and `OverviewCard`; US5's fallback is US4's text.
- Within each story: tests first and failing, core before client, `features/` before `ui/`, docs
  in the same milestone.

## Parallel Example: US1 part A

```text
T006 (core tests; T007 and T008 follow it in the same file, not in parallel)
T009 features_agent_confirm.rs · T010 tests/overview.rs · T011 overview_card.rs · T012 card_grid.rs
T013 composition_contrast.rs · T014 main_tests.rs
```

## Implementation Strategy

MVP first: Setup + Foundational + US1 part A is the first thing on `main` someone can open and use.
Each later milestone adds one observable step (counts, the chord and badge, activation, progress,
agent activity). Stop at any checkpoint and the Overview is complete for the stories before it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Overview grid with session-derived card states 🎯 MVP

- **Tasks**: T001–T023
- **Deliverable**: the top-bar Overview button opens a main-pane grid with one card per sidebar
  location showing type, branch, session counts and its card state (from sessions, agent requests
  and the existing pull request status) by colour, icon and word, worst first, updating live;
  clicking the button again or selecting in the sidebar closes it (the button's pressed look,
  badge and shortcut come in M3)
- **Satisfies**: US1 acceptance scenarios 1, 2, 4, 5, 6; FR-001, FR-002, FR-003, FR-005, FR-006,
  FR-007, FR-008 (sessions, requests, PR reading), FR-010 (button), FR-021 (card, grid), FR-023;
  edge cases empty project, no session, several sessions, unknown activity, disconnected
- **Verify**: `cargo test -p micold-core overview`; `cargo test -p micold-client --test overview`;
  `scripts/gate.sh`; quickstart §B B1, B13, B15 (card and grid parts)
- **Depends on**: —
- **Tier**: full

### M2 — Pull request counts and merge conflict on cards

- **Tasks**: T024–T030
- **Deliverable**: each card with a branch shows *X open · Y merged* (or *—* with the reason) from
  the existing status reading, and a conflicting pull request makes its card Needs attention
- **Satisfies**: US1 acceptance scenario 3; FR-004, FR-019 (no new poller); edge cases pull
  request status off / unreadable, no branch
- **Verify**: `cargo test -p micold-core pull_request`; `cargo test -p micold-client --test
  features_pr_status --test overview`; quickstart §B B4, B5
- **Depends on**: M1
- **Tier**: full

### M3 — Overview shortcut and attention badge

- **Tasks**: T031–T039
- **Deliverable**: Ctrl+Shift+O (Cmd+Shift+O) toggles the Overview from anywhere, sending nothing
  to a focused terminal; the button looks pressed while open and carries a red or yellow badge
  with icon and count
- **Satisfies**: US2 acceptance scenarios 1–5; FR-009, FR-010 (shortcut), FR-011, FR-012, FR-022
  (shortcut lists); SC-006
- **Verify**: `cargo test -p micold-client --test keymap --test overview`; quickstart §B B6, B7,
  B11
- **Depends on**: M1
- **Tier**: full

### M4 — Activate a card and move between cards by keyboard

- **Tasks**: T040–T049
- **Deliverable**: clicking a card or pressing Enter / Space on it closes the Overview and opens
  the location's main session, or reveals the row and opens its start menu at the row, or only
  reveals an orphan's row; Tab and arrows move between cards and Escape closes
- **Satisfies**: US3 acceptance scenarios 1–5; FR-013, FR-014; SC-002
- **Verify**: `cargo test -p micold-core overview`; `cargo test -p micold-client --test overview`;
  quickstart §B B8, B9
- **Depends on**: M1
- **Tier**: full

### M5 — Task summary and progress on cards

- **Tasks**: T050–T060
- **Deliverable**: cards show the linked issue's title (else branch) and the phase and task ratio
  with a bar from the worktree's spec-kit files and ledger, or *No task progress*, refreshed within
  5 s of a change on disk
- **Satisfies**: US4 acceptance scenarios 1–5; FR-008 (files), FR-015, FR-016, FR-017, FR-019
  (titles, file reads off the UI thread), FR-020 (titles); edge cases malformed task list, several feature folders,
  issue unreadable
- **Verify**: `cargo test -p micold-core task_progress`; `cargo test -p micold-core naming`; `cargo test -p micold-client
  --test overview`; quickstart §B B2, B3, B4
- **Depends on**: M1
- **Tier**: full

### M6 — Hook-fed agent activity on cards

- **Tasks**: T061–T069
- **Deliverable**: a card whose main session has Claude Code hooks shows the agent's in-progress
  todo (else the latest prompt) and its todo ratio, carried on `SessionSummary` (protocol 49)
- **Satisfies**: US5 acceptance scenarios 1–3; FR-018, FR-020
- **Verify**: `cargo test -p micold-core hook_activity`; `cargo test -p micold-daemon`;
  `cargo test -p micold-client --test overview`; quickstart §B B10
- **Depends on**: M1, M5 (fallback text)
- **Tier**: full

Polish (T070–T071) changes no code: it is left to the close unit.

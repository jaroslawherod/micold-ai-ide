# Tasks: Readable worktree titles without a GitHub issue title

**Input**: Design documents from `/specs/1504-readable-worktree-titles/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/location-title.md,
quickstart.md

**Tests**: Mandatory (Constitution I). In every phase the test tasks come first and must fail
before their implementation tasks. Contract IDs (L1–L9) are in
[contracts/location-title.md](contracts/location-title.md).

**Documentation**: each story's user-guide task sits in that story's phase and milestone
(Constitution VII; CI's user-guide gate).

**Cross-platform**: paths via `Path::join`, git via the existing bounded `GitCli`, ticket parsing
string-only (Constitution VI; plan Constitution Check).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 (spec.md)

---

## Phase 1: Setup

- [ ] T001 Create `crates/micold-core/src/location_title.rs` (module doc citing FR-001 and the
  contract) and register `pub mod location_title;` in `crates/micold-core/src/lib.rs`

---

## Phase 2: Foundational (chooser behind every surface, behaviour unchanged)

**Purpose**: one pure chooser (plan D1, research R1) that the card, the sidebar row and the header
all read, before any new source is fed in.

### Tests

- [ ] T002 [P] Write failing unit tests for `first_line` in `crates/micold-core/src/location_title.rs`
  (L4): BOM stripped, first line only, trailing `\r` stripped, trimmed, empty or whitespace-only →
  `None`
- [ ] T003 Write failing unit tests for `choose` in `crates/micold-core/src/location_title.rs`
  (L1): one test per source in FR-001 order (GitHub issue, Jira summary, pull request, spec,
  session, commit, branch, folder), each winning over every lower one; a source whose text
  normalises to nothing is skipped; `branch` and `display_name` used as they are. Move the five
  `card_title` tests from `crates/micold-core/src/overview_text.rs` (lines ~290–310) here as
  `choose` cases
- [ ] T004 Write a failing unit test in `crates/micold-core/src/location_title.rs` that
  `format!("{:?}", LocationTitle)` does not contain the title text (L9, FR-011)
- [ ] T005 [P] Write failing client tests in `crates/micold-client/tests/features_sidebar.rs` that
  the Overview card title and `features::sidebar::row_title` agree for a worktree titled by its
  GitHub issue, by its branch and by its folder, that a custom name (1125) still wins in the
  row (FR-007), and that the project root (Default location) and an orphan folder keep today's
  title (display name, or branch; spec edge case *Project root*)

### Implementation

- [ ] T006 Implement `TitleTicket` (`Github(u64) | Jira(String)`), `TitleSource`
  (`GithubIssue(u64) | JiraIssue(String) | PullRequest(u64) | Spec | Session | Commit | Branch |
  Folder`), `TitleInputs<'a>` (fields per data-model.md), `LocationTitle { text, source }` with a
  hand-written `Debug` omitting `text`, `first_line` and `choose` in
  `crates/micold-core/src/location_title.rs` (data-model "Core")
- [ ] T007 Remove `overview_text::card_title` from `crates/micold-core/src/overview_text.rs`; add
  `CardModel.title_source: TitleSource` in `crates/micold-core/src/overview.rs` and fix its doc
  link (line ~397)
- [ ] T008 Add `features::overview::location_title(app, worktree, display_name) -> LocationTitle`
  in `crates/micold-client/src/features/overview.rs`, filling `TitleInputs` from today's data only
  (folder GitHub title via `State::known_title`, branch, display name); switch the card in
  `crates/micold-client/src/ui/overview.rs` (~line 754; Default keeps `choose` with only
  `display_name`), `features::sidebar::row_title` in `crates/micold-client/src/features/sidebar.rs`
  (takes `&LocationTitle`, custom name still wins) and its caller in
  `crates/micold-client/src/ui/sidebar.rs` (~line 931) to it; set `CardModel.title_source`

**Checkpoint**: `mise run gate` green; every title reads as before.

---

## Phase 3: User Story 1 — a ticket in the branch or a Jira ticket gives the issue's title (P1) 🎯 MVP

**Goal**: the title ticket comes from the folder, else the branch (FR-002, FR-003); a Jira title
ticket gives its summary (FR-004).

**Independent test**: spec US1 *Independent Test*; quickstart §B rows A–D.

### Tests — GitHub ticket from the branch (US1.1, US1.3)

- [ ] T009 [P] [US1] Write failing table tests for `title_ticket(dir_name, branch)` in
  `crates/micold-core/src/location_title.rs`, one per L2 row (folder wins; `feat/1504-readable-titles`
  and `feat/1504_readable-titles` → `Github(1504)`; Jira keys uppercased; `fix/some_bug`,
  `claude/project-thread-7041o6` and a detached HEAD → none), plus `feat/1504_überblick` →
  `Github(1504)`
- [ ] T010 [P] [US1] Write failing client tests in `crates/micold-client/tests/overview.rs`:
  `titles_to_fetch` asks 1504 for folder `project-thread-t2a600` + branch
  `feat/1504-readable-titles`; asks 813 (not 900) for folder `feat-813_x` + branch `feat/900_y`;
  asks nothing without GitHub (extend the test at ~line 1340); `State::issue_title` returns the
  branch ticket's known title (US1.1, US1.3, FR-003)
- [ ] T011 [P] [US1] Write a failing test `worktrees_fingerprint_follows_the_branch` in
  `crates/micold-client/src/main_tests.rs` that `shell::issues::worktrees_fingerprint` changes when only a worktree's branch changes (plan D2)
- [ ] T012 [P] [US1] Write a failing test in `crates/micold-client/tests/features_sidebar.rs` that
  the card and the row of a worktree whose ticket is only in its branch both show that issue's
  title (US1.1, SC-002, FR-007)

### Implementation — GitHub ticket from the branch

- [ ] T013 [US1] Implement `title_ticket` in `crates/micold-core/src/location_title.rs` per
  research R2 (folder via `naming`, else branch via `short_branch`'s rule)
- [ ] T014 [US1] In `crates/micold-client/src/features/overview.rs`: replace `known_title` with
  `State::issue_title(&TitleTicket, &StatusBoard)` (GitHub arm only here), make `titles_to_fetch`
  take title tickets, and switch the two `numeric_ticket` callers here to `title_ticket`; feed
  the title ticket into `location_title` (`numeric_ticket` stays until T015)
- [ ] T015 [US1] In `crates/micold-client/src/shell/issues.rs`: `request_titles` takes the
  worktrees' GitHub title tickets from `(dir_name, branch)` (~line 785), and
  `worktrees_fingerprint` (~line 756) hashes `(dir_name, branch)`; then remove
  `features::overview::numeric_ticket`, which has no caller left
- [ ] T016 [US1] Update the *Title* section of `docs/user-guide/overview.md`: a ticket in the branch
  name gives the issue's title when the folder name has none; the folder's ticket wins

### Tests — Jira summary (US1.2, US1.4)

- [ ] T017 [P] [US1] Write failing core tests in `crates/micold-core/src/jira_status.rs` (and
  `crates/micold-core/tests/jira_parse.rs` for the parser): `parse_statuses` reads
  `fields.summary` into `KeyedIssue { key, status, summary }`; `StatusBoard::begin(status_keys,
  summary_keys)` asks the union; on `Ok` statuses are stored for status keys only and summaries for
  every asked key; on `Err` summaries are kept like statuses until retired, then cleared; `clear()`
  drops both; `summary_of`; `Debug` of the board and of `KeyedIssue` omit summaries (L6, L9)
- [ ] T018 [P] [US1] Write failing client tests: in `crates/micold-client/tests/features_jira_status.rs`
  `State::plan` asks a branch-only Jira title ticket (folder without ticket, valid key of the
  connection's project) as a summary-only key, skips a key of another project, and asks it not
  at all (nothing sent to Jira) with the Jira switch off or no connection (FR-004, L6); in
  `crates/micold-client/tests/jira_status_sidebar.rs` that worktree shows no status chip and is
  not in `linked_keys` (FR-002, FR-004)
- [ ] T019 [P] [US1] Write a failing assertion in `crates/micold-client/tests/jira_http_adapter.rs`
  that the request body's `fields` contains `summary` beside `status` (L6)
- [ ] T020 [P] [US1] Write failing tests in `crates/micold-client/tests/features_sidebar.rs`: a
  folder Jira key with a summary on the board titles card and row with the summary (US1.2); with no
  summary (no connection, unknown key, failed read) the title falls through to the next source
  (US1.4)

### Implementation — Jira summary

- [ ] T021 [US1] In `crates/micold-core/src/jira_status.rs`: `KeyedIssue` replaces `KeyedStatus`;
  `parse_statuses` reads `summary`; `StatusBoard` gains `summaries`, `status_keys`,
  `begin(status_keys, summary_keys)`, `summary_of`, hand-written `Debug` (data-model "jira_status");
  follow through `crates/micold-core/src/jira.rs` (`JiraSource::statuses` ~line 636/963, the fake's
  `with_statuses`/`script_statuses` ~lines 799–843)
- [ ] T022 [US1] Ask `["status", "summary"]` in `read_chunk` of
  `crates/micold-client/src/shell/jira_http.rs`
- [ ] T023 [US1] Add `branch_title_keys(worktrees, project)` and pass it to `begin` from
  `State::plan` in `crates/micold-client/src/features/jira_status.rs`; give
  `State::issue_title` its Jira arm (`StatusBoard::summary_of`) in
  `crates/micold-client/src/features/overview.rs`
- [ ] T024 [US1] Update the *Title* section of `docs/user-guide/overview.md`: a Jira ticket's
  summary is the title, read with the status request under the same switch; a key only in the
  branch gets no status chip

**Checkpoint**: quickstart §B rows A–D.

---

## Phase 4: User Story 2 — a worktree with no readable issue is titled from its own work (P1)

**Goal**: pull request, spec, session and commit sources (FR-001 items 3–6), the source named in
each tooltip (FR-008), titles that follow their sources (FR-010).

**Independent test**: spec US2 *Independent Test*; quickstart §B rows E–I and *Refresh*.

### Tests — held sources and tooltips (US2.1, US2.3, US2.5–2.7)

- [ ] T025 [P] [US2] Write a failing table test for `TitleSource::words` in
  `crates/micold-core/src/location_title.rs`, one row per L3 line (`None` for `Folder`)
- [ ] T026 [P] [US2] Write failing client tests in `crates/micold-client/tests/features_overview.rs`
  for `location_title`: an open or draft pull request of the branch titles the card, a merged or
  closed one does not, pull request status off gives none (R9, US2.1); the main session's
  `Named`/`Derived` label titles it, a pending or placeholder label or a non-main session does not
  (R8, US2.3); nothing gives branch, then folder (US2.5); after a PR-opened reading or a session
  archive the derived title changes with no other message (FR-010, US2.7)
- [ ] T027 [P] [US2] Write failing tooltip tests: `features::sidebar::with_title_line` puts the
  words line after the title and keeps today's tooltip for `Folder`
  (`crates/micold-client/tests/features_sidebar.rs`); `HeaderText::of` title tooltip has the words
  line (`crates/micold-core/src/context_header.rs` tests); `ExpandedText::of` has `source`
  (`crates/micold-core/src/overview_text.rs` tests) (FR-008, US2.6)

### Implementation — held sources and tooltips

- [ ] T028 [US2] Implement `TitleSource::words` in `crates/micold-core/src/location_title.rs`
- [ ] T029 [US2] Feed the pull request (open/draft `PullRequestStatus` of the branch, R9) and main
  session (`Named`/`Derived` label, R8) inputs into `location_title` in
  `crates/micold-client/src/features/overview.rs` (no new reading, L8)
- [ ] T030 [US2] Add the source line: `with_title_line(node, &LocationTitle, tooltip)` in
  `crates/micold-client/src/features/sidebar.rs`; `HeaderText::of` in
  `crates/micold-core/src/context_header.rs`; `ExpandedText.source` in
  `crates/micold-core/src/overview_text.rs`, drawn beside the display-name line in
  `crates/micold-client/src/ui/material/overview_card.rs` (research R6)
- [ ] T031 [US2] Pose an Overview card per source in the showcase
  (`crates/micold-client/src/showcase/samples.rs`, `sections/surfaces.rs`); regenerate
  `crates/micold-client/tests/layout_snapshot.rs` only if the expanded card's height changes
- [ ] T032 [US2] Update `docs/user-guide/overview.md` *Title* (source order; PR and session
  sources; where the source is shown) and `docs/user-guide/worktrees-and-sessions.md` (row first
  line and tooltip)

### Tests — spec and commit facts (US2.2, US2.4)

- [ ] T033 [P] [US2] Write failing core tests in `crates/micold-core/src/location_title.rs`:
  `spec_title` strips `Feature Specification:`, gives none for `[FEATURE NAME]`/`[...]`-only and no
  heading (L4, FR-012); `read_spec_title` on a temp dir finds the folder of `.specify/feature.json`,
  else `specs/<n>-*` for ticket `n`, handles CRLF and BOM, gives none for a missing file, and
  reads a worktree whose path has non-ASCII characters (`wt-überblick`)
- [ ] T034 [P] [US2] Write failing core tests in `crates/micold-core/tests/review_git.rs` for
  `GitCli::own_commit_subject` on a temp repo: own commit → its subject; fresh branch → none;
  detached HEAD → none; no default branch → none; a dir that is not a git repository → none; and in `crates/micold-core/tests/git_fake.rs`
  for `FakeGit::with_own_commit` (L7)
- [ ] T035 [P] [US2] Write failing reducer tests in
  `crates/micold-client/tests/features_overview.rs`: `TitleFactsRequested` bumps `facts_round` and
  records it per dir; `TitleFactsRead` is dropped for an older round, an unlisted dir or another
  project, stored otherwise; `ProjectSwitched` clears facts; `Debug` of `TitleFacts` omits texts;
  stored facts title the card (spec over session, commit under session) (L7, L9, US2.2, US2.4)
- [ ] T036 [P] [US2] Write failing tests named `title_facts_*` in
  `crates/micold-client/src/main_tests.rs`: through `update`, a `set_worktrees` listing, a created
  worktree (`features::worktree::created`), an included one and an Overview opening each leave
  their dirs in `facts_asked`; with capabilities `without_local_git` (worktrees not on this
  machine) none does (FR-006, L7)

### Implementation — spec and commit facts

- [ ] T037 [US2] Implement `spec_title` and `read_spec_title` in
  `crates/micold-core/src/location_title.rs`; make `task_progress::read_text` `pub(crate)` in
  `crates/micold-core/src/task_progress.rs`
- [ ] T038 [US2] Add `Git::own_commit_subject` in `crates/micold-core/src/git.rs` (with
  `FakeGit::with_own_commit`) and its `GitCli` impl in `crates/micold-core/src/review/git.rs`
  (`review_base` merge-base, `log -1 --format=%s base..HEAD` through the bounded `read`)
- [ ] T039 [US2] In `crates/micold-client/src/features/overview.rs`: `TitleFacts` (texts-free
  `Debug`), `State.facts`/`facts_round`/`facts_asked`, `Msg::TitleFactsRequested` and
  `Msg::TitleFactsRead` reducers (prune unlisted dirs), `read_project` arm; feed spec and commit
  into `location_title`
- [ ] T040 [US2] New `crates/micold-client/src/shell/location_titles.rs` (registered in the shell
  module): `read_facts(app, &dirs)` applies `TitleFactsRequested`, then per valid worktree one
  `Task::perform` + `spawn_blocking` with `shell/changes.rs`'s `READ_DEADLINE` reading the spec
  heading and the own-commit subject independently (a failing or missing `git` loses only the
  commit); nothing at all (no `TitleFactsRequested`, no read) without local git
  (`caps.shared_git()` none: the worktrees are the daemon's, not this machine's); call it from `crates/micold-client/src/main.rs` beside the fingerprint check
  (~line 490, also on an active-project change) and from `issues::on_overview_opened`
  (`crates/micold-client/src/shell/issues.rs` ~line 737); log no text (L9)
- [ ] T041 [US2] Update `docs/user-guide/overview.md` *Title*: spec and latest-commit sources, read
  when the worktree list changes or the Overview opens

**Checkpoint**: quickstart §B rows E–I, *Refresh*.

---

## Phase 5: User Story 3 — a failed issue read is tried again (P2)

**Goal**: FR-009 retry on existing occasions after 5 minutes; `NoIssue` never re-asked.

**Independent test**: spec US3 *Independent Test*; quickstart §B *Retry*.

### Tests

- [ ] T042 [P] [US3] Write failing tests in `crates/micold-client/tests/overview.rs`:
  `titles_to_fetch(..., now)` re-asks `Failed { at }` only when `now - at >= 300`, never
  `NoIssue`, `Known` or `Pending` (US3.1–3.3, L5); while `Failed` or `Pending` the card shows the
  next source's title, never a loading text
- [ ] T043 [P] [US3] Write failing tests named `title_answer_*` in
  `crates/micold-client/src/main_tests.rs`: `on_title_remotes(None)` → `Failed`, no GitHub remote
  → `NoIssue`, `read_title` `Ok(None)` → `NoIssue`, `Err(Load)` → `Failed { at }` stamped with
  `micold_core::unix_time::unix_secs()` (Unix seconds, research R3)

### Implementation

- [ ] T044 [US3] In `crates/micold-client/src/features/overview.rs`: `TitleLookup` gains `NoIssue`
  and `Failed { at: u64 }` (replacing `Unavailable`); `titles_to_fetch` takes `now` and adds due
  retries
- [ ] T045 [US3] In `crates/micold-client/src/shell/issues.rs`: map `on_title_remotes` (~line 882)
  and `read_title` (~line 980) answers per research R3, stamping `Failed { at }` with
  `micold_core::unix_time::unix_secs()`; pass that `now` to `titles_to_fetch`
- [ ] T046 [US3] Update `docs/user-guide/overview.md` *Title*: a failed read is asked again after
  5 minutes, on the next Overview opening or list refresh

**Checkpoint**: quickstart §B *Retry*.

---

## Phase 6: Polish & cross-cutting

- [ ] T047 Confirm `crates/micold-client/tests/issues_are_requested_only_on_named_events.rs` and
  `crates/micold-client/tests/jira_is_requested_only_on_named_events.rs` pass with caller counts
  unchanged (L8, SC-004)
- [ ] T048 (ships in M1) Add the *Amended by 1504* notes to `specs/813-worktree-overview/spec.md` FR-015 and
  `specs/1120-overview-card-readability/spec.md` US2 scenarios 1–2 (FR-013, plan D8)
- [ ] T049 Run quickstart.md Part A and Part B (visual pass) and record the result in
  `specs/1504-readable-worktree-titles/quickstart.md`

---

## Dependencies & execution order

- Setup → Foundational → US1 (GitHub branch ticket, then Jira) → US2 (held sources and tooltips,
  then facts) → US3 → Polish.
- US2 and US3 depend only on Foundational and T013–T015 (the title ticket); US3 does not depend on
  US2.
- Within each block, tests first (red), then implementation, then the user-guide task.
- T014 before T015 (T015 removes `numeric_ticket` once both moved its callers); T021 before T023
  (board API); T037 and T038 before T040; T039 before T040.

## Parallel examples

- Foundational: T002 then T003 then T004 (one file), alongside T005.
- US1: T009–T012 together; T017–T020 together.
- US2: T025–T027 together; T033–T036 together.
- US3: T042, T043 together.

## Implementation Strategy

MVP is M1: the chooser plus branch tickets, so worktrees like this one (`project-thread-t2a600` on
a ticketed branch) get their issue title. Each later milestone adds sources or recovery and ships on
its own; every milestone leaves all titles readable and `main` green.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Chooser and branch tickets 🎯 MVP

- **Tasks**: T001–T016, T048 (the 813/1120 *Amended by 1504* notes ship with the first change to
  the title order, so those specs never disagree with `main`)
- **Deliverable**: a worktree whose GitHub ticket is only in its branch name shows that issue's
  title on its Overview card and sidebar row, through one chooser that every surface reads.
- **Satisfies**: US1 acceptance scenarios 1 and 3; FR-001 (chooser), FR-002 (GitHub), FR-003,
  FR-007, FR-011 (`LocationTitle`), FR-012 (`first_line`); SC-002
- **Verify**: `mise run test-core` (or `cargo test -p micold-core --lib location_title`);
  `cargo test -p micold-client --test overview --test features_sidebar`;
  `cargo test -p micold-client --bin micold-ai-ide worktrees_fingerprint`; quickstart §B rows A–B,
  title text only: card and sidebar row show the issue's title (source words arrive in M3)
- **Depends on**: —
- **Tier**: full

### M2 — Jira summaries

- **Tasks**: T017–T024
- **Deliverable**: a worktree with a Jira title ticket (folder or branch) shows the Jira issue's
  summary as its title; a branch-only key gets no status chip.
- **Satisfies**: US1 acceptance scenarios 2 and 4; FR-002 (Jira), FR-004, FR-011 (board)
- **Verify**: `mise run test-core` (or `cargo test -p micold-core --lib jira_status` and
  `cargo test -p micold-core --test jira_parse`); `cargo test -p micold-client --test
  features_jira_status --test jira_status_sidebar --test jira_http_adapter --test
  features_sidebar`; quickstart §B rows C–D, title text only: card and row show the Jira summary,
  C has its status chip, D has none (source words arrive in M3)
- **Depends on**: M1
- **Tier**: full

### M3 — Pull request and session titles, source in the tooltip

- **Tasks**: T025–T032
- **Deliverable**: a worktree with no readable issue is titled by its open pull request or main
  session, and every title's tooltip (row, header, expanded card) names its source.
- **Satisfies**: US2 acceptance scenarios 1, 3, 5, 6, 7; FR-001 items 3 and 5, FR-005, FR-008,
  FR-010
- **Verify**: `cargo test -p micold-client --test features_overview --test features_sidebar`;
  `mise run test-core` (or, one filter per command, `cargo test -p micold-core --lib
  location_title`, `cargo test -p micold-core --lib context_header`, `cargo test -p micold-core
  --lib overview_text`); quickstart §B rows A–E, G, I with their source words, and the *Refresh*
  steps that exist in M3: close E's pull request (E falls to its branch name) and archive G's
  session (G falls to the next source)
- **Depends on**: M1
- **Tier**: full

### M4 — Spec and commit titles

- **Tasks**: T033–T041
- **Deliverable**: a worktree with no issue, PR or session title is titled by its feature spec
  heading or its own latest commit, read off the UI thread when the worktree list changes or the
  Overview opens.
- **Satisfies**: US2 acceptance scenarios 2 and 4; FR-001 items 4 and 6, FR-006, FR-011 (facts),
  FR-012 (spec); SC-001, SC-005
- **Verify**: `mise run test-core` (or `cargo test -p micold-core --lib location_title` and
  `cargo test -p micold-core --test review_git --test git_fake`); `cargo test -p micold-client
  --test features_overview`; `cargo test -p micold-client --bin micold-ai-ide title_facts`;
  quickstart §B rows F, H and the *Refresh* steps open-a-PR-for-H and late commit
- **Depends on**: M3 (tooltip words and `ExpandedText.source` show the new sources)
- **Tier**: full

### M5 — Retry failed issue reads

- **Tasks**: T042–T046
- **Deliverable**: a GitHub title read that failed is asked again on the next title-asking occasion
  at least 5 minutes later, so the card recovers without a restart; a "no such issue" answer is
  never re-asked.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-009; SC-003
- **Verify**: `cargo test -p micold-client --test overview`; `cargo test -p micold-client --bin
  micold-ai-ide title_answer`; quickstart §B *Retry*
- **Depends on**: M1
- **Tier**: light

Polish T047 and T049 change no code: the close unit does them in the close PR. T048 ships in M1.

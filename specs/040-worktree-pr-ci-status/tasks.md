---
description: "Task list for feature 040 — pull request and check status for each worktree"
---

# Tasks: Pull Request and Check Status for Each Worktree

**Input**: Design documents from `/specs/040-worktree-pr-ci-status/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing before their implementation. The test list is
[tdd/test-list.md](./tdd/test-list.md); test tasks, and the implementation tasks that make them
pass, carry its behavior ids (`[A#]`, `[U#]`), and `/speckit.tdd.run` ticks a task from them. Every
phase lists its failing tests first. The GUI exception is claimed only for `src/ui/` composition and
the shell's task glue, verified by the recorded quickstart §B pass.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own user-guide
task in the milestone that ships it (CI's user-guide gate).

**Cross-platform**: Per Constitution Principle VI, the feature adds no `cfg` arm. `gh` is located
and run by 034's code and the browser is opened by the existing `LinkOpener`; every new rule is in
`micold-core` or a platform-neutral reducer, whose suites CI runs on Linux, macOS and Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US4)
- Every description carries an exact file path

## Path Conventions

Three-crate workspace: `crates/micold-core/` (render-free rules), `crates/micold-daemon/` (one new
read-only RPC arm, one settings field), `crates/micold-client/` (reducer, shell, UI). Build and test
through `mise run <task>` (CLAUDE.md). Shell event tests live in
`crates/micold-client/src/main_tests.rs` (the binary's `tests` module); name each new shell test
with `pr_status` so a milestone's Verify filter selects it. Contract references: **PS** =
[contracts/pull-request-source.md](./contracts/pull-request-source.md), **RW** =
[contracts/reading-and-wire.md](./contracts/reading-and-wire.md), **UI** =
[contracts/pull-request-ui.md](./contracts/pull-request-ui.md), **DM** =
[data-model.md](./data-model.md).

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: no new crate, dependency, binary or configuration (plan, Technical Context).

---

## Phase 2: Foundational (Blocking Prerequisites)

No foundational phase of its own. The one prerequisite every story shares — the wire change
(protocol 21) with the setting and the `MergedBranchCheck` RPC — is User Story 1's slice B below,
because the switch is part of story 1 and the repository takes one protocol bump per feature
(research R11).

---

## Phase 3: User Story 1, slice A — the pull request source (Priority: P1)

**Goal**: Everything story 1 decides about GitHub's answer, render-free and tested against recorded
`gh` output: the query, the arguments, parsing, which pull request is the branch's, the check
reduction, the failure kinds and the rate-limit pause, and the source trait with its `gh`
implementation and fake. Nothing calls it yet (milestones.md rule 6); slice C does.

**Independent Test**: `mise run test-core` passes with the new `pull_request_*` suites; the
recorded fixtures of three branches yield open + failing, merged, and no entry.

### Tests for User Story 1, slice A (MANDATORY — Constitution Principle I) ⚠️

- [ ] T001 [US1] [U1] Record the fixtures of PS §5 with a real `gh api graphql --include` into `crates/micold-core/tests/fixtures/gh/pr_*.txt` (the 14 files of the table, `pr_rate_limited_secondary_no_retry_after.txt` included) and write `crates/micold-core/tests/fixtures/gh/pr_README.md`: the recording command first, then per file how it was produced and which ones are written from GitHub's documented answer. Strip tokens and request ids; keep status line, headers and body.
- [ ] T002 [P] [US1] [U2] [U3] [U4] [U5] [U6] Write `crates/micold-core/tests/pull_request_query.rs` (PS §2):
  - `status_query(1)` and `status_query(50)` are pinned; the document holds `o<i>`/`r<i>` for every `i`, `rateLimit`, the fragment, and is one line.
  - No branch name appears in the document.
  - `status_args` for branches holding `"`, `$`, a space, a leading `-`, `true` and `123` yields one `-f b<i>=<branch>` pair each, `--hostname github.com`, `--include`, `owner`, `name`, and nothing else: no `-F`, no token, no header argument (FR-028, FR-031).
- [ ] T003 [US1] [U1] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] Write `crates/micold-core/tests/pull_request_parse.rs` against T001's fixtures (PS §5):
  - `split_response`: `\r\n` and `\n` line ends, header names without case, `None` without a status line or an empty line.
  - `parse_status` on `pr_three_branches.txt`: open, merged, and no entry for the third branch; keys are the branches passed in.
  - Each of `pr_draft`, `pr_closed`, `pr_review_states`, `pr_no_checks`, `pr_checks_*`: number, title, url, state, checks, review, head as recorded (SC-002).
  - `pr_truncated.txt`, a missing alias, a `null` repository and an `errors` entry are failures, never a partial map.
- [ ] T004 [P] [US1] [U16] [U17] [U18] [U19] [U20] [U21] [U22] Write `crates/micold-core/tests/pull_request_select.rs` (PS §3), one case each: open beats a newer merged; newest of two open; newest of merged and closed when none open; a cross-repository open node ignored and the same-repository closed one shown; only cross-repository nodes → none; a draft; an unknown `state` → unreadable.
- [ ] T005 [P] [US1] [U23] [U24] [U25] [U26] [U27] [U28] [U29] [U30] Write `crates/micold-core/tests/pull_request_checks.rs` (PS §4): a table-driven case per state name of both lists alone; failing beside pending and passing; pending beside passing; only skipped and neutral → passing; an unknown name alone → pending; `null` rollup and all-zero counts → none.
- [ ] T006 [US1] [U31] [U32] [U33] [U34] [U35] [U36] [U37] [U38] [U39] [U40] [U41] Write `crates/micold-core/tests/pull_request_failure.rs` (PS §5 table and `rate_limit_pause`), against T001's fixtures and 034's stderr fixtures:
  - Rows 1 to 6 of the `reading_failure` table, one case each, rate limiting checked before access.
  - `rate_limit_pause`: `Retry-After: 30` → `now + 30`; remaining 0 → the reset time; secondary limit with remaining above 0 and no `Retry-After` → `now + 60`; a reset in the past → `now + 1`; an unparsable value skipped.
- [ ] T007 [P] [US1] [U42] [U43] [U44] [U45] Write `crates/micold-core/tests/pull_request_source.rs` (PS §1): `FakePullRequestSource` scripts answers and records `(owner/name, branches)` per call; an empty branch list answers an empty map with no call; 51 and 120 branches are read in chunks of 50 through a chunk-level seam, and a failing second chunk fails the whole reading with nothing of the first returned.
- [ ] T008 [P] [US1] [U46] [U47] Write `crates/micold-core/tests/pull_request_is_never_stored.rs` (DM §1, FR-032, SC-011): a source gate that `crates/micold-core/src/pull_request.rs` derives neither `Serialize`, `Deserialize` nor `Debug` on `PullRequestStatus`, and a unit case that its hand-written `Debug` output holds the number and the enums and neither the title nor the address.

### Implementation for User Story 1, slice A

- [ ] T009 [US1] [U46] [U47] Create `crates/micold-core/src/pull_request.rs` and export it from `crates/micold-core/src/lib.rs`: `PullRequestStatus`, `PrState`, `CheckStatus`, `ReviewState`, `ReadingFailure` (DM §1–2), with the hand-written `Debug`. T008 passes.
- [ ] T010 [US1] [U2] [U3] [U4] [U5] [U6] Add `status_query` and `status_args` to `crates/micold-core/src/pull_request.rs` (PS §2). T002 passes.
- [ ] T011 [US1] [U7] [U8] [U9] [U10] [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] [U19] [U20] [U21] [U22] [U23] [U24] [U25] [U26] [U27] [U28] [U29] [U30] Add `split_response`, `parse_status`, `select_pull_request` and `reduce_checks` to `crates/micold-core/src/pull_request.rs` (PS §3–5). T003, T004 and T005 pass.
- [ ] T012 [US1] [U31] [U32] [U33] [U34] [U35] [U36] [U37] [U38] [U39] [U40] [U41] Add `rate_limit_pause` and `reading_failure` to `crates/micold-core/src/pull_request.rs`, on top of `github::classify` (PS §5). T006 passes.
- [ ] T013 [US1] [U42] [U43] [U44] [U45] Add the `PullRequestSource` trait, `FakePullRequestSource` and the chunking to `crates/micold-core/src/pull_request.rs`, and `impl PullRequestSource for GhCli` in `crates/micold-core/src/github.rs` through the existing bounded runner (PS §1). T007 passes; `mise run test-core` is green.

**Checkpoint**: the core can turn `gh`'s answer into per-branch statuses and name every failure.

---

## Phase 4: User Story 1, slice B — the switch on the wire and the merged-branch question (Priority: P1)

**Goal**: The feature's one wire change (protocol 20 → 21): `pr_status_enabled` in
`settings.json`, on `DaemonSettings` and settable through `SettingsSet`; and the read-only
`MergedBranchCheck` RPC with its daemon arm. No UI sets or shows either yet; slices C and D and
story 3 do.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-daemon --test merged_branch_check --test pr_status_setting` pass; `mise run test-core` passes with the re-pinned schema.

### Tests for User Story 1, slice B (MANDATORY — Constitution Principle I) ⚠️

- [ ] T014 [P] [US1] [U48] [U49] Extend `crates/micold-core/tests/settings_roundtrip.rs` (DM §5): `pr_status_enabled` is `false` by default and when read from a `settings.json` written without it; `true` survives a round trip; `SETTINGS_VERSION` is unchanged.
- [ ] T015 [P] [US1] [U50] [U51] [U52] [U53] [U54] [U55] Write `crates/micold-core/tests/git_containment.rs` (DM §6, RW §3): `containment(tip, head, ancestor)` for equal tip, ancestor, not an ancestor, no tip, unknown ancestry; `RealGit::branch_tip` and `RealGit::is_ancestor` against a temporary repository (branch at, behind and ahead of a commit; a missing branch; a commit id not in the repository); `FakeGit` scripts both.
- [ ] T016 [US1] [U56] [U57] [U58] Extend `crates/micold-core/tests/protocol_roundtrip.rs` with `ClientMsg::MergedBranchCheck`, `OperationResult::MergedBranchCheck`, `SettingsSet { pr_status_enabled }` and `DaemonSettings.pr_status_enabled` (RW §3–5); `crates/micold-core/tests/schema_hash.rs` fails until re-pinned in T020.
- [ ] T017 [P] [US1] [U59] [U60] [U61] [U62] [U63] [U64] [U65] [U66] Write `crates/micold-daemon/tests/merged_branch_check.rs` (RW §3) against a temporary repository: tip equal to `head` → `Contained`; branch behind `head` → `Contained`; a commit after `head` → `Beyond`; missing branch → `Unknown`; `head` not a local object → `Unknown`; a `head` that is not 40 or 64 hexadecimal characters → `Unknown` without running git; answers in query order; 51 queries refused; a non-repository refused as `RemoteList` refuses it; no ref or file of the repository changed afterwards.
- [ ] T018 [P] [US1] [U67] [U68] [U69] Write `crates/micold-daemon/tests/pr_status_setting.rs` (RW §4): `SettingsSet { pr_status_enabled: Some(true) }` is persisted, reported in the next `Welcome`, and broadcast as `SettingsChanged` to two connected clients; `None` leaves it as it is.

### Implementation for User Story 1, slice B

- [ ] T019 [US1] [U48] [U49] [U50] [U51] [U52] [U53] [U54] [U55] Add `pr_status_enabled` to `Settings` in `crates/micold-core/src/settings.rs` (and its on-disk form) and `containment`, `Git::branch_tip`, `Git::is_ancestor` (`RealGit`, `FakeGit`) to `crates/micold-core/src/git.rs`. T014 and T015 pass.
- [ ] T020 [US1] [U56] [U57] [U58] Add `MergedBranchQuery`, `BranchContainment`, `ClientMsg::MergedBranchCheck`, `OperationResult::MergedBranchCheck` and `pr_status_enabled` on `DaemonSettings` and `SettingsSet` to `crates/micold-core/src/protocol/messages.rs`; set `PROTOCOL_VERSION` to the next free number (21 unless `main` took it) in `crates/micold-core/src/protocol/version.rs`; re-pin `crates/micold-core/tests/schema_hash.rs`. T016 passes.
- [ ] T021 [US1] [U59] [U60] [U61] [U62] [U63] [U64] [U65] [U66] [U67] [U68] [U69] Add the `MergedBranchCheck` arm (a copy of `RemoteList`'s) and the `SettingsSet` field to `crates/micold-daemon/src/server.rs`, and `pr_status_enabled` to `crates/micold-daemon/src/catalog.rs` and `crates/micold-daemon/src/state.rs` as `tool_server_enabled` is held. T017 and T018 pass.
- [ ] T022 [US1] Keep the client building on the new wire: pass `pr_status_enabled: None` at every `SettingsSet` in `crates/micold-client/src/shell/persist.rs` and accept the new `DaemonSettings` field in `crates/micold-client/src/shell/daemon_sync.rs` (stored nowhere yet). `mise run gate` is green.

**Checkpoint**: client and daemon speak protocol 21; the daemon stores the switch and answers the merged-branch question.

---

## Phase 5: User Story 1, slice C — the reading in the client (Priority: P1)

**Goal**: The window that holds a project reads its pull request status once when the project's
listing arrives and once when the switch turns on, off the update loop, and keeps the result in
its own state. Nothing draws it yet (slice D), and the Settings control does not exist yet, so on
`main` the reading runs only for a `settings.json` that already holds `pr_status_enabled: true`.

**Independent Test**: `scripts/build-lock.sh cargo test -p micold-client pr_status` passes: with a fake source, `Attached` then `CatalogChanged` produces one reading whose statuses land in `state.pr_status`; with the switch off nothing is asked.

### Tests for User Story 1, slice C (MANDATORY — Constitution Principle I) ⚠️

- [ ] T023 [P] [US1] [U70] [U71] [U72] [U73] [U74] [U75] [U76] [U77] [U78] [U79] [U80] [U81] [U82] [U83] [U84] Write `crates/micold-client/tests/features_pr_status.rs` (RW §2, DM §3) for the reducer:
  - `Held` then `ListingArrived` → one `Read { seq }`; a second `ListingArrived` → none.
  - `EnabledChanged { true }`: reads when held and listed; starts nothing when not held or while awaiting the listing; the same value twice starts nothing.
  - `EnabledChanged { false }` clears statuses, removable, `read_at`, `pause_until` and goes `Idle` (invariant 3).
  - `Finished`: `Ok` replaces statuses and sets `read_at` to the reading's start; `Unavailable` clears; `Passing` changes nothing; `RateLimited { until }` keeps statuses and sets `pause_until`; an answer with another `seq` is dropped.
  - `Released` clears statuses, sets `held = false` and keeps `pause_until`; afterwards only `Held` + `ListingArrived` reads, exactly once (SC-007); `ListingArrived` while paused starts nothing.
  - No sequence of these messages yields two `Read` effects without a `Finished` or `Released` between them.
- [ ] T024 [P] [US1] [U85] [U86] [U87] [U88] Write `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` (RW §1), after `issues_are_requested_only_on_named_events.rs`: `shell::pr_status::start` is called only from the lines handling S1 and S2 (S3 to S5 are added by US4, which raises the count); `Msg::ListingArrived` is sent from one line, inside the `CatalogChanged` arm; `PullRequestSource::read` is called from one place; no line of `crates/micold-client/src` logs a pull request's title or address.
- [ ] T025 [US1] [A10] [A11] [A12] [A14] [A31] [A40] [A41] [A42] [U89] [U90] [U91] [U92] [U93] [U94] [U95] [U96] [U97] Add shell tests named `pr_status_*` to `crates/micold-client/src/main_tests.rs` (RW §1–2), with `FakePullRequestSource`:
  - `DaemonMsg::Attached` for the active project, then `CatalogChanged`: `RemoteList` is sent, then the source is read with the listing's branches, deduplicated, in listing order, detached worktrees left out; the answer lands in `state.pr_status.statuses`.
  - A later `CatalogChanged` starts nothing.
  - No GitHub remote → the source is never called and statuses are cleared; `gh` not found → the same (FR-025, FR-026).
  - `RemoteList` unanswered for 10 s → statuses kept (`Passing`).
  - `Displaced` and `Refused { ProjectBusy }` for the active project, a project switch and a disconnect clear the statuses and start nothing; a take-over (`Attached` + `CatalogChanged`) reads once.
  - `SettingsChanged` turning the switch on while held reads once; turning it off clears.
  - A finished reading leaves selection, expansion, scroll, filter and sessions of the application state equal to before (FR-020), and no notice, dialog or error line exists after any failure kind (SC-004).
  - A reading never sends `ClientMsg::WorktreeRefresh` (FR-018a).

### Implementation for User Story 1, slice C

- [ ] T026 [US1] [U70] [U71] [U72] [U73] [U74] [U75] [U76] [U77] [U78] [U79] [U80] [U81] [U82] [U83] [U84] Create `crates/micold-client/src/features/pr_status.rs` (`State`, `Phase`, `Msg`, `Effect`, `update`) for the messages of T023, and hold it in the application state and `Message` enum (`crates/micold-client/src/app.rs`, `crates/micold-client/src/features/mod.rs`). T023 passes; `features_are_render_free` stays green.
- [ ] T027 [US1] [U89] Add the pull request source factory to `IssueTooling` in `crates/micold-client/src/shell/capabilities.rs` (`GhCli` in `Capabilities::real()`, the fake in tests); `no_concrete_implementations` stays green.
- [ ] T028 [US1] [A10] [A11] [A12] [A14] [U87] [U88] [U89] [U91] [U96] [U97] Create `crates/micold-client/src/shell/pr_status.rs` (steps 0 to 3 and 5 of RW §2 "What one reading does", after `shell/issues.rs`; step 4 is story 3's), with one `debug` log line naming the outcome kind and the branch count.
- [ ] T029 [US1] [A31] [A40] [A41] [A42] [U85] [U86] [U90] [U92] [U93] [U94] [U95] Wire the events in `crates/micold-client/src/shell/daemon_sync.rs` and `crates/micold-client/src/shell/workspace.rs`: `Attached` → `Held`; the `CatalogChanged` arm → `ListingArrived`; `Displaced`, `Refused { ProjectBusy }`, disconnect and a project switch → `Released`; the settings mirror → `EnabledChanged`. T024 and T025 pass.

**Checkpoint**: the client reads and holds pull request status; nothing shows it yet.

---

## Phase 6: User Story 1, slice D — the indicator on the row and the switch in Settings (Priority: P1) 🎯 MVP

**Goal**: Story 1 whole: the user turns the switch on in Settings and every worktree row whose
branch has a pull request shows the indicator — state and, for an open or draft one, the combined
check status.

**Independent Test**: Spec story 1's independent test, as quickstart §B2 and §B3; automated: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test features_settings --test layout_snapshot --test showcase_completeness`.

### Tests for User Story 1, slice D (MANDATORY — Constitution Principle I) ⚠️

- [ ] T030 [P] [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A7] [A8] [A9] [U98] [U99] [U100] Extend `crates/micold-client/tests/features_sidebar.rs` (DM §4): a worktree row whose branch is in `statuses` projects `Some(RowPullRequest)` with that status and `age_secs` from the `now` passed in; a branch without an entry, a detached worktree, the "Default" entry and session rows project `None`; with empty statuses every row's projection equals today's.
- [ ] T031 [P] [US1] [A13] [U101] Extend `crates/micold-client/tests/icons_font.rs` and `crates/micold-client/tests/icons.rs` for `PrOpen`, `PrDraft`, `PrMerged`, `PrClosed`, `ChecksPassing`, `ChecksPending`, `ChecksFailing` (UI §1): each has a codepoint in the shipped font, and the seven are distinct glyphs.
- [ ] T032 [P] [US1] [U102] [U103] Extend `crates/micold-client/tests/showcase_completeness.rs` and the `material_builder_api` gate for `PullRequestIndicator` (UI §1): builder form, and a "Pull request indicator" catalogue entry with its 12 poses.
- [ ] T033 [P] [US1] [U104] [U105] [U106] [U107] Extend `crates/micold-client/tests/features_settings.rs` and `crates/micold-client/tests/settings_sections.rs` (UI §5): `Msg::PrStatusToggled` changes the draft; applying it sends `SettingsSet { pr_status_enabled: Some(v) }` and an unchanged draft sends `None`; the draft follows a `SettingsChanged` from the daemon; the section's title is "GitHub".
- [ ] T034 [US1] [U108] [U109] [U110] Add the covered states of UI §6 that belong to this slice to `crates/micold-client/tests/support/covered_states.rs`: a row with an indicator (open, failing), the same at the narrowest sidebar width, a row without a pull request with the switch on (geometry equal to the switch off), Settings → GitHub with the switch, and the showcase entry. `layout_coverage_registry` fails until T039.

### Implementation for User Story 1, slice D

- [ ] T035 [US1] [A13] [U101] Add the seven `Icon` variants to `crates/micold-client/src/icons.rs`. T031 passes.
- [ ] T036 [US1] [U102] [U103] Create `crates/micold-client/src/ui/material/pull_request_indicator.rs` (`PullRequestIndicator`, `PrMark`, `CheckMark`; stale form included, as the showcase poses it), export it from `crates/micold-client/src/ui/material/mod.rs`, and add the catalogue entry in `crates/micold-client/src/showcase/catalogue.rs` and `crates/micold-client/src/showcase/sections/atoms.rs`. T032 passes.
- [ ] T037 [US1] [A1] [A2] [A3] [A4] [A5] [A6] [A7] [A8] [A9] [U98] [U99] [U100] Add `RowPullRequest` to the row projection in `crates/micold-client/src/features/sidebar.rs` and draw the indicator in the row's trailing element in `crates/micold-client/src/ui/sidebar.rs::build_items` (UI §2), passing `now` from the view glue. T030 passes.
- [ ] T038 [US1] [U104] [U105] [U106] [U107] Add the switch: the draft field and `Msg::PrStatusToggled` in `crates/micold-client/src/features/settings.rs` (section title "GitHub"), the `Checkbox` and `field_note` with the wording of UI §5 in `crates/micold-client/src/ui/settings/github.rs`, and the field in `crates/micold-client/src/shell/persist.rs`. T033 passes.
- [ ] T039 [US1] [U108] [U109] [U110] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` for T034's states; the layout gates pass.
- [ ] T040 [US1] Add "Pull request status" to `docs/user-guide/worktrees-and-sessions.md` (the indicator, its four states and three check statuses with their glyphs, which pull request is shown, that only the project's own github.com repository is read, that a window which does not hold the project shows none) and the switch to `docs/user-guide/settings.md` (off by default, what is read, when, what is sent, prerequisites `gh` and `gh auth login`, that nothing is shown and no error appears without them); add `PullRequestIndicator` to `docs/development/component-library.md`.

**Checkpoint**: story 1 works end to end and is the MVP.

---

## Phase 7: User Story 2 - Read the pull request's details and open it in the browser (Priority: P2)

**Goal**: The worktree tooltip names the pull request, its state, checks and review; the row menu
opens it in the browser.

**Independent Test**: Spec story 2's independent test, as quickstart §B4 and §B5; automated: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test worktree_menu_pull_request --test pr_open`.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T041 [P] [US2] [A15] [A16] [A17] [A18] [A20] [A24] [U111] [U112] [U113] [U114] Extend `crates/micold-client/tests/features_sidebar.rs` for `worktree_tooltip` (UI §3): with `None` the output equals today's expected strings byte for byte; with a status, lines 1 to 4 follow today's lines in order; no `Checks:` line for merged, closed or no checks; no `Review:` line without a decision; a 200-character title is cut to 72 characters ending in `…` with the number before it; control characters and line breaks in a title become spaces; no line holds the address.
- [ ] T042 [P] [US2] [A23] [U115] Write `crates/micold-client/tests/worktree_menu_pull_request.rs` (UI §4): `worktree_menu_items` for a row with a status holds **Open pull request** directly above **Delete**; for a row without one the items equal today's, entry for entry.
- [ ] T043 [P] [US2] [A19] [U116] [U117] Write `crates/micold-client/tests/pr_open.rs` (UI §4) with a recording `LinkOpener`: `WorktreeMsg::PullRequestOpenRequested(dir)` opens exactly the stored address; an address not starting with `https://github.com/` opens nothing; a row that lost its status opens nothing; selection, sessions and sidebar state are equal before and after (FR-014).
- [ ] T044 [P] [US2] [A21] [U118] Extend `crates/micold-client/tests/icons_font.rs` for the new `OpenInBrowser` variant (`open_in_new`; `icons.rs` has no variant drawing that glyph today), and assert in `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` that `worktree_tooltip` and the menu builder call nothing in `shell/` (FR-012, SC-008).

### Implementation for User Story 2

- [ ] T045 [US2] [A15] [A16] [A17] [A18] [A20] [A24] [U111] [U112] [U113] [U114] Extend `worktree_tooltip` in `crates/micold-client/src/features/sidebar.rs` with the `Option<RowPullRequest>` argument and lines 1 to 4 of UI §3, and pass the row's projection at its call sites. T041 passes.
- [ ] T046 [US2] [A19] [A21] [A23] [U115] [U116] [U117] [U118] Add `Msg::PullRequestOpenRequested` to `crates/micold-client/src/features/worktree.rs`, the menu entry to `crates/micold-client/src/ui/mod.rs::worktree_menu_items`, the `OpenInBrowser` icon to `crates/micold-client/src/icons.rs`, and the handler that hands the address to `LinkOpener` in `crates/micold-client/src/shell/pr_status.rs`. T042, T043 and T044 pass.
- [ ] T047 [US2] Add the tooltip lines and **Open pull request** to "Pull request status" in `docs/user-guide/worktrees-and-sessions.md`.

**Checkpoint**: stories 1 and 2 work.

---

## Phase 8: User Story 3 - A merged pull request suggests removing the worktree (Priority: P3)

**Goal**: A worktree whose merged pull request holds all of its branch's work is marked "can be
removed", in the row and the tooltip; removing it stays the existing Delete action.

**Independent Test**: Spec story 3's independent test, as quickstart §B6 and §B7; automated: `scripts/build-lock.sh cargo test -p micold-client pr_status` and `--test features_sidebar`.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T048 [P] [US3] [U119] [U120] [U121] Extend `crates/micold-client/tests/features_pr_status.rs` (DM §3 invariant 5): `Finished(Ok)` stores `removable`; a branch in `removable` that is not merged, or not in `statuses`, is dropped; `Unavailable`, `Released` and switching off clear it.
- [ ] T049 [US3] [A25] [A27] [A28] [A29] [A30] [U122] [U123] [U124] [U125] Add shell tests named `pr_status_merged_*` to `crates/micold-client/src/main_tests.rs` (RW §2 step 4, RW §3): after the source answers, `MergedBranchCheck` is sent with one query per merged branch carrying its `head`, and none for open, draft or closed ones; with no merged branch it is not sent; `Contained` marks the branch, `Beyond` and `Unknown` do not; no answer within 10 s, a refusal, or an answer list of another length applies the statuses with no suggestion; a merged branch with a newer open pull request is not asked about (story 3 scenario 6).
- [ ] T050 [P] [US3] [A25] [A26] [U126] [U127] Extend `crates/micold-client/tests/features_sidebar.rs` (UI §2–3): `RowPullRequest.removable` follows `removable`; the tooltip's last line is `Cleanup: merged — this worktree can be removed (right-click, Delete)` only then; the row's menu and the message **Delete** sends are the same as for a row without the mark (FR-016).
- [ ] T051 [US3] [U128] Add the covered state "row with indicator and the can-be-removed chip" (and its narrow form) to `crates/micold-client/tests/support/covered_states.rs`.

### Implementation for User Story 3

- [ ] T052 [US3] [A25] [A27] [A28] [A29] [A30] [U119] [U120] [U121] [U122] [U123] [U124] [U125] Add step 4 of RW §2 to `crates/micold-client/src/shell/pr_status.rs` and `PendingOp::MergedBranchCheck` to `crates/micold-client/src/shell/daemon_sync.rs`; store `removable` in `crates/micold-client/src/features/pr_status.rs`. T048 and T049 pass.
- [ ] T053 [US3] [A25] [A26] [U126] [U127] [U128] Carry `removable` in the projection and add the `Cleanup:` line in `crates/micold-client/src/features/sidebar.rs`; draw the label-only chip **can be removed** in `crates/micold-client/src/ui/sidebar.rs::build_items`; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`. T050 and T051 pass.
- [ ] T054 [US3] Add "A merged pull request" to "Pull request status" in `docs/user-guide/worktrees-and-sessions.md`: when the mark appears, that nothing is removed without the Delete confirmation, and that no mark appears when the pull request's last commits were never fetched.

**Checkpoint**: stories 1 to 3 work.

---

## Phase 9: User Story 4 - The status stays current without asking, and can be refreshed when wanted (Priority: P4)

**Goal**: The status is read again every 5 minutes and after each refresh of the worktree list,
one reading at a time, paused by GitHub's request limit, and shown as stale after 10 minutes
without a successful reading.

**Independent Test**: Spec story 4's independent test, as quickstart §B8 and §B13; automated: `scripts/build-lock.sh cargo test -p micold-client pr_status` and `--test idle_subscriptions`.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T055 [P] [US4] [U129] [U130] [U131] Write `crates/micold-core/tests/pull_request_stale.rs` (R13): `is_stale` is false at 600 s, true at 601 s, false when `now` is before `read_at`.
- [ ] T056 [P] [US4] [U132] [U133] [U134] [U135] [U136] [U137] [U138] [U139] Extend `crates/micold-client/tests/features_pr_status.rs` (RW §2):
  - `Trigger { Interval }` and `Trigger { Refresh }` read from `Idle`; start nothing while off, not held, awaiting the listing or paused.
  - During a reading, three `Trigger { Refresh }` and its `Finished` yield exactly one further `Read`; `Trigger { Interval }` during a reading yields none.
  - After `RateLimited { until }`: every trigger before `until` yields none, a pending `again` is dropped, the first trigger at or after `until` reads; the pause survives `Released` and a new `Held` + `ListingArrived`.
  - A `Trigger` 60 s or more after a reading started abandons it, starts a new one with a new `seq`, and the old answer is dropped.
- [ ] T057 [P] [US4] [U140] [U141] [U142] Extend `crates/micold-client/tests/idle_subscriptions.rs` (RW §1): the 300 s subscription is absent with the switch off, absent in a window that does not hold its project, and present when on and held.
- [ ] T058 [US4] [A32] [A33] [A34] [A35] [A36] [A37] [A38] [A39] [U143] [U144] [U145] Add shell tests named `pr_status_refresh_*` and `pr_status_tick_*` to `crates/micold-client/src/main_tests.rs` (RW §1 S3–S5): `RefreshFinished` and `RefreshTimedOut` start a reading for the branches the updated listing shows, after the refresh control is idle and its notice shown, and a failed reading changes neither (FR-027); `Message::PrStatusTick` starts one; and raise the counts of `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` to S1 to S5.
- [ ] T059 [P] [US4] [A22] [U146] [U147] Extend `crates/micold-client/tests/features_sidebar.rs` (UI §3): with `age_secs` above 600 the tooltip gains `Read: <n> min ago` after the review line (`Read: <h> h ago` from 120 minutes); at 600 and below it has no such line; on a removable row the `Read:` line stands before the `Cleanup:` line, which stays last (UI §3 rows 5 and 6).
- [ ] T060 [US4] [U148] Add the covered state "row with a stale indicator" to `crates/micold-client/tests/support/covered_states.rs`: same geometry as the current form.

### Implementation for User Story 4

- [ ] T061 [US4] [U129] [U130] [U131] Add `is_stale` to `crates/micold-core/src/pull_request.rs`. T055 passes.
- [ ] T062 [US4] [U132] [U133] [U134] [U135] [U136] [U137] [U138] [U139] Add `Msg::Trigger`, `again`, the pause check and the abandoned-reading rule to `crates/micold-client/src/features/pr_status.rs`. T056 passes.
- [ ] T063 [US4] [A32] [A33] [A34] [A35] [A36] [A37] [A38] [A39] [U140] [U141] [U142] [U143] [U144] [U145] Add the guarded `iced::time::every(300 s)` subscription and `Message::PrStatusTick` to `crates/micold-client/src/shell/subscriptions.rs`, and start a reading at the end of a list refresh in `crates/micold-client/src/shell/daemon_sync.rs` (`RefreshFinished`, `RefreshTimedOut`). T057 and T058 pass.
- [ ] T064 [US4] [A22] [U146] [U147] [U148] Draw the stale form (`.stale(age_secs > 600)`) in `crates/micold-client/src/ui/sidebar.rs` and add the `Read:` line in `crates/micold-client/src/features/sidebar.rs`; regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt`. T059 and T060 pass.
- [ ] T065 [US4] Add "When the status is read" to "Pull request status" in `docs/user-guide/worktrees-and-sessions.md`: on opening, every 5 minutes, with the refresh button; the dimmed (stale) form and the `Read:` line; what happens at GitHub's request limit; that failures are silent and what clears the indicators.

**Checkpoint**: all four stories work.

---

## Phase 10: Polish & Cross-Cutting Concerns

- [ ] T066 [P] Describe the feature in `docs/development/architecture.md`: the `pull_request` core module, where the reading runs and why (R5), the holding rule (R6), the start events, protocol 21's `MergedBranchCheck` and `pr_status_enabled`.
- [ ] T067 Run quickstart §B (B1 to B17) with the `visual-pass` skill in both themes and record results and screenshots under `specs/040-worktree-pr-ci-status/evidence/`; fix what it finds under the test-first rule.
- [ ] T068 Run `mise run gate` on the finished feature and cross-check `cargo check --target aarch64-apple-darwin`; confirm no `cfg` arm was added (FR-034): `git diff <merge-base of M1>..HEAD -- crates/ | grep -n '^+.*cfg[(!]'` prints only `#[cfg(test)]` lines; and tick `specs/040-worktree-pr-ci-status/checklists/requirements.md` items that the implementation closes.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 3** (slice A): no dependency.
- **Phase 4** (slice B): no code dependency on phase 3; it follows phase 3 only so that the two slices merge in one order.
- **Phase 5** (slice C): phases 3 and 4.
- **Phase 6** (slice D): phase 5.
- **Phase 7** (US2): phase 6 (the row projection).
- **Phase 8** (US3): phases 4 (the RPC), 6 and 7 (the tooltip argument).
- **Phase 9** (US4): phases 6 and 7; independent of phase 8.
- **Phase 10**: all stories.

### Within Each Phase

Tests are written and seen failing before the implementation tasks that name them. T001 (fixtures)
precedes T003 and T006. A task that regenerates `layout_snapshot.txt` is the last code task of its
phase.

### Parallel Opportunities

Tasks marked [P] within one phase touch different files. `features_sidebar.rs`, `main_tests.rs`,
`features_pr_status.rs` and `covered_states.rs` are each extended by one task per phase.

---

## Parallel Example: User Story 1, slice A

```text
T002 pull_request_query.rs   T004 pull_request_select.rs   T005 pull_request_checks.rs
T007 pull_request_source.rs  T008 pull_request_is_never_stored.rs
```

---

## Implementation Strategy

Slices A to D of story 1 are the MVP: after phase 6 a user can turn the switch on and see the
indicators. Stories 2, 3 and 4 each add one behaviour on top and ship on their own. Polish closes
with the architecture page and the recorded visual pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

Story 1 has 40 tasks and is split along its layers into four milestones (milestones.md rule 3); the
first three ship no UI (rule 6) and M4 completes them. The PRs of M1 to M3 change no user-visible
behaviour and carry the `docs-not-needed` label; M4 to M7 each carry their user-guide task.

### M1 — The pull request source (US1 core)

- **Tasks**: T001–T013
- **Deliverable**: On `main`, `micold-core` can ask `gh` for the pull requests of up to 50 branches in one request and turn the recorded answers into per-branch statuses — state, combined check status, review, head commit — and name every failure kind and the rate-limit pause, all held by tests against recorded `gh --include` output. Nothing calls it yet (milestones.md rule 6); M3 does.
- **Satisfies**: FR-002 to FR-006, FR-008 (rules); FR-023, FR-028, FR-031 (query and arguments); FR-019, FR-024, FR-025 (failure kinds, pause time); FR-032 (no `Serialize`, redacting `Debug`); SC-002
- **Verify**: `mise run test-core`
- **Depends on**: —
- **Tier**: full

### M2 — The switch on the wire and the merged-branch question

- **Tasks**: T014–T022
- **Deliverable**: On `main`, client and daemon speak protocol 21: the daemon stores `pr_status_enabled` (off by default), reports it in `Welcome`, broadcasts a change to every window, and answers `MergedBranchCheck` for a repository without changing it. No UI sets or uses either yet; M4 and M6 do.
- **Satisfies**: FR-030 (default off, kept across restarts); FR-029 (every window follows); FR-015, FR-017, FR-018a (the containment rule and its RPC)
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-daemon --test merged_branch_check --test pr_status_setting`
- **Depends on**: M1 (order only)
- **Tier**: full

### M3 — The reading in the client

- **Tasks**: T023–T029
- **Deliverable**: On `main`, a window that holds a project and finds `pr_status_enabled: true` reads the project's pull request status once when the listing arrives and once when the switch turns on, off the update loop, and holds it in memory; a window that is refused or displaced reads nothing; no failure produces a notice. Nothing draws the status yet and no control sets the switch; M4 completes it.
- **Satisfies**: US1 scenarios 10–12, 14; US4 scenarios 1, 10–12; FR-018 (opening, switching on), FR-018a, FR-020, FR-021, FR-025, FR-026; SC-004, SC-005 (the reading is off the update loop, A12), SC-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client pr_status`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_status --test pr_status_is_read_only_on_named_events`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — The indicator on the row and the switch in Settings 🎯 MVP

- **Tasks**: T030–T040
- **Deliverable**: On `main`, a user checks **Show pull request status on worktrees** in Settings → GitHub and, within 10 seconds, every worktree row whose branch has a pull request shows the indicator — open, draft, merged or closed, and failing, pending or passing for an open or draft one; unchecking it removes them at once. The showcase shows the indicator in every state.
- **Satisfies**: US1 scenarios 1–9, 13; FR-001 to FR-003, FR-007, FR-009, FR-029, FR-030, FR-033, FR-035 (indicator and switch); SC-001
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test features_settings --test settings_sections --test showcase_completeness --test layout_snapshot`; quickstart §B1 to §B3
- **Depends on**: M3
- **Tier**: full

### M5 — Pull request details in the tooltip, and Open pull request

- **Tasks**: T041–T047
- **Deliverable**: On `main`, hovering a row with an indicator shows the pull request's number and title, state, checks and review; right-clicking it offers **Open pull request**, which opens the page in the browser. Rows without an indicator have today's tooltip and menu.
- **Satisfies**: US2 scenarios 1–7, 9, 10; FR-010 to FR-014, FR-035 (tooltip, opening); SC-008, SC-009
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test worktree_menu_pull_request --test pr_open`; quickstart §B4, §B5
- **Depends on**: M4
- **Tier**: light

### M6 — A merged pull request suggests removing the worktree

- **Tasks**: T048–T054
- **Deliverable**: On `main`, the row of a worktree whose pull request was merged and whose branch holds nothing newer carries the chip **can be removed** and a `Cleanup:` tooltip line; a merged branch with later commits, or one whose last commits were never fetched, does not. Delete and its confirmation are unchanged.
- **Satisfies**: US3 scenarios 1–6; FR-015 to FR-017, FR-018a, FR-035 (removal suggestion); SC-010
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client pr_status_merged`; `scripts/build-lock.sh cargo test -p micold-client --test features_sidebar --test layout_snapshot`; quickstart §B6, §B7
- **Depends on**: M2, M5
- **Tier**: full

### M7 — The status stays current: interval, refresh, request limit, stale form

- **Tasks**: T055–T065
- **Deliverable**: On `main`, the indicators follow GitHub every 5 minutes without the user asking and within 10 seconds of pressing the sidebar's refresh; presses during a reading cause one further reading; a rate-limit answer pauses readings until GitHub's reset time; a status older than 10 minutes is drawn dimmed and its tooltip says how old it is. With the switch off the idle window has no new timer.
- **Satisfies**: US4 scenarios 2–9; US2 scenario 8; FR-018 (interval, refresh), FR-019, FR-022, FR-024, FR-027, FR-035 (when GitHub is contacted); SC-003, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client pr_status`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_status --test idle_subscriptions`; quickstart §B8
- **Depends on**: M5, M6 (order: both extend the tooltip in `features/sidebar.rs` and the row in `ui/sidebar.rs::build_items`, and each regenerates `layout_snapshot.txt`)
- **Tier**: full

### M8 — Architecture page and the recorded visual pass

- **Tasks**: T066–T068
- **Deliverable**: On `main`, `docs/development/architecture.md` describes the pull request reading, and `specs/040-worktree-pr-ci-status/evidence/` holds the recorded quickstart §B pass (B1 to B17) in both themes.
- **Satisfies**: FR-009, FR-034; SC-005, SC-007, SC-009 to SC-011 (observed in §B)
- **Verify**: `ls specs/040-worktree-pr-ci-status/evidence/`; `mise run gate`
- **Depends on**: M1–M7
- **Tier**: light

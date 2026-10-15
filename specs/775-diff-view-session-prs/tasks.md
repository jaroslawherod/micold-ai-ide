---

description: "Task list for feature 775: PRs in the diff view"
---

# Tasks: Diff view shows PRs created within sessions and worktrees

**Input**: `specs/775-diff-view-session-prs/` (plan.md, spec.md, data-model.md, contracts/, research.md, quickstart.md)

**Tests**: mandatory (Constitution I). Every implementation task is preceded by a failing test task in the same milestone.

**Documentation**: the user-guide task of each story sits in that story's milestone (Constitution VII).

## Format: `[ID] [P?] [Story] Description`

`[P]` = different files, no dependency on an unfinished task. Paths are relative to the repo root.

## Phase 1: Setup

- [x] T001 [P] Add gh fixtures `crates/micold-core/tests/fixtures/gh/pr_list_*.txt`: open+merged+closed+draft, a cross-repository (fork) PR, an unknown `state`, an empty list, three PRs out of `createdAt` order

## Phase 2: Foundational (core list read, shared by every story)

- [x] T002 Write failing tests `crates/micold-core/tests/pull_request_list.rs`: `parse_list` (states incl. draft, fork dropped, newest first, unknown state is `ReadingFailure::Passing`), `list_args` (variables only, no branch in the document, `--hostname github.com`), `FakePullRequestSource::list_branch` scripted answers and recorded calls (FR-001, FR-002, FR-003, FR-011)
- [x] T003 Implement `BranchPullRequest`, `BranchPrState`, `list_query`, `list_args`, `parse_list` in `crates/micold-core/src/pull_request.rs`
- [x] T004 Implement `PullRequestSource::list_branch` in `crates/micold-core/src/pull_request.rs` (trait, Fake) and `crates/micold-core/src/github.rs` (`GhCli`, classified by the existing `reading_failure`); make `BlockingSource` in `crates/micold-client/src/main_tests.rs` implement it so the workspace compiles
- [x] T005 Amend the guards in `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs`: `the_source_is_read_from_one_place` also pins `.list_branch(` to `shell/pr_status.rs` (the `.diff(` pin moves to the M2 task that adds `diff`); `a_pull_requests_title_and_address_are_never_logged` covers the new files that touch a title (`features/changes.rs`, `shell/changes.rs`, `ui/changes.rs`); add `BranchPullRequest` to `crates/micold-core/tests/pull_request_is_never_stored.rs`

## Phase 3: User Story 1 - See the PRs of the worktree (P1) 🎯 MVP

**Goal**: Changes pane lists the PRs of the entry's branch with number, title, state. **Independent test**: `crates/micold-client/tests/features_pr_changes.rs`, quickstart §B steps 1, 2, 5.

- [x] T006 [P] [US1] Write failing reducer tests `crates/micold-client/tests/features_pr_changes.rs` (new): `Opened` returns `ReadList` first and `ReadPullRequests` independent of it (FR-009, SC-003); setting off, no branch (Default, detached) give `Absent` and no effect (FR-014); `PullRequestsRead(Ok)` fills `Ready`; off-to-on `PullRequestSettingChanged` returns `ReadPullRequests`, on-to-off gives `Absent` and clears the list (FR-014); `Err` keeps the last list and raises no notice (FR-008); a worktree with no session lists the same (FR-002)
- [x] T007 [P] [US1] Write failing shell tests named `pr_list_read_*` in `crates/micold-client/src/main_tests.rs`: two projects with the same branch name each read only their own repo through `FakePullRequestSource` (FR-010); two sessions on one worktree share one view and one read (Principle II); no `gh` or no GitHub remote gives `Absent` and no notice; setting off sends no `ReadPullRequests`
- [x] T008 [P] [US1] Write a failing test for the single `BranchPrState -> material::PrMark` mapping (open, draft, merged, closed distinct, FR-003) beside it in `crates/micold-client/src/features/changes.rs`
- [x] T009 [P] [US1] Write the failing component test for `PullRequestRow` in `crates/micold-client/src/ui/material/pull_request_row.rs` and a `COMPONENTS` entry in `crates/micold-client/src/showcase/catalogue.rs` so `showcase_completeness` fails until the showcase section exists, in the pattern of `PullRequestIndicator` (Constitution VIII)
- [x] T010 [US1] Implement `PrList`, `OpenView.pull_requests`, `Msg::{PullRequestsRead, PullRequestSettingChanged}`, `Effect::ReadPullRequests` and their transitions in `crates/micold-client/src/features/changes.rs`, plus the state mapping from T008
- [x] T011 [US1] Implement the `PullRequestRow` builder in `crates/micold-client/src/ui/material/pull_request_row.rs` (`mod` in `ui/material/mod.rs`), composing the existing `PullRequestIndicator`, and add it to `crates/micold-client/src/showcase/sections/review.rs`
- [x] T012 [US1] Wire the read: `crates/micold-client/src/shell/changes.rs` hands `ReadPullRequests` to `crates/micold-client/src/shell/pr_status.rs`, the one place that calls `(tooling.pull_requests)(gh)`; result returns as `Msg::PullRequestsRead`; a `RateLimited{until}` from the read is fed to the existing `pr_status` `Outcome::Err` path so `pause_until` is set (R3); gate on `app.core.pr_status.enabled`
- [x] T013 [US1] Add `pull_request_section` to `crates/micold-client/src/ui/changes.rs`: rows from `PullRequestRow` above the file list (`#number title`, state mark), absent when `PrList::Absent`
- [x] T014 [US1] User guide: new "Pull requests" section in `docs/user-guide/reviewing-changes.md`; pointer from "Pull request status" in `docs/user-guide/worktrees-and-sessions.md`; what the toggle now controls in `docs/user-guide/settings.md`
- [ ] T015 [US1] Visual pass (quickstart §B steps 1, 2, 5 without the offline part) via the `visual-pass` skill; record evidence in `specs/775-diff-view-session-prs/`

## Phase 4: User Story 2 - Review a PR's diff (P1)

**Goal**: selecting a PR shows its base...head files and diff; "Working tree" returns. **Independent test**: `pr_diff_split.rs`, `features_pr_changes.rs`, quickstart §B steps 3, 4.

- [x] T016 [P] [US2] Add fixtures `crates/micold-core/tests/fixtures/gh/pr_diff_*.txt`: rename, add, delete, binary, large, quoted path, empty
- [x] T017 [US2] Write failing tests `crates/micold-core/tests/pr_diff_split.rs` (each fixture: path, kind, binary, counts, `large` by `limits::MAX_CHANGED_LINES`; `patch` feeds `parse_unified`) and `crates/micold-core/tests/pull_request_diff.rs` (`diff_args`, HTTP 406 gives `DiffReadFailure::TooLarge`, other failures `Failed`, Fake diff answers) (FR-004)
- [x] T018 [US2] Implement `crates/micold-core/src/review/pr_diff.rs` (`split_pr_diff`, `PrDiff`, `PrFile`), `mod pr_diff` in `crates/micold-core/src/review/mod.rs`
- [x] T019 [US2] Implement `PullRequestSource::diff`, `DiffReadFailure`, `diff_args`, `diff_failure` in `crates/micold-core/src/pull_request.rs` and `crates/micold-core/src/github.rs`; `BlockingSource` in `crates/micold-client/src/main_tests.rs` implements `diff`
- [x] T020 [P] [US2] Write a failing ordering test (inline `#[cfg(test)]` in `crates/micold-core/src/session.rs`) and derive `PartialOrd, Ord` on `SessionLocation` in `crates/micold-core/src/session.rs` (key of `remembered`)
- [x] T021 [P] [US2] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: `SourceChosen(Some(n))` returns `ReadPullRequestDiff`, files listed from the PR, file selection parses `patch` with no read, `SourceChosen(None)` returns to the working-tree view (FR-005, FR-006), per-entry memory across entry switches, a vanished PR returns to working tree (FR-013), `TooLarge` and `Failed` show a line and no notice; amend `crates/micold-client/tests/features_changes.rs` where `OpenView` gains `source`
- [x] T022 [US2] Implement `Source`, `PrDiffLoad`, `State.remembered`, `Msg::{SourceChosen, PullRequestDiffRead}`, `Effect::ReadPullRequestDiff` in `crates/micold-client/src/features/changes.rs`
- [x] T023 [P] [US2] Write a failing shell test `pr_diff_read_*` in `crates/micold-client/src/main_tests.rs`: `ReadPullRequestDiff` runs `diff` on the source from the single factory call; failure raises no notice
- [x] T023b [US2] Wire `ReadPullRequestDiff` through `crates/micold-client/src/shell/changes.rs` and `crates/micold-client/src/shell/pr_status.rs` (same single factory call)
- [x] T024 [US2] `crates/micold-client/src/ui/changes.rs`: "Working tree" row and PR rows selectable, PR file list through `file_row`, existing `diff_pane` and layouts, base line "Pull request #N: base ... head", "too large" line with the existing open-address action
- [x] T025 [US2] User guide: selecting a PR, returning, merged-and-deleted heads, the large-PR line in `docs/user-guide/reviewing-changes.md`
- [ ] T026 [US2] Visual pass (quickstart §B steps 3, 4); record evidence in `specs/775-diff-view-session-prs/`

## Phase 5: User Story 3 - Entries stay current (P2)

**Goal**: list refreshes on the status triggers while the view is open; stale marked. **Independent test**: shell tests in `main_tests.rs`, quickstart §B step 6.

- [x] T027 [P] [US3] Write failing shell tests named `pr_list_refresh_*` in `crates/micold-client/src/main_tests.rs`: a status trigger while a view is open requests the list (SC-004); none while no view is open; skipped while `pr_status` is paused by `RateLimited{until}` and while a read is under way; `Unavailable` is re-asked only on a trigger (R3); a list-read `RateLimited` sets `pause_until`
- [x] T028 [P] [US3] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: `PullRequestsWanted` returns `ReadPullRequests`; a failed refresh keeps the list, which is stale by `is_stale(read_at, now)` (FR-007, FR-012); a refreshed state replaces the mark
- [x] T029 [US3] Extend `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs` so `Msg::PullRequestsWanted` is sent only from the named status triggers
- [x] T030 [US3] (`Msg::PullRequestsWanted`, its `ReadPullRequests` transition and `pub fn paused` were pulled forward into M1; M3 only sends it from the triggers) Implement `Msg::PullRequestsWanted` in `crates/micold-client/src/features/changes.rs`, make `paused` `pub` in `crates/micold-client/src/features/pr_status.rs`, send the message from the existing triggers in `crates/micold-client/src/shell/pr_status.rs`
- [x] T031 [US3] "possibly stale" caption in `crates/micold-client/src/ui/changes.rs`
- [x] T032 [US3] User guide: refresh interval, stale caption, no notice when GitHub is unreachable, in `docs/user-guide/reviewing-changes.md` ("Keeping up with changes") and `docs/user-guide/settings.md`
- [ ] T033 [US3] Visual pass (quickstart §B steps 5 offline part, 6); record evidence in `specs/775-diff-view-session-prs/`

## Polish (no milestone)

No code changes remain after T033 (T034 below): recording the quickstart Part A and Part B results in `quickstart.md` and ticking tasks is the close unit's work, in the close PR.

- [x] T034 Record the quickstart Part A and Part B results in `quickstart.md` (Part B not observed, accepted; see Results there)

## Dependencies

Setup, Foundational, then US1 (MVP). US2 needs US1 (the section and list). US3 needs US1 (the list read); M3 is ordered after M2 only because both edit `features/changes.rs`.

Parallel: T006-T009 together (disjoint files); T016, T020, T021 together; T027, T028 together.

## Implementation Strategy

MVP is M1: the list is visible and correct. M2 makes entries selectable and shows the diff; until then M1 rows are display only (information, not buttons). M3 adds live refresh. The list read is a separate effect from the working-tree read at every step (FR-009).

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — PRs listed in the Changes pane 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: with "Show pull request status" on, the Changes pane of a worktree or session lists the PRs of its branch, newest first, each with number, title and its own open/draft/merged/closed mark; rows are informational until M2; absent when setting off, no branch or no GitHub access, with no notice
- **Satisfies**: US1 acceptance scenarios 1–4; FR-001, FR-002, FR-003, FR-008, FR-009, FR-010, FR-011, FR-014; SC-002, SC-003, SC-005
- **Verify**: `mise run test-core`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes`; `scripts/build-lock.sh cargo test -p micold-client pr_list_read`; quickstart §B steps 1, 2, 5
- **Depends on**: —
- **Tier**: full

### M2 — Select a PR and read its diff

- **Tasks**: T016–T026, T023b
- **Deliverable**: selecting a listed PR shows its base...head files and diff in the existing layouts, also for a merged PR with a deleted head; "Working tree" returns; the choice is remembered per entry
- **Satisfies**: US2 acceptance scenarios 1–3; FR-004, FR-005, FR-006, FR-013; SC-001
- **Verify**: `mise run test-core` (`pr_diff_split`, `pull_request_diff`); `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes`; `scripts/build-lock.sh cargo test -p micold-client pr_diff_read`; quickstart §B steps 3, 4
- **Depends on**: M1
- **Tier**: full

### M3 — The list stays current

- **Tasks**: T027–T033
- **Deliverable**: a PR opened, merged or closed while the Changes view is open shows its new state within the PR status refresh interval; a failed refresh keeps the list and marks it possibly stale
- **Satisfies**: US3 acceptance scenarios 1–2; FR-007, FR-012; SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test pr_status_is_read_only_on_named_events --test features_pr_changes`; `scripts/build-lock.sh cargo test -p micold-client pr_list_refresh` (the T027 tests are named `pr_list_refresh_*`); quickstart §B steps 5, 6
- **Depends on**: M2
- **Tier**: full

## Phase 6: TDD remediation

Source: `specs/775-diff-view-session-prs/tdd/verification.md` (verdict FAIL). The feature is not done
until R001-R006 are cleared. Tests only; no source change is needed unless a new test exposes a bug.

- [x] R001 [US2] (finding 1, 9) In `crates/micold-client/tests/features_pr_changes.rs` assert that `SourceChosen(None)` forgets the remembered PR (`state.remembered` empty; switching away and back shows the working tree) and does so while the list is still awaited; proves `changes.rs:596-602`. Verify: `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes`
- [x] R002 [US2] (finding 2, 3) Add tests for `SourceChosen(Some(n))`: an unlisted `n` returns `Effect::None` and changes nothing; choosing the shown PR again returns `Effect::None`; choosing a PR whose diff `Failed` reads again (`changes.rs:616-618`). Verify: same command.
- [x] R003 [US2] (finding 4) In `crates/micold-core/tests/pull_request_diff.rs` pin `diff_failure` for a rate limit and an unavailable `gh` through `diff_outcome` (exact `DiffReadFailure::Failed(..)` values, not `matches!(Failed(_))`) (`pull_request.rs:896`). Verify: `scripts/build-lock.sh cargo test -p micold-core --test pull_request_diff`
- [x] R004 [US2] (finding 5, 6, 7) In `crates/micold-core/tests/pr_diff_split.rs` add fixtures and tests: exactly 5000 changed lines is not large, 5001 is; a mode-only change is `ChangeKind::ModeOnly`; a `GIT binary patch` section is `Content::Binary` (`pr_diff.rs:78,94,111`). Verify: `scripts/build-lock.sh cargo test -p micold-core --test pr_diff_split`
- [x] R005 [US2] (finding 8) Assert `PrFile.patch` equals the file's exact section bytes in `pr_diff_split.rs` (`pr_diff.rs:114`). Verify: same command as R004.
- [x] R006a [US3] (finding 10) Added the M3 entry to `tdd/cycle-log.md`, stating no red was observed.
- [ ] R006b [US3] (finding 11) `tdd/test-list.md`: declined, not added. The spec was not planned through the tdd extension; the FR traceability table in `tdd/verification.md` stands in.

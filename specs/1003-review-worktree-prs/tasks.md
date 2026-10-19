---

description: "Task list for feature 1003: Review changes reaches every pull request of the worktree"
---

# Tasks: Review changes reaches every pull request of the worktree

**Input**: `specs/1003-review-worktree-prs/` (plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md)

**Tests**: mandatory (Constitution I). Every implementation task is preceded by a failing test task in the same milestone. Only `ui/` drawing and `shell/` spawn wiring rely on the quickstart §B exception (plan, Constitution Check I).

**Documentation**: the user-guide task of each story sits in that story's milestone (Constitution VII), all in `docs/user-guide/reviewing-changes.md` § Pull requests.

**Cross-platform**: git and `gh` run as argument vectors, no shell; `--date=unix` output parses the same on Linux, macOS and Windows (Constitution VI).

## Format: `[ID] [P?] [Story] Description`

`[P]` = different files, no dependency on an unfinished task. Paths are relative to the repo root. Builds on feature 775 (`specs/775-diff-view-session-prs/`).

## Phase 1: Setup

- [X] T001 [P] Add gh fixtures `crates/micold-core/tests/fixtures/gh/pr_list_branches_*.txt` (HTTP head + body, as the existing `pr_list_*.txt`) for the multi-alias answer of contracts/pull-request-list.md §2: `pr_list_branches_three.txt` (aliases `p0`–`p2`, one merged, one closed, one open and one draft PR, each with its own `headRefName`, out of `createdAt` order); `pr_list_branches_reopened.txt` (one alias with three PRs of the same `headRefName`: merged, closed, open); `pr_list_branches_default.txt` (one alias whose branch equals `defaultBranchRef.name`, with a PR); `pr_list_branches_fork.txt` (an `isCrossRepository: true` PR beside a same-repo one); `pr_list_branches_tie.txt` (two PRs with equal `createdAt`, numbers 7 and 9); `pr_list_branches_empty.txt` (aliases with empty `nodes`)

## Phase 2: Foundational (branch set and multi-branch read, core)

- [X] T002 Write failing unit tests in an inline `#[cfg(test)] mod tests` of the new `crates/micold-core/src/worktree_branches.rs` (with `mod worktree_branches` in `crates/micold-core/src/lib.rs` and empty `pub fn` stubs so it compiles), on reflog text in `%gD%x09%H%x09%gs` form (research R2, data-model §1): `parse_reflog` skips a line that does not split into 3 fields or whose `@{…}` is not a number, and strips `@{…}` from `refname`; `candidates` takes both names of `checkout: moving from <A> to <B>`, both names of `Branch: renamed refs/heads/<X> to refs/heads/<Y>`, and the current branch, dedups in first-seen order, and skips 40- and 64-hex commit names and `HEAD`; `branch_set` keeps a branch created here (oldest ref entry `branch: Created from …` at commit `c`, time `t`, HEAD entry at `c` within `MATCH_WINDOW_SECS` = `2` after which HEAD is on it), keeps it at `t ± 2` and drops it at `t ± 3`; keeps the `git worktree add -b` start (entries before the first checkout are on that checkout's `<A>`, or on the current branch when there is no checkout); drops a branch created elsewhere (no matching HEAD entry); drops a same-name remote copy (`Created from refs/remotes/origin/<name>` and `Created from origin/<name>`, rule 3, `copies_remote`); keeps `git switch -c x origin/main` (`Created from origin/main`); keeps a deleted branch (no ref log) and an expired one (oldest entry not a creation, rule 4); counts a renamed branch under both names (rule 5); always keeps the current branch (rule 6); a detached HEAD (`current: None`) gives only the created branches
- [X] T003 Implement `ReflogEntry`, `parse_reflog`, `candidates`, `copies_remote`, `branch_set` and `MATCH_WINDOW_SECS` in `crates/micold-core/src/worktree_branches.rs` until T002 passes (pure; no default-branch rule here)
- [X] T004 Write failing integration tests `crates/micold-core/tests/worktree_branch_set.rs` with real git in temp repos (helpers from `crates/micold-core/tests/support/attach_fixture.rs`), calling `worktree_branches::read_branch_set(&GitCli, dir, current)`: a worktree added with `-b A`, then `git switch -c B`, then `git switch -c C`, with A deleted, gives {A, B, C}; a branch created in the main checkout and checked out in the worktree is excluded; `git switch <name>` of a remote-only branch (a second clone as `origin`) is excluded; a second worktree of the same repo gets a disjoint set (FR-001, FR-002, FR-014, SC-001); a `FakeGit` whose HEAD reflog run fails gives `[current]`; a failing refs run treats every candidate as record-gone (kept)
- [X] T005 Implement `Git::reflogs(&self, dir, refs) -> io::Result<String>` in `crates/micold-core/src/git.rs` (trait; `GitCli` runs `git log -g --ignore-missing --date=unix --format=%gD%x09%H%x09%gs <refs> --` as an argument vector in `dir`, bounded like the other `GitCli` runs; `FakeGit` gains scripted reflog text per ref list) and `read_branch_set(git, dir, current)` in `crates/micold-core/src/worktree_branches.rs` (HEAD run, then one run over the candidates as `refs/heads/<n>`), until T004 passes
- [X] T006 Write failing tests in `crates/micold-core/tests/pull_request_list.rs` for the multi-branch read (contracts/pull-request-list.md §2, research R3), replacing the `list_branch` tests: `list_query(n)` declares `$b0`…`$b<n-1>`, one alias `p<i>` per branch with `first: 100` and `orderBy: {field: CREATED_AT, direction: DESC}`, `defaultBranchRef { name }` and `rateLimit`, and no branch name in the document; `list_args(repo, branches)` passes `--hostname github.com`, `owner`, `name` and `-f b<i>=<branch>` (a branch named `true` or `@x` stays a string); `parse_list` on the T001 fixtures: every PR of every alias with `head_branch` from `headRefName`, the reopened branch's three PRs, the default branch's alias dropped, the fork dropped, unknown `state` is `ReadingFailure::Passing`; `FakePullRequestSource::list_branches` records `(repo, branches)` per call; `list_branches` with 25 branches makes 1 request and with 26 makes 2 (25 + 1, `LIST_BRANCHES_PER_REQUEST`), sorts the merged list `created_at` descending then `number` descending (FR-004), fails the whole read when the second chunk fails (FR-013, SC-007), and with empty `branches` returns `Ok(vec![])` with no request
- [X] T007 Implement in `crates/micold-core/src/pull_request.rs`: `BranchPullRequest.head_branch`, `LIST_BRANCHES_PER_REQUEST: usize = 25`, `list_query(n)`, `list_args(repo, branches)`, `parse_list(body, branches)`, `PullRequestSource::list_branches(repo, branches, now)` (chunks one after another, first failure fails all, merged and sorted) replacing `list_branch` in the trait and `FakePullRequestSource`; `GhCli::list_branches` in `crates/micold-core/src/github.rs` (classified by the existing `reading_failure`); `BlockingSource` in `crates/micold-client/src/main_tests.rs` implements `list_branches`, and the one caller in `crates/micold-client/src/shell/pr_status.rs` calls `.list_branches(&repo, &[branch], now())` for now (T012 gives it the branch set), so the workspace compiles after this task; until T006 passes
- [X] T008 Amend the guards in `crates/micold-client/tests/pr_status_is_read_only_on_named_events.rs`: `the_source_is_read_from_one_place` pins `.list_branches(` (was `.list_branch(`) to `shell/pr_status.rs`; and check `crates/micold-core/tests/pull_request_is_never_stored.rs` still covers `BranchPullRequest` with its new field

## Phase 3: User Story 1 - See every pull request the worktree produced (P1) 🎯 MVP

**Goal**: the Pull requests section lists the pull requests of every branch of the worktree's branch set, newest first, each with number, title, state and check status. **Independent test**: `worktree_branch_set.rs`, `pull_request_list.rs`, `features_pr_changes.rs`, `pr_list_branch_set_*` in `main_tests.rs`, quickstart §B steps 2, 6.

- [X] T009 [P] [US1] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: `Effect::ReadPullRequests` carries `entry` (the worktree's `SessionLocation`) and `current: Option<String>` (was `branch`); a worktree on a detached HEAD still reads, with `current: None`; the project root (`Default`) stays `Absent` with no effect (FR-012); `base_line` names the shown PR's own `head_branch`, not the entry's current branch (research R6); amend the existing tests that build `ReadPullRequests` with `branch`
- [X] T010 [P] [US1] Write failing shell tests named `pr_list_branch_set_*` in `crates/micold-client/src/main_tests.rs`: `ReadPullRequests` reads the branch set in the entry's directory with `FakeGit` reflogs and calls `list_branches` with exactly that set (worktree that had A, B, C lists all three PRs, newest first; US1 scenarios 1, 2); two worktrees of one project each ask only their own set (US1 scenario 4, FR-014); no local git (`NOT_LOCAL`) asks with the current branch alone and raises no notice
- [X] T011 [US1] Implement in `crates/micold-client/src/features/changes.rs`: `Effect::ReadPullRequests { seq, project, entry, current }`, `request_pull_requests` reading for every worktree entry while enabled (also with `current: None`), `base_line` from `head_branch`, and the `Effect::ReadPullRequests` match in `crates/micold-client/src/shell/changes.rs` takes the new fields (passing `current` as the single branch, or no read when `None`, until T012), so the client builds after this task; until T009 passes
- [X] T012 [US1] Wire the read until T010 passes: `crates/micold-client/src/shell/changes.rs` passes `entry` and `current` and exposes `local` to `crates/micold-client/src/shell/pr_status.rs`, whose `on_list_remotes` runs `worktree_branches::read_branch_set` with the shared local `Git` on the list read's blocking thread, then `list_branches` (still the one place that calls `(tooling.pull_requests)(gh)`); no local git gives `[current]`
- [X] T013 [US1] User guide: in `docs/user-guide/reviewing-changes.md` § Pull requests, which pull requests are listed (every branch created in the worktree plus its current branch, never the default branch, merged-and-deleted branches included, branches only checked out there excluded)
- [x] T014 [US1] Visual pass (quickstart §B steps 1, 2, 6, without the check marks) via the `visual-pass` skill; record evidence in `specs/1003-review-worktree-prs/`. **Moved out of M1 to the close unit, beside T037**: it needs `gh` signed in to a GitHub repository, which M1's unit did not have (`visual-pass.md`)

### Check status on every entry (US1 scenario 3; milestone M2)

- [x] T015 [P] [US1] Write failing tests in `crates/micold-core/tests/pull_request_list.rs` for check status (FR-003, research R4) on new fixtures `crates/micold-core/tests/fixtures/gh/pr_list_branches_checks.txt` (one PR per check outcome: passing, failing, pending, no rollup; a merged and a closed PR with a failing rollup): `list_query` asks the last commit's `statusCheckRollup` counts with the sidebar's `FRAGMENT` fields; `parse_list` sets `BranchPullRequest.checks` by `reduce_checks` for every state, merged and closed included
- [x] T016 [US1] Add the rollup fields to `list_query` and `checks: CheckStatus` to `BranchPullRequest` in `crates/micold-core/src/pull_request.rs` (reuse `reduce_checks` and the `status_query` fragment, no copy), until T015 passes
- [x] T017 [P] [US1] Write failing tests for one shared `check_mark(CheckStatus) -> Option<CheckMark>` beside `pr_mark` in `crates/micold-client/src/features/changes.rs` (`None` → no mark, the other three distinct), and the `PullRequestRow::checks(CheckMark)` component test in `crates/micold-client/src/ui/material/pull_request_row.rs`, in the pattern of its existing test (Constitution VIII)
- [x] T018 [US1] Implement `check_mark` in `crates/micold-client/src/features/changes.rs` and make `indicator_marks` in `crates/micold-client/src/ui/sidebar.rs` use it (one mapping); implement `PullRequestRow::checks` passing the mark to its `PullRequestIndicator`; in `crates/micold-client/src/showcase/sections/review.rs` show a row with each check mark; until T017 passes
- [x] T019 [US1] `crates/micold-client/src/ui/changes.rs`: each pull request row draws its check mark with `.checks(..)` (US1 scenario 3)
- [x] T020 [US1] User guide: the check mark on every entry (merged and closed included, same marks as the sidebar) in `docs/user-guide/reviewing-changes.md` § Pull requests
- [x] T021 [US1] Visual pass (quickstart §B step 2, the check marks); record evidence in `specs/1003-review-worktree-prs/`. **Moved out of M2 to the close unit, beside T014 and T037**: no `gh` auth and no `xdotool` in the milestone container (`visual-pass.md`)

## Phase 4: User Story 2 - Switch between the pull requests and review each one's diff (P1)

**Goal**: previous/next and a header naming the shown pull request; every listed pull request's own diff is reachable. **Independent test**: `features_pr_changes.rs`, quickstart §B step 3.

- [x] T022 [P] [US2] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: three listed PRs with distinct diffs, `SourceChosen(Some(2nd))` then `SourceChosen(Some(3rd))` each read and show that PR's own files (SC-002, US2 scenarios 1, 2); `adjacent(prs, number, step)` in list order, `None` at each end and for an unlisted number; `PullRequestStepped(Next|Previous)` selects the adjacent PR through the `SourceChosen` path, does nothing at an end or while the working tree is shown (FR-006, SC-003); `pr_header` gives number, title, state mark, check mark and base line of the shown PR, `None` for the working tree (FR-007); three fast choices whose diffs answer out of order show only the last one's diff under its header (FR-016); a `Failed` or `TooLarge` diff keeps the header and stepping works; choosing the working tree from a shown PR makes `pr_header` `None` and stepping a no-op (US2 scenario 5); `Next` repeated from the first of 30 entries reaches the last (edge case: more PRs than fit)
- [x] T023 [US2] Implement `Step::{Previous, Next}`, `Msg::PullRequestStepped(Step)`, `pub fn adjacent`, `PrHeader` and `pub fn pr_header` in `crates/micold-client/src/features/changes.rs`, until T022 passes
- [x] T024 [P] [US2] Add `Icon::ChevronLeft` and `Icon::ChevronRight` glyphs in `crates/micold-client/src/icons.rs`, covered by the existing every-glyph checks in `crates/micold-client/tests/icons.rs` (extend their list first, so they fail)
- [x] T025 [US2] `crates/micold-client/src/ui/changes.rs`: header above the diff while a PR is shown: `PullRequestIndicator` (state + checks), `#number title`, the base line, and previous/next `IconButton`s, each disabled when `adjacent(..)` is `None` (contracts/changes-view.md § Header)
- [x] T026 [US2] User guide: the header and previous/next in `docs/user-guide/reviewing-changes.md` § Pull requests
- [x] T027 [US2] Visual pass (quickstart §B steps 3, 4); record evidence in `specs/1003-review-worktree-prs/`. **Moved out of M3 to the close unit, beside T014, T021 and T037**: no `gh` auth and no `xdotool` in the milestone container (`visual-pass.md`)

## Phase 5: User Story 3 - Selection holds and the list stays current (P2)

**Goal**: across refreshes the selection stays and new or changed pull requests show. Built by 775 (`remembered`, triggers), M1 (the set is recomputed per read) and M2 (the header reads the current list); these tasks pin it. **Independent test**: `features_pr_changes.rs`, `pr_list_refresh_branch_set_*`, quickstart §B step 4.

- [x] T028 [P] [US3] Write reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: with the second of three selected, a refresh adding a PR at the top keeps the second selected and shown (US3 scenario 1, FR-008); a refresh changing the selected PR's state and checks updates its row and `pr_header` and issues no new diff read (US3 scenario 3); the selected PR leaving the list returns to the working tree (US3 scenario 4)
- [x] T029 [P] [US3] Write a shell test `pr_list_refresh_branch_set_*` in `crates/micold-client/src/main_tests.rs`: a branch created in the worktree after the view opened (new `FakeGit` reflog text) is in the `list_branches` call of the next status trigger (US3 scenario 2, FR-009, SC-004)
- [x] T030 [US3] Fix in `crates/micold-client/src/features/changes.rs` or `crates/micold-client/src/shell/pr_status.rs` whatever T028–T029 expose; if they pass at once, record in the PR body that M1 and this milestone's T023 already delivered the behaviour
- [x] T031 [US3] User guide: a pull request opened from a new branch of the worktree appears at the next refresh and the shown one stays, in `docs/user-guide/reviewing-changes.md` ("Keeping up with changes")

## Phase 6: User Story 4 - A clear empty state (P2)

**Goal**: the section shows loading, empty and unavailable lines per the spec's section-states table. **Independent test**: `features_pr_changes.rs`, quickstart §B step 5.

- [x] T032 [P] [US4] Write failing reducer tests in `crates/micold-client/tests/features_pr_changes.rs`: `section(view, enabled, now)` for every row of the spec's section-states table (setting off and project root `Absent`; first read pending `Loading`; `Unavailable { last: None }` `Unavailable`; an empty read `Empty`; a list `List`; a failed refresh after a list `List { stale }` by `possibly_stale`) (FR-010, FR-011, FR-012, FR-015, SC-005); `PullRequestsRead(Err(Unavailable))` keeps `PrList::Unavailable { last }` (was `Absent`), keeps `view.source` with a last list and resets it to `Source::Changes` without one, and raises no notice (SC-006); the same for `Err(Passing)` and `Err(RateLimited{..})` (every failure kind keeps `last`, data-model §2); amend the 775 tests that expected `Absent` after `Unavailable`
- [x] T033 [US4] Implement `PrSection` and `pub fn section` and the `Err(Unavailable)` arm in `crates/micold-client/src/features/changes.rs`, until T032 passes
- [x] T034 [US4] `crates/micold-client/src/ui/changes.rs`: draw the section from `section(..)` per contracts/changes-view.md § Section ("Reading pull requests…", muted "Pull requests can't be shown for this worktree", "No pull requests from this worktree yet", the list), with the rows in a scrollable of bounded height (edge case: more PRs than fit)
- [x] T035 [US4] User guide: the loading, empty and unavailable lines, and that no GitHub access now shows a muted line instead of no section, in `docs/user-guide/reviewing-changes.md` § Pull requests
- [x] T036 [US4] Visual pass (quickstart §B step 5); record evidence in `specs/1003-review-worktree-prs/`. **Moved out of M4 to the close unit, beside T014, T021, T027 and T037**: no `gh` auth and no `xdotool` in the milestone container (`visual-pass.md`)

## Polish (no milestone)

No code changes remain after T036: recording quickstart Part A and Part B results in `quickstart.md` and ticking tasks is the close unit's work, in the close PR.

- [x] T037 (with T014, moved here from M1) Record the quickstart Part A and Part B results in `specs/1003-review-worktree-prs/quickstart.md`

## Dependencies & Execution Order

Setup and Foundational (T001–T008) block every story. US1 needs them. US1 scenario 3 (check status, T015–T021) needs T001–T014. US2 needs all of US1 (the list across branches and the check marks the header shows). US3 needs US1 (set per read) and US2 (`pr_header`). US4 needs US1 (the list read); it is ordered last because it rewrites the section drawing the earlier milestones extend. Within each story, a test task precedes the implementation it covers.

## Parallel Opportunities

- T001 with T002 (fixtures beside the branch-set unit tests).
- T009 with T010 once T008 is done; T015 with T017.
- T022 with T024; T028 with T029; T032 alone.

## Parallel Example: User Story 1

```text
T009 reducer tests (tests/features_pr_changes.rs)
T010 shell tests (src/main_tests.rs)
```

## Implementation Strategy

MVP is M1: the issue's missing pull requests (earlier branches, merged-and-deleted heads) are listed, and each is already selectable through 775's rows, so its diff is reachable. M2 adds the check mark to every entry. M3 adds previous/next and the header, and pins the refresh and selection behaviour with tests. M4 replaces the absent section with explicit loading, empty and unavailable lines. The list read stays a separate effect from the working-tree read at every step (FR-013).

US1 is split (milestones.md rule 3: 21 tasks) along its acceptance scenarios: scenarios 1, 2, 4 (which pull requests) in M1, scenario 3 (check status) in M2. US3 has no milestone of its own: it adds no code, only tests pinning behaviour that M1 (the set is recomputed per read) and M3's header deliver, so a milestone of it alone would have no deliverable; its tasks ride in M3, after the header they test.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Every pull request of the worktree is listed 🎯 MVP

- **Tasks**: T001–T013 (T014 moved to the close unit) · **M1 PR**: #1094
- **Deliverable**: with "Show pull request status" on, Review changes of a worktree lists the pull requests of its current branch and of every branch created in it (deleted ones included, the default branch and branches only checked out there excluded), newest first, each with number, title and state; selecting any of them shows its own diff (775) under a base line naming its own head branch
- **Satisfies**: US1 acceptance scenarios 1, 2, 4; FR-001, FR-002, FR-004, FR-013, FR-014; SC-001, SC-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test worktree_branch_set --test pull_request_list`; `scripts/build-lock.sh cargo test -p micold-core --lib worktree_branches`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes --test pr_status_is_read_only_on_named_events`; `scripts/build-lock.sh cargo test -p micold-client pr_list_branch_set`; quickstart §B steps 1, 2, 6 in the close unit (T014)
- **Depends on**: —
- **Tier**: full

### M2 — Check status on every entry

- **Tasks**: T015–T020 (T021 moved to the close unit) · **M2 PR**: #1097
- **Deliverable**: every listed pull request, merged and closed included, shows its last commit's check mark (passing, failing, pending, or none) with the same marks the sidebar uses, from one shared mapping
- **Satisfies**: US1 acceptance scenario 3; FR-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-core --test pull_request_list`; `scripts/build-lock.sh cargo test -p micold-client --lib -- pull_request_row`; `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes`; quickstart §B step 2 (check marks) in the close unit (T021)
- **Depends on**: M1
- **Tier**: full

### M3 — Previous/next and a header naming the shown pull request

- **Tasks**: T022–T031 (T027 moved to the close unit) · **M3 PR**: #1100
- **Deliverable**: while a pull request is shown, a header names it with its state and checks, previous/next step through the list (disabled at the ends), only the last choice's diff is ever shown, and the selection and header hold across refreshes, including a pull request from a branch created after the view opened
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenarios 1–4; FR-005, FR-006, FR-007, FR-008, FR-009, FR-016; SC-002, SC-003, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes --test icons`; `scripts/build-lock.sh cargo test -p micold-client pr_list_refresh_branch_set`; quickstart §B steps 3, 4 in the close unit (T027)
- **Depends on**: M2
- **Tier**: full

### M4 — Loading, empty and unavailable lines

- **Tasks**: T032–T036 (T036 moved to the close unit) · **M4 PR**: #1103
- **Deliverable**: the Pull requests section shows "Reading pull requests…" until the first read, "No pull requests from this worktree yet" for a worktree without any, and the muted "Pull requests can't be shown for this worktree" without GitHub access and no list read, with no notice; a failed refresh after a list keeps the list and the selection
- **Satisfies**: US4 acceptance scenarios 1–3; FR-010, FR-011, FR-012, FR-015; SC-005, SC-006
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes`; quickstart §B step 5
- **Depends on**: M3 (it rewrites the section drawing M2 and M3 extend)
- **Tier**: full

## Phase 7: TDD remediation

Source: `tdd/verification.md` at `c226a7d3`, verdict **FAIL**. The feature is not done until T038-T040 are cleared (findings 1-3, all HIGH).

- [x] T038 [US1] Finding 1: make `a_check_mark_keeps_the_row_height_and_is_drawn` (`crates/micold-client/src/ui/material/pull_request_row.rs:138`) fail when the mark is not drawn (compare the row with and without `.checks(..)`, or assert on the indicator), then show it red against a no-op `.checks()` before restoring. Done when `scripts/build-lock.sh cargo test -p micold-client --lib -- pull_request_row` is green and the no-op run is recorded red in `tdd/cycle-log.md`
- [x] T039 [US2] Finding 2: pin what the view draws, or record the passes that stand in for it. Either add view-level tests for the `PRS_EMPTY`/`PRS_UNAVAILABLE`/`PRS_LOADING` lines (`ui/changes.rs:48-52`), the header contents, the check mark on every row, and previous/next disabled where `adjacent(..)` is `None` (`ui/changes.rs:441`), in the pattern of `support::layout`; or run the `visual-pass` skill for T014, T021, T027, T036 and record results under T037 in `quickstart.md`. Done when `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes` is green and each of the five criteria in finding 2 names a test or a recorded pass — done: the five criteria are covered by the recorded visual pass (`visual-pass.md`, close unit) plus the view-model tests, except the drawn "Reading pull requests…" line, which neither a test nor the pass shows (recorded as unverified)
- [x] T040 [US1] Finding 3: re-establish red for T015, T017, T022, T024 after the fact. For each, break the implementation in a throwaway worktree (for example `check_mark` returning `None`, `adjacent` returning `None`, `Icon::ChevronLeft` mapped to another glyph, `reduce_checks` dropped from `parse_list`), run the named test, paste the failing output into `tdd/cycle-log.md`, restore. Done when each of the four has a red line in `cycle-log.md` and `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes --test icons` and `-p micold-core --test pull_request_list` are green
- [x] T041 Finding 4: add `specs/1003-review-worktree-prs/tdd/test-list.md` (behavior ids, `traces`, states) from tasks.md T002-T037, or state in the cycle log that tasks.md is the list and keep it current. Done when `/speckit.tdd.verify` no longer reports a missing list — done: tasks.md is the test list (stated in `tdd/cycle-log.md`)
- [x] T042 Finding 5: record a full green gate at the close commit. Free disk first (`mise run sweep`), then `mise run gate`. Done when it exits 0 and the counts go in `quickstart.md` Part A
- [x] T043 Test strength unmeasured: with 15 GB or more free, run the eight deliberate mutants listed in `tdd/verification.md` "Mutation results" (or `cargo mutants` scoped to `worktree_branches.rs`, `pull_request.rs`, `features/changes.rs`, `pull_request_row.rs` if installed), restore each and rerun green. Done when every survivor is triaged in a refreshed `verification.md`
- [x] T044 Finding 6: merge `the_selected_pull_request_leaving_the_list_removes_the_header` (`tests/features_pr_changes.rs:1073`) into `a_pull_request_that_leaves_the_list_returns_the_view_to_the_working_tree` (`:575`). Done when `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes` is green with one test fewer
- [x] T045 Findings 7-12 (MED/LOW): make the query tests assert shape not substrings (`pull_request_list.rs:169-187,290`), drop the direct field write (`features_pr_changes.rs:298`), compare `header.check` with a literal mark (`:946,1067`), label the asserts at `worktree_branches.rs:342`, split `:888` of `features_pr_changes.rs`, share one `git_at` (`worktree_branch_set.rs:21`, `main_tests.rs:10550`), and decide whether `pull_request_list.rs:262` should test the fake. Done when `mise run test-core` and the client test above are green — done for the items worth doing; not done, with reasons: direct field write at `features_pr_changes.rs` (state unreachable through `update`), shared `git_at` (two crates, would need a cross-crate helper), `pull_request_list.rs` fake test (client tests depend on the fake)

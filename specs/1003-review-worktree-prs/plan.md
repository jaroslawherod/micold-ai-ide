# Implementation Plan: Review changes reaches every pull request of the worktree

**Branch**: `claude/project-thread-c3olht` | **Date**: 2026-10-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1003-review-worktree-prs/spec.md`

## Summary

The Changes pane's **Pull requests** section (feature 775) lists the pull requests of every branch
of the worktree's branch set, not only its current branch. The set is derived on each list read
from the worktree's own git reflogs, read locally with the Changes view's existing `Git` (research
R1, R2). One GraphQL request per 25 branches reads every pull request of the set with its check
counts (R3, R4). The reducer gains previous/next, a header naming the shown pull request, and an
explicit section state for loading, empty and unavailable (R5, R6). No daemon, protocol or on-disk
change.

## Technical Context

**Language/Version**: Rust stable, iced GUI.

**Primary Dependencies**: existing only: `micold-core` (`pull_request`, `git`, new
`worktree_branches`), `micold-client` (`features/changes`, `shell/changes`, `shell/pr_status`,
`ui/changes`, `ui/material`), `gh` through `GhCli`, local `git`. No new crate.

**Storage**: none (040 FR-032). The branch set is recomputed per read, never kept.

**Testing**: `cargo test` via `scripts/gate.sh` / `mise run gate`; core integration tests with real
git in temp repos (`tests/support/attach_fixture.rs` helpers) and `tests/fixtures/gh/`; client
reducer tests `tests/features_pr_changes.rs`; shell tests in `main_tests.rs`; quickstart §B.

**Target Platform**: Linux, macOS, Windows; git and `gh` run as argument vectors, no shell.

**Project Type**: desktop app (Rust workspace).

**Performance Goals**: the working-tree list renders before the PR read (FR-013): the branch set
and GitHub reads are one separate effect, as 775. Two local git runs plus one `gh` run per 25
branches per refresh.

**Constraints**: offline-capable; `gh` bounded at 10 s per run, no retry; GitHub cost about 50
points per 25-branch request (R3).

**Scale/Scope**: any number of branches (chunks of 25, none dropped); up to 100 pull requests per
branch (775's page).

## Constitution Check

- [x] **I. Test-First**: branch-set rules, query/parse, ordering, section state, stepping and
  header are pure (`micold-core`, render-free `features/changes.rs`) and test-first. Only
  `ui/changes.rs` drawing and the `shell/` spawn wiring use the quickstart exception.
- [x] **II. Multi-Session**: the list and selection stay per (project, entry) (775); sessions of one
  worktree share it; nothing crosses entries (FR-014 tested with two worktrees).
- [x] **III. Worktree**: the set is read in the entry's own directory, read-only; the project root
  shows no section. No worktree is created or changed.
- [x] **IV. Local-First**: nothing stored; offline gives the muted line or the last list. Only the
  opt-in setting sends owner, repository and branch names (040 FR-031).
- [x] **V. Rust + iced**: yes; `PrSection` and `Step` enums make the section and stepping states
  explicit.
- [x] **VI. Cross-Platform**: git/gh argument vectors; `--date=unix` output parses the same on all
  three; CI runs all three.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` § Pull requests (branch set,
  check marks, previous/next, empty and unavailable lines).
- [x] **VIII. Reusable UI**: `PullRequestRow` gains a `.checks(..)` builder method; the header
  composes `PullRequestIndicator`, `Text` and `IconButton`; no new private widget. Two icon glyphs
  are added to the shared `Icon` set.

No violation: Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002 | `worktree_branches::{parse_reflog, candidates, branch_set, read_branch_set}`, `Git::reflogs`; called from `shell/pr_status.rs::on_list_remotes` with `shell/changes.rs::local` |
| FR-003 | `BranchPullRequest.checks` via `reduce_checks`; `check_mark` mapping shared with the sidebar; `PullRequestRow::checks` |
| FR-004 | `list_branches` merge + sort (created desc, number desc) |
| FR-005 | unchanged 775 `SourceChosen` / diff read |
| FR-006 | `Msg::PullRequestStepped`, `adjacent` |
| FR-007 | `pr_header`, `base_line` with `head_branch` |
| FR-008, FR-009 | unchanged 775 `remembered` / triggers; set recomputed per read |
| FR-010, FR-011, FR-012, FR-015 | `PrSection`, `section`; `Err(Unavailable)` keeps `Unavailable { last }` |
| FR-013 | `list_branches` chunks of 25, whole-or-failure; separate effect as 775 |
| FR-014 | branch-set rule 3 (created elsewhere dropped), forks and default branch dropped |
| FR-016 | 775 `seq` + number guard on `PullRequestDiffRead`; pinned by a test |

## Test strategy

| Requirement | Layer and test |
|---|---|
| FR-002 rules | core unit: `worktree_branches` on reflog text (checkout, rename, worktree-add start, created elsewhere, remote copy by `switch <remote-branch>` and `switch -c <name> origin/<name>` (dropped), `switch -c x origin/main` (kept), deleted, expired creation, ±2 s, commit-named checkouts) |
| FR-001, FR-002, FR-014, SC-001 | core integration `tests/worktree_branch_set.rs`, real git: one worktree on A then B then C (A deleted), a branch created in the main checkout and checked out in the worktree (excluded), a `git switch <name>` of a remote-only branch (excluded), a second worktree (disjoint set) — Isolation gate |
| FR-003, FR-004, FR-013, SC-007 | core `tests/pull_request_list.rs`: multi-alias fixture, checks per state, default branch dropped, tie by number, 26 branches = 2 requests, second chunk failing fails all |
| FR-006, FR-007, FR-016, SC-002, SC-003 | reducer `features_pr_changes.rs`: three PRs with distinct diffs, select 2nd then 3rd, next/previous incl. ends, header fields, three fast choices show only the last |
| FR-008, FR-009, SC-004 | reducer: new PR at top keeps selection; state/check change updates header |
| FR-010, FR-011, FR-012, FR-015, SC-005, SC-006 | reducer: `section` for each table row; `Err(Unavailable)` with and without a last list, `view.source` kept with a last list and reset to `Changes` without one; no notice |
| shell | `main_tests.rs`: `ReadPullRequests` reads the set with `FakeGit` reflogs and calls `list_branches` with it; no local git → current branch only |
| Look | quickstart §B visual pass (`visual-pass` skill) |

## Project Structure

### Documentation (this feature)

```text
specs/1003-review-worktree-prs/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/pull-request-list.md   # git reflog reads and the multi-branch GraphQL read
├── contracts/changes-view.md        # reducer messages, section states, header
└── tasks.md                         # /speckit-tasks
```

### Source Code

```text
crates/micold-core/src/
├── worktree_branches.rs   # NEW: parse_reflog, candidates, branch_set, read_branch_set
├── lib.rs                 # + mod worktree_branches
├── git.rs                 # + Git::reflogs (GitCli, FakeGit)
└── pull_request.rs        # BranchPullRequest + head_branch, checks; list_query(n), list_args(repo, branches), parse_list, list_branches (replaces list_branch); Fake
├── github.rs              # GhCli::list_branches
crates/micold-core/tests/  # worktree_branch_set.rs (NEW), pull_request_list.rs, fixtures/gh/pr_list_*.txt
crates/micold-client/src/
├── features/changes.rs    # Effect::ReadPullRequests{entry,current}, Step, PullRequestStepped, adjacent, PrSection, section, pr_header, check_mark; Err(Unavailable) keeps list
├── shell/changes.rs       # passes entry + current; exposes `local` to pr_status
├── shell/pr_status.rs     # on_list_remotes: read_branch_set then list_branches
├── ui/changes.rs          # section states, scrollable list, header with previous/next
├── ui/sidebar.rs          # uses the shared check_mark
├── ui/material/pull_request_row.rs  # + .checks(..)
├── icons.rs               # + ChevronLeft, ChevronRight
├── showcase/sections/review.rs     # rows with check marks
crates/micold-client/tests/features_pr_changes.rs; src/main_tests.rs (BlockingSource: list_branches)
docs/user-guide/reviewing-changes.md
```

**Structure Decision**: logic in `micold-core` (branch set, query, parse) and the render-free
`features/changes.rs`; `shell/` only spawns reads, `ui/` only draws.

## Complexity Tracking

None.

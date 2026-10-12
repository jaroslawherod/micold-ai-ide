# Implementation Plan: Diff view shows PRs created within sessions and worktrees

**Branch**: `feat/775_diff-view-should-show-prs-created-within-sessions` | **Date**: 2026-10-12 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/775-diff-view-session-prs/spec.md`

## Summary

The Changes pane (`micold-client` `features/changes.rs`, feature 482) gains a "Pull requests"
section above its file list: every pull request whose head branch is the selected entry's branch,
newest first, each with number, title and state (open, draft, merged, closed). Selecting one swaps
the file list and diff for that PR's base...head files, read once from GitHub as a single unified
diff and split per file in `micold-core`; selecting "Working tree" returns. The list rides the
existing PR-status machinery (feature 040): same setting, `gh` discovery, remote choice, refresh
triggers and 600 s stale threshold, with one new read per branch through the existing
`PullRequestSource` seam. No daemon, protocol or on-disk change.

## Technical Context

**Language/Version**: Rust stable (mise-managed), iced GUI.

**Primary Dependencies**: existing only: `micold-core` (`pull_request`, `github`, `review::diff`),
`micold-client` (`features/changes`, `shell/changes`, `shell/pr_status`, `ui/changes`, `ui/material`),
`gh` CLI through `GhCli`. No new crate.

**Storage**: none. The list, the selection and the PR diff live in the window's memory only; titles
and addresses are never logged or serialised (as `PullRequestStatus`; `pull_request_is_never_stored.rs`).

**Testing**: `cargo test` via `mise run test-core` / `mise run gate`. Core unit and integration with
`tests/fixtures/gh/` fixtures and `FakePullRequestSource`; client reducer tests
(`tests/features_changes.rs`); shell-level tests in `main_tests.rs`; quickstart §B visual pass.

**Target Platform**: Linux, macOS, Windows. `gh` found by the existing `locate_gh`; no platform branch.

**Project Type**: desktop app (Rust workspace).

**Performance Goals**: working-tree list renders before any PR read returns (SC-003): the PR read is
a separate effect, never awaited by the list read.

**Constraints**: offline-capable (the section is simply absent); one `gh` run bounded at 10 s like
the status read; no retry inside a read.

**Scale/Scope**: up to 100 PRs per branch listed (GraphQL page limit); a PR diff above GitHub's
300-file / 20 000-line limit is reported, not shown.

## Constitution Check

- [x] **I. Test-First**: every task pairs a failing test first; decision logic (query, parse, diff
  split, reducer) lands in `micold-core` and the render-free `features/changes.rs`. Only `ui/changes.rs`
  rendering and the `shell/` spawn wiring rely on the quickstart exception.
- [x] **II. Multi-Session**: the list is keyed by (project, branch) and shared by every session of
  the worktree (one read serves them); the selected PR is remembered per (project, entry), so no
  state crosses entries (FR-010, FR-013).
- [x] **III. Worktree**: the branch comes from the listed worktree (`worktree.branch`); Default
  (project root) and detached HEAD have no branch of their own and show no section. Nothing creates
  or modifies a worktree.
- [x] **IV. Local-First**: nothing stored; works offline (section absent or last list kept). The
  network read is the already opt-in `github.pr_status_enabled` setting (default off) and sends only
  owner, repository name, branch name and PR number (as feature 040 FR-031).
- [x] **V. Rust + iced**: yes. A selected PR is an enum (`Source::{Changes, PullRequest}`), so a PR
  view cannot hold a working-tree list.
- [x] **VI. Cross-Platform**: `gh` args built as `Vec<String>`, no shell; paths from a diff are
  `RelPath`; CI runs all three platforms.
- [x] **VII. Documentation**: `docs/user-guide/reviewing-changes.md` (new section),
  `worktrees-and-sessions.md` (pointer from "Pull request status"), `settings.md` (what the toggle
  now also controls).
- [x] **VIII. Reusable UI**: PR rows reuse `PullRequestIndicator`/`PrMark` for state and the Changes
  list's row/virtual-row primitives; any new row type is a builder struct in `ui/material`
  (`PullRequestRow::new(..).selected(..).into()`), not a private widget. Re-checked after design:
  PASS.

No violation: Complexity Tracking is empty.

## Requirement map

| FR | Where |
|---|---|
| FR-001, FR-002, FR-010, FR-011 | `pull_request::BranchPullRequest`, `list_query`/`list_args`, `PullRequestSource::list_branch` (contracts/pull-request-reads.md §1); branch from `entry` worktree in `shell/changes.rs` |
| FR-003 | `BranchPrState` (new core enum: Open/Draft/Merged/Closed, no check status); one mapping `BranchPrState -> material::PrMark` used by `ui/changes.rs` |
| FR-004 | `PullRequestSource::diff` (§2), `review/pr_diff.rs::split_pr_diff`, `Source::PullRequest` in `OpenView` (data-model §2) |
| FR-005, FR-006, FR-013 | `Msg::SourceChosen`, `Source`, `State.remembered` (data-model §2, §3) |
| FR-007, FR-012 | `shell/pr_status.rs` fires `Msg::PullRequestsStale`/refresh on the status triggers; `PrList.read_at` with `is_stale` |
| FR-008, FR-009, FR-014 | read failure maps to `PrList::Unavailable`, no `notice`; separate effect; gated on `app.core.pr_status.enabled` |

## Test strategy

| Requirement | Layer and test |
|---|---|
| FR-001, FR-002, FR-011, FR-003 | core: `tests/pull_request_list.rs` (fixtures `gh/pr_list_*.txt`: open+merged+closed+draft, fork dropped, ordering, unknown state) |
| FR-004 | core: `tests/pr_diff_split.rs` (rename, add, delete, binary, large, quoted path); client reducer `features_pr_changes.rs` (select, no extra read per file) |
| FR-005, FR-006, FR-013 | client reducer: `features_pr_changes.rs` (return to working tree, per-entry memory, vanished PR returns) |
| FR-007, SC-004 | shell test in `main_tests.rs`: status trigger while a view is open requests the list; `pr_status_is_read_only_on_named_events.rs` extended |
| FR-008, FR-012, SC-005 | reducer: failure keeps list, raises no notice; stale marked via `is_stale`; shell: no `gh`/no remote gives `Absent` |
| FR-009, SC-003 | reducer: `Opened` returns `ReadList` before/independent of `ReadPullRequests`; list renders with `PrList::Loading` |
| FR-010 | integration (Principle II gate): two projects, same branch name, each reads only its own repo via `FakePullRequestSource` calls |
| FR-014 | reducer + shell: setting off gives `Absent`, no `ReadPullRequests` |
| Look of the section | quickstart §B visual pass. No geometry gate: the Changes pane has none and the section is plain rows (decided here; revisit in tasks if rows go virtual). |

## Project Structure

### Documentation (this feature)

```text
specs/775-diff-view-session-prs/
├── plan.md  research.md  data-model.md  quickstart.md
├── contracts/pull-request-reads.md   # gh requests and what is read from them
├── contracts/changes-view.md         # reducer messages, effects, UI
└── tasks.md                          # /speckit-tasks, not this command
```

### Source Code

```text
crates/micold-core/src/
├── pull_request.rs        # + BranchPullRequest, list_query, list_args, parse_list, PullRequestSource::{list_branch, diff}, Fake support
├── github.rs              # + GhCli impl of the two methods; diff_args
└── review/
    ├── pr_diff.rs         # NEW: split_pr_diff(raw) -> PrDiff; PrFile
    └── mod.rs             # + mod pr_diff
crates/micold-core/tests/  # pull_request_list.rs, pr_diff_split.rs, fixtures/gh/pr_list_*.txt, pr_diff_*.txt
crates/micold-client/src/
├── features/changes.rs    # + Source, PrList, PrDiffLoad, Msg/Effect variants, State.remembered
├── shell/changes.rs       # + hands ReadPullRequests / ReadPullRequestDiff effects to shell/pr_status.rs
├── shell/pr_status.rs     # + the ONE place that calls `(tooling.pull_requests)(gh)` (one factory call, then `read`, `list_branch` and `diff` on the returned source; no second call line): new `list_branch`/`diff` runs live here, and it requests the list on the status triggers while a view is open
├── ui/changes.rs          # + pull_request_section
├── ui/material/           # NEW PullRequestRow builder, promoted from the sidebar-private row_pull_request and added to showcase sections/review.rs
crates/micold-client/tests/features_changes.rs, features_pr_changes.rs (NEW); main_tests.rs `BlockingSource` implements the new trait methods
crates/micold-core/src/session.rs  # SessionLocation derives PartialOrd, Ord (key of `remembered`)
crates/micold-client/src/features/pr_status.rs # `paused` made `pub` for the rate-limit hold
docs/user-guide/reviewing-changes.md, worktrees-and-sessions.md, settings.md
```

**Structure Decision**: all logic in `micold-core` (query, parse, split) and the render-free
`features/changes.rs` reducer; `shell/` only spawns the reads, `ui/` only draws.

## Complexity Tracking

None.

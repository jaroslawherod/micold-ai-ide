# Autopilot ledger — #775 diff-view-session-prs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #775 "Diff view should show PRs created within sessions and worktrees"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #775
- **Worktree branch**: feat/775_diff-view-should-show-prs-created-within-sessions
- **Started**: 2026-10-12
- **Phase**: tasks
- **Next step**: design PR merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #782 | Design PR (spec, plan, tasks) | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | PRs of the entry's branch listed in the Changes pane | — | pending |
| M2 | T016–T026, T023b | full | Select a PR, read its base...head diff, return | — | pending |
| M3 | T027–T033 | full | List refreshes on status triggers, stale marked | — | pending |

## Decisions

- Clarify round 1: deleted-head PR diff is shown from GitHub (agent-resolved, FR-004). No escalation.
- Tasks: M1 (15 tasks) and M2 (11) exceed the ~10-task guide but have no split along an acceptance scenario that leaves a user-observable deliverable (state marks, selection and return are one flow); kept whole. M3 follows M2 only to avoid conflicts in `features/changes.rs`.
- Tasks: analyze M-1..M-6 fixed (T034 left to the close unit, rate-limit hold in T012, two-sessions test in T007, off-to-on in T006, too-large edge in spec, Verify commands); plan Constitution II wording made "read per (project, branch), held by the entry's view".

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 31e1f276:13165b3a (orig) | CHANGES: 2 MAJOR (US2 s3 untestable; PR status setting unspecified), 3 MINOR; all fixed |
| Spec | 2 | 33713142e3fdd0cdca9affa9cea75fea057f1713:1206eb1f207ab7e71834248b5788b51af061713b | CLEAN (1 MINOR fixed) |
| Plan | 1 | 9542cb17a47a5cde8de0d120e2391d023bbabb49:4055448678d6361c09ed6bf9f33d1e0188781e25 | CHANGES: 4 MAJOR (PrState/BranchPrState mismatch, no test map, single call-site guard test, rate-limit hold), 3 MINOR; all fixed |
| Plan | 2 | 44a2dbea1d4c763aa859c8ae3c982ed7bf415968:4055448678d6361c09ed6bf9f33d1e0188781e25 | CLEAN (2 MINOR fixed, prose only) |
| Tasks | 1 | f299e17b1b36488e83cceece626596063613b3d7:a3576aeee9ba5abbc3ef2b89553f242f5bed2ac7 | CHANGES: 2 MAJOR (PullRequestRow premise, showcase paths), 3 MINOR; all fixed, wording and task split only, checked against the repo |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M1 handed over at ~150k context (design PR #782 merged as 0de0b3ce; branch reset to origin/main).
Done and green: T001-T005 (core: `BranchPullRequest`, `list_query/list_args/parse_list/read_list_outcome`,
`PullRequestSource::list_branch`, fake, `GhCli`, fixtures `pr_list_*.txt`, `tests/pull_request_list.rs`,
guards amended; `.diff(` guard pin left to M2 since there is no `diff` method yet).
Written, NOT yet compiled or run (the first client build waited on another worktree's build lock):
T006 `tests/features_pr_changes.rs`; T008/T010 `features/changes.rs` (`PrList`, `pr_mark`,
`OpenView.{branch,pull_requests}`, `State.pr_enabled`, `Msg::{PullRequestsWanted,PullRequestsRead,
PullRequestSettingChanged}`, `Effect::ReadPullRequests`); T009/T011 `ui/material/pull_request_row.rs`
+ catalogue entry + `showcase/sections/review.rs::pull_request_row`; T012 shell wiring
(`shell/pr_status.rs::{list_requested,on_list_remotes}`, `PendingOp::PrListRemotes` in
`daemon_sync.rs` x4 arms, `shell/changes.rs` run_pending sends `PullRequestsWanted{branch}` after the
list effect, rate limit sets `pr_status.pause_until` in `shell/changes.rs::update`, `enabled_changed`
forwards `PullRequestSettingChanged`); T013 `ui/changes.rs::pull_request_section`; T014 user guide
(3 files). Tests for T007 (`pr_list_read_*` in `main_tests.rs`, harness `holding()`/`app_over`/
`FAKE_GH`/`BlockingSource` near line 8884) are NOT written; T015 visual pass not run.
Next step: `scripts/build-lock.sh cargo test -p micold-client --test features_pr_changes --test
pr_status_is_read_only_on_named_events`, fix compile errors in the unverified code (cargo fmt too),
write T007, run `mise run test-core` + the Verify commands, tick T006-T015, visual pass, then
review A (high) + B, full gate, PR "feat(775): list session PRs in the Changes pane (#775)".
Honest TDD note: T006 tests were written beside the reducer code, not shown red first.

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

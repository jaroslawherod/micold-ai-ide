# Autopilot ledger — #837 issue-relationship-metadata

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #837 "Show all GitHub issue metadata (blocked by, blocking, parent, sub-issues…) when a worktree is created"; labels: enhancement, effort:high, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: high
- **Issue**: #837
- **Worktree branch**: claude/project-thread-tw6sph
- **Started**: 2026-10-16
- **Phase**: clarify
- **Next step**: clarify unit: apply the user's answer to the open escalation (US2 scenario 6), then run the next clarify round

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 939e5694d83a882c00de3c9ff98235ef44a95c5d:8a04f4206330fa7552531e4694e2244b186d345f | CLEAN: 3 MINOR, all fixed (F1, F3 prose only; F2 added FR-011/SC-005 tool-call bound, no new round needed after CLEAN) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product/scope decision the repo does not settle). Question: After a
worktree is created from a picked GitHub issue, should the issue's relationships (blocked by,
parent, sub-issues, linked PRs...) stay visible on that worktree in the sidebar, or is showing them
in the New worktree form before Create enough? (spec US2 scenario 6)
- A (Recommended): Form only. The issue's motivation ("see that an issue is blocked before starting
  work") is met at pick time; the sidebar holds no issue data today (its issue tag is parsed from
  the branch name, docs/user-guide/worktrees-and-sessions.md:103), and spec Assumptions put
  "keeping details fresh after creation" out of scope.
- B: Also on the sidebar row: a "blocked" marker on the created worktree's row, with the details
  in its tooltip; needs storing the details per worktree and decides when they go stale.
- C: Also on the sidebar row and refreshed: as B, re-read from GitHub periodically or on demand.

## Follow-ups not done

None.

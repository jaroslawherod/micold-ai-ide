# Autopilot ledger — #483 parallel-agent-runs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #483 "Run one prompt across several agents in parallel worktrees and pick the best result" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #483
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-08
- **Phase**: spec
- **Next step**: clarify unit: apply the user's answers to the two Open escalation questions, then run clarify round 2

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
| Spec review | 1 | db71250366df0d22ca0ef7ddd4040b1d4304ac11:88c3a363b2acf0a05c3a9ad40b2cb2d052a27229 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec review | 2 | 8c96629cfa543d92ed3e94d0ecb58df2b625fe8a:4a403ba28452cbfef1e57944050a01b8beb435f0 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product decision the repo does not settle; the app has no git
commit, merge or rebase code today: grep of crates/ and docs/user-guide finds none).

1. US4 scenario 9: the picked run has uncommitted changes. Options: (A, Recommended) refuse the
   pick, naming the uncommitted files, until the user commits them in the run's session or
   terminal; (B) offer to commit them as part of the pick; (C) integrate only committed work and
   warn.
2. FR-012: how the pick integrates. Options: (A, Recommended) always merge: fast-forward when the
   base has not moved, otherwise a merge commit; the run's branch is never rewritten; (B) always
   rebase the run onto the base, then fast-forward; (C) the user chooses merge or rebase in the
   pick dialog, merge by default.

## Follow-ups not done

None yet.

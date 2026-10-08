# Autopilot ledger — #483 parallel-agent-runs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #483 "Run one prompt across several agents in parallel worktrees and pick the best result" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #483
- **Worktree branch**: claude/project-thread-wysm57
- **Started**: 2026-10-08
- **Phase**: plan
- **Next step**: plan unit, continuing the handover below (plan review round 1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- Clarify US4 scenario 9: a picked run with uncommitted changes is refused, naming the files;
  nothing changes. _(default, pending user confirmation)_
- Clarify FR-012: a pick always merges (fast-forward when possible, else a merge commit); the run's
  branch is never rewritten. _(default, pending user confirmation)_
- Clarify round 2: no critical ambiguities left; clarify done.
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

Plan unit, handing over at the context cap right after writing the artifacts.

- Done: `branch-start.sh` rebased 4 unmerged commits onto `origin/main`; `setup-plan.sh` run;
  `specs/483-parallel-agent-runs/{plan,research,data-model,quickstart}.md` and
  `contracts/{run-group-wire,integration,parallel-surfaces}.md` written and committed.
- Next step: plan review round 1 — dispatch a fresh reviewer per `tasks/review.md` with
  `rubrics/plan.md` (round 1 of a feature artifact review: omit `model`), paths
  `specs/483-parallel-agent-runs/{plan,research,data-model,quickstart}.md`, its three contracts,
  `spec.md` and `.specify/memory/constitution.md`. Record the `review-snapshot.sh` pair in
  *Review rounds* before the round. Then act on it per `tasks/review-rounds.md`, commit, and set
  **Next step** to `tasks unit`. No PR: the plan ships in the design PR.
- Open findings: none yet. Review rounds for the plan: none counted yet.
- Known design tension to tell the reviewer is deliberate, recorded in research R7: FR-009 counts
  against the group's base branch while FR-010 equates them with the Changes view's totals, which
  use the merge-base with the *default* branch; they agree when the group base is the default
  branch (the dialog default), and research R7 chooses FR-009 with a spec follow-up noted.

## Open escalation

None.

## Follow-ups not done

None yet.

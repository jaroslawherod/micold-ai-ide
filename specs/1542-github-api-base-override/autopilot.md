# Autopilot ledger — #1542 github-api-base-override

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1542: Base-URL override for the GitHub API and git host so the scoped-token checks run against a local fake; labels: none (orchestrator added in-progress, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1542
- **Worktree branch**: feat/1542_base-url-override-for-the-github-api-and-git-host
- **Started**: 2026-11-06
- **Phase**: tasks
- **Next step**: design PR opened; wait for merge, then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | ApiBase parameter, fake GitHub API, tests for Part C steps 2, 3, 8, 10, 12 | | pending |
| M2 | T010–T011 | full | Runtime-gated container/kind test (steps 4-8, 10, 11) | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Plan | 1 | 55f713a4:7bd55840 | CHANGES: 1 MAJOR (test targets unregistered), 3 MINOR; fixed (Cargo.toml [[test]], .gitattributes, FR-009 row, git backend R6), structural doc edits, self-checked |
| Spec | 1 | 3ce1418f:c695061be | CHANGES: 3 MAJOR; fixed (spec reworded, FR-004 kept open, SC counts, checklist), prose only |
| Tasks | 1 | a08fafa1:91e6af16 | CHANGES: 1 MAJOR (T006 [[test]] before file), 2 MINOR; fixed (registration folded into T007, dr_case, transport, checklist text), self-checked |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

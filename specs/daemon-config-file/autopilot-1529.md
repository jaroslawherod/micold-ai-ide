# Autopilot ledger — #1529 daemon-config-file (Issue 8)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1529` (One GitHub token for many repositories); labels: `flow:feature`, `effort:high`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-007, FR-009, SC-002, `contracts/daemon-config.md#githubtokens`); the tasks unit only cuts milestones for the "Issue 8" group (T040–T045).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1529
- **Worktree branch**: feat/1529_one-github-token-for-many-repositories
- **Started**: 2026-11-04
- **Phase**: spec (skipped; next: tasks)
- **Next step**: tasks unit: cut milestones for T040–T045 in `specs/daemon-config-file/tasks.md`, open the design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1529? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-007/FR-009/SC-002. Skipped as for #1524/#1525/#1526/#1534. | agent-resolved | specs/daemon-config-file/autopilot-1525.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

# Autopilot ledger — #1530 daemon-config-file (Issue 9)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1530` (The daemon's credential helper chooses the token from the configuration); labels: `effort:high`, `flow:feature`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1529 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-008, FR-010, FR-016, FR-030, FR-031, FR-035, `contracts/daemon-config.md` sections `[git]` and `[[github.tokens]]`); the tasks unit only cuts milestones for the "Issue 9" group (T046–T054, T092–T094).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1530
- **Worktree branch**: feat/1530_the-daemon-s-credential-helper-chooses-the-token
- **Started**: 2026-11-04
- **Phase**: spec (skipped, design on main)
- **Next step**: tasks unit: cut milestones for Issue 9 (T046–T054, T092–T094)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1530? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-008/010/016/030/031/035. Skipped as for #1524/#1525/#1526/#1529/#1534. Blockers #1524 and #1529 are merged (#1529 via PR #1684). | agent-resolved | specs/daemon-config-file/autopilot-1529.md |

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

# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: plan
- **Next step**: tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| none yet | Design | - | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

- FR-014 (decided by user): removed daemon's worktrees kept, sessions stopped, shown as "no daemon" until rebound; FR-003 is "at most one".
- FR-016 (agent-resolved): names and branches unique across a project's daemons.
- Several daemons of one runtime allowed (agent-resolved: issue #491).

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1deadc93434025bb863375f513b4bce46ac46929:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CHANGES: 1 MAJOR, 3 MINOR (fixed) |
| Spec | 2 | 26c2ce0a99dd2b7b51f6ae0d78cedecb665d7169:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CLEAN (1 MINOR, fixed, prose only) |
| Plan | 1 | 45443d59c4d375cbe2310511d0aa4b7c45cc4658:fe509bbbdf367dfff39b8a00877b46d761e0bb60 | CHANGES: 3 MAJOR, 3 MINOR (fixed) |
| Plan | 2 | dd98153eb049be30b1f7ded35e6bab4d1b5cee26:fe509bbbdf367dfff39b8a00877b46d761e0bb60 | CLEAN (2 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.

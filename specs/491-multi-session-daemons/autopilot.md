# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: clarify
- **Next step**: clarify unit: resolve the 2 [NEEDS CLARIFICATION] markers (FR-014, FR-016) in spec.md

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| none yet | Design | - | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1deadc93434025bb863375f513b4bce46ac46929:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CHANGES: 1 MAJOR, 3 MINOR (fixed) |
| Spec | 2 | 26c2ce0a99dd2b7b51f6ae0d78cedecb665d7169:e9abd92efe2c7d8ad097ff82d4a7073477349524 | CLEAN (1 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1. FR-016 and same-runtime-daemons resolved from evidence (recorded in spec.md
Clarifications). Open for the user: FR-014, what happens to worktrees bound to a removed daemon.
Options: (A, recommended) kept, sessions stopped, shown unbound until user binds a daemon, nothing
deleted on disk; (B) user must pick a target daemon in the confirmation dialog; (C) deleted from the
app. Evidence: specs/014-forget-project (forgetting never deletes on disk, sessions stopped).

## Follow-ups not done

SSH (#687) and Kubernetes (#688) split out of #491; extension points only.

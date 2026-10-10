# Autopilot ledger — #491 multi-session-daemons

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #491 "Support multiple session daemons: host and container" (host and container runtimes only; SSH #687 and Kubernetes #688 out of scope); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #491
- **Worktree branch**: feat/491_support-multiple-session-daemons-host-container
- **Started**: 2026-10-09
- **Phase**: tasks
- **Next step**: tasks review round 1, close checklists, gate, open design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| none yet | Design | - | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Upgraded install runs on a one-daemon registry, worktrees bound and labelled, daemon choice persists | - | pending |
| M2 | T018–T029, T048–T049 | full | Host and container daemons concurrent and failure-isolated | - | pending |
| M3 | T030–T039 | full | Settings Daemons section with removal and rebind | - | pending |
| M4 | T040–T045 | full | Real-runtime multi-daemon e2e in CI | - | pending |

## Decisions

- FR-014 (decided by user): removed daemon's worktrees kept, sessions stopped, shown as "no daemon" until rebound; FR-003 is "at most one".
- FR-016 (agent-resolved): names and branches unique across a project's daemons.
- Several container daemons allowed (agent-resolved: issue #491); at most one host daemon (plan Complexity Tracking, R3); spec amended in the tasks unit to match (clarification, FR-001, edge case, assumption).
- Tasks unit: bindings and legacy default keyed by a stable `DaemonId` (analyze F1: name-keyed bindings rebound on remove-and-re-add); contracts, data model, plan updated. Unknown runtime kind is `DaemonRuntime::Unsupported`; `ProjectAdd` sent to the bound daemon on first bind and each connect (research risk resolved).
- M1 is split from the P1 stories along acceptance scenarios (rule 3): M1 carries US1 scenarios 2, 4 and US4, M2 US1 scenarios 1, 3 and US2; the P1 story has no smaller observable deliverable before the registry exists. Polish (T046, T047) is doc-only, left to the close unit.

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

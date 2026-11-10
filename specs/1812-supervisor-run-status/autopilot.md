# Autopilot ledger — #1812 supervisor-run-status

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot supervisor M1: machine-readable run status and non-blocking escalation, issue #1812 (see issue body); labels: enhancement, in-progress, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1812
- **Worktree branch**: feat/1812_autopilot-supervisor-m1-machine-readable-run
- **Started**: 2026-11-10
- **Phase**: tasks
- **Next step**: M1 (after design PR merges)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1819 | design (spec, plan, tasks) | open | |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014, T019 | full | run-status.sh and its per-state tests (US1 + US3) | | pending |
| M2 | T015–T018, T020–T024 | full | supervised escalation: supervised.sh, answer.sh, skill prose (US2) | | pending |

## Decisions

US3 (tests only) is folded into M1 (rule 7, tests travel with code); US2 is M2 (scripts + skill prose: not light, adds a ledger-writing script). No Polish milestone: no code left.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 2d70083d087364f8dc5908be94adf0ee0c21b4ef:e82cc8df6bddd76f87e2d0786098f9d4ceaf8b28 | CHANGES: 3 MAJOR |
| Spec | 2 | a6d59dc504310543054184cb26c5aa527d345d62:e82cc8df6bddd76f87e2d0786098f9d4ceaf8b28 | CLEAN (1 MINOR fixed) |
| Plan | 1 | cdc0fc6e30bf489c61a4a8f414e6dc44d4b1f749:dc817a78f2bc0a282455f5c21afbe7eb77fb7056 | CHANGES: 4 MAJOR, 3 MINOR (all fixed) |
| Plan | 2 | defe853eceacab7ba1241c8b27afdd856c54de92:dc817a78f2bc0a282455f5c21afbe7eb77fb7056 | CLEAN (1 MINOR fixed, prose only) |
| Tasks | 1 | ed4856e13049fb820ec0fa7c3c53ea472bc12c34:ebe5384e95fabe60e32205366e58241d7ad4414c | CLEAN (3 MINOR fixed, prose only; analyze HIGH C1-C3 fixed first) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

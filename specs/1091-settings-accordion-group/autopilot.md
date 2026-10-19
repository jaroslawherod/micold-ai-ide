# Autopilot ledger — #1091 settings-accordion-group

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1091 "Settings accordions should form a group: opening one closes the others" (see spec Input); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1091
- **Worktree branch**: feat/1091_settings-accordions-should-form-a-group-opening
- **Started**: 2026-10-19
- **Phase**: milestone M1 (review)
- **Next step**: M1 reviews, full gate, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1135 | Design | merged | f3c827a5f8ce6947d331a5bb9aadddc4f8b552e7 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Daemon form sections form a group; error-held sections stay open | | pending |
| M2 | T010–T012 | light | Showcase shows grouped and independent sets | | pending |

## Decisions

- Tasks unit: milestones.md rule 2 would make US3 its own milestone, but its rule (FR-006) is the same `State` change as US1 (rule 7), so US1+US3 are M1; US2 (showcase) is M2. Polish (quickstart §B) is left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1135 | Design | merged | f3c827a5f8ce6947d331a5bb9aadddc4f8b552e7 |
| Spec | 1 | 6c09689ef177f6b42e67ad2577e443830a87b4d1:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CHANGES: 1 MAJOR |
| Spec | 2 | 0bf2cd470ebf87d48ab4abff4222017bd50d4253:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CLEAN (1 MINOR fixed) |
| Plan | 1 | ba7c38f22d155fb504ddb33aa8737b70c5a640eb:83ef880756ad3299406803c3e66a8b5984a0018b | CLEAN (3 MINOR fixed, prose only) |
| Tasks | 1 | f14a22d473606b7e75e25f4b4e9bbd2a586ba061:933b6630274d782e6037b84d4a919f036546d3c9 | CHANGES: 1 MAJOR ([P] on shared file), fixed, prose only; 2 MINOR fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| #1135 | Design | merged | f3c827a5f8ce6947d331a5bb9aadddc4f8b552e7 |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

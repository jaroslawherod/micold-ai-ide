# Autopilot ledger — #1091 settings-accordion-group

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1091 "Settings accordions should form a group: opening one closes the others" (see spec Input); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1091
- **Worktree branch**: feat/1091_settings-accordions-should-form-a-group-opening
- **Started**: 2026-10-19
- **Phase**: milestone M2 (PR open)
- **Next step**: orchestrator merges M2 PR, then close unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1135 | Design | merged | f3c827a5f8ce6947d331a5bb9aadddc4f8b552e7 |
| #1140 | M1 | merged | 71fd52bdb46ec17805319fae33a6aca191d60092 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T009 | full | Daemon form sections form a group; error-held sections stay open | #1140 | merged |
| M2 | T010–T012 | light | Showcase shows grouped and independent sets | #1142 | in review |

## Decisions

- Tasks unit: milestones.md rule 2 would make US3 its own milestone, but its rule (FR-006) is the same `State` change as US1 (rule 7), so US1+US3 are M1; US2 (showcase) is M2. Polish (quickstart §B) is left to the close unit.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| #1135 | Design | open | |
| Spec | 1 | 6c09689ef177f6b42e67ad2577e443830a87b4d1:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CHANGES: 1 MAJOR |
| Spec | 2 | 0bf2cd470ebf87d48ab4abff4222017bd50d4253:0b267cf632915083624acd3d8b44e7c4cfe3f66f | CLEAN (1 MINOR fixed) |
| Plan | 1 | ba7c38f22d155fb504ddb33aa8737b70c5a640eb:83ef880756ad3299406803c3e66a8b5984a0018b | CLEAN (3 MINOR fixed, prose only) |
| M1 A (code-review high) | 1 | 13b113bded067ae9c3d0314bcc2e7ee8337071ee:e9406cd35042eeb4dc63d27f5b8874c4e0fd50b3 | no BLOCKER/MAJOR; stale-press guard and per-section overflow check fixed; rest declined |
| M1 B (conformance) | 1 | 13b113bded067ae9c3d0314bcc2e7ee8337071ee:e9406cd35042eeb4dc63d27f5b8874c4e0fd50b3 | CLEAN (1 MINOR doc wrap fixed) |
| Tasks | 1 | f14a22d473606b7e75e25f4b4e9bbd2a586ba061:933b6630274d782e6037b84d4a919f036546d3c9 | CHANGES: 1 MAJOR ([P] on shared file), fixed, prose only; 2 MINOR fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | press on error-held section not recorded | FR-002: the press changes nothing |
| M1 | A | default key not stored in the group; in_group unused | data-model.md/T004 fix the API; in_group is used by M2 showcase |
| M1 | A | rapid-sequence test tautological; open_form stitching | kept: pins determinism; stitching is the FR-009 geometry tradeoff |
| #1135 | Design | open | |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

## M2 notes

Review A (code-review high): 7 findings; fixed lockstep independent set (own state), in_group use, GROUP_DEFAULT, T011 text, test strength, live caption; declined: completeness test for new poses (catalogue/captions tests already tie poses to render), summary on group sections kept. Review B (sonnet): CLEAN (1 MINOR fixed). Visual pass: PASS. Full gate green.

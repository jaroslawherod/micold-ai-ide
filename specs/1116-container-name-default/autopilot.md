# Autopilot ledger — #1116 container-name-default

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1116 (issue: Container daemon: default the container name to the daemon name when none is specified); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1116
- **Worktree branch**: feat/1116_container-daemon-default-the-container-name-to-the
- **Started**: 2026-10-23
- **Phase**: done
- **Next step**: handoff

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1297 | Design | merged | 2ecf9e9eaa23625479047f65c2f6a7f2625b7c2b |
| #1362 | M1 | merged | 74a1b4b280b1ac42f2552c05557f15d446f3daa5 |
| TBD | Close | open | – |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Empty container name defaults to the sanitized daemon name | #1362 | merged |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Collision on defaulted name: refuse or suffix? | Refuse, no suffix | agent-resolved | spec.md#Clarifications |
| D2 | clarify | Length limit | 63 chars, defaulted names only | agent-resolved | spec.md#Clarifications |
| D3 | clarify | Typed names charset-checked? | No, today's validation | agent-resolved | spec.md#Clarifications |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 81c88859a3a10908cdd0fa870e9dd8a153b136a0:f8d2257e9c0f112ac40a6a3e85cb74b9d46eaa10 | CHANGES: 3 MAJOR (fixed) |
| Spec review | 2 | d1dc2627e7c471c4fd0886ffdd06b8898735a213:f8d2257e9c0f112ac40a6a3e85cb74b9d46eaa10 | CLEAN (2 MINOR fixed) |
| Plan review | 1 | 7eb8f0557e92900b6ded9f7b12df73a2977aca3a:1c6576b9833ac192a4518a3b5add680eca80ae58 | CLEAN (3 MINOR fixed, prose only) |
| Tasks review | 1 | f8a62c6461cd10a70849a0054ef26320f185ed28:a5b8e5e01b17e843112acab15b68f3ace542dd0c | CHANGES: 1 MAJOR (T005 path, fixed, prose only) |
| M1 code review A (high) | 1 | 7c93b3fef4c8dde01996ca6745345a3303f1388c:636da86ef2842bf145d1a2eb8a6191b49aadf370 | MINORs only |
| M1 review B | 1 | same | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | Default of one character (`a`) is rejected by Docker (needs 2+) | FR-002/FR-011 fix the charset and length only; typed names are not length-checked either. Noted as a follow-up. |
| M1 | A | Hint not shown for whitespace-only field; hint/resolve share logic; dead field_name arm; test names | MINOR, not fixed |

## Handover

None.

## Open escalation

Close PR push blocked by the gate hook: `mise run gate` is red only on `the_layout_matches_the_committed_fixture` (font-dependent, host-only; every other step green) and the diff touches tests. Needs the user to allow one push with the green-gate check bypassed, as for M1.

## Follow-ups not done

- A defaulted container name of one character (`a`) passes FR-002 but Docker needs at least two; consider a minimum length for defaulted and typed names.
- TDD audit (`tdd/verification.md`) verdict FAIL: no recorded red phase (tests and source in one commit); cannot be repaired. Not fixed: placeholder is not tested through the rendered form (T014), and the empty-name message text is not asserted (T015).
- Close review round 1 (sonnet): CHANGES (1 MAJOR T018 wording, 2 MINOR task wording), all fixed, prose only.

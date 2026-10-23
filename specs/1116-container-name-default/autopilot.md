# Autopilot ledger — #1116 container-name-default

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1116 (issue: Container daemon: default the container name to the daemon name when none is specified); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1116
- **Worktree branch**: feat/1116_container-daemon-default-the-container-name-to-the
- **Started**: 2026-10-23
- **Phase**: tasks
- **Next step**: design PR merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Empty container name defaults to the sanitized daemon name | #… | pending |

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None. <or: the banner as sent, and when>

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

# Autopilot ledger — #1116 container-name-default

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1116 (issue: Container daemon: default the container name to the daemon name when none is specified); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1116
- **Worktree branch**: feat/1116_container-daemon-default-the-container-name-to-the
- **Started**: 2026-10-23
- **Phase**: milestone M1
- **Next step**: M1 PR merge, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1297 | Design | merged | 2ecf9e9eaa23625479047f65c2f6a7f2625b7c2b |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012 | full | Empty container name defaults to the sanitized daemon name | – | in review |

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

None beyond the open escalation: the next step is the PR (pr.md) once the gate question is answered. Gate fixes already made: daemons_section_geometry.rs line 873 label, features_daemon_sections.rs marker test.

## Open escalation

M1 is implemented, ticked (T001-T012 except a green gate), reviewed (A: MINORs only; B: CLEAN), pushed nowhere. `mise run gate` is red on `layout_snapshot::the_layout_matches_the_committed_fixture` (status-bar-warnings, element 0/0/0/2/0/0/0/1 width 775.2 vs recorded 778.1). It fails identically with `crates/` checked out from origin/main, so it is host-dependent (fonts) and not caused by this diff; origin/main CI is green. The push hook needs a green gate. Asked 2026-10-24: how to push.

## Follow-ups not done

- A defaulted container name of one character (`a`) passes FR-002 but Docker needs at least two; consider a minimum length for defaulted and typed names.

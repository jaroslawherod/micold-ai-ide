# Autopilot ledger — #1525 daemon-config-file (Issue 4)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1525` (The app renders each daemon's configuration from Settings); labels: `flow:feature`, `effort:low`. User decision 2026-10-30 (decided by user): Spec, Clarify, Plan skipped; design already on main (PR #1509); tasks unit only cuts milestones for the "Issue 4" group (T021–T024).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1525
- **Worktree branch**: feat/1525_the-app-renders-each-daemon-s-configuration-from
- **Started**: 2026-10-30
- **Phase**: tasks
- **Next step**: handoff (merge design PR, then M1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1617 | Design (milestones for Issue 4) | open | — |
| #1620 | M1 | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T021–T024, T022a, T024a | full | `render` golden per kind, stable bytes, validate before delivery, errors in Settings → Daemons, user guide | #1620 | open |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main | user | prompt 2026-10-30 |
| D2 | tasks | Split Issue 4? | No: 4 tasks, one story, render and validation share the rendered text (rule 3) | agent-resolved | milestones.md rule 3 |
| D3 | tasks | User-guide task | Added T024a to the milestone (CI user-guide gate) | agent-resolved | milestones.md rule 4 |
| D4 | tasks | Tier | full: touches the Settings UI and a public client API; M1 is always full | agent-resolved | milestone-format.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| M1 review B | 1 | d1f11923:84c91cfc | CLEAN (2 MINOR accepted) |
| Tasks 1525 | 1 | b494679f:b6f32a1a | CHANGES: 2 MAJOR (no failing test for validate/UI: added T022a; T022 had no file: given one), 2 MINOR (Verify made runnable; F4 accepted, rebase); test-task additions, no further round |

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 4 (#1525)"; Issue 3's PR #1616 edits the same section, so the second to merge rebases.
- TDD test list not generated here; the milestone's implement run derives it.

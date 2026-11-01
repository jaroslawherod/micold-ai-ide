# Autopilot ledger — #1533 start-script

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1533 (An optional start script prepares each daemon); labels: `effort:high`, `flow:feature`, in-progress
- **Kind**: feature
- **Effort**: high
- **Issue**: #1533
- **Worktree branch**: feat/1533_an-optional-start-script-prepares-each-daemon
- **Started**: 2026-11-01
- **Phase**: tasks
- **Next step**: design PR merge, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001-T009 | full | Script runs before serving; exports applied; StartupReport + protocol bump | | pending |
| M2 | T010-T017 | full | env_file secrets, precedence, run/failure policies, reload, host guide | | pending |
| M3 | T018-T022 | full | Settings Start script group (hidden per kind until delivery) | | pending |
| M4 | T023-T029 | full | Host+SSH delivery and failure display (after #1526) | | pending |
| M5 | T030-T033 | full | Container delivery (after #1527) | | pending |
| M6 | T034-T036 | full | Pod delivery (after #1528) | | pending |

T037 (quickstart results) is left to the close unit. M1 may need the `docs-not-needed` label.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Spec location | `specs/1533-start-script/` narrows parent `specs/daemon-config-file/` (Story 5); tasks T068-T077 stay in the parent tasks.md | agent-resolved | parent spec.md |
| D2 | clarify | Round 1 of speckit-clarify | No critical ambiguities; record location, inline-script storage and warning transport are plan-level | agent-resolved | spec.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 1533 | 1 | 78dc2822:dbb82d13 | CHANGES: 2 MAJOR (PATH contradiction, unmerged deps assumption) fixed; 3 MINOR fixed |
| Spec 1533 | 2 | 2c305bf0:7f105e4f | CLEAN (3 MINOR: per-session variable set left to the contract, script_file required checked against the parent contract table, ledger row added) |
| Plan 1533 | 1 | 9afe4badc3b5:6e4fc30a516c | CHANGES: 3 MAJOR (timeout mechanism, align_container_home order, host spawn failure trigger) and 3 MINOR fixed; re-review next |
| Plan 1533 | 2 | ac54eddf2e07:6e4fc30a516c | CLEAN (3 MINOR fixed, prose only) |
| Tasks 1533 analyze | 1 | 43696b27:c6372cd5 | 0 CRITICAL, 1 HIGH (Windows tests/doc) fixed; MEDIUM/LOW fixed except task splitting (declined) |
| Tasks 1533 | 1 | 43696b27:c6372cd5 | CHANGES: 1 MAJOR (M3 half-wired UI: group hidden per kind until delivery) and 3 MINOR fixed |
| Tasks 1533 | 2 | 80d6c8d2:c6372cd5 | CHANGES: 1 MAJOR (guide task T023 moved to M4) and 2 MINOR fixed; prose/ordering only, no round 3 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| tasks | analyze C3 | split T008, T015, T024 | each is one cohesive file or change; no rubric item on task size |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Dependencies #1526, #1527, #1528 are still OPEN on GitHub (no delivery commits on main); #1524 is closed. Script delivery (T075) and per-kind failure surfacing need them: plan/milestones must order or stub accordingly.

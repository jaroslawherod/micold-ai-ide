# Autopilot ledger — #1526 daemon-config-file (Issue 5)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1526` (Deliver the configuration to host and SSH daemons); labels: `flow:feature`, `effort:low`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525 and #1534 (see `specs/daemon-config-file/autopilot-1524.md`, `autopilot-1525.md`, `autopilot-1534.md`); the tasks unit only cuts milestones for the "Issue 5" group (T025–T029, T080a).
- **Kind**: feature
- **Effort**: low
- **Issue**: #1526
- **Worktree branch**: fix/1526_deliver-the-configuration-to-host-and-ssh-daemons
- **Started**: 2026-11-02
- **Phase**: tasks
- **Next step**: open the design PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T025–T029, T080a | full | host file written `0600` before spawn, only on change; SSH file and secrets copied to `~/.micold/remote/` (`0700`/`0600`), a rotated token re-copies only that file; SSH env list for user hosts gone | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | tasks | Skip spec/clarify/plan? | Yes, design merged on main (PR #1509); same precedent as #1524, #1525, #1534 | agent-resolved | specs/daemon-config-file/autopilot-1524.md, autopilot-1525.md, autopilot-1534.md |
| D2 | tasks | Split Issue 5? | No: 6 tasks, one story (host + SSH delivery share the write-if-changed path and FR-005/FR-029), rule 3 of milestones.md | agent-resolved | milestones.md rule 3 |
| D3 | tasks | Include T080a? | Yes: Issue 13's ledger (#1534, D2) already moved it here because it needs T027's delivery | agent-resolved | specs/daemon-config-file/autopilot-1534.md |
| D4 | tasks | Tier | full: touches process spawn, SSH provisioning and file permissions (security-relevant); M1 is always full | agent-resolved | milestone-format.md |

## Review rounds

- Tasks review round 1 (snapshot a9a1e1257e80ba2b94fe9cefd52688d321ef553f:8176ca30aac5c24fca47a85695e888d3122ad2b5): CHANGES; F1 (MAJOR, Satisfies line omitted US3/FR-014 for T080a) fixed, prose only (`Satisfies: US1, US3; FR-005, FR-014, FR-029`); no re-review needed.

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Milestones live in tasks.md under "Issue 4 (#1525)" through "Issue 14 (#1535)"; #1525 stays open for follow-up work in PR #1654 and may edit the same section — expect a rebase if the second to merge.

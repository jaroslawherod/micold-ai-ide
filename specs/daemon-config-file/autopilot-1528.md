# Autopilot ledger — #1528 daemon-config-file (Issue 7)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1528` (Deliver the configuration to Kubernetes daemons); labels: `flow:feature`, `effort:high`. Design already on main (PR #1509, `specs/daemon-config-file/`): Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526 and #1534 (see `autopilot-1524.md`, `autopilot-1525.md`, `autopilot-1526.md`); the tasks unit only cuts milestones for the "Issue 7" group (T036–T039, T091).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1528
- **Worktree branch**: feat/1528_deliver-the-configuration-to-kubernetes-daemons
- **Started**: 2026-11-03
- **Phase**: tasks
- **Next step**: open design PR; then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| — | none yet | — | — |

## Milestones

| M | Deliverable | Tasks | Tier | PR |
|---|---|---|---|---|
| M1 | pod daemons read ConfigMap `<name>-config` at `/run/micold/config` and directory-mounted Secrets under `/run/micold/secrets/…`; no `MICOLD_*` env; `TZ`, `LANG=C.UTF-8` | T036–T039, T091 | full | — |

Five tasks, one acceptance scenario, so not split. Paths in the tasks checked 2026-11-03.

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | New spec for #1528? | No: parent spec, plan, contracts §3.4 and tasks T036–T039, T091 already on main (PR #1509); blockers #1524, #1525 are CLOSED | agent-resolved | specs/daemon-config-file/tasks.md "Issue 7"; autopilot-1526.md D1 |

## Review rounds

- Tasks review round 1 (snapshot 47d0d8181a03c75d1692d4824ddc42544481c27d:d0ddfd28dc82009eccfd212cbf64220e528f9722): CHANGES; F1 (MAJOR, T091 test not before code, T036 env list contradicted T091) fixed, prose only (T036 golden now expects `TZ`/`LANG`, T091 implements); F2 (MINOR) declined, column order is cosmetic; F3 (MINOR) left, T091 ID stable.

## Declined review findings

None.

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

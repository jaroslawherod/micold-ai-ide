# Autopilot ledger — #1981 kube-e2e

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Epic: end-to-end tests on a kind cluster, with a Helm chart for the orchestrator and web UI (#1981); jaro's words in specs/1981-kube-e2e/spec.md. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Supervised**: no <!-- `yes` when the run started with AUTOPILOT_SUPERVISED set: an escalation is recorded and the turn ends, no interactive question -->
- **Issue**: #1981
- **Worktree branch**: claude/project-thread-pcdbm7
- **Started**: 2026-11-15
- **Phase**: plan
- **Next step**: Plan review (pending, orchestrator dispatches reviewers), then tasks unit.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #… | Design | open / merged | … |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | tbd | full | Image + chart + data-dir/socket flags | #… | pending |
| M2 | tbd | full | Ops /healthz /readyz, probes, --ops-addr (before harness, see plan) | #… | pending |
| M3 | tbd | full | kind harness, mise task, CI job kube-e2e | #… | pending |
| M4 | tbd | full | API e2e suite (ops, link, restart, seeded breakages) | #… | pending |
| M5 | tbd | full | network/ui placeholder slots; real tests after #1732/#1954 | #… | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | How does the test daemon reach the orchestrator? | Test-only daemon container sharing the emptyDir socket | agent-resolved (orchestrator; jaro can overturn) | spec.md FR-011 |
| D2 | clarify | Default chart state mode? | `file` on a PVC; sqlite/postgres fail rendering until #1948/#1740 | agent-resolved (orchestrator; jaro can overturn) | spec.md FR-017 |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 | spec | n/a | 3d44f8b336c799ba30e46d236c626356b3939dc1:a089ee32c8f892cce0e56df14602c477047c6834 | CHANGES: 3 MAJOR (F1 F2 F3), 3 MINOR; all fixed, not re-reviewed |
| spec | 2 | scoped | 5c0712c5da20b4c7631132c4264657df9548e803 | 6b26e7bcb50218985959800c72c608652ee03be7 | CLEAN |
| plan | 1 | full | n/a | a93ee5534a7e31dc2375b7e0774a822515594c74:2a8afeac5f380ebde8010c8aac79ded2d55c4283 | CHANGES: 4 MAJOR (F1-F4), 2 MINOR (F5 F6); all fixed, not re-reviewed |
| plan | 2 | scoped | af46640a697fa39efc4643087b4a875ff9152982 | b99f4c78af95f4dad1805307dae6165ce0f9da7b | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None. <or, when a unit handed over: what is done, the next step, open findings with their review
snapshots. The next unit sets it back to None.>

## Open escalation

None.

## Follow-ups not done

None.

## Plan notes

- Clarify unit skipped by the orchestrator (no markers left in the spec). Milestones reordered: M2 ops before M3 harness (smoke needs the readiness probe); spec table updated. Plan written, plan review pending (reviewer round 1 not run: unit has no Agent tool).

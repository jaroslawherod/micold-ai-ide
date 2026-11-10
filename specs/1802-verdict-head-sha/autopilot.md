# Autopilot ledger — #1802 verdict-head-sha

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot: bind review verdicts to the PR head SHA and refuse stale merges (issue #1802, see issue body); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1802
- **Worktree branch**: feat/1802_autopilot-bind-review-verdicts-to-the-pr-head-sha
- **Started**: 2026-11-10
- **Phase**: spec
- **Next step**: clarify (no open markers; may be skipped), then plan

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| none yet | Design | - | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| - | - | - | - | - | - |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| - | - | - | - | - | - |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 17711061d98b76cdd180592ea1476e81d74cdd20:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 1 BLOCKER 3 MAJOR (F1-F4 fixed: ledger-excluded binding model, guard helper on all merge paths, verdict vocabulary and Scope column, Head vs Snapshot reconciled; F5-F7 fixed) |
| Spec | 2 | 389ee48108da761a86aff384ee6d48665552c109:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 4 MAJOR (squash/hook mechanism, carry persistence without moving head, assumption contradiction, design scope mapping; all fixed) |
| Spec | 3 | 7a3a1bccf9922b772e9dd6e4dc666ac7d828b9e1:653769a81adcc13e2f9695de8f9ade892d15cf58 | CLEAN (2 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

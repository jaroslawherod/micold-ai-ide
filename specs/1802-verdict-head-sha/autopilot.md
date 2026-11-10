# Autopilot ledger — #1802 verdict-head-sha

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot: bind review verdicts to the PR head SHA and refuse stale merges (issue #1802, see issue body); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1802
- **Worktree branch**: feat/1802_autopilot-bind-review-verdicts-to-the-pr-head-sha
- **Started**: 2026-11-10
- **Phase**: tasks
- **Next step**: M1 (after design PR merges)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1818 | Design | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | record verdicts, guard refuses stale merges, hook denies bare merge | - | pending |
| M2 | T016–T019 | full | carry verdict over clean replay | - | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | record verdicts, guard refuses stale merges, hook denies bare merge | - | pending |
| M2 | T016–T019 | full | carry verdict over clean replay | - | pending |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 17711061d98b76cdd180592ea1476e81d74cdd20:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 1 BLOCKER 3 MAJOR (F1-F4 fixed: ledger-excluded binding model, guard helper on all merge paths, verdict vocabulary and Scope column, Head vs Snapshot reconciled; F5-F7 fixed) |
| Spec | 2 | 389ee48108da761a86aff384ee6d48665552c109:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 4 MAJOR (squash/hook mechanism, carry persistence without moving head, assumption contradiction, design scope mapping; all fixed) |
| Spec | 3 | 7a3a1bccf9922b772e9dd6e4dc666ac7d828b9e1:653769a81adcc13e2f9695de8f9ade892d15cf58 | CLEAN (2 MINOR fixed, prose only) |
| Plan | 1 | c7eba65c50f3b3082937ba9e8b1a6326843f84d0:a01912112b04ada1ab01aed1ecd4869197834b0d | CHANGES: 3 MAJOR (ledger pathspecs, unreadable ledger not unguarded, hook test change; all fixed) |
| Plan | 2 | 7eb9ad18b06bf39a7c7de16ef2ec64e8420ded4b:a01912112b04ada1ab01aed1ecd4869197834b0d | CHANGES: 1 MAJOR (hook test at line 514 also changes; fixed), 1 MINOR (:(glob) magic; fixed) |
| Plan | 3 | dad4eb34edb2f746a9a8d808e13ddcf0e60fc424:a01912112b04ada1ab01aed1ecd4869197834b0d | CLEAN |
| Tasks | 1 | d7cdb0773ef6a1ffbd9bb4cec79dd694d16e1d33:91e1a11146d02e0534177bfd454a80b08a4a8e81 | CLEAN (3 MINOR: 2 fixed prose only, 1 declined: T002/T003 covered by T004 noted in Milestones) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

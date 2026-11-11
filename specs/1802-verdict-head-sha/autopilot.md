# Autopilot ledger — #1802 verdict-head-sha

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: autopilot: bind review verdicts to the PR head SHA and refuse stale merges (issue #1802, see issue body); labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #1802
- **Worktree branch**: feat/1802_autopilot-bind-review-verdicts-to-the-pr-head-sha
- **Started**: 2026-11-10
- **Phase**: milestone M2
- **Next step**: wait-merge M2 PR, then close

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1818 | Design | merged | 4b2a22c130a6087fb3b38e3edf22505025cb9f00 |
| #1821 | Milestone M1 | merged | 653cd72150666522b51b58028f01b3e72cbfe726 |
| #1825 | Milestone M2 | open | - |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | record verdicts, guard refuses stale merges, hook denies bare merge | #1821 | merged |
| M2 | T016–T019 | full | carry verdict over clean replay | #1825 | in review |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | record verdicts, guard refuses stale merges, hook denies bare merge | - | pending |
| M2 | T016–T019 | full | carry verdict over clean replay | - | pending |

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Spec | 1 |  |  | 17711061d98b76cdd180592ea1476e81d74cdd20:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 1 BLOCKER 3 MAJOR (F1-F4 fixed: ledger-excluded binding model, guard helper on all merge paths, verdict vocabulary and Scope column, Head vs Snapshot reconciled; F5-F7 fixed) |
| Spec | 2 |  |  | 389ee48108da761a86aff384ee6d48665552c109:653769a81adcc13e2f9695de8f9ade892d15cf58 | CHANGES: 4 MAJOR (squash/hook mechanism, carry persistence without moving head, assumption contradiction, design scope mapping; all fixed) |
| Spec | 3 |  |  | 7a3a1bccf9922b772e9dd6e4dc666ac7d828b9e1:653769a81adcc13e2f9695de8f9ade892d15cf58 | CLEAN (2 MINOR fixed, prose only) |
| Plan | 1 |  |  | c7eba65c50f3b3082937ba9e8b1a6326843f84d0:a01912112b04ada1ab01aed1ecd4869197834b0d | CHANGES: 3 MAJOR (ledger pathspecs, unreadable ledger not unguarded, hook test change; all fixed) |
| Plan | 2 |  |  | 7eb9ad18b06bf39a7c7de16ef2ec64e8420ded4b:a01912112b04ada1ab01aed1ecd4869197834b0d | CHANGES: 1 MAJOR (hook test at line 514 also changes; fixed), 1 MINOR (:(glob) magic; fixed) |
| Plan | 3 |  |  | dad4eb34edb2f746a9a8d808e13ddcf0e60fc424:a01912112b04ada1ab01aed1ecd4869197834b0d | CLEAN |
| Tasks | 1 |  |  | d7cdb0773ef6a1ffbd9bb4cec79dd694d16e1d33:91e1a11146d02e0534177bfd454a80b08a4a8e81 | CLEAN (3 MINOR: 2 fixed prose only, 1 declined: T002/T003 covered by T004 noted in Milestones) |
| Review A M1 | 1 | M1 | dc7cfd142ac29c7302e810435155b3472b59a84b | 7add53fedf32f84607640e1a326baa5a07c4b007:1e6c3dcca1460b5617f00b4eaba7eedee8cb1847 | CHANGES: 2 MAJOR 3 MINOR (F1 hook help exemption, F2 hook command positions, F3-F5 fixed) |
| Review A M1 | 2 | M1 | dc7cfd142ac29c7302e810435155b3472b59a84b | 22ae9ff370a0701dbaffe1b7a820af180448b97f:dc7cfd142ac29c7302e810435155b3472b59a84b | CLEAN (1 MINOR declined: backtick/bash -c hardening out of scope) |
| Review B M1 | 1 | M1 | dc7cfd142ac29c7302e810435155b3472b59a84b | 7add53fedf32f84607640e1a326baa5a07c4b007:1e6c3dcca1460b5617f00b4eaba7eedee8cb1847 | CLEAN (1 MINOR prose wording, left) |
| Review A M2 | 1 | M2 | 9e340c7c66a804c1e6b4e7eeeb8f0435cfe1da64 | 9e1162fc01454cb33a2355b5d587b5d6b740bd03:c1e951ba852b1ba9da59fc3550ee2ef47b149aab | CHANGES: 1 MAJOR 1 MINOR (F1 CARRIED head pinned to guarded P, F2 git errors vs no-change; both fixed) |
| Review B M2 | 1 | M2 | 9e340c7c66a804c1e6b4e7eeeb8f0435cfe1da64 | 9e1162fc01454cb33a2355b5d587b5d6b740bd03:c1e951ba852b1ba9da59fc3550ee2ef47b149aab | CLEAN (1 MINOR: CARRIED names last H, documented in merge.md) |
| Review A M2 | 2 | M2 | 9e340c7c66a804c1e6b4e7eeeb8f0435cfe1da64 | fix-diff | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

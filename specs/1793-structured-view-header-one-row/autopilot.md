# Autopilot ledger — #1793 structured-view-header-one-row

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: /speckit-autopilot #1793 (labels: enhancement, flow:feature)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1793
- **Worktree branch**: feat/1793_feat-structured-view-header-in-one-row-status-and
- **Started**: 2026-11-10
- **Phase**: milestone M3 (in progress)
- **Next step**: wait CI, merge M3, then M4

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1797 | Design | merged | ad2d932dc0b678b50e19e0b3646e8036d62c6e8a |
| #1798 | M1 | merged | 1c675009f14aaf26035b3c593512a76365ad5009 |
| #1827 | M2 | merged | cb41c216e778ce87dcc7379f7852f3ffa85fa3bf |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T012, T028 | full | Top bar ring, status, Changes (with collapse gate) | #1798 | merged |
| M2 | T013–T016 | full | Worktree menu with raw events and view control | #1827 | merged |
| M3 | T017–T020 | full | Single `+` menu | PR | open |
| M4 | T021–T027, T029, T031, T034, T035 | full | One-row header | | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Scope | Head | Snapshot | Verdict |
| --- | --- | --- | --- | --- | --- |
| Tasks | 1 |  |  | a2463a39:901c3164 | CHANGES (4 MAJOR, fixed) |
| Tasks | 2 (sonnet) |  |  | fix diff | CLEAN (2 MINOR, fixed) |
| Spec | 1 |  |  | bb128d1c688f3a42eaea356845c2b2497ccfa4d1:c63bfc7c625e9cac5e814565ff80e2d20296bb2c | CLEAN (3 MINOR, fixed prose only) |
| M1 Review A (high) | 1 |  |  | 5ff40e78:ca940bff | CLEAN (2 MINOR: changes_button near-copy and floor constant, left) |
| M1 Review B (sonnet) | 1 |  |  | 14aea717:34cfb594 | CLEAN |
| M2 Review A (high) | 1 |  |  | 1b71d9df:4b30f70c | CLEAN (1 MINOR: Chosen doc, left) |
| M2 Review A+B (sonnet) | 2 | M2 | bc9262d05a01f8146f898a15d2a0896dc2e19b29 | d28d938bd831dae098ca3fd7dcb6453d3054c43f:bc9262d05a01f8146f898a15d2a0896dc2e19b29 | CLEAN (2 MINOR: worktree menu keyboard highlight to confirm in M4; no tdd dir, left) |
| M3 Review A (high) | 1 | M3 |  | edb22866:18a8d7b5 | CHANGES (1 MAJOR anchor y fixed, 2 MINOR fixed) |
| M3 Review B (sonnet) | 1 | M3 |  |  | CHANGES (2 MAJOR: drift-M3.md written, orphaned switch offer removed; 2 MINOR fixed); fixes are gate-covered, no re-run |
| M3 Review A+B | 1 | M3 | 1341b48328fe557270f4d776fb13371590a30dc0 | 94fab10166996ae7d1b1a78b37d0209300817186:1341b48328fe557270f4d776fb13371590a30dc0 | CLEAN after fixes (A 1 MAJOR, B 2 MAJOR fixed; gate green) |

## Declined review findings

None. Plan review: round 1 CHANGES (3 MAJOR, fixed), round 2 (sonnet) CLEAN; snapshot 082794ab:53c2cfac. Open for tasks unit: session-bar leftovers placement (research R6) is a recommended default the spec does not state.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

RESOLVED (human, option 1: slim in-body row; spec US1 AC1, FR-001, SC-001 amended with a clarification entry). Was: Tasks review round 1 (snapshot a2463a39:901c3164): CHANGES, 4 MAJOR. F2-F4 fixed in tasks.md (collapse gate moved to M1, M5 dropped, gate registration in layout_snapshot.rs, test-first and gate retirement for R6 relocations). F1 open: research R6 default (conditional slim row in the transcript body for the tasks accordion and mode label) contradicts spec US1 AC1 / FR-001 / SC-001 "exactly one header row precedes the event list" whenever background tasks or a mode exist. Needs a decision; tasks T026/T034 are written to the default and change with the answer.

## Follow-ups not done

<Defects found outside this flow's work, scope deliberately cut, etc. Copied into the handoff.>

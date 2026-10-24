# Autopilot ledger — #1271 review-reviewed-marks

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start work on issue #1271 "Review UI: reviewed marks and progress" (blocked by #1269, not merged; spec builds on its design in docs/development/review-redesign.md); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1271
- **Worktree branch**: claude/project-thread-umhor9
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan unit (clarify done; D4 provisional, pending user on Open escalation Q1)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Reset marks on every new head commit, or per file's change? | Per file's change (a mark holds while the file's whole change is the same) | agent-resolved | spec.md#Assumptions |
| D2 | spec | Row symbol for a marked file with drafts | Reviewed wins; draft count stays on the row | agent-resolved | spec.md#Assumptions |
| D3 | clarify | Header wording: *n of m reviewed* or *n/m reviewed*? | *n of m reviewed* | agent-resolved | issue #1271 body (spec.md#Input) |
| D4 | clarify | Mark a file while a toggle hides part of its change? | A: no — Mark reviewed unavailable (tooltip) for a file with changes of a hidden kind; unmark stays available. Applied to FR-002, FR-005, Edge Cases > Toggles | provisional, pending user | spec.md US2 rationale, FR-005; Open escalation Q1 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 5cce47b02a0a2a5998d722bd75a1292810c331b6:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CHANGES: 1 MAJOR, 3 MINOR (run by the orchestrator). F1 fixed: FR-016 keeps a mark until unmarked/re-marked/entry removed, edge case and US2 scenario 6 added; F2 fixed (prose): US2 s5 limited to entries with a base; F3 fixed: older-window edge case; F4 fixed: US4 s2, PR marks kept like PR comments |
| Spec | 2 | ebf5c8ada3cf70b15771dbcb49161fbc5e7fd27f:029348b9eeae8148929f5bcf846f762b40db03b6 | CLEAN (model sonnet, scoped re-review of F1, F3, F4) |
| Clarify | 1 | b4b18fc6 | 2 questions: D3 agent-resolved, Q1 escalated, applied provisionally as A (D4) |
| Clarify | 2 | after D4 applied | No critical ambiguities (US1 s7 added to test D4) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle). With the user; A applied provisionally (D4). If the user picks B or C, revise FR-002, FR-005, Edge Cases > Toggles and the Clarifications bullet.

- **Q1**: When a Committed or Uncommitted toggle hides part of a file's change, may the user still
  mark that file reviewed? FR-005 judges a mark on the whole file's change, so a mark made while
  only the committed part is shown would also cover uncommitted lines the user never saw — the
  "claims a review that did not happen" that US2's rationale rules out.
  - **A (Recommended)**: The mark control is unavailable, with a tooltip "Turn on Committed and
    Uncommitted to see this file's whole change", for a file that has changes of a hidden kind;
    files with no hidden part can be marked as usual. Evidence: spec.md US2 "Why this priority",
    FR-005, Edge Cases > Toggles.
  - **B**: Marking is always available; the mark covers the whole change, including the part the
    toggle hides.
  - **C**: Marking is always available, but the mark records only the shown part and covers the
    file only while the same toggles are on (changes FR-005).

## Follow-ups not done

None yet.

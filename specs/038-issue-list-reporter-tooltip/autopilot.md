# Autopilot ledger — 038-issue-list-reporter-tooltip

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #518 (https://github.com/jaroslawherod/micold-ai-ide/issues/518): in the new-worktree form's issue list, show the reporter and labels on a second, wrapping line, and show a truncated description in a tooltip after the cursor rests on a row for 3 seconds. Read the issue with `gh issue view 518` for the acceptance criteria and code pointers.
- **Kind**: feature
- **Issue**: #518
- **Worktree branch**: feat/issue-list-reporter-labels-tooltip
- **Started**: 2026-10-02
- **Phase**: 1-spec
- **Next step**: Review spec.md against the spec rubric, then open PR 1 `docs(038): specify reporter, labels and a description tooltip in the issue list`.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Which spec number? | 038: `036-tooltip-follow-cursor-delay` is taken by the flow in worktree `fix-issue-430` (not on `main` yet), 037 is on `main`. | agent-resolved | `ls <worktree>/specs` across `git worktree list`, 2026-10-02 |
| D2 | spec | Does this feature build the tooltip show delay that issue #430 asks for? | No. Spec 036 (another flow) adds it. 036's delay counts from pointer entry; #518 needs "cursor still for 3 s", so the rest-delay is this feature's requirement (FR-015, FR-016) and the plan decides how it builds on 036. Stories 1 and 2 do not depend on 036. | agent-resolved | spec.md#Assumptions; `fix-issue-430/specs/036-tooltip-follow-cursor-delay/spec.md` FR-001 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 562a5e7f5fec78d4f0c7fc40b756ae28c5f93b5d:91e721245225f5de62861883e63a5ad816a190d7 | CHANGES: 3 MAJOR (FR-016 vs rest tolerance, FR-005 untestable, SC-008 unmeasurable), 3 MINOR; all six fixed |
| Spec | 2 | 8b82c5e2181ac182fb31b8e4e9aad9f2b2e5603a:91e721245225f5de62861883e63a5ad816a190d7 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Token usage

## Follow-ups not done

- Spec 036 (GitHub issue #430, tooltip show delay) is not on `main`. User story 3 (the description
  tooltip) may build on it; the design unit must check its state before cutting milestones.

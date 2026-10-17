# Autopilot ledger — #937 required-field-markers

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937 using speckit autopilot — issue #937 "Forms show which fields are required, following Material Design"; labels: enhancement, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #937
- **Worktree branch**: claude/project-thread-rkhzho
- **Started**: 2026-10-17
- **Phase**: plan
- **Next step**: plan unit (continue handover: plan review round 1); D2/D3 are orchestrator defaults pending jaro's card — re-apply to spec.md and plan.md *Pending decisions* if jaro answers B

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Marker convention: asterisk on required fields, or mark optional fields where most are required? | Asterisk on required fields, app-wide, explained by "* required" above the first field | agent-resolved | issue #937 acceptance criteria ("consistent across forms"); spec review round 1 F1; spec.md FR-001, FR-004 |
| D2 | clarify | Create/Confirm disabled while a required field is empty: enable them? | A: enable; a press with empty required fields shows the required errors, no submit (FR-012) | orchestrator default pending jaro's card | worktree_form.rs:602, parallel_dialog.rs:125; spec.md Clarifications |
| D3 | clarify | Show the required error on blur too, or only on submit attempt? | A: on blur and on submit attempt, never on a fresh form (FR-006) | orchestrator default pending jaro's card | issue #937 "left empty"; material/field_focus.rs; spec.md Clarifications |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c97f54c4c7b32b3f5f8c11552756f9e77d1de18:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN (1 MINOR: plan to confirm rename-dialog files) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

- Done: `speckit-plan` ran; plan.md, research.md (R1–R9, incl. the open items: rename-dialog files R4, select/typeahead blur R2, form inventory R5), data-model.md, contracts/required-field.md, quickstart.md written and committed. No extension hooks registered for plan.
- Next: plan review round 1 (tasks/review.md with rubrics/plan.md, model omitted), act on it (review-rounds.md), then set **Next step** to `tasks unit` and return DONE, PR none.
- Open findings: none yet. PR: none.
- Unit handed over at 156k context (cap 150k).

## Open escalation

None.

## Follow-ups not done

- Spec review round 2 MINOR (rename-dialog files, container fields): addressed in research.md R4 and R5.


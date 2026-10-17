# Autopilot ledger — #937 required-field-markers

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: start #937 using speckit autopilot — issue #937 "Forms show which fields are required, following Material Design"; labels: enhancement, flow:feature (orchestrator added in-progress)
- **Kind**: feature
- **Effort**: default
- **Issue**: #937
- **Worktree branch**: claude/project-thread-rkhzho
- **Started**: 2026-10-17
- **Phase**: clarify
- **Next step**: clarify unit round 1: apply the user's answers to the open escalation (Q1 disabled submit buttons, Q2 on-blur error) to spec.md, then run round 2

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

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 1c97f54c4c7b32b3f5f8c11552756f9e77d1de18:d93c9dbc687412cced0a9b7854cf44323ee3930e | CLEAN (1 MINOR: plan to confirm rename-dialog files) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1 (category 1, product/UX decision the repo does not settle):

- **Q1**: Create (add-worktree form, `crates/micold-client/src/ui/worktree_form.rs:602`, `can_submit()`) and Confirm (run-in-parallel, `crates/micold-client/src/ui/parallel_dialog.rs:125`, `validate().is_ok()`) are disabled while a required field is empty, so FR-006's "submit attempt" can never happen there (and US3 scenario 1 cannot run). What should these buttons do?
  - A (Recommended): enable them; a press with empty required fields shows the required errors (FR-006) and does not submit. Evidence: rename (`rename_dialog.rs:53`), Save daemon (`settings/daemons.rs:403`) and Settings Save (`settings_view.rs:128`) are already always enabled; one behaviour across forms is the issue's "consistent across forms".
  - B: keep them disabled; in those two forms the required error appears only on blur (forces Q2 = on blur there).
- **Q2**: Should an empty required field also show the required error when the user leaves it after focusing it, before any submit attempt? (FR-006 [NEEDS CLARIFICATION])
  - A (Recommended): yes, on blur as well as on a submit attempt; never on a freshly opened form. Evidence: the issue says "when a required field is left empty"; fields already report focus (`crates/micold-client/src/ui/material/field_focus.rs`), so blur is detectable.
  - B: no, submit attempt only (the spec's default).


## Follow-ups not done

- Spec review round 2 MINOR: the plan should confirm which files implement the project and worktree rename dialogs (rename_dialog.rs, rename.rs, worktree_rename.rs) and list the container fields under Settings › Daemons.


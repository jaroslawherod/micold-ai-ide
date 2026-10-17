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
- **Next step**: clarify unit: resolve the FR-006 [NEEDS CLARIFICATION] (on-blur required error); D1 already settles the marker convention

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

None.

## Follow-ups not done

- Spec review round 2 MINOR: the plan should confirm which files implement the project and worktree rename dialogs (rename_dialog.rs, rename.rs, worktree_rename.rs) and list the container fields under Settings › Daemons.


# Autopilot ledger — #1128 layout-redesign-docs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1128 "Layout: docs, layout snapshot and visual pass for the redesign". Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions, terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e; labels: enhancement, flow:feature (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1128
- **Worktree branch**: claude/project-thread-bznqpy
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: dispatch round 2 of the spec review (re-review of the fix diff, `model: "sonnet"`) on snapshot `9b68594184610e77e6ce0baafee15f2b72aaea93:e50400c2af72d2333c02fc4f00db4f78c08dd287` (diff against round 1: `scripts/autopilot/review-snapshot.sh diff 8604b804860bddadd362930f075cce48e8c64032:a08a4b365389835342d3751fd4b69a9a9c05c8b6`); then the clarify unit for the three `[NEEDS CLARIFICATION]` markers (US4 scenario 4, FR-011, FR-012).

Blockers at spec time: #1124 and #1126 merged; #1125 open (M1, M2 on main; M3–M5 todo); #1127 open, no spec. Spec stages the work: stage 1 (status bar, session area) now; stage 2 (sidebar, title bar, whole-window states, fixture, visual pass) after #1125 and #1127 merge.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | `8604b804860bddadd362930f075cce48e8c64032:a08a4b365389835342d3751fd4b69a9a9c05c8b6` | CHANGES: F1 MAJOR fixed (plan usage dropped from FR-010 and the #1127 Context row; edge case added for #1127 shipping it in the title bar); F2 MINOR fixed, prose only (FR-019 now covers every link, as SC-003); F3 MINOR fixed, prose only (in-region banners such as the stored-sessions offer are not removed elements, in Terms and FR-001). Round 2 needed: F1 changes a requirement. |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Round 1 of the spec review ran (orchestrator-dispatched); its fixes are committed. Earlier: spec unit wrote spec.md, checklists/requirements.md and this ledger, committed and pushed. The spec review did not run: this unit had no Agent tool to dispatch a fresh reviewer, and rules forbid self-review. Next: the orchestrator (or a spec unit with Agent) dispatches round 1 of the spec review on the snapshot in *Next step*, acts on it per tasks/review-rounds.md, then clarify.

## Open escalation

None.

## Follow-ups not done

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.

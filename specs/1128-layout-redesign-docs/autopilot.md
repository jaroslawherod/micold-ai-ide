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
- **Next step**: dispatch round 1 of the spec review (tasks/review.md, rubrics/spec.md, model omitted) on snapshot `9ef8f7f0ffe4cac62cc4ed2ea4c0582e3bc467d0:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2`; then the clarify unit for the three `[NEEDS CLARIFICATION]` markers (US4 scenario 4, FR-011, FR-012).

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

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Spec unit wrote spec.md, checklists/requirements.md and this ledger, committed and pushed. The spec review did not run: this unit had no Agent tool to dispatch a fresh reviewer, and rules forbid self-review. Next: the orchestrator (or a spec unit with Agent) dispatches round 1 of the spec review on the snapshot in *Next step*, acts on it per tasks/review-rounds.md, then clarify.

## Open escalation

None.

## Follow-ups not done

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.

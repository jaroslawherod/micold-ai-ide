# Autopilot ledger — #1128 layout-redesign-docs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1128 "Layout: docs, layout snapshot and visual pass for the redesign". Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions, terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e; labels: enhancement, flow:feature (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1128
- **Worktree branch**: claude/project-thread-bznqpy
- **Started**: 2026-10-24
- **Phase**: plan written, review pending
- **Next step**: plan review round 1 (orchestrator dispatches the reviewer on the snapshot below; this unit had no Agent tool), then act on findings; then tasks unit. Milestones per plan.md: M1 (stage 1 docs) now; M2–M4 after #1125 and #1127 merge.

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
| D1 | clarify | Visual-pass defect in a shipped region: fix here or record? | Record as *differs by defect* + ledger follow-up; no code change | agent | spec#Context; 1124 ledger follow-ups; 1126 visual-pass.md |
| D2 | clarify | Sweep removed-element mentions in user-guide pages outside the four? | Yes, only those sentences/rows, staged by region | agent | constitution VII; spec#Context (closing pass) |
| D3 | clarify | Add two whole-window covered states or only regenerate? | Add both, then regenerate | agent | layout-snapshot.md#Adding a covered state, #What is covered |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| spec | 1 | `8604b804860bddadd362930f075cce48e8c64032:a08a4b365389835342d3751fd4b69a9a9c05c8b6` | CHANGES: F1 MAJOR fixed (plan usage dropped from FR-010 and the #1127 Context row; edge case added for #1127 shipping it in the title bar); F2 MINOR fixed, prose only (FR-019 now covers every link, as SC-003); F3 MINOR fixed, prose only (in-region banners such as the stored-sessions offer are not removed elements, in Terms and FR-001). Round 2 needed: F1 changes a requirement. |
| spec | 2 | `482d3e84…:e0a83183…` (Sonnet re-review of the diff from round 1) | CLEAN; one MINOR fixed, prose only (Terms "removed element" paragraph rewrapped). |
| clarify | 1 | — | 3 questions, all agent-resolved (D1–D3); FR-011, FR-012, FR-020, SC-001, US4 scenario 4 changed. |
| clarify | 2 | — | No critical ambiguity; unit ends. |
| plan | 1 | `042e8e0058432d5b45e9377611122c12f1cce667:2fa8a2688c30d42e47a2ec0733539e77cba161bc` | pending: reviewer to be dispatched by the orchestrator (rubric `rubrics/plan.md`) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.
- Plan unit: no Agent tool in this environment, so the plan review was not dispatched by the unit; snapshot recorded for the orchestrator. `lychee` not installed locally (links check runs in CI).

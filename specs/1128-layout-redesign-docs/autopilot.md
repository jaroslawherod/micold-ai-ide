# Autopilot ledger — #1128 layout-redesign-docs

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1128 "Layout: docs, layout snapshot and visual pass for the redesign". Blocked by: #1124, #1125, #1126, #1127. Update user guide pages (worktrees-and-sessions, terminal-panes, overview, project-selection), regenerate the layout snapshot fixture, and run the visual pass at 1280x800 in both themes. Mockup: https://claude.ai/artifact/QXMeLzfh2oEkC9CAwn1M2e; labels: enhancement, flow:feature (in-progress added by orchestrator)
- **Kind**: feature
- **Effort**: default
- **Issue**: #1128
- **Worktree branch**: claude/project-thread-bznqpy
- **Started**: 2026-10-24
- **Phase**: tasks written, tasks review pending
- **Next step**: tasks review round 1 (fresh reviewer, `rubrics/tasks.md`, snapshot in Review rounds; orchestrator dispatches), then the rest of the tasks unit: `speckit-analyze`, checklists (none unchecked), specs-only gate, design PR. M1 can be built after the design PR merges; M2–M4 wait for #1125 and #1127 to merge.

Blockers at spec time: #1124 and #1126 merged; #1125 open (M1, M2 on main; M3–M5 todo); #1127 open, no spec. Spec stages the work: stage 1 (status bar, session area) now; stage 2 (sidebar, title bar, whole-window states, fixture, visual pass) after #1125 and #1127 merge.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011 | full | Status bar and session-area mentions corrected and linked; no bar under the terminal, no unnamed "bar" | — | todo |
| M2 | T012–T020 | docs | Sidebar and title bar in the user guide; FR-009 opening section; media alt text | — | blocked: #1125 and #1127 must merge |
| M3 | T021–T029 | full | `window-session-view`, `window-overview`, gate W, regenerated fixture, `layout-snapshot.md` passage | — | blocked: #1125 and #1127 must merge; after M2 |
| M4 | T030–T034 | full | Visual pass: 4 frames, 20 verdicts, contrast | — | blocked: after M3 |

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
| plan | 1 | `042e8e0058432d5b45e9377611122c12f1cce667:2fa8a2688c30d42e47a2ec0733539e77cba161bc` | CHANGES: F1 MAJOR fixed (`title_bar` anchors the 64 px bar, not its 65 px container; W1 `h == app_bar::HEIGHT`, W3 `title_bar.y + app_bar::BOTTOM_EDGE == shell.body.y`; hairline is the bar's bottom edge for SC-004's gap; follows #1127 if it drops the divider; R6, contract, data model, plan updated); F2 MINOR fixed (W5 in R6 and data model, asserted on `State` not records, selection accessor open until #1125 M3; mutation row: Overview closed → W5); F3 MINOR fixed, prose only (extended `strip_with` replaces `feat-working`'s session, keeping Working, in R5 and plan). Round 2 needed: F1 changes gate assertions. |
| plan | 2 | `2d670a35839ea1afff7c10d0094098102cdaa020:7301669f92b15e41ef74c6fbb0abe48878927cef` (Sonnet re-review of the diff from `042e8e00…:2fa8a268…`) | CLEAN; one MINOR declined (see Declined review findings). |
| tasks | 1 | `d43d91dcbdf609cfb6bd9034bc18cb89f689db50:fdcd26120f013b2724b34a84e104a307cb892afb` | pending: orchestrator dispatches the tasks review (`rubrics/tasks.md`) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| — (design) | plan round 2 | MINOR: `contracts/covered-states.md:40` says each anchor is "held by W1–W4" under a W1–W5 table | W5 is a state-level assertion on the `State`, not on an anchor, so W1–W4 are the anchors' properties; the sentence is correct. |

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- Cloud environment: `gh` not authenticated; `branch-start.sh` ran without a previous PR (none). Issue #1127's body was read through the GitHub MCP instead.
- Tasks unit: no Agent tool, so the tasks review was not dispatched; snapshot recorded for the orchestrator. Stopped before `speckit-analyze`, the gate and the design PR, as the orchestrator asked. `branch-start.sh` not run: origin/main is 2 unrelated commits ahead, and a rebase would rewrite the commits the plan-review snapshots name; the design-PR unit rebases. Optional hooks (`docguard.review` before tasks; `tdd.plan`, `docguard.score` after) skipped, non-interactive run. M3 will meet the mandatory `before_implement` `tdd.run` hook.
- Plan unit: no Agent tool in this environment, so the plan review was not dispatched by the unit; snapshot recorded for the orchestrator. `lychee` not installed locally (links check runs in CI).

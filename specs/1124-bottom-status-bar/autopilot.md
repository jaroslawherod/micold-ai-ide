# Autopilot ledger — #1124 bottom-status-bar

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1124 — "Layout: bottom status bar for connection, sandbox, CLI and plan usage" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1124
- **Worktree branch**: claude/project-thread-6jo3w5
- **Started**: 2026-10-24
- **Phase**: design-pr
- **Next step**: orchestrator opens the design PR from scratchpad pr-title.md / pr-body.md; after merge, milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T015 | full | Status bar on every body branch with connection and sandbox items; StatusBar in the showcase | — | planned |
| M2 | T016–T020 | light | Bring-up shown in the bar; no row above the body | — | planned |
| M3 | T021–T025 | light | Lost connection in the bar only; 7 action banners kept | — | planned |
| M4 | T026–T032 | light | CLI item follows the focused pane; Overview/Jump key hints | — | planned |
| M5 | T033–T038 | light | Plan usage moves from the app bar into the status bar | — | planned |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Where is plan usage shown today? | App bar trailing actions (`ui/toolbar.rs` usage_indicator, feature 490); mockup board draws it in the title bar, canvas note puts it in the status bar — left as a clarify marker | agent-resolved | crates/micold-client/src/ui/toolbar.rs:95; mockup project/Main.dc.html, canvas.json note s1 |
| D2 | spec | CLI item when the session view is not the visible body | Omitted (FR-007, after spec review F2) | agent-resolved | spec.md FR-007 |
| D3 | clarify | Does a lost connection (no action) still raise a banner? | No: bar only (error dot, reconnecting text); conditions with an action keep their banner (FR-003, FR-009, US3 s5) | issue text | issue #1124 body "Failures that need action still raise a banner…"; ui/mod.rs connection_banner (Disconnected has no action) |
| D5 | tasks | Milestone cut | One milestone per story (M1 US1+setup+foundational, M2 US2, M3 US3, M4 US4, M5 US5); the full connection/sandbox tables land in M1 because the model's match must be exhaustive; T039 (visual pass) left to close | agent-resolved | tasks.md §Milestones; milestones.md rules 1, 2, 4 |
| D4 | clarify | Plan usage in status bar or title bar? | Status bar, after the CLI item; removed from the app bar; #1127 builds on that (FR-011, US5) | issue text | issue #1124 title; canvas note s1 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | dc7a76326c7ffef720f23d0c9ebe55080ecfac9a:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | dfed0a366bf36453ce50a47b075d694ef9d0d313:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (1 MINOR: US4 scenario order — fixed, prose only) |
| Plan | 1 | 2e9c5ba755a6058d8286f9f7db0235b26e5fb28b:fe7a79742614d045774446cb51b847fe9564bb1e | CHANGES: 1 MAJOR (anatomy figure shape/ALL/tests), 1 MINOR (R6 location key) — both fixed |
| Plan | 2 | a336c6f1ebfd5e1598c453a4535ea5aeff6790b2:fe7a79742614d045774446cb51b847fe9564bb1e | CLEAN (2 MINOR: R11 typo, R6 None fallback — fixed, prose only) |
| Tasks | 1 | c125de1a0438346af357aa10e590a942959da084:697e95407233a56669e0a960535abc7d181ec6f6 | CHANGES: 1 MAJOR (M4 Verify ran main_tests with --lib), 3 MINOR (SC-001 split, checklist note, macOS chord example) — all fixed |
| Tasks | 2 | c42ec5e995bdd8f5e3c6560f3f5651acdf8c85f1:697e95407233a56669e0a960535abc7d181ec6f6 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

Notes: speckit-analyze (sonnet) 0 CRITICAL/HIGH; its MEDIUM/LOW findings fixed in tasks.md, plan.md (state count), quickstart.md, spec.md (US3 wording). Optional after_tasks hooks (tdd.plan, docguard.score) skipped. Checklist requirements.md: last item ticked (markers resolved in clarify).

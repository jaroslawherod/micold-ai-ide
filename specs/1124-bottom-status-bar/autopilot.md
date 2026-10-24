# Autopilot ledger — #1124 bottom-status-bar

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1124 — "Layout: bottom status bar for connection, sandbox, CLI and plan usage" (title and body as in spec.md *Input*); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1124
- **Worktree branch**: claude/project-thread-6jo3w5
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: plan unit — spec clarified (2 markers resolved from the issue text; round 2 found no critical ambiguities)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Where is plan usage shown today? | App bar trailing actions (`ui/toolbar.rs` usage_indicator, feature 490); mockup board draws it in the title bar, canvas note puts it in the status bar — left as a clarify marker | agent-resolved | crates/micold-client/src/ui/toolbar.rs:95; mockup project/Main.dc.html, canvas.json note s1 |
| D2 | spec | CLI item when the session view is not the visible body | Omitted (FR-007, after spec review F2) | agent-resolved | spec.md FR-007 |
| D3 | clarify | Does a lost connection (no action) still raise a banner? | No: bar only (error dot, reconnecting text); conditions with an action keep their banner (FR-003, FR-009, US3 s5) | issue text | issue #1124 body "Failures that need action still raise a banner…"; ui/mod.rs connection_banner (Disconnected has no action) |
| D4 | clarify | Plan usage in status bar or title bar? | Status bar, after the CLI item; removed from the app bar; #1127 builds on that (FR-011, US5) | issue text | issue #1124 title; canvas note s1 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | dc7a76326c7ffef720f23d0c9ebe55080ecfac9a:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CHANGES: 3 MAJOR, 3 MINOR — all fixed |
| Spec | 2 | dfed0a366bf36453ce50a47b075d694ef9d0d313:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (1 MINOR: US4 scenario order — fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

# Autopilot ledger — #1125 sidebar-state-groups

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1125 "Layout: sidebar grouped by state with Overview as its first row" (body verbatim in the issue and in spec.md *Input*); the user asked to start the app redesign issues in dependency order. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1125
- **Worktree branch**: claude/project-thread-22fhnj
- **Started**: 2026-10-24
- **Phase**: clarify
- **Next step**: clarify unit (continued with the user's answers): record Q2/Q3 below as decided by user, apply them to spec.md US4 sc.6 + FR-015 and US5 sc.2 + FR-005, update checklists/requirements.md, then judge whether another clarify round is needed

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Overview button in the top bar: removed here? | No: it stays; #1127 removes it ("Blocked by: #1125 (Overview moves into the sidebar first)") | agent-resolved | issue #1127 body |
| D2 | spec | Order of Done vs Idle | Done last, after Idle | agent-resolved | issue #1125 body; mockup board sidebar |
| D3 | clarify 1 | Needs attention: own group or joins Needs action? | Joins Needs action; no separate group | agent-resolved | issue #1125 lists four groups; mockup puts fix/1117 "checks failing" (NeedsAttention, overview.rs card_state) under "Needs action · 2" |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | da2d807c9816acd2626754adf775bd75bfdf81bb:3c7db2eb08425cc9cab3f4001be9e64c03c8ccd2 | CLEAN (3 MINOR, fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product decision the repo does not settle):

- **Q2 (US4 sc.6, FR-015)**: Where do refresh worktree list, attach existing sessions and run in parallel go, now that only add and hide sit beside the chips? Options: (a) **(Recommended)** an overflow (⋯ more) menu at the chip bar's end, beside add and hide — keeps them in the sidebar with today's enabled rules and tooltips, matches the mockup's two visible actions, and does not depend on #1127, which is blocked by #1125 and does not list these actions for its title-bar overflow menu; (b) keep them as icons beside add and hide — no new menu, but the bar gets crowded and departs from the mockup; (c) move them to #1127's title-bar overflow menu — they would be unreachable from the sidebar until #1127 ships.
- **Q3 (US5 sc.2, FR-005)**: Which sidebar groups are collapsible (mockup's open question "keep the sidebar groups collapsible?")? Options: (a) **(Recommended)** every group collapsible, Done collapsed at start, the others expanded — one header behaviour for keyboard and pointer, and US5 sc.3/sc.4 already speak of any collapsed group; (b) only Done collapsible — fewer controls, but headers behave differently and Needs action/Working headers become inert.

## Follow-ups not done

None yet.

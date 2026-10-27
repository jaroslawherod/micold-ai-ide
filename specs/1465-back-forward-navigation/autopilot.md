# Autopilot ledger — #1465 back-forward-navigation

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1465 (issue #1465: Back/forward navigation with mouse side buttons and Alt+←/→); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1465
- **Worktree branch**: claude/project-thread-e4cve5
- **Started**: 2026-10-27
- **Phase**: plan written; review round 1 fixed (round 2 pending)
- **Next step**: plan review round 2 (scoped re-review of the round-1 fix diff), then tasks unit

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | macOS chord: Option+←/→ or Cmd+[ / Cmd+]? | Cmd+[ / Cmd+] on macOS; Alt+←/→ on Linux/Windows; same focus rule everywhere | agent | terminal-panes.md app-chord table (all macOS forms use Cmd, none Option); keymap.rs `is_primary_shift`; Cmd+Shift+[ / ] already prev/next session |
| 2 | clarify | Only the Review tab's sheet lists Back/Forward? | Yes; app-wide sheet out of scope | agent | spec.md Out of Scope |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 122d576a4ef8abf602bba7274943871a5a80802e:b4c2abed5e49a26d0925bc7011e0fabdc7c4eaa6 | CHANGES: 1 MAJOR, 3 MINOR (all fixed; F2–F4 prose only) |
| Spec review | 2 | 8dd4e55cf8901b03c0b2a3a594c57e3780e9ea71:90b186cd61c01d98ee42519bfcddf69b8bd79acf | CLEAN, 1 MINOR (FR-014/US5: Back/Forward app-wide but only the Review tab's sheet lists them; scoped explicitly, no fix required; noted in Clarifications) |
| Clarify | 1 | 2 questions, both agent-resolved | macOS chord → Cmd+[ / Cmd+]; Review-tab-only sheet confirmed |
| Clarify | 2 | rescan after round 1 | no critical ambiguities |
| Plan review | 1 | 11f3d102ca7aabfb92c1199ff5ca3699e08f8622:0ae4673ae350ac1a5197583750a38820c1241a47 | CHANGES: 1 MAJOR, 2 MINOR (F1 cross-project Compare: `runs.known` per-project group lists, R5 limit narrowed, tests + B10 added; F2 availability refreshed before `exists` (U2 step 0); F3 borrow split via `mem::take` in `resolve`) — all fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Plan review round 1 returned CHANGES (F1 MAJOR, F2/F3 MINOR); all fixed in research R5,
data-model, contracts/ui U2/U4/U7, plan D5/requirement map/tests, quickstart B10. Next: plan
review round 2, scoped to the fix diff from the round-1 snapshot (11f3d102…:0ae4673ae), on a
cheaper model; if CLEAN or only MINORs, set Next step to `tasks unit`.

## Open escalation

None.

## Follow-ups not done

None.

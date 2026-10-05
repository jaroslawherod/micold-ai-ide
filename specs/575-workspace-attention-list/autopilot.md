# Autopilot ledger — #575 workspace-attention-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #575 "Show sessions that need attention at workspace level" (show unread sessions at workspace level: per-project marks with counts in the switcher, one workspace-wide list naming project, worktree and session, selecting an entry opens it like a notification click, live and restart-proof, reusing the existing attention state). labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #575
- **Worktree branch**: claude/project-thread-8dnq8h
- **Started**: 2026-10-05
- **Phase**: clarify
- **Next step**: clarify unit (continued with the user's FR-001 answer): record it as decided by user, apply it to spec.md FR-001, then decide whether another round is needed (FR-004 answer changed a requirement, so a round 2 runs).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | What of #575 does 039 already ship? | Switcher row counts (FR-021/022), button total (FR-023), live clearing (FR-019), restart persistence (FR-008a), reveal path (FR-011–014). Spec covers only the workspace-wide list and its navigation. | agent-resolved | specs/039-session-attention-notifications/spec.md; crates/micold-client/src/ui/toolbar.rs:78; crates/micold-client/src/app.rs:503 |
| D2 | clarify | FR-004: order of attention-list entries? | Grouped by project in switcher order, then worktree and session in sidebar order; never by time of becoming unread. | agent-resolved | specs/039-session-attention-notifications/data-model.md (attention_seq is per-session, no time kept); spec.md FR-012, FR-014 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 575 | 1 | 30231c55b37effe9790da6ea31243981031aa938:937ae9f7da42e25571db7b988153f707ba931e3e | CLEAN: 3 MINOR (all fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Category 1 (product/UX decision the repo does not settle). Clarify round 1, FR-001:
Where does the attention list open from?
- (a) (Recommended) A section at the top of the project switcher's panel, above the project rows. The switcher button already carries the other-projects unread total (039 FR-023, crates/micold-client/src/ui/toolbar.rs:79-84), so the mark that says "something needs you" opens the place that says what; no new top-bar control, the panel and its overlay/keyboard handling are reused.
- (b) Its own button in the top bar beside the switcher, opening its own panel, with its own all-projects count. More discoverable as a separate control, but a third trailing action and a second unread badge beside the switcher's.

## Follow-ups not done

None.

# Autopilot ledger — #575 workspace-attention-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #575 "Show sessions that need attention at workspace level" (show unread sessions at workspace level: per-project marks with counts in the switcher, one workspace-wide list naming project, worktree and session, selecting an entry opens it like a notification click, live and restart-proof, reusing the existing attention state). labels: enhancement
- **Scope change (user, 2026-10-05)**: after spec and clarify, the user wrote verbatim: "the clue was to add indicator of attention at sidebar with the list of worktrees". The workspace attention list in the switcher's panel (old FR-001..FR-015, D2, D3) is dropped. New scope: each sidebar worktree row and the Default row with unread sessions (039) shows the unread mark with their count. Spec rewritten in place (directory name kept, title renamed).
- **Kind**: feature
- **Effort**: default
- **Issue**: #575
- **Worktree branch**: claude/project-thread-8dnq8h
- **Started**: 2026-10-05
- **Phase**: plan
- **Next step**: plan unit (clarify on the rewritten spec CLEAN after round 1: one agent-resolved question, no requirement changed).

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
| D2 | clarify | **Superseded (D4).** FR-004: order of attention-list entries? | Grouped by project in switcher order, then worktree and session in sidebar order; never by time of becoming unread. | agent-resolved | specs/039-session-attention-notifications/data-model.md (attention_seq is per-session, no time kept); spec.md FR-012, FR-014 |
| D3 | clarify | **Superseded (D4).** FR-001: where does the attention list open from? | (a) Section at the top of the switcher's panel, above the project rows. | orchestrator default, pending user answer on decision card | crates/micold-client/src/ui/toolbar.rs:57-94; 039 FR-023 |
| D4 | spec | What does #575 ask for? (scope change) | An attention indicator (unread mark + count) on each sidebar worktree row and the Default row holding unread sessions; the switcher attention list is dropped. | user: "the clue was to add indicator of attention at sidebar with the list of worktrees" | spec.md Clarifications |
| D5 | spec | Does the indicator show on an expanded location row? | Yes, expanded or collapsed; session rows keep their own marks. | orchestrator default | spec.md FR-003 |
| D6 | spec | Do closed (archived) unread sessions count? | No, on location rows nor in the switcher counts/button total, so a project's location rows add up to its switcher count (FR-010). | agent-resolved | 039 spec US1 scenario 9; crates/micold-core/src/workspace.rs:334 (`unread_session_count` filters on `unread` only); crates/micold-core/src/session.rs:339 `archived` |
| D7 | clarify | Which hidden worktrees does "the sidebar's filter" cover (edge case, FR-010)? | Both tag filters (008 FR-025) and hidden agent-owned worktrees (014); hidden rows show no indicator but still count on the switcher; the 024 re-admitted row shows its indicator. Wording only. | agent-resolved | crates/micold-client/src/features/sidebar.rs `filtered_worktree_tree`, `visible_worktrees`; crates/micold-core/src/workspace.rs:334 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 575 | 1 | 30231c55b37effe9790da6ea31243981031aa938:937ae9f7da42e25571db7b988153f707ba931e3e | CLEAN: 3 MINOR (all fixed, prose only) — superseded by the rewrite |
| Spec 575 (rewrite) | 1 | 84afa026eb373957d3319604344384c2c53d8ee4:6075557947bfb5e1361568a3fa8ad0408ce9d424 | CLEAN: 3 MINOR (all fixed, prose only; run via `claude -p --agent autopilot-reviewer`) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (D3's decision card is moot: superseded by D4.)

## Follow-ups not done

None.

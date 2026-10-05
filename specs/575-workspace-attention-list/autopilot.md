# Autopilot ledger — #575 workspace-attention-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #575 "Show sessions that need attention at workspace level" (show unread sessions at workspace level: per-project marks with counts in the switcher, one workspace-wide list naming project, worktree and session, selecting an entry opens it like a notification click, live and restart-proof, reusing the existing attention state). labels: enhancement
- **Kind**: feature
- **Effort**: default
- **Issue**: #575
- **Worktree branch**: claude/project-thread-8dnq8h
- **Started**: 2026-10-05
- **Phase**: plan
- **Next step**: plan unit. Clarify CLEAN after 2 rounds (round 2: no critical ambiguities). If the user picks (b) for FR-001, re-apply FR-001 first.

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
| D3 | clarify | FR-001: where does the attention list open from? | (a) Section at the top of the switcher's panel, above the project rows. | orchestrator default, pending user answer on decision card | crates/micold-client/src/ui/toolbar.rs:57-94; 039 FR-023 |

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

None. (D3 awaits the user's confirmation on the orchestrator's decision card; a (b) answer reopens FR-001.)

## Follow-ups not done

None.

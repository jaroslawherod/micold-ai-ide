# Autopilot ledger — #1122 overview-card-issue-link

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1122: Overview cards: link to the worktree's issue. Blocked by: #1120 (the branch line the link sits on). "Issue #N" beside the branch opens the GitHub issue in the browser; N comes from the worktree name, as the issue-title summary already does. A Jira key (MIC-87) in the name links to `<Jira base URL>/browse/<key>`, which needs a Jira base URL setting and shows no link while it is unset. A press on a link does not activate the card. No link on Default or on a worktree with no issue. Acceptance: both links open the right URL; Settings has the Jira field; card activation is unaffected; docs updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1122
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: spec
- **Next step**: Review A done. Clarify unit: FR-001 (links follow integration switches) and FR-010 (Jira base URL source) markers, answers from jaro's decision cards.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | How is the issue link reached by keyboard, given 1121's focus-expands rule? | A Tab stop right after its card; arrows move between cards only; focus on the link counts as focus on the card | agent-resolved | spec.md#FR-008; specs/1121-overview-card-full-text/spec.md#FR-002 |
| D2 | spec | A collapsed card's link moves when the card expands (lines are added above it); how is a click aimed at it kept off the card? | A pointer resting on a collapsed card's link does not expand the card | agent-resolved | spec.md#FR-007 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | 399604367e0651717c12d766941fef7e892d58e9:48d810a53d17d50a45ab3b3ffaabec63220e6048 | CHANGES: 5 MAJOR, 3 MINOR (F1–F8), all fixed in spec.md, none declined |
| Review A spec | 2 | 5b39717f:af373d3b (fix diff, sonnet) | CHANGES: 1 MAJOR (F1 FR-005/FR-006 press-off-link contradiction, ripple on press), 1 MINOR (F2 SC-007 not measurable); both fixed, prose only, no round 3 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

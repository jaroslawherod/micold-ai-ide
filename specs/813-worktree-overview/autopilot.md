# Autopilot ledger — #813 worktree-overview

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #813 "feat: worktree overview with state-aware cards (sessions, PRs, task
  progress)" (title and body as given to the spec unit); labels: enhancement, effort:high,
  flow:feature
- **Kind**: feature
- **Effort**: high
- **Issue**: #813
- **Worktree branch**: claude/project-thread-2lmupk
- **Started**: 2026-10-17
- **Phase**: clarify
- **Next step**: clarify unit: resolve FR-023's `[NEEDS CLARIFICATION]` (overlay vs main pane)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Definition of "main session" (issue open question) | Feature 776's main session: the un-archived session marked main, else the most recently active | agent-resolved | crates/micold-core/src/occupancy.rs `main_session`; specs/776-one-session-per-worktree |
| D2 | spec | Is the LLM summary (option 3) in the first iteration? | No: out of scope; options 1 and 2 ship (issue's recommendation) | agent-resolved | issue #813 "Recommended: 1 as the baseline and 2 where hooks are configured" |
| D3 | spec | Card state vs feature 905's location state | Reuse 905's state; Blocked: needs permission → Needs action, Blocked: failed/resumable → Needs attention | agent-resolved | crates/micold-core/src/location_state.rs |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c54be34533424e39665e73004ef6ce8f0cadc4e2:992e129808513fd43ef6b1982b4cb1881cdb0921 | CLEAN (3 MINOR: F1 F2 F3 fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

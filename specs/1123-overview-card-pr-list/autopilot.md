# Autopilot ledger — #1123 overview-card-pr-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #1123: Overview cards: pull request list with state, linked. Blocked by: #1121 (expanded view for the rest of the list). The app reads PR counts only. Read, for each PR of a branch: number, URL, state (open, draft, merged, closed), checks (passing, failing, running, none), review (approved, changes requested) and merge conflict. Reuse the existing PR status reading and its rate-limit handling; no new polling. The card shows up to two PR rows (number as a link, state pill, checks or review in words), the rest on expand. A press on the number opens the PR and does not activate the card. A worktree with no PR shows no row. Acceptance: core type and parser tests; unreadable states keep today's dash reasons; counts derive from the list; docs and layout snapshot updated. labels: enhancement, flow:feature, in-progress
- **Kind**: feature
- **Effort**: default
- **Issue**: #1123
- **Worktree branch**: claude/project-thread-7041o6
- **Started**: 2026-10-21
- **Phase**: spec
- **Next step**: spec review A round 1 (orchestrator dispatches; prompt in the scratchpad)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Base | origin/main f1d7d1b5 (#1121, #1122 merged) | agent-resolved | orchestrator prompt |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review A spec | 1 | afc9cb5d94279a3c0ae00d37d80855a145ded446:f1d7d1b5b3251cbb108318452ccb6a4ede3d1f3e | pending |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.

# Autopilot ledger — #1400 github-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1400; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1400
- **Worktree branch**: claude/project-thread-5zc5qk
- **Started**: 2026-10-27
- **Phase**: clarify
- **Next step**: clarify round 1 escalated (Open escalation, category 1); on the answer, the clarify unit records it, applies it to spec.md (US6, FR-003, FR-004, FR-005, Key Entities) and runs round 2 if it changed a requirement.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | a81cb29b681279bab9bac6fd165413021a0bd717:1132114243bde1ea489deb6c3c5ea3032f754e6f | CHANGES: F1 MAJOR (FR-002 vs Jira-and-back edge case: draft) fixed, draft kept; F2 MINOR (FR-013 empty last run query) fixed, a run empty query re-runs, prompt only when none ever run; F3 MINOR (FR-007 advanced mode) fixed, FR-007 and US1 s6 basic mode only |
| Spec review | 2 | ef51560eb3b59349331931d5814bbada52ed7dff:6929b5aec40be28081e16fd917d82c2cc3390bad | CLEAN (scoped re-review, sonnet): round-1 F1–F3 all fixed; F1 MINOR (Edge case 'Switching source to Jira and back': what reloads in advanced mode) fixed, re-runs the last run query, not the draft |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product or scope decision the repo does not settle).

- **Q1**: Issue #1400 lists "optionally assignee, author and milestone" among the basic filters. Are those three basic filters part of this feature?
  - **(Recommended) Include them, as P3 in the last milestone** — spec.md US6/FR-003/FR-005 already specify them in full; the issue names them in its basic-filter proposal; shipping them last keeps the P1 use case (exclude `in-progress`/`blocked`) unblocked and lets them be dropped late if needed.
  - **Leave them out; advanced mode covers them** (`no:assignee`, `author:@me`, `milestone:"v2.4"`) — smaller scope: no assignable-user search, viewer lookup or milestone choice loads; US6 moves to Out of Scope for a later issue. Precedent: 1185 left assignee out of the Jira basic filters (specs/1185-jira-selector-filters/spec.md Out of Scope).
  - Evidence checked: issue #1400 body (only "optionally"), issue comments (none), specs/1185-jira-selector-filters, specs/1186-jira-advanced-jql, specs/034-github-issue-worktree — none settles it.

## Follow-ups not done

- Cloud session: `gh` unauthenticated, so `branch-start.sh` and `issue.sh` were skipped (orchestrator set labels/assignee via GitHub MCP; branch already at origin/main 828bc8e24). Not pushed: the orchestrator pushes with the design PR.

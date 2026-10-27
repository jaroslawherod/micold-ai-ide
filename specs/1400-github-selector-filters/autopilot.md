# Autopilot ledger — #1400 github-selector-filters

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1400; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1400
- **Worktree branch**: claude/project-thread-5zc5qk
- **Started**: 2026-10-27
- **Phase**: plan
- **Next step**: plan review round 3 (scoped, sonnet; last counted round, a BLOCKER/MAJOR there escalates, category 5); then tasks unit. D1 still provisional: plan keeps US6 separable in M4 (plan.md §US6 separability).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | clarify | Are assignee, author and milestone basic filters in scope (issue says "optionally")? | Yes: US6 as P3, built in the last milestone | provisional: recommended option, awaiting user | spec.md US6/FR-003/FR-005 already specify them; issue #1400 lists them in the basic-filter proposal; no repo source settles it. If the user picks "leave them out", a later unit moves US6 to Out of Scope. |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | a81cb29b681279bab9bac6fd165413021a0bd717:1132114243bde1ea489deb6c3c5ea3032f754e6f | CHANGES: F1 MAJOR (FR-002 vs Jira-and-back edge case: draft) fixed, draft kept; F2 MINOR (FR-013 empty last run query) fixed, a run empty query re-runs, prompt only when none ever run; F3 MINOR (FR-007 advanced mode) fixed, FR-007 and US1 s6 basic mode only |
| Spec review | 2 | ef51560eb3b59349331931d5814bbada52ed7dff:6929b5aec40be28081e16fd917d82c2cc3390bad | CLEAN (scoped re-review, sonnet): round-1 F1–F3 all fixed; F1 MINOR (Edge case 'Switching source to Jira and back': what reloads in advanced mode) fixed, re-runs the last run query, not the draft |
| Plan review | 1 | dca80a0048327e23892aea730395cc936f33c90d:300a7b70807ff667b356bcdc9dcd3528cc1373e4 | CHANGES: F1 MAJOR (filtered search beyond appends raw typed text under ISSUE_ADVANCED, `OR`/`)`/`repo:` escape the filter) fixed: typed text enters as quoted words, `None` when no word, search-rules §4 rows for `crash OR ui`, `a)`, `repo:x` + core tests, R4/data-model updated; F2 MINOR (Closed chip vs status-board status) fixed: Closed wins, not stale, selector-ui §3 + issue_rows_show_all_text arm |
| Plan review | 2 | 1df92c5dd1194b03f86ff53cb2d49ca2d9e61d4d:062d5a81eaec848b15a02f648cb52f8e2be7d71c | CHANGES (scoped, sonnet): round-1 F1 and F2 confirmed fixed; F1 MAJOR (typed `\` could close its own quote under ISSUE_ADVANCED, unverified) fixed: `\` dropped like `"`, §4 row `a\ OR b` + `None` rows, R4/data-model updated, backslash probe added to R7/M1 fixture probe; F2 MINOR (R4 Rejected list order a,c,b) fixed |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

Clarify round 1, category 1 (product or scope decision the repo does not settle). On the user's decision card; proceeding on the recommended option as D1 (provisional). Move to Decisions as decided by user when answered.

- **Q1**: Issue #1400 lists "optionally assignee, author and milestone" among the basic filters. Are those three basic filters part of this feature?
  - **(Recommended) Include them, as P3 in the last milestone** — spec.md US6/FR-003/FR-005 already specify them in full; the issue names them in its basic-filter proposal; shipping them last keeps the P1 use case (exclude `in-progress`/`blocked`) unblocked and lets them be dropped late if needed.
  - **Leave them out; advanced mode covers them** (`no:assignee`, `author:@me`, `milestone:"v2.4"`) — smaller scope: no assignable-user search, viewer lookup or milestone choice loads; US6 moves to Out of Scope for a later issue. Precedent: 1185 left assignee out of the Jira basic filters (specs/1185-jira-selector-filters/spec.md Out of Scope).
  - Evidence checked: issue #1400 body (only "optionally"), issue comments (none), specs/1185-jira-selector-filters, specs/1186-jira-advanced-jql, specs/034-github-issue-worktree — none settles it.

## Follow-ups not done

- Plan unit: LSP tool not available in this session (ToolSearch found no `LSP`); symbols checked with grep. ISSUE_ADVANCED and the refused-search error shape not probed live (no signed-in `gh`): research R3/R7, M1 fixture probe.
- Cloud session: `gh` unauthenticated, so `branch-start.sh` and `issue.sh` were skipped (orchestrator set labels/assignee via GitHub MCP; branch already at origin/main 828bc8e24). Not pushed: the orchestrator pushes with the design PR.

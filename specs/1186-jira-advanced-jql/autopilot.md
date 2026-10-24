# Autopilot ledger — #1186 jira-advanced-jql

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1186 — issue #1186 "Support advanced JQL in the Jira issue selector"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1186
- **Worktree branch**: claude/project-thread-vf2ck6
- **Started**: 2026-10-24
- **Phase**: spec
- **Next step**: clarify unit (spec reviewed; see Review rounds)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D2 | spec | What does an empty advanced query list? | `project = "<KEY>" ORDER BY updated DESC` (no status filter, same rule as any advanced query) | agent-resolved | spec review round 1 F6 |
| D1 | spec | How does advanced mode compose with #1185's basic filters? | Basic/advanced toggle: advanced hides and ignores search text and basic filters; switching back restores them | agent-resolved | orchestrator scope note; spec.md#Assumptions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 3f9cc1e70535abc53505245bbebd94feceb8cdb5:b43218cdb7025279d1129ac4f6452b88298f123b | CHANGES: 3 MAJOR, 3 MINOR — all fixed (quote/sort-clause terms; FR-004a auto-run on open; draft vs last run query; moved-issue edge; HTTP 400 = invalid query; empty query = same rule as sort-only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.

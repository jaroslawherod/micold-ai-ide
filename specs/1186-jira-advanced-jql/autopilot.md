# Autopilot ledger — #1186 jira-advanced-jql

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 1186 — issue #1186 "Support advanced JQL in the Jira issue selector"; labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1186
- **Worktree branch**: claude/project-thread-vf2ck6
- **Started**: 2026-10-24
- **Phase**: plan
- **Next step**: tasks unit: plan committed, review round 1 CLEAN; milestones M1 core rules, M2 request+storage, M3 selector state+view (plan.md#delivery-order); #1185 overlap listed in plan.md#overlap-with-1185-basic-field-filters

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D6 | plan | Where are the mode and last run query remembered? | Owner-only `<data_dir>/jira/<project id>.query.json` beside the token, deleted with the connection; not the project state file, not settings.json | agent-resolved | research.md R7 |
| D5 | clarify | Remembering and source switch | Mode remembered at each switch, query at each run; GitHub↔Jira keeps mode, resets draft to last run query, re-runs it in advanced (FR-003a) | agent-resolved | 533 FR-017; #1185 FR-016 |
| D4 | clarify | Open in advanced with no last run query | Nothing sent; prompt in empty input; basic list loads only on first switch to basic | agent-resolved | 533 FR-020 |
| D3 | spec | How does the code compose with #1185's changes to search_jql/list_jql? | Plan adds a separate advanced-JQL builder beside search_jql; search_jql/list_jql are not restructured (if the plan finds it must, record it here and tell the orchestrator) | agent-resolved | coordinator note 2026-10-24; specs/1185-jira-selector-filters on origin/claude/project-thread-xwkotj lists #1186 as out of scope |
| D2 | spec | What does an empty advanced query list? | `project = "<KEY>" ORDER BY updated DESC` (no status filter, same rule as any advanced query) | agent-resolved | spec review round 1 F6 |
| D1 | spec | How does advanced mode compose with #1185's basic filters? | Basic/advanced toggle: advanced hides and ignores search text and basic filters; switching back restores them | agent-resolved | orchestrator scope note; spec.md#Assumptions |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec review | 1 | 3f9cc1e70535abc53505245bbebd94feceb8cdb5:b43218cdb7025279d1129ac4f6452b88298f123b | CHANGES: 3 MAJOR, 3 MINOR — all fixed (quote/sort-clause terms; FR-004a auto-run on open; draft vs last run query; moved-issue edge; HTTP 400 = invalid query; empty query = same rule as sort-only) |
| Clarify | 1–2 | — | CLEAN: round 1 five questions agent-resolved (1 deferred to plan); round 2 no critical ambiguity |
| Plan review | 1 | 840893a6b7c27c77bf984af45f25a08d59317ea2:57116fa3b1d2a80ddd0ef5798f90e39ed94bd94b | CLEAN (3 MINOR, fixed, prose only: awaited_jira_query accessor named; data-model §3 ref; FR-016 source scan added) |
| Spec review | 2 | 50939e7590e958b50053133db00b8fe7f26e966b:5510e0471524f4d4a847f1d9f80dc643b9b99499 | CLEAN (1 MINOR kept: HTTP 400 in FR-010 is the observable rule round 1 asked for) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None.
